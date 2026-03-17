use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Instant;
use tracing_subscriber::FmtSubscriber;
use viewer_core::{Camera, Scene};
use viewer_render::RenderBackend;
use viewer_ui::UiSystem;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
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
    renderer: RenderBackend,
    ui: UiSystem,
    camera: Camera,
    scene: Scene,
    input: InputState,
    last_frame_time: Instant,
}

#[derive(Default)]
struct InputState {
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    mouse_look_active: bool,
    pending_look_delta: [f32; 2],
    last_cursor_pos: Option<PhysicalPosition<f64>>,
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

    fn init_state(event_loop: &ActiveEventLoop) -> Result<AppState> {
        let window = Self::init_window(event_loop)?;
        let renderer = RenderBackend::new(window.clone())?;
        let ui = UiSystem::new(&window, renderer.device(), renderer.surface_format());

        Ok(AppState {
            window,
            renderer,
            ui,
            camera: Camera::default(),
            scene: Scene::prototype(),
            input: InputState::default(),
            last_frame_time: Instant::now(),
        })
    }
}

impl AppState {
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    fn redraw(&mut self) -> Result<()> {
        let now = Instant::now();
        let dt_seconds = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        let [look_x, look_y] = self.input.take_look_delta();
        self.camera.add_look_delta(look_x, look_y);
        self.input.update_camera(&mut self.camera, dt_seconds);

        let window = self.window.clone();
        let ui = &mut self.ui;
        let camera = self.camera;

        self.renderer.render_frame(
            &camera,
            &self.scene,
            |device, queue, encoder, target_view, surface_size| {
                ui.render(
                    &window,
                    device,
                    queue,
                    encoder,
                    target_view,
                    surface_size,
                    &camera,
                );
            },
        )
    }

    fn handle_input_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.input.handle_key_event(event);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input.handle_mouse_button(*button, *state);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(*position);
            }
            _ => {}
        }
    }
}

impl InputState {
    fn handle_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;

        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };

        match code {
            KeyCode::KeyW => self.move_forward = pressed,
            KeyCode::KeyS => self.move_backward = pressed,
            KeyCode::KeyA => self.move_left = pressed,
            KeyCode::KeyD => self.move_right = pressed,
            KeyCode::KeyE => self.move_up = pressed,
            KeyCode::KeyQ => self.move_down = pressed,
            _ => {}
        }
    }

    fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if button != MouseButton::Right {
            return;
        }

        self.mouse_look_active = state == ElementState::Pressed;
        self.last_cursor_pos = None;
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        if !self.mouse_look_active {
            self.last_cursor_pos = Some(position);
            return;
        }

        if let Some(last) = self.last_cursor_pos {
            let sensitivity = 0.0025;
            let dx = (position.x - last.x) as f32;
            let dy = (position.y - last.y) as f32;

            self.pending_look_delta[0] += -dx * sensitivity;
            self.pending_look_delta[1] += -dy * sensitivity;
        }

        self.last_cursor_pos = Some(position);
    }

    fn take_look_delta(&mut self) -> [f32; 2] {
        let delta = self.pending_look_delta;
        self.pending_look_delta = [0.0, 0.0];
        delta
    }

    fn update_camera(&self, camera: &mut Camera, dt_seconds: f32) {
        let forward =
            ((self.move_forward as i8 - self.move_backward as i8) as f32) * 3.5 * dt_seconds;
        let right = ((self.move_right as i8 - self.move_left as i8) as f32) * 3.5 * dt_seconds;
        let up = ((self.move_up as i8 - self.move_down as i8) as f32) * 3.5 * dt_seconds;

        camera.move_local(forward, right, up);
    }
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            match Self::init_state(event_loop) {
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

        let ui_consumed = state.ui.handle_event(&state.window, &event);
        if ui_consumed {
            state.window.request_redraw();
            return;
        }

        state.handle_input_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.window.request_redraw();
            }
            WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. } => {
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
