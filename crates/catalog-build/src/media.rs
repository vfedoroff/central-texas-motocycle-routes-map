use anyhow::{Context, Result, bail};
use catalog_model::{OutputPhoto, Photo, PhotoVariant};
use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use sha2::{Digest, Sha256};
use std::io::Cursor;

pub const MAX_INPUT_BYTES: usize = 20 * 1024 * 1024; // 20 MB
pub const MAX_PIXELS: u64 = 40_000_000; // 40 million decoded pixels

#[derive(Clone, Debug)]
pub struct GeneratedVariant {
    pub variant: PhotoVariant,
    pub bytes: Vec<u8>,
    pub filename: String,
}

#[derive(Clone, Debug)]
pub struct ProcessedMedia {
    pub output_photo: OutputPhoto,
    pub variants: Vec<GeneratedVariant>,
}

fn has_transparency(img: &DynamicImage) -> bool {
    match img {
        DynamicImage::ImageRgba8(rgba) => rgba.pixels().any(|p| p[3] < 255),
        DynamicImage::ImageRgba16(rgba) => rgba.pixels().any(|p| p[3] < 65535),
        _ => false,
    }
}

/// Process a single source image file into responsive variants.
/// - Validates input <= 20 MB and <= 40M decoded pixels.
/// - Accepts only JPEG and PNG.
/// - Encodes metadata-free JPEG (or PNG if transparent).
/// - Creates variants at 480, 960, 1600 width below source width, plus source width variant.
/// - Never upscales.
pub fn process_image(photo: &Photo, source_bytes: &[u8]) -> Result<ProcessedMedia> {
    if source_bytes.len() > MAX_INPUT_BYTES {
        bail!(
            "Image '{}' exceeds maximum input size of 20 MB (got {} bytes)",
            photo.path,
            source_bytes.len()
        );
    }

    let format = match image::guess_format(source_bytes) {
        Ok(ImageFormat::Jpeg) => ImageFormat::Jpeg,
        Ok(ImageFormat::Png) => ImageFormat::Png,
        Ok(other) => bail!(
            "Unsupported image format '{:?}' in '{}'. Only JPEG and PNG are supported in version 1.",
            other,
            photo.path
        ),
        Err(_) => bail!("Could not determine image format for '{}'", photo.path),
    };

    let mut reader = ImageReader::new(Cursor::new(source_bytes));
    reader.set_format(format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(10_000);
    limits.max_image_height = Some(10_000);
    limits.max_alloc = Some(150_000_000);
    reader.limits(limits);

    let decoded = reader
        .decode()
        .with_context(|| format!("Failed to decode image '{}'", photo.path))?;

    let (orig_width, orig_height) = (decoded.width(), decoded.height());
    if (orig_width as u64) * (orig_height as u64) > MAX_PIXELS {
        bail!(
            "Image '{}' exceeds maximum limit of 40 million pixels ({}x{})",
            photo.path,
            orig_width,
            orig_height
        );
    }

    let is_transparent = has_transparency(&decoded);
    let ext = if is_transparent { "png" } else { "jpg" };

    let candidate_widths = [480, 960, 1600];
    let mut target_widths = Vec::new();
    for &w in &candidate_widths {
        if w < orig_width {
            target_widths.push(w);
        }
    }
    if !target_widths.contains(&orig_width) {
        target_widths.push(orig_width);
    }
    target_widths.sort_unstable();

    let mut generated_variants = Vec::new();
    let mut photo_variants = Vec::new();

    for width in target_widths {
        let (resized_img, actual_w, actual_h) = if width == orig_width {
            (decoded.clone(), orig_width, orig_height)
        } else {
            let resized = decoded.resize(width, u32::MAX, image::imageops::FilterType::Lanczos3);
            let w = resized.width();
            let h = resized.height();
            (resized, w, h)
        };

        let mut encoded_bytes = Vec::new();
        if is_transparent {
            resized_img.write_to(&mut Cursor::new(&mut encoded_bytes), ImageFormat::Png)?;
        } else {
            let rgb = resized_img.to_rgb8();
            rgb.write_to(&mut Cursor::new(&mut encoded_bytes), ImageFormat::Jpeg)?;
        }

        let mut hasher = Sha256::new();
        hasher.update(&encoded_bytes);
        let hash = hasher.finalize();
        let mut hash_prefix = String::with_capacity(12);
        for b in &hash[..6] {
            use std::fmt::Write;
            let _ = write!(hash_prefix, "{:02x}", b);
        }

        let filename = format!("{}_{}x{}.{}", hash_prefix, actual_w, actual_h, ext);
        let url = format!("/media/{}", filename);

        let pv = PhotoVariant {
            url,
            width: actual_w,
            height: actual_h,
        };

        photo_variants.push(pv.clone());
        generated_variants.push(GeneratedVariant {
            variant: pv,
            bytes: encoded_bytes,
            filename,
        });
    }

    let output_photo = OutputPhoto {
        alt: photo.alt.clone(),
        credit: photo.credit.clone(),
        rights: photo.rights.clone(),
        variants: photo_variants,
    };

    Ok(ProcessedMedia {
        output_photo,
        variants: generated_variants,
    })
}
