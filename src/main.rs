mod device;
mod frame_data;
mod graphics_pipeline;
mod renderer;
mod surface;
mod surface_factory;
mod swapchain;
mod vulkan_debug;

use std::error::Error;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;

#[derive(Default)]
struct App {
    window: Option<Box<dyn Window>>,
    renderer: Option<renderer::Renderer>,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = WindowAttributes::default()
            .with_title("Vulkan")
            .with_surface_size(LogicalSize::new(WIDTH, HEIGHT));

        let window = event_loop.create_window(window_attributes).unwrap();
        let display_handle = window
            .rwh_06_display_handle()
            .display_handle()
            .unwrap()
            .as_raw();

        let window_handle = window
            .rwh_06_window_handle()
            .window_handle()
            .unwrap()
            .as_raw();

        let window_size = window.surface_size();

        let renderer = match renderer::Renderer::new(display_handle, window_handle, window_size) {
            Ok(renderer) => renderer,
            Err(e) => {
                eprintln!(
                    "Fatal Error: Failed to initialize Vulkan renderer: {e}\n
                    Try using the vulkan validation layers to debug the issue"
                );
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if self.renderer.is_some() {
                    self.renderer.as_mut().unwrap().draw();
                }

                self.window.as_mut().unwrap().request_redraw();
            }
            WindowEvent::SurfaceResized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    if let Some(renderer) = self.renderer.as_mut() {
                        renderer
                            .recreate_swapchain(new_size)
                            .expect("Failed to resize");
                    }
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop.run_app(App::default())?;

    Ok(())
}
