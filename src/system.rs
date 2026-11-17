use crate::memory::{Bios, Ram};

pub(crate) struct System {
    ram: Ram,
    bios: Bios,
}

impl System {
    pub(crate) fn new() -> Self {
        System {
            ram: Ram::new(),
            bios: Bios::new(),
        }
    }
}
