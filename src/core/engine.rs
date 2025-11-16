use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui_wgpu::{
    ScreenDescriptor,
    wgpu::{
        self, CommandEncoder, Limits, SurfaceError, SurfaceTexture, TextureView, util::DeviceExt,
    },
};
use winit::window::Window;

use crate::core::{
    app::Params,
    asset::AssetManager,
    egui::EguiRenderer,
    ray_tracer::{MAX_TEXTURES, RayTracer},
    renderer::Renderer,
    scene::{Scene, SceneManager, SceneName},
};

pub struct TmpResources {
    pub use_mouse: bool,
    pub mouse_pressed: bool,
    pub fullscreen: bool,
    pub low_res: bool,
}

impl Default for TmpResources {
    fn default() -> Self {
        Self {
            use_mouse: false,
            mouse_pressed: false,
            fullscreen: false,
            low_res: false,
        }
    }
}

pub struct GraphicsResources {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub params_buffer: wgpu::Buffer,
    pub scale_factor: f32,
}
impl GraphicsResources {
    pub fn create_screen_descriptor(&mut self, window: Arc<Window>) -> ScreenDescriptor {
        ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: window.scale_factor() as f32 * self.scale_factor,
        }
    }
    pub fn get_surface_view_and_texture(&mut self) -> (SurfaceTexture, TextureView) {
        let surface_texture = self.surface.get_current_texture();

        match surface_texture {
            Err(SurfaceError::Outdated) => {
                panic!("Wgpu Surface Outdated");
            }
            Err(_) => {
                surface_texture.expect("Failed to aquire next swap chain texture");
                panic!("Failed to aquire next swap chain texture");
            }
            Ok(_) => {}
        };

        let surface_texture = surface_texture.unwrap();
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        (surface_texture, surface_view)
    }
    pub fn create_command_encoder(&mut self) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }
    pub async fn create_graphics_resources(window: Arc<Window>, width: u32, height: u32) -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                    | wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
                required_limits: Limits {
                    max_binding_array_elements_per_shader_stage: MAX_TEXTURES as u32,
                    ..Default::default()
                },
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to find device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("Failed to select proper surface texture format");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Immediate,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Param buffer"),
            contents: bytemuck::bytes_of(&Params::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        Self {
            device,
            queue,
            surface_config,
            surface,
            texture,
            texture_view,
            params_buffer,
            scale_factor: 1.0,
        }
    }
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}
pub struct FrameTiming {
    pub last_render_time: Instant,
    pub dt: Duration,
    pub average_frame_time: Duration,
}
impl FrameTiming {
    pub fn new() -> Self {
        Self {
            last_render_time: Instant::now(),
            dt: Duration::ZERO,
            average_frame_time: Duration::ZERO,
        }
    }
    pub fn update(&mut self, dt: Duration) {
        self.dt = dt;
        self.average_frame_time += dt;
        self.average_frame_time /= 2;
    }
    pub fn reset(&mut self) {
        self.average_frame_time = Duration::ZERO;
    }
}
pub const RENDER_SIZE: (u32, u32) = (1920, 1080);

pub struct Engine {
    pub resources: GraphicsResources,
    pub ray_tracer: RayTracer,
    pub renderer: Renderer,
    pub egui: EguiRenderer,
    pub timing: FrameTiming,
    pub scene_manager: SceneManager,
    pub params: Params,
    pub tmp: TmpResources,
}

impl Engine {
    pub async fn new(window: Arc<Window>, width: u32, height: u32) -> Self {
        let resources =
            GraphicsResources::create_graphics_resources(window.clone(), width, height).await;
        let mut ray_tracer = RayTracer::new(resources.device.clone(), resources.queue.clone());
        ray_tracer.create_gpu_resources(&resources.texture_view, &resources.params_buffer);

        let mut egui_renderer = EguiRenderer::new(
            resources.device.clone(),
            resources.surface_config.format,
            None,
            1,
            window.clone(),
        );

        let renderer = Renderer::new(
            resources.device.clone(),
            &mut egui_renderer.renderer,
            &resources.texture_view,
            &resources.surface_config,
            &resources.params_buffer,
        )
        .unwrap();

        let asset_manager = AssetManager::new();
        let mut scene_manager = SceneManager::new(asset_manager);
        scene_manager.request_scene(SceneName::Room2);

        let timing = FrameTiming::new();
        let params = Params {
            width,
            height,
            number_of_bounces: 5,
            rays_per_pixel: 1,
            ..Default::default()
        };
        let tmp = TmpResources::default();

        Self {
            resources,
            ray_tracer,
            renderer,
            egui: egui_renderer,
            timing,
            scene_manager,
            params,
            tmp,
        }
    }
}
