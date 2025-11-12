use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

use egui_wgpu::wgpu::{
    self, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, Origin3d,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureView,
};
use image::{ImageBuffer, Rgba32FImage};

pub struct Texture {
    texture: egui_wgpu::wgpu::Texture,
    texture_view: TextureView,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture {
            texture,
            texture_view,
        }
    }

    pub fn save_to_file(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let width = 1920;
        let height = 1080;

        // Calculate aligned bytes per row (wgpu requires 256-byte alignment)
        let bytes_per_pixel = 16; // RGBA
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = (bytes_per_row * height) as wgpu::BufferAddress;

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Final Render Buffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Final Render Encoder"),
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let buffer_slice = buffer.slice(..);

        let map_complete = Arc::new(AtomicBool::new(false));
        let map_error = Arc::new(std::sync::Mutex::new(None));

        let map_complete_clone = Arc::clone(&map_complete);
        let map_error_clone = Arc::clone(&map_error);

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| match result {
            Ok(()) => map_complete_clone.store(true, Ordering::SeqCst),
            Err(e) => *map_error_clone.lock().unwrap() = Some(e),
        });

        while !map_complete.load(Ordering::SeqCst) {
            device.poll(wgpu::MaintainBase::Wait)?;
            if let Some(err) = map_error.lock().unwrap().take() {
                return Err(Box::new(err));
            }
        }

        let data = buffer_slice.get_mapped_range();
        let mut image_data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            let row_start = (y * bytes_per_row) as usize;

            for x in (0..width).rev() {
                let pixel_start = row_start + (x * bytes_per_pixel) as usize;

                let r = f32::from_ne_bytes([
                    data[pixel_start],
                    data[pixel_start + 1],
                    data[pixel_start + 2],
                    data[pixel_start + 3],
                ]);
                let g = f32::from_ne_bytes([
                    data[pixel_start + 4],
                    data[pixel_start + 5],
                    data[pixel_start + 6],
                    data[pixel_start + 7],
                ]);
                let b = f32::from_ne_bytes([
                    data[pixel_start + 8],
                    data[pixel_start + 9],
                    data[pixel_start + 10],
                    data[pixel_start + 11],
                ]);
                let a = f32::from_ne_bytes([
                    data[pixel_start + 12],
                    data[pixel_start + 13],
                    data[pixel_start + 14],
                    data[pixel_start + 15],
                ]);

                let r_byte = (r.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
                let g_byte = (g.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
                let b_byte = (b.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
                let a_byte = (a.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;

                image_data.push(r_byte);
                image_data.push(g_byte);
                image_data.push(b_byte);
                image_data.push(a_byte);
            }
        }

        let mut image = ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, image_data)
            .ok_or("Failed to create image from buffer")
            .unwrap();
        image::imageops::flip_horizontal_in_place(&mut image);
        image::imageops::flip_vertical_in_place(&mut image);
        image.save(path.clone()).unwrap();
        drop(data);
        buffer.unmap();
        println!("Saved Render to {}", path);
        Ok(())
    }

    pub fn binding_resource(&'_ self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::TextureView(&self.texture_view)
    }

    pub fn binding_type(&self, access: wgpu::StorageTextureAccess) -> wgpu::BindingType {
        wgpu::BindingType::StorageTexture {
            access,
            format: wgpu::TextureFormat::Rgba32Float,
            view_dimension: wgpu::TextureViewDimension::D2,
        }
    }
}
