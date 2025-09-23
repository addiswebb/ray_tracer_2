use std::time::Duration;

use egui_wgpu::wgpu;
use glam::Vec3;
#[allow(unused_imports)]
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseScrollDelta},
    keyboard::KeyCode,
};
const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
const SAFE_FRAC_PI_2_DEG: f32 = SAFE_FRAC_PI_2 * (180.0 / std::f32::consts::PI);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct CameraUniform {
    pub origin: [f32; 3],
    _padding1: f32,
    pub lower_left_corner: [f32; 3],
    _padding2: f32,
    pub horizontal: [f32; 3],
    _padding3: f32,
    pub vertical: [f32; 3],
    _padding4: f32,
    pub near: f32,
    pub far: f32,
    _padding5: [f32; 2],
    pub w: [f32; 3],
    _padding6: f32,
    pub u: [f32; 3],
    _padding7: f32,
    pub v: [f32; 3],
    pub lens_radius: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub origin: Vec3,
    pub look_at: Vec3,
    pub view_up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub aperture: f32,
    pub focus_dist: f32,
    pub controller: CameraController,
}

#[allow(unused)]
pub struct CameraDescriptor {
    pub origin: Vec3,
    pub look_at: Vec3,
    pub view_up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
    pub aperture: f32,
    pub focus_dist: f32,
}
impl Camera {
    pub fn new(camera_descriptor: &CameraDescriptor) -> Self {
        Camera {
            origin: camera_descriptor.origin,
            look_at: camera_descriptor.look_at,
            view_up: camera_descriptor.view_up,
            fov: camera_descriptor.fov,
            // aspect: camera_descriptor.aspect,
            aspect: 16.0 / 9.0,
            near: camera_descriptor.near,
            far: camera_descriptor.far,
            aperture: camera_descriptor.aperture,
            focus_dist: camera_descriptor.focus_dist,
            controller: CameraController::new(600.0, 1.8),
        }
    }
    pub fn to_uniform(&self) -> CameraUniform {
        let theta = radians(self.fov);
        let half_height = self.near * f32::tan(theta / 2.0);
        let half_width = self.aspect * half_height;
        let w = (self.origin - self.look_at).normalize();
        let u = self.view_up.cross(w).normalize();
        let v = w.cross(u);
        let half_width_u = half_width * u;
        let half_height_v = half_height * v;
        let horizontal = 2.0 * half_width_u;
        let vertical = 2.0 * half_height_v;
        let lower_left_corner = self.origin - half_width_u - half_height_v - self.near * w;

        CameraUniform {
            origin: self.origin.to_array(),
            _padding1: 0.0,
            lower_left_corner: lower_left_corner.to_array(),
            _padding2: 0.0,
            horizontal: horizontal.to_array(),
            _padding3: 0.0,
            vertical: vertical.to_array(),
            _padding4: 0.0,
            near: self.near,
            far: self.far,
            _padding5: [0.0; 2],
            w: w.to_array(),
            _padding6: 0.0,
            u: u.to_array(),
            _padding7: 0.0,
            v: v.to_array(),
            lens_radius: self.aperture / 2.0,
        }
    }
    pub fn update_camera(&mut self, dt: Duration) {
        let dt = dt.as_secs_f32();

        let direction = (self.look_at - self.origin).normalize();
        let mut pitch = direction.y.asin();
        let mut yaw = direction.x.atan2(direction.z);
        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = yaw.sin_cos();
        let forward = Vec3::new(yaw_sin, 0.0, yaw_cos).normalize();
        let right = Vec3::new(yaw_cos, 0.0, -yaw_sin).normalize();
        self.origin += forward
            * (self.controller.amount_forward - self.controller.amount_backward)
            * self.controller.speed
            * dt;
        self.origin += right
            * (self.controller.amount_right - self.controller.amount_left)
            * self.controller.speed
            * dt;

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        let (pitch_sin, pitch_cos) = pitch.sin_cos();
        let scrollward = Vec3::new(pitch_cos * yaw_sin, pitch_sin, pitch_cos * yaw_cos).normalize();
        self.origin -= scrollward
            * self.controller.scroll
            * self.controller.speed
            * self.controller.sensitivity
            * dt;
        self.controller.scroll = 0.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        self.origin.y +=
            (self.controller.amount_up - self.controller.amount_down) * self.controller.speed * dt;

        let scalar = self.controller.sensitivity * dt;
        // Rotate
        yaw += self.controller.rotate_horizontal * scalar;
        pitch += -self.controller.rotate_vertical * scalar;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.controller.rotate_horizontal = 0.0;
        self.controller.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if pitch < -SAFE_FRAC_PI_2_DEG {
            pitch = -SAFE_FRAC_PI_2_DEG;
        } else if pitch > SAFE_FRAC_PI_2_DEG {
            pitch = SAFE_FRAC_PI_2_DEG;
        }
        self.look_at = self.origin
            + Vec3::new(
                pitch.cos() * yaw.sin(),
                pitch.sin(),
                pitch.cos() * yaw.cos(),
            );
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

    pub fn is_moving(&self) -> bool {
        [
            self.amount_left,
            self.amount_right,
            self.amount_forward,
            self.amount_backward,
            self.amount_up,
            self.amount_down,
            self.scroll,
        ]
        .iter()
        .any(|&x| x != 0.0)
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
pub fn radians(deg: f32) -> f32 {
    deg * (std::f32::consts::PI / 180.0)
}
#[allow(unused)]
pub fn degrees(rad: f32) -> f32 {
    rad * (180.0 / std::f32::consts::PI)
}
