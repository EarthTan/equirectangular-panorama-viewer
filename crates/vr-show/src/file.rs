use crate::error::LoadError;
use image::DynamicImage;
use std::path::Path;

pub const ASPECT_TOLERANCE: f32 = 0.05;
pub const TARGET_ASPECT: f32 = 2.0;

pub fn load_panorama(path: &Path) -> Result<DynamicImage, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io(path.to_path_buf(), e))?;
    let cursor = std::io::Cursor::new(bytes);
    let reader = image::ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|e| LoadError::Io(path.to_path_buf(), e))?;
    let format = reader.format();
    let img = reader.decode().map_err(|source| LoadError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    if !matches!(
        format,
        Some(image::ImageFormat::Png) | Some(image::ImageFormat::Jpeg)
    ) {
        return Err(LoadError::NotAnImage(path.to_path_buf()));
    }
    Ok(img)
}

/// Returns Some(warning_message) if the aspect ratio deviates from 2:1 by more
/// than `ASPECT_TOLERANCE`. Returns None if the image is acceptably panoramic.
pub fn aspect_ratio_warning(image: &DynamicImage) -> Option<String> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    let ratio = w / h;
    if (ratio - TARGET_ASPECT).abs() > ASPECT_TOLERANCE {
        Some(format!(
            "Image aspect ratio is {w}:{h}, not 2:1. It will display stretched."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};

    fn tmp_dir() -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("vr-show-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        let mut img: RgbImage = ImageBuffer::new(w, h);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([128, 64, 32]);
        }
        img.save(path).unwrap();
    }

    #[test]
    fn load_panorama_png_succeeds() {
        let dir = tmp_dir();
        let path = dir.join("pano.png");
        write_png(&path, 2048, 1024);
        let img = load_panorama(&path).unwrap();
        assert_eq!(img.width(), 2048);
        assert_eq!(img.height(), 1024);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_nonexistent_returns_io_error() {
        let result = load_panorama(Path::new("/nonexistent/path/file.png"));
        assert!(matches!(result, Err(LoadError::Io(_, _))));
    }

    #[test]
    fn aspect_warning_emitted_for_non_2to1() {
        let img = DynamicImage::new_rgb8(1024, 1024);
        let msg = aspect_ratio_warning(&img);
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("1024:1024"));
    }

    #[test]
    fn aspect_warning_silent_for_2to1() {
        let img = DynamicImage::new_rgb8(2048, 1024);
        assert!(aspect_ratio_warning(&img).is_none());
    }

    #[test]
    fn aspect_warning_silent_within_tolerance() {
        // 2016x1024 is within 5% of 2:1.
        let img = DynamicImage::new_rgb8(2016, 1024);
        assert!(aspect_ratio_warning(&img).is_none());
    }
}
