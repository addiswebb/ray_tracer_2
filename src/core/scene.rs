use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use egui::ahash::AHashMap;
use egui_wgpu::wgpu::{self, util::DeviceExt};
use glam::{Vec3, Vec4};
use rand::Rng;

use crate::core::{
    camera::{CameraDescriptor, CameraUniform},
    mesh::{MeshUniform, Sphere, Vertex},
};

use super::camera::Camera;

pub struct Scene {
    pub camera: Camera,
    pub spheres: Vec<Sphere>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub meshes: Vec<MeshUniform>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct SceneUniform {
    spheres: u32,
    vertices: u32,
    indices: u32,
    meshes: u32,
    camera: CameraUniform,
}

#[allow(dead_code)]
impl Scene {
    pub fn new(config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
            origin: Vec3::ZERO,
            look_at: Vec3::ZERO,
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 1.0,
            focus_dist: 2.0,
        });
        Self {
            camera,
            spheres: vec![],
            vertices: vec![],
            indices: vec![],
            meshes: vec![],
        }
    }
    pub async fn obj_test(config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
            origin: Vec3::new(0.0, 0.0, 0.0),
            look_at: Vec3::new(1.0, 0.0, 0.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 1.0,
        });

        let spheres = vec![Sphere::new(
            Vec3::new(4.0, 1.0, 0.0),
            0.2,
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            Vec4::ONE,
            10.0,
            1.0,
        )];

        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];
        let mut meshes: Vec<MeshUniform> = vec![];

        let _ = load_model_obj(
            Path::new("Suzanne.obj"),
            &mut indices,
            &mut vertices,
            &mut meshes,
        )
        .await
        .unwrap();

        Self {
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn random_balls(config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
            origin: Vec3::new(13.0, 2.0, 3.0),
            look_at: Vec3::new(0.0, 0.0, 0.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 20.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.1,
            focus_dist: 10.0,
        });
        let mut spheres: Vec<Sphere> = vec![
            // Floor
            Sphere::new(
                Vec3::new(0.0, -1000.0, 0.0),
                1000.0,
                Vec4::new(0.5, 0.5, 0.5, 1.0),
                Vec4::ZERO,
                0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(0.0, 1.0, 0.0),
                1.0,
                Vec4::ONE,
                Vec4::ZERO,
                0.0,
                -1.0,
            ),
            Sphere::new(
                Vec3::new(-4.0, 1.0, 0.0),
                1.0,
                Vec4::new(0.4, 0.2, 0.1, 1.0),
                Vec4::ZERO,
                0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(4.0, 1.0, 0.0),
                1.0,
                Vec4::new(0.7, 0.6, 0.5, 1.0),
                Vec4::ZERO,
                0.0,
                0.9,
            ),
        ];

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
                        let albedo = Vec4::new(
                            rng.random::<f32>(),
                            rng.random::<f32>(),
                            rng.random::<f32>(),
                            1.0,
                        );
                        spheres.push(Sphere::new(center, 0.2, albedo, Vec4::ZERO, 0.0, 0.0));
                    } else if mat < 0.95 {
                        let albedo = Vec4::new(
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            rng.random_range(0.5..1.0),
                            1.0,
                        );
                        let fuzz = rng.random_range(0.0..0.5);
                        spheres.push(Sphere::new(center, 0.2, albedo, Vec4::ZERO, 0.0, fuzz));
                    } else {
                        spheres.push(Sphere::new(center, 0.2, Vec4::ONE, Vec4::ZERO, 0.0, -1.0));
                    }
                }
            }
        }

        let vertices = vec![Vertex::new(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(2.0, -3.0, -1.0),
        )];
        let indices = vec![1u32];
        let meshes = vec![MeshUniform::new(
            Vec3::new(0.0, 0.0, 0.0),
            0,
            0,
            0,
            Vec4::new(0.0, 0.6, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            0.0,
            0.5,
        )];
        println!("{:#?}", vertices.len());
        println!("{:#?}", indices.len());
        println!("{:#?}", meshes.len());
        println!("{:#?}", spheres.len());
        Self {
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn room(config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
            origin: Vec3::new(-7.0, 0.0, 0.0),
            look_at: Vec3::new(1.0, 0.0, 0.0),
            view_up: Vec3::new(0.0, 1.0, 0.0),
            fov: 45.0,
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0,
            aperture: 0.0,
            focus_dist: 0.1,
        });

        let spheres = vec![
            Sphere::new(
                Vec3::new(4.0, 0.0, 1.7),
                1.2,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                1.0,
            ),
            Sphere::new(
                Vec3::new(4.0, 0.0, -1.7),
                1.2,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.5,
            ),
        ];

        #[allow(unused_mut)]
        let mut vertices = vec![
            Vertex::new(Vec3::new(3.0, -3.0, -3.0), Vec3::new(2.0, -3.0, -3.0)),
            Vertex::new(Vec3::new(3.0, -3.0, 3.0), Vec3::new(4.0, -3.0, 0.0)),
            Vertex::new(Vec3::new(-3.0, -3.0, 3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0, -3.0, -3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(3.0, 3.0, -3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(3.0, 3.0, 3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0, 3.0, 3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(-3.0, 3.0, -3.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(1.0, 1.0, -1.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, 1.0), Vec3::new(3.0, -4.0, 2.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, -1.0), Vec3::new(3.0, -4.0, 2.0)),
        ];
        #[allow(unused_mut)]
        let mut indices = vec![
            3u32, 2u32, 1u32, 3u32, 1u32, 0u32, 7u32, 0u32, 4u32, 7u32, 3u32, 0u32, 7u32, 6u32,
            2u32, 7u32, 2u32, 3u32, 2u32, 6u32, 5u32, 2u32, 5u32, 1u32, 1u32, 5u32, 4u32, 1u32,
            4u32, 0u32, 5u32, 6u32, 7u32, 5u32, 7u32, 4u32, 9u32, 10u32, 11u32, 9u32, 11u32, 8u32,
        ];
        #[allow(unused_mut)]
        let mut meshes = vec![
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                0,
                2,
                0,
                Vec4::new(0.8, 0.2, 0.2, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                6,
                2,
                0,
                Vec4::new(0.2, 0.8, 0.2, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                12,
                2,
                0,
                Vec4::new(0.2, 0.2, 0.8, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                18,
                2,
                0,
                Vec4::new(0.5, 0.5, 0.1, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                24,
                2,
                0,
                Vec4::new(0.0, 0.5, 0.5, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 0.0, 0.0),
                30,
                2,
                0,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.5,
            ),
            MeshUniform::new(
                Vec3::new(3.0, 1.9, 0.0),
                36,
                2,
                0,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                10.5,
                0.0,
            ),
        ];

        Self {
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn metal(config: &wgpu::SurfaceConfiguration) -> Self {
        // let lookfrom= Vec3::new(3.0,3.0,2.0);
        // let lookat = Vec3::new(0.0,0.0,-1.0);
        // let length = (lookfrom - lookat).length();

        let camera = Camera::new(&CameraDescriptor {
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
        let spheres = vec![
            //floor
            Sphere::new(
                Vec3::new(0.0, -100.5, -1.0),
                100.0,
                Vec4::new(0.8, 0.8, 0.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(0.0, 0.0, -1.0),
                0.5,
                Vec4::new(0.7, 0.3, 0.3, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.0,
            ),
            Sphere::new(
                Vec3::new(-1.0, 0.0, -1.0),
                0.5,
                Vec4::new(0.8, 0.8, 0.8, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                -1.0,
            ),
            Sphere::new(
                Vec3::new(1.0, 0.0, -1.0),
                0.5,
                Vec4::new(0.8, 0.6, 0.2, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.15,
            ),
        ];
        //TODO allow for no meshes or sphers in a scene
        let vertices = vec![Vertex::new(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(2.0, -3.0, -1.0),
        )];
        let indices = vec![1u32];
        let meshes = vec![MeshUniform::new(
            Vec3::new(0.0, 0.0, 0.0),
            0,
            0,
            0,
            Vec4::new(0.0, 0.6, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            0.0,
            0.5,
        )];
        Self {
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }
    pub fn balls(config: &wgpu::SurfaceConfiguration) -> Self {
        let camera = Camera::new(&CameraDescriptor {
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

        let spheres = vec![
            Sphere::new(
                Vec3::new(-3.64, -0.42, 0.8028),
                0.75,
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.7,
            ),
            Sphere::new(
                Vec3::new(-2.54, -0.72, 0.5),
                0.6,
                Vec4::new(1.0, 0.0, 0.0, 1.0),
                Vec4::new(0.0, 0.0, 0.0, 1.0),
                0.0,
                0.5,
            ),
            Sphere::new(
                Vec3::new(-1.27, -0.72, 1.0),
                0.5,
                Vec4::new(0.0, 1.0, 0.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.2,
            ),
            Sphere::new(
                Vec3::new(-0.5, -0.9, 1.55),
                0.35,
                Vec4::new(0.0, 0.0, 1.0, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.0,
            ),
            /*  floor*/
            Sphere::new(
                Vec3::new(-3.46, -15.88, 2.76),
                15.0,
                Vec4::new(0.5, 0.0, 0.8, 1.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                0.0,
                0.0,
            ),
            /*  Light Object       */
            Sphere::new(
                Vec3::new(-7.44, -0.72, 20.0),
                15.0,
                Vec4::new(0.1, 0.1, 0.1, 0.0),
                Vec4::new(1.0, 1.0, 1.0, 1.0),
                2.0,
                0.0,
            ),
        ];
        let vertices = vec![Vertex::new(
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(2.0, -3.0, -1.0),
        )];
        let indices = vec![1u32];
        let meshes = vec![MeshUniform::new(
            Vec3::new(0.0, 0.0, 0.0),
            0,
            0,
            0,
            Vec4::new(0.0, 0.6, 0.0, 1.0),
            Vec4::new(1.0, 1.0, 1.0, 1.0),
            0.0,
            0.5,
        )];
        // #[allow(unused_mut)]
        // let mut vertices = vec![
        //     Vertex::new(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0,-3.0,-1.0)),
        //     Vertex::new(Vec3::new(0.0, 0.15, 0.0), Vec3::new(4.0,-3.0, 0.0)),
        //     Vertex::new(Vec3::new(1.0, 0.3, 0.0), Vec3::new(3.0,-4.0, 2.0)),
        // ];
        // #[allow(unused_mut)]
        // let mut indices = vec![
        //     2u32,1u32,0u32,
        // ];
        // #[allow(unused_mut)]
        // let mut meshes = vec![
        //     Mesh::new(
        //         Vec3::new(0.0,0.0,0.0),
        //         0, 1, 0,
        //         Vec4::new(0.0,0.6,0.0,1.0),
        //         Vec4::new(1.0,1.0,1.0,1.0), 0.0, 0.5,
        //     ),
        // ];

        //load_model(Path::new("cube2.obj"),&mut vertices, &mut indices, &mut meshes).await.unwrap();

        Self {
            camera,
            spheres,
            vertices,
            indices,
            meshes,
        }
    }

    pub fn to_uniform(&self) -> SceneUniform {
        SceneUniform {
            spheres: self.spheres.len() as u32,
            vertices: self.vertices.len() as u32,
            indices: self.indices.len() as u32,
            meshes: self.meshes.len() as u32,
            camera: self.camera.to_uniform(),
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

pub async fn load_model_obj(
    path: &Path,
    indices: &mut Vec<u32>,
    vertices: &mut Vec<Vertex>,
    meshes: &mut Vec<MeshUniform>,
) -> anyhow::Result<bool> {
    let path = std::path::Path::new(FILE).join("assets").join(path);

    let obj_text = load_string(&path).await?;
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
    let first = vertices.len() as u32;
    for m in models.into_iter() {
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

        meshes.push(MeshUniform::new(
            Vec3::new(5.0, 0.0, 0.0),
            first,
            (m.mesh.positions.len() / 3) as u32 - 1,
            0,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec4::ONE,
            0.0,
            0.5,
        ));
    }

    return Ok(true);
}
