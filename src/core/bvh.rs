use std::{collections::HashMap, sync::Arc, time::Instant};

use glam::Vec3;

use crate::core::mesh::{MeshInstance, MeshUniform, Vertex};

#[derive(Debug, Copy, Clone, Default)]
pub struct BVHTriangle {
    pub centroid: Vec3,
    pub min: Vec3,
    pub max: Vec3,
    pub i: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PackedTriangle {
    pub v1: [f32; 3],
    pub uv10: f32,
    pub v2: [f32; 3],
    pub uv11: f32,
    pub v3: [f32; 3],
    pub uv20: f32,
    pub n1: [f32; 3],
    pub uv21: f32,
    pub n2: [f32; 3],
    pub uv30: f32,
    pub n3: [f32; 3],
    pub uv31: f32,
}

impl PackedTriangle {
    pub fn new(v1: Vertex, v2: Vertex, v3: Vertex) -> Self {
        Self {
            v1: v1.pos.to_array(),
            v2: v2.pos.to_array(),
            v3: v3.pos.to_array(),
            n1: v1.normal.to_array(),
            n2: v2.normal.to_array(),
            n3: v3.normal.to_array(),
            uv10: v1.uv[0],
            uv11: v1.uv[1],
            uv20: v2.uv[0],
            uv21: v2.uv[1],
            uv30: v3.uv[0],
            uv31: v3.uv[1],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Node {
    pub left: u32,
    pub right: u32,
    pub first: u32,
    pub count: u32,
    pub aabb_min: [f32; 3],
    _p1: f32,
    pub aabb_max: [f32; 3],
    _p2: f32,
}

impl Node {
    pub fn cost(&self) -> f32 {
        let e = Vec3::from_array(self.aabb_max) - Vec3::from_array(self.aabb_min);
        let half_area = e.x * e.y + e.y * e.z + e.x * e.z;
        half_area * self.count as f32
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn grow(&mut self, p: &BVHTriangle) {
        self.min = self.min.min(p.min);
        self.max = self.max.max(p.max);
    }
    pub fn half_area(&self) -> f32 {
        let e = self.max - self.min;
        e.x * e.y + e.y * e.z + e.x * e.z
    }
}
impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: Vec3::INFINITY,
            max: Vec3::NEG_INFINITY,
        }
    }
}

#[derive(Debug)]
pub struct BVH {
    pub build_triangles: Vec<BVHTriangle>,
    pub packed_triangles: Vec<PackedTriangle>,
    pub nodes: Vec<Node>,
    pub n_nodes: u32,
    pub quality: Quality,
}

#[derive(Debug)]
pub struct MeshDataList {
    pub triangles: Vec<PackedTriangle>,
    pub nodes: Vec<Node>,
    pub mesh_uniforms: Vec<MeshUniform>,
}
impl Default for MeshDataList {
    fn default() -> Self {
        Self {
            triangles: vec![],
            nodes: vec![],
            mesh_uniforms: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Low,
    High,
    Disabled,
}

impl Default for Quality {
    fn default() -> Self {
        Quality::High
    }
}

impl BVH {
    pub const MAX_NODES: u32 = 520000;
    pub const MAX_DEPTH: u64 = 32;
    pub const TEST_SPLITS: u32 = 50;
    pub fn empty() -> Self {
        Self {
            build_triangles: vec![],
            packed_triangles: vec![],
            nodes: vec![],
            n_nodes: 0,
            quality: Quality::Disabled,
        }
    }
    pub fn build_per_mesh(meshes: &Vec<MeshInstance>, quality: Quality) -> MeshDataList {
        log::info!("Building BVH {:#?}", quality);
        let mut stats = BVHStats::start();
        let mut data = MeshDataList::default();
        let mut mesh_lookup: HashMap<String, (usize, usize)> = HashMap::new();

        for (i, mesh_instance) in meshes.iter().enumerate() {
            let key = if let Some(key) = mesh_instance.label.clone() {
                key
            } else {
                i.to_string()
            };
            if !mesh_lookup.contains_key(&key) {
                mesh_lookup.insert(key.clone(), (data.nodes.len(), data.triangles.len()));
                let mut bvh = BVH::build(
                    mesh_instance.mesh.vertices.clone(),
                    mesh_instance.mesh.indices.clone(),
                    quality,
                    &mut stats,
                );
                data.triangles.append(&mut bvh.packed_triangles);
                data.nodes.append(&mut bvh.nodes);
            }
            let (node_offset, triangle_offset) = mesh_lookup.get(&key).unwrap().clone();
            let model_to_world = mesh_instance.transform.to_matrix();
            data.mesh_uniforms.push(MeshUniform {
                world_to_model: model_to_world.inverse().to_cols_array_2d(),
                model_to_world: model_to_world.to_cols_array_2d(),
                node_offset: node_offset as u32,
                triangles: (mesh_instance.mesh.indices.len() / 3) as u32,
                triangle_offset: triangle_offset as u32,
                material: mesh_instance.material,
                ..Default::default()
            });
        }
        data
    }
    pub fn build(
        vertices: Arc<Vec<Vertex>>,
        indices: Arc<Vec<u32>>,
        quality: Quality,
        stats: &mut BVHStats,
    ) -> Self {
        let n_tris = indices.len() / 3;
        let mut build_triangles = Vec::with_capacity(n_tris);
        let packed_triangles = Vec::with_capacity(n_tris);
        if n_tris == 0 {
            return Self::empty();
        }

        let mut min: [f32; 3] = [f32::MAX; 3];
        let mut max: [f32; 3] = [f32::MIN; 3];

        let mut nodes = Vec::new();
        for i in (0..indices.len()).step_by(3) {
            let index1 = indices[i + 0] as usize;
            let index2 = indices[i + 1] as usize;
            let index3 = indices[i + 2] as usize;

            let v1 = vertices[index1].pos;
            let v2 = vertices[index2].pos;
            let v3 = vertices[index3].pos;

            let centroid = (v1 + v2 + v3) * (1.0 / 3.0);

            let tri = BVHTriangle {
                centroid: centroid,
                max: v1.max(v2.max(v3)),
                min: v1.min(v2.min(v3)),
                i: i as i32,
            };
            BVH::fit_bounds(&mut min, &mut max, &tri);
            build_triangles.push(tri);
        }
        nodes.push(Node {
            aabb_min: min,
            aabb_max: max,
            left: 0,
            right: 0,
            first: 0,
            count: n_tris as u32,
            ..Default::default()
        });

        let mut bvh = Self {
            build_triangles,
            nodes,
            packed_triangles,
            n_nodes: 1,
            quality,
        };
        match quality {
            Quality::Disabled => {
                return bvh;
            }
            _ => {
                bvh.subdivide(0, 0, bvh.build_triangles.len(), 0, stats);
            }
        }
        for i in 0..bvh.build_triangles.len() {
            let built_tri = bvh.build_triangles[i];
            let vert1 = vertices[indices[built_tri.i as usize + 0] as usize];
            let vert2 = vertices[indices[built_tri.i as usize + 1] as usize];
            let vert3 = vertices[indices[built_tri.i as usize + 2] as usize];
            bvh.packed_triangles
                .push(PackedTriangle::new(vert1, vert2, vert3));
        }
        bvh
    }

    pub fn fit_bounds(min: &mut [f32; 3], max: &mut [f32; 3], tri: &BVHTriangle) {
        for axis in 0..3 {
            min[axis] = min[axis].min(tri.min[axis]);
            max[axis] = max[axis].max(tri.max[axis]);
        }
    }

    pub fn find_best_split(
        &self,
        node: &Node,
        axis: &mut usize,
        split_pos: &mut f32,
        start: usize,
        count: usize,
    ) -> f32 {
        if node.count <= 1 {
            *axis = 0;
            *split_pos = 0.0;
            return f32::INFINITY;
        }
        let bounds = (Vec3::from_array(node.aabb_max) - Vec3::from_array(node.aabb_min)).to_array();
        return match self.quality {
            Quality::Low => {
                *axis = if bounds[0] > bounds[1] && bounds[0] > bounds[2] {
                    0
                } else {
                    if bounds[1] > bounds[2] { 1 } else { 2 }
                };
                *split_pos = node.aabb_min[*axis] + bounds[*axis] * 0.5;
                self.evaluate_sah(axis.clone() as i32, split_pos.clone(), start, count)
            }
            Quality::High => {
                let mut best_cost = f32::INFINITY;
                let max_axis = bounds[0].max(bounds[1].max(bounds[2]));
                for a in 0..3 {
                    let axis_size = bounds[a];
                    let axis_min = node.aabb_min[a];
                    if axis_size == 0.0 {
                        continue;
                    }
                    let n_split_tests = ((axis_size / max_axis * BVH::TEST_SPLITS as f32).ceil()
                        as u32)
                        .clamp(1, BVH::TEST_SPLITS);

                    for i in 0..n_split_tests {
                        let split_t = (i + 1) as f32 / (n_split_tests as f32 + 1.0);
                        let test_split_pos = axis_min + axis_size * split_t;
                        let cost = self.evaluate_sah(a as i32, test_split_pos, start, count);
                        if cost < best_cost {
                            *split_pos = test_split_pos;
                            *axis = a;
                            best_cost = cost;
                        }
                    }
                }
                best_cost
            }
            Quality::Disabled => f32::INFINITY,
        };
    }
    pub fn evaluate_sah(&self, axis: i32, pos: f32, start: usize, count: usize) -> f32 {
        let mut left_bounds = Aabb::default();
        let mut right_bounds = Aabb::default();
        let mut left_count = 0.0;
        let mut right_count = 0.0;
        let end = start + count;
        for i in start..end {
            let tri = &self.build_triangles[i];
            if tri.centroid[axis as usize] < pos {
                left_count += 1.0;
                left_bounds.grow(&tri);
            } else {
                right_count += 1.0;
                right_bounds.grow(&tri);
            }
        }
        let cost = left_count * left_bounds.half_area() + right_count * right_bounds.half_area();
        cost
    }

    pub fn subdivide(
        &mut self,
        node_idx: usize,
        tri_global_start: usize,
        n_tris: usize,
        depth: u64,
        stats: &mut BVHStats,
    ) {
        let parent_cost = self.nodes[node_idx].cost();

        let mut axis = 0;
        let mut split_pos = 0.0;
        let cost = self.find_best_split(
            &self.nodes[node_idx],
            &mut axis,
            &mut split_pos,
            tri_global_start,
            n_tris,
        );
        if cost < parent_cost && depth < BVH::MAX_DEPTH {
            let mut left_min: [f32; 3] = [f32::MAX; 3];
            let mut left_max: [f32; 3] = [f32::MIN; 3];

            let mut right_min: [f32; 3] = [f32::MAX; 3];
            let mut right_max: [f32; 3] = [f32::MIN; 3];

            let mut left_count = 0;

            for i in tri_global_start..tri_global_start + n_tris {
                let tri = &self.build_triangles[i];

                if tri.centroid[axis] < split_pos {
                    BVH::fit_bounds(&mut left_min, &mut left_max, &tri);
                    self.build_triangles
                        .swap(tri_global_start + left_count, i as usize);
                    left_count += 1;
                } else {
                    BVH::fit_bounds(&mut right_min, &mut right_max, &tri);
                }
            }
            let right_count = (n_tris - left_count) as u32;
            let left_first = tri_global_start as u32;
            let right_first = left_first + left_count as u32;

            let left_index = self.n_nodes;
            let right_index = self.n_nodes + 1;
            self.n_nodes += 2;

            // TODO: resize nodes vector if needed

            {
                // Left child
                self.nodes.push(Node {
                    aabb_min: left_min,
                    aabb_max: left_max,
                    left: 0,
                    right: 0,
                    first: left_first,
                    count: left_count as u32,
                    ..Default::default()
                })
            }
            {
                // Right child
                self.nodes.push(Node {
                    aabb_min: right_min,
                    aabb_max: right_max,
                    left: 0,
                    right: 0,
                    first: right_first,
                    count: right_count,
                    ..Default::default()
                })
            }
            {
                let parent = &mut self.nodes[node_idx];
                parent.left = left_index;
                parent.right = right_index;
                parent.count = 0;
                stats.record_node();
            }
            self.subdivide(
                left_index as usize,
                tri_global_start,
                left_count,
                depth + 1,
                stats,
            );
            self.subdivide(
                right_index as usize,
                tri_global_start + left_count,
                right_count as usize,
                depth + 1,
                stats,
            );
        } else {
            stats.record_leaf_node(n_tris as u32, depth as u32);
        }
    }
}

#[allow(unused)]
pub struct BVHStats {
    start_time: Instant,
    leaf_count: u32,
    leaf_min_depth: u32,
    leaf_max_depth: u32,
    sum_depth: f32,
    min_tris: u32,
    max_tris: u32,
    sum_tris: f32,
    node_count: u32,
}

impl BVHStats {
    pub fn start() -> Self {
        Self {
            start_time: Instant::now(),
            leaf_count: 0,
            leaf_min_depth: u32::MAX,
            leaf_max_depth: 0,
            sum_depth: 0.0,
            min_tris: u32::MAX,
            max_tris: 0,
            sum_tris: 0.0,
            node_count: 0,
        }
    }

    pub fn record_leaf_node(&mut self, triangle_count: u32, depth: u32) {
        self.record_node();
        self.leaf_count += 1;
        self.sum_depth += depth as f32;
        self.leaf_min_depth = self.leaf_min_depth.min(depth);
        self.leaf_max_depth = self.leaf_max_depth.max(depth);
        self.sum_tris += triangle_count as f32;
        self.max_tris = self.max_tris.max(triangle_count);
        self.min_tris = self.min_tris.min(triangle_count);
    }
    pub fn record_node(&mut self) {
        self.node_count += 1;
    }
    pub fn print(&self) {
        let now = Instant::now();
        println!("BVH: ({:#?})", now - self.start_time);
        println!("Node Count: {}", self.node_count);
        println!("Leaf Count: {}", self.leaf_count);
        println!("Leaf Depth:");
        println!(" - Max: {}", self.leaf_max_depth);
        println!(" - Min: {}", self.leaf_min_depth);
        println!(" - Mean: {}", self.sum_depth / self.leaf_count as f32);
        println!("Leaf Triangles: ");
        println!(" - Max: {}", self.max_tris);
        println!(" - Min: {}", self.min_tris);
        println!(" - mean: {}", self.sum_tris / self.leaf_count as f32);
        println!(" - Total: {}", self.sum_tris);
    }
}
