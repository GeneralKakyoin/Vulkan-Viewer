use super::*;

pub(super) fn map_net_face_material_to_core(
    face: &viewer_net::DecodedObjectFaceMaterial,
) -> viewer_core::DecodedWorldObjectFaceMaterial {
    viewer_core::DecodedWorldObjectFaceMaterial {
        face_id: face.face_id,
        texture_id: face
            .texture_id
            .as_deref()
            .map(viewer_core::AssetID::new)
            .filter(|id| !id.is_empty()),
        normal_id: face
            .normal_id
            .as_deref()
            .map(viewer_core::AssetID::new)
            .filter(|id| !id.is_empty()),
        specular_id: face
            .specular_id
            .as_deref()
            .map(viewer_core::AssetID::new)
            .filter(|id| !id.is_empty()),
        material_id: face
            .material_id
            .as_deref()
            .map(viewer_core::AssetID::new)
            .filter(|id| !id.is_empty()),
        rgba: face.rgba,
        offset_s: face.offset_s,
        offset_t: face.offset_t,
        scale_s: face.scale_s,
        scale_t: face.scale_t,
        rotation: face.rotation,
        bump: face.bump,
        fullbright: face.fullbright,
        shiny: face.shiny,
        media_flags: face.media_flags,
        glow: face.glow,
    }
}

pub(super) fn summarize_decoded_object_feed_mesh_ids(
    snapshot: &LiveVisualSnapshot,
    sample_cap: usize,
) -> (usize, String) {
    let mesh_ids = extract_decoded_object_feed_mesh_ids(
        snapshot,
        snapshot.decoded_object_feed_objects.len().max(sample_cap),
    );
    let sample = if mesh_ids.is_empty() {
        String::from("none")
    } else {
        mesh_ids
            .iter()
            .take(sample_cap)
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    (mesh_ids.len(), sample)
}

pub(super) fn summarize_decoded_object_feed_texture_ids(
    snapshot: &LiveVisualSnapshot,
    sample_cap: usize,
) -> (usize, String) {
    let texture_ids = extract_decoded_object_feed_texture_ids(
        snapshot,
        snapshot.decoded_object_feed_objects.len(),
    );
    let sample = if texture_ids.is_empty() {
        String::from("none")
    } else {
        texture_ids
            .iter()
            .take(sample_cap)
            .map(|id| id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    };
    (texture_ids.len(), sample)
}

pub(super) fn count_exported_object_feed_mesh_objects(snapshot: &LiveVisualSnapshot) -> usize {
    snapshot
        .decoded_object_feed_objects
        .iter()
        .filter(|obj| obj.mesh_id.is_some())
        .count()
}

pub(super) fn summarize_decoded_object_feed_face_materials(
    snapshot: &LiveVisualSnapshot,
) -> (usize, usize, usize) {
    let mut objects_with_default = 0usize;
    let mut objects_with_overrides = 0usize;
    let mut override_faces = 0usize;
    for obj in &snapshot.decoded_object_feed_objects {
        if obj.default_face_material.is_some() {
            objects_with_default += 1;
        }
        if !obj.face_material_overrides.is_empty() {
            objects_with_overrides += 1;
            override_faces += obj.face_material_overrides.len();
        }
    }
    (objects_with_default, objects_with_overrides, override_faces)
}

pub(super) fn summarize_decoded_object_feed_parenting(
    snapshot: &LiveVisualSnapshot,
) -> (usize, usize) {
    let parented = snapshot
        .decoded_object_feed_objects
        .iter()
        .filter(|obj| obj.parent_local_id.is_some())
        .count();
    let root = snapshot
        .decoded_object_feed_objects
        .len()
        .saturating_sub(parented);
    (root, parented)
}

pub(super) fn format_object_feed_state_export_counts(
    snapshot: &LiveVisualSnapshot,
    decode: &viewer_net::SimulatorPayloadDecodeSummary,
) -> String {
    let export_mesh_objects = count_exported_object_feed_mesh_objects(snapshot);
    format!(
        "state_total_objects={} export_objects={} state_mesh_objects={} export_mesh_objects={} export_truncated={}",
        snapshot.decoded_object_feed_total_objects,
        snapshot.decoded_object_feed_objects.len(),
        decode.object_feed_state_mesh_objects,
        export_mesh_objects,
        snapshot.decoded_object_feed_export_truncated
    )
}

pub(super) fn format_object_feed_message_family_counts(
    decode: &viewer_net::SimulatorPayloadDecodeSummary,
) -> String {
    format!(
        "ObjectUpdate:{},ObjectUpdateCompressed:{},ObjectExtraParams:{},ImprovedTerseObjectUpdate:{}",
        decode.object_feed_object_update_messages,
        decode.object_feed_object_update_compressed_messages,
        decode.object_feed_object_extra_params_messages,
        decode.object_feed_improved_terse_messages,
    )
}

pub(super) fn format_object_feed_mesh_hit_counts(
    decode: &viewer_net::SimulatorPayloadDecodeSummary,
) -> String {
    format!(
        "ObjectUpdate:{},ObjectUpdateCompressed:{},ObjectExtraParams:{}",
        decode.object_feed_object_update_mesh_hits,
        decode.object_feed_object_update_compressed_mesh_hits,
        decode.object_feed_object_extra_params_mesh_hits,
    )
}

pub(super) fn emit_object_feed_startup_summary(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    snapshot: &LiveVisualSnapshot,
    connection: &Connection,
) {
    let decode = connection.simulator_payload_decode_summary();
    let (mesh_id_count, mesh_sample) = summarize_decoded_object_feed_mesh_ids(snapshot, 6);
    let (texture_id_count, texture_sample) = summarize_decoded_object_feed_texture_ids(snapshot, 6);
    let (objects_with_default, objects_with_overrides, override_faces) =
        summarize_decoded_object_feed_face_materials(snapshot);
    let (root_objects, parented_objects) = summarize_decoded_object_feed_parenting(snapshot);
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "object_feed",
        &format!(
            "startup: {} {} {} mesh_id_count={} sample_mesh_ids={}",
            format_object_feed_state_export_counts(snapshot, decode),
            format_object_feed_message_family_counts(decode),
            format_object_feed_mesh_hit_counts(decode),
            mesh_id_count,
            mesh_sample
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "object_feed",
        &format!(
            "startup textures: texture_id_count={} sample_texture_ids={}",
            texture_id_count, texture_sample
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "object_feed",
        &format!(
            "startup face_materials: objects_with_default={} objects_with_overrides={} override_faces={}",
            objects_with_default, objects_with_overrides, override_faces
        ),
    );
    emit_relay(
        tx,
        RuntimeRelayLevel::Info,
        "object_feed",
        &format!(
            "startup parenting: root_objects={} parented_objects={}",
            root_objects, parented_objects
        ),
    );
    persist_object_face_checklist_candidates(snapshot);
}

pub(super) fn emit_object_feed_tick_summary(
    tx: &mpsc::Sender<LiveFeedUpdate>,
    snapshot: &LiveVisualSnapshot,
    connection: &Connection,
) {
    let decode = connection.simulator_payload_decode_summary();
    let (objects_with_default, objects_with_overrides, override_faces) =
        summarize_decoded_object_feed_face_materials(snapshot);
    let (mesh_id_count, mesh_sample) = summarize_decoded_object_feed_mesh_ids(snapshot, 6);
    let (texture_id_count, texture_sample) = summarize_decoded_object_feed_texture_ids(snapshot, 6);
    let (root_objects, parented_objects) = summarize_decoded_object_feed_parenting(snapshot);
    let level = RuntimeRelayLevel::Info;
    emit_relay(
        tx,
        level,
        "object_feed",
        &format!(
            "tick: {} {} {} mesh_id_count={} sample_mesh_ids={}",
            format_object_feed_state_export_counts(snapshot, decode),
            format_object_feed_message_family_counts(decode),
            format_object_feed_mesh_hit_counts(decode),
            mesh_id_count,
            mesh_sample
        ),
    );
    emit_relay(
        tx,
        level,
        "object_feed",
        &format!(
            "tick textures: texture_id_count={} sample_texture_ids={}",
            texture_id_count, texture_sample
        ),
    );
    emit_relay(
        tx,
        level,
        "object_feed",
        &format!(
            "tick face_materials: objects_with_default={} objects_with_overrides={} override_faces={}",
            objects_with_default, objects_with_overrides, override_faces
        ),
    );
    emit_relay(
        tx,
        level,
        "object_feed",
        &format!(
            "tick parenting: root_objects={} parented_objects={}",
            root_objects, parented_objects
        ),
    );
    persist_object_face_checklist_candidates(snapshot);
}

pub(super) fn persist_object_face_checklist_candidates(snapshot: &LiveVisualSnapshot) {
    let path = PathBuf::from("artifacts/logs/object_face_checklist_autodiscover.jsonl");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    for obj in &snapshot.decoded_object_feed_objects {
        let object_id = obj.object_id.as_deref().unwrap_or("none");
        let face_ids = if obj.face_material_overrides.is_empty() {
            String::from("none")
        } else {
            obj.face_material_overrides
                .iter()
                .map(|face| face.face_id.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let line = format!(
            "{{\"local_id\":{},\"object_id\":\"{}\",\"has_default\":{},\"override_count\":{},\"override_faces\":\"{}\"}}\n",
            obj.local_id,
            object_id.replace('"', ""),
            obj.default_face_material.is_some(),
            obj.face_material_overrides.len(),
            face_ids
        );
        let _ = file.write_all(line.as_bytes());
    }
}
