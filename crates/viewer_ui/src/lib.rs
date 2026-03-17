use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use viewer_core::Camera;
use wgpu::{
    CommandEncoder, Device, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

/// Thin wrapper for all egui pieces that the viewer exposes.
pub struct UiSystem {
    egui_ctx: egui::Context,
    egui_state: State,
    egui_renderer: Renderer,
}

impl UiSystem {
    /// Builds the UI stack that will drive input handling and rendering.
    pub fn new(window: &Window, device: &Device, surface_format: TextureFormat) -> Self {
        let egui_ctx = egui::Context::default();
        let egui_state = State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = Renderer::new(device, surface_format, None, 1, false);

        Self {
            egui_ctx,
            egui_state,
            egui_renderer,
        }
    }

    /// Forwards window events to egui and returns whether the event was consumed.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    /// Draws the persistent overlay that existed in the prior viewer_app implementation.
    /// The caller owns the encoder and render target so that it can merge this draw call
    /// with other renderer work.
    pub fn render(
        &mut self,
        window: &Window,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        target_view: &TextureView,
        surface_size: PhysicalSize<u32>,
        camera: &Camera,
    ) {
        if surface_size.width == 0 || surface_size.height == 0 {
            return;
        }

        let raw_input = self.egui_state.take_egui_input(window);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
                ui.heading("SL Viewer Rewrite");
            });

            egui::Window::new("Debug")
                .default_pos(egui::pos2(16.0, 48.0))
                .resizable(false)
                .show(ctx, |ui| {
                ui.label("First vertical slice is alive.");
                ui.separator();
                ui.label(format!(
                    "Surface: {}x{}",
                    surface_size.width, surface_size.height
                ));
                ui.separator();
                ui.label(format!(
                    "Camera pos: x={:.2} y={:.2} z={:.2}",
                    camera.position[0], camera.position[1], camera.position[2]
                ));
                ui.label(format!(
                    "Camera yaw/pitch: {:.2} / {:.2} rad",
                    camera.yaw, camera.pitch
                ));
            });
        });

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let pixels_per_point = window.scale_factor() as f32;
        let paint_jobs = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [surface_size.width, surface_size.height],
            pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.egui_renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        {
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("viewer_ui_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
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
    }
}
