use crate::error::{BenchError, BenchResult};
use image::{GenericImageView, Rgba, RgbaImage};

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub total_pixels: u64,
    pub diff_pixels: u64,
    pub diff_ratio: f64,
    pub diff_image: Option<RgbaImage>,
}

pub fn compare_images(
    baseline_bytes: &[u8],
    actual_bytes: &[u8],
    threshold: u8,
    generate_diff_image: bool,
) -> BenchResult<DiffResult> {
    let img1 = image::load_from_memory(baseline_bytes)?;
    let img2 = image::load_from_memory(actual_bytes)?;

    let (w1, h1) = img1.dimensions();
    let (w2, h2) = img2.dimensions();

    if w1 != w2 || h1 != h2 {
        return Err(BenchError::Diff(format!(
            "Dimension mismatch: baseline is {}x{}, actual is {}x{}",
            w1, h1, w2, h2
        )));
    }

    let total_pixels = (w1 as u64) * (h1 as u64);
    let mut diff_pixels = 0u64;

    let mut diff_img = if generate_diff_image {
        Some(RgbaImage::new(w1, h1))
    } else {
        None
    };

    for y in 0..h1 {
        for x in 0..w1 {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let diff_r = (p1[0] as i16 - p2[0] as i16).abs() as u8;
            let diff_g = (p1[1] as i16 - p2[1] as i16).abs() as u8;
            let diff_b = (p1[2] as i16 - p2[2] as i16).abs() as u8;
            let diff_a = (p1[3] as i16 - p2[3] as i16).abs() as u8;

            let is_diff = diff_r > threshold || diff_g > threshold || diff_b > threshold || diff_a > threshold;

            if is_diff {
                diff_pixels += 1;
                if let Some(ref mut d_img) = diff_img {
                    d_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                }
            } else if let Some(ref mut d_img) = diff_img {
                d_img.put_pixel(x, y, Rgba([p1[0] / 3, p1[1] / 3, p1[2] / 3, 255]));
            }
        }
    }

    let diff_ratio = if total_pixels > 0 {
        diff_pixels as f64 / total_pixels as f64
    } else {
        0.0
    };

    Ok(DiffResult {
        total_pixels,
        diff_pixels,
        diff_ratio,
        diff_image: diff_img,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::png::PngEncoder;
    use image::ImageEncoder;

    fn encode_png(img: &RgbaImage) -> Vec<u8> {
        let mut buf = Vec::new();
        let encoder = PngEncoder::new(&mut buf);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        buf
    }

    #[test]
    fn test_identical_images() {
        let mut img = RgbaImage::new(2, 2);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([100, 150, 200, 255]);
        }
        let bytes = encode_png(&img);

        let result = compare_images(&bytes, &bytes, 0, true).unwrap();
        assert_eq!(result.total_pixels, 4);
        assert_eq!(result.diff_pixels, 0);
        assert_eq!(result.diff_ratio, 0.0);
    }

    #[test]
    fn test_different_images() {
        let mut img1 = RgbaImage::new(2, 2);
        let mut img2 = RgbaImage::new(2, 2);

        for pixel in img1.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }
        for pixel in img2.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        // Change 1 pixel in img2
        img2.put_pixel(0, 0, Rgba([255, 255, 255, 255]));

        let b1 = encode_png(&img1);
        let b2 = encode_png(&img2);

        let result = compare_images(&b1, &b2, 10, true).unwrap();
        assert_eq!(result.total_pixels, 4);
        assert_eq!(result.diff_pixels, 1);
        assert!((result.diff_ratio - 0.25).abs() < 1e-6);
        assert!(result.diff_image.is_some());
    }

    #[test]
    fn test_dimension_mismatch() {
        let img1 = RgbaImage::new(2, 2);
        let img2 = RgbaImage::new(4, 4);

        let b1 = encode_png(&img1);
        let b2 = encode_png(&img2);

        let res = compare_images(&b1, &b2, 0, false);
        assert!(res.is_err());
    }
}
