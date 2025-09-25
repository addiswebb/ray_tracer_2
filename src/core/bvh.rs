use std::time::Instant;

use glam::Vec3;

use crate::core::mesh::{MeshUniform, Vertex};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Triangle {
    pub vertex_0: [f32; 3],
    pub vertex_1: [f32; 3],
    pub vertex_2: [f32; 3],
    pub centroid: [f32; 3],
}

// Add padding correctly
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

#[derive(Debug)]
pub struct BVH {
    pub triangles: Vec<Triangle>,
    pub nodes: Vec<Node>,
    pub triangle_indices: Vec<u32>,
    pub n_nodes: u32,
    pub max_nodes: u64,
    pub max_triangles: u64,
    pub min_triangles_per_node: usize,
}

impl BVH {
    pub fn empty() -> Self {
        Self {
            triangles: vec![],
            nodes: vec![],
            triangle_indices: vec![],
            n_nodes: 0,
            max_nodes: 15000,
            max_triangles: 15000,
            min_triangles_per_node: 2,
        }
    }
    pub fn build(
        meshes: &Vec<MeshUniform>,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        max_nodes: u64,
        max_triangles: u64,
        min_triangles_per_node: usize,
    ) -> Self {
        let start_time = Instant::now();
        let number_of_triangles = indices.len() / 3;
        let mut triangles = Vec::with_capacity(number_of_triangles);
        if number_of_triangles == 0 {
            return Self {
                triangles: vec![],
                nodes: vec![],
                triangle_indices: vec![],
                n_nodes: 0,
                max_nodes,
                max_triangles,
                min_triangles_per_node,
            };
        }
        let mut nodes = Vec::with_capacity(max_nodes as usize);
        println!("Max nodes: {max_nodes}");
        for _ in 0..max_nodes {
            nodes.push(Node::default())
        }
        for mesh in meshes {
            let first = mesh.first as usize;
            let offset = mesh.offset as usize;
            for i in 0..mesh.triangles as usize {
                let index1 = indices[first + i * 3] as usize;
                let index2 = indices[first + i * 3 + 1] as usize;
                let index3 = indices[first + i * 3 + 2] as usize;

                // let mesh_pos = Vec3::from_array(mesh.pos);
                let v0 = Vec3::from_array(vertices[offset + index1].pos); // + mesh_pos;
                let v1 = Vec3::from_array(vertices[offset + index2].pos); // + mesh_pos;
                let v2 = Vec3::from_array(vertices[offset + index3].pos); // + mesh_pos;
                let centroid = (v0 + v1 + v2) * (1.0 / 3.0);

                triangles.push(Triangle {
                    vertex_0: v0.to_array(),
                    vertex_1: v1.to_array(),
                    vertex_2: v2.to_array(),
                    centroid: centroid.to_array(),
                })
            }
        }
        let root = nodes.get_mut(0).unwrap();
        root.left = 0;
        root.right = 0;
        root.first = 0;
        root.count = number_of_triangles as u32;

        let mut bvh = Self {
            triangles,
            nodes,
            triangle_indices: (0..number_of_triangles)
                .into_iter()
                .map(|i| i as u32)
                .collect::<Vec<u32>>(),
            n_nodes: 1,
            max_nodes,
            max_triangles,
            min_triangles_per_node,
        };
        bvh.update_node_bounds(0);
        bvh.subdivide(0);
        println!("Generated BVH in {:?}", Instant::now() - start_time);
        bvh
    }
    pub fn update_node_bounds(&mut self, i: usize) {
        let node = self.nodes.get_mut(i).unwrap();
        node.aabb_min = [1e30, 1e30, 1e30];
        node.aabb_max = [-1e30, -1e30, -1e30];
        for i in 0..node.count as usize {
            let tri_index = self.triangle_indices[node.first as usize + i] as usize;
            let leaf_triangle = self.triangles[tri_index];
            node.aabb_min = BVH::fminf(node.aabb_min, leaf_triangle.vertex_0);
            node.aabb_min = BVH::fminf(node.aabb_min, leaf_triangle.vertex_1);
            node.aabb_min = BVH::fminf(node.aabb_min, leaf_triangle.vertex_2);
            node.aabb_max = BVH::fmaxf(node.aabb_max, leaf_triangle.vertex_0);
            node.aabb_max = BVH::fmaxf(node.aabb_max, leaf_triangle.vertex_1);
            node.aabb_max = BVH::fmaxf(node.aabb_max, leaf_triangle.vertex_2);
        }
        node.aabb_max = (Vec3::from_array(node.aabb_max)).to_array();
        node.aabb_min = (Vec3::from_array(node.aabb_min)).to_array();
    }

    pub fn subdivide(&mut self, node_idx: usize) {
        let parent_first = self.nodes[node_idx].first as usize;
        let parent_count = self.nodes[node_idx].count as usize;
        if self.n_nodes as u64 >= self.max_nodes || parent_count < self.min_triangles_per_node {
            return;
        }
        let aabb_max = self.nodes[node_idx].aabb_max;
        let aabb_min = self.nodes[node_idx].aabb_min;
        let extent = Vec3::from_array(aabb_max) - Vec3::from_array(aabb_min);

        let mut axis = 0;
        if extent.y > extent.x {
            axis = 1
        }
        if extent.z > extent[axis] {
            axis = 2
        }
        let split_pos = aabb_min[axis] + extent[axis] * 0.5;

        let mut i = parent_first;
        let mut j = parent_first + parent_count - 1;
        while i <= j {
            let tri_idx = self.triangle_indices[i] as usize;
            if self.triangles[tri_idx].centroid[axis] < split_pos {
                i += 1
            } else {
                self.triangle_indices.swap(i, j);
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }
        let left_count = (i - parent_first) as u32;
        if left_count == 0 || left_count == parent_count as u32 {
            return;
        }

        let left_index = self.n_nodes;
        let right_index = self.n_nodes + 1;
        self.n_nodes += 2;

        // TODO: resize nodes vector if needed

        {
            // Left child
            let left = &mut self.nodes[left_index as usize];
            left.first = parent_first as u32;
            left.count = left_count as u32;
        }
        {
            // Right child
            let right = &mut self.nodes[right_index as usize];
            right.first = i as u32;
            right.count = (parent_count as u32) - left_count;
        }
        {
            let parent = &mut self.nodes[node_idx];
            parent.left = left_index as u32;
            parent.right = right_index as u32;
            parent.count = 0;
        }
        self.update_node_bounds(left_index as usize);
        self.update_node_bounds(right_index as usize);
        self.subdivide(left_index as usize);
        self.subdivide(right_index as usize);
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
