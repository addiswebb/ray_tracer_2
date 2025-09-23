use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui_wgpu::{
    ScreenDescriptor,
    wgpu::{self, SurfaceError, util::DeviceExt},
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, PhysicalSize},
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::core::{
    egui::EguiRenderer, ray_tracer::RayTracer, renderer::Renderer, scene::Scene, texture::Texture,
};

const WORKGROUP_SIZE: (u32, u32) = (16, 16);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Params {
    width: u32,
    height: u32,
    number_of_bounces: i32,
    rays_per_pixel: i32,
    skybox: i32,
    frames: i32,
    accumulate: i32,
}
pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub scale_factor: f32,
    pub ray_tracer: RayTracer,
    pub egui_renderer: EguiRenderer,
    pub params: Params,
    pub scene: Scene,
    pub selected_scene: i32,
    pub texture: Texture,
    pub prev_scene: i32,
    pub params_buffer: wgpu::Buffer,
    pub mouse_pressed: bool,
    pub renderer: Renderer,
    pub use_mouse: bool,
    pub dt: Duration,
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &Window,
        width: u32,
        height: u32,
    ) -> Self {
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
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: Default::default(),
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

        let params = Params {
            width: surface_config.width,
            height: surface_config.height,
            number_of_bounces: 3,
            rays_per_pixel: 3,
            skybox: 1,
            frames: 0,
            accumulate: 1,
        };
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("parameters buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture = Texture::new(
            &device,
            params.width,
            params.height,
            wgpu::TextureFormat::Rgba32Float,
        );

        let scene = Scene::room(&surface_config);

        let ray_tracer = RayTracer::new(&device, &texture, &params_buffer);

        let mut egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, window);
        let renderer = Renderer::new(
            &device,
            &mut egui_renderer.renderer,
            &texture,
            &surface_config,
            &params_buffer,
        )
        .unwrap();

        Self {
            device,
            queue,
            surface,
            surface_config,
            egui_renderer,
            ray_tracer,
            params,
            scene,
            texture,
            scale_factor: 1.0,
            selected_scene: 0,
            params_buffer,
            prev_scene: 0,
            mouse_pressed: false,
            renderer,
            use_mouse: false,
            dt: Duration::ZERO,
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    window: Option<Arc<Window>>,
    last_render_time: Instant,
}

impl App {
    pub fn new() -> Self {
        let instance = egui_wgpu::wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            instance,
            state: None,
            window: None,
            last_render_time: Instant::now(),
        }
    }
    pub async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 800;
        let initial_height = 600;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));
        // window.set_maximized(true);

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let state = AppState::new(
            &self.instance,
            surface,
            &window,
            initial_width,
            initial_height,
        )
        .await;

        self.window.get_or_insert(window);
        self.state.get_or_insert(state);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let state = self.state.as_mut().unwrap();
            state.resize_surface(width, height);
            state.surface_config.width = width;
            state.surface_config.height = height;
            state.scene.camera.aspect = width as f32 / height as f32;
            state
                .surface
                .configure(&state.device, &state.surface_config);

            state.params.width = width;
            state.params.height = height;
            state.params.frames = -1;

            state.queue.write_buffer(
                &state.params_buffer,
                0,
                bytemuck::cast_slice(&[state.params]),
            );
            state.texture = Texture::new(
                &state.device,
                width,
                height,
                wgpu::TextureFormat::Rgba32Float,
            );
            state
                .ray_tracer
                .update_bind_group(&state.device, &state.texture, &state.params_buffer);
            state.renderer.update_bind_group(
                &state.device,
                &state.texture,
                &state.params_buffer,
                &mut state.egui_renderer.renderer,
            );
        }
    }
    pub fn clear_accumulation(&mut self) {
        let state = self.state.as_mut().unwrap();
        state.params.frames = -1;
        state.queue.write_buffer(
            &state.params_buffer,
            0,
            bytemuck::cast_slice(&[state.params]),
        );
    }

    pub fn update(&mut self, dt: Duration) {
        let state = self.state.as_mut().unwrap();
        state.dt = dt;
        state.scene.camera.update_camera(dt);

        if state.params.accumulate != 0 {
            state.params.frames += 1;
        } else {
            // Reset Accumulation
            state.params.frames = -1;
        }
        state.queue.write_buffer(
            &state.params_buffer,
            0,
            bytemuck::cast_slice(&[state.params]),
        );
        state.ray_tracer.update_buffers(&state.queue, &state.scene);
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: key_state,
                        ..
                    },
                ..
            } => match key {
                KeyCode::Escape => {
                    state.use_mouse = false;
                    let window = self.window.as_mut().unwrap();
                    window.set_cursor_visible(true);
                    window
                        .set_cursor_grab(winit::window::CursorGrabMode::None)
                        .unwrap();
                    true
                }
                _ => state
                    .scene
                    .camera
                    .controller
                    .process_keyboard(*key, *key_state),
            },
            WindowEvent::MouseWheel { delta, .. } => {
                state.scene.camera.controller.process_scroll(delta)
            }
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: button_state,
                ..
            } => {
                state.mouse_pressed = *button_state == winit::event::ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn handle_redraw(&mut self) {
        if self
            .state
            .as_ref()
            .unwrap()
            .scene
            .camera
            .controller
            .is_moving()
        {
            self.clear_accumulation();
        }
        // Skip if window is minimized (maybe unwanted behaviour)
        if let Some(window) = self.window.as_ref() {
            if let Some(min) = window.is_minimized() {
                if min {
                    log::warn!("Skipping, Window minimised");
                    return;
                }
            }
        }

        let state = self.state.as_mut().unwrap();

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [state.surface_config.width, state.surface_config.height],
            pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32
                * state.scale_factor,
        };

        let surface_texture = state.surface.get_current_texture();

        match surface_texture {
            Err(SurfaceError::Outdated) => {
                log::error!("Wgpu Surface Outdated");
                return;
            }
            Err(_) => {
                surface_texture.expect("Failed to aquire next swap chain texture");
                return;
            }
            Ok(_) => {}
        };

        let surface_texture = surface_texture.unwrap();
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let window = self.window.as_mut().unwrap();
        // Ray Tracer Pass
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RayTracer Compute Pass"),
                timestamp_writes: None,
            });
            let xdim = state.surface_config.width + WORKGROUP_SIZE.0 - 1;
            let xgroups = xdim / WORKGROUP_SIZE.0;
            let ydim = state.surface_config.height + WORKGROUP_SIZE.1 - 1;
            let ygroups = ydim / WORKGROUP_SIZE.1;

            compute_pass.set_pipeline(&state.ray_tracer.pipeline);
            compute_pass.set_bind_group(0, &state.ray_tracer.bind_group, &[]);
            compute_pass.dispatch_workgroups(xgroups, ygroups, 1);
        }

        // RENDER
        {
            state.egui_renderer.begin_frame(window);

            let mut skybox = state.params.skybox != 0;
            let mut accumulate = state.params.accumulate != 0;

            egui::SidePanel::left("Debug")
                .resizable(true)
                .width_range(200.0..=400.0)
                .show(state.egui_renderer.context(), |ui| {
                    ui.heading("Debug");
                    ui.separator();
                    ui.label(format!("Frame: {}", state.params.frames));
                    ui.label(format!(
                        "FPS: {:.0}",
                        1.0 / (1.0 * state.dt.as_secs_f64()) // state.renderer.dt.as_micros()
                    ));
                    ui.label(format!("Position: ({})", state.scene.camera.origin));
                    ui.label(format!("Look At: ({})", state.scene.camera.look_at));
                    ui.add(egui::Slider::new(&mut state.scene.camera.fov, 10.0..=90.0).text("Fov"));
                    ui.add(
                        egui::Slider::new(&mut state.params.number_of_bounces, 0..=100)
                            .text("Bounces"),
                    );
                    ui.add(
                        egui::Slider::new(&mut state.params.rays_per_pixel, 0..=100)
                            .text("Rays Per Pixel"),
                    );
                    // ui.label("Skybox");
                    ui.checkbox(&mut skybox, "Skybox");
                    // ui.label("Accumulate");
                    ui.checkbox(&mut accumulate, "Accumulate");

                    ui.add(
                        egui::Slider::new(&mut state.scene.camera.focus_dist, 0.0..=10.0)
                            .text("Focus Distance"),
                    );
                    ui.add(
                        egui::Slider::new(&mut state.scene.camera.aperture, -2.0..=2.0)
                            .text("Aperture"),
                    );
                    ui.add(egui::Slider::new(&mut state.selected_scene, 0..=3).text("Scene ID"));
                    if ui.button("Delete shpere").clicked() {
                        println!("{:#?}", state.scene.spheres.len());
                        state.scene.spheres.pop();
                        println!("{:#?}", state.scene.spheres.len());
                    }
                });

            egui::CentralPanel::default().show(state.egui_renderer.context(), |ui| {
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    if state.renderer.render_ray_traced_image(ui) {
                        state.use_mouse = true;
                        window.set_cursor_visible(!state.use_mouse);
                        window
                            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                            .unwrap();
                    }
                });
            });

            if !(state.selected_scene == state.prev_scene) {
                log::info!("Changing Scene: {}", state.selected_scene);
                match state.selected_scene {
                    0 => {
                        state.scene = Scene::balls(&state.surface_config);
                    }
                    1 => {
                        state.scene = Scene::random_balls(&state.surface_config);
                    }
                    2 => {
                        state.scene = Scene::room(&state.surface_config);
                    }
                    3 => {
                        state.scene = Scene::metal(&state.surface_config);
                    }
                    _ => (),
                }
                state.ray_tracer.update_buffers(&state.queue, &state.scene);
                state.params.frames = -1;
                state.queue.write_buffer(
                    &state.params_buffer,
                    0,
                    bytemuck::cast_slice(&[state.params]),
                );
            }
            state.prev_scene = state.selected_scene;
            state.params.skybox = skybox as i32;
            state.params.accumulate = accumulate as i32;

            state.egui_renderer.end_frame_and_draw(
                &state.device,
                &state.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }

        state.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        pollster::block_on(self.set_window(window));
    }
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if state.use_mouse {
                    state
                        .scene
                        .camera
                        .controller
                        .process_mouse(delta.0, delta.1);
                    self.clear_accumulation();
                }
            }
            DeviceEvent::Button { button, state: x } => {
                if button == 0 {
                    state.mouse_pressed = x == winit::event::ElementState::Pressed;
                }
            }
            DeviceEvent::MouseWheel { delta } => {
                if state.use_mouse {
                    state.scene.camera.controller.process_scroll(&delta);
                }
            }
            _ => {}
        }
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if !self
            .state
            .as_mut()
            .unwrap()
            .egui_renderer
            .handle_input(self.window.as_ref().unwrap(), &event)
        {
            self.handle_input(&event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resized(new_size.width, new_size.height);
            }
            _ => (),
        }
    }
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.last_render_time;
        self.last_render_time = now;
        self.update(dt);
        self.window.as_ref().unwrap().request_redraw();
    }
}
