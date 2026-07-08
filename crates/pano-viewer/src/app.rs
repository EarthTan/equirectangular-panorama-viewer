use crate::error::AppError;
use crate::input::InputAction;
use crate::loader::{decode_blocking, LoadResult};
use crate::renderer::Renderer;
use crate::scene::camera::CameraState;
use crate::window::WindowState;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

pub struct App {
    window_state: Option<WindowState>,
    renderer: Option<Renderer>,
    camera: CameraState,
    dragging: bool,
    last_pointer: Option<PhysicalPosition<f64>>,
    last_frame: Option<std::time::Instant>,
    pub panorama: Option<crate::scene::texture::PanoramaTexture>,
    pending_load: Option<PathBuf>,
    // Async panorama loading. `loading` is shown in the UI so the user sees a
    // spinner while a background thread reads + decodes the image.
    loading: Option<PathBuf>,
    load_receiver: Option<Receiver<LoadResult>>,
    load_thread: Option<std::thread::JoinHandle<()>>,
    // egui state
    egui_ctx: egui::Context,
    egui_renderer: Option<egui_wgpu::Renderer>,
    egui_winit: Option<egui_winit::State>,
    ui_state: crate::ui::UiState,
}

impl App {
    pub fn new() -> Result<Self, AppError> {
        let egui_ctx = egui::Context::default();
        crate::ui::install_fonts(&egui_ctx);
        Ok(Self {
            window_state: None,
            renderer: None,
            camera: CameraState::default(),
            dragging: false,
            last_pointer: None,
            last_frame: None,
            panorama: None,
            pending_load: None,
            loading: None,
            load_receiver: None,
            load_thread: None,
            egui_ctx: egui_ctx.clone(),
            egui_renderer: None,
            egui_winit: None,
            ui_state: crate::ui::UiState::default(),
        })
    }

    pub fn set_pending_load(&mut self, path: PathBuf) {
        self.pending_load = Some(path);
    }

    /// Open a native file picker and load the chosen image.
    /// Uses rfd's blocking API — the event loop is paused while the dialog
    /// is open, which is acceptable for a simple viewer.
    pub fn pick_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg"])
            .pick_file()
        {
            self.start_async_load(path);
        }
    }

    /// Kick off a background decode of `path`. Any in-flight load is
    /// cancelled by dropping its channel (its thread's `send` will fail
    /// silently and the result is discarded). The UI is told to show a
    /// spinner; the main loop keeps redrawing while the worker runs.
    pub fn start_async_load(&mut self, path: PathBuf) {
        // Drop any previous load. The receiver going away causes the prior
        // thread's `send` to return Err; the thread itself keeps running to
        // completion but its result is silently discarded.
        self.load_receiver = None;
        self.load_thread = None;

        let (tx, rx) = channel::<LoadResult>();
        let path_for_thread = path.clone();
        let thread = std::thread::Builder::new()
            .name("pano-decode".into())
            .spawn(move || {
                let result = decode_blocking(path_for_thread);
                // If the receiver was dropped (cancelled), this fails and
                // the result is discarded — exactly what we want.
                let _ = tx.send(result);
            })
            .expect("failed to spawn decode thread");

        self.load_receiver = Some(rx);
        self.load_thread = Some(thread);
        self.ui_state.begin_loading(path.clone());
        self.loading = Some(path);

        // Wake the renderer so the spinner appears on the next frame.
        if let Some(ws) = &self.window_state {
            ws.window.request_redraw();
        }
    }

    /// Poll the load channel. If a worker has finished, perform the GPU
    /// upload + renderer rebuild on the main thread and clear the spinner.
    fn poll_load_result(&mut self) {
        let Some(rx) = &self.load_receiver else {
            return;
        };
        // Non-blocking try_recv — we don't want to stall the render loop on
        // an in-progress decode. A successful load wakes us via the next
        // request_redraw (see `start_async_load`); a still-running load
        // produces no event here and the spinner keeps animating.
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // Sender dropped without sending (shouldn't normally happen
                // because the thread always sends, even on cancellation).
                self.load_receiver = None;
                self.load_thread = None;
                self.loading = None;
                self.ui_state.clear_loading();
                return;
            }
        };
        // The load is complete; clear thread bookkeeping.
        self.load_receiver = None;
        self.load_thread = None;

        match result {
            LoadResult::Ok {
                path,
                rgba,
                width,
                height,
                warning,
            } => {
                if let Some(ws) = &self.window_state {
                    if let Some(warning) = warning {
                        log::warn!("{warning}");
                    }
                    let texture = crate::scene::texture::PanoramaTexture::from_rgba(
                        &ws.device, &ws.queue, rgba, width, height,
                    );
                    let renderer = Renderer::new(&ws.device, ws.surface_format(), Some(&texture));
                    self.renderer = Some(renderer);
                    self.panorama = Some(texture);
                    self.camera = CameraState::default();
                    log::info!("loaded panorama: {width}x{height} from {}", path.display());
                }
                self.loading = None;
                self.ui_state.clear_loading();
                self.ui_state.show_panorama_loaded();
            }
            LoadResult::Err { path, error } => {
                let msg = crate::ui::UiState::error_for_load_error(&error);
                self.ui_state.show_error(msg, 3000);
                log::error!("failed to load panorama {}: {error}", path.display());
                self.loading = None;
                self.ui_state.clear_loading();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_state.is_some() {
            return;
        }
        match WindowState::new(event_loop) {
            Ok(ws) => {
                self.egui_renderer = Some(egui_wgpu::Renderer::new(
                    &ws.device,
                    ws.surface_format(),
                    egui_wgpu::RendererOptions {
                        depth_stencil_format: None,
                        msaa_samples: 1,
                        ..Default::default()
                    },
                ));
                self.egui_winit = Some(egui_winit::State::new(
                    self.egui_ctx.clone(),
                    egui::viewport::ViewportId::ROOT,
                    &ws.window,
                    None,
                    None,
                    None,
                ));

                let renderer = Renderer::new(&ws.device, ws.surface_format(), None);
                self.window_state = Some(ws);
                self.renderer = Some(renderer);
                if let Some(p) = self.pending_load.take() {
                    self.start_async_load(p);
                }
            }
            Err(e) => log::error!("Failed to create window: {e}"),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Forward to egui first.
        if let (Some(ws), Some(egui_winit)) = (&self.window_state, &mut self.egui_winit) {
            let response = egui_winit.on_window_event(&ws.window, &event);
            if response.consumed {
                return;
            }
        }

        // Pointer tracking.
        match &event {
            WindowEvent::CursorMoved { position, .. } if self.dragging => {
                if let Some(prev) = self.last_pointer {
                    let dx = (position.x - prev.x) as f32;
                    let dy = (position.y - prev.y) as f32;
                    if dx != 0.0 || dy != 0.0 {
                        self.camera.apply_drag(dx, dy);
                        if self.camera.fire_first_interaction() {
                            log::info!("first interaction: drag");
                        }
                    }
                }
                self.last_pointer = Some(*position);
            }
            WindowEvent::MouseInput { state, button, .. } if *button == MouseButton::Left => {
                match state {
                    ElementState::Pressed => {
                        self.dragging = true;
                        self.last_pointer = None;
                    }
                    ElementState::Released => {
                        self.dragging = false;
                        self.last_pointer = None;
                    }
                }
            }
            _ => {}
        }

        // Typed events.
        for action in crate::input::translate(&event) {
            match action {
                InputAction::CloseRequested => event_loop.exit(),
                InputAction::Resize { width, height } => {
                    if let Some(ws) = &mut self.window_state {
                        ws.resize(winit::dpi::PhysicalSize::new(width, height));
                    }
                }
                InputAction::Wheel { delta_y } => {
                    if self.camera.apply_wheel(delta_y) {
                        log::info!("first interaction: wheel");
                    }
                }
                InputAction::FilesDropped(paths) => {
                    if let Some(p) = paths.into_iter().next() {
                        self.start_async_load(p);
                    }
                }
                InputAction::FirstInteractionTriggered => {}
                InputAction::Drag { .. } => unreachable!("drag handled above"),
            }
        }

        // Render.
        if matches!(event, WindowEvent::RedrawRequested) {
            self.render_frame();
        }
    }
}

impl App {
    fn render_frame(&mut self) {
        // Drain any completed background decode before drawing so the new
        // panorama shows up on this frame instead of one frame later.
        self.poll_load_result();

        let Some(ws) = &mut self.window_state else {
            return;
        };
        let Some(renderer) = &self.renderer else {
            return;
        };

        // Update camera state.
        let now = std::time::Instant::now();
        let dt = self
            .last_frame
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(0.016)
            .min(0.1);
        self.last_frame = Some(now);
        self.camera.update(dt);
        renderer.update_camera(&ws.queue, &self.camera, ws.aspect());

        // Acquire frame.
        let frame = match ws.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                let size = ws.window.inner_size();
                ws.resize(size);
                return;
            }
            Err(e) => {
                log::error!("Surface error: {e}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = ws
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // Render 3D scene.
        renderer.render(&mut encoder, &view);

        // Render egui UI.
        let Some(egui_winit) = &mut self.egui_winit else {
            return;
        };
        let Some(egui_renderer) = &mut self.egui_renderer else {
            return;
        };
        let raw_input = egui_winit.take_egui_input(&ws.window);
        self.egui_ctx.begin_pass(raw_input);
        let ui_out = crate::ui::draw(&self.egui_ctx, &self.ui_state);
        let output = self.egui_ctx.end_pass();
        let paint_jobs = self
            .egui_ctx
            .tessellate(output.shapes, output.pixels_per_point);
        for (id, image_delta) in &output.textures_delta.set {
            egui_renderer.update_texture(&ws.device, &ws.queue, *id, image_delta);
        }
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [ws.config.width, ws.config.height],
            pixels_per_point: ws.window.scale_factor() as f32,
        };
        let extra_cmds = egui_renderer.update_buffers(
            &ws.device,
            &ws.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        // Submit 3D + egui command buffers.
        ws.queue
            .submit(std::iter::once(encoder.finish()).chain(extra_cmds));

        // Second render pass for egui (on the frame's texture view).
        let mut egui_encoder = ws
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui_encoder"),
            });
        {
            let rpass = egui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            egui_renderer.render(
                &mut wgpu::RenderPass::forget_lifetime(rpass),
                &paint_jobs,
                &screen_descriptor,
            );
        }
        ws.queue.submit(std::iter::once(egui_encoder.finish()));
        frame.present();
        ws.window.request_redraw();

        // Handle UI output after all borrows of `ws` are released.
        if ui_out.open_file_picker {
            self.pick_file();
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
            .expect("App::new only fails on winit::error::OsError, which is not normally reachable")
    }
}
