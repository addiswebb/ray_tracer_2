use std::mem;

use egui_wgpu::wgpu::{self, PipelineCompilationOptions};

use crate::core::{
    app::Params,
    camera::CameraUniform,
    mesh::{MeshUniform, Sphere, Vertex},
    scene::{Scene, SceneUniform},
    texture::Texture,
};

pub struct RayTracer {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    pub sphere_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub mesh_buffer: wgpu::Buffer,
    pub scene_buffer: wgpu::Buffer,
}

impl RayTracer {
    pub fn new(device: &wgpu::Device, texture: &Texture, params_buffer: &wgpu::Buffer) -> Self {
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
                        ty: texture.binding_type(wgpu::StorageTextureAccess::ReadWrite),
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
                    // Vertex Buffer
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
                    // Index Buffer
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
                    // Meshes
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
        let scene_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raytracer Scene Buffer"),
            size: std::mem::size_of::<SceneUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let max_vertices = 1000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let max_indices = 1000;
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Index Buffer"),
            size: (max_indices * std::mem::size_of::<u32>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let max_spheres = 500;
        let sphere_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Sphere Buffer"),
            size: (max_spheres * std::mem::size_of::<Sphere>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let max_meshes = 10;
        let mesh_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Mesh Buffer"),
            size: (max_meshes * std::mem::size_of::<MeshUniform>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
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
                    resource: texture.binding_resource(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: mesh_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("RayTracer Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
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
            pipeline,
            bind_group,
            bind_group_layout,
            vertex_buffer,
            index_buffer,
            sphere_buffer,
            mesh_buffer,
            scene_buffer,
        }
    }
    pub fn update_buffers(&mut self, queue: &wgpu::Queue, scene: &Scene) {
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&scene.vertices),
        );
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&scene.indices));
        queue.write_buffer(&self.sphere_buffer, 0, bytemuck::cast_slice(&scene.spheres));
        queue.write_buffer(&self.mesh_buffer, 0, bytemuck::cast_slice(&scene.meshes));
        queue.write_buffer(
            &self.scene_buffer,
            0,
            bytemuck::cast_slice(&[scene.to_uniform()]),
        );
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        texture: &Texture,
        params_buffer: &wgpu::Buffer,
    ) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RayTracer Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.scene_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: texture.binding_resource(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.sphere_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: self.mesh_buffer.as_entire_binding(),
                },
            ],
        });
    }
}
