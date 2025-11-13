use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use egui::ahash::AHashMap;
use egui_wgpu::wgpu;
use glam::{Quat, Vec3, Vec4};
use rand::Rng;

use crate::core::{
    bvh::{self, BVH, MeshDataList, Node, Quality},
    camera::{CameraDescriptor, CameraUniform},
    mesh::{Material, Mesh, Sphere, Transform, Vertex},
};

use super::camera::Camera;

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub meshes: Vec<Mesh>,
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
    pub fn new(_config: &wgpu::SurfaceConfiguration) -> Self {
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
    pub fn bvh(&mut self) -> &Vec<Node> {
        if !self.built_bvh && self.meshes.len() > 0 {
            self.bvh_data = BVH::build_per_mesh(&self.meshes, bvh::Quality::High);
            self.built_bvh = true;
        }
        &self.bvh_data.nodes
    }
    pub fn set_camera(&mut self, camera_descriptor: &CameraDescriptor) {
        self.camera = Camera::new(camera_descriptor);
    }
    pub fn add_sphere(&mut self, sphere: Sphere) {
        self.spheres.push(sphere);
    }
    pub fn add_spheres(&mut self, spheres: Vec<Sphere>) {
        self.spheres.extend(spheres);
    }
    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    pub async fn load_mesh(&mut self, path: &Path, transform: Transform, material: Material) {
        self.meshes
            .extend(load_model_obj(path, transform, material).await);
    }

    pub async fn obj_test(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 1.0,
            ..Default::default()
        });

        let mut mesh = load_model_obj(
            Path::new("dragon.obj"),
            Transform::default(),
            Material::new(),
        )
        .await;
        let dragon = mesh.first_mut().unwrap();
        dragon.material = Material::new().color([1.0, 0.0, 0.0, 1.0]);

        scene.meshes.extend(mesh);

        let mut mesh = load_model_obj(
            Path::new("dragon_large.obj"),
            Transform::default(),
            Material::new(),
        )
        .await;
        println!("LENGTH: {}", mesh.len());
        let dragon = mesh.first_mut().unwrap();
        dragon.material = Material::new()
            .color([1.0, 0.0, 1.0, 1.0])
            .emissive([1.0, 0.0, 0.0, 1.0], 0.3);
        scene.meshes.extend(mesh);

        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new()
                .color([1.0, 1.0, 0.0, 1.0])
                .emissive([1.0, 0.0, 0.0, 1.0], 0.4),
            vertices: vec![
                Vertex::new(Vec3::new(0.5, 0.0, -1.0), Vec3::X),
                Vertex::new(Vec3::new(0.5, 1.0, -1.0), Vec3::X),
                Vertex::new(Vec3::new(0.0, 1.0, 1.0), Vec3::X),
                Vertex::new(Vec3::new(0.2, 0.0, 1.0), Vec3::X),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_sphere(Sphere::new(
            Vec3::new(1.8, 0.1, 1.0),
            0.6,
            Material::new().color([1.0, 0.0, 0.0, 1.0]),
        ));

        scene.add_sphere(Sphere::new(
            Vec3::new(1.0, 0.5, 1.0),
            0.3,
            Material::new().color([1.0, 0.0, 0.0, 1.0]),
        ));

        scene.add_sphere(Sphere::new(
            Vec3::new(0.0, -10.0, 0.0),
            10.0,
            Material::new().color([1.0, 0.0, 0.0, 1.0]),
        ));

        scene
    }
    pub async fn lighting_test(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(5.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 1.0,
            ..Default::default()
        });

        let mut mesh = load_model_obj(
            Path::new("dragon.obj"),
            Transform::default(),
            Material::new(),
        )
        .await;
        let sphere = mesh.first_mut().unwrap();
        sphere.material = Material::new()
            .color([1.0, 1.0, 1.0, 1.0])
            .emissive([0.2, 0.2, 0.8, 1.0], 0.3);

        scene.meshes.extend(mesh);

        scene
    }
    pub fn texture_test(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.28, 13.5), Vec3::new(0.0, 1.28, 12.5)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            ..Default::default()
        });

        // scene.add_mesh(Mesh {
        //     label: Some("Back Wall".to_string()),
        //     transform: Transform::default(),
        //     material: Material::new().color([1.0, 1.0, 1.0, 1.0]).texture(0),
        //     vertices: Mesh::quad(Transform {
        //         pos: Vec3::new(0.0, 0.0, -2.0),
        //         rot: Quat::IDENTITY,
        //         scale: Vec3::new(3.0, 3.0, 1.0),
        //     }),
        //     indices: vec![0, 1, 2, 0, 2, 3],
        // });
        scene.add_sphere(Sphere::new(
            Vec3::new(0.0, 0.0, 3.0),
            1.0,
            Material::new().specular([1.0; 4], 0.2).texture(0),
        ));
        scene
    }
    pub fn random_balls(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(13.0, 2.0, 3.0), Vec3::new(0.0, 0.0, 0.0)),
            fov: 20.0,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
            focus_dist: 10.0,
            ..Default::default()
        });
        scene.add_spheres(vec![
            // Floor
            Sphere::new(
                Vec3::new(0.0, -1000.0, 0.0),
                1000.0,
                Material::new().color([0.5, 0.5, 0.5, 1.0]),
            ),
            Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0, Material::new().glass(1.5)),
            Sphere::new(
                Vec3::new(-4.0, 1.0, 0.0),
                1.0,
                Material::new().color([0.4, 0.2, 0.1, 1.0]),
            ),
            Sphere::new(
                Vec3::new(4.0, 1.0, 0.0),
                1.0,
                Material::new()
                    .color([0.7, 0.6, 0.5, 1.0])
                    .specular([0.7, 0.6, 0.5, 1.0], 1.0)
                    .smooth(1.0),
            ),
        ]);

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
                        scene.add_sphere(Sphere::new(center, 0.2, Material::new().color(albedo)));
                    } else if mat < 0.95 {
                        let albedo = [
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            1.0,
                        ];
                        let fuzz = rng.random_range(0.0..0.5);
                        scene.add_sphere(Sphere::new(
                            center,
                            0.2,
                            Material::new()
                                .color(albedo)
                                .specular([1.0, 1.0, 1.0, 1.0], fuzz),
                        ));
                    } else {
                        scene.add_sphere(Sphere::new(center, 0.2, Material::new().glass(1.3)));
                    }
                }
            }
        }

        scene
    }
    pub fn room(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.0, 3.0), Vec3::new(0.0, 1.0, 2.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        // Floor
        scene.add_mesh(Mesh {
            label: Some("Floor".to_string()),
            transform: Transform::default(),
            material: Material::new().color([1.0, 0.0, 0.0, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::Y),
                Vertex::new(Vec3::new(2.0, 0.0, -2.0), Vec3::Y),
                Vertex::new(Vec3::new(2.0, 0.0, 2.0), Vec3::Y),
                Vertex::new(Vec3::new(-2.0, 0.0, 2.0), Vec3::Y),
            ],
            indices: vec![2, 1, 0, 3, 2, 0],
        });

        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new().color([0.0, 0.3, 0.3, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-2.0, 4.0, -2.0), -Vec3::Y),
                Vertex::new(Vec3::new(2.0, 4.0, -2.0), -Vec3::Y),
                Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::Y),
                Vertex::new(Vec3::new(-2.0, 4.0, 2.0), -Vec3::Y),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new()
                .specular([1.0, 1.0, 1.0, 1.0], 1.0)
                .smooth(1.0),
            vertices: vec![
                Vertex::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::X),
                Vertex::new(Vec3::new(-2.0, 4.0, -2.0), Vec3::X),
                Vertex::new(Vec3::new(-2.0, 4.0, 2.0), Vec3::X),
                Vertex::new(Vec3::new(-2.0, 0.0, 2.0), Vec3::X),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new()
                .specular([1.0, 1.0, 1.0, 1.0], 0.99)
                .smooth(0.99),
            vertices: vec![
                Vertex::new(Vec3::new(2.0, 0.0, -2.0), -Vec3::X),
                Vertex::new(Vec3::new(2.0, 0.0, 2.0), -Vec3::X),
                Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::X),
                Vertex::new(Vec3::new(2.0, 4.0, -2.0), -Vec3::X),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new()
                .color([0.0, 0.5, 1.0, 1.0])
                .specular([1.0, 1.0, 1.0, 1.0], 1.0)
                .smooth(1.0),
            vertices: vec![
                Vertex::new(Vec3::new(-2.0, 0.0, -2.0), Vec3::Z),
                Vertex::new(Vec3::new(2.0, 0.0, -2.0), Vec3::Z),
                Vertex::new(Vec3::new(2.0, 4.0, -2.0), Vec3::Z),
                Vertex::new(Vec3::new(-2.0, 4.0, -2.0), Vec3::Z),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        // Back wall (blue)
        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new()
                .color([0.2, 0.2, 0.82, 1.0])
                .specular([1.0, 1.0, 1.0, 1.0], 0.99)
                .smooth(0.99),
            vertices: vec![
                Vertex::new(Vec3::new(-2.0, 0.0, 2.0), -Vec3::Z),
                Vertex::new(Vec3::new(2.0, 0.0, 2.0), -Vec3::Z),
                Vertex::new(Vec3::new(2.0, 4.0, 2.0), -Vec3::Z),
                Vertex::new(Vec3::new(-2.0, 4.0, 2.0), -Vec3::Z),
            ],
            indices: vec![2, 1, 0, 3, 2, 0],
        });
        scene.add_mesh(Mesh {
            label: None,
            transform: Transform::default(),
            material: Material::new().emissive([1.0, 1.0, 1.0, 1.0], 3.0),
            vertices: vec![
                Vertex::new(Vec3::new(-0.4, 3.98, -0.4), -Vec3::Y),
                Vertex::new(Vec3::new(0.4, 3.98, -0.4), -Vec3::Y),
                Vertex::new(Vec3::new(0.4, 3.98, 0.4), -Vec3::Y),
                Vertex::new(Vec3::new(-0.4, 3.98, 0.4), -Vec3::Y),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        scene.add_sphere(Sphere::new(
            Vec3::new(0.4, 1.0, 0.0),
            0.3,
            Material::new().color([0.4, 0.9, 0.4, 1.0]).glass(1.34),
        ));

        // Right diffuse/specular sphere
        scene.add_sphere(Sphere::new(
            Vec3::new(-0.4, 1.0, 0.0),
            0.4,
            Material::new()
                .color([0.7, 0.7, 0.7, 1.0])
                .specular([1.0, 1.0, 1.0, 1.0], 0.2),
        ));
        // scene.meshes = vec![];
        scene
    }
    pub async fn dragon(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.2, 9.0), Vec3::new(0.0, 1.2, 8.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        scene
            .load_mesh(
                Path::new("dragon.obj"),
                Transform {
                    pos: Vec3::new(0.0, 1.2, 0.0),
                    rot: Quat::from_euler(glam::EulerRot::XYX, 0.0, -1.5708, 0.0),
                    scale: Vec3::splat(5.0),
                },
                Material::new()
                    .color([0.96078, 0.11372, 0.4039, 1.0])
                    .smooth(0.8)
                    .specular([1.0; 4], 0.015),
            )
            .await;
        scene
    }
    pub async fn room_2(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 1.28, 13.5), Vec3::new(0.0, 1.28, 12.5)),
            fov: 26.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 8.6,
            defocus_strength: 100.0,
            diverge_strength: 1.5,
        });

        let (width, depth, height) = (3.0, 2.0, 4.0);
        scene
            .load_mesh(
                Path::new("Dragon_80K.obj"),
                Transform {
                    pos: Vec3::new(0.0, 1.2, -0.6),
                    rot: Quat::from_euler(glam::EulerRot::XYX, 0.0, -1.5708, 0.0),
                    scale: Vec3::splat(4.7),
                },
                Material::new()
                    .color([0.96078, 0.11372, 0.4039, 1.0])
                    .smooth(0.8)
                    .specular([1.0; 4], 0.015),
            )
            .await;
        // Large floor
        scene.add_mesh(Mesh {
            label: Some("Large Floor".to_string()),
            transform: Transform::default(),
            material: Material::new().color([0.4, 0.4, 0.64313, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-10.0, -0.01, -10.0), Vec3::Y),
                Vertex::new(Vec3::new(10.0, -0.01, -10.0), Vec3::Y),
                Vertex::new(Vec3::new(10.0, -0.01, 10.0), Vec3::Y),
                Vertex::new(Vec3::new(-10.0, -0.01, 10.0), Vec3::Y),
            ],
            indices: vec![2, 1, 0, 3, 2, 0],
        });

        // Large Roof
        scene.add_mesh(Mesh {
            label: Some("Large Roof".to_string()),
            transform: Transform::default(),
            material: Material::new()
                .color([0.898, 0.87, 0.815, 1.0])
                .smooth(0.877)
                .specular([1.0; 4], 0.327),
            vertices: vec![
                Vertex::new(Vec3::new(-10.0, 8.5, -10.0), -Vec3::Y),
                Vertex::new(Vec3::new(10.0, 8.5, -10.0), -Vec3::Y),
                Vertex::new(Vec3::new(10.0, 8.5, 10.0), -Vec3::Y),
                Vertex::new(Vec3::new(-10.0, 8.5, 10.0), -Vec3::Y),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        // Floor
        scene.add_mesh(Mesh {
            label: Some("Floor".to_string()),
            transform: Transform::default(),
            material: Material::new().color([0.898, 0.87, 0.815, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::Y),
                Vertex::new(Vec3::new(width, 0.0, -depth), Vec3::Y),
                Vertex::new(Vec3::new(width, 0.0, depth), Vec3::Y),
                Vertex::new(Vec3::new(-width, 0.0, depth), Vec3::Y),
            ],
            indices: vec![2, 1, 0, 3, 2, 0],
        });

        // Roof
        scene.add_mesh(Mesh {
            label: Some("Roof".to_string()),
            transform: Transform::default(),
            material: Material::new().color([1.0, 0.9647, 0.9019, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-width, height, -depth), -Vec3::Y),
                Vertex::new(Vec3::new(width, height, -depth), -Vec3::Y),
                Vertex::new(Vec3::new(width, height, depth), -Vec3::Y),
                Vertex::new(Vec3::new(-width, height, depth), -Vec3::Y),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: Some("Right Wall".to_string()),
            transform: Transform::default(),
            material: Material::new().color([0.0705, 0.596, 0.2078, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::X),
                Vertex::new(Vec3::new(-width, height, -depth), Vec3::X),
                Vertex::new(Vec3::new(-width, height, depth), Vec3::X),
                Vertex::new(Vec3::new(-width, 0.0, depth), Vec3::X),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        // Left Wall
        scene.add_mesh(Mesh {
            label: Some("Left Wall".to_string()),
            transform: Transform::default(),
            material: Material::new().color([0.7725, 0.12156, 0.188235, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(width, 0.0, -depth), -Vec3::X),
                Vertex::new(Vec3::new(width, 0.0, depth), -Vec3::X),
                Vertex::new(Vec3::new(width, height, depth), -Vec3::X),
                Vertex::new(Vec3::new(width, height, -depth), -Vec3::X),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: Some("Back Wall".to_string()),
            transform: Transform::default(),
            material: Material::new().color([0.1254, 0.41176, 0.8274, 1.0]),
            vertices: vec![
                Vertex::new(Vec3::new(-width, 0.0, -depth), Vec3::Z),
                Vertex::new(Vec3::new(width, 0.0, -depth), Vec3::Z),
                Vertex::new(Vec3::new(width, height, -depth), Vec3::Z),
                Vertex::new(Vec3::new(-width, height, -depth), Vec3::Z),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });

        scene.add_mesh(Mesh {
            label: Some("Light".to_string()),
            transform: Transform::default(),
            material: Material::new().emissive([1.0, 0.8588, 0.3529, 1.0], 60.0),
            vertices: vec![
                Vertex::new(Vec3::new(-0.8, height - 0.02, -0.8), -Vec3::Y),
                Vertex::new(Vec3::new(0.8, height - 0.02, -0.8), -Vec3::Y),
                Vertex::new(Vec3::new(0.8, height - 0.02, 0.8), -Vec3::Y),
                Vertex::new(Vec3::new(-0.8, height - 0.02, 0.8), -Vec3::Y),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        });
        scene.add_sphere(Sphere::new(
            Vec3::new(0.0, 1.0, 4.8),
            1.15,
            Material {
                color: [0.0; 4],
                emission_color: [0.0; 4],
                specular_color: [1.0, 1.0, 1.0, 1.0],
                absorption: [0.0; 4],
                absorption_stength: 0.0,
                emission_strength: 0.0,
                smoothness: 1.0,
                specular: 0.517,
                ior: 1.6,
                flag: 1,
                ..Default::default()
            },
        ));
        // scene
        //     .load_mesh(
        //         Path::new("Icosphere.obj"),
        //         Transform {
        //             pos: Vec3::new(1.0, 1.0, 5.0),
        //             rot: Quat::default(),
        //             scale: Vec3::ONE,
        //         },
        //         Material {
        //             color: [0.95, 0.95, 1.0, 0.1],
        //             emission_color: [0.0, 0.0, 0.0, 0.0],
        //             specular_color: [1.0, 1.0, 1.0, 1.0],
        //             absorption: [0.5, 0.5, 0.5, 1.0],
        //             absorption_stength: 0.1,
        //             emission_strength: 0.0,
        //             smoothness: 0.99, // Very smooth for clear glass
        //             specular: 0.05,   // Low specular (glass uses refraction primarily)
        //             ior: 1.5,         // Glass IOR
        //             flag: 1,
        //             _p1: [0.0; 2],
        //         },
        //     )
        //     .await;

        scene
    }
    pub fn metal(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(0.0, 0.0, 3.0), Vec3::new(0.0, 0.0, -1.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });
        scene.add_spheres(vec![
            //floor
            Sphere::new(
                Vec3::new(0.0, -100.5, -1.0),
                100.0,
                Material::new().color([0.8, 0.8, 0.0, 1.0]),
            ),
            Sphere::new(
                Vec3::new(0.0, 0.0, -1.0),
                0.5,
                Material::new().color([0.7, 0.3, 0.3, 1.0]),
            ),
            Sphere::new(
                Vec3::new(-1.0, 0.0, -1.0),
                0.5,
                Material::new().color([0.8, 0.8, 0.8, 1.0]).glass(1.3),
            ),
            Sphere::new(
                Vec3::new(1.0, 0.0, -1.0),
                0.5,
                Material::new()
                    .color([0.8, 0.6, 0.2, 1.0])
                    .specular([1.0, 1.0, 1.0, 1.0], 0.15),
            ),
        ]);

        scene
    }
    pub fn balls(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(3.089, 1.53, -3.0), Vec3::new(-2.0, -1.0, 2.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        scene.add_spheres(vec![
            Sphere::new(
                Vec3::new(-3.64, -0.42, 0.8028),
                0.75,
                Material::new().specular([1.0, 1.0, 1.0, 1.0], 0.7),
            ),
            Sphere::new(
                Vec3::new(-2.54, -0.72, 0.5),
                0.6,
                Material::new()
                    .color([1.0, 0.0, 0.0, 1.0])
                    .specular([1.0, 0.0, 0.0, 1.0], 0.5),
            ),
            Sphere::new(
                Vec3::new(-1.27, -0.72, 1.0),
                0.5,
                Material::new()
                    .color([0.0, 1.0, 0.0, 1.0])
                    .specular([0.0, 1.0, 0.0, 1.0], 0.2),
            ),
            Sphere::new(
                Vec3::new(-0.5, -0.9, 1.55),
                0.35,
                Material::new().color([0.0, 0.0, 1.0, 1.0]),
            ),
            /*  floor*/
            Sphere::new(
                Vec3::new(-3.46, -15.88, 2.76),
                15.0,
                Material::new().color([0.5, 0.0, 0.8, 1.0]),
            ),
            /*  Light Object       */
            Sphere::new(
                Vec3::new(-7.44, -0.72, 20.0),
                15.0,
                Material::new()
                    .color([0.1, 0.1, 0.1, 0.0])
                    .emissive([1.0, 1.0, 1.0, 1.0], 1.0),
            ),
        ]);

        scene
    }
    pub async fn sponza(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            transform: Transform::cam(Vec3::new(8.0, 12.0, 10.5), Vec3::new(9.0, 12.0, 10.0)),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            focus_dist: 0.1,
            ..Default::default()
        });

        scene
            .load_mesh(
                Path::new("sponza.obj"),
                Transform {
                    pos: Vec3::ZERO,
                    rot: Quat::default(),
                    scale: Vec3::splat(0.02),
                },
                Material::default(),
            )
            .await;
        scene.add_sphere(Sphere {
            pos: [-15.78, 16.4, 8.25],
            radius: 1.0,
            material: Material::default().emissive([1.0; 4], 5.0),
        });
        scene
    }
    pub fn to_uniform(&self) -> SceneUniform {
        let mut n_vertices: u32 = 0;
        let mut n_indices: u32 = 0;
        for mesh in self.meshes.iter() {
            n_vertices += mesh.vertices.len() as u32;
            n_indices += mesh.indices.len() as u32;
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

pub const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));

pub async fn load_string(path: &Path) -> anyhow::Result<String> {
    assert!(
        path.exists(),
        "Text file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read_to_string(path)?)
}

#[allow(unused)]
pub async fn load_binary(path: &Path) -> anyhow::Result<Vec<u8>> {
    assert!(
        path.exists(),
        "Binary file does not exist: {}",
        path.display()
    );

    Ok(std::fs::read(path)?)
}

pub async fn load_model_obj(path: &Path, transform: Transform, material: Material) -> Vec<Mesh> {
    let mut meshes: Vec<Mesh> = vec![];
    let file_path = std::path::Path::new(FILE).join("assets").join(path);

    let obj_text = load_string(&file_path).await.unwrap();
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);
    let (models, _) = tobj::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default() // Skip material loading
        },
        |_p| Ok((Vec::new(), AHashMap::default())),
    )
    .unwrap();
    for (i, m) in models.into_iter().enumerate() {
        let mut vertices = vec![];
        let mut indices = vec![];
        for (i, _) in (0..m.mesh.positions.len() / 3).enumerate() {
            vertices.push(Vertex::new(
                Vec3::new(
                    m.mesh.positions[i * 3],
                    m.mesh.positions[i * 3 + 1],
                    m.mesh.positions[i * 3 + 2],
                ),
                Vec3::new(
                    m.mesh.normals[i * 3],
                    m.mesh.normals[i * 3 + 1],
                    m.mesh.normals[i * 3 + 2],
                ),
            ));
        }
        for index in m.mesh.indices {
            indices.push(index as u32);
        }

        meshes.push(Mesh {
            label: Some(path.to_str().unwrap().to_string() + "_" + &i.to_string()),
            transform,
            material,
            vertices,
            indices,
        });
    }

    return meshes;
}
