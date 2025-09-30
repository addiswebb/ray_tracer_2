use glam::{Mat4, Quat, Vec3};

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
    pub position: [f32; 3],
    pub radius: f32,
    pub material: Material,
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
            smoothness: 0.0,
            specular: 0.1,
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
#[derive(Debug)]
pub struct Mesh {
    pub transform: Transform,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material: Material,
}

#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}
impl Transform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
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

#[allow(unused)]
impl Mesh {
    pub fn material(&mut self, material: Material) -> &Self {
        self.material = material;
        self
    }
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
