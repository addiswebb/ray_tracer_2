use std::{f32::consts::FRAC_PI_2, time::Duration};

use egui_wgpu::wgpu;
use glam::{EulerRot, Quat, Vec3};
#[allow(unused_imports)]
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};

use crate::scene::components::transform::Transform;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform {
    pub cam_to_world: [[f32; 4]; 4],
    pub view_params: [f32; 3],
    pub defocus_strength: f32,
    pub diverge_strength: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub transform: Transform,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub focus_dist: f32,
    pub controller: CameraController,
    pub defocus_strength: f32,
    pub diverge_strength: f32,
}

#[allow(unused)]
pub struct CameraDescriptor {
    pub transform: Transform,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub focus_dist: f32,
    pub defocus_strength: f32,
    pub diverge_strength: f32,
}

impl Default for CameraDescriptor {
    fn default() -> Self {
        Self {
            transform: Transform {
                pos: Vec3::ZERO,
                rot: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            fov: 90.0,
            aspect: 16.0 / 9.0,
            near: 0.01,
            far: 1000.0,
            focus_dist: 1.0,
            defocus_strength: 0.0,
            diverge_strength: 0.0,
        }
    }
}
impl Camera {
    pub fn new(camera_descriptor: &CameraDescriptor) -> Self {
        Camera {
            transform: camera_descriptor.transform,
            fov: camera_descriptor.fov,
            aspect: camera_descriptor.aspect,
            near: camera_descriptor.near,
            far: camera_descriptor.far,
            focus_dist: camera_descriptor.focus_dist.max(1.0),
            controller: CameraController::new(10.0, 1.8),
            defocus_strength: camera_descriptor.defocus_strength,
            diverge_strength: camera_descriptor.diverge_strength,
        }
    }
    pub fn to_uniform(&self) -> CameraUniform {
        assert!(self.focus_dist != 0.0, "Focus Distance cannot be zero");
        let plane_height = self.focus_dist * (self.fov * 0.5).to_radians().tan() * 2.0;
        let plane_width = plane_height * self.aspect;
        CameraUniform {
            cam_to_world: self.transform.to_matrix().to_cols_array_2d(),
            view_params: [plane_width, plane_height, self.focus_dist],
            defocus_strength: self.defocus_strength,
            diverge_strength: self.diverge_strength,
        }
    }
    pub fn update_camera(&mut self, dt: Duration) -> bool {
        let dt = dt.as_secs_f32();
        let mut moved = false;
        let scalar = self.controller.sensitivity * dt;

        // Handle rotation - FPS style (no roll)
        if self.controller.rotate_horizontal != 0.0 || self.controller.rotate_vertical != 0.0 {
            let (mut yaw, mut pitch, _roll) = self.transform.rot.to_euler(EulerRot::YXZ);

            yaw += self.controller.rotate_horizontal * scalar;
            pitch += self.controller.rotate_vertical * scalar;

            // Clamp pitch to avoid flipping
            const MAX_PITCH: f32 = FRAC_PI_2 - 0.1; // 89 degrees
            pitch = pitch.clamp(-MAX_PITCH, MAX_PITCH);

            // Reconstruct quaternion with zero roll
            self.transform.rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

            // Reset rotation inputs
            self.controller.rotate_horizontal = 0.0;
            self.controller.rotate_vertical = 0.0;
            moved = true;
        }

        let mut local_move = Vec3::ZERO;
        local_move.z += self.controller.amount_forward - self.controller.amount_backward;
        local_move.x += self.controller.amount_right - self.controller.amount_left;
        local_move.y += self.controller.amount_up - self.controller.amount_down;

        if local_move != Vec3::ZERO {
            let world_move =
                self.transform.rot * (local_move.normalize() * self.controller.speed * dt);
            self.transform.pos += world_move;
            moved = true;
        }

        if self.controller.scroll != 0.0 {
            let zoom_delta =
                self.transform.rot * Vec3::Z * self.controller.scroll * self.controller.speed * dt;
            self.transform.pos += zoom_delta;
            self.controller.scroll = 0.0;
            moved = true;
        }
        moved
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            0.01
        } else {
            0.0
        };

        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) -> bool {
        self.scroll = -match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 0.1,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
        return true;
    }
}
