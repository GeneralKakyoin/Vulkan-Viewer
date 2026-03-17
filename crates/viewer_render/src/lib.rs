use anyhow::{Context, Result};
use std::num::NonZeroU64;
use std::sync::Arc;
use viewer_core::{Camera, MeshKind, Scene, Transform};
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
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    scene_pipeline: RenderPipeline,
    axis_pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    object_buffer: Buffer,
    object_bind_group: wgpu::BindGroup,
    object_uniform_stride: u64,
    max_objects: usize,
    axis_vertex_buffer: Buffer,
    axis_vertex_count: u32,
    ground_mesh: MeshBuffers,
    cube_mesh: MeshBuffers,
}

struct MeshBuffers {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
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
        let (_depth_texture, depth_view) = create_depth_resources(&device, &config);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform_buffer"),
            size: 64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let object_uniform_size = 80_u64;
        let min_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let object_uniform_stride = align_up(object_uniform_size, min_alignment);
        let max_objects = 256_usize;

        let object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("object_uniform_buffer"),
            size: object_uniform_stride * max_objects as u64,
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

        let object_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("object_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: NonZeroU64::new(80),
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

        let object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("object_bind_group"),
            layout: &object_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &object_buffer,
                    offset: 0,
                    size: NonZeroU64::new(80),
                }),
            }],
        });

        let scene_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("scene_shader"),
            source: ShaderSource::Wgsl(SCENE_SHADER.into()),
        });

        let scene_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("scene_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &object_bind_group_layout],
            push_constant_ranges: &[],
        });

        let scene_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("scene_pipeline"),
            layout: Some(&scene_pipeline_layout),
            vertex: VertexState {
                module: &scene_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: 12,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &scene_shader,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let axis_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("axis_shader"),
            source: ShaderSource::Wgsl(AXIS_SHADER.into()),
        });
        let axis_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("axis_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let axis_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("axis_pipeline"),
            layout: Some(&axis_pipeline_layout),
            vertex: VertexState {
                module: &axis_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: 24,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            fragment: Some(FragmentState {
                module: &axis_shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let axis_vertices: [[f32; 6]; 6] = [
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            [1.5, 0.0, 0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            [0.0, 1.5, 0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 0.2, 0.6, 1.0],
            [0.0, 0.0, 1.5, 0.2, 0.6, 1.0],
        ];
        let axis_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("axis_vertex_buffer"),
            contents: bytemuck::cast_slice(&axis_vertices),
            usage: BufferUsages::VERTEX,
        });

        let ground_vertices: [[f32; 3]; 4] = [
            [-0.5, 0.0, -0.5],
            [0.5, 0.0, -0.5],
            [0.5, 0.0, 0.5],
            [-0.5, 0.0, 0.5],
        ];
        let ground_indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let ground_mesh = create_mesh_buffers(
            &device,
            "ground",
            bytemuck::cast_slice(&ground_vertices),
            bytemuck::cast_slice(&ground_indices),
            ground_indices.len() as u32,
        );

        let cube_vertices: [[f32; 3]; 8] = [
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ];
        let cube_indices: [u16; 36] = [
            0, 1, 2, 0, 2, 3, // back
            4, 6, 5, 4, 7, 6, // front
            4, 0, 3, 4, 3, 7, // left
            1, 5, 6, 1, 6, 2, // right
            3, 2, 6, 3, 6, 7, // top
            4, 5, 1, 4, 1, 0, // bottom
        ];
        let cube_mesh = create_mesh_buffers(
            &device,
            "cube",
            bytemuck::cast_slice(&cube_vertices),
            bytemuck::cast_slice(&cube_indices),
            cube_indices.len() as u32,
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            _depth_texture,
            depth_view,
            scene_pipeline,
            axis_pipeline,
            camera_buffer,
            camera_bind_group,
            object_buffer,
            object_bind_group,
            object_uniform_stride,
            max_objects,
            axis_vertex_buffer,
            axis_vertex_count: axis_vertices.len() as u32,
            ground_mesh,
            cube_mesh,
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
        let (depth_texture, depth_view) = create_depth_resources(&self.device, &self.config);
        self._depth_texture = depth_texture;
        self.depth_view = depth_view;
    }

    pub fn render_frame<F>(&mut self, camera: &Camera, scene: &Scene, draw_overlay: F) -> Result<()>
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
        let object_offsets = self.upload_object_uniforms(scene);

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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            let mut object_offset_index = 0usize;
            for instance in &scene.instances {
                match instance.mesh {
                    MeshKind::AxisMarker => {
                        render_pass.set_pipeline(&self.axis_pipeline);
                        render_pass.set_vertex_buffer(0, self.axis_vertex_buffer.slice(..));
                        render_pass.draw(0..self.axis_vertex_count, 0..1);
                    }
                    MeshKind::GroundPlane | MeshKind::Cube => {
                        if object_offset_index >= object_offsets.len() {
                            continue;
                        }

                        let mesh = match instance.mesh {
                            MeshKind::GroundPlane => &self.ground_mesh,
                            MeshKind::Cube => &self.cube_mesh,
                            MeshKind::AxisMarker => unreachable!(),
                        };

                        render_pass.set_pipeline(&self.scene_pipeline);
                        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        render_pass
                            .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        render_pass.set_bind_group(
                            1,
                            &self.object_bind_group,
                            &[object_offsets[object_offset_index]],
                        );
                        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                        object_offset_index += 1;
                    }
                }
            }
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
        let forward = [cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw];
        let view = look_to_rh(camera.position, forward, [0.0, 1.0, 0.0]);
        let view_projection = mat4_mul(projection, view);

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&view_projection));
    }

    fn upload_object_uniforms(&self, scene: &Scene) -> Vec<u32> {
        let mut dynamic_offsets = Vec::new();
        let mut object_index = 0_usize;

        for instance in &scene.instances {
            if matches!(instance.mesh, MeshKind::AxisMarker) {
                continue;
            }

            if object_index >= self.max_objects {
                break;
            }

            let byte_offset = self.object_uniform_stride * object_index as u64;
            if let Ok(dynamic_offset) = u32::try_from(byte_offset) {
                self.update_object_uniform(byte_offset, instance.transform, instance.color);
                dynamic_offsets.push(dynamic_offset);
            }
            object_index += 1;
        }

        dynamic_offsets
    }

    fn update_object_uniform(&self, byte_offset: u64, transform: Transform, color: [f32; 3]) {
        let model = model_matrix(transform);
        let flat = flatten_mat4(model);
        let object_uniform: [f32; 20] = [
            flat[0], flat[1], flat[2], flat[3], flat[4], flat[5], flat[6], flat[7], flat[8],
            flat[9], flat[10], flat[11], flat[12], flat[13], flat[14], flat[15], color[0],
            color[1], color[2], 1.0,
        ];
        self.queue
            .write_buffer(&self.object_buffer, byte_offset, bytemuck::bytes_of(&object_uniform));
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

fn create_depth_resources(
    device: &wgpu::Device,
    config: &SurfaceConfiguration,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene_depth_texture"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn model_matrix(transform: Transform) -> [[f32; 4]; 4] {
    [
        [transform.scale[0], 0.0, 0.0, 0.0],
        [0.0, transform.scale[1], 0.0, 0.0],
        [0.0, 0.0, transform.scale[2], 0.0],
        [
            transform.position[0],
            transform.position[1],
            transform.position[2],
            1.0,
        ],
    ]
}

fn flatten_mat4(m: [[f32; 4]; 4]) -> [f32; 16] {
    [
        m[0][0], m[0][1], m[0][2], m[0][3], m[1][0], m[1][1], m[1][2], m[1][3], m[2][0],
        m[2][1], m[2][2], m[2][3], m[3][0], m[3][1], m[3][2], m[3][3],
    ]
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

const SCENE_SHADER: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> object: ObjectUniform;

struct VsInput {
    @location(0) position: vec3<f32>,
};

struct VsOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(in: VsInput) -> VsOutput {
    var out: VsOutput;
    out.clip_position = camera.view_projection * object.model * vec4<f32>(in.position, 1.0);
    out.color = object.color.rgb;
    return out;
}

@fragment
fn fs_main(in: VsOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

const AXIS_SHADER: &str = r#"
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

fn create_mesh_buffers(
    device: &wgpu::Device,
    label_prefix: &str,
    vertex_data: &[u8],
    index_data: &[u8],
    index_count: u32,
) -> MeshBuffers {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label_prefix}_vertex_buffer")),
        contents: vertex_data,
        usage: BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label_prefix}_index_buffer")),
        contents: index_data,
        usage: BufferUsages::INDEX,
    });

    MeshBuffers {
        vertex_buffer,
        index_buffer,
        index_count,
    }
}
