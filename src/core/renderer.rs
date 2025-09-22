use std::{mem, time::Duration};

use egui_wgpu::wgpu::{
    self, Device, PipelineCompilationOptions, SurfaceConfiguration, util::DeviceExt,
};

use crate::core::{app::Params, texture::Texture};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TexVertex {
    _pos: [f32; 4],
    _tex_coord: [f32; 2],
}

pub struct Renderer {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub dt: Duration,
}

impl Renderer {
    fn vertex(pos: [i8; 2], tc: [i8; 2]) -> TexVertex {
        TexVertex {
            _pos: [pos[0] as f32, pos[1] as f32, 1.0, 1.0],
            _tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
    fn create_vertices() -> (Vec<TexVertex>, Vec<u16>) {
        let vertex_data = [
            Renderer::vertex([-1, -1], [1, 0]),
            Renderer::vertex([1, -1], [0, 0]),
            Renderer::vertex([1, 1], [0, 1]),
            Renderer::vertex([-1, 1], [1, 1]),
        ];
        let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];
        (vertex_data.to_vec(), index_data.to_vec())
    }
    pub fn new(
        device: &Device,
        texture: &Texture,
        surface_config: &SurfaceConfiguration,
        params_buffer: &wgpu::Buffer,
    ) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Renderer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(mem::size_of::<Params>() as _),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Renderer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture.binding_resource(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Renderer Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Renderer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/renderer.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{:?}", shader)),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vert"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TexVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x2],
                }],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("frag"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let (vertices, indices) = Renderer::create_vertices();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Renderer Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let dt = Duration::new(0, 0);
        Self {
            pipeline,
            bind_group,
            bind_group_layout,
            vertex_buffer,
            index_buffer,
            dt,
        }
    }
    pub fn update_bind_group(
        &mut self,
        device: &wgpu::Device,
        params_buffer: &wgpu::Buffer,
        texture: &Texture,
    ) {
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Renderer Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: texture.binding_resource(),
                },
            ],
        });
    }
}
