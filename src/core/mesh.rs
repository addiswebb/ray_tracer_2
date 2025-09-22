use glam::{Vec3, Vec4};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub _padding1: f32,
    pub normal: [f32; 3],
    pub _padding2: f32,
}

impl Vertex {
    pub fn new(pos: Vec3, normal: Vec3) -> Self {
        Self {
            pos: pos.to_array(),
            _padding1: 0.0,
            normal: normal.to_array(),
            _padding2: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Sphere {
    position: [f32; 3],
    radius: f32,
    color: [f32; 4],
    emission_color: [f32; 4],
    emission_strength: f32,
    smoothness: f32,
    _padding: [f32; 2],
}

impl Sphere {
    pub fn new(
        position: Vec3,
        radius: f32,
        color: Vec4,
        emission_color: Vec4,
        emission_strength: f32,
        specular: f32,
    ) -> Self {
        Self {
            position: position.to_array(),
            radius,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            _padding: [0.0; 2],
            smoothness: if specular < 1.0 { specular } else { 1.0 },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Material {
    color: [f32; 4],
    emission_color: [f32; 4],
    emission_strength: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Mesh {
    pub first: u32,
    pub triangles: u32,
    pub offset: u32,
    pub _padding: f32,
    pub pos: [f32; 3],
    pub _padding2: f32,
    pub color: [f32; 4],
    pub emission_color: [f32; 4],
    pub emission_strength: f32,
    pub specular: f32,
    pub _padding3: [f32; 2],
}

impl Mesh {
    pub fn new(
        pos: Vec3,
        first: u32,
        triangles: u32,
        offset: u32,
        color: Vec4,
        emission_color: Vec4,
        emission_strength: f32,
        specular: f32,
    ) -> Self {
        Self {
            first,
            triangles,
            offset,
            _padding2: 0.0,
            pos: pos.to_array(),
            _padding: 0.0,
            color: color.to_array(),
            emission_color: emission_color.to_array(),
            emission_strength,
            specular: if specular < 1.0 { specular } else { 1.0 },
            _padding3: [0.0; 2],
        }
    }
}
