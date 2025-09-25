use glam::Vec3;

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
    material: Material,
}

impl Sphere {
    pub fn new(position: Vec3, radius: f32, material: Material) -> Self {
        Self {
            position: position.to_array(),
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
    pub emission_strength: f32,
    pub smoothness: f32,
    pub specular: f32,
    pub _padding: f32,
}

impl Material {
    pub fn new() -> Material {
        Material {
            color: [1.0, 1.0, 1.0, 1.0],
            emission_color: [1.0, 1.0, 1.0, 1.0],
            specular_color: [1.0, 1.0, 1.0, 1.0],
            emission_strength: 0.0,
            smoothness: 0.2,
            specular: 0.5,
            _padding: 0.0,
        }
    }
    pub fn color(&mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self.specular_color = color;
        *self
    }

    pub fn emissive(&mut self, color: [f32; 4], strength: f32) -> Self {
        self.emission_color = color;
        self.emission_strength = strength;
        *self
    }
    #[allow(unused)]
    pub fn glass(&mut self, refractive_index: f32) -> Self {
        self.smoothness = -refractive_index;
        *self
    }
    #[allow(unused)]
    pub fn specular(&mut self, color: [f32; 4], specular: f32) -> Self {
        self.specular_color = color;
        self.specular = specular;
        *self
    }
    pub fn smooth(&mut self, smoothness: f32) -> Self {
        self.smoothness = smoothness;
        *self
    }
}

#[allow(unused)]
pub struct Mesh {
    pub position: Vec3,
    pub size: Vec3,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Material,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct MeshUniform {
    pub first: u32,
    pub triangles: u32,
    pub offset: u32,
    pub _padding: f32,
    pub pos: [f32; 3],
    pub _padding2: f32,
    pub material: Material,
}
