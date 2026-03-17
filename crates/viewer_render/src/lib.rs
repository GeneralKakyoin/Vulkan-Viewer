use anyhow::{Context, Result};
use std::sync::Arc;
use viewer_core::Camera;
use wgpu::util::DeviceExt;
use wgpu::{
    Buffer, BufferUsages, ColorTargetState, CommandEncoder, CommandEncoderDescriptor,
    DeviceDescriptor, Features, FragmentState, Instance, InstanceDescriptor, Limits, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PowerPreference, PresentMode, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    ShaderSource, StoreOp, Surface, SurfaceConfiguration, SurfaceError, TextureUsages, TextureView,
    TextureViewDescriptor, VertexBufferLayout, VertexState, VertexStepMode,
};
use winit::{dpi::PhysicalSize, window::Window};

pub struct RenderBackend {
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    vertex_buffer: Buffer,
    vertex_count: u32,
}

const DEBUG_CLIP_SPACE_TRIANGLE: bool = false;

impl RenderBackend {
    pub fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(&InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
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

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform_buffer"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("triangle_shader"),
            source: ShaderSource::Wgsl(TRIANGLE_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("triangle_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("triangle_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                cull_mode: None,
                ..PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertices: [[f32; 6]; 3] = [
            [3.0, 0.5, 0.0, 1.0, 0.2, 0.2],
            [3.0, -0.5, -0.5, 0.2, 1.0, 0.2],
            [3.0, -0.5, 0.5, 0.2, 0.4, 1.0],
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: BufferUsages::VERTEX,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            camera_buffer,
            camera_bind_group,
            vertex_buffer,
            vertex_count: vertices.len() as u32,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.config.width, self.config.height)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render_frame<F>(&mut self, camera: &Camera, draw_overlay: F) -> Result<()>
    where
        F: FnOnce(
            &wgpu::Device,
            &wgpu::Queue,
            &mut CommandEncoder,
            &TextureView,
            PhysicalSize<u32>,
        ),
    {
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

        self.update_camera_uniform(camera);

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("main_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_world_pass"),
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

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        draw_overlay(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            self.surface_size(),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn update_camera_uniform(&self, camera: &Camera) {
        if DEBUG_CLIP_SPACE_TRIANGLE {
            let identity = [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
            self.queue
                .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&identity));
            return;
        }

        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        let projection = perspective_rh_zo(60.0_f32.to_radians(), aspect, 0.1, 100.0);

        let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = camera.pitch.sin_cos();
        let forward = [
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw,
        ];
        let view = look_to_rh(camera.position, forward, [0.0, 1.0, 0.0]);
        let view_projection = mat4_mul(projection, view);

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&view_projection));
    }
}

fn perspective_rh_zo(fovy_radians: f32, aspect: f32, znear: f32, zfar: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (0.5 * fovy_radians).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, zfar / (znear - zfar), -1.0],
        [0.0, 0.0, (zfar * znear) / (znear - zfar), 0.0],
    ]
}

fn look_to_rh(eye: [f32; 3], direction: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let forward = normalize(direction);
    // For our camera-forward convention, this basis keeps objects "in front"
    // mapping to negative view-space Z, which matches the RH projection matrix.
    let side = normalize(cross(up, forward));
    let camera_up = cross(forward, side);

    [
        [side[0], camera_up[0], -forward[0], 0.0],
        [side[1], camera_up[1], -forward[1], 0.0],
        [side[2], camera_up[2], -forward[2], 0.0],
        [-dot(side, eye), -dot(camera_up, eye), dot(forward, eye), 1.0],
    ]
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] = a[0][r] * b[c][0]
                + a[1][r] * b[c][1]
                + a[2][r] * b[c][2]
                + a[3][r] * b[c][3];
        }
    }
    out
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (dot(v, v)).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, -1.0]
    }
}

const TRIANGLE_SHADER: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VsInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VsOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VsInput) -> VsOutput {
    var out: VsOutput;
    out.clip_position = camera.view_projection * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VsOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;
