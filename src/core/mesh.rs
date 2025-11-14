use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Copy, Clone, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub normal: Vec3,
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn new(pos: Vec3, normal: Vec3) -> Self {
        Self {
            pos,
            normal,
            uv: [0.0; 2],
        }
    }
    pub fn with_uv(pos: Vec3, normal: Vec3, uv: [f32; 2]) -> Self {
        Self { pos, normal, uv }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Sphere {
    pub pos: [f32; 3],
    pub radius: f32,
    pub material: Material,
}

impl Sphere {
    pub fn new(pos: Vec3, radius: f32, material: Material) -> Self {
        Self {
            pos: pos.to_array(),
            radius,
            material,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Material {
    pub color: [f32; 4],
    pub emission_color: [f32; 4],
    pub specular_color: [f32; 4],
    pub absorption: [f32; 4],
    pub absorption_stength: f32,
    pub emission_strength: f32,
    pub smoothness: f32,
    pub specular: f32,
    pub ior: f32,
    pub flag: i32,
    pub texture_index: u32,
    pub _p1: f32,
}

impl Material {
    pub fn new() -> Material {
        Material {
            color: [1.0, 1.0, 1.0, 1.0],
            emission_color: [1.0, 1.0, 1.0, 1.0],
            specular_color: [1.0, 1.0, 1.0, 1.0],
            absorption: [0.0, 0.0, 0.0, 0.0],
            absorption_stength: 0.0,
            emission_strength: 0.0,
            smoothness: 0.0,
            specular: 0.1,
            ior: 0.0,
            flag: 0,
            texture_index: 0,
            _p1: 0.0,
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct Mesh {
    pub label: Option<String>,
    pub transform: Transform,
    pub vertices: Arc<Vec<Vertex>>,
    pub indices: Arc<Vec<u32>>,
    pub material: Material,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}
impl Transform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }
    pub fn cam(origin: Vec3, look_at: Vec3) -> Self {
        Self {
            pos: origin,
            rot: Quat::look_at_lh(origin, look_at, Vec3::Y),
            scale: Vec3::ONE,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Mesh {
    pub fn material(&mut self, material: Material) -> &Self {
        self.material = material;
        self
    }

    pub fn quad(t: Transform) -> Vec<Vertex> {
        let mut vertices = vec![
            Vertex::with_uv(Vec3::new(-1.0, -1.0, 0.0), Vec3::Z, [0.0, 0.0]),
            Vertex::with_uv(Vec3::new(1.0, -1.0, 0.0), Vec3::Z, [1.0, 0.0]),
            Vertex::with_uv(Vec3::new(1.0, 1.0, 0.0), Vec3::Z, [1.0, 1.0]),
            Vertex::with_uv(Vec3::new(-1.0, 1.0, 0.0), Vec3::Z, [0.0, 1.0]),
        ];
        for v in &mut vertices {
            v.pos += t.pos;
            v.normal = t.rot * v.normal;
            v.pos = t.rot * v.pos;
        }
        vertices
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
    pub material: Material,
}
