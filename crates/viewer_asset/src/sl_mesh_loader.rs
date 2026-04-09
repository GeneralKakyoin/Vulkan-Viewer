use crate::ProcessedMesh;
use anyhow::{Context, Result};
use flate2::read::ZlibDecoder;
use std::collections::BTreeMap;
use std::io::Read;
use viewer_core::Vertex;
use viewer_core::geometry::llvolume::SubMesh;

const DEPRECATED_LLSD_BINARY_HEADER: &[u8] = b"<? LLSD/Binary ?>";
const SECOND_LIFE_LOD_NAMES: [&str; 4] = ["high_lod", "medium_lod", "low_lod", "lowest_lod"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MeshSourceFormat {
    Unknown,
    Gltf,
    SecondLifeMesh,
}

pub fn detect_mesh_source_format(data: &[u8]) -> MeshSourceFormat {
    if data.len() >= 4 && &data[..4] == b"glTF" {
        return MeshSourceFormat::Gltf;
    }

    if data.starts_with(DEPRECATED_LLSD_BINARY_HEADER) {
        return MeshSourceFormat::SecondLifeMesh;
    }

    if data.len() >= 5 && data[0] == b'{' && data[1..5].contains(&0) {
        return MeshSourceFormat::SecondLifeMesh;
    }

    MeshSourceFormat::Unknown
}

pub fn load_second_life_mesh(data: &[u8], requested_lod: u32) -> Result<ProcessedMesh> {
    let data = strip_deprecated_header(data);
    let (header_value, header_size) =
        parse_binary_llsd(data).context("failed to parse SL mesh LLSD header")?;
    let header_map = header_value
        .as_map()
        .context("SL mesh header is not an LLSD map")?;
    let block = select_lod_block(header_map, requested_lod)?;
    let block_start = header_size
        .checked_add(block.offset)
        .context("mesh LOD offset overflowed")?;
    let block_end = block_start
        .checked_add(block.size)
        .context("mesh LOD size overflowed")?;
    let block_bytes = data
        .get(block_start..block_end)
        .with_context(|| format!("mesh {} block is outside asset bounds", block.name))?;

    let mut decoded = Vec::new();
    ZlibDecoder::new(block_bytes)
        .read_to_end(&mut decoded)
        .with_context(|| format!("failed to decompress mesh {} block", block.name))?;

    let (lod_value, _) = parse_binary_llsd(&decoded)
        .with_context(|| format!("failed to parse mesh {}", block.name))?;
    decode_lod_mesh(lod_value)
}

#[derive(Debug, Clone)]
struct MeshBlockLocation {
    name: String,
    offset: usize,
    size: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum BinaryLlsdValue {
    Undefined,
    Bool(bool),
    Integer(i32),
    Real(f64),
    Date(f64),
    String(String),
    Binary(Vec<u8>),
    Array(Vec<BinaryLlsdValue>),
    Map(BTreeMap<String, BinaryLlsdValue>),
    Uuid([u8; 16]),
}

impl BinaryLlsdValue {
    fn as_map(&self) -> Option<&BTreeMap<String, BinaryLlsdValue>> {
        match self {
            Self::Map(value) => Some(value),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&[BinaryLlsdValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Integer(value) => Some(*value as f32),
            Self::Real(value) => Some(*value as f32),
            _ => None,
        }
    }

    fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Self::Binary(value) => Some(value),
            _ => None,
        }
    }
}

fn strip_deprecated_header(data: &[u8]) -> &[u8] {
    if data.starts_with(DEPRECATED_LLSD_BINARY_HEADER) {
        &data[DEPRECATED_LLSD_BINARY_HEADER.len()..]
    } else {
        data
    }
}

fn parse_binary_llsd(data: &[u8]) -> Result<(BinaryLlsdValue, usize)> {
    let mut cursor = 0usize;
    let value = parse_binary_llsd_value(data, &mut cursor)?;
    Ok((value, cursor))
}

fn parse_binary_llsd_value(data: &[u8], cursor: &mut usize) -> Result<BinaryLlsdValue> {
    let token = read_byte(data, cursor)?;
    match token {
        b'{' => parse_binary_llsd_map(data, cursor),
        b'[' => parse_binary_llsd_array(data, cursor),
        b'!' => Ok(BinaryLlsdValue::Undefined),
        b'0' => Ok(BinaryLlsdValue::Bool(false)),
        b'1' => Ok(BinaryLlsdValue::Bool(true)),
        b'i' => Ok(BinaryLlsdValue::Integer(read_i32_be(data, cursor)?)),
        b'r' => Ok(BinaryLlsdValue::Real(read_f64_be(data, cursor)?)),
        b'd' => Ok(BinaryLlsdValue::Date(read_f64_be(data, cursor)?)),
        b'\'' | b'"' => Ok(BinaryLlsdValue::String(read_delimited_string(
            data, cursor, token,
        )?)),
        b's' | b'l' | b'k' => Ok(BinaryLlsdValue::String(read_len_prefixed_string(
            data, cursor,
        )?)),
        b'b' => Ok(BinaryLlsdValue::Binary(read_len_prefixed_bytes(
            data, cursor,
        )?)),
        b'u' => Ok(BinaryLlsdValue::Uuid(read_uuid(data, cursor)?)),
        _ => anyhow::bail!("unsupported binary LLSD token 0x{token:02x}"),
    }
}

fn parse_binary_llsd_map(data: &[u8], cursor: &mut usize) -> Result<BinaryLlsdValue> {
    let count = read_u32_be(data, cursor)? as usize;
    let mut map = BTreeMap::new();
    for _ in 0..count {
        let key_token = read_byte(data, cursor)?;
        let key = match key_token {
            b'k' | b's' | b'l' => read_len_prefixed_string(data, cursor)?,
            b'\'' | b'"' => read_delimited_string(data, cursor, key_token)?,
            _ => anyhow::bail!("unsupported binary LLSD map key token 0x{key_token:02x}"),
        };
        let value = parse_binary_llsd_value(data, cursor)?;
        map.insert(key, value);
    }
    let end = read_byte(data, cursor)?;
    if end != b'}' {
        anyhow::bail!("binary LLSD map missing closing terminator");
    }
    Ok(BinaryLlsdValue::Map(map))
}

fn parse_binary_llsd_array(data: &[u8], cursor: &mut usize) -> Result<BinaryLlsdValue> {
    let count = read_u32_be(data, cursor)? as usize;
    let mut array = Vec::with_capacity(count);
    for _ in 0..count {
        array.push(parse_binary_llsd_value(data, cursor)?);
    }
    let end = read_byte(data, cursor)?;
    if end != b']' {
        anyhow::bail!("binary LLSD array missing closing terminator");
    }
    Ok(BinaryLlsdValue::Array(array))
}

fn select_lod_block(
    header: &BTreeMap<String, BinaryLlsdValue>,
    requested_lod: u32,
) -> Result<MeshBlockLocation> {
    let preference = match requested_lod {
        0 => [0usize, 1, 2, 3],
        1 => [1usize, 0, 2, 3],
        2 => [2usize, 1, 3, 0],
        _ => [3usize, 2, 1, 0],
    };

    for idx in preference {
        let name = SECOND_LIFE_LOD_NAMES[idx];
        let Some(block_value) = header.get(name) else {
            continue;
        };
        let Some(block_map) = block_value.as_map() else {
            continue;
        };
        let offset = block_map
            .get("offset")
            .and_then(BinaryLlsdValue::as_i32)
            .context("mesh block offset missing")?;
        let size = block_map
            .get("size")
            .and_then(BinaryLlsdValue::as_i32)
            .context("mesh block size missing")?;
        if offset < 0 || size <= 0 {
            continue;
        }
        return Ok(MeshBlockLocation {
            name: String::from(name),
            offset: offset as usize,
            size: size as usize,
        });
    }

    anyhow::bail!("mesh asset did not contain a usable display LOD block")
}

fn decode_lod_mesh(value: BinaryLlsdValue) -> Result<ProcessedMesh> {
    let submesh_values = value
        .as_array()
        .context("mesh LOD block is not an LLSD array")?;
    if submesh_values.is_empty() {
        anyhow::bail!("mesh LOD block contained no submeshes");
    }

    let mut vertices = Vec::new();
    let mut submeshes = Vec::new();
    let mut saw_non_zero_normal = false;

    for (face_id, submesh_value) in submesh_values.iter().enumerate() {
        let Some(submesh_map) = submesh_value.as_map() else {
            continue;
        };
        if submesh_map
            .get("NoGeometry")
            .and_then(BinaryLlsdValue::as_bool)
            .unwrap_or(false)
        {
            continue;
        }

        let position_domain = submesh_map
            .get("PositionDomain")
            .and_then(BinaryLlsdValue::as_map)
            .context("mesh submesh missing PositionDomain")?;
        let position_min = vector3_from_map(position_domain, "Min")?;
        let position_max = vector3_from_map(position_domain, "Max")?;
        let position_bytes = submesh_map
            .get("Position")
            .and_then(BinaryLlsdValue::as_binary)
            .context("mesh submesh missing Position bytes")?;
        let triangle_bytes = submesh_map
            .get("TriangleList")
            .and_then(BinaryLlsdValue::as_binary)
            .context("mesh submesh missing TriangleList bytes")?;

        let normal_bytes = submesh_map
            .get("Normal")
            .and_then(BinaryLlsdValue::as_binary)
            .unwrap_or(&[]);
        let texcoord_domain = submesh_map
            .get("TexCoord0Domain")
            .and_then(BinaryLlsdValue::as_map);
        let texcoord_bytes = submesh_map
            .get("TexCoord0")
            .and_then(BinaryLlsdValue::as_binary)
            .unwrap_or(&[]);

        let decoded_positions = decode_position_stream(position_bytes, position_min, position_max)?;
        if decoded_positions.is_empty() {
            continue;
        }

        let decoded_normals = decode_normal_stream(normal_bytes, decoded_positions.len());
        if decoded_normals
            .iter()
            .any(|normal| normal.iter().any(|component| component.abs() > 0.0001))
        {
            saw_non_zero_normal = true;
        }
        let decoded_texcoords =
            decode_texcoord_stream(texcoord_bytes, texcoord_domain, decoded_positions.len());
        let decoded_indices = decode_triangle_indices(triangle_bytes);
        if decoded_indices.len() < 3 {
            continue;
        }

        let base_index = vertices.len() as u32;
        for idx in 0..decoded_positions.len() {
            vertices.push(Vertex {
                position: decoded_positions[idx],
                normal: decoded_normals[idx],
                tex_coord: decoded_texcoords[idx],
            });
        }

        submeshes.push(SubMesh {
            face_id: face_id as u16,
            indices: decoded_indices
                .into_iter()
                .map(|index| base_index + u32::from(index))
                .collect(),
        });
    }

    if vertices.is_empty() || submeshes.is_empty() {
        anyhow::bail!("mesh LOD block decoded no drawable geometry");
    }

    if !saw_non_zero_normal {
        rebuild_vertex_normals(&mut vertices, &submeshes);
    }

    let aabb = compute_aabb(&vertices);
    Ok(ProcessedMesh {
        vertices,
        submeshes,
        aabb,
    })
}

fn decode_position_stream(bytes: &[u8], min: [f32; 3], max: [f32; 3]) -> Result<Vec<[f32; 3]>> {
    if !bytes.len().is_multiple_of(6) {
        anyhow::bail!("position byte stream length is not divisible by 6");
    }
    let mut positions = Vec::with_capacity(bytes.len() / 6);
    for chunk in bytes.chunks_exact(6) {
        let x = u16::from_le_bytes([chunk[0], chunk[1]]);
        let y = u16::from_le_bytes([chunk[2], chunk[3]]);
        let z = u16::from_le_bytes([chunk[4], chunk[5]]);
        positions.push([
            lerp_u16(x, min[0], max[0]),
            lerp_u16(y, min[1], max[1]),
            lerp_u16(z, min[2], max[2]),
        ]);
    }
    Ok(positions)
}

fn decode_normal_stream(bytes: &[u8], vertex_count: usize) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0, 0.0, 0.0]; vertex_count];
    if bytes.len() / 6 < vertex_count {
        return normals;
    }

    for (idx, chunk) in bytes.chunks_exact(6).take(vertex_count).enumerate() {
        let x = u16::from_le_bytes([chunk[0], chunk[1]]);
        let y = u16::from_le_bytes([chunk[2], chunk[3]]);
        let z = u16::from_le_bytes([chunk[4], chunk[5]]);
        normals[idx] = [
            normalized_signed_u16(x),
            normalized_signed_u16(y),
            normalized_signed_u16(z),
        ];
    }
    normals
}

fn decode_texcoord_stream(
    bytes: &[u8],
    domain: Option<&BTreeMap<String, BinaryLlsdValue>>,
    vertex_count: usize,
) -> Vec<[f32; 2]> {
    let mut texcoords = vec![[0.0, 0.0]; vertex_count];
    let Some(domain) = domain else {
        return texcoords;
    };
    let Ok(min) = vector2_from_map(domain, "Min") else {
        return texcoords;
    };
    let Ok(max) = vector2_from_map(domain, "Max") else {
        return texcoords;
    };
    if bytes.len() / 4 < vertex_count {
        return texcoords;
    }
    for (idx, chunk) in bytes.chunks_exact(4).take(vertex_count).enumerate() {
        let u = u16::from_le_bytes([chunk[0], chunk[1]]);
        let v = u16::from_le_bytes([chunk[2], chunk[3]]);
        texcoords[idx] = [lerp_u16(u, min[0], max[0]), lerp_u16(v, min[1], max[1])];
    }
    texcoords
}

fn decode_triangle_indices(bytes: &[u8]) -> Vec<u16> {
    let usable_len = bytes.len() - (bytes.len() % 2);
    bytes[..usable_len]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

fn vector3_from_map(map: &BTreeMap<String, BinaryLlsdValue>, key: &str) -> Result<[f32; 3]> {
    let values = map
        .get(key)
        .and_then(BinaryLlsdValue::as_array)
        .with_context(|| format!("missing {key} vector3"))?;
    if values.len() < 3 {
        anyhow::bail!("{key} vector3 was truncated");
    }
    Ok([
        values[0].as_f32().context("vector3 x missing")?,
        values[1].as_f32().context("vector3 y missing")?,
        values[2].as_f32().context("vector3 z missing")?,
    ])
}

fn vector2_from_map(map: &BTreeMap<String, BinaryLlsdValue>, key: &str) -> Result<[f32; 2]> {
    let values = map
        .get(key)
        .and_then(BinaryLlsdValue::as_array)
        .with_context(|| format!("missing {key} vector2"))?;
    if values.len() < 2 {
        anyhow::bail!("{key} vector2 was truncated");
    }
    Ok([
        values[0].as_f32().context("vector2 u missing")?,
        values[1].as_f32().context("vector2 v missing")?,
    ])
}

fn compute_aabb(vertices: &[Vertex]) -> viewer_core::Aabb {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for vertex in vertices {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex.position[axis]);
            max[axis] = max[axis].max(vertex.position[axis]);
        }
    }
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let size = [
        (max[0] - min[0]) * 0.5,
        (max[1] - min[1]) * 0.5,
        (max[2] - min[2]) * 0.5,
    ];
    viewer_core::Aabb::new(center, size)
}

fn rebuild_vertex_normals(vertices: &mut [Vertex], submeshes: &[SubMesh]) {
    let mut accum = vec![[0.0f32; 3]; vertices.len()];
    for submesh in submeshes {
        for triangle in submesh.indices.chunks_exact(3) {
            let [ia, ib, ic] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            if ia >= vertices.len() || ib >= vertices.len() || ic >= vertices.len() {
                continue;
            }
            let a = vertices[ia].position;
            let b = vertices[ib].position;
            let c = vertices[ic].position;
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            for idx in [ia, ib, ic] {
                accum[idx][0] += normal[0];
                accum[idx][1] += normal[1];
                accum[idx][2] += normal[2];
            }
        }
    }

    for (vertex, normal) in vertices.iter_mut().zip(accum.into_iter()) {
        let len_sq = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
        if len_sq > 0.000001 {
            let inv_len = len_sq.sqrt().recip();
            vertex.normal = [
                normal[0] * inv_len,
                normal[1] * inv_len,
                normal[2] * inv_len,
            ];
        } else {
            vertex.normal = [0.0, 1.0, 0.0];
        }
    }
}

fn lerp_u16(value: u16, min: f32, max: f32) -> f32 {
    min + (f32::from(value) / 65535.0) * (max - min)
}

fn normalized_signed_u16(value: u16) -> f32 {
    (f32::from(value) / 65535.0) * 2.0 - 1.0
}

fn read_byte(data: &[u8], cursor: &mut usize) -> Result<u8> {
    let byte = *data
        .get(*cursor)
        .with_context(|| format!("unexpected end of binary LLSD at byte {}", *cursor))?;
    *cursor += 1;
    Ok(byte)
}

fn read_u32_be(data: &[u8], cursor: &mut usize) -> Result<u32> {
    let bytes = read_exact(data, cursor, 4)?;
    Ok(u32::from_be_bytes(
        bytes.try_into().expect("length already checked"),
    ))
}

fn read_i32_be(data: &[u8], cursor: &mut usize) -> Result<i32> {
    Ok(read_u32_be(data, cursor)? as i32)
}

fn read_f64_be(data: &[u8], cursor: &mut usize) -> Result<f64> {
    let bytes = read_exact(data, cursor, 8)?;
    Ok(f64::from_bits(u64::from_be_bytes(
        bytes.try_into().expect("length already checked"),
    )))
}

fn read_uuid(data: &[u8], cursor: &mut usize) -> Result<[u8; 16]> {
    let bytes = read_exact(data, cursor, 16)?;
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(bytes);
    Ok(uuid)
}

fn read_len_prefixed_string(data: &[u8], cursor: &mut usize) -> Result<String> {
    let bytes = read_len_prefixed_bytes(data, cursor)?;
    String::from_utf8(bytes).context("binary LLSD string was not UTF-8")
}

fn read_delimited_string(data: &[u8], cursor: &mut usize, delimiter: u8) -> Result<String> {
    let mut out = Vec::new();
    let mut escaping = false;
    let mut hex_digits_needed = 0usize;
    let mut hex_value = 0u8;

    loop {
        let byte = read_byte(data, cursor)?;
        if hex_digits_needed > 0 {
            let nybble = hex_nybble(byte).with_context(|| {
                format!("binary LLSD delimited string had invalid hex escape byte 0x{byte:02x}")
            })?;
            hex_value = (hex_value << 4) | nybble;
            hex_digits_needed -= 1;
            if hex_digits_needed == 0 {
                out.push(hex_value);
                hex_value = 0;
                escaping = false;
            }
            continue;
        }

        if escaping {
            match byte {
                b'a' => out.push(0x07),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0c),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'v' => out.push(0x0b),
                b'x' => {
                    hex_digits_needed = 2;
                    continue;
                }
                other => out.push(other),
            }
            escaping = false;
            continue;
        }

        if byte == b'\\' {
            escaping = true;
            continue;
        }

        if byte == delimiter {
            break;
        }

        out.push(byte);
    }

    if escaping || hex_digits_needed > 0 {
        anyhow::bail!("binary LLSD delimited string ended during escape sequence");
    }

    String::from_utf8(out).context("binary LLSD delimited string was not UTF-8")
}

fn hex_nybble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn read_len_prefixed_bytes(data: &[u8], cursor: &mut usize) -> Result<Vec<u8>> {
    let len = read_u32_be(data, cursor)? as usize;
    Ok(read_exact(data, cursor, len)?.to_vec())
}

fn read_exact<'a>(data: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .context("binary LLSD cursor overflowed")?;
    let slice = data
        .get(*cursor..end)
        .with_context(|| format!("unexpected end of binary LLSD at byte {}", *cursor))?;
    *cursor = end;
    Ok(slice)
}

pub fn debug_triangle_second_life_mesh_bytes() -> Vec<u8> {
    build_triangle_second_life_mesh_asset("high_lod")
}

fn build_triangle_second_life_mesh_asset(lod_name: &str) -> Vec<u8> {
    let lod_block = encode_binary_llsd_value(&single_triangle_submesh_array());
    let compressed = zlib_compress_bytes(&lod_block);
    let header = BinaryLlsdValue::Map(BTreeMap::from([(
        String::from(lod_name),
        BinaryLlsdValue::Map(BTreeMap::from([
            (String::from("offset"), BinaryLlsdValue::Integer(0)),
            (
                String::from("size"),
                BinaryLlsdValue::Integer(compressed.len() as i32),
            ),
        ])),
    )]));

    let mut bytes = encode_binary_llsd_value(&header);
    bytes.extend_from_slice(&compressed);
    bytes
}

fn single_triangle_submesh_array() -> BinaryLlsdValue {
    BinaryLlsdValue::Array(vec![single_triangle_submesh()])
}

fn single_triangle_submesh() -> BinaryLlsdValue {
    BinaryLlsdValue::Map(BTreeMap::from([
        (
            String::from("PositionDomain"),
            BinaryLlsdValue::Map(BTreeMap::from([
                (
                    String::from("Min"),
                    BinaryLlsdValue::Array(vec![
                        BinaryLlsdValue::Real(-0.5),
                        BinaryLlsdValue::Real(-0.5),
                        BinaryLlsdValue::Real(0.0),
                    ]),
                ),
                (
                    String::from("Max"),
                    BinaryLlsdValue::Array(vec![
                        BinaryLlsdValue::Real(0.5),
                        BinaryLlsdValue::Real(0.5),
                        BinaryLlsdValue::Real(0.0),
                    ]),
                ),
            ])),
        ),
        (
            String::from("TexCoord0Domain"),
            BinaryLlsdValue::Map(BTreeMap::from([
                (
                    String::from("Min"),
                    BinaryLlsdValue::Array(vec![
                        BinaryLlsdValue::Real(0.0),
                        BinaryLlsdValue::Real(0.0),
                    ]),
                ),
                (
                    String::from("Max"),
                    BinaryLlsdValue::Array(vec![
                        BinaryLlsdValue::Real(1.0),
                        BinaryLlsdValue::Real(1.0),
                    ]),
                ),
            ])),
        ),
        (
            String::from("Position"),
            BinaryLlsdValue::Binary(
                [0u16, 0, 0, 65535, 0, 0, 32767, 65535, 0]
                    .into_iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            ),
        ),
        (
            String::from("Normal"),
            BinaryLlsdValue::Binary(
                [
                    32767u16, 32767, 65535, 32767, 32767, 65535, 32767, 32767, 65535,
                ]
                .into_iter()
                .flat_map(|value| value.to_le_bytes())
                .collect(),
            ),
        ),
        (
            String::from("TexCoord0"),
            BinaryLlsdValue::Binary(
                [0u16, 0, 65535, 0, 32767, 65535]
                    .into_iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            ),
        ),
        (
            String::from("TriangleList"),
            BinaryLlsdValue::Binary(
                [0u16, 1, 2]
                    .into_iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            ),
        ),
    ]))
}

fn zlib_compress_bytes(bytes: &[u8]) -> Vec<u8> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).expect("zlib write should succeed");
    encoder.finish().expect("zlib finish should succeed")
}

fn encode_binary_llsd_value(value: &BinaryLlsdValue) -> Vec<u8> {
    let mut out = Vec::new();
    encode_binary_llsd_value_into(value, &mut out);
    out
}

fn encode_binary_llsd_value_into(value: &BinaryLlsdValue, out: &mut Vec<u8>) {
    match value {
        BinaryLlsdValue::Undefined => out.push(b'!'),
        BinaryLlsdValue::Bool(false) => out.push(b'0'),
        BinaryLlsdValue::Bool(true) => out.push(b'1'),
        BinaryLlsdValue::Integer(value) => {
            out.push(b'i');
            out.extend_from_slice(&(*value as u32).to_be_bytes());
        }
        BinaryLlsdValue::Real(value) => {
            out.push(b'r');
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        BinaryLlsdValue::Date(value) => {
            out.push(b'd');
            out.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        BinaryLlsdValue::String(value) => {
            out.push(b's');
            encode_len_prefixed_bytes(value.as_bytes(), out);
        }
        BinaryLlsdValue::Binary(value) => {
            out.push(b'b');
            encode_len_prefixed_bytes(value, out);
        }
        BinaryLlsdValue::Array(values) => {
            out.push(b'[');
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for value in values {
                encode_binary_llsd_value_into(value, out);
            }
            out.push(b']');
        }
        BinaryLlsdValue::Map(values) => {
            out.push(b'{');
            out.extend_from_slice(&(values.len() as u32).to_be_bytes());
            for (key, value) in values {
                out.push(b'k');
                encode_len_prefixed_bytes(key.as_bytes(), out);
                encode_binary_llsd_value_into(value, out);
            }
            out.push(b'}');
        }
        BinaryLlsdValue::Uuid(value) => {
            out.push(b'u');
            out.extend_from_slice(value);
        }
    }
}

fn encode_len_prefixed_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn detects_second_life_mesh_from_binary_llsd_header() {
        let bytes = build_test_second_life_mesh_asset();
        assert_eq!(
            detect_mesh_source_format(&bytes),
            MeshSourceFormat::SecondLifeMesh
        );
    }

    #[test]
    fn decodes_second_life_mesh_asset_into_vertices_and_submeshes() {
        let mesh = load_second_life_mesh(&build_test_second_life_mesh_asset(), 0)
            .expect("test SL mesh should decode");
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.submeshes.len(), 1);
        assert_eq!(mesh.submeshes[0].face_id, 0);
        assert_eq!(mesh.submeshes[0].indices, vec![0, 1, 2]);
        assert_eq!(mesh.vertices[0].tex_coord, [0.0, 0.0]);
        assert!(mesh.aabb.size[0] > 0.4);
        assert!(mesh.aabb.size[1] > 0.4);
    }

    #[test]
    fn decodes_second_life_mesh_asset_with_lod_fallback() {
        let mesh = load_second_life_mesh(&build_test_second_life_mesh_asset_without_high_lod(), 0)
            .expect("fallback LOD should decode");
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.submeshes.len(), 1);
    }

    #[test]
    fn parses_binary_llsd_map_with_quoted_keys_and_values() {
        let bytes = [
            b'{', 0, 0, 0, 1, b'"', b'h', b'i', b'g', b'h', b'_', b'l', b'o', b'd', b'"', b'{', 0,
            0, 0, 2, b'\'', b'o', b'f', b'f', b's', b'e', b't', b'\'', b'i', 0, 0, 0, 0, b'"',
            b's', b'i', b'z', b'e', b'"', b'i', 0, 0, 0, 4, b'}', b'}',
        ];
        let (value, consumed) = parse_binary_llsd(&bytes).expect("quoted map should parse");
        assert_eq!(consumed, bytes.len());
        let header = value.as_map().expect("header should be a map");
        let block = header
            .get("high_lod")
            .and_then(BinaryLlsdValue::as_map)
            .expect("high_lod block should be present");
        assert_eq!(
            block.get("offset").and_then(BinaryLlsdValue::as_i32),
            Some(0)
        );
        assert_eq!(block.get("size").and_then(BinaryLlsdValue::as_i32), Some(4));
    }

    #[test]
    fn parses_binary_llsd_date_token_without_failing_header_walk() {
        let bytes = [
            b'{', 0, 0, 0, 2, b'k', 0, 0, 0, 4, b'd', b'a', b't', b'e', b'd', 0x3f, 0xf0, 0, 0, 0,
            0, 0, 0, b'k', 0, 0, 0, 4, b'n', b'a', b'm', b'e', b's', 0, 0, 0, 4, b't', b'e', b's',
            b't', b'}',
        ];
        let (value, consumed) = parse_binary_llsd(&bytes).expect("date token should parse");
        assert_eq!(consumed, bytes.len());
        let header = value.as_map().expect("header should be a map");
        assert!(matches!(
            header.get("date"),
            Some(BinaryLlsdValue::Date(value)) if (*value - 1.0).abs() < f64::EPSILON
        ));
        assert_eq!(
            header.get("name"),
            Some(&BinaryLlsdValue::String(String::from("test")))
        );
    }

    #[test]
    fn parses_binary_llsd_delimited_string_escapes() {
        let bytes = [
            b'\'', b'h', b'e', b'l', b'l', b'o', b'\\', b'n', b'\\', b'x', b'2', b'1', b'\'',
        ];
        let (value, consumed) = parse_binary_llsd(&bytes).expect("delimited string should parse");
        assert_eq!(consumed, bytes.len());
        assert_eq!(value, BinaryLlsdValue::String(String::from("hello\n!")));
    }

    pub(crate) fn build_test_second_life_mesh_asset() -> Vec<u8> {
        debug_triangle_second_life_mesh_bytes()
    }

    fn build_test_second_life_mesh_asset_without_high_lod() -> Vec<u8> {
        build_triangle_second_life_mesh_asset("medium_lod")
    }
}
