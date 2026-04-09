use super::*;

pub(super) fn decode_coarse_location_update(payload: &[u8]) -> Option<DecodedCoarseLocationUpdate> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_medium_frequency_message_number(LLUDP_COARSE_LOCATION_UPDATE_MEDIUM_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    let location_count = *body.first()?;
    let needed = 1usize.saturating_add(usize::from(location_count).saturating_mul(3));
    if body.len() < needed {
        return None;
    }
    let count = usize::from(location_count);
    // Message-template shape:
    // [Location variable count:u8][count * (x:u8,y:u8,z:u8)][Index You:i16,Prey:i16][AgentData variable count:u8][count * AgentID:uuid]
    // We decode location entries first (always), then index/agent blocks when present.
    let mut avatars = Vec::with_capacity(count);
    let mut first_location = None;
    let mut second_location = None;
    let mut third_location = None;
    for idx in 0..count {
        let base = 1 + idx * 3;
        let xyz = [body[base], body[base + 1], body[base + 2]];
        if idx == 0 {
            first_location = Some(xyz);
        } else if idx == 1 {
            second_location = Some(xyz);
        } else if idx == 2 {
            third_location = Some(xyz);
        }
        avatars.push(DecodedCoarseAvatar {
            agent_id: None,
            xyz,
        });
    }
    let mut self_index: Option<u16> = None;
    let mut offset = 1usize.saturating_add(count.saturating_mul(3));
    if body.len() >= offset + 4 {
        let you = i16::from_le_bytes(body.get(offset..offset + 2)?.try_into().ok()?);
        let _prey = i16::from_le_bytes(body.get(offset + 2..offset + 4)?.try_into().ok()?);
        if you >= 0 {
            let you_idx = you as usize;
            if you_idx < count {
                self_index = Some(you as u16);
            }
        }
        offset += 4;
    }
    if body.len() > offset {
        let agent_count = usize::from(*body.get(offset)?);
        offset += 1;
        let max_assign = agent_count.min(count);
        if body.len() >= offset + max_assign * 16 {
            for idx in 0..max_assign {
                let raw: [u8; 16] = body
                    .get(offset..offset + 16)
                    .and_then(|s| s.try_into().ok())?;
                offset += 16;
                let id = format_uuid_bytes(raw);
                if !is_null_uuid(&id)
                    && let Some(entry) = avatars.get_mut(idx)
                {
                    entry.agent_id = Some(id);
                }
            }
        }
    }
    Some(DecodedCoarseLocationUpdate {
        location_count,
        first_location,
        second_location,
        third_location,
        avatars,
        self_index,
    })
}

pub(super) fn decode_health_message(payload: &[u8]) -> Option<DecodedHealthMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_HEALTH_MESSAGE_LOW_ID) {
        return None;
    }
    let body = header.body(payload)?;
    let health_bytes: [u8; 4] = body.get(0..4)?.try_into().ok()?;
    let health = f32::from_le_bytes(health_bytes);
    if !health.is_finite() {
        return None;
    }
    Some(DecodedHealthMessage { health })
}

pub(super) fn decode_zerocoded_body(body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len().min(4096));
    let mut i = 0usize;
    while i < body.len() {
        let b = body[i];
        i += 1;
        if b != 0 {
            out.push(b);
            continue;
        }
        let zeros = *body.get(i)? as usize;
        i += 1;
        if out.len().saturating_add(zeros) > MAX_ZEROCODED_BODY_BYTES {
            return None;
        }
        out.extend(std::iter::repeat_n(0u8, zeros));
    }
    Some(out)
}

pub(super) fn quantize_vector3_centi(vec: [f32; 3]) -> Option<[u16; 3]> {
    let mut out = [0u16; 3];
    for (idx, v) in vec.iter().copied().enumerate() {
        if !v.is_finite() {
            return None;
        }
        let scaled = (v * 100.0).round();
        if scaled < 0.0 {
            return None;
        }
        out[idx] = u16::try_from(scaled as i64).ok()?;
    }
    Some(out)
}

pub(super) fn quantize_vector3_signed_centi(vec: [f32; 3]) -> Option<[i32; 3]> {
    let mut out = [0i32; 3];
    for (idx, v) in vec.iter().copied().enumerate() {
        if !v.is_finite() {
            return None;
        }
        let scaled = (v * 100.0).round();
        out[idx] = i32::try_from(scaled as i64).ok()?;
    }
    Some(out)
}

pub(super) fn quantize_quat_i16(quat: [f32; 4]) -> Option<[i16; 4]> {
    let mut out = [0i16; 4];
    for (idx, component) in quat.iter().copied().enumerate() {
        if !component.is_finite() {
            return None;
        }
        let clamped = component.clamp(-1.0, 1.0);
        let scaled = (clamped * 32_767.0).round();
        out[idx] = i16::try_from(scaled as i32).ok()?;
    }
    Some(out)
}

pub(super) fn decode_packed_unit_quaternion_xyz(xyz: [f32; 3]) -> Option<[f32; 4]> {
    if !xyz.iter().all(|component| component.is_finite()) {
        return None;
    }
    let norm_sq = xyz[0] * xyz[0] + xyz[1] * xyz[1] + xyz[2] * xyz[2];
    if norm_sq > 4.0 {
        return None;
    }
    let w_sq = (1.0 - norm_sq).max(0.0);
    let w = w_sq.sqrt();
    Some([xyz[0], xyz[1], xyz[2], w])
}

pub(super) fn read_u8(body: &[u8], offset: &mut usize) -> Option<u8> {
    let v = *body.get(*offset)?;
    *offset += 1;
    Some(v)
}

pub(super) fn read_u16_le(body: &[u8], offset: &mut usize) -> Option<u16> {
    let bytes: [u8; 2] = body.get(*offset..(*offset + 2))?.try_into().ok()?;
    *offset += 2;
    Some(u16::from_le_bytes(bytes))
}

pub(super) fn read_u32_le(body: &[u8], offset: &mut usize) -> Option<u32> {
    let bytes: [u8; 4] = body.get(*offset..(*offset + 4))?.try_into().ok()?;
    *offset += 4;
    Some(u32::from_le_bytes(bytes))
}

pub(super) fn read_u64_le(body: &[u8], offset: &mut usize) -> Option<u64> {
    let bytes: [u8; 8] = body.get(*offset..(*offset + 8))?.try_into().ok()?;
    *offset += 8;
    Some(u64::from_le_bytes(bytes))
}

pub(super) fn read_uuid_bytes(body: &[u8], offset: &mut usize) -> Option<[u8; 16]> {
    let bytes: [u8; 16] = body.get(*offset..(*offset + 16))?.try_into().ok()?;
    *offset += 16;
    Some(bytes)
}

pub(super) fn read_f32_le(body: &[u8], offset: &mut usize) -> Option<f32> {
    let bytes: [u8; 4] = body.get(*offset..(*offset + 4))?.try_into().ok()?;
    *offset += 4;
    Some(f32::from_le_bytes(bytes))
}

pub(super) fn read_vector3f(body: &[u8], offset: &mut usize) -> Option<[f32; 3]> {
    let x = read_f32_le(body, offset)?;
    let y = read_f32_le(body, offset)?;
    let z = read_f32_le(body, offset)?;
    Some([x, y, z])
}

pub(super) fn decode_object_update_ids_and_scales(
    payload: &[u8],
) -> Option<Vec<DecodedObjectFeedIngressObject>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_HIGH_ID) {
        return None;
    }

    let body_raw = header.body(payload)?;
    let body = decode_zerocoded_body(body_raw)?;
    let mut offset = 0usize;

    // RegionData single
    let _region_handle = read_u64_le(&body, &mut offset)?;
    let _time_dilation = read_u16_le(&body, &mut offset)?;

    // ObjectData variable
    let count = read_u8(&body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let local_id = read_u32_le(&body, &mut offset)?;
        offset += 1; // State
        let full_id = read_uuid_bytes(&body, &mut offset)?; // FullID
        offset += 4; // CRC
        offset += 1; // PCode
        offset += 1; // Material
        offset += 1; // ClickAction
        let scale = read_vector3f(&body, &mut offset)?;
        let scale_centi = quantize_vector3_centi(scale);

        // ObjectData (packed), variable 1 (u8 length)
        let object_data_len = read_u8(&body, &mut offset)? as usize;
        let object_data = body.get(offset..offset.checked_add(object_data_len)?)?;
        let position_centi = decode_object_update_object_data_position_centi(object_data);
        offset = offset.checked_add(object_data_len)?;

        // ParentID, UpdateFlags
        offset += 4;
        offset += 4;

        // Path/Profile params (fixed sizes)
        offset += 1; // PathCurve
        offset += 1; // ProfileCurve
        offset += 2; // PathBegin
        offset += 2; // PathEnd
        offset += 1; // PathScaleX
        offset += 1; // PathScaleY
        offset += 1; // PathShearX
        offset += 1; // PathShearY
        offset += 1; // PathTwist
        offset += 1; // PathTwistBegin
        offset += 1; // PathRadiusOffset
        offset += 1; // PathTaperX
        offset += 1; // PathTaperY
        offset += 1; // PathRevolutions
        offset += 1; // PathSkew
        offset += 2; // ProfileBegin
        offset += 2; // ProfileEnd
        offset += 2; // ProfileHollow

        // TextureEntry (var 2), TextureAnim (var 1), NameValue (var 2), Data (var 2), Text (var 1)
        let texture_entry_len = read_u16_le(&body, &mut offset)? as usize;
        let texture_entry = body.get(offset..offset.checked_add(texture_entry_len)?)?;
        let decoded_texture_material = decode_texture_entry_material_data(texture_entry);
        let texture_id_bytes = decoded_texture_material
            .as_ref()
            .and_then(|decoded| decoded.default_face_material.as_ref())
            .and_then(|mat| mat.texture_id_bytes)
            .or_else(|| decode_default_texture_id_from_texture_entry(texture_entry));
        offset = offset.checked_add(texture_entry_len)?;
        let texture_anim_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(texture_anim_len)?;
        let name_value_len = read_u16_le(&body, &mut offset)? as usize;
        offset = offset.checked_add(name_value_len)?;
        let data_len = read_u16_le(&body, &mut offset)? as usize;
        offset = offset.checked_add(data_len)?;
        let text_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(text_len)?;

        // TextColor fixed 4, MediaURL var 1, PSBlock var 1, ExtraParams var 1
        offset += 4;
        let media_url_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(media_url_len)?;
        let ps_len = read_u8(&body, &mut offset)? as usize;
        offset = offset.checked_add(ps_len)?;
        let extra_params_len = read_u8(&body, &mut offset)? as usize;
        let extra_params = body.get(offset..offset.checked_add(extra_params_len)?)?;
        let mesh_id_bytes = decode_mesh_asset_id_from_extra_params(extra_params);
        offset = offset.checked_add(extra_params_len)?;

        // Sound UUID, OwnerID UUID, Gain f32, Flags u8, Radius f32
        offset += 16;
        offset += 16;
        offset += 4;
        offset += 1;
        offset += 4;

        // JointType u8, JointPivot vec3, JointAxisOrAnchor vec3
        offset += 1;
        offset += 12;
        offset += 12;

        let object_id_bytes = if full_id == [0u8; 16] {
            None
        } else {
            Some(full_id)
        };
        out.push(DecodedObjectFeedIngressObject {
            local_id,
            scale_centi,
            position_centi,
            rotation_quat_i16: None,
            mesh_id_bytes,
            texture_id_bytes,
            default_face_material: decoded_texture_material
                .as_ref()
                .and_then(|decoded| decoded.default_face_material.clone()),
            face_material_overrides: decoded_texture_material
                .map(|decoded| decoded.face_material_overrides)
                .unwrap_or_default(),
            object_id_bytes,
        });
    }
    Some(out)
}

// OpenSim/Firestorm ExtraParams IDs used for mesh/sculpt payloads.
pub(super) const EXTRA_PARAM_SCULPT_EP: u16 = 0x30;
const EXTRA_PARAM_MESH_EP: u16 = 0x60;
const SCULPT_TYPE_MASK: u8 = 0x07;
pub(super) const SCULPT_TYPE_MESH: u8 = 0x05;

pub(super) fn decode_mesh_asset_id_from_extra_params(extra_params: &[u8]) -> Option<[u8; 16]> {
    let mut offset = 0usize;
    let count = read_u8(extra_params, &mut offset)? as usize;
    let mut mesh_id = None;

    for _ in 0..count {
        let Some(param_type) = read_u16_le(extra_params, &mut offset) else {
            break;
        };
        let Some(len) =
            read_u32_le(extra_params, &mut offset).and_then(|v| usize::try_from(v).ok())
        else {
            break;
        };
        let Some(end) = offset.checked_add(len) else {
            break;
        };
        let Some(value) = extra_params.get(offset..end) else {
            break;
        };
        offset = end;

        if let Some(decoded) = decode_mesh_id_from_extra_param_entry(param_type, value) {
            mesh_id = Some(decoded);
        }
    }
    mesh_id
}

pub(super) fn decode_mesh_id_from_extra_param_entry(
    param_type: u16,
    value: &[u8],
) -> Option<[u8; 16]> {
    if param_type == EXTRA_PARAM_MESH_EP && value.len() >= 16 {
        let uuid: [u8; 16] = value.get(0..16)?.try_into().ok()?;
        let mesh_flag_ok = value.len() < 17 || (value[16] & SCULPT_TYPE_MASK) == SCULPT_TYPE_MESH;
        if uuid != [0u8; 16] && mesh_flag_ok {
            return Some(uuid);
        }
    }

    if param_type == EXTRA_PARAM_SCULPT_EP && value.len() >= 17 {
        let uuid: [u8; 16] = value.get(0..16)?.try_into().ok()?;
        let sculpt_type = value[16];
        if (sculpt_type & SCULPT_TYPE_MASK) == SCULPT_TYPE_MESH && uuid != [0u8; 16] {
            return Some(uuid);
        }
    }

    None
}

pub(super) fn decode_default_texture_id_from_texture_entry(
    texture_entry: &[u8],
) -> Option<[u8; 16]> {
    let uuid: [u8; 16] = texture_entry.get(0..16)?.try_into().ok()?;
    if uuid == [0u8; 16] {
        return None;
    }
    Some(uuid)
}

#[derive(Debug, Clone, Default)]
pub(super) struct DecodedTextureEntryMaterialData {
    pub(super) default_face_material: Option<ObjectFaceMaterialState>,
    pub(super) face_material_overrides: Vec<(u8, ObjectFaceMaterialState)>,
}

pub(super) fn decode_texture_entry_material_data(
    texture_entry: &[u8],
) -> Option<DecodedTextureEntryMaterialData> {
    if texture_entry.len() < 16 {
        return None;
    }
    let mut offset = 0usize;
    let default_texture =
        read_uuid_bytes(texture_entry, &mut offset).filter(|id| !is_null_uuid_bytes(*id));
    let mut default_face = ObjectFaceMaterialState::with_defaults(default_texture);
    let mut by_face: BTreeMap<u8, ObjectFaceMaterialState> = BTreeMap::new();

    parse_texture_entry_uuid_face_overrides(
        texture_entry,
        &mut offset,
        |mat| &mut mat.texture_id_bytes,
        &mut default_face,
        &mut by_face,
    )?;
    if !parse_texture_entry_byte_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.rgba,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_i16_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.scale_s,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_i16_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.scale_t,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_i16_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.offset_s,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_i16_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.offset_t,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_i16_exceptions(
        texture_entry,
        &mut offset,
        |mat| &mut mat.rotation,
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }

    if let Some(default_material) = read_u8(texture_entry, &mut offset) {
        apply_material_byte(&mut default_face, default_material);
        if !parse_texture_entry_u8_face_overrides(
            texture_entry,
            &mut offset,
            apply_material_byte,
            &default_face,
            &mut by_face,
        ) {
            return finalize_texture_entry_decode(default_face, by_face);
        }
    } else {
        return finalize_texture_entry_decode(default_face, by_face);
    }

    if !parse_texture_entry_u8_exceptions(
        texture_entry,
        &mut offset,
        |mat, value| {
            mat.media_flags = value;
        },
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }
    if !parse_texture_entry_u8_exceptions(
        texture_entry,
        &mut offset,
        |mat, value| {
            mat.glow = value;
        },
        &mut default_face,
        &mut by_face,
    ) {
        return finalize_texture_entry_decode(default_face, by_face);
    }

    // Optional per-face material UUID extension references.
    if texture_entry.len().saturating_sub(offset) >= 16 {
        let _ = parse_texture_entry_uuid_exceptions(
            texture_entry,
            &mut offset,
            |mat| &mut mat.material_id_bytes,
            &mut default_face,
            &mut by_face,
        );
    }

    finalize_texture_entry_decode(default_face, by_face)
}

pub(super) fn finalize_texture_entry_decode(
    default_face: ObjectFaceMaterialState,
    by_face: BTreeMap<u8, ObjectFaceMaterialState>,
) -> Option<DecodedTextureEntryMaterialData> {
    let has_default = default_face.texture_id_bytes.is_some()
        || default_face.material_id_bytes.is_some()
        || default_face.rgba != [255, 255, 255, 255]
        || default_face.normal_id_bytes.is_some()
        || default_face.specular_id_bytes.is_some();
    let face_material_overrides: Vec<(u8, ObjectFaceMaterialState)> = by_face.into_iter().collect();
    if !has_default && face_material_overrides.is_empty() {
        return None;
    }
    Some(DecodedTextureEntryMaterialData {
        default_face_material: has_default.then_some(default_face),
        face_material_overrides,
    })
}

pub(super) fn parse_texture_entry_uuid_exceptions(
    texture_entry: &[u8],
    offset: &mut usize,
    field: impl Fn(&mut ObjectFaceMaterialState) -> &mut Option<[u8; 16]>,
    default_face: &mut ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> Option<()> {
    let default_value =
        read_uuid_bytes(texture_entry, offset).filter(|id| !is_null_uuid_bytes(*id));
    *field(default_face) = default_value;
    loop {
        let face_bits = read_texture_entry_face_bitfield(texture_entry, offset)?;
        if face_bits == 0 {
            break;
        }
        let value = read_uuid_bytes(texture_entry, offset).filter(|id| !is_null_uuid_bytes(*id));
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| {
            *field(mat) = value;
        });
    }
    Some(())
}

pub(super) fn parse_texture_entry_uuid_face_overrides(
    texture_entry: &[u8],
    offset: &mut usize,
    field: impl Fn(&mut ObjectFaceMaterialState) -> &mut Option<[u8; 16]>,
    default_face: &mut ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> Option<()> {
    loop {
        let face_bits = read_texture_entry_face_bitfield(texture_entry, offset)?;
        if face_bits == 0 {
            break;
        }
        let value = read_uuid_bytes(texture_entry, offset).filter(|id| !is_null_uuid_bytes(*id));
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| {
            *field(mat) = value;
        });
    }
    Some(())
}

pub(super) fn parse_texture_entry_byte_exceptions(
    texture_entry: &[u8],
    offset: &mut usize,
    field: impl Fn(&mut ObjectFaceMaterialState) -> &mut [u8; 4],
    default_face: &mut ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> bool {
    let Some(rgba) = read_rgba_bytes(texture_entry, offset) else {
        return false;
    };
    *field(default_face) = rgba;
    loop {
        let Some(face_bits) = read_texture_entry_face_bitfield(texture_entry, offset) else {
            return false;
        };
        if face_bits == 0 {
            break;
        }
        let Some(value) = read_rgba_bytes(texture_entry, offset) else {
            return false;
        };
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| {
            *field(mat) = value;
        });
    }
    true
}

pub(super) fn parse_texture_entry_i16_exceptions(
    texture_entry: &[u8],
    offset: &mut usize,
    field: impl Fn(&mut ObjectFaceMaterialState) -> &mut i16,
    default_face: &mut ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> bool {
    let Some(default_value) = read_i16_le(texture_entry, offset) else {
        return false;
    };
    *field(default_face) = default_value;
    loop {
        let Some(face_bits) = read_texture_entry_face_bitfield(texture_entry, offset) else {
            return false;
        };
        if face_bits == 0 {
            break;
        }
        let Some(value) = read_i16_le(texture_entry, offset) else {
            return false;
        };
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| {
            *field(mat) = value;
        });
    }
    true
}

pub(super) fn parse_texture_entry_u8_exceptions(
    texture_entry: &[u8],
    offset: &mut usize,
    apply: impl Fn(&mut ObjectFaceMaterialState, u8),
    default_face: &mut ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> bool {
    let Some(default_value) = read_u8(texture_entry, offset) else {
        return false;
    };
    apply(default_face, default_value);
    loop {
        let Some(face_bits) = read_texture_entry_face_bitfield(texture_entry, offset) else {
            return false;
        };
        if face_bits == 0 {
            break;
        }
        let Some(value) = read_u8(texture_entry, offset) else {
            return false;
        };
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| apply(mat, value));
    }
    true
}

pub(super) fn parse_texture_entry_u8_face_overrides(
    texture_entry: &[u8],
    offset: &mut usize,
    apply: impl Fn(&mut ObjectFaceMaterialState, u8),
    default_face: &ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
) -> bool {
    loop {
        let Some(face_bits) = read_texture_entry_face_bitfield(texture_entry, offset) else {
            return false;
        };
        if face_bits == 0 {
            break;
        }
        let Some(value) = read_u8(texture_entry, offset) else {
            return false;
        };
        apply_texture_entry_face_bits(face_bits, default_face, by_face, |mat| apply(mat, value));
    }
    true
}

pub(super) fn read_texture_entry_face_bitfield(
    texture_entry: &[u8],
    offset: &mut usize,
) -> Option<u32> {
    let mut bits = 0u32;
    loop {
        let value = read_u8(texture_entry, offset)?;
        bits = (bits << 7) | u32::from(value & 0x7f);
        if (value & 0x80) == 0 {
            return Some(bits);
        }
    }
}

pub(super) fn apply_texture_entry_face_bits(
    face_bits: u32,
    default_face: &ObjectFaceMaterialState,
    by_face: &mut BTreeMap<u8, ObjectFaceMaterialState>,
    mut apply: impl FnMut(&mut ObjectFaceMaterialState),
) {
    for face_id in 0u8..32 {
        if (face_bits & (1u32 << u32::from(face_id))) == 0 {
            continue;
        }
        let entry = by_face
            .entry(face_id)
            .or_insert_with(|| default_face.clone());
        apply(entry);
    }
}

pub(super) fn read_rgba_bytes(body: &[u8], offset: &mut usize) -> Option<[u8; 4]> {
    let rgba: [u8; 4] = body.get(*offset..(*offset + 4))?.try_into().ok()?;
    *offset += 4;
    Some(rgba)
}

pub(super) fn read_i16_le(body: &[u8], offset: &mut usize) -> Option<i16> {
    let bytes: [u8; 2] = body.get(*offset..(*offset + 2))?.try_into().ok()?;
    *offset += 2;
    Some(i16::from_le_bytes(bytes))
}

pub(super) fn apply_material_byte(mat: &mut ObjectFaceMaterialState, value: u8) {
    mat.bump = value & 0x1f;
    mat.fullbright = (value & 0x20) != 0;
    mat.shiny = (value >> 6) & 0x03;
}

pub(super) fn format_object_face_material_with_face(
    face_id: u8,
    mat: &ObjectFaceMaterialState,
) -> DecodedObjectFaceMaterial {
    let mut formatted = format_object_face_material(mat);
    formatted.face_id = u16::from(face_id);
    formatted
}

pub(super) fn format_object_face_material(
    mat: &ObjectFaceMaterialState,
) -> DecodedObjectFaceMaterial {
    DecodedObjectFaceMaterial {
        face_id: 0,
        texture_id: mat
            .texture_id_bytes
            .map(format_uuid_bytes)
            .filter(|id| !is_null_uuid(id)),
        normal_id: mat
            .normal_id_bytes
            .map(format_uuid_bytes)
            .filter(|id| !is_null_uuid(id)),
        specular_id: mat
            .specular_id_bytes
            .map(format_uuid_bytes)
            .filter(|id| !is_null_uuid(id)),
        material_id: mat
            .material_id_bytes
            .map(format_uuid_bytes)
            .filter(|id| !is_null_uuid(id)),
        rgba: mat.rgba,
        offset_s: mat.offset_s,
        offset_t: mat.offset_t,
        scale_s: mat.scale_s,
        scale_t: mat.scale_t,
        rotation: mat.rotation,
        bump: mat.bump,
        fullbright: mat.fullbright,
        shiny: mat.shiny,
        media_flags: mat.media_flags,
        glow: mat.glow,
    }
}

pub(super) fn decode_object_extra_params_mesh_updates(
    payload: &[u8],
) -> Option<Vec<DecodedObjectFeedIngressObject>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_OBJECT_EXTRA_PARAMS_LOW_ID)
    {
        return None;
    }
    let body = decode_maybe_zerocoded_body(payload, header)?;
    let mut offset = 0usize;

    // AgentData block.
    let _agent_id = read_uuid_bytes(&body, &mut offset)?;
    let _session_id = read_uuid_bytes(&body, &mut offset)?;

    let count = read_u8(&body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(16));
    let mut saw_object_block = false;

    for _ in 0..count {
        let Some(local_id) = read_u32_le(&body, &mut offset) else {
            break;
        };
        let Some(param_type) = read_u16_le(&body, &mut offset) else {
            break;
        };
        let Some(in_use) = read_u8(&body, &mut offset) else {
            break;
        };
        let Some(param_size) =
            read_u32_le(&body, &mut offset).and_then(|v| usize::try_from(v).ok())
        else {
            break;
        };
        let Some(param_data_len) = read_u8(&body, &mut offset).map(usize::from) else {
            break;
        };
        let Some(end) = offset.checked_add(param_data_len) else {
            break;
        };
        let Some(param_data) = body.get(offset..end) else {
            break;
        };
        offset = end;
        saw_object_block = true;

        if param_size > param_data.len() {
            continue;
        }
        let param_value = &param_data[..param_size];
        let Some(mesh_id_bytes) = (in_use != 0)
            .then_some(param_value)
            .and_then(|value| decode_mesh_id_from_extra_param_entry(param_type, value))
        else {
            continue;
        };

        out.push(DecodedObjectFeedIngressObject {
            local_id,
            mesh_id_bytes: Some(mesh_id_bytes),
            ..DecodedObjectFeedIngressObject::default()
        });
    }

    if !saw_object_block {
        return None;
    }
    Some(out)
}

pub(super) fn decode_object_update_cached_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_CACHED_HIGH_ID) {
        return None;
    }
    let body = header.body(payload)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let id = read_u32_le(body, &mut offset)?;
        offset += 4; // CRC
        offset += 4; // UpdateFlags
        out.push(id);
    }
    Some(out)
}

pub(super) fn decode_object_update_compressed_objects(
    payload: &[u8],
) -> Option<Vec<DecodedObjectFeedIngressObject>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_OBJECT_UPDATE_COMPRESSED_HIGH_ID) {
        return None;
    }
    let body = header.body(payload)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    let mut saw_object_block = false;
    for _ in 0..count {
        if offset.checked_add(4)? > body.len() {
            break;
        }
        offset += 4; // UpdateFlags
        let Some(data_len) = read_u16_le(body, &mut offset).map(usize::from) else {
            break;
        };
        let Some(end) = offset.checked_add(data_len) else {
            break;
        };
        let Some(data) = body.get(offset..end) else {
            break;
        };
        offset = end;
        saw_object_block = true;
        if let Some(obj) = parse_compressed_object_update_data(data) {
            out.push(obj);
        }
    }
    if !saw_object_block {
        return None;
    }
    Some(out)
}

pub(super) fn decode_improved_terse_object_update_objects(
    payload: &[u8],
) -> Option<Vec<DecodedObjectFeedIngressObject>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_IMPROVED_TERSE_OBJECT_UPDATE_HIGH_ID) {
        return None;
    }
    let body = header.body(payload)?;
    let mut offset = 0usize;
    offset += 8; // RegionHandle
    offset += 2; // TimeDilation
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        let data_len = read_u8(body, &mut offset)? as usize;
        let data = body.get(offset..offset + data_len)?;
        offset += data_len;
        out.push(parse_terse_object_update_data(data)?);
        let tex_len = read_u16_le(body, &mut offset)? as usize;
        offset = offset.checked_add(tex_len)?;
    }
    Some(out)
}

pub(super) fn decode_object_update_object_data_position_centi(
    object_data: &[u8],
) -> Option<[i32; 3]> {
    let pos_offset = match object_data.len() {
        len if len >= 140 => 16,
        len if len >= 124 => 0,
        len if len >= 76 => 16,
        len if len >= 60 => 0,
        _ => return None,
    };
    let mut offset = pos_offset;
    let pos = read_vector3f(object_data, &mut offset)?;
    quantize_vector3_signed_centi(pos)
}

pub(super) fn parse_compressed_object_update_data(
    data: &[u8],
) -> Option<DecodedObjectFeedIngressObject> {
    let mut offset = 0usize;
    let full_id = read_uuid_bytes(data, &mut offset)?;
    let local_id = read_u32_le(data, &mut offset)?;
    offset += 1; // PCode
    offset += 1; // State
    offset += 4; // CRC
    offset += 1; // Material
    offset += 1; // ClickAction
    let scale = read_vector3f(data, &mut offset)?;
    let position = read_vector3f(data, &mut offset)?;
    let rotation_xyz = read_vector3f(data, &mut offset)?;
    let rotation_quat_i16 =
        decode_packed_unit_quaternion_xyz(rotation_xyz).and_then(quantize_quat_i16);
    let compressed_flags = read_u32_le(data, &mut offset)?;
    offset += 16; // OwnerID (present; may be zeroed)
    if compressed_flags & COMPRESSED_FLAG_HAS_ANGULAR_VELOCITY != 0 {
        offset += 12;
    }
    if compressed_flags & COMPRESSED_FLAG_HAS_PARENT != 0 {
        offset += 4;
    }
    if compressed_flags & COMPRESSED_FLAG_HAS_TEXT != 0 {
        skip_c_string(data, &mut offset)?;
        offset += 4; // hover text color
    }
    if compressed_flags & COMPRESSED_FLAG_MEDIA_URL != 0 {
        skip_c_string(data, &mut offset)?;
    }
    if compressed_flags & COMPRESSED_FLAG_HAS_PARTICLES_LEGACY != 0 {
        // OpenSim/Firestorm legacy compressed particle payload has fixed 86-byte shape.
        offset += 86;
    }
    let mesh_id_bytes = parse_mesh_id_from_compressed_extra_params(data, &mut offset);

    Some(DecodedObjectFeedIngressObject {
        local_id,
        scale_centi: quantize_vector3_centi(scale),
        position_centi: quantize_vector3_signed_centi(position),
        rotation_quat_i16,
        mesh_id_bytes,
        texture_id_bytes: None,
        object_id_bytes: if full_id == [0u8; 16] {
            None
        } else {
            Some(full_id)
        },
        ..DecodedObjectFeedIngressObject::default()
    })
}

const COMPRESSED_FLAG_HAS_TEXT: u32 = 0x04;
const COMPRESSED_FLAG_HAS_PARTICLES_LEGACY: u32 = 0x08;
const COMPRESSED_FLAG_HAS_PARENT: u32 = 0x20;
const COMPRESSED_FLAG_HAS_ANGULAR_VELOCITY: u32 = 0x80;
const COMPRESSED_FLAG_MEDIA_URL: u32 = 0x200;

pub(super) fn skip_c_string(data: &[u8], offset: &mut usize) -> Option<()> {
    while *offset < data.len() {
        let value = *data.get(*offset)?;
        *offset += 1;
        if value == 0 {
            return Some(());
        }
    }
    None
}

pub(super) fn read_compressed_extra_params_slice<'a>(
    data: &'a [u8],
    offset: &mut usize,
) -> Option<&'a [u8]> {
    let start = *offset;
    let count = read_u8(data, offset)? as usize;
    if count == 0 {
        return data.get(start..*offset);
    }
    for _ in 0..count {
        read_u16_le(data, offset)?; // type
        let len = usize::try_from(read_u32_le(data, offset)?).ok()?;
        let end = offset.checked_add(len)?;
        data.get(*offset..end)?;
        *offset = end;
    }
    data.get(start..*offset)
}

pub(super) fn parse_mesh_id_from_compressed_extra_params(
    data: &[u8],
    offset: &mut usize,
) -> Option<[u8; 16]> {
    let start = *offset;
    let decoded = read_compressed_extra_params_slice(data, offset)
        .and_then(decode_mesh_asset_id_from_extra_params);
    if decoded.is_none() {
        // Fail-safe: keep object decode alive and avoid speculative mesh-id recovery.
        *offset = start;
    }
    decoded
}

pub(super) fn parse_terse_object_update_data(
    data: &[u8],
) -> Option<DecodedObjectFeedIngressObject> {
    let mut offset = 0usize;
    let local_id = read_u32_le(data, &mut offset)?;
    offset += 1; // state / attachment byte
    let is_avatar = read_u8(data, &mut offset)? != 0;
    if is_avatar {
        offset += 16; // collision plane
    }
    let position = read_vector3f(data, &mut offset)?;
    Some(DecodedObjectFeedIngressObject {
        local_id,
        position_centi: quantize_vector3_signed_centi(position),
        mesh_id_bytes: None,
        rotation_quat_i16: None,
        ..DecodedObjectFeedIngressObject::default()
    })
}

pub(super) fn decode_kill_object_local_ids(payload: &[u8]) -> Option<Vec<u32>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != u32::from(LLUDP_KILL_OBJECT_HIGH_ID) {
        return None;
    }
    let body = header.body(payload)?;
    let mut offset = 0usize;
    let count = read_u8(body, &mut offset)? as usize;
    let mut out = Vec::with_capacity(count.min(32));
    for _ in 0..count {
        out.push(read_u32_le(body, &mut offset)?);
    }
    Some(out)
}

pub(super) fn decode_region_handshake(payload: &[u8]) -> Option<DecodedRegionHandshake> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_REGION_HANDSHAKE_LOW_ID) {
        return None;
    }
    let body = decode_maybe_zerocoded_body(payload, header)?;
    let mut offset = 0usize;
    let region_flags = read_u32_le(&body, &mut offset)?;
    offset += 1; // SimAccess
    let sim_name = read_var_string_u8(&body, &mut offset)?;
    Some(DecodedRegionHandshake {
        region_flags,
        sim_name: if sim_name.trim().is_empty() {
            None
        } else {
            Some(sim_name)
        },
    })
}

pub(super) fn decode_maybe_zerocoded_body(
    payload: &[u8],
    header: FirstSimulatorPacketHeader,
) -> Option<Vec<u8>> {
    let body = header.body(payload)?;
    if payload.first().copied().unwrap_or_default() & LLUDP_ZERO_CODE_FLAG != 0 {
        decode_zerocoded_body(body)
    } else {
        Some(body.to_vec())
    }
}

pub(super) fn read_vector3f_i32(body: &[u8], offset: &mut usize) -> Option<[i32; 3]> {
    if body.len() < *offset + 12 {
        return None;
    }
    let x = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    let y = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    let z = f32::from_le_bytes(body.get(*offset..(*offset + 4))?.try_into().ok()?);
    *offset += 4;
    Some([x.round() as i32, y.round() as i32, z.round() as i32])
}

pub(super) fn decode_agent_movement_complete(
    payload: &[u8],
) -> Option<DecodedAgentMovementComplete> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AGENT_MOVEMENT_COMPLETE_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    let mut offset = 0usize;
    offset += 16; // AgentID
    offset += 16; // SessionID
    let position = read_vector3f_i32(body, &mut offset)?;
    offset += 12; // LookAt
    let region_handle = u64::from_le_bytes(body.get(offset..offset + 8)?.try_into().ok()?);
    Some(DecodedAgentMovementComplete {
        position,
        region_handle,
    })
}

pub(super) fn decode_chat_from_simulator(payload: &[u8]) -> Option<NearbyChatMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_CHAT_FROM_SIMULATOR_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;

    let mut offset = 0usize;
    let from_name_len = usize::from(*body.get(offset)?);
    offset += 1;
    let from_name = std::str::from_utf8(body.get(offset..offset + from_name_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    offset += from_name_len;

    offset += 16; // SourceID
    offset += 16; // OwnerID
    offset += 1; // SourceType
    offset += 1; // ChatType
    offset += 1; // Audible
    offset += 12; // Position

    let msg_len_bytes: [u8; 2] = body.get(offset..offset + 2)?.try_into().ok()?;
    let msg_len = usize::from(u16::from_le_bytes(msg_len_bytes));
    offset += 2;
    let text = std::str::from_utf8(body.get(offset..offset + msg_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();

    Some(NearbyChatMessage {
        sender: if from_name.is_empty() {
            String::from("unknown")
        } else {
            from_name
        },
        text,
        source: String::from("ChatFromSimulator"),
    })
}

pub(super) fn decode_social_events(payload: &[u8]) -> Vec<SocialEvent> {
    let Some(header) = decode_first_simulator_packet_header(payload) else {
        return Vec::new();
    };
    let body = match header.body(payload) {
        Some(v) => v,
        None => return Vec::new(),
    };
    if header.message_number == lludp_low_frequency_message_number(LLUDP_ONLINE_NOTIFICATION_LOW_ID)
    {
        return decode_online_offline_notification_events(body, true);
    }
    if header.message_number
        == lludp_low_frequency_message_number(LLUDP_OFFLINE_NOTIFICATION_LOW_ID)
    {
        return decode_online_offline_notification_events(body, false);
    }
    if header.message_number == lludp_low_frequency_message_number(LLUDP_CHANGE_USER_RIGHTS_LOW_ID)
    {
        return decode_change_user_rights_events(body);
    }
    if header.message_number
        == lludp_low_frequency_message_number(LLUDP_IMPROVED_INSTANT_MESSAGE_LOW_ID)
    {
        return decode_improved_instant_message_event(body)
            .into_iter()
            .collect();
    }
    Vec::new()
}

pub(super) fn decode_online_offline_notification_events(
    body: &[u8],
    online: bool,
) -> Vec<SocialEvent> {
    let mut out = Vec::new();
    let count = match body.first() {
        Some(v) => usize::from(*v),
        None => return out,
    };
    let mut offset = 1usize;
    for _ in 0..count {
        let raw: [u8; 16] = match body
            .get(offset..offset + 16)
            .and_then(|s| s.try_into().ok())
        {
            Some(v) => v,
            None => break,
        };
        offset += 16;
        let agent_id = format_uuid_bytes(raw);
        if online {
            out.push(SocialEvent::FriendOnline { agent_id });
        } else {
            out.push(SocialEvent::FriendOffline { agent_id });
        }
    }
    out
}

pub(super) fn decode_change_user_rights_events(body: &[u8]) -> Vec<SocialEvent> {
    let mut out = Vec::new();
    if body.len() < 17 {
        return out;
    }
    let agent_id = match body.get(0..16).and_then(|s| s.try_into().ok()) {
        Some(raw) => format_uuid_bytes(raw),
        None => return out,
    };
    let rights_count = usize::from(body[16]);
    let mut offset = 17usize;
    for _ in 0..rights_count {
        let related_id = match body
            .get(offset..offset + 16)
            .and_then(|s| s.try_into().ok())
        {
            Some(raw) => format_uuid_bytes(raw),
            None => break,
        };
        offset += 16;
        let rights = match body
            .get(offset..offset + 4)
            .and_then(|s| s.try_into().ok())
            .map(i32::from_le_bytes)
        {
            Some(v) => v,
            None => break,
        };
        offset += 4;
        out.push(SocialEvent::FriendRights {
            agent_id: agent_id.clone(),
            related_id,
            rights,
        });
    }
    out
}

pub(super) fn decode_improved_instant_message_event(body: &[u8]) -> Option<SocialEvent> {
    if body.len() < 32 + 1 + 16 + 4 + 16 + 12 + 1 + 1 + 16 + 4 + 1 + 2 + 2 {
        return None;
    }
    let mut offset = 0usize;
    let from_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 16; // agent session id
    offset += 1; // from_group
    let to_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 4; // parent estate
    offset += 16; // region id
    offset += 12; // position
    offset += 1; // offline
    let dialog = *body.get(offset)?;
    offset += 1;
    let session_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let timestamp = u32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);
    offset += 4;
    let from_name_len = usize::from(*body.get(offset)?);
    offset += 1;
    let from_name = std::str::from_utf8(body.get(offset..offset + from_name_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    offset += from_name_len;
    let message_len = usize::from(u16::from_le_bytes(
        body.get(offset..offset + 2)?.try_into().ok()?,
    ));
    offset += 2;
    let message = std::str::from_utf8(body.get(offset..offset + message_len)?)
        .ok()?
        .trim_end_matches('\0')
        .to_string();
    Some(SocialEvent::DirectIm(DirectImPayload {
        from_id,
        to_id,
        session_id,
        from_name,
        message,
        dialog,
        timestamp,
    }))
}

pub(super) fn decode_legacy_avatar_properties_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfileData> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AVATAR_PROPERTIES_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 68 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let avatar_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !avatar_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let sl_image_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let fl_image_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let partner_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;

    let sl_about_text = read_var_string_u16(body, &mut offset)?;
    let fl_about_text = read_var_string_u8(body, &mut offset)?;
    let born_on = read_var_string_u8(body, &mut offset)?;
    let profile_url = read_var_string_u8(body, &mut offset)?;
    let _caption = read_var_string_u8(body, &mut offset)?;
    let flags = i32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);

    Some(AgentProfileData {
        id: avatar_id,
        profile_url: if profile_url.is_empty() {
            None
        } else {
            Some(profile_url)
        },
        sl_about_text,
        fl_about_text,
        notes: String::new(),
        sl_image_id: if is_null_uuid(&sl_image_id) {
            None
        } else {
            Some(sl_image_id)
        },
        fl_image_id: if is_null_uuid(&fl_image_id) {
            None
        } else {
            Some(fl_image_id)
        },
        partner_id: if is_null_uuid(&partner_id) {
            None
        } else {
            Some(partner_id)
        },
        member_since: if born_on.is_empty() {
            None
        } else {
            Some(born_on)
        },
        online: Some((flags & (1 << 4)) != 0),
        allow_publish: Some((flags & (1 << 0)) != 0),
        identified: Some((flags & (1 << 2)) != 0),
        transacted: Some((flags & (1 << 3)) != 0),
        display_name: None,
        username: None,
        groups: Vec::new(),
        picks: Vec::new(),
        pick_details: Vec::new(),
        classifieds: Vec::new(),
        classified_details: Vec::new(),
    })
}

pub(super) fn decode_legacy_avatar_groups_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfileGroup>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_GROUPS_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 33 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let avatar_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !avatar_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let group_count = usize::from(*body.get(offset)?);
    offset += 1;
    let mut groups = Vec::new();
    for _ in 0..group_count {
        if body.len() < offset + 8 + 1 + 16 + 16 {
            break;
        }
        offset += 8; // GroupPowers
        offset += 1; // AcceptNotices
        let _group_title = read_var_string_u8(body, &mut offset)?;
        let group_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let group_name = read_var_string_u8(body, &mut offset)?;
        let insignia_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        groups.push(AgentProfileGroup {
            id: group_id,
            name: group_name,
            image_id: if is_null_uuid(&insignia_id) {
                None
            } else {
                Some(insignia_id)
            },
        });
    }
    Some(groups)
}

pub(super) fn decode_legacy_avatar_notes_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<String> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_NOTES_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 32 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    read_var_string_u16(body, &mut offset)
}

pub(super) fn decode_legacy_avatar_picks_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfilePick>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_AVATAR_PICKS_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 33 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let mut picks = Vec::new();
    while offset < body.len() {
        if body.len() < offset + 16 {
            break;
        }
        let pick_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let pick_name = read_var_string_u8(body, &mut offset)?;
        picks.push(AgentProfilePick {
            id: pick_id,
            name: pick_name,
        });
    }
    Some(picks)
}

pub(super) fn decode_legacy_pick_info_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfilePickDetails> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number != lludp_low_frequency_message_number(LLUDP_PICK_INFO_REPLY_LOW_ID) {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 16 + 16 + 1 + 16 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let pick_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let creator_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !creator_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    offset += 1; // top_pick
    let parcel_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let name = read_var_string_u8(body, &mut offset)?;
    let desc = read_var_string_u16(body, &mut offset)?;
    let snapshot_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let _user = read_var_string_u8(body, &mut offset)?;
    let _original_name = read_var_string_u8(body, &mut offset)?;
    let sim_name = read_var_string_u8(body, &mut offset)?;
    let global_position = read_vector3d_i32(body, &mut offset)?;
    Some(AgentProfilePickDetails {
        id: pick_id,
        name: if name.is_empty() { None } else { Some(name) },
        description: if desc.is_empty() { None } else { Some(desc) },
        snapshot_id: if is_null_uuid(&snapshot_id) {
            None
        } else {
            Some(snapshot_id)
        },
        parcel_id: if is_null_uuid(&parcel_id) {
            None
        } else {
            Some(parcel_id)
        },
        sim_name: if sim_name.is_empty() {
            None
        } else {
            Some(sim_name)
        },
        parcel_name: None,
        global_position: Some(global_position),
    })
}

pub(super) fn decode_legacy_avatar_classifieds_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<Vec<AgentProfileClassified>> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_AVATAR_CLASSIFIED_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 32 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let target_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !target_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    let mut classifieds = Vec::new();
    while offset < body.len() {
        if body.len() < offset + 16 {
            break;
        }
        let classified_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
        offset += 16;
        let name = read_var_string_u8(body, &mut offset)?;
        classifieds.push(AgentProfileClassified {
            id: classified_id,
            name,
        });
    }
    Some(classifieds)
}

pub(super) fn decode_legacy_classified_info_reply(
    payload: &[u8],
    expected_avatar_id: &str,
) -> Option<AgentProfileClassifiedDetails> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_CLASSIFIED_INFO_REPLY_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    if body.len() < 16 + 16 + 4 + 4 + 4 {
        return None;
    }
    let mut offset = 0usize;
    offset += 16; // AgentID
    let classified_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let creator_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    if !expected_avatar_id.is_empty() && !creator_id.eq_ignore_ascii_case(expected_avatar_id) {
        return None;
    }
    offset += 16;
    offset += 4; // creation date
    offset += 4; // expiration date
    let category = u32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);
    offset += 4;
    let name = read_var_string_u8(body, &mut offset)?;
    let desc = read_var_string_u16(body, &mut offset)?;
    let parcel_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    offset += 4; // parent estate
    let snapshot_id = format_uuid_bytes(body.get(offset..offset + 16)?.try_into().ok()?);
    offset += 16;
    let sim_name = read_var_string_u8(body, &mut offset)?;
    let global_position = read_vector3d_i32(body, &mut offset)?;
    let parcel_name = read_var_string_u8(body, &mut offset)?;
    let flags = *body.get(offset)?;
    offset += 1;
    let price_for_listing = i32::from_le_bytes(body.get(offset..offset + 4)?.try_into().ok()?);

    Some(AgentProfileClassifiedDetails {
        id: classified_id,
        name: if name.is_empty() { None } else { Some(name) },
        description: if desc.is_empty() { None } else { Some(desc) },
        snapshot_id: if is_null_uuid(&snapshot_id) {
            None
        } else {
            Some(snapshot_id)
        },
        parcel_id: if is_null_uuid(&parcel_id) {
            None
        } else {
            Some(parcel_id)
        },
        sim_name: if sim_name.is_empty() {
            None
        } else {
            Some(sim_name)
        },
        parcel_name: if parcel_name.is_empty() {
            None
        } else {
            Some(parcel_name)
        },
        global_position: Some(global_position),
        category: Some(category),
        flags: Some(flags),
        price_for_listing: Some(price_for_listing),
    })
}

pub(super) fn read_var_string_u8(body: &[u8], offset: &mut usize) -> Option<String> {
    let len = usize::from(*body.get(*offset)?);
    *offset += 1;
    let text = std::str::from_utf8(body.get(*offset..(*offset + len))?).ok()?;
    *offset += len;
    Some(text.trim_end_matches('\0').to_string())
}

pub(super) fn read_var_string_u16(body: &[u8], offset: &mut usize) -> Option<String> {
    let len = usize::from(u16::from_le_bytes(
        body.get(*offset..(*offset + 2))?.try_into().ok()?,
    ));
    *offset += 2;
    let text = std::str::from_utf8(body.get(*offset..(*offset + len))?).ok()?;
    *offset += len;
    Some(text.trim_end_matches('\0').to_string())
}

pub(super) fn read_vector3d_i32(body: &[u8], offset: &mut usize) -> Option<[i32; 3]> {
    if body.len() < *offset + 24 {
        return None;
    }
    let x = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    let y = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    let z = f64::from_le_bytes(body.get(*offset..(*offset + 8))?.try_into().ok()?);
    *offset += 8;
    Some([x.round() as i32, y.round() as i32, z.round() as i32])
}

pub(super) fn is_null_uuid(uuid: &str) -> bool {
    uuid == "00000000-0000-0000-0000-000000000000"
}

pub(super) fn is_null_uuid_bytes(uuid: [u8; 16]) -> bool {
    uuid == [0u8; 16]
}

pub(super) fn decode_simulator_viewer_time_message(
    payload: &[u8],
) -> Option<DecodedSimulatorViewerTimeMessage> {
    let header = decode_first_simulator_packet_header(payload)?;
    if header.message_number
        != lludp_low_frequency_message_number(LLUDP_SIMULATOR_VIEWER_TIME_LOW_ID)
    {
        return None;
    }
    let body = header.body(payload)?;
    let body_len = u16::try_from(body.len()).ok()?;
    let signature = body.get(0..4).and_then(|bytes| {
        let raw: [u8; 4] = bytes.try_into().ok()?;
        Some(u32::from_le_bytes(raw))
    });
    Some(DecodedSimulatorViewerTimeMessage {
        body_len,
        signature,
    })
}

pub(super) fn health_to_basis_points(value: f32) -> u16 {
    let normalized = if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    };
    (normalized * 10_000.0).round() as u16
}
