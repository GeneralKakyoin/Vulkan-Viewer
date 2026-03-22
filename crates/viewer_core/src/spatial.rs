use crate::{Aabb, Frustum, IntersectionResult};

/// Maximum number of items in a node before it attempts to split.
const MAX_CAPACITY: usize = 8;
/// Minimum half-extent size for a node; prevents infinite splitting for deeply stacked identical items.
const MIN_SIZE: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct OctreeNode {
    pub center: [f32; 3],
    pub size: [f32; 3], // half-extents
    pub items: Vec<(usize, Aabb)>, // (Instance ID, Aabb)
    pub children: Option<Box<[OctreeNode; 8]>>,
}

impl OctreeNode {
    pub fn new(center: [f32; 3], size: [f32; 3]) -> Self {
        Self {
            center,
            size,
            items: Vec::new(),
            children: None,
        }
    }

    /// Determines which octant (0-7) a position falls into relative to this node's center.
    /// Bit 0: X > centerX
    /// Bit 1: Y > centerY
    /// Bit 2: Z > centerZ
    pub fn get_octant(center: &[f32; 3], pos: &[f32; 3]) -> usize {
        let mut octant = 0;
        if pos[0] > center[0] { octant |= 1; }
        if pos[1] > center[1] { octant |= 2; }
        if pos[2] > center[2] { octant |= 4; }
        octant
    }

    /// Checks if a child completely contains the given Aabb.
    fn child_contains(child_center: &[f32; 3], child_size: &[f32; 3], aabb: &Aabb) -> bool {
        let c_min = [
            child_center[0] - child_size[0],
            child_center[1] - child_size[1],
            child_center[2] - child_size[2],
        ];
        let c_max = [
            child_center[0] + child_size[0],
            child_center[1] + child_size[1],
            child_center[2] + child_size[2],
        ];
        
        let a_min = aabb.min();
        let a_max = aabb.max();

        a_min[0] >= c_min[0] && a_max[0] <= c_max[0] &&
        a_min[1] >= c_min[1] && a_max[1] <= c_max[1] &&
        a_min[2] >= c_min[2] && a_max[2] <= c_max[2]
    }

    pub fn insert(&mut self, instance_id: usize, aabb: Aabb) {
        // If we have children, check if this fits entirely into one of them.
        if let Some(ref mut children) = self.children {
            let octant = Self::get_octant(&self.center, &aabb.center);
            let child = &mut children[octant];
            if Self::child_contains(&child.center, &child.size, &aabb) {
                child.insert(instance_id, aabb);
                return;
            }
        }

        // Doesn't fit cleanly in a child, or we don't have children yet.
        self.items.push((instance_id, aabb));

        // Should we split?
        if self.children.is_none() && self.items.len() >= MAX_CAPACITY && self.size[0] > MIN_SIZE && self.size[1] > MIN_SIZE && self.size[2] > MIN_SIZE {
            self.split();
        }
    }

    fn split(&mut self) {
        let half_size = [self.size[0] * 0.5, self.size[1] * 0.5, self.size[2] * 0.5];
        let mut children: [_; 8] = core::array::from_fn(|i| {
            let offset_x = if (i & 1) != 0 { half_size[0] } else { -half_size[0] };
            let offset_y = if (i & 2) != 0 { half_size[1] } else { -half_size[1] };
            let offset_z = if (i & 4) != 0 { half_size[2] } else { -half_size[2] };
            
            OctreeNode::new([
                self.center[0] + offset_x,
                self.center[1] + offset_y,
                self.center[2] + offset_z,
            ], half_size)
        });

        // Redistribute existing items
        let mut keep = Vec::new();
        let old_items = core::mem::take(&mut self.items);
        for (id, aabb) in old_items {
            let octant = Self::get_octant(&self.center, &aabb.center);
            let child = &mut children[octant];
            if Self::child_contains(&child.center, &child.size, &aabb) {
                child.insert(id, aabb);
            } else {
                keep.push((id, aabb));
            }
        }
        self.items = keep;
        self.children = Some(Box::new(children));
    }

    pub fn remove(&mut self, instance_id: usize, aabb: Aabb) -> bool {
        // Optimization: skip if node bounds don't contain item center (or could use full AABB check)
        // For Milestone 1, we keep it simple but check children only if they could contain the AABB.
        
        // First check locally
        if let Some(idx) = self.items.iter().position(|&(id, _)| id == instance_id) {
            self.items.swap_remove(idx);
            return true;
        }

        // If not found locally, check child that would contain it
        if let Some(ref mut children) = self.children {
            let octant = Self::get_octant(&self.center, &aabb.center);
            let child = &mut children[octant];
            if child.remove(instance_id, aabb) {
                return true;
            }
            
            // Fallback: search all children if it might have straddled
            for (i, c) in children.iter_mut().enumerate() {
                if i == octant { continue; }
                if c.remove(instance_id, aabb) {
                    return true;
                }
            }
        }

        false
    }
    
    pub fn collect_all(&self, items: &mut Vec<usize>) {
        for (id, _) in &self.items {
            items.push(*id);
        }
        if let Some(ref children) = self.children {
            for child in children.iter() {
                child.collect_all(items);
            }
        }
    }

    pub fn query_frustum(&self, frustum: &Frustum, visible_items: &mut Vec<usize>) {
        let node_aabb = Aabb::new(self.center, self.size);
        let intersection = frustum.contains_aabb(&node_aabb);

        match intersection {
            IntersectionResult::Outside => {
                // Node is completely culled, ignore it and all children
                return;
            }
            IntersectionResult::Inside => {
                // Node is completely inside, add all items recursively without further checks
                self.collect_all(visible_items);
            }
            IntersectionResult::Intersecting => {
                // Node intersects the frustum; check individual items
                for (id, aabb) in &self.items {
                    if frustum.contains_aabb(aabb) != IntersectionResult::Outside {
                        visible_items.push(*id);
                    }
                }

                // Recurse into children
                if let Some(ref children) = self.children {
                    for child in children.iter() {
                        child.query_frustum(frustum, visible_items);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Octree {
    pub root: OctreeNode,
}

impl Octree {
    pub fn new(world_size: f32) -> Self {
        Self {
            root: OctreeNode::new([0.0, 0.0, 0.0], [world_size, world_size, world_size]),
        }
    }

    pub fn insert(&mut self, instance_id: usize, aabb: Aabb) {
        self.root.insert(instance_id, aabb);
    }

    pub fn remove(&mut self, instance_id: usize, aabb: Aabb) -> bool {
        self.root.remove(instance_id, aabb)
    }
    
    pub fn update(&mut self, instance_id: usize, old_aabb: Aabb, new_aabb: Aabb) {
        // For Milestone 1, we just remove and re-insert.
        // A future optimization could check if they are in the same node.
        self.remove(instance_id, old_aabb);
        self.insert(instance_id, new_aabb);
    }

    pub fn query_frustum(&self, frustum: &Frustum) -> Vec<usize> {
        let mut visible_items = Vec::new();
        self.root.query_frustum(frustum, &mut visible_items);
        visible_items
    }
}
