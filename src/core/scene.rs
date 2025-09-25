use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use egui::ahash::AHashMap;
use egui_wgpu::wgpu;
use glam::Vec3;
use rand::Rng;

use crate::core::{
    bvh::{BVH, Node},
    camera::{CameraDescriptor, CameraUniform},
    mesh::{Material, Mesh, MeshUniform, Sphere, Vertex},
};

use super::camera::Camera;

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub meshes: Vec<Mesh>,
    pub bvh: BVH,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct SceneUniform {
    spheres: u32,
    vertices: u32,
    indices: u32,
    meshes: u32,
    camera: CameraUniform,
    nodes: u32,
    padding: [f32; 3],
}

#[allow(dead_code)]
impl Scene {
    pub fn new(_config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
            origin: Vec3::ZERO,
            look_at: Vec3::ZERO,
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            // aspect: config.width as f32 / config.height as f32,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
            aperture: 1.0,
            focus_dist: 2.0,
        });
        Self {
            camera,
            spheres: vec![],
            meshes: vec![],
            bvh: BVH::empty(),
        }
    }
    pub fn bvh(&mut self, meshes: &Vec<MeshUniform>) -> &Vec<Node> {
        if self.bvh.n_nodes == 0 && self.meshes.len() > 0 {
            let (vertices, indices) = self.vertices_and_indices();
            self.bvh = BVH::build(meshes, vertices, indices);
        }
        &self.bvh.nodes
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
    pub async fn load_mesh(&mut self, path: &Path) {
        self.meshes.extend(load_model_obj(path).await);
    }

    pub fn vertices_and_indices(&self) -> (Vec<Vertex>, Vec<u32>) {
        let mut vertices: Vec<Vertex> =
            Vec::with_capacity(self.meshes.iter().map(|m| m.vertices.len()).sum());
        let mut indices: Vec<u32> =
            Vec::with_capacity(self.meshes.iter().map(|m| m.indices.len()).sum());

        for mesh in &self.meshes {
            let vertex_offset = vertices.len() as u32;
            vertices.extend_from_slice(&mesh.vertices);
            indices.extend(mesh.indices.iter().map(|i| i + vertex_offset));
        }
        (vertices, indices)
    }

    pub fn meshes(&self) -> Vec<MeshUniform> {
        let mut meshes: Vec<MeshUniform> = Vec::with_capacity(self.meshes.len());
        let mut first = 0;
        for mesh in &self.meshes {
            meshes.push(MeshUniform {
                first,
                triangles: (mesh.indices.len() / 3) as u32,
                offset: 0,
                _padding: 0.0,
                pos: mesh.position.into(),
                _padding2: 0.0,
                material: mesh.material,
            });
            first += mesh.indices.len() as u32;
        }
        meshes
    }

    pub async fn obj_test(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            origin: Vec3::new(5.0, 0.0, 0.0),
            look_at: Vec3::new(1.0, 0.0, 0.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 1.0,
        });

        // scene.add_sphere(Sphere::new(
        //     Vec3::new(-4.0, 1.0, 0.0),
        //     1.0,
        //     Material::new().color([1.0, 0.0, 0.0, 1.0]),
        // ));

        scene.load_mesh(Path::new("dragon_large.obj")).await;

        scene
    }
    pub fn random_balls(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);
        scene.set_camera(&CameraDescriptor {
            origin: Vec3::new(13.0, 2.0, 3.0),
            look_at: Vec3::new(0.0, 0.0, 0.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 20.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 10.0,
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
                    .specular([1.0, 1.0, 1.0, 1.0], 0.9),
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
            origin: Vec3::new(0.0, 1.0, 3.0),
            look_at: Vec3::new(0.0, 1.0, 2.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 0.1,
        });

        // Floor
        scene.add_mesh(Mesh {
            position: Vec3::ZERO,
            size: Vec3::ONE,
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
            position: Vec3::ZERO,
            size: Vec3::ONE,
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
            position: Vec3::ZERO,
            size: Vec3::ONE * 2.0,
            // material: Material::new().color([1.0, 1.0, 0.0, 1.0]),
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
            position: Vec3::ZERO,
            size: Vec3::ONE * 2.0,
            // material: Material::new().color([0.0, 1.0, 0.0, 1.0]),
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
            position: Vec3::ZERO,
            size: Vec3::ONE * 2.0,
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
            position: Vec3::ZERO,
            size: Vec3::ONE * 2.0,
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
            position: Vec3::ZERO,
            size: Vec3::ONE,
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
    pub fn metal(config: &wgpu::SurfaceConfiguration) -> Self {
        let mut scene = Scene::new(config);

        scene.set_camera(&CameraDescriptor {
            origin: Vec3::new(0.0, 0.0, 3.0),
            look_at: Vec3::new(0.0, 0.0, -1.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 0.1,
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
            origin: Vec3::new(3.089, 1.53, -3.0),
            look_at: Vec3::new(-2.0, -1.0, 2.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 0.1,
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

    pub fn to_uniform(&self) -> SceneUniform {
        let mut vertices_len: u32 = 0;
        let mut indices_len: u32 = 0;
        for mesh in self.meshes.iter() {
            vertices_len += mesh.vertices.len() as u32;
            indices_len += mesh.indices.len() as u32;
        }
        SceneUniform {
            spheres: self.spheres.len() as u32,
            vertices: vertices_len,
            indices: indices_len,
            meshes: self.meshes.len() as u32,
            camera: self.camera.to_uniform(),
            nodes: self.bvh.n_nodes,
            padding: [69., 0., 0.],
        }
    }
}

const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));

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

pub async fn load_model_obj(path: &Path) -> Vec<Mesh> {
    let mut meshes: Vec<Mesh> = vec![];
    let path = std::path::Path::new(FILE).join("assets").join(path);

    let obj_text = load_string(&path).await.unwrap();
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
    for m in models.into_iter() {
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
            position: Vec3::ZERO,
            size: Vec3::ONE,
            material: Material::new().color([0.2, 0.2, 0.8, 1.0]).glass(1.5),
            vertices,
            indices,
        });
    }

    return meshes;
}
