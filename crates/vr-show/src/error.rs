use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("window creation failed: {0}")]
    Window(#[from] winit::error::OsError),

    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("GPU error: {0}")]
    Gpu(#[from] wgpu::Error),

    #[error("GPU request adapter failed: {0}")]
    RequestAdapter(String),

    #[error("GPU request device failed: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("surface error: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),

    #[error("surface lost")]
    SurfaceLost,
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("not an image file: {0}")]
    NotAnImage(PathBuf),

    #[error("decode failed for {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("io error reading {0}")]
    Io(PathBuf, #[source] std::io::Error),
}
