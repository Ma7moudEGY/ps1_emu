use crate::cpu::instruction::Instruction;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    tag_valid: u32,
    line: [Instruction; 4],
}

impl ICacheLine {
    pub fn new() -> Self {
        ICacheLine {
            tag_valid: 0x0,
            line: [Instruction::new(0); 4],
        }
    }

    pub fn tag(&self) -> u32 {
        self.tag_valid & 0xfffff000
    }

    pub fn valid(&self, index: u32) -> bool {
        self.tag_valid & (1 << index) != 0
    }

    pub fn set_valid(&mut self, index: u32) {
        self.tag_valid |= 1 << index;
    }

    pub fn set_tag_valid(&mut self, tag: u32) {
        self.tag_valid = tag & 0xfffff000;
    }

    pub fn instruction(&self, index: u32) -> Instruction {
        self.line[index as usize]
    }

    pub fn set_instruction(&mut self, index: u32, instruction: Instruction) {
        self.line[index as usize] = instruction;
    }

    pub fn write_tag(&mut self, addr: u32, val: u32) {
        self.tag_valid = (val & 0xf) | (addr & 0xfffff000);
    }
}

impl Default for ICacheLine {
    fn default() -> ICacheLine {
        ICacheLine::new()
    }
}
