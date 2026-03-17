use anyhow::{Context, Result};
use egui_wgpu::wgpu;
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use std::sync::Arc;
use tracing_subscriber::FmtSubscriber;
use wgpu::{
    CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor,
    Limits, LoadOp, Operations, PowerPreference, PresentMode, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration,
    SurfaceError, TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder().finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let event_loop = EventLoop::new()?;
    let mut app = ViewerApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct ViewerApp {
    state: Option<AppState>,
}

struct AppState {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,

    egui_ctx: egui::Context,
    egui_state: EguiWinitState,
    egui_renderer: EguiRenderer,
}

impl ViewerApp {
    fn init_window(event_loop: &ActiveEventLoop) -> Result<Arc<Window>> {
        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("SL Viewer Rewrite")
                    .with_inner_size(PhysicalSize::new(1280, 720)),
            )
            .context("failed to create window")?;

        Ok(Arc::new(window))
    }

    fn init_wgpu(window: Arc<Window>) -> Result<AppState> {
        let size = window.inner_size();

        let instance = Instance::new(&InstanceDescriptor::default());

        let surface = instance
            .create_surface(window.clone())
            .context("failed to create wgpu surface")?;

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .context("failed to find a suitable GPU adapter")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("viewer_device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .context("failed to create device")?;

        let surface_caps = surface.get_capabilities(&adapter);

        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = if surface_caps.present_modes.contains(&PresentMode::Mailbox) {
            PresentMode::Mailbox
        } else {
            PresentMode::Fifo
        };

        let alpha_mode = surface_caps.alpha_modes[0];

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        let egui_state = EguiWinitState::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        // No depth buffer and single-sample target are enough for the initial UI-only slice.
        let egui_renderer = EguiRenderer::new(&device, format, None, 1, false);

        Ok(AppState {
            window,
            surface,
            device,
            queue,
            config,
            egui_ctx,
            egui_state,
            egui_renderer,
        })
    }
}

impl AppState {
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn redraw(&mut self) -> Result<()> {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(SurfaceError::OutOfMemory) => anyhow::bail!("out of GPU memory"),
            Err(SurfaceError::Timeout) => return Ok(()),
            Err(SurfaceError::Other) => return Ok(()),
        };

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let raw_input = self.egui_state.take_egui_input(&self.window);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.heading("SL Viewer Rewrite");
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("First vertical slice is alive.");
                ui.separator();
                ui.label(format!(
                    "Surface: {}x{}",
                    self.config.width, self.config.height
                ));
            });
        });

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, self.window.scale_factor() as f32);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.09,
                            b: 0.11,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let mut render_pass = render_pass.forget_lifetime();

            self.egui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            match Self::init_window(event_loop).and_then(Self::init_wgpu) {
                Ok(state) => self.state = Some(state),
                Err(err) => {
                    eprintln!("startup failed: {err:#}");
                    event_loop.exit();
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if state.window.id() != window_id {
            return;
        }

        let egui_consumed = state
            .egui_state
            .on_window_event(&state.window, &event)
            .consumed;

        if egui_consumed {
            state.window.request_redraw();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(err) = state.redraw() {
                    eprintln!("redraw failed: {err:#}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}
