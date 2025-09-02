use egui_wgpu::wgpu;

pub struct Texture {
    texture_view: wgpu::TextureView,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        width:u32,
        height:u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture {
            texture_view,
        }
    }

    pub fn binding_resource(&'_ self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::TextureView(&self.texture_view)
    }

    pub fn binding_type(
        &self,
        access: wgpu::StorageTextureAccess
    ) -> wgpu::BindingType {
        wgpu::BindingType::StorageTexture {
            access,
            format: wgpu::TextureFormat::Rgba32Float,
            view_dimension: wgpu::TextureViewDimension::D2,
        }
    }
}