mod draw_helpers;

use anyhow::{Context, Result};
use draw_helpers::build_draw_list;
// use std::collections::{BTreeMap, HashMap}; // Removed unused imports
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use viewer_core::{
    AlphaMode, AssetID, AvatarRenderMode, Camera, GeometrySource, MaterialDescriptor, MeshKind,
    Scene, Vertex, flatten_mat4, look_to_rh, mat4_mul, perspective_rh_zo,
};
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

mod texture_provider;
pub use texture_provider::{
    DefaultTextureProvider, PendingTexture, StreamingTextureProvider, TextureProvider,
};

pub struct RenderBackend {
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    scene_pipeline: RenderPipeline,
    alpha_test_pipeline: RenderPipeline,
    transparent_pipeline: RenderPipeline,
    axis_pipeline: RenderPipeline,
    camera_buffer: Buffer,
    camera_bind_group: wgpu::BindGroup,
    object_buffer: Buffer,
    object_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    loading_view: wgpu::TextureView,
    missing_view: wgpu::TextureView,
    object_uniform_stride: u64,
    max_objects: usize,
    axis_vertex_buffer: Buffer,
    _axis_vertex_count: u32,
    ground_mesh: MeshBuffers,
    cube_mesh: MeshBuffers,
    avatar_proxy_mesh: Option<MeshBuffers>,
    dynamic_geometries: std::collections::HashMap<GeometrySource, MeshBuffers>,
    texture_provider: StreamingTextureProvider,
    textures: std::collections::HashMap<AssetID, wgpu::Texture>,
}

struct FrameCaptureJob {
    path: PathBuf,
    buffer: wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
}

struct MeshBuffers {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    submeshes: Vec<SubMeshRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubMeshRange {
    pub face_id: u16,
    pub index_start: u32,
    pub index_count: u32,
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
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
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

        let object_uniform_size = 144_u64;
        let min_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let object_uniform_stride = align_up(object_uniform_size, min_alignment);
        let max_objects = 2048_usize; // Note: max_objects is effectively the cap on total uniform slots (submeshes)

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
                        min_binding_size: NonZeroU64::new(144),
                    },
                    count: None,
                }],
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
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
                    size: NonZeroU64::new(144),
                }),
            }],
        });

        let scene_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("scene_shader"),
            source: ShaderSource::Wgsl(SCENE_SHADER.into()),
        });

        let scene_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("scene_pipeline_layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &object_bind_group_layout,
                &texture_bind_group_layout,
            ],
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
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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

        let alpha_test_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("alpha_test_pipeline"),
            layout: Some(&scene_pipeline_layout),
            vertex: VertexState {
                module: &scene_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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

        let transparent_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("transparent_pipeline"),
            layout: Some(&scene_pipeline_layout),
            vertex: VertexState {
                module: &scene_shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                }],
            },
            fragment: Some(FragmentState {
                module: &scene_shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
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

        let make_vertex = |position: [f32; 3]| Vertex {
            position,
            normal: [0.0, 1.0, 0.0],
            tex_coord: [0.0, 0.0],
        };

        let ground_verts: [Vertex; 4] = [
            make_vertex([-100.0, 0.0, -100.0]),
            make_vertex([100.0, 0.0, -100.0]),
            make_vertex([100.0, 0.0, 100.0]),
            make_vertex([-100.0, 0.0, 100.0]),
        ];
        let ground_indices: [u32; 6] = [0, 1, 2, 2, 3, 0];
        let ground_mesh = create_mesh_buffers(
            &device,
            "ground",
            bytemuck::cast_slice(&ground_verts),
            bytemuck::cast_slice(&ground_indices),
            vec![SubMeshRange {
                face_id: 0,
                index_start: 0,
                index_count: 6,
            }],
        );

        let cube_verts: [Vertex; 8] = [
            make_vertex([-0.5, -0.5, -0.5]),
            make_vertex([0.5, -0.5, -0.5]),
            make_vertex([0.5, 0.5, -0.5]),
            make_vertex([-0.5, 0.5, -0.5]),
            make_vertex([-0.5, -0.5, 0.5]),
            make_vertex([0.5, -0.5, 0.5]),
            make_vertex([0.5, 0.5, 0.5]),
            make_vertex([-0.5, 0.5, 0.5]),
        ];
        let cube_indices: [u32; 36] = [
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            0, 1, 5, 5, 4, 0, // front
            2, 3, 7, 7, 6, 2, // back
            1, 2, 6, 6, 5, 1, // right
            3, 0, 4, 4, 7, 3, // left
        ];
        let cube_mesh = create_mesh_buffers(
            &device,
            "cube",
            bytemuck::cast_slice(&cube_verts),
            bytemuck::cast_slice(&cube_indices),
            vec![SubMeshRange {
                face_id: 0,
                index_start: 0,
                index_count: 36,
            }],
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("default_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let loading_view =
            create_fallback_texture(&device, &queue, "loading", [1.0, 1.0, 0.0, 1.0]); // Yellow
        let missing_view =
            create_fallback_texture(&device, &queue, "missing", [1.0, 0.0, 1.0, 1.0]); // Magenta

        let avatar_proxy_mesh = if avatar_proxy_fallback_forced() {
            None
        } else {
            let avatar_vertices: [Vertex; 16] = [
                // torso
                make_vertex([-0.20, 0.00, -0.12]),
                make_vertex([0.20, 0.00, -0.12]),
                make_vertex([0.20, 0.70, -0.12]),
                make_vertex([-0.20, 0.70, -0.12]),
                make_vertex([-0.20, 0.00, 0.12]),
                make_vertex([0.20, 0.00, 0.12]),
                make_vertex([0.20, 0.70, 0.12]),
                make_vertex([-0.20, 0.70, 0.12]),
                // head
                make_vertex([-0.14, 0.72, -0.14]),
                make_vertex([0.14, 0.72, -0.14]),
                make_vertex([0.14, 1.00, -0.14]),
                make_vertex([-0.14, 1.00, -0.14]),
                make_vertex([-0.14, 0.72, 0.14]),
                make_vertex([0.14, 0.72, 0.14]),
                make_vertex([0.14, 1.00, 0.14]),
                make_vertex([-0.14, 1.00, 0.14]),
            ];
            let avatar_indices: [u32; 72] = [
                // torso
                0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 4, 0, 3, 4, 3, 7, 1, 5, 6, 1, 6, 2, 3, 2, 6, 3,
                6, 7, 4, 5, 1, 4, 1, 0, // head
                8, 9, 10, 8, 10, 11, 12, 14, 13, 12, 15, 14, 12, 8, 11, 12, 11, 15, 9, 13, 14, 9,
                14, 10, 11, 10, 14, 11, 14, 15, 12, 13, 9, 12, 9, 8,
            ];
            Some(create_mesh_buffers(
                &device,
                "avatar_proxy",
                bytemuck::cast_slice(&avatar_vertices),
                bytemuck::cast_slice(&avatar_indices),
                vec![SubMeshRange {
                    face_id: 0,
                    index_start: 0,
                    index_count: 72,
                }],
            ))
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            _depth_texture,
            depth_view,
            scene_pipeline,
            alpha_test_pipeline,
            transparent_pipeline,
            axis_pipeline,
            camera_buffer,
            camera_bind_group,
            object_buffer,
            object_bind_group,
            texture_bind_group_layout,
            sampler,
            loading_view,
            missing_view,
            object_uniform_stride,
            max_objects,
            axis_vertex_buffer,
            _axis_vertex_count: axis_vertices.len() as u32,
            ground_mesh,
            cube_mesh,
            avatar_proxy_mesh,
            dynamic_geometries: Default::default(),
            texture_provider: StreamingTextureProvider::new(texture_budget_mb_from_env()),
            textures: Default::default(),
        })
    }

    pub fn avatar_render_mode(&self) -> AvatarRenderMode {
        if self.avatar_proxy_mesh.is_some() {
            AvatarRenderMode::Proxy
        } else {
            AvatarRenderMode::FallbackBox
        }
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

    pub fn aspect_ratio(&self) -> f32 {
        self.config.width as f32 / self.config.height.max(1) as f32
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

    pub fn upsert_geometry(
        &mut self,
        id: GeometrySource,
        vertices: &[u8],
        indices: &[u8],
        submeshes: Vec<SubMeshRange>,
    ) {
        self.dynamic_geometries.insert(
            id,
            create_mesh_buffers(&self.device, "dynamic_mesh", vertices, indices, submeshes),
        );
    }

    pub fn has_dynamic_geometry(&self, source: &GeometrySource) -> bool {
        self.dynamic_geometries.contains_key(source)
    }

    pub fn has_texture(&self, id: &AssetID) -> bool {
        self.texture_provider.contains(id)
    }

    pub fn texture_vram_usage_mb(&self) -> f32 {
        self.texture_provider.vram_usage_mb()
    }

    pub fn upsert_texture_rgba8(
        &mut self,
        id: AssetID,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<()> {
        if id.is_empty() {
            return Ok(());
        }
        if width == 0 || height == 0 {
            return Ok(());
        }
        let expected_len = width as usize * height as usize * 4;
        if rgba.len() != expected_len {
            anyhow::bail!("rgba buffer length mismatch");
        }

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("asset_texture_rgba8"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_row = 4 * width;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let size_bytes = rgba.len() as u64;
        let evicted = self
            .texture_provider
            .insert(id.clone(), Arc::clone(&view), size_bytes);
        for evict in evicted {
            self.textures.remove(&evict);
        }

        self.textures.insert(id, texture);
        Ok(())
    }

    pub fn render_frame<F>(
        &mut self,
        camera: &Camera,
        scene: &Scene,
        visibility_list: &[usize],
        time: f32,
        capture_path: Option<&Path>,
        draw_overlay: F,
    ) -> Result<()>
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
        let sorted_visibility = build_draw_list(
            &scene.instances,
            visibility_list,
            camera.position,
            self.max_objects,
        );
        let object_offsets = self.upload_object_uniforms(scene, &sorted_visibility, time);

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

            for &id in &sorted_visibility {
                let Some(instance) = scene.instances.get(&id) else {
                    continue;
                };

                match &instance.geometry {
                    GeometrySource::Diagnostic(MeshKind::AxisMarker) => {
                        render_pass.set_pipeline(&self.axis_pipeline);
                        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, self.axis_vertex_buffer.slice(..));
                        render_pass.draw(0..6, 0..1);
                    }
                    _ => {
                        if object_offset_index >= object_offsets.len() {
                            continue;
                        }

                        let mesh_buffers = match &instance.geometry {
                            GeometrySource::Diagnostic(kind) => match kind {
                                MeshKind::GroundPlane => &self.ground_mesh,
                                MeshKind::Cube => &self.cube_mesh,
                                MeshKind::AvatarProxy => {
                                    self.avatar_proxy_mesh.as_ref().unwrap_or(&self.cube_mesh)
                                }
                                MeshKind::AxisMarker => unreachable!(),
                            },
                            source => {
                                if let Some(buffers) = self.dynamic_geometries.get(source) {
                                    buffers
                                } else {
                                    &self.cube_mesh
                                }
                            }
                        };

                        let pipeline = match instance.alpha_mode {
                            AlphaMode::Opaque => &self.scene_pipeline,
                            AlphaMode::AlphaTest { .. } => &self.alpha_test_pipeline,
                            AlphaMode::Blend => &self.transparent_pipeline,
                        };

                        render_pass.set_pipeline(pipeline);
                        render_pass.set_vertex_buffer(0, mesh_buffers.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(
                            mesh_buffers.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );

                        // Draw each submesh with its specific material and uniform slot
                        for submesh in &mesh_buffers.submeshes {
                            if object_offset_index >= object_offsets.len() {
                                break;
                            }

                            render_pass.set_bind_group(
                                1,
                                &self.object_bind_group,
                                &[object_offsets[object_offset_index]],
                            );

                            // Bind textures for this submesh
                            let mat = instance.materials.material_for_face(submesh.face_id);
                            let bind_group = self.create_material_bind_group(mat);
                            render_pass.set_bind_group(2, &bind_group, &[]);

                            render_pass.draw_indexed(
                                submesh.index_start..submesh.index_start + submesh.index_count,
                                0,
                                0..1,
                            );

                            object_offset_index += 1;
                        }
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

        let frame_capture = if let Some(path) = capture_path {
            Some(self.schedule_frame_capture(&mut encoder, &output.texture, path)?)
        } else {
            None
        };

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        if let Some(job) = frame_capture {
            self.complete_frame_capture(job)?;
        }

        Ok(())
    }

    fn schedule_frame_capture(
        &self,
        encoder: &mut CommandEncoder,
        surface_texture: &wgpu::Texture,
        path: &Path,
    ) -> Result<FrameCaptureJob> {
        let width = self.config.width.max(1);
        let height = self.config.height.max(1);
        let unpadded_bytes_per_row = width * 4;
        let padded_bytes_per_row = align_up(
            unpadded_bytes_per_row as u64,
            wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64,
        ) as u32;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame_capture_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: surface_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(FrameCaptureJob {
            path: path.to_path_buf(),
            buffer,
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        })
    }

    fn complete_frame_capture(&self, job: FrameCaptureJob) -> Result<()> {
        let slice = job.buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        receiver
            .recv()
            .context("frame capture map channel closed")?
            .context("failed to map frame capture buffer")?;

        let mapped = slice.get_mapped_range();
        let mut rgba = vec![0_u8; (job.unpadded_bytes_per_row * job.height) as usize];
        for row in 0..job.height as usize {
            let src_offset = row * job.padded_bytes_per_row as usize;
            let dst_offset = row * job.unpadded_bytes_per_row as usize;
            let src = &mapped[src_offset..src_offset + job.unpadded_bytes_per_row as usize];
            let dst = &mut rgba[dst_offset..dst_offset + job.unpadded_bytes_per_row as usize];
            dst.copy_from_slice(src);
        }
        drop(mapped);
        job.buffer.unmap();

        if let Some(parent) = job.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create frame capture directory: {}",
                    parent.display()
                )
            })?;
        }

        image::save_buffer_with_format(
            &job.path,
            &rgba,
            job.width,
            job.height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .with_context(|| format!("failed to write screenshot: {}", job.path.display()))?;

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

    fn upload_object_uniforms(
        &self,
        scene: &Scene,
        visibility_list: &[usize],
        time: f32,
    ) -> Vec<u32> {
        let mut dynamic_offsets = Vec::new();
        let mut object_index = 0_usize;

        for &id in visibility_list {
            let Some(instance) = scene.instances.get(&id) else {
                continue;
            };

            if matches!(
                instance.geometry,
                GeometrySource::Diagnostic(MeshKind::AxisMarker)
            ) {
                continue;
            }

            // Get submesh count to reserve uniform slots
            let submesh_count = match &instance.geometry {
                GeometrySource::Diagnostic(kind) => match kind {
                    MeshKind::AxisMarker => 0,
                    _ => 1,
                },
                source => self
                    .dynamic_geometries
                    .get(source)
                    .map(|m| m.submeshes.len())
                    .unwrap_or(1),
            };

            for sub_idx in 0..submesh_count {
                if object_index >= self.max_objects {
                    break;
                }

                let byte_offset = self.object_uniform_stride * object_index as u64;
                if let Ok(dynamic_offset) = u32::try_from(byte_offset) {
                    // Identify face_id for this submesh slot
                    let face_id = match &instance.geometry {
                        source => self
                            .dynamic_geometries
                            .get(source)
                            .and_then(|m| m.submeshes.get(sub_idx))
                            .map(|s| s.face_id)
                            .unwrap_or(0),
                    };

                    let mat = instance.materials.material_for_face(face_id);
                    let (uv_matrix, material_tint) = match mat {
                        MaterialDescriptor::Legacy(entry) => (
                            viewer_core::material::animation::compute_texture_matrix(
                                entry,
                                &instance.texture_anim,
                                time,
                            ),
                            entry.rgba,
                        ),
                        MaterialDescriptor::Pbr(pbr) => (
                            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                            pbr.base_color_tint,
                        ),
                    };
                    let final_tint = multiply_rgba(instance.color, material_tint);

                    // Keep instance-level modulation while applying per-material tint semantics.
                    self.update_object_uniform(
                        byte_offset,
                        instance.world_matrix,
                        final_tint,
                        uv_matrix,
                        instance.alpha_mode,
                    );
                    dynamic_offsets.push(dynamic_offset);
                }
                object_index += 1;
            }
        }

        dynamic_offsets
    }

    fn create_material_bind_group(&self, mat: &MaterialDescriptor) -> wgpu::BindGroup {
        let (base, normal, mr, emissive) = match mat {
            MaterialDescriptor::Legacy(entry) => {
                let view = if entry.texture_id.is_empty() {
                    Arc::new(self.missing_view.clone())
                } else {
                    match self.texture_provider.get_texture(&entry.texture_id) {
                        PendingTexture::Ready(v) => v,
                        PendingTexture::Loading => Arc::new(self.loading_view.clone()),
                        PendingTexture::Missing => Arc::new(self.missing_view.clone()),
                    }
                };
                (
                    view.clone(),
                    Arc::new(self.missing_view.clone()),
                    Arc::new(self.missing_view.clone()),
                    Arc::new(self.missing_view.clone()),
                )
            }
            MaterialDescriptor::Pbr(pbr) => {
                let get_v = |id: &AssetID| {
                    if id.is_empty() {
                        Arc::new(self.missing_view.clone())
                    } else {
                        match self.texture_provider.get_texture(id) {
                            PendingTexture::Ready(v) => v,
                            PendingTexture::Loading => Arc::new(self.loading_view.clone()),
                            PendingTexture::Missing => Arc::new(self.missing_view.clone()),
                        }
                    }
                };
                (
                    get_v(&pbr.base_color_id),
                    get_v(&pbr.normal_id),
                    get_v(&pbr.metallic_roughness_id),
                    get_v(&pbr.emissive_id),
                )
            }
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&base),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&mr),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&emissive),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn update_object_uniform(
        &self,
        byte_offset: u64,
        model: [[f32; 4]; 4],
        color: [f32; 4],
        uv_matrix: [[f32; 3]; 3],
        alpha_mode: AlphaMode,
    ) {
        let (mode_index, cutoff) = match alpha_mode {
            AlphaMode::Opaque => (0u32, 0.0f32),
            AlphaMode::AlphaTest { cutoff } => (1u32, cutoff),
            AlphaMode::Blend => (2u32, 0.0f32),
        };
        let flat = flatten_mat4(model);
        // mat3x3 in WGSL is 3 columns of vec4 (16 byte alignment per column)
        let m = uv_matrix;
        let object_uniform: [f32; 36] = [
            flat[0],
            flat[1],
            flat[2],
            flat[3],
            flat[4],
            flat[5],
            flat[6],
            flat[7],
            flat[8],
            flat[9],
            flat[10],
            flat[11],
            flat[12],
            flat[13],
            flat[14],
            flat[15],
            color[0],
            color[1],
            color[2],
            color[3],
            m[0][0],
            m[0][1],
            m[0][2],
            0.0,
            m[1][0],
            m[1][1],
            m[1][2],
            0.0,
            m[2][0],
            m[2][1],
            m[2][2],
            0.0,
            mode_index as f32,
            cutoff,
            0.0,
            0.0,
        ];
        self.queue.write_buffer(
            &self.object_buffer,
            byte_offset,
            bytemuck::bytes_of(&object_uniform),
        );
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

fn avatar_proxy_fallback_forced() -> bool {
    matches!(
        std::env::var("VIEWER_RENDER_FORCE_AVATAR_PROXY_FALLBACK")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

fn texture_budget_mb_from_env() -> u64 {
    std::env::var("VIEWER_RENDER_VRAM_BUDGET_MB")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(512)
        .clamp(16, 4096)
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

fn create_fallback_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    color: [f32; 4],
) -> wgpu::TextureView {
    let rgba = [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    ];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("{}_fallback_texture", label)),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
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
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn multiply_rgba(lhs: [f32; 4], rhs: [f32; 4]) -> [f32; 4] {
    [
        lhs[0] * rhs[0],
        lhs[1] * rhs[1],
        lhs[2] * rhs[2],
        lhs[3] * rhs[3],
    ]
}

// model_matrix removed, using pre-computed world_matrix from Scene instead

// Internal math helpers removed, using viewer_core instead

// Vector math removed

// Vector math removed

const SCENE_SHADER: &str = r#"
struct CameraUniform {
    view_projection: mat4x4<f32>,
};

struct ObjectUniform {
    model: mat4x4<f32>,
    color: vec4<f32>,
    uv_matrix: mat3x3<f32>,
    alpha_mode: u32,
    alpha_cutoff: f32,
    _padding: vec2<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> object: ObjectUniform;

@group(2) @binding(0) var t_base: texture_2d<f32>;
@group(2) @binding(1) var t_normal: texture_2d<f32>;
@group(2) @binding(2) var t_mr: texture_2d<f32>;
@group(2) @binding(3) var t_emissive: texture_2d<f32>;
@group(2) @binding(4) var s_base: sampler;

struct VsInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VsOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VsInput) -> VsOutput {
    var out: VsOutput;
    out.clip_position = camera.view_projection * object.model * vec4<f32>(in.position, 1.0);
    
    // Apply UV matrix
    let uv3 = object.uv_matrix * vec3<f32>(in.tex_coord, 1.0);
    out.tex_coord = uv3.xy;
    
    out.color = object.color;
    return out;
}

@fragment
fn fs_main(in: VsOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(t_base, s_base, in.tex_coord);
    let final_color = base_color * in.color;
    
    if (object.alpha_mode == 1u && final_color.a < object.alpha_cutoff) {
        discard;
    }
    return final_color;
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
    submeshes: Vec<SubMeshRange>,
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
        submeshes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use viewer_core::{AssetID, MaterialDescriptor, MaterialSet, TextureEntry};

    #[test]
    fn test_material_slot_mapping_respects_face_id() {
        let mut mat_set = MaterialSet::default();
        mat_set.default = MaterialDescriptor::Legacy(TextureEntry {
            texture_id: AssetID::new("default_tex"),
            ..TextureEntry::default()
        });

        let face_1_mat = MaterialDescriptor::Legacy(TextureEntry {
            texture_id: AssetID::new("face_1_tex"),
            ..TextureEntry::default()
        });
        mat_set.by_face.insert(1, face_1_mat.clone());

        // Face 0 should fallback to default
        let mat_0 = mat_set.material_for_face(0);
        assert_eq!(mat_0.texture_ids()[0], AssetID::new("default_tex"));

        // Face 1 should use specific override
        let mat_1 = mat_set.material_for_face(1);
        assert_eq!(mat_1.texture_ids()[0], AssetID::new("face_1_tex"));

        // Face 2 should fallback to default
        let mat_2 = mat_set.material_for_face(2);
        assert_eq!(mat_2.texture_ids()[0], AssetID::new("default_tex"));
    }

    #[test]
    fn test_multiply_rgba_is_componentwise() {
        let lhs = [0.5, 0.25, 1.0, 0.8];
        let rhs = [0.2, 1.0, 0.5, 0.25];
        assert_eq!(multiply_rgba(lhs, rhs), [0.1, 0.25, 0.5, 0.2]);
    }
}
