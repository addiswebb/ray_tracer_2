use crate::scene::components::material::MaterialUniform;
use glam::Vec3;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Sphere {
    pub pos: [f32; 3],
    pub radius: f32,
    pub material: MaterialUniform,
}

impl Sphere {
    pub fn new(pos: Vec3, radius: f32, material: MaterialUniform) -> Self {
        Self {
            pos: pos.to_array(),
            radius,
            material,
        }
    }
}
