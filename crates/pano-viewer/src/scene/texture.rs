use image::{imageops::FilterType, DynamicImage, RgbaImage};

#[allow(dead_code)]
pub struct PanoramaTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl PanoramaTexture {
    /// Build a GPU texture from already-decoded RGBA8 bytes. The image is
    /// downscaled to fit within the device's `max_texture_dimension_2d`,
    /// preserving aspect ratio (see `texture.rs` history for the prior panic
    /// this avoids).
    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba_in: Vec<u8>,
        width_in: u32,
        height_in: u32,
    ) -> Self {
        let (rgba, width, height) = fit_to_device(
            rgba_in,
            width_in,
            height_in,
            device.limits().max_texture_dimension_2d,
        );
        upload_rgba(device, queue, &rgba, width, height)
    }
}

fn fit_to_device(rgba_in: Vec<u8>, w0: u32, h0: u32, max_dim: u32) -> (Vec<u8>, u32, u32) {
    if w0 <= max_dim && h0 <= max_dim {
        return (rgba_in, w0, h0);
    }
    let scale = max_dim as f32 / w0.max(h0) as f32;
    let new_w = ((w0 as f32) * scale).round().max(1.0) as u32;
    let new_h = ((h0 as f32) * scale).round().max(1.0) as u32;
    log::warn!(
        "panorama image {w0}x{h0} exceeds device max texture dimension {max_dim}; \
         downscaling to {new_w}x{new_h}"
    );
    let img = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(w0, h0, rgba_in).expect("rgba byte length matches dimensions"),
    );
    let resized = img.resize_exact(new_w, new_h, FilterType::Triangle);
    let buf = resized.into_rgba8();
    let dims = buf.dimensions();
    (buf.into_raw(), dims.0, dims.1)
}

fn upload_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> PanoramaTexture {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("panorama_texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("panorama_sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    PanoramaTexture {
        texture,
        view,
        sampler,
        width,
        height,
    }
}
