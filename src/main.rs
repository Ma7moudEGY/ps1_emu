mod system;
mod memory;
mod cpu;
mod cop0;
mod interrupts;
mod dma;
mod timers;
mod gpu;
mod frontend;
mod debugger;

fn main() {
    let _system = system::System::new();
    println!("skeleton boots");
}
