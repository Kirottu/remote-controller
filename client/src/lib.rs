use egui_wgpu::wgpu;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod app;

#[cfg(target_os = "android")]
const PIXELS_PER_POINT: f32 = 4.0;
#[cfg(not(target_os = "android"))]
const PIXELS_PER_POINT: f32 = 2.0;

#[cfg_attr(target_os = "android", ndk_glue::main)]
pub fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut wgpu_state = egui_wgpu::winit::Painter::new(
        wgpu::Backends::all(),
        wgpu::PowerPreference::HighPerformance,
        wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::downlevel_defaults(),
        },
        wgpu::PresentMode::Fifo,
        1,
    );
    let mut winit_state = egui_winit::State::new(&event_loop);
    let mut app = app::App::new();
    let egui_ctx = egui::Context::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event, .. } => {
                winit_state.on_event(&egui_ctx, &event);
                if let WindowEvent::Resized(size) = event {
                    wgpu_state.on_window_resized(size.width, size.height);
                }
                if event == WindowEvent::CloseRequested {
                    *control_flow = ControlFlow::Exit;
                }
                if let WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } = event
                {
                    println!("{:?}, {:?}, {}", device_id, input, is_synthetic);
                }
            }
            Event::Resumed => {
                unsafe { wgpu_state.set_window(Some(&window)) };
                winit_state.set_pixels_per_point(PIXELS_PER_POINT);
            }
            Event::Suspended => {
                unsafe { wgpu_state.set_window(None) };
            }
            Event::RedrawRequested(..) => {
                let raw_input = winit_state.take_egui_input(&window);
                let full_output = egui_ctx.run(raw_input, |ctx| {
                    app.update(ctx);
                });

                let clipped_primitives = egui_ctx.tessellate(full_output.shapes);

                wgpu_state.paint_and_update_textures(
                    PIXELS_PER_POINT,
                    egui::Rgba::BLACK,
                    &clipped_primitives,
                    &full_output.textures_delta,
                );

                winit_state.handle_platform_output(&window, &egui_ctx, full_output.platform_output);
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => (),
        }
    });
}
