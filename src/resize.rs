//! Decode, Lanczos3 downscale, and JPEG re-encode of a single image.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};

use crate::geometry::{self, Dimensions};

/// Fixed internal encoder quality (not a CLI option).
const JPEG_QUALITY: u8 = 90;

/// Distinct failure modes, mapping onto `SkipReason::DecodeFailed` / `EncodeFailed`.
#[derive(Debug)]
pub enum ResizeError {
    Decode(String),
    Encode(String),
}

/// A re-encoded, metadata-free JPEG plus its dimensions.
#[derive(Debug)]
pub struct Resized {
    pub bytes: Vec<u8>,
    /// The decoded source dimensions the target was derived from.
    #[allow(dead_code)]
    pub source: Dimensions,
    pub dimensions: Dimensions,
}

/// Decode `bytes`, downscale by `factor`, and re-encode as JPEG.
pub fn resize_jpeg(bytes: &[u8], factor: u8) -> Result<Resized, ResizeError> {
    let decoded = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
        .map_err(|err| ResizeError::Decode(err.to_string()))?;

    let source = Dimensions::new(decoded.width().max(1), decoded.height().max(1));
    let target = geometry::resized(source, factor);

    let scaled = decoded.resize_exact(target.width, target.height, FilterType::Lanczos3);
    let rgb = DynamicImage::ImageRgb8(scaled.to_rgb8());

    let mut out = Cursor::new(Vec::new());
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|err| ResizeError::Encode(err.to_string()))?;

    Ok(Resized {
        bytes: out.into_inner(),
        source,
        dimensions: target,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient_jpeg(width: u32, height: u32) -> Vec<u8> {
        let mut img = image::RgbImage::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
        }
        let mut out = Cursor::new(Vec::new());
        JpegEncoder::new_with_quality(&mut out, 95)
            .encode_image(&DynamicImage::ImageRgb8(img))
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn resizes_to_the_geometry_dimensions() {
        let source = gradient_jpeg(200, 100);
        let out = resize_jpeg(&source, 25).unwrap();
        assert_eq!(out.dimensions, Dimensions::new(100, 50));
        let decoded = image::load_from_memory_with_format(&out.bytes, ImageFormat::Jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (100, 50));
    }

    #[test]
    fn garbage_is_a_decode_error() {
        let err = resize_jpeg(b"not a jpeg at all", 50).unwrap_err();
        assert!(matches!(err, ResizeError::Decode(_)));
    }
}
