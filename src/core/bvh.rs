use std::time::Instant;

use glam::Vec3;

use crate::core::mesh::{MeshUniform, Vertex};

#[derive(Debug, Copy, Clone, Default)]
pub struct BVHTriangle {
    pub centroid: Vec3,
    pub min: Vec3,
    pub max: Vec3,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PackedTriangle {
    pub v1: [f32; 3],
    pub _p1: f32,
    pub v2: [f32; 3],
    pub _p2: f32,
    pub v3: [f32; 3],
    pub _p3: f32,
    pub n1: [f32; 3],
    pub _p4: f32,
    pub n2: [f32; 3],
    pub _p5: f32,
    pub n3: [f32; 3],
    pub mesh_index: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Node {
    pub aabb_min: [f32; 3],
    pub _p1: f32,
    pub aabb_max: [f32; 3],
    pub _p2: f32,
    pub left: u32,
    pub right: u32,
    pub first: u32,
    pub count: u32,
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
    pub triangles: Vec<BVHTriangle>,
    pub packed_triangles: Vec<PackedTriangle>,
    pub nodes: Vec<Node>,
    pub triangle_indices: Vec<u32>,
    pub n_nodes: u32,
    pub quality: Quality,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    Low,
    High,
    Disabled,
}

impl BVH {
    pub const MAX_NODES: u32 = 200000;
    pub const MAX_DEPTH: u64 = 32;
    pub const TEST_SPLITS: u32 = 50;
    pub fn empty() -> Self {
        Self {
            triangles: vec![],
            packed_triangles: vec![],
            nodes: vec![],
            triangle_indices: vec![],
            n_nodes: 0,
            quality: Quality::Disabled,
        }
    }
    pub fn build(
        meshes: &Vec<MeshUniform>,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        quality: Quality,
    ) -> Self {
        println!("Building BVH {:#?}", quality);
        let mut stats = BVHStats::start();

        let number_of_triangles = indices.len() / 3;
        let mut triangles = Vec::with_capacity(number_of_triangles);
        let mut packed_triangles = Vec::with_capacity(number_of_triangles);
        if number_of_triangles == 0 {
            return Self::empty();
        }

        let mut min: [f32; 3] = [f32::MAX; 3];
        let mut max: [f32; 3] = [f32::MIN; 3];

        let mut nodes = vec![Node::default(); BVH::MAX_NODES as usize];
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            let first = mesh.first as usize;
            let offset = mesh.offset as usize;
            for i in 0..mesh.triangles as usize {
                let index1 = indices[first + i * 3] as usize;
                let index2 = indices[first + i * 3 + 1] as usize;
                let index3 = indices[first + i * 3 + 2] as usize;

                let v1 = Vec3::from_array(vertices[offset + index1].pos);
                let v2 = Vec3::from_array(vertices[offset + index2].pos);
                let v3 = Vec3::from_array(vertices[offset + index3].pos);

                let n1 = Vec3::from_array(vertices[offset + index1].normal);
                let n2 = Vec3::from_array(vertices[offset + index2].normal);
                let n3 = Vec3::from_array(vertices[offset + index3].normal);
                let centroid = (v1 + v2 + v3) * (1.0 / 3.0);

                let tri = BVHTriangle {
                    centroid: centroid,
                    max: v1.max(v2.max(v3)),
                    min: v1.min(v2.min(v3)),
                };
                BVH::fit_bounds(&mut min, &mut max, &tri);
                triangles.push(tri);

                packed_triangles.push(PackedTriangle {
                    v1: v1.to_array(),
                    v2: v2.to_array(),
                    v3: v3.to_array(),
                    n1: n1.to_array(),
                    n2: n2.to_array(),
                    n3: n3.to_array(),
                    mesh_index: mesh_index as u32,
                    _p1: 1.0,
                    _p2: 1.0,
                    _p3: 1.0,
                    _p4: 1.0,
                    _p5: 1.0,
                })
            }
        }
        nodes[0] = Node {
            aabb_min: min,
            _p1: 0.0,
            aabb_max: max,
            _p2: 0.0,
            left: 0,
            right: 0,
            first: 0,
            count: number_of_triangles as u32,
        };

        let mut bvh = Self {
            triangles,
            nodes,
            packed_triangles,
            triangle_indices: (0..number_of_triangles)
                .into_iter()
                .map(|i| i as u32)
                .collect::<Vec<u32>>(),
            n_nodes: 1,
            quality,
        };
        match quality {
            Quality::Disabled => {
                return bvh;
            }
            _ => {
                bvh.subdivide(0, 0, &mut stats);
            }
        }
        stats.print();
        bvh
    }

    pub fn fit_bounds(min: &mut [f32; 3], max: &mut [f32; 3], tri: &BVHTriangle) {
        for axis in 0..3 {
            min[axis] = min[axis].min(tri.min[axis]);
            max[axis] = max[axis].max(tri.max[axis]);
        }
    }

    pub fn find_best_split(&self, node: &Node, axis: &mut usize, split_pos: &mut f32) -> f32 {
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
                self.evaluate_sah(node, axis.clone() as i32, split_pos.clone())
            }
            Quality::High => {
                let mut best_cost = f32::INFINITY;
                for a in 0..3 {
                    let bounds_min = node.aabb_min[a];
                    let bounds_max = node.aabb_max[a];
                    if bounds_max == bounds_min {
                        continue;
                    }
                    for i in 0..BVH::TEST_SPLITS {
                        let split_t = (i + 1) as f32 / (BVH::TEST_SPLITS as f32 + 1.0);
                        let test_split_pos = bounds_min + (bounds_max - bounds_min) * split_t;
                        let cost = self.evaluate_sah(node, a as i32, test_split_pos);
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
    pub fn evaluate_sah(&self, node: &Node, axis: i32, pos: f32) -> f32 {
        let mut left_bounds = Aabb::default();
        let mut right_bounds = Aabb::default();
        let mut left_count = 0.0;
        let mut right_count = 0.0;
        for i in 0..node.count as usize {
            let tri = &self.triangles[self.triangle_indices[node.first as usize + i] as usize];
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

    pub fn subdivide(&mut self, node_idx: usize, depth: u64, stats: &mut BVHStats) {
        let parent_first = self.nodes[node_idx].first;
        let parent_count = self.nodes[node_idx].count;
        let parent_cost = self.nodes[node_idx].cost();

        let mut axis = 0;
        let mut split_pos = 0.0;
        let cost = self.find_best_split(&self.nodes[node_idx], &mut axis, &mut split_pos);
        if cost < parent_cost && depth < BVH::MAX_DEPTH {
            let mut left_min: [f32; 3] = [f32::MAX; 3];
            let mut left_max: [f32; 3] = [f32::MIN; 3];

            let mut right_min: [f32; 3] = [f32::MAX; 3];
            let mut right_max: [f32; 3] = [f32::MIN; 3];

            let mut left_count = 0;
            for i in 0..parent_count as usize {
                let tri =
                    &self.triangles[self.triangle_indices[parent_first as usize + i] as usize];

                if tri.centroid[axis] < split_pos {
                    BVH::fit_bounds(&mut left_min, &mut left_max, &tri);
                    self.triangle_indices.swap(
                        left_count + parent_first as usize,
                        i + parent_first as usize,
                    );
                    self.packed_triangles.swap(
                        left_count + parent_first as usize,
                        i + parent_first as usize,
                    );
                    left_count += 1;
                } else {
                    BVH::fit_bounds(&mut right_min, &mut right_max, &tri);
                }
            }
            let right_count = parent_count - left_count as u32;
            let left_first = parent_first;
            let right_first = parent_first + left_count as u32;

            let left_index = self.n_nodes;
            let right_index = self.n_nodes + 1;
            self.n_nodes += 2;

            // TODO: resize nodes vector if needed

            {
                // Left child
                let left = &mut self.nodes[left_index as usize];
                left.first = left_first as u32;
                left.count = left_count as u32;
                left.aabb_max = left_max;
                left.aabb_min = left_min;
            }
            {
                // Right child
                let right = &mut self.nodes[right_index as usize];
                right.first = right_first as u32;
                right.count = right_count;
                right.aabb_max = right_max;
                right.aabb_min = right_min;
            }
            {
                let parent = &mut self.nodes[node_idx];
                parent.left = left_index;
                parent.right = right_index;
                parent.count = 0;
                stats.record_node();
            }
            self.subdivide(left_index as usize, depth + 1, stats);
            self.subdivide(right_index as usize, depth + 1, stats);
        } else {
            stats.record_leaf_node(parent_count, depth as u32);
        }
    }
    pub fn fminf(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut c = [0.0, 0.0, 0.0];
        for i in 0..3 {
            c[i] = if a[i] < b[i] { a[i] } else { b[i] };
        }
        c
    }

    pub fn fmaxf(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        let mut c = [0.0, 0.0, 0.0];
        for i in 0..3 {
            c[i] = if a[i] > b[i] { a[i] } else { b[i] };
        }
        c
    }
}

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
