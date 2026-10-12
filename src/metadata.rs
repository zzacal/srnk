//! Metadata handling: preserve (with patched pixel dimensions), strip, or drop + warn.

use img_parts::jpeg::{markers, Jpeg};
use img_parts::{Bytes, ImageEXIF, ImageICC};

use crate::geometry::Dimensions;
use crate::summary::MetadataMode;

/// What happened to the source metadata for one copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataOutcome {
    Preserved,
    Stripped,
    UnreadableDropped,
}

/// The bytes to write, plus the outcome to report.
#[derive(Debug)]
pub struct MetadataResult {
    pub bytes: Vec<u8>,
    pub outcome: MetadataOutcome,
}

/// Apply the metadata mode to the freshly encoded JPEG.
pub fn apply(
    source: &[u8],
    encoded: Vec<u8>,
    mode: MetadataMode,
    dimensions: Dimensions,
) -> MetadataResult {
    match mode {
        MetadataMode::Strip => MetadataResult {
            bytes: encoded,
            outcome: MetadataOutcome::Stripped,
        },
        MetadataMode::Preserve => match preserve(source, &encoded, dimensions) {
            Some(bytes) => MetadataResult {
                bytes,
                outcome: MetadataOutcome::Preserved,
            },
            None => MetadataResult {
                bytes: encoded,
                outcome: MetadataOutcome::UnreadableDropped,
            },
        },
    }
}

/// Lift APP1/Exif, APP2/ICC and APP13/IPTC from the source onto the re-encoded output.
/// `None` means the source metadata could not be parsed: caller drops it and warns.
fn preserve(source: &[u8], encoded: &[u8], dimensions: Dimensions) -> Option<Vec<u8>> {
    let src = Jpeg::from_bytes(Bytes::copy_from_slice(source)).ok()?;
    let exif = src.exif();
    let icc = src.icc_profile();
    let iptc: Vec<_> = src
        .segments()
        .iter()
        .filter(|segment| segment.marker() == markers::APP13)
        .cloned()
        .collect();

    if exif.is_none() && icc.is_none() && iptc.is_empty() {
        // Nothing to carry over -- the re-encoded bytes are already correct.
        return Some(encoded.to_vec());
    }

    let patched = match exif {
        Some(raw) => {
            let mut buffer = raw.to_vec();
            patch_exif(&mut buffer, dimensions)?;
            // Verify the edited blob still parses before it is re-inserted.
            exif::Reader::new()
                .read_raw(tiff_payload(&buffer).to_vec())
                .ok()?;
            Some(Bytes::from(buffer))
        }
        None => None,
    };

    let mut out = Jpeg::from_bytes(Bytes::copy_from_slice(encoded)).ok()?;
    out.set_exif(patched);
    out.set_icc_profile(icc);
    for (index, segment) in iptc.into_iter().enumerate() {
        out.segments_mut().insert(index, segment);
    }

    let mut buffer = Vec::with_capacity(encoded.len());
    out.encoder().write_to(&mut buffer).ok()?;
    Some(buffer)
}

/// Some writers keep the `Exif\0\0` identifier in the payload; tolerate both shapes.
fn exif_prefix_len(buffer: &[u8]) -> usize {
    if buffer.len() >= 6 && &buffer[0..4] == b"Exif" {
        6
    } else {
        0
    }
}

fn tiff_payload(buffer: &[u8]) -> &[u8] {
    &buffer[exif_prefix_len(buffer)..]
}

/// Rewrite the Exif values the resize invalidates, in place (lengths never change):
/// `PixelXDimension`/`PixelYDimension` to the new size, `ImageWidth`/`ImageLength`
/// neutralised to the padding tag, and the IFD1 thumbnail dropped.
fn patch_exif(buffer: &mut [u8], dimensions: Dimensions) -> Option<()> {
    let start = exif_prefix_len(buffer);
    let tiff = &mut buffer[start..];
    if tiff.len() < 8 {
        return None;
    }
    let little_endian = match &tiff[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    if read_u16(tiff, 2, little_endian)? != 42 {
        return None;
    }
    let ifd0 = read_u32(tiff, 4, little_endian)? as usize;
    patch_ifd0(tiff, ifd0, little_endian, dimensions)
}

/// `0xEA1C` is the Exif padding tag: a fixed-width way to retire a stale entry.
const TAG_PADDING: u16 = 0xEA1C;
const TAG_IMAGE_WIDTH: u16 = 0x0100;
const TAG_IMAGE_LENGTH: u16 = 0x0101;
const TAG_EXIF_IFD: u16 = 0x8769;
const TAG_PIXEL_X: u16 = 0xA002;
const TAG_PIXEL_Y: u16 = 0xA003;

fn patch_ifd0(
    tiff: &mut [u8],
    offset: usize,
    little_endian: bool,
    dimensions: Dimensions,
) -> Option<()> {
    let count = read_u16(tiff, offset, little_endian)? as usize;
    let end = offset.checked_add(2 + count * 12)?;
    if end + 4 > tiff.len() {
        return None;
    }
    let mut exif_ifd = None;
    for index in 0..count {
        let entry = offset + 2 + index * 12;
        let tag = read_u16(tiff, entry, little_endian)?;
        match tag {
            TAG_IMAGE_WIDTH | TAG_IMAGE_LENGTH => {
                write_u16(tiff, entry, TAG_PADDING, little_endian)
            }
            TAG_EXIF_IFD => exif_ifd = Some(read_u32(tiff, entry + 8, little_endian)? as usize),
            _ => {}
        }
    }
    // Drop IFD1 (the thumbnail) by cutting the next-IFD pointer.
    write_u32(tiff, end, 0, little_endian);

    if let Some(sub) = exif_ifd {
        patch_exif_ifd(tiff, sub, little_endian, dimensions)?;
    }
    Some(())
}

fn patch_exif_ifd(
    tiff: &mut [u8],
    offset: usize,
    little_endian: bool,
    dimensions: Dimensions,
) -> Option<()> {
    let count = read_u16(tiff, offset, little_endian)? as usize;
    if offset + 2 + count * 12 + 4 > tiff.len() {
        return None;
    }
    for index in 0..count {
        let entry = offset + 2 + index * 12;
        let tag = read_u16(tiff, entry, little_endian)?;
        let value = match tag {
            TAG_PIXEL_X => dimensions.width,
            TAG_PIXEL_Y => dimensions.height,
            _ => continue,
        };
        let field_type = read_u16(tiff, entry + 2, little_endian)?;
        match field_type {
            3 => {
                // SHORT, stored in the first two bytes of the value field.
                write_u16(
                    tiff,
                    entry + 8,
                    u16::try_from(value).unwrap_or(u16::MAX),
                    little_endian,
                );
                write_u16(tiff, entry + 10, 0, little_endian);
            }
            4 => write_u32(tiff, entry + 8, value, little_endian),
            _ => return None,
        }
    }
    Some(())
}

fn read_u16(buffer: &[u8], at: usize, little_endian: bool) -> Option<u16> {
    let slice = buffer.get(at..at + 2)?;
    let raw = [slice[0], slice[1]];
    Some(if little_endian {
        u16::from_le_bytes(raw)
    } else {
        u16::from_be_bytes(raw)
    })
}

fn read_u32(buffer: &[u8], at: usize, little_endian: bool) -> Option<u32> {
    let slice = buffer.get(at..at + 4)?;
    let raw = [slice[0], slice[1], slice[2], slice[3]];
    Some(if little_endian {
        u32::from_le_bytes(raw)
    } else {
        u32::from_be_bytes(raw)
    })
}

fn write_u16(buffer: &mut [u8], at: usize, value: u16, little_endian: bool) {
    let raw = if little_endian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    if let Some(slot) = buffer.get_mut(at..at + 2) {
        slot.copy_from_slice(&raw);
    }
}

fn write_u32(buffer: &mut [u8], at: usize, value: u32, little_endian: bool) {
    let raw = if little_endian {
        value.to_le_bytes()
    } else {
        value.to_be_bytes()
    };
    if let Some(slot) = buffer.get_mut(at..at + 4) {
        slot.copy_from_slice(&raw);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_exif_is_unreadable() {
        let mut buffer = b"Exif\0\0not a tiff header".to_vec();
        assert!(patch_exif(&mut buffer, Dimensions::new(10, 10)).is_none());
    }

    #[test]
    fn strip_mode_passes_the_encoded_bytes_through() {
        let encoded = vec![1, 2, 3, 4];
        let result = apply(
            &[],
            encoded.clone(),
            MetadataMode::Strip,
            Dimensions::new(2, 2),
        );
        assert_eq!(result.bytes, encoded);
        assert_eq!(result.outcome, MetadataOutcome::Stripped);
    }

    #[test]
    fn unparseable_source_container_drops_metadata() {
        let result = apply(
            b"definitely not a jpeg",
            vec![9, 9, 9],
            MetadataMode::Preserve,
            Dimensions::new(2, 2),
        );
        assert_eq!(result.outcome, MetadataOutcome::UnreadableDropped);
        assert_eq!(result.bytes, vec![9, 9, 9]);
    }
}
