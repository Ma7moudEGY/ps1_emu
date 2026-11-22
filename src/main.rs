use winit::{application::ApplicationHandler, event::{ElementState, WindowEvent}, event_loop::EventLoop, keyboard::{Key, KeyCode, PhysicalKey}};

use crate::frontend::video::Renderer;

mod cop0;
mod cpu;
mod debugger;
mod dma;
mod frontend;
mod gpu;
mod interconnect;
mod interrupts;
mod memory;
mod timers;

const INSTRUCTION_PER_FRAME: u32 = 33868800 / 60;

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if $crate::trace_enabled() {
            println!($($arg)*);
        }
    };
}

struct App {
    cpu: cpu::Cpu,
    renderer: Option<Renderer>,
}

impl App {
    fn run_frame(&mut self) {
        for _ in 0..INSTRUCTION_PER_FRAME {
            self.cpu.run_next_instruction();
        }

        if let Some(renderer) = &mut self.renderer {
            self.cpu.interconnect_mut().gpu_mut().render(renderer);
            renderer.request_redraw();
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        match Renderer::new(event_loop) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(e) => {
                eprintln!("failed_to_create_renderer: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => {
                if let Some(renderer) = &self.renderer {
                    renderer.resize();
                }
            },
            WindowEvent::RedrawRequested => self.run_frame(),
            WindowEvent::KeyboardInput { event, ..} => {
                if event.state == ElementState::Pressed && event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    event_loop.exit();
                }
            },
            _ => {}
        }
    }
}

fn main() {
    let bios = memory::Bios::new("bios/SCPH1001.BIN").expect("failed_to_load_bios");
    let ram = memory::Ram::new();
    let dma = dma::Dma::new();
    let gpu = gpu::Gpu::new();
    let inter = interconnect::Interconnect::new(bios, ram, dma, gpu);
    let cpu = cpu::Cpu::new(inter);

    let event_loop = EventLoop::new().expect("failed_to_create_event_loop");
    let mut app = App {
        cpu, renderer: None
    };

    event_loop.run_app(&mut app).expect("failed_to_run_event_loop");
}
