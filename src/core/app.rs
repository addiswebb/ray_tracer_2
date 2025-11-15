use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use egui_wgpu::wgpu::{
    self, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, Origin3d,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect,
};
use image::ImageBuffer;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window},
};

use crate::core::{
    egui::UiContext,
    engine::{Engine, FrameTiming, RENDER_SIZE},
    ray_tracer::DebugMode,
    scene::Scene,
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq)]
pub struct Params {
    pub width: u32,
    pub height: u32,
    pub number_of_bounces: i32,
    pub rays_per_pixel: i32,
    pub skybox: i32,
    pub frames: i32,
    pub accumulate: i32,
    pub debug_flag: i32,
    pub debug_scale: i32,
    pub _p1: [f32; 3],
}

impl Params {
    pub fn update(&mut self, is_moving: bool) -> bool {
        if is_moving {
            self.reset_frame();
            return true;
        }
        if self.accumulate == 1 {
            self.frames += 1;
            return false;
        }
        self.reset_frame();
        true
    }
    pub fn reset_frame(&mut self) {
        self.frames = -1;
    }
    pub fn for_buffer(&self, is_moving: bool) -> Self {
        let mut params = self.clone();
        params.number_of_bounces = if is_moving { 1 } else { self.number_of_bounces };
        params.rays_per_pixel = if is_moving { 1 } else { self.rays_per_pixel };
        params.width = if is_moving { 1280 } else { self.width };
        params.height = if is_moving { 1080 } else { self.height };
        params
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            number_of_bounces: 5,
            rays_per_pixel: 1,
            skybox: 0,
            frames: 0,
            accumulate: 1,
            debug_flag: 0,
            debug_scale: 0,
            _p1: [0.0; 3],
        }
    }
}
pub const DEBUG_MODES: u32 = DebugMode::NodesAndTriangles as u32 + 1;

pub struct App {
    engine: Option<Engine>,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            engine: None,
        }
    }
    pub async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 800;
        let initial_height = 600;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let engine = Engine::new(window.clone(), RENDER_SIZE.0, RENDER_SIZE.1).await;

        self.window.get_or_insert(window);
        self.engine.get_or_insert(engine);
    }
    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.engine
                .as_mut()
                .unwrap()
                .resources
                .resize_surface(width, height);
        }
    }

    pub fn update(&mut self, dt: Duration) {
        let engine = self.engine.as_mut().unwrap();
        let timing = &mut engine.timing;
        timing.update(dt);

        let camera_moved = engine.scene_manager.scene.camera.update_camera(dt);
        let reset_frame = engine.params.update(camera_moved);
        if camera_moved || reset_frame {
            timing.reset();
        }

        if engine.scene_manager.selected_scene != engine.scene_manager.prev_scene {
            log::info!("Changing Scene: {}", engine.scene_manager.selected_scene);
            // Todo: Make this work with new thing
            match engine.scene_manager.selected_scene {
                0 => {
                    engine.scene_manager.load_scene(
                        &Scene::balls(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                1 => {
                    engine.scene_manager.load_scene(
                        &Scene::room(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                2 => {
                    engine.scene_manager.load_scene(
                        &Scene::metal(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                3 => {
                    engine.scene_manager.load_scene(
                        &Scene::random_balls(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                4 => {
                    engine.scene_manager.load_scene(
                        &Scene::room_2(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                5 => {
                    engine.scene_manager.load_scene(
                        &Scene::sponza(),
                        &mut engine.assets,
                        &mut engine.ray_tracer,
                    );
                }
                _ => (),
            }
            engine.scene_manager.prev_scene = engine.scene_manager.selected_scene;
        }
        engine.resources.queue.write_buffer(
            &engine.resources.params_buffer,
            0,
            bytemuck::cast_slice(&[engine.params.for_buffer(camera_moved || engine.tmp.low_res)]),
        );
        // Todo: investigate performance effects of this
        if engine.tmp.update_buffers {
            engine
                .ray_tracer
                .update_buffers(&engine.resources.queue, &mut engine.scene_manager.scene);
        }
    }

    fn handle_input(&mut self, event: &WindowEvent) -> bool {
        let engine = self.engine.as_mut().unwrap();
        if !engine.tmp.use_mouse {
            return false;
        }
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
                    engine.tmp.use_mouse = false;
                    let window = self.window.as_mut().unwrap();
                    window.set_cursor_visible(true);
                    window
                        .set_cursor_grab(winit::window::CursorGrabMode::None)
                        .unwrap();
                    true
                }
                KeyCode::KeyQ => {
                    if key_state.is_pressed() {
                        engine.scene_manager.selected_scene += 1;
                        if engine.scene_manager.selected_scene > 5 {
                            engine.scene_manager.selected_scene = 0;
                        }
                        engine.params.reset_frame();
                        engine.timing.reset();
                    }
                    true
                }
                KeyCode::KeyE => {
                    if key_state.is_pressed() {
                        engine.params.debug_flag += 1;
                        if engine.params.debug_flag > DEBUG_MODES as i32 {
                            engine.params.debug_flag = 0;
                        }
                        engine.params.reset_frame();
                        engine.timing.reset();
                    }
                    true
                }
                KeyCode::KeyP => {
                    if key_state.is_pressed() {
                        println!("Saving Render to file");
                        let _ = App::save_render_to_file(
                            &engine.resources.texture,
                            &engine.resources.device,
                            &engine.resources.queue,
                            "C:/users/addis/downloads/test.png".to_string(),
                        )
                        .unwrap();
                    }
                    true
                }
                KeyCode::KeyF => {
                    if key_state.is_pressed() {
                        let window = self.window.as_mut().unwrap();
                        engine.tmp.fullscreen = match engine.tmp.fullscreen {
                            true => {
                                window.set_fullscreen(None);
                                false
                            }
                            false => {
                                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                                true
                            }
                        };
                    }
                    true
                }
                KeyCode::KeyR => {
                    if key_state.is_pressed() {
                        engine.tmp.low_res = !engine.tmp.low_res;
                        engine.params.reset_frame();
                        engine.timing.reset();
                    }
                    true
                }
                KeyCode::Digit1 => {
                    if key_state.is_pressed() {
                        engine.params.skybox = if engine.params.skybox != 0 { 0 } else { 1 };
                        engine.params.reset_frame();
                        engine.timing.reset();
                    }
                    true
                }
                KeyCode::Digit2 => {
                    if key_state.is_pressed() {
                        engine.params.accumulate =
                            if engine.params.accumulate != 0 { 0 } else { 1 };
                    }
                    true
                }
                _ => engine
                    .scene_manager
                    .scene
                    .camera
                    .controller
                    .process_keyboard(*key, *key_state),
            },
            WindowEvent::MouseInput {
                button: winit::event::MouseButton::Left,
                state: button_state,
                ..
            } => {
                engine.tmp.mouse_pressed = *button_state == winit::event::ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn handle_redraw(&mut self) {
        let engine = self.engine.as_mut().unwrap();

        // Skip if window is minimized (maybe unwanted behaviour)
        if let Some(window) = self.window.as_ref() {
            if let Some(min) = window.is_minimized() {
                if min {
                    log::warn!("Skipping, Window minimised");
                    return;
                }
            }
        }

        let screen_descriptor = engine
            .resources
            .create_screen_descriptor(self.window.as_ref().unwrap().clone());

        let (surface_texture, surface_view) = engine.resources.get_surface_view_and_texture();

        let mut encoder = engine.resources.create_command_encoder();

        let window = self.window.as_mut().unwrap();

        // Ray Tracer Pass
        engine
            .ray_tracer
            .render(&mut encoder, engine.params.width, engine.params.height);

        // Render egui and Ray Tracer output
        {
            engine.egui.begin_frame(window);
            let mut ui_ctx = UiContext {
                renderer: &mut engine.renderer,
                scene_manager: &mut engine.scene_manager,
                timing: &mut engine.timing,
                tmp: &mut engine.tmp,
                params: &mut engine.params,
                window: window.clone(),
            };
            engine.egui.render_ui(&mut ui_ctx);

            engine.egui.end_frame_and_draw(
                &engine.resources.device,
                &engine.resources.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }

        engine.resources.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
    pub fn save_render_to_file(
        texture: &wgpu::Texture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate aligned bytes per row (wgpu requires 256-byte alignment)
        let bytes_per_pixel = 16; // RGBA
        let unpadded_bytes_per_row = RENDER_SIZE.0 * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = (bytes_per_row * RENDER_SIZE.1) as wgpu::BufferAddress;

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
                texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(RENDER_SIZE.1),
                },
            },
            Extent3d {
                width: RENDER_SIZE.0,
                height: RENDER_SIZE.1,
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
        let mut image_data = Vec::with_capacity((RENDER_SIZE.0 * RENDER_SIZE.1 * 4) as usize);

        for y in 0..RENDER_SIZE.1 {
            let row_start = (y * bytes_per_row) as usize;

            for x in (0..RENDER_SIZE.0).rev() {
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

        let mut image =
            ImageBuffer::<image::Rgba<u8>, _>::from_raw(RENDER_SIZE.0, RENDER_SIZE.1, image_data)
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
        let engine = self.engine.as_mut().unwrap();
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if engine.tmp.use_mouse {
                    engine
                        .scene_manager
                        .scene
                        .camera
                        .controller
                        .process_mouse(delta.0, delta.1);
                }
            }
            DeviceEvent::MouseWheel { delta } => {
                if engine.tmp.use_mouse {
                    engine
                        .scene_manager
                        .scene
                        .camera
                        .controller
                        .process_scroll(&delta);
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
            .engine
            .as_mut()
            .unwrap()
            .egui
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
        let timing = &mut self.engine.as_mut().unwrap().timing;
        let now = Instant::now();
        let dt = now - timing.last_render_time;
        timing.last_render_time = now;
        self.update(dt);
        self.window.as_ref().unwrap().request_redraw();
    }
}
