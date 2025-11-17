use glam::Vec3;
use std::sync::Arc;

use crate::scene::components::{
    geometry::vertex::Vertex, material::MaterialUniform, transform::Transform,
};

#[derive(Debug)]
pub struct MeshData {
    pub vertices: Arc<Vec<Vertex>>,
    pub indices: Arc<Vec<u32>>,
}

#[derive(Clone)]
pub struct MeshInstance {
    pub label: Option<String>,
    pub data: Arc<MeshData>,
    pub transform: Transform,
    pub material: MaterialUniform,
}

impl MeshData {
    pub fn quad() -> Vec<Vertex> {
        vec![
            Vertex::with_uv(Vec3::new(-1.0, -1.0, 0.0), Vec3::Z, [0.0, 0.0]),
            Vertex::with_uv(Vec3::new(1.0, -1.0, 0.0), Vec3::Z, [1.0, 0.0]),
            Vertex::with_uv(Vec3::new(1.0, 1.0, 0.0), Vec3::Z, [1.0, 1.0]),
            Vertex::with_uv(Vec3::new(-1.0, 1.0, 0.0), Vec3::Z, [0.0, 1.0]),
        ]
    }
}
pub enum MeshDefinition {
    FromFile {
        path: String,
        use_mtl: bool,
    },
    FromData {
        vertices: Arc<Vec<Vertex>>,
        indices: Arc<Vec<u32>>,
    },
}

impl MeshDefinition {
    pub fn from_data(vertices: Vec<Vertex>, indices: Vec<u32>) -> MeshDefinition {
        MeshDefinition::FromData {
            vertices: Arc::new(vertices),
            indices: Arc::new(indices),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct MeshUniform {
    pub world_to_model: [[f32; 4]; 4],
    pub model_to_world: [[f32; 4]; 4],
    pub node_offset: u32,
    pub triangles: u32,
    pub triangle_offset: u32,
    pub _p1: f32,
    pub material: MaterialUniform,
}
