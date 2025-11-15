use std::{collections::HashMap, fs::File, io::Read, sync::Arc};

use egui_wgpu::wgpu::{self};
use glam::Vec3;
use image::{ImageBuffer, Rgba};

use crate::core::{
    mesh::{Material, Mesh, MeshInstance, Transform, Vertex},
    ray_tracer::MAX_TEXTURES,
};

pub struct AssetManager {
    loaded_meshes: HashMap<String, Arc<Mesh>>,
    loaded_textures: HashMap<String, TextureRef>,
    pub cpu_textures: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>,
}

#[derive(Clone, Copy, Default)]
pub struct TextureRef {
    pub width: u32,
    pub height: u32,
    pub index: u32,
}

pub enum MeshDefinition {
    FromFile {
        path: String,
        use_loaded_materials: bool,
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
        path: String,
    },
    #[allow(unused)]
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
    pub fn new() -> Self {
        Self {
            loaded_meshes: HashMap::new(),
            loaded_textures: HashMap::new(),
            cpu_textures: vec![],
        }
    }
    pub fn load_texture(&mut self, path: &String) -> TextureRef {
        println!("Loading Texture: {}", path);
        if self.loaded_textures.len() == MAX_TEXTURES as usize {
            panic!("Cannot load more than {} textures", MAX_TEXTURES);
        }
        // Check if we have already loaded this texture, then return its text_index
        if let Some(texture_ref) = self.loaded_textures.get(path) {
            return texture_ref.clone();
        }
        let mut buffer = vec![];
        let file_path = std::path::Path::new(FILE).join("assets").join(path.clone());
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut buffer)
            .unwrap();
        let image = image::imageops::flip_horizontal(&image::load_from_memory(&buffer).unwrap());
        println!("Image dimensions {} {}", image.width(), image.height());

        let index = self.loaded_textures.len();

        let texture_ref = TextureRef {
            index: index as u32,
            width: image.width(),
            height: image.height(),
        };
        self.loaded_textures
            .insert(path.clone(), texture_ref.clone());
        self.cpu_textures.push(image);
        texture_ref
    }
    pub fn load_model_with_material(
        &mut self,
        path: &String,
        transform: Transform,
        load_materials: bool,
        material: Material,
    ) -> Vec<MeshInstance> {
        let mut meshes = self.load_model(path, transform, load_materials);
        if !load_materials {
            for mesh in &mut meshes {
                mesh.material = material;
            }
        }
        meshes
    }

    pub fn load_model(
        &mut self,
        path: &String,
        transform: Transform,
        load_materials: bool,
    ) -> Vec<MeshInstance> {
        println!("Loading model: {}", path);
        let mut meshes: Vec<MeshInstance> = vec![];
        let mut material_defs: Vec<Material> = vec![];
        let file_path = std::path::Path::new(FILE).join("assets").join(path);

        let (models, materials) = tobj::load_obj(
            file_path,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: false,
                ..Default::default()
            },
        )
        .expect("Failed to load OBJ File");
        if load_materials && let Ok(materials) = materials {
            for m in materials {
                let color = m.diffuse.unwrap_or([0.7; 3]);
                let spec = m.specular.unwrap_or([1.0; 3]);
                let mut flag = match m.illumination_model.unwrap_or(0) {
                    4 => MaterialFlag::GLASS,
                    6 => MaterialFlag::GLASS,
                    7 => MaterialFlag::GLASS,
                    9 => MaterialFlag::GLASS,
                    _ => MaterialFlag::NORMAL,
                };
                let texture_ref = if let Some(path) = &m.diffuse_texture {
                    // Handle if there is a texture to be loaded
                    flag = MaterialFlag::TEXTURE;
                    self.load_texture(path)
                } else {
                    TextureRef::default()
                };
                material_defs.push(Material {
                    color: [color[0], color[1], color[2], 1.0],
                    emission_color: [color[0], color[1], color[2], 1.0],
                    specular_color: [spec[0], spec[1], spec[2], 1.0],
                    absorption: [0.0; 4],
                    absorption_stength: 0.0,
                    emission_strength: 0.0,
                    smoothness: 0.0,
                    specular: 0.05,
                    // specular: m.shininess.unwrap_or(0.5).min(1.0),
                    ior: m.optical_density.unwrap_or(1.0),
                    flag: flag as i32,
                    texture_index: texture_ref.index,
                    width: texture_ref.width,
                    height: texture_ref.height,
                    _p1: [0.0; 3],
                });
            }
        }

        for (i, m) in models.into_iter().enumerate() {
            let mut mesh = Mesh {
                vertices: Arc::new(vec![]),
                indices: Arc::new(vec![]),
            };
            let mut vertices = vec![];

            if let Some(mesh_ref) = self.loaded_meshes.get(&format!("{}_{}", m.name, i)) {
                println!("Used cached vertices");
                mesh.vertices = mesh_ref.vertices.clone();
                mesh.indices = mesh_ref.indices.clone();
            } else {
                println!("Read new vertices");
                for (j, &vi) in m.mesh.indices.iter().enumerate() {
                    let pi = vi as usize;
                    let pos = Vec3::new(
                        m.mesh.positions[3 * pi],
                        m.mesh.positions[3 * pi + 1],
                        m.mesh.positions[3 * pi + 2],
                    );

                    let normal = if !m.mesh.normals.is_empty() && !m.mesh.normal_indices.is_empty()
                    {
                        let ni = m.mesh.normal_indices[j] as usize;
                        Vec3::new(
                            m.mesh.normals[3 * ni],
                            m.mesh.normals[3 * ni + 1],
                            m.mesh.normals[3 * ni + 2],
                        )
                    } else if !m.mesh.normals.is_empty() {
                        // fallback: just use the position index (may be wrong for some OBJ files)
                        let ni = pi;
                        Vec3::new(
                            m.mesh.normals[3 * ni],
                            m.mesh.normals[3 * ni + 1],
                            m.mesh.normals[3 * ni + 2],
                        )
                    } else {
                        Vec3::ZERO
                    };

                    let uv = if !m.mesh.texcoords.is_empty() && !m.mesh.texcoord_indices.is_empty()
                    {
                        let ti = m.mesh.texcoord_indices[j] as usize; // j from enumerate()
                        [m.mesh.texcoords[2 * ti], m.mesh.texcoords[2 * ti + 1]]
                    } else {
                        [0.0, 0.0] // fallback for missing UVs
                    };

                    vertices.push(Vertex::with_uv(pos, normal, uv));
                }
                mesh.indices = Arc::new((0..vertices.len() as u32).collect());
                mesh.vertices = Arc::new(vertices);
            }
            let material = if load_materials && let Some(id) = m.mesh.material_id {
                material_defs[id]
            } else {
                Material::new()
            };
            let mesh = Arc::new(mesh);
            self.loaded_meshes
                .insert(format!("{}_{}", m.name, i), mesh.clone());
            meshes.push(MeshInstance {
                label: Some(m.name),
                transform,
                mesh: mesh.clone(),
                material,
            });
        }

        return meshes;
    }
}
