use std::{
    collections::HashMap,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
    sync::Arc,
};

use egui::ahash::AHashMap;
use egui_wgpu::{Texture, wgpu};
use glam::Vec3;

use crate::core::mesh::{Mesh, Transform, Vertex};

pub struct AssetManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    meshes: HashMap<String, Arc<Mesh>>,
    textures: HashMap<String, Arc<Texture>>,
}

pub enum MeshDefinition {
    FromFile {
        path: PathBuf,
    },
    FromData {
        vertices: Arc<Vec<Vertex>>,
        indices: Arc<Vec<u32>>,
    },
}

impl MeshDefinition {
    pub fn from_data(vertices: Vec<Vertex>, indices: Vec<u32>) -> MeshDefinition {
        MeshDefinition::FromData {
            vertices: Arc::new(vertices),
            indices: Arc::new(indices),
        }
    }
}

pub enum TextureDefinition {
    FromFile {
        path: PathBuf,
    },
    FromData {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
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
        self.smoothness = 1.0;
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

pub enum Primitive {
    Sphere { centre: Vec3, radius: f32 },
    Mesh(MeshDefinition),
}

pub struct EntityDefinition {
    pub transform: Transform,
    pub primitive: Primitive,
    pub material: MaterialDefinition,
}

pub const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));
impl AssetManager {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            meshes: HashMap::new(),
            textures: HashMap::new(),
        }
    }
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

    pub async fn load_mesh(
        &mut self,
        path: &Path,
    ) -> (Vec<MeshDefinition>, Vec<MaterialDefinition>) {
        let mut mesh_defs: Vec<MeshDefinition> = vec![];
        let mut material_defs: Vec<MaterialDefinition> = vec![];
        let file_path = std::path::Path::new(FILE).join("assets").join(path);

        let obj_text = AssetManager::load_string(&file_path).await.unwrap();
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);
        let (models, _materials) = tobj::load_obj_buf(
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
            // Make this return a whole Arc<Mesh> later, with fully loaded material.
            // Add path and Arc<Mesh> to meshes HashMap
            // Add any loaded textures to HashMap also
            mesh_defs.push(MeshDefinition::FromData {
                vertices: Arc::new(vertices),
                indices: Arc::new(indices),
            });
            material_defs.push(MaterialDefinition::new());
        }

        return (mesh_defs, material_defs);
    }
}
