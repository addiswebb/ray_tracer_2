use glam::Vec3;

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
