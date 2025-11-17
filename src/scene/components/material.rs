use crate::scene::components::texture::TextureDefinition;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniform {
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
    pub width: u32,
    pub height: u32,
    pub _p1: [f32; 3],
}
impl Default for MaterialUniform {
    fn default() -> Self {
        Self {
            color: [0.7, 0.7, 0.7, 1.0],
            emission_color: [0.0; 4],
            specular_color: [0.0; 4],
            absorption: [0.0; 4],
            absorption_stength: 0.0,
            emission_strength: 0.0,
            smoothness: 0.9,
            specular: 0.00,
            ior: 1.0,
            flag: 0,
            texture_index: 0,
            width: 0,
            height: 0,
            _p1: [0.0; 3],
        }
    }
}

#[derive(Clone, Copy)]
pub enum MaterialFlag {
    NORMAL = 0,
    GLASS = 1,
    TEXTURE = 2,
}

pub struct MaterialDefinition {
    pub color: [f32; 4],
    pub emission_color: [f32; 4],
    pub specular_color: [f32; 4],
    pub absorption: [f32; 4],
    pub absorption_stength: f32,
    pub emission_strength: f32,
    pub smoothness: f32,
    pub specular: f32,
    pub ior: f32,
    pub flag: MaterialFlag,
    pub texture: Option<TextureDefinition>,
}

impl MaterialDefinition {
    pub fn texture_from_obj() -> MaterialDefinition {
        MaterialDefinition {
            flag: MaterialFlag::GLASS,
            ..Default::default()
        }
    }
}

impl Default for MaterialDefinition {
    fn default() -> Self {
        Self {
            color: [0.7, 0.7, 0.7, 1.0],
            emission_color: [0.0; 4],
            specular_color: [1.0; 4],
            absorption: [0.0; 4],
            absorption_stength: 0.0,
            emission_strength: 0.0,
            smoothness: 1.0,
            specular: 0.0,
            ior: 1.0,
            flag: MaterialFlag::NORMAL,
            texture: None,
        }
    }
}

#[allow(unused)]
impl MaterialDefinition {
    pub fn new() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            emission_color: [1.0, 1.0, 1.0, 1.0],
            specular_color: [1.0, 1.0, 1.0, 1.0],
            absorption: [0.0, 0.0, 0.0, 0.0],
            absorption_stength: 0.0,
            emission_strength: 0.0,
            smoothness: 0.0,
            specular: 0.1,
            ior: 0.0,
            flag: MaterialFlag::NORMAL,
            texture: None,
        }
    }
    pub fn color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn emissive(mut self, color: [f32; 4], strength: f32) -> Self {
        self.emission_color = color;
        self.emission_strength = strength;
        self
    }
    pub fn glass(mut self, index_of_refraction: f32) -> Self {
        self.ior = index_of_refraction;
        self.flag = MaterialFlag::GLASS;
        self
    }
    pub fn specular(mut self, color: [f32; 4], specular: f32) -> Self {
        self.specular_color = color;
        self.specular = specular;
        self
    }
    pub fn smooth(mut self, smoothness: f32) -> Self {
        self.smoothness = smoothness;
        self
    }
}
