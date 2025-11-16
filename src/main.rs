use winit::event_loop::{ControlFlow, EventLoop};

use crate::core::app;
mod core;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

async fn run() {
    env_logger::builder()
        .filter_module("ray_tracer_2", log::LevelFilter::Info)
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .init();
    log::info!("Starting Ray Tracer");

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app::App::new();

    event_loop.run_app(&mut app).expect("Failed to run App");
}
