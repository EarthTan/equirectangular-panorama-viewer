use crate::error::AppError;
use std::sync::Arc;
use winit::window::WindowAttributes;

/// Clamp a window's physical pixel size to the GPU adapter's maximum
/// supported texture dimension. wgpu rejects `Surface::configure` when
/// either dimension exceeds `adapter.limits().max_texture_dimension_2d`,
/// which can happen on HiDPI displays (e.g. 1280×800 logical @ 2x scale
/// = 2560×1600) or on adapters with low texture limits. This is a pure
/// function so it can be unit-tested without winit/wgpu.
fn clamp_surface_size(
    size: winit::dpi::PhysicalSize<u32>,
    max_extent: u32,
) -> winit::dpi::PhysicalSize<u32> {
    winit::dpi::PhysicalSize::new(
        size.width.min(max_extent).max(1),
        size.height.min(max_extent).max(1),
    )
}

pub struct WindowState {
    pub window: Arc<winit::window::Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl WindowState {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Result<Self, AppError> {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("360° 全景图查看器")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
                )
                .map_err(AppError::Window)?,
        );

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(Arc::clone(&window))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| AppError::RequestAdapter(format!("{e}")))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("pano-viewer_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            }))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let size = window.inner_size();
        let max_extent = adapter.limits().max_texture_dimension_2d;
        let size = clamp_surface_size(size, max_extent);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        let max_extent = self.device.limits().max_texture_dimension_2d;
        let clamped = clamp_surface_size(new_size, max_extent);
        self.config.width = clamped.width;
        self.config.height = clamped.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalSize;

    #[test]
    fn clamp_under_max_is_identity() {
        let s = PhysicalSize::new(1920, 1080);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 1920);
        assert_eq!(out.height, 1080);
    }

    #[test]
    fn clamp_over_max_caps_both_axes() {
        // The actual bug case: 1280x800 logical @ 2x Retina on a max=2048 adapter.
        // Per-axis min caps width to 2048; height (1600) is within budget and is
        // preserved unchanged. Aspect shifts from 1.6 to 1.28 in this corner case.
        let s = PhysicalSize::new(2560, 1600);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 1600);
    }

    #[test]
    fn clamp_caps_only_offending_axis_when_one_axis_over() {
        // Only width overflows; height (1024) is within budget and preserved.
        let s = PhysicalSize::new(4096, 1024);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 1024);
    }

    #[test]
    fn clamp_zero_returns_one() {
        let s = PhysicalSize::new(0, 600);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 1);
        assert_eq!(out.height, 600);
    }

    #[test]
    fn clamp_with_max_zero_returns_one() {
        let s = PhysicalSize::new(800, 600);
        let out = clamp_surface_size(s, 0);
        assert_eq!(out.width, 1);
        assert_eq!(out.height, 1);
    }

    #[test]
    fn clamp_exact_max_is_identity() {
        let s = PhysicalSize::new(2048, 2048);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 2048);
    }
}
