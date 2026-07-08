use crate::error::LoadError;
use std::path::PathBuf;

/// The result of decoding a panorama on a background thread. The raw RGBA
/// pixels are sent across the channel (owned `Vec<u8>`, no extra copy on
/// transfer); the GPU upload happens back on the main thread.
pub enum LoadResult {
    Ok {
        path: PathBuf,
        rgba: Vec<u8>,
        width: u32,
        height: u32,
        warning: Option<String>,
    },
    Err {
        path: PathBuf,
        error: LoadError,
    },
}

/// Read the file from disk and decode it. Runs on a worker thread so the
/// main event loop can keep redrawing (and the spinner can animate) while
/// the user waits for a large image.
pub fn decode_blocking(path: PathBuf) -> LoadResult {
    match crate::file::load_panorama(&path) {
        Ok(image) => {
            let warning = crate::file::aspect_ratio_warning(&image);
            let rgba_img = image.to_rgba8();
            let (width, height) = rgba_img.dimensions();
            let rgba = rgba_img.into_raw();
            LoadResult::Ok {
                path,
                rgba,
                width,
                height,
                warning,
            }
        }
        Err(error) => LoadResult::Err { path, error },
    }
}
