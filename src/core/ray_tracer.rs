use std::mem;

use egui_wgpu::wgpu::{self, PipelineCompilationOptions};

use crate::core::{
    app::Params,
    bvh::Node,
    mesh::{MeshUniform, Sphere, Vertex},
    scene::{Scene, SceneUniform},
    texture::Texture,
};

const WORKGROUP_SIZE: (u32, u32) = (16, 16);

pub struct RayTracer {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group: wgpu::BindGroup,
    pub sphere_buffer: wgpu::Buffer,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub mesh_buffer: wgpu::Buffer,
    pub scene_buffer: wgpu::Buffer,
    pub bvh_nodes_buffer: wgpu::Buffer,
    pub triangle_indices_buffer: wgpu::Buffer,
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
                    // Nodes
                    wgpu::BindGroupLayoutEntry {
                        binding: 7,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Triangle Indices
                    wgpu::BindGroupLayoutEntry {
                        binding: 8,
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

        let max_vertices = 3000;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<Vertex>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let max_indices = 3000;
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
        let max_nodes = 1000;
        let bvh_nodes_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Nodes Buffer"),
            size: (max_nodes * std::mem::size_of::<Node>() as wgpu::BufferAddress),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let max_triangle_indices = 1000;
        let triangle_indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RayTracer Triangle Indices Buffer"),
            size: (max_triangle_indices * std::mem::size_of::<u32>() as wgpu::BufferAddress),
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
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: bvh_nodes_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: triangle_indices_buffer.as_entire_binding(),
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
            vertex_buffer,
            index_buffer,
            sphere_buffer,
            mesh_buffer,
            scene_buffer,
            bvh_nodes_buffer,
            triangle_indices_buffer,
        }
    }
    pub fn update_buffers(&mut self, queue: &wgpu::Queue, scene: &mut Scene) {
        let (vertices, indices) = scene.vertices_and_indices();
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        queue.write_buffer(&self.sphere_buffer, 0, bytemuck::cast_slice(&scene.spheres));
        queue.write_buffer(&self.mesh_buffer, 0, bytemuck::cast_slice(&scene.meshes()));
        queue.write_buffer(
            &self.bvh_nodes_buffer,
            0,
            bytemuck::cast_slice(&scene.bvh(&scene.meshes())),
        );
        queue.write_buffer(
            &self.triangle_indices_buffer,
            0,
            bytemuck::cast_slice(&scene.bvh.triangle_indices),
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
        compute_pass.dispatch_workgroups(xgroups, ygroups, 1);
    }
}
