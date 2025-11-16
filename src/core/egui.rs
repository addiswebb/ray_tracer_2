use std::{sync::Arc, time::Duration};

use egui::Context;
use egui_wgpu::{
    Renderer, ScreenDescriptor,
    wgpu::{self, CommandEncoder, Device, Queue, TextureFormat, TextureView},
};
use egui_winit::State;
use glam::Quat;
use winit::{event::WindowEvent, window::Window};

use crate::core::{
    app::{DEBUG_MODES, Params},
    bvh,
    engine::{FrameTiming, RENDER_SIZE, TmpResources},
    scene::{SceneManager, SceneName},
};
pub struct UiContext<'a> {
    pub renderer: &'a mut crate::core::renderer::Renderer,
    pub scene_manager: &'a mut SceneManager,
    pub timing: &'a mut FrameTiming,
    pub tmp: &'a mut TmpResources,
    pub params: &'a mut Params,
    pub window: Arc<Window>,
}

pub struct EguiRenderer {
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
        window: Arc<Window>,
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
            state,
            renderer,
            frame_started: false,
        }
    }

    pub fn render_ui(&mut self, ctx: &mut UiContext) {
        let mut camera = ctx.scene_manager.scene.camera.clone();
        let mut params = ctx.params.clone();

        let mut skybox = params.skybox != 0;
        let mut accumulate = params.accumulate != 0;

        if !ctx.tmp.fullscreen {
            egui::TopBottomPanel::top("menu").show(self.context(), |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            log::warn!("idk how to close the window like this..");
                        }
                    });
                });
            });
            egui::SidePanel::right("Inspector")
                .resizable(true)
                .width_range(200.0..=400.0)
                .show(self.context(), |ui| {
                    ui.heading("Inspector");
                    ui.separator();
                    ui.heading("Camera");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut camera.transform.pos.x).speed(0.01));
                        ui.add(egui::DragValue::new(&mut camera.transform.pos.y).speed(0.01));
                        ui.add(egui::DragValue::new(&mut camera.transform.pos.z).speed(0.01));
                        ui.label(format!("Position"));
                    });
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut camera.transform.rot.x).speed(0.01));
                        ui.add(egui::DragValue::new(&mut camera.transform.rot.y).speed(0.01));
                        ui.add(egui::DragValue::new(&mut camera.transform.rot.z).speed(0.01));
                        ui.label(format!("Look At"));
                    });
                    ui.add(egui::Slider::new(&mut camera.fov, 10.0..=90.0).text("Fov"));
                    ui.add(
                        egui::Slider::new(&mut params.number_of_bounces, 0..=100).text("Bounces"),
                    );
                    ui.add(
                        egui::Slider::new(&mut params.rays_per_pixel, 0..=100)
                            .text("Rays Per Pixel"),
                    );
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut accumulate, "Accumulate");
                        params.accumulate = accumulate as i32;
                        if ui.button("Clear").clicked() {
                            params.reset_frame();
                            ctx.timing.reset();
                        }
                    });

                    ui.add(
                        egui::Slider::new(&mut camera.diverge_strength, 0.0..=500.0)
                            .step_by(0.1)
                            .text("Diverge Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut camera.defocus_strength, 0.0..=500.0)
                            .step_by(0.1)
                            .text("Defocus Strength"),
                    );
                    ui.add(
                        egui::Slider::new(&mut camera.focus_dist, 0.0..=10.0)
                            .step_by(0.01)
                            .text("Focus Distance"),
                    );
                    ui.separator();
                    ui.heading("Scene");
                    ui.checkbox(&mut skybox, "Skybox");
                    params.skybox = skybox as i32;
                    ui.horizontal(|ui| {
                        ui.label("Scene ID");
                        egui::ComboBox::from_label("Scene")
                            .selected_text(format!("{:?}", ctx.scene_manager.selected_scene))
                            .show_ui(ui, |ui| {
                                for &scene in SceneName::ALL.iter() {
                                    ui.selectable_value(
                                        &mut ctx.scene_manager.selected_scene,
                                        scene,
                                        format!("{:?}", scene),
                                    );
                                }
                            });
                    });
                    if ctx.scene_manager.selected_entity != -1 {
                        ui.separator();
                        if ctx.scene_manager.selected_entity
                            < ctx.scene_manager.scene.spheres.len() as i32
                        {
                            let s = &mut ctx.scene_manager.scene.spheres
                                [ctx.scene_manager.selected_entity as usize];
                            ui.heading("Sphere");
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.pos[0]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut s.pos[1]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut s.pos[2]).speed(0.01));
                                ui.label(format!("Position"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.radius).speed(0.01));
                                ui.label(format!("Radius"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.material.color[0]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut s.material.color[1]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut s.material.color[2]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut s.material.color[3]).speed(0.01));
                                ui.label(format!("Color"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut s.material.emission_color[0])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.emission_color[1])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.emission_color[2])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.emission_color[3])
                                        .speed(0.01),
                                );
                                ui.label(format!("Emissive Color"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut s.material.emission_strength)
                                        .speed(0.01),
                                );
                                ui.label(format!("Emission Strength"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut s.material.specular_color[0])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.specular_color[1])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.specular_color[2])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut s.material.specular_color[3])
                                        .speed(0.01),
                                );
                                ui.label(format!("Specular Color"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.material.specular).speed(0.01));
                                ui.label(format!("Specular Probability"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut s.material.smoothness).speed(0.01),
                                );
                                ui.label(format!("Smoothness"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.material.ior).speed(0.01));
                                ui.label(format!("Refractive Index"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut s.material.flag).speed(1));
                                ui.label(format!("Flag"));
                            });
                        } else {
                            let m = &mut ctx.scene_manager.scene.meshes[ctx
                                .scene_manager
                                .selected_entity
                                as usize
                                - ctx.scene_manager.scene.spheres.len()];
                            ui.heading("Mesh");
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.transform.pos.x).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.transform.pos.y).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.transform.pos.z).speed(0.01));
                                ui.label(format!("Position"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.transform.scale.x).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.transform.scale.y).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.transform.scale.z).speed(0.01));
                                ui.label(format!("Size"));
                            });

                            let (mut r_x, mut r_y, mut r_z) =
                                m.transform.rot.to_euler(glam::EulerRot::XYX);
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut r_x)
                                        .update_while_editing(false)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut r_y)
                                        .update_while_editing(false)
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut r_z)
                                        .update_while_editing(false)
                                        .speed(0.01),
                                );
                                ui.label(format!("Rotation"));
                            });
                            m.transform.rot = Quat::from_euler(glam::EulerRot::XYZ, r_x, r_y, r_z);

                            // Add size?

                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.material.color[0]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.material.color[1]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.material.color[2]).speed(0.01));
                                ui.add(egui::DragValue::new(&mut m.material.color[3]).speed(0.01));
                                ui.label(format!("Color"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut m.material.emission_color[0])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.emission_color[1])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.emission_color[2])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.emission_color[3])
                                        .speed(0.01),
                                );
                                ui.label(format!("Emissive Color"));
                            });

                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut m.material.emission_strength)
                                        .speed(0.01),
                                );
                                ui.label(format!("Emission Strength"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut m.material.specular_color[0])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.specular_color[1])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.specular_color[2])
                                        .speed(0.01),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut m.material.specular_color[3])
                                        .speed(0.01),
                                );
                                ui.label(format!("Specular Color"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.material.specular).speed(0.01));
                                ui.label(format!("Specular Probability"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut m.material.smoothness).speed(0.01),
                                );
                                ui.label(format!("Smoothness"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.material.ior).speed(0.01));
                                ui.label(format!("Refractive Index"));
                            });
                            ui.horizontal(|ui| {
                                ui.add(egui::DragValue::new(&mut m.material.flag).speed(1));
                                ui.label(format!("Flag"));
                            });
                        }
                    }
                    ui.separator();
                    ui.heading("Entities");
                    ui.label(format!(
                        "Meshes: {:#?}",
                        ctx.scene_manager.scene.meshes.len()
                    ));
                    ui.label(format!(
                        "Spheres: {:#?}",
                        ctx.scene_manager.scene.spheres.len()
                    ));
                });

            egui::SidePanel::left("Debug")
                .resizable(true)
                .width_range(200.0..=350.0)
                .show(self.context(), |ui| {
                    ui.heading("Debug");
                    ui.separator();
                    ui.label(format!("Frame: {}", params.frames));
                    ui.label(format!(
                        "FPS: {:.0}",
                        1.0 / (1.0 * ctx.timing.dt.as_secs_f64())
                    ));
                    ui.label(format!(
                        "Avg Frame Time: {:#?}",
                        ctx.timing.average_frame_time
                    ));
                    ui.separator();
                    ui.heading("BVH");
                    ui.label(format!(
                        "Nodes: {}",
                        ctx.scene_manager.scene.bvh_data.nodes.len()
                    ));
                    ui.label(format!(
                        "Triangle: {}",
                        ctx.scene_manager.scene.bvh_data.triangles.len()
                    ));

                    egui::ComboBox::from_label("Quality")
                        .selected_text(format!("{:?}", ctx.scene_manager.scene.bvh_quality))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut ctx.scene_manager.scene.bvh_quality,
                                bvh::Quality::High,
                                "High",
                            );
                            ui.selectable_value(
                                &mut ctx.scene_manager.scene.bvh_quality,
                                bvh::Quality::Low,
                                "Low",
                            );
                            ui.selectable_value(
                                &mut ctx.scene_manager.scene.bvh_quality,
                                bvh::Quality::Disabled,
                                "Disabled",
                            );
                        });

                    if ui.button("Rebuild BVH").clicked() {
                        ctx.scene_manager.scene.built_bvh = false;
                        params.reset_frame();
                        ctx.timing.reset();
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Resolution");
                        ui.add(
                            egui::DragValue::new(&mut params.width)
                                .update_while_editing(false)
                                .range(1..=RENDER_SIZE.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut params.height)
                                .update_while_editing(false)
                                .range(1..=RENDER_SIZE.1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Debug Mode:");
                        ui.add(
                            egui::DragValue::new(&mut params.debug_flag)
                                .speed(1)
                                .range(0..=DEBUG_MODES),
                        );
                    });
                    ui.add(
                        egui::Slider::new(&mut params.debug_scale, 1..=1000)
                            .text("Depth Threshold"),
                    );
                    ui.separator();
                    ui.heading("Entity List");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let nothing_selected = ctx.scene_manager.selected_entity == -1;
                        for (i, _) in ctx.scene_manager.scene.spheres.iter().enumerate() {
                            let selected =
                                ctx.scene_manager.selected_entity == i as i32 && !nothing_selected;
                            if ui.selectable_label(selected, "Sphere").clicked() {
                                ctx.scene_manager.selected_entity = i as i32;
                            }
                        }

                        for (i, m) in ctx.scene_manager.scene.meshes.iter().enumerate() {
                            let selected = ctx.scene_manager.selected_entity
                                - ctx.scene_manager.scene.spheres.len() as i32
                                == i as i32
                                && !nothing_selected;
                            if ui
                                .selectable_label(
                                    selected,
                                    m.label.clone().unwrap_or("Mesh".to_owned()),
                                )
                                .clicked()
                            {
                                ctx.scene_manager.selected_entity =
                                    (ctx.scene_manager.scene.spheres.len() + i) as i32;
                            }
                        }
                    });
                });
        }
        egui::CentralPanel::default().show(self.context(), |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                if ctx.renderer.render_ray_traced_image(ui) {
                    ctx.tmp.use_mouse = true;
                    ctx.window.set_cursor_visible(!ctx.tmp.use_mouse);
                    ctx.window
                        .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                        .unwrap();
                }
            });
        });

        if *ctx.params != params {
            *ctx.params = params;
            ctx.params.reset_frame();
            ctx.timing.reset();
        }
        if camera != ctx.scene_manager.scene.camera {
            ctx.scene_manager.scene.camera = camera;
            ctx.params.reset_frame();
            ctx.timing.reset();
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
