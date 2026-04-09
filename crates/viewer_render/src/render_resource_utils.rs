use super::*;

pub(super) fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

pub(super) fn avatar_proxy_fallback_forced() -> bool {
    matches!(
        std::env::var("VIEWER_RENDER_FORCE_AVATAR_PROXY_FALLBACK")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes") | Some("on")
    )
}

pub(super) fn texture_budget_mb_from_env() -> u64 {
    std::env::var("VIEWER_RENDER_VRAM_BUDGET_MB")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(512)
        .clamp(16, 4096)
}

pub(super) fn create_depth_resources(
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

pub(super) fn create_fallback_texture(
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

pub(super) fn multiply_rgba(lhs: [f32; 4], rhs: [f32; 4]) -> [f32; 4] {
    [
        lhs[0] * rhs[0],
        lhs[1] * rhs[1],
        lhs[2] * rhs[2],
        lhs[3] * rhs[3],
    ]
}

pub(super) fn blend_clear_color_from_environment(
    environment: &viewer_core::EnvironmentState,
) -> [f32; 4] {
    let env = environment.sanitized();
    if !env.sky_enabled {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let t = env.time_of_day_normalized;
    let top_weight = (0.2 + t * 0.6).clamp(0.0, 1.0);
    [
        env.sky.bottom_color[0] * (1.0 - top_weight) + env.sky.top_color[0] * top_weight,
        env.sky.bottom_color[1] * (1.0 - top_weight) + env.sky.top_color[1] * top_weight,
        env.sky.bottom_color[2] * (1.0 - top_weight) + env.sky.top_color[2] * top_weight,
        env.sky.bottom_color[3] * (1.0 - top_weight) + env.sky.top_color[3] * top_weight,
    ]
}

pub(super) fn create_mesh_buffers(
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
