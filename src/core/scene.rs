use std::{f32::consts::PI, sync::Arc};

use glam::{Quat, Vec3};
use rand::Rng;

use crate::core::{
    asset::{
        AssetManager, EntityDefinition, MaterialDefinition, MaterialFlag, MeshDefinition,
        Primitive, TextureDefinition, TextureRef,
    },
    bvh::{self, BVH, MeshDataList, Node, Quality},
    camera::{CameraDescriptor, CameraUniform},
    mesh::{Material, Mesh, MeshInstance, Sphere, Transform, Vertex},
    ray_tracer::RayTracer,
};

use super::camera::Camera;

pub struct SceneDefinition {
    camera: Camera,
    entities: Vec<EntityDefinition>,
}

impl SceneDefinition {
    pub fn set_camera(&mut self, camera_description: &CameraDescriptor) {
        self.camera = Camera::new(camera_description);
    }
    pub fn add_sphere(&mut self, centre: Vec3, radius: f32, material: MaterialDefinition) {
        self.entities.push(EntityDefinition {
            transform: Transform::default(),
            primitive: Primitive::Sphere { centre, radius },
            material,
        });
    }

    pub fn add_mesh(
        &mut self,
        transform: Transform,
        mesh_definition: MeshDefinition,
        material: MaterialDefinition,
    ) {
        self.entities.push(EntityDefinition {
            transform,
            primitive: Primitive::Mesh(mesh_definition),
            material,
        });
    }
}
impl Default for SceneDefinition {
    fn default() -> Self {
        Self {
            camera: Camera::new(&CameraDescriptor::default()),
            entities: vec![],
        }
    }
}

pub struct SceneManager {
    pub scene: Scene,
    pub selected_scene: i32,
    pub selected_entity: i32,
    pub prev_scene: i32,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            prev_scene: 0,
            selected_scene: 0,
            selected_entity: -1,
        }
    }
    pub fn load_scene(
        &mut self,
        scene_definition: &SceneDefinition,
        assets: &mut AssetManager,
        ray_tracer: &mut RayTracer,
    ) {
        self.scene = Scene::instantiate_scene(scene_definition, assets);
        ray_tracer.load_scene_gpu_resources(assets);
    }
}

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub meshes: Vec<MeshInstance>,
    pub bvh_data: MeshDataList,
    pub bvh_quality: Quality,
    pub built_bvh: bool,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct SceneUniform {
    spheres: u32,
    n_vertices: u32,
    n_indices: u32,
    meshes: u32,
    camera: CameraUniform,
    nodes: u32,
    padding: [f32; 6],
}

#[allow(dead_code)]
impl Scene {
    pub fn new() -> Self {
        let camera = Camera::new(&CameraDescriptor {
            transform: Transform::cam(Vec3::ZERO, Vec3::Z),
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 2.0,
            ..Default::default()
        });
        Self {
            camera,
            spheres: vec![],
            meshes: vec![],
            bvh_data: MeshDataList::default(),
            bvh_quality: Quality::default(),
            built_bvh: false,
        }
    }
    pub fn instantiate_scene(
        scene_definition: &SceneDefinition,
        asset_manager: &mut AssetManager,
    ) -> Scene {
        println!("Instantiating Scene");
        let mut spheres: Vec<Sphere> = vec![];
        let mut meshes: Vec<MeshInstance> = vec![];
        for (i, e) in scene_definition.entities.iter().enumerate() {
            let mut flag = e.material.flag as i32;
            let mut texture_ref = TextureRef::default();
            if let Some(texture) = &e.material.texture {
                // Handle loading texture (use asset_manager)
                match texture {
                    TextureDefinition::FromFile { path } => {
                        texture_ref = asset_manager.load_texture(&path);
                        flag = MaterialFlag::TEXTURE as i32;
                    }
                    _ => (),
                };
            }
            let material = Material {
                color: e.material.color,
                emission_color: e.material.emission_color,
                specular_color: e.material.specular_color,
                absorption: e.material.absorption,
                absorption_stength: e.material.absorption_stength,
                emission_strength: e.material.emission_strength,
                smoothness: e.material.smoothness,
                specular: e.material.specular,
                ior: e.material.ior,
                flag,
                texture_index: texture_ref.index,
                width: texture_ref.width,
                height: texture_ref.height,
                _p1: [0.0; 3],
            };
            match &e.primitive {
                Primitive::Sphere { centre, radius } => {
                    spheres.push(Sphere::new(*centre, *radius, material));
                }
                Primitive::Mesh(mesh_def) => {
                    match mesh_def {
                        MeshDefinition::FromFile {
                            path,
                            use_mtl: use_loaded_materials,
                        } => {
                            // Load mesh using asset manager
                            let mut m = asset_manager.load_model_with_material(
                                path,
                                e.transform,
                                *use_loaded_materials,
                                material,
                            );
                            meshes.append(&mut m);
                        }
                        MeshDefinition::FromData { vertices, indices } => {
                            meshes.push(MeshInstance {
                                label: Some(format!("mesh_{}", i)),
                                transform: e.transform,
                                mesh: Arc::new(Mesh {
                                    vertices: vertices.clone(),
                                    indices: indices.clone(),
                                }),
                                material,
                            })
                        }
                    };
                }
            }
        }

        let bvh_data = BVH::build_per_mesh(&meshes, bvh::Quality::High);

        Self {
            camera: scene_definition.camera,
            spheres,
            meshes,
            bvh_data,
            bvh_quality: bvh::Quality::High,
            built_bvh: true,
        }
    }
    pub fn bvh(&mut self) -> &Vec<Node> {
        if !self.built_bvh && self.meshes.len() > 0 {
            self.bvh_data = BVH::build_per_mesh(&self.meshes, bvh::Quality::High);
            self.built_bvh = true;
        }
        &self.bvh_data.nodes
    }

    pub fn texture_test() -> SceneDefinition {
        let mut scene_def = SceneDefinition::default();
        scene_def.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::NEG_Z, Vec3::ZERO),
            ..Default::default()
        });

        scene_def.add_sphere(
            Vec3::ZERO,
            1.0,
            MaterialDefinition {
                color: [1.0, 0.0, 0.0, 1.0],
                emission_color: [0.0; 4],
                specular_color: [1.0; 4],
                absorption: [0.0; 4],
                absorption_stength: 0.0,
                emission_strength: 0.0,
                smoothness: 0.0,
                specular: 0.05,
                ior: 1.0,
                flag: MaterialFlag::TEXTURE,
                texture: Some(TextureDefinition::FromFile {
                    path: "earthmap.png".to_string(),
                }),
            },
        );

        scene_def
    }

    pub async fn obj_test() -> SceneDefinition {
        let mut scene_def = SceneDefinition::default();
        scene_def.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 1.0,
            ..Default::default()
        });
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::FromFile {
                path: "dragon.obj".to_string(),
                use_mtl: false,
            },
            MaterialDefinition::new(),
        );

        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(0.5, 0.0, -1.0), Vec3::X),
                    Vertex::new(Vec3::new(0.5, 1.0, -1.0), Vec3::X),
                    Vertex::new(Vec3::new(0.0, 1.0, 1.0), Vec3::X),
                    Vertex::new(Vec3::new(0.2, 0.0, 1.0), Vec3::X),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new()
                .color([1.0, 1.0, 0.0, 1.0])
                .emissive([1.0, 0.0, 0.0, 1.0], 0.4),
        );

        // Add spheres
        scene_def.add_sphere(
            Vec3::new(1.8, 0.1, 1.0),
            0.6,
            MaterialDefinition::new().color([1.0, 0.0, 0.0, 1.0]),
        );

        scene_def.add_sphere(
            Vec3::new(1.0, 0.5, 1.0),
            0.3,
            MaterialDefinition::new().color([1.0, 0.0, 0.0, 1.0]),
        );

        scene_def.add_sphere(
            Vec3::new(0.0, -10.0, 0.0),
            10.0,
            MaterialDefinition::new().color([1.0, 0.0, 0.0, 1.0]),
        );
        scene_def
    }

    pub fn random_balls() -> SceneDefinition {
        let mut scene_def = SceneDefinition::default();
        scene_def.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(13.0, 2.0, 3.0), Vec3::new(0.0, 0.0, 0.0)),
            fov: 20.0,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 10.0,
            ..Default::default()
        });
        scene_def.add_sphere(
            // Floor
            Vec3::new(0.0, -1000.0, 0.0),
            1000.0,
            MaterialDefinition::new().color([0.5, 0.5, 0.5, 1.0]),
        );
        scene_def.add_sphere(
            Vec3::new(0.0, 1.0, 0.0),
            1.0,
            MaterialDefinition::new().glass(1.5),
        );
        scene_def.add_sphere(
            Vec3::new(-4.0, 1.0, 0.0),
            1.0,
            MaterialDefinition::new().color([0.4, 0.2, 0.1, 1.0]),
        );
        scene_def.add_sphere(
            Vec3::new(4.0, 1.0, 0.0),
            1.0,
            MaterialDefinition::new()
                .color([0.7, 0.6, 0.5, 1.0])
                .specular([0.7, 0.6, 0.5, 1.0], 1.0)
                .smooth(1.0),
        );

        for a in -11..11 {
            for b in -11..11 {
                let mut rng = rand::rng();

                let mat = rng.random::<f32>();

                let center = Vec3::new(
                    a as f32 + 0.9 * rng.random::<f32>(),
                    0.2,
                    b as f32 + 0.9 * rng.random::<f32>(),
                );
                if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                    if mat < 0.8 {
                        let albedo = [
                            rng.random::<f32>(),
                            rng.random::<f32>(),
                            rng.random::<f32>(),
                            1.0,
                        ];
                        scene_def.add_sphere(center, 0.2, MaterialDefinition::new().color(albedo));
                    } else if mat < 0.95 {
                        let albedo = [
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            1.0,
                        ];
                        let fuzz = rng.random_range(0.0..0.5);
                        scene_def.add_sphere(
                            center,
                            0.2,
                            MaterialDefinition::new()
                                .color(albedo)
                                .specular([1.0, 1.0, 1.0, 1.0], fuzz),
                        );
                    } else {
                        scene_def.add_sphere(center, 0.2, MaterialDefinition::new().glass(1.3));
                    }
                }
            }
        }

        scene_def
    }
    pub fn room() -> SceneDefinition {
        let mut scene_def = SceneDefinition::default();

        // Set camera
        scene_def.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.0, 3.0), Vec3::new(0.0, 1.0, 2.0)),
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        // Floor
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::Y),
                    Vertex::new(Vec3::new(2.0, 0.0, -2.0), Vec3::Y),
                    Vertex::new(Vec3::new(2.0, 0.0, 2.0), Vec3::Y),
                    Vertex::new(Vec3::new(-2.0, 0.0, 2.0), Vec3::Y),
                ],
                vec![2, 1, 0, 3, 2, 0],
            ),
            MaterialDefinition::new().color([1.0, 0.0, 0.0, 1.0]),
        );

        // Ceiling
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-2.0, 4.0, -2.0), -Vec3::Y),
                    Vertex::new(Vec3::new(2.0, 4.0, -2.0), -Vec3::Y),
                    Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::Y),
                    Vertex::new(Vec3::new(-2.0, 4.0, 2.0), -Vec3::Y),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().color([0.0, 0.3, 0.3, 1.0]),
        );

        // Left wall
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::X),
                    Vertex::new(Vec3::new(-2.0, 4.0, -2.0), Vec3::X),
                    Vertex::new(Vec3::new(-2.0, 4.0, 2.0), Vec3::X),
                    Vertex::new(Vec3::new(-2.0, 0.0, 2.0), Vec3::X),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new()
                .specular([1.0, 1.0, 1.0, 1.0], 1.0)
                .smooth(1.0),
        );

        // Right wall
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(2.0, 0.0, -2.0), -Vec3::X),
                    Vertex::new(Vec3::new(2.0, 0.0, 2.0), -Vec3::X),
                    Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::X),
                    Vertex::new(Vec3::new(2.0, 4.0, -2.0), -Vec3::X),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new()
                .specular([1.0, 1.0, 1.0, 1.0], 0.99)
                .smooth(0.99),
        );

        // Back wall
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-2.0, 0.0, 2.0), -Vec3::Z),
                    Vertex::new(Vec3::new(2.0, 0.0, 2.0), -Vec3::Z),
                    Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::Z),
                    Vertex::new(Vec3::new(-2.0, 4.0, 2.0), -Vec3::Z),
                ],
                vec![2, 1, 0, 3, 2, 0],
            ),
            MaterialDefinition::new()
                .color([0.2, 0.2, 0.82, 1.0])
                .specular([1.0, 1.0, 1.0, 1.0], 0.99)
                .smooth(0.99),
        );

        // Light
        scene_def.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-0.4, 3.98, -0.4), -Vec3::Y),
                    Vertex::new(Vec3::new(0.4, 3.98, -0.4), -Vec3::Y),
                    Vertex::new(Vec3::new(0.4, 3.98, 0.4), -Vec3::Y),
                    Vertex::new(Vec3::new(-0.4, 3.98, 0.4), -Vec3::Y),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().emissive([1.0, 1.0, 1.0, 1.0], 3.0),
        );

        // Spheres
        scene_def.add_sphere(
            Vec3::new(0.4, 1.0, 0.0),
            0.3,
            MaterialDefinition::new()
                .color([0.4, 0.9, 0.4, 1.0])
                .glass(1.34),
        );

        scene_def.add_sphere(
            Vec3::new(-0.4, 1.0, 0.0),
            0.4,
            MaterialDefinition::new()
                .color([0.7, 0.7, 0.7, 1.0])
                .specular([1.0, 1.0, 1.0, 1.0], 0.2),
        );

        scene_def
    }
    pub fn room_2() -> SceneDefinition {
        let mut scene = SceneDefinition::default();

        // Set camera
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.28, 13.5), Vec3::new(0.0, 1.28, 12.5)),
            fov: 26.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 8.6,
            defocus_strength: 100.0,
            diverge_strength: 1.5,
            ..Default::default()
        });

        let (width, depth, height) = (3.0, 2.0, 4.0);

        // Async loaded mesh (Dragon)
        scene.add_mesh(
            Transform {
                pos: Vec3::new(0.0, 1.2, -0.6),
                rot: Quat::from_euler(glam::EulerRot::XYX, 0.0, -1.5708, 0.0),
                scale: Vec3::splat(4.7),
            },
            MeshDefinition::FromFile {
                path: "Dragon_80K.obj".to_string(),
                use_mtl: false,
            },
            MaterialDefinition::new()
                .color([0.96078, 0.11372, 0.4039, 1.0])
                .smooth(0.8)
                .specular([1.0; 4], 0.015),
        );
        scene.add_mesh(
            Transform {
                pos: Vec3::new(0.0, 7.2, 2.0),
                rot: Quat::from_euler(glam::EulerRot::XYX, 0.0, -1.5708, 0.0),
                scale: Vec3::splat(1.0),
            },
            MeshDefinition::FromFile {
                path: "Dragon_80K.obj".to_string(),
                use_mtl: false,
            },
            MaterialDefinition::new()
                .color([0.96078, 0.11372, 0.4039, 1.0])
                .smooth(0.8)
                .specular([1.0; 4], 0.015),
        );

        // Large Floor
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-10.0, -0.01, -10.0), Vec3::Y),
                    Vertex::new(Vec3::new(10.0, -0.01, -10.0), Vec3::Y),
                    Vertex::new(Vec3::new(10.0, -0.01, 10.0), Vec3::Y),
                    Vertex::new(Vec3::new(-10.0, -0.01, 10.0), Vec3::Y),
                ],
                vec![2, 1, 0, 3, 2, 0],
            ),
            MaterialDefinition::new().color([0.4, 0.4, 0.64313, 1.0]),
        );

        // Large Roof
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-10.0, 8.5, -10.0), -Vec3::Y),
                    Vertex::new(Vec3::new(10.0, 8.5, -10.0), -Vec3::Y),
                    Vertex::new(Vec3::new(10.0, 8.5, 10.0), -Vec3::Y),
                    Vertex::new(Vec3::new(-10.0, 8.5, 10.0), -Vec3::Y),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new()
                .color([0.898, 0.87, 0.815, 1.0])
                .smooth(0.877)
                .specular([1.0; 4], 0.327),
        );

        // Floor
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::Y),
                    Vertex::new(Vec3::new(width, 0.0, -depth), Vec3::Y),
                    Vertex::new(Vec3::new(width, 0.0, depth), Vec3::Y),
                    Vertex::new(Vec3::new(-width, 0.0, depth), Vec3::Y),
                ],
                vec![2, 1, 0, 3, 2, 0],
            ),
            MaterialDefinition::new().color([0.898, 0.87, 0.815, 1.0]),
        );

        // Roof
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-width, height, -depth), -Vec3::Y),
                    Vertex::new(Vec3::new(width, height, -depth), -Vec3::Y),
                    Vertex::new(Vec3::new(width, height, depth), -Vec3::Y),
                    Vertex::new(Vec3::new(-width, height, depth), -Vec3::Y),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().color([1.0, 0.9647, 0.9019, 1.0]),
        );

        // Right Wall
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::X),
                    Vertex::new(Vec3::new(-width, height, -depth), Vec3::X),
                    Vertex::new(Vec3::new(-width, height, depth), Vec3::X),
                    Vertex::new(Vec3::new(-width, 0.0, depth), Vec3::X),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().color([0.0705, 0.596, 0.2078, 1.0]),
        );

        // Left Wall
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(width, 0.0, -depth), -Vec3::X),
                    Vertex::new(Vec3::new(width, 0.0, depth), -Vec3::X),
                    Vertex::new(Vec3::new(width, height, depth), -Vec3::X),
                    Vertex::new(Vec3::new(width, height, -depth), -Vec3::X),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().color([0.7725, 0.12156, 0.188235, 1.0]),
        );

        // Back Wall
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::Z),
                    Vertex::new(Vec3::new(width, 0.0, -depth), Vec3::Z),
                    Vertex::new(Vec3::new(width, height, -depth), Vec3::Z),
                    Vertex::new(Vec3::new(-width, height, -depth), Vec3::Z),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().color([0.1254, 0.41176, 0.8274, 1.0]),
        );

        // Light
        scene.add_mesh(
            Transform::default(),
            MeshDefinition::from_data(
                vec![
                    Vertex::new(Vec3::new(-0.8, height - 0.02, -0.8), -Vec3::Y),
                    Vertex::new(Vec3::new(0.8, height - 0.02, -0.8), -Vec3::Y),
                    Vertex::new(Vec3::new(0.8, height - 0.02, 0.8), -Vec3::Y),
                    Vertex::new(Vec3::new(-0.8, height - 0.02, 0.8), -Vec3::Y),
                ],
                vec![0, 1, 2, 0, 2, 3],
            ),
            MaterialDefinition::new().emissive([1.0, 0.8588, 0.3529, 1.0], 60.0),
        );

        // Spheres
        scene.add_sphere(
            Vec3::new(0.0, 1.0, 4.8),
            1.15,
            MaterialDefinition::new()
                .specular([1.0; 4], 0.517)
                .smooth(1.0)
                .glass(1.6),
        );

        scene
    }

    pub fn metal() -> SceneDefinition {
        let mut scene = SceneDefinition::default();

        // Set camera
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0)),
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        // Add spheres
        scene.add_sphere(
            Vec3::new(0.0, -100.5, -1.0),
            100.0,
            MaterialDefinition::new().color([0.8, 0.8, 0.0, 1.0]),
        );

        scene.add_sphere(
            Vec3::new(0.0, 0.0, -1.0),
            0.5,
            MaterialDefinition::new().color([0.7, 0.3, 0.3, 1.0]),
        );

        scene.add_sphere(
            Vec3::new(-1.0, 0.0, -1.0),
            0.5,
            MaterialDefinition::new()
                .color([0.8, 0.8, 0.8, 1.0])
                .glass(1.3),
        );

        scene.add_sphere(
            Vec3::new(1.0, 0.0, -1.0),
            0.5,
            MaterialDefinition::new()
                .color([0.8, 0.6, 0.2, 1.0])
                .specular([1.0; 4], 0.15),
        );

        scene
    }

    pub fn balls() -> SceneDefinition {
        let mut scene = SceneDefinition::default();

        // Set camera
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(3.089, 1.53, -3.0), Vec3::new(-2.0, -1.0, 2.0)),
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        // Add spheres
        scene.add_sphere(
            Vec3::new(-3.64, -0.42, 0.8028),
            0.75,
            MaterialDefinition::new()
                .specular([1.0; 4], 0.7)
                .color([1.0, 1.0, 1.0, 1.0]),
        );

        scene.add_sphere(
            Vec3::new(-2.54, -0.72, 0.5),
            0.6,
            MaterialDefinition::new()
                .color([1.0, 0.0, 0.0, 1.0])
                .specular([1.0, 0.0, 0.0, 1.0], 0.5),
        );

        scene.add_sphere(
            Vec3::new(-1.27, -0.72, 1.0),
            0.5,
            MaterialDefinition::new()
                .color([0.0, 1.0, 0.0, 1.0])
                .specular([0.0, 1.0, 0.0, 1.0], 0.2),
        );

        scene.add_sphere(
            Vec3::new(-0.5, -0.9, 1.55),
            0.35,
            MaterialDefinition::new().color([0.0, 0.0, 1.0, 1.0]),
        );

        // Floor
        scene.add_sphere(
            Vec3::new(-3.46, -15.88, 2.76),
            15.0,
            MaterialDefinition::new().color([0.5, 0.0, 0.8, 1.0]),
        );

        // Light Object
        scene.add_sphere(
            Vec3::new(-7.44, -0.72, 20.0),
            15.0,
            MaterialDefinition::new()
                .color([0.1, 0.1, 0.1, 0.0])
                .emissive([1.0, 1.0, 1.0, 1.0], 1.0),
        );

        scene
    }

    pub fn sponza() -> SceneDefinition {
        let mut scene_def = SceneDefinition::default();

        scene_def.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::Y * 4.0, Vec3::X),
            ..Default::default()
        });

        scene_def.add_mesh(
            Transform {
                pos: Vec3::ZERO,
                rot: Quat::IDENTITY,
                scale: Vec3::splat(0.05),
            },
            MeshDefinition::FromFile {
                path: "sponza.obj".to_string(),
                use_mtl: true,
            },
            MaterialDefinition::texture_from_obj(),
        );
        scene_def.add_mesh(
            Transform {
                pos: Vec3::new(-15.0, 60.0, 0.0),
                rot: Quat::from_rotation_x(PI / 2.0),
                scale: Vec3::new(40.0, 20.0, 1.0),
            },
            MeshDefinition::from_data(Mesh::quad(), vec![0, 1, 2, 0, 2, 3]),
            MaterialDefinition::default().emissive([1.0; 4], 4.0),
        );

        scene_def.add_sphere(
            Vec3::new(5.0, 2.0, 0.0),
            2.0,
            MaterialDefinition {
                emission_color: [1.0; 4],
                emission_strength: 10.0,
                color: [1.0; 4],
                specular_color: [1.0; 4],
                absorption: [0.0; 4],
                absorption_stength: 0.0,
                smoothness: 0.0,
                specular: 0.0,
                ior: 1.0,
                flag: MaterialFlag::NORMAL,
                texture: None,
            },
        );
        scene_def
    }

    pub fn to_uniform(&self) -> SceneUniform {
        let mut n_vertices: u32 = 0;
        let mut n_indices: u32 = 0;
        for mesh in self.meshes.iter() {
            n_vertices += mesh.mesh.vertices.len() as u32;
            n_indices += mesh.mesh.indices.len() as u32;
        }
        SceneUniform {
            spheres: self.spheres.len() as u32,
            n_vertices,
            n_indices,
            meshes: self.meshes.len() as u32,
            camera: self.camera.to_uniform(),
            nodes: self.bvh_data.nodes.len() as u32,
            padding: [0.0; 6],
        }
    }
}
