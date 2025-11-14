use std::{fs::File, io::Read, mem, num::NonZeroU32, sync::Arc};

use egui_wgpu::wgpu::{
    self, Extent3d, PipelineCompilationOptions, TextureView, wgt::TextureViewDescriptor,
};

use crate::core::{
    app::Params,
    asset::FILE,
    bvh::{BVH, Node, PackedTriangle},
    mesh::{MeshUniform, Sphere},
    scene::{Scene, SceneUniform},
};

const WORKGROUP_SIZE: (u32, u32) = (8, 8);
const MAX_MESHES: u64 = 400;
const MAX_SPHERS: u64 = 500;
const MAX_TRIANGLES: u64 = 275000;
pub const MAX_TEXTURES: u64 = 2;

const TEXTURE_SIZE: (u32, u32) = (1024, 1024);

pub struct RayTracer {
    pub device: Arc<wgpu::Device>,
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
    pub textures_bind_group: wgpu::BindGroup,
    pub sphere_buffer: wgpu::Buffer,
    pub triangle_buffer: wgpu::Buffer,
    pub mesh_buffer: wgpu::Buffer,
    pub scene_buffer: wgpu::Buffer,
    pub bvh_nodes_buffer: wgpu::Buffer,
    pub textures: Vec<wgpu::Texture>,
    pub n_textures: u32,
}

impl RayTracer {
    pub fn new(
        device: Arc<wgpu::Device>,
        texture_view: &TextureView,
        params_buffer: &wgpu::Buffer,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RayTracer Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/ray_tracer.wgsl").into()),
        });
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("RayTracer Bind Group Layout"),
                entries: &[
                    // Params
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(mem::size_of::<Params>() as _),
                        },
                        count: None,
                    },
                    // Scene
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                mem::size_of::<SceneUniform>() as _
                            ),
                        },
                        count: None,
                    },
                    //Texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba32Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    //Spheres
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Triangle Buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Meshes
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Nodes
                    wgpu::BindGroupLayoutEntry {
                        binding: 6,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let textures_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("RayTracer Textures Bind Group Layout"),
                entries: &[
                    // Textures
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: NonZeroU32::new(MAX_TEXTURES as u32),
                    },
                    // Sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raytracer Scene Buffer"),
            size: std::mem::size_of::<SceneUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let triangle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Triangle Buffer"),
            size: (MAX_TRIANGLES * std::mem::size_of::<PackedTriangle>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let sphere_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Sphere Buffer"),
            size: (MAX_SPHERS * std::mem::size_of::<Sphere>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let mesh_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Mesh Buffer"),
            size: (MAX_MESHES * std::mem::size_of::<MeshUniform>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let bvh_nodes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Nodes Buffer"),
            size: (BVH::MAX_NODES as u64 * std::mem::size_of::<Node>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let t1 = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("t1"),
            size: Extent3d {
                width: TEXTURE_SIZE.0,
                height: TEXTURE_SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let t2 = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("t2"),
            size: Extent3d {
                width: TEXTURE_SIZE.0,
                height: TEXTURE_SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let t1_view = t1.create_view(&TextureViewDescriptor::default());
        let t2_view = t2.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RayTracer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scene_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: triangle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mesh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: bvh_nodes_buffer.as_entire_binding(),
                },
            ],
        });
        let textures_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RayTracer Textures Bind Group"),
            layout: &textures_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureViewArray(&[&t1_view, &t2_view]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RayTracer Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout, &textures_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RayTracer Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None,
        });
        Self {
            device,
            pipeline,
            bind_group,
            textures_bind_group,
            triangle_buffer,
            sphere_buffer,
            mesh_buffer,
            scene_buffer,
            bvh_nodes_buffer,
            textures: vec![t1, t2],
            n_textures: 0,
        }
    }
    pub fn update_buffers(&mut self, queue: &wgpu::Queue, scene: &mut Scene) {
        queue.write_buffer(
            &self.triangle_buffer,
            0,
            bytemuck::cast_slice(&scene.bvh_data.triangles),
        );
        queue.write_buffer(&self.sphere_buffer, 0, bytemuck::cast_slice(&scene.spheres));
        queue.write_buffer(
            &self.mesh_buffer,
            0,
            bytemuck::cast_slice(&scene.bvh_data.mesh_uniforms),
        );
        queue.write_buffer(
            &self.bvh_nodes_buffer,
            0,
            bytemuck::cast_slice(&scene.bvh()),
        );
        queue.write_buffer(
            &self.scene_buffer,
            0,
            bytemuck::cast_slice(&[scene.to_uniform()]),
        );
    }
    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, width: u32, height: u32) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("RayTracer Compute Pass"),
            timestamp_writes: None,
        });
        let xdim = width + WORKGROUP_SIZE.0 - 1;
        let xgroups = xdim / WORKGROUP_SIZE.0;
        let ydim = height + WORKGROUP_SIZE.1 - 1;
        let ygroups = ydim / WORKGROUP_SIZE.1;

        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        compute_pass.set_bind_group(1, &self.textures_bind_group, &[]);
        compute_pass.dispatch_workgroups(xgroups, ygroups, 1);
    }

    pub fn load_texture(&mut self, queue: &wgpu::Queue, path: String) -> u32 {
        let mut buffer = vec![];
        let file_path = std::path::Path::new(FILE).join("assets").join(path);
        File::open(file_path)
            .unwrap()
            .read_to_end(&mut buffer)
            .unwrap();
        let mut image = image::load_from_memory(&buffer).unwrap();
        image::imageops::flip_vertical_in_place(&mut image);
        let data = image.to_rgba8();

        queue.write_texture(
            self.textures[self.n_textures as usize].as_image_copy(),
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEXTURE_SIZE.0 * 4),
                rows_per_image: Some(TEXTURE_SIZE.1),
            },
            Extent3d {
                width: TEXTURE_SIZE.0,
                height: TEXTURE_SIZE.1,
                depth_or_array_layers: 1,
            },
        );
        self.n_textures += 1;
        self.n_textures
    }
}
