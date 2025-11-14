use std::sync::Arc;

use egui::Context;
use egui_wgpu::{
    Renderer, ScreenDescriptor,
    wgpu::{self, CommandEncoder, Device, Queue, TextureFormat, TextureView},
};
use egui_winit::State;
use winit::{event::WindowEvent, window::Window};

pub struct EguiRenderer {
    device: Arc<wgpu::Device>,
    state: State,
    pub renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn new(
        device: Arc<wgpu::Device>,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();
        let state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024),
        );

        let renderer = Renderer::new(
            device.clone().as_ref(),
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );
        EguiRenderer {
            device,
            state,
            renderer,
            frame_started: false,
        }
    }
    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    pub fn ppp(&mut self, v: f32) {
        self.context().set_pixels_per_point(v);
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!(
                "EguiRenderer::begin_frame must be called before EguiRenderer::end_frame_and_draw can be called"
            );
        }

        self.ppp(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();
        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }
        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("Egui Main Render Pass"),
            occlusion_query_set: None,
        });

        self.renderer.render(
            &mut render_pass.forget_lifetime(),
            &tris,
            &screen_descriptor,
        );

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        self.frame_started = false;
    }
}
