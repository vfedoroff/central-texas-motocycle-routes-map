use anyhow::Result;
use catalog_build::{MAX_INPUT_BYTES, process_image};
use catalog_model::Photo;
use image::{ImageBuffer, ImageFormat, Rgb, Rgba};
use std::io::Cursor;

fn create_jpeg_bytes(width: u32, height: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width, height, Rgb([100, 150, 200]));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)
        .expect("encode jpeg");
    bytes
}

fn create_transparent_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(width, height, Rgba([200, 100, 50, 128]));
    let mut bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .expect("encode png");
    bytes
}

fn sample_photo(path: &str) -> Photo {
    Photo {
        path: path.to_string(),
        alt: "A scenic overlook over the canyon".to_string(),
        credit: "Jane Rider".to_string(),
        rights: "CC-BY-4.0".to_string(),
    }
}

#[test]
fn test_small_image_not_upscaled() -> Result<()> {
    let photo = sample_photo("content/media/small.jpg");
    let bytes = create_jpeg_bytes(300, 200);

    let processed = process_image(&photo, &bytes)?;

    // Source is 300px wide, so candidate widths 480, 960, 1600 are all > 300.
    // Only the source-width variant (300) should be produced.
    assert_eq!(processed.variants.len(), 1);
    let v = &processed.variants[0];
    assert_eq!(v.variant.width, 300);
    assert_eq!(v.variant.height, 200);
    assert!(v.filename.ends_with(".jpg"));

    // alt, credit, rights preserved
    assert_eq!(processed.output_photo.alt, photo.alt);
    assert_eq!(processed.output_photo.credit, photo.credit);
    assert_eq!(processed.output_photo.rights, photo.rights);

    Ok(())
}

#[test]
fn test_large_image_variants() -> Result<()> {
    let photo = sample_photo("content/media/large.jpg");
    let bytes = create_jpeg_bytes(2000, 1000);

    let processed = process_image(&photo, &bytes)?;

    // 2000 width should produce 480, 960, 1600, and 2000 (source width)
    assert_eq!(processed.variants.len(), 4);
    assert_eq!(processed.variants[0].variant.width, 480);
    assert_eq!(processed.variants[0].variant.height, 240);
    assert_eq!(processed.variants[1].variant.width, 960);
    assert_eq!(processed.variants[1].variant.height, 480);
    assert_eq!(processed.variants[2].variant.width, 1600);
    assert_eq!(processed.variants[2].variant.height, 800);
    assert_eq!(processed.variants[3].variant.width, 2000);
    assert_eq!(processed.variants[3].variant.height, 1000);

    Ok(())
}

#[test]
fn test_transparent_image_preserved_as_png() -> Result<()> {
    let photo = sample_photo("content/media/transparent.png");
    let bytes = create_transparent_png_bytes(500, 400);

    let processed = process_image(&photo, &bytes)?;

    // 500 width produces 480 and 500 variants
    assert_eq!(processed.variants.len(), 2);
    for v in &processed.variants {
        assert!(v.filename.ends_with(".png"));

        // Verify the output bytes decode as PNG and retain alpha
        let decoded = image::load_from_memory(&v.bytes)?;
        assert!(decoded.color().has_alpha());
    }

    Ok(())
}

#[test]
fn test_invalid_and_unsupported_formats_rejected() {
    let photo = sample_photo("content/media/bad.gif");

    // Random non-image bytes
    let err = process_image(&photo, b"not an image at all");
    assert!(err.is_err());

    // GIF bytes header
    let gif_bytes = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\xff\xff\xff\x00\x00\x00!\xf9\x04\x01\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
    let err_gif = process_image(&photo, gif_bytes);
    assert!(err_gif.is_err());
    assert!(
        err_gif
            .unwrap_err()
            .to_string()
            .contains("Only JPEG and PNG are supported")
    );
}

#[test]
fn test_oversized_input_bytes_rejected() {
    let photo = sample_photo("content/media/huge.jpg");
    let fake_oversized = vec![0u8; MAX_INPUT_BYTES + 1];

    let err = process_image(&photo, &fake_oversized);
    assert!(err.is_err());
    assert!(
        err.unwrap_err()
            .to_string()
            .contains("exceeds maximum input size of 20 MB")
    );
}

#[test]
fn test_source_metadata_absent_in_output() -> Result<()> {
    let photo = sample_photo("content/media/clean.jpg");
    let bytes = create_jpeg_bytes(400, 300);

    let processed = process_image(&photo, &bytes)?;
    let output_bytes = &processed.variants[0].bytes;

    // Check that output bytes start with standard JPEG SOI marker (0xFF, 0xD8)
    assert_eq!(output_bytes[0], 0xFF);
    assert_eq!(output_bytes[1], 0xD8);

    // Standard re-encoded output should not have EXIF APP1 header with "Exif\0\0"
    let contains_exif = output_bytes.windows(6).any(|window| window == b"Exif\0\0");
    assert!(!contains_exif);

    Ok(())
}
