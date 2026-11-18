mod interconnect;
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
    let bios = memory::Bios::new("bios/SCPH1001.BIN").expect("failed_to_load_bios");
    let ram = memory::Ram::new();
    let inter = interconnect::Interconnect::new(bios, ram);
    let mut cpu = cpu::Cpu::new(inter);

    loop {
        cpu.run_next_instruction();
    }
}
