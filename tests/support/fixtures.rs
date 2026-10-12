//! Test fixture generation: JPEGs, EXIF blobs and helpers. No binary images are committed.
#![allow(dead_code)]

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat};
use img_parts::jpeg::Jpeg;
use img_parts::{Bytes, ImageEXIF, ImageICC};
use tempfile::TempDir;

/// A scratch folder used as the tool's working directory.
pub fn temp() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

/// The binary under test, run inside `dir`.
pub fn srnk(dir: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("srnk").expect("built binary");
    cmd.current_dir(dir);
    cmd
}

/// A known gradient, encoded as JPEG.
pub fn jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut img = image::RgbImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]);
    }
    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, 95)
        .encode_image(&DynamicImage::ImageRgb8(img))
        .expect("encode fixture");
    out.into_inner()
}

pub fn write_bytes(root: &Path, rel: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture parent");
    }
    std::fs::write(&path, bytes).expect("write fixture");
    path
}

pub fn write_jpeg(root: &Path, rel: &str, width: u32, height: u32) -> PathBuf {
    write_bytes(root, rel, &jpeg_bytes(width, height))
}

/// A JPEG carrying a known EXIF blob: Model, Orientation, DateTimeOriginal, GPS,
/// and PixelXDimension/PixelYDimension.
pub fn write_jpeg_with_exif(root: &Path, rel: &str, width: u32, height: u32) -> PathBuf {
    let base = jpeg_bytes(width, height);
    let mut jpeg = Jpeg::from_bytes(Bytes::from(base)).expect("fixture container");
    jpeg.set_exif(Some(Bytes::from(exif_blob(width, height))));
    jpeg.set_icc_profile(Some(Bytes::from_static(b"fake-icc-profile-payload")));
    let mut out = Vec::new();
    jpeg.encoder()
        .write_to(&mut out)
        .expect("write fixture jpeg");
    write_bytes(root, rel, &out)
}

/// A JPEG with valid pixels but an unparseable EXIF segment.
pub fn write_jpeg_with_corrupt_exif(root: &Path, rel: &str, width: u32, height: u32) -> PathBuf {
    let base = jpeg_bytes(width, height);
    let mut jpeg = Jpeg::from_bytes(Bytes::from(base)).expect("fixture container");
    jpeg.set_exif(Some(Bytes::from_static(
        b"this is not a tiff header at all",
    )));
    let mut out = Vec::new();
    jpeg.encoder()
        .write_to(&mut out)
        .expect("write fixture jpeg");
    write_bytes(root, rel, &out)
}

/// A truncated (corrupt) JPEG: the SOI marker survives, everything after it is rubbish.
/// The fixture asserts the result really is undecodable, so the skip path is exercised.
pub fn write_truncated_jpeg(root: &Path, rel: &str) -> PathBuf {
    let full = jpeg_bytes(200, 200);
    let mut cut = full[..2].to_vec();
    cut.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x10]);
    cut.extend_from_slice(b"truncated and corrupt payload");
    assert!(
        image::load_from_memory_with_format(&cut, ImageFormat::Jpeg).is_err(),
        "the corrupt fixture must not be decodable"
    );
    write_bytes(root, rel, &cut)
}

pub fn dimensions(path: &Path) -> (u32, u32) {
    image::image_dimensions(path).unwrap_or_else(|e| panic!("dimensions of {path:?}: {e}"))
}

pub fn decodes(path: &Path) -> bool {
    let bytes = std::fs::read(path).expect("read copy");
    image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg).is_ok()
}

pub fn exif_of(path: &Path) -> Option<exif::Exif> {
    let bytes = std::fs::read(path).ok()?;
    let jpeg = Jpeg::from_bytes(Bytes::from(bytes)).ok()?;
    let raw = jpeg.exif()?;
    let mut payload = raw.to_vec();
    if payload.len() >= 6 && &payload[0..4] == b"Exif" {
        payload = payload[6..].to_vec();
    }
    exif::Reader::new().read_raw(payload).ok()
}

pub fn has_exif(path: &Path) -> bool {
    let bytes = std::fs::read(path).expect("read copy");
    Jpeg::from_bytes(Bytes::from(bytes))
        .expect("container")
        .exif()
        .is_some()
}

pub fn has_icc(path: &Path) -> bool {
    let bytes = std::fs::read(path).expect("read copy");
    Jpeg::from_bytes(Bytes::from(bytes))
        .expect("container")
        .icc_profile()
        .is_some()
}

/// Sorted file names directly inside `dir` (empty when it does not exist).
pub fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

/// One `  key:   value` line of the run summary, trimmed.
pub fn summary_field(stdout: &str, key: &str) -> String {
    let needle = format!("{key}:");
    stdout
        .lines()
        .find(|line| line.trim_start().starts_with(&needle))
        .map(|line| line.trim().trim_start_matches(&needle).trim().to_string())
        .unwrap_or_else(|| panic!("summary has no `{key}` line in:\n{stdout}"))
}

/// Whether this filesystem distinguishes `a.jpg` from `a.JPG` (gates the collision test).
pub fn case_sensitive_fs(root: &Path) -> bool {
    let lower = root.join("case-probe.tmp");
    std::fs::write(&lower, b"lower").expect("probe");
    let upper = root.join("CASE-PROBE.TMP");
    let sensitive = !upper.exists();
    let _ = std::fs::remove_file(&lower);
    let _ = std::fs::remove_file(&upper);
    sensitive
}

/// Build a little-endian TIFF/Exif blob with known tags.
pub fn exif_blob(width: u32, height: u32) -> Vec<u8> {
    let model: &[u8] = b"SRNK Camera\0";
    let captured: &[u8] = b"2021:01:02 03:04:05\0";

    let ifd0_entries: u16 = 6;
    let exif_entries: u16 = 3;
    let gps_entries: u16 = 2;

    let ifd0_off = 8usize;
    let exif_off = ifd0_off + 2 + 12 * ifd0_entries as usize + 4;
    let gps_off = exif_off + 2 + 12 * exif_entries as usize + 4;
    let data_off = gps_off + 2 + 12 * gps_entries as usize + 4;
    let model_off = data_off;
    let captured_off = model_off + model.len();
    let latitude_off = captured_off + captured.len();

    let mut out = Vec::new();
    out.extend_from_slice(b"II");
    out.extend_from_slice(&42u16.to_le_bytes());
    out.extend_from_slice(&(ifd0_off as u32).to_le_bytes());

    // IFD0
    out.extend_from_slice(&ifd0_entries.to_le_bytes());
    entry(&mut out, 0x0100, 4, 1, width); // ImageWidth
    entry(&mut out, 0x0101, 4, 1, height); // ImageLength
    entry(&mut out, 0x0110, 2, model.len() as u32, model_off as u32); // Model
    short_entry(&mut out, 0x0112, 6); // Orientation
    entry(&mut out, 0x8769, 4, 1, exif_off as u32); // Exif IFD pointer
    entry(&mut out, 0x8825, 4, 1, gps_off as u32); // GPS IFD pointer
    out.extend_from_slice(&0u32.to_le_bytes()); // no IFD1

    // Exif IFD
    out.extend_from_slice(&exif_entries.to_le_bytes());
    entry(
        &mut out,
        0x9003,
        2,
        captured.len() as u32,
        captured_off as u32,
    ); // DateTimeOriginal
    entry(&mut out, 0xA002, 4, 1, width); // PixelXDimension
    entry(&mut out, 0xA003, 4, 1, height); // PixelYDimension
    out.extend_from_slice(&0u32.to_le_bytes());

    // GPS IFD
    out.extend_from_slice(&gps_entries.to_le_bytes());
    inline_ascii(&mut out, 0x0001, b"N\0"); // GPSLatitudeRef
    entry(&mut out, 0x0002, 5, 3, latitude_off as u32); // GPSLatitude
    out.extend_from_slice(&0u32.to_le_bytes());

    assert_eq!(out.len(), data_off, "fixture offsets must line up");
    out.extend_from_slice(model);
    out.extend_from_slice(captured);
    for (numerator, denominator) in [(47u32, 1u32), (30, 1), (0, 1)] {
        out.extend_from_slice(&numerator.to_le_bytes());
        out.extend_from_slice(&denominator.to_le_bytes());
    }
    out
}

fn entry(out: &mut Vec<u8>, tag: u16, field_type: u16, count: u32, value: u32) {
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&field_type.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&value.to_le_bytes());
}

fn short_entry(out: &mut Vec<u8>, tag: u16, value: u16) {
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&3u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
}

fn inline_ascii(out: &mut Vec<u8>, tag: u16, text: &[u8]) {
    assert!(text.len() <= 4);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    let mut padded = [0u8; 4];
    padded[..text.len()].copy_from_slice(text);
    out.extend_from_slice(&padded);
}
