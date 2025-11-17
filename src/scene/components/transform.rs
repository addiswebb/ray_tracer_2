use glam::{Mat4, Quat, Vec3};

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
