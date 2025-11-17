use glam::Vec3;

use crate::scene::components::{
    geometry::mesh::MeshDefinition, material::MaterialDefinition, transform::Transform,
};

pub enum Primitive {
    Sphere { centre: Vec3, radius: f32 },
    Mesh(MeshDefinition),
}

pub struct EntityDefinition {
    pub transform: Transform,
    pub primitive: Primitive,
    pub material: MaterialDefinition,
}
