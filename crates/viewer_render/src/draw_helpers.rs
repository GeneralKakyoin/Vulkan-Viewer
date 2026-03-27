use viewer_core::{AlphaMode, RenderableInstance};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bucket {
    Opaque = 0,
    AlphaTest = 1,
    Transparent = 2,
}

#[derive(Debug, Clone)]
pub struct DrawItem {
    pub instance_id: usize,
    pub bucket: Bucket,
    pub distance_sq: f32,
    pub geometry_key: usize, // e.g. MeshKind or dynamic id
}

/// Build a sorted list of instance IDs to be rendered.
///
/// NOTE: `max_objects` acts as a cap on the number of uniform slots (submeshes),
/// but the draw list is truncated by instance count for efficiency.
pub fn build_draw_list(
    instances: &std::collections::BTreeMap<usize, RenderableInstance>,
    visibility_list: &[usize],
    camera_pos: [f32; 3],
    max_objects: usize,
) -> Vec<usize> {
    let mut items = Vec::with_capacity(visibility_list.len());

    for &id in visibility_list {
        let Some(instance) = instances.get(&id) else {
            continue;
        };

        let bucket = match instance.alpha_mode {
            AlphaMode::Opaque => Bucket::Opaque,
            AlphaMode::AlphaTest { .. } => Bucket::AlphaTest,
            AlphaMode::Blend => Bucket::Transparent,
        };

        let dist_sq = calculate_distance_sq(instance.world_aabb.center, camera_pos);

        // Geometry key for sorting stability (opaque only really)
        // For now, we use the raw geometry enum/pointer representation as a proxy
        let geometry_key = match &instance.geometry {
            viewer_core::GeometrySource::Diagnostic(kind) => *kind as usize,
            viewer_core::GeometrySource::Procedural(_params, _detail) => {
                // For simplicity, we use a fixed offset for procedural.
                // In a real viewer, we might hash the params for better batching stability.
                1000
            }
            viewer_core::GeometrySource::Sculpt(uuid, _) => {
                let mut h = 2000usize;
                for b in uuid.as_bytes() {
                    h = h.wrapping_add(*b as usize);
                }
                h
            }
            viewer_core::GeometrySource::Mesh(uuid, lod) => {
                let mut h = 3000usize + (*lod as usize);
                for b in uuid.as_bytes() {
                    h = h.wrapping_add(*b as usize);
                }
                h
            }
        };

        items.push(DrawItem {
            instance_id: id,
            bucket,
            distance_sq: dist_sq,
            geometry_key,
        });
    }

    // Sort by bucket first
    // Then within bucket:
    // Opaque/AlphaTest: Front-to-back (dist_sq asc), then geometry, then instance_id
    // Transparent: Back-to-front (dist_sq desc), then geometry, then instance_id
    items.sort_by(|a, b| {
        if a.bucket != b.bucket {
            return a.bucket.cmp(&b.bucket);
        }

        match a.bucket {
            Bucket::Opaque | Bucket::AlphaTest => {
                // Front-to-back
                a.distance_sq
                    .partial_cmp(&b.distance_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.geometry_key.cmp(&b.geometry_key))
                    .then_with(|| a.instance_id.cmp(&b.instance_id))
            }
            Bucket::Transparent => {
                // Back-to-front
                b.distance_sq
                    .partial_cmp(&a.distance_sq)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.instance_id.cmp(&b.instance_id))
            }
        }
    });

    // Cap
    items.truncate(max_objects);

    items.into_iter().map(|it| it.instance_id).collect()
}

fn calculate_distance_sq(pos: [f32; 3], camera_pos: [f32; 3]) -> f32 {
    let dx = pos[0] - camera_pos[0];
    let dy = pos[1] - camera_pos[1];
    let dz = pos[2] - camera_pos[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use viewer_core::{GeometrySource, InstanceRole, MeshKind, Transform};

    fn mock_instance(alpha_mode: AlphaMode, pos: [f32; 3]) -> RenderableInstance {
        RenderableInstance::new(
            GeometrySource::Diagnostic(MeshKind::Cube),
            InstanceRole::SceneStatic,
            Transform {
                position: pos,
                ..Transform::default()
            },
            [1.0, 1.0, 1.0, 1.0],
            alpha_mode,
        )
    }

    #[test]
    fn test_bucketing_and_sorting() {
        let mut instances = BTreeMap::new();

        // 0: Opaque Far
        instances.insert(0, mock_instance(AlphaMode::Opaque, [0.0, 0.0, 10.0]));
        // 1: Opaque Near
        instances.insert(1, mock_instance(AlphaMode::Opaque, [0.0, 0.0, 2.0]));
        // 2: Transparent Far
        instances.insert(2, mock_instance(AlphaMode::Blend, [0.0, 0.0, 12.0]));
        // 3: Transparent Near
        instances.insert(3, mock_instance(AlphaMode::Blend, [0.0, 0.0, 4.0]));
        // 4: AlphaTest Mid
        instances.insert(
            4,
            mock_instance(AlphaMode::AlphaTest { cutoff: 0.5 }, [0.0, 0.0, 6.0]),
        );

        let visibility = vec![0, 1, 2, 3, 4];
        let camera_pos = [0.0, 0.0, 0.0];

        let draw_list = build_draw_list(&instances, &visibility, camera_pos, 10);

        // Expected order:
        // Opaque: 1 (dist 2), 0 (dist 10)
        // AlphaTest: 4 (dist 6)
        // Transparent: 2 (dist 12), 3 (dist 4) -> Back-to-front

        assert_eq!(draw_list, vec![1, 0, 4, 2, 3]);
    }

    #[test]
    fn test_deterministic_tie_break() {
        let mut instances = BTreeMap::new();

        // Two identical opaque instances at same distance
        instances.insert(10, mock_instance(AlphaMode::Opaque, [0.0, 0.0, 5.0]));
        instances.insert(5, mock_instance(AlphaMode::Opaque, [0.0, 0.0, 5.0]));

        let visibility = vec![10, 5];
        let camera_pos = [0.0, 0.0, 0.0];

        let draw_list = build_draw_list(&instances, &visibility, camera_pos, 10);

        // Should sort by instance_id ASC
        assert_eq!(draw_list, vec![5, 10]);
    }

    #[test]
    fn test_capping() {
        let mut instances = BTreeMap::new();
        for i in 0..10 {
            instances.insert(
                i,
                mock_instance(AlphaMode::Opaque, [0.0, 0.0, (10 - i) as f32]),
            );
        }

        let visibility: Vec<_> = (0..10).collect();
        let camera_pos = [0.0, 0.0, 0.0];

        // Cap to 3 nearest
        let draw_list = build_draw_list(&instances, &visibility, camera_pos, 3);

        // i=9 is at dist 1, i=8 at dist 2, i=7 at dist 3
        assert_eq!(draw_list, vec![9, 8, 7]);
    }

    #[test]
    fn test_transparent_tie_break_is_instance_id() {
        let mut instances = BTreeMap::new();

        // Identical geometry and identical positions => identical distance_sq.
        // The plan requires tie-break by instance_id ASC.
        instances.insert(10, mock_instance(AlphaMode::Blend, [0.0, 0.0, 5.0]));
        instances.insert(5, mock_instance(AlphaMode::Blend, [0.0, 0.0, 5.0]));

        let visibility = vec![10, 5];
        let camera_pos = [0.0, 0.0, 0.0];

        let draw_list = build_draw_list(&instances, &visibility, camera_pos, 10);

        assert_eq!(draw_list, vec![5, 10]);
    }
}
