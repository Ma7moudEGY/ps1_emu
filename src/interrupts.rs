use crate::interconnect::Addressable;

pub struct InterruptControl {
    status: u32,
    mask: u32,
}

impl InterruptControl {
    pub fn new() -> Self {
        InterruptControl { status: 0, mask: 0 }
    }

    pub fn load<T: Addressable>(&self, offset: u32) -> T {
        let val = match offset {
            0 => self.status,
            4 => self.mask,
            _ => unreachable!(),
        };

        Addressable::from_u32(val)
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        let val = val.as_u32();

        match offset {
            0 => self.status &= !val,
            4 => self.mask = val & 0x1ff,
            _ => unreachable!(),
        }
    }

    pub fn raise(&mut self, bit: u32) {
        self.status |= 1 << bit;
    }

    pub fn pending(&self) -> bool {
        self.status & self.mask != 0
    }
}
