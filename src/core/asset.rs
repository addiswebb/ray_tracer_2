use std::{
    fs::File,
    io::Read,
    sync::{Arc, atomic::AtomicU32},
};

use dashmap::DashMap;
use glam::Vec3;
use image::{ImageBuffer, RgbaImage};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use crate::rendering::ray_tracer::MAX_TEXTURES;
use crate::scene::components::{
    geometry::{
        mesh::{MeshData, MeshInstance},
        vertex::Vertex,
    },
    material::{MaterialFlag, MaterialUniform},
    transform::Transform,
};

pub struct AssetManager {
    loaded_meshes: Arc<DashMap<String, Arc<MeshData>>>,
    pub loaded_textures: Arc<DashMap<String, i32>>,
    pub cpu_textures: DashMap<String, Arc<RgbaImage>>,
    next_texture_index: AtomicU32,
}
impl AssetManager {
    pub fn create_texture_array(&self) -> Vec<Arc<RgbaImage>> {
        let mut texture_array: Vec<Arc<RgbaImage>> =
            vec![Arc::new(ImageBuffer::new(1, 1)); MAX_TEXTURES as usize];

        for entry in self.cpu_textures.iter() {
            let key = entry.key();
            if let Some(index) = self.loaded_textures.get(key) {
                let index = index.clone() as usize;
                if index < MAX_TEXTURES as usize {
                    texture_array[index] = entry.value().clone();
                }
            }
        }

        texture_array
    }
}

pub const FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"));
impl AssetManager {
    pub fn new() -> Self {
        Self {
            loaded_meshes: Arc::new(DashMap::new()),
            loaded_textures: Arc::new(DashMap::new()),
            cpu_textures: DashMap::new(),
            next_texture_index: AtomicU32::new(0),
        }
    }
    pub fn load_texture(&self, path: &String) -> i32 {
        if self.loaded_textures.len() == MAX_TEXTURES as usize {
            log::warn!("Cannot load more than {} textures", MAX_TEXTURES);
            return -1;
        }
        // Check if we have already loaded this texture,
        // we can find the texture_ref and arc-texture later using its path
        if let Some(loaded_ref) = self.loaded_textures.get(path) {
            return loaded_ref.clone();
        }
        let mut buffer = vec![];
        let file_path = std::path::Path::new(FILE).join("assets").join(path.clone());
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut buffer)
            .unwrap();

        let image = image::imageops::flip_horizontal(&image::load_from_memory(&buffer).unwrap());
        let index = self
            .next_texture_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst) as i32;

        self.loaded_textures.insert(path.clone(), index.clone());
        self.cpu_textures.insert(path.clone(), Arc::new(image));
        index
    }
    pub fn load_model_with_material(
        &self,
        path: &String,
        transform: Transform,
        use_mtl: bool,
        material: MaterialUniform,
    ) -> Vec<MeshInstance> {
        let mut meshes = self.load_model(path, transform, use_mtl);
        if !use_mtl {
            meshes.iter_mut().for_each(|mesh| {
                mesh.material = material;
            });
        }
        meshes
    }

    pub fn load_model(
        &self,
        path: &String,
        transform: Transform,
        load_materials: bool,
    ) -> Vec<MeshInstance> {
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

        let material_map: DashMap<usize, MaterialUniform> = DashMap::new();

        // Must get index before textures are added,
        // This is index of where the next texture will be stored on gpu texture array
        if load_materials && let Ok(materials) = materials {
            let texture_refs: DashMap<String, i32> = DashMap::new();
            materials.par_iter().for_each(|m| {
                if let Some(diffuse_path) = &m.diffuse_texture {
                    if !texture_refs.contains_key(diffuse_path) {
                        let texture_ref = self.load_texture(diffuse_path);
                        texture_refs.insert(diffuse_path.clone(), texture_ref.clone());
                    }
                }

                if let Some(normal_path) = m.unknown_param.get("map_Disp") {
                    if !texture_refs.contains_key(normal_path) {
                        let texture_ref = self.load_texture(normal_path);
                        texture_refs.insert(normal_path.clone(), texture_ref.clone());
                    }
                }
            });
            materials.par_iter().enumerate().for_each(|(i, m)| {
                let color = m.diffuse.unwrap_or([0.7; 3]);
                let spec = m.specular.unwrap_or([1.0; 3]);
                let mut flag = match m.illumination_model.unwrap_or(0) {
                    4 => MaterialFlag::GLASS,
                    6 => MaterialFlag::GLASS,
                    7 => MaterialFlag::GLASS,
                    9 => MaterialFlag::GLASS,
                    _ => MaterialFlag::DEFAULT,
                };
                let diffuse_index = if let Some(diffuse_path) = &m.diffuse_texture {
                    flag = MaterialFlag::TEXTURE;
                    texture_refs.get(diffuse_path).unwrap().value().clone()
                } else {
                    -1
                };
                let normal_index = if let Some(normal_path) = m.unknown_param.get("map_Disp") {
                    flag = MaterialFlag::TEXTURE;
                    texture_refs.get(normal_path).unwrap().value().clone()
                } else {
                    -1
                };
                let mat = MaterialUniform {
                    color: [color[0], color[1], color[2], 1.0],
                    emission_color: [color[0], color[1], color[2], 1.0],
                    specular_color: [spec[0], spec[1], spec[2], 1.0],
                    emission_strength: 0.0,
                    smoothness: 1.0 - (m.shininess.unwrap_or(0.0) / 1000.0),
                    specular: (spec[0] + spec[1] + spec[2]) / 3.0,
                    ior: m.optical_density.unwrap_or(1.0),
                    flag: flag as i32,
                    diffuse_index,
                    normal_index,
                    ..Default::default()
                };
                // Index in m.materials, path if uses texture, the loaded material
                material_map.insert(i, mat);
            });
        }

        let meshes: Vec<MeshInstance> = models
            .into_par_iter()
            .take(30)
            .map(|m| {
                let mut mesh_data = MeshData {
                    vertices: Arc::new(vec![]),
                    indices: Arc::new(vec![]),
                };

                if let Some(mesh_ref) = self.loaded_meshes.get(&format!("{}", m.name)) {
                    mesh_data.vertices = mesh_ref.vertices.clone();
                    mesh_data.indices = mesh_ref.indices.clone();
                } else {
                    mesh_data.vertices = Arc::new(
                        m.mesh
                            .indices
                            .par_iter()
                            .enumerate()
                            .map(|(j, &vi)| {
                                let pi = vi as usize;
                                let pos = Vec3::new(
                                    m.mesh.positions[3 * pi],
                                    m.mesh.positions[3 * pi + 1],
                                    m.mesh.positions[3 * pi + 2],
                                );

                                let normal = if !m.mesh.normals.is_empty()
                                    && !m.mesh.normal_indices.is_empty()
                                {
                                    let ni = m.mesh.normal_indices[j] as usize;
                                    Vec3::new(
                                        m.mesh.normals[3 * ni],
                                        m.mesh.normals[3 * ni + 1],
                                        m.mesh.normals[3 * ni + 2],
                                    )
                                } else if !m.mesh.normals.is_empty() {
                                    // if no normals are found, use the position index (may be wrong for some OBJ files)
                                    let ni = pi;
                                    Vec3::new(
                                        m.mesh.normals[3 * ni],
                                        m.mesh.normals[3 * ni + 1],
                                        m.mesh.normals[3 * ni + 2],
                                    )
                                } else {
                                    Vec3::ZERO
                                };

                                let uv = if !m.mesh.texcoords.is_empty()
                                    && !m.mesh.texcoord_indices.is_empty()
                                {
                                    let ti = m.mesh.texcoord_indices[j] as usize;
                                    [m.mesh.texcoords[2 * ti], m.mesh.texcoords[2 * ti + 1]]
                                } else {
                                    [0.0, 0.0] // no texcoords given
                                };

                                Vertex::with_uv(pos, normal, uv)
                            })
                            .collect(),
                    );
                    mesh_data.indices = Arc::new((0..mesh_data.vertices.len() as u32).collect());
                }
                let material = if load_materials && let Some(id) = m.mesh.material_id {
                    material_map.get(&id).unwrap().clone()
                } else {
                    MaterialUniform::default()
                };
                let mesh_data = Arc::new(mesh_data);
                self.loaded_meshes
                    .insert(format!("{}", m.name), mesh_data.clone());
                MeshInstance {
                    label: Some(m.name),
                    transform,
                    data: mesh_data.clone(),
                    material,
                }
            })
            .collect();

        return meshes;
    }
}
