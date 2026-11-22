use crate::interconnect::Addressable;

pub struct Timers {
    channels: [TimerChannel; 3],
}

impl Timers {
    pub fn new() -> Self {
        let channels: [TimerChannel; 3] = std::array::from_fn(|i| TimerChannel::new(i as u32));

        Timers { channels }
    }

    pub fn load<T: Addressable>(&self, offset: u32) -> T {
        let index = offset / 0x10;
        let channel = self.channel(index);
        let reg = offset % 0x10;

        let res = match reg {
            0 => channel.counter,
            4 => channel.mode as u16,
            8 => channel.target,
            12 => 0,
            _ => unreachable!(),
        };
    
        Addressable::from_u32(res as u32)
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        let val = val.as_u32() as u16;
        let index = offset / 0x10;
        let channel = self.channel_mut(index);
        let reg = offset % 0x10;

        match reg {
            0 => channel.counter = val,
            4 => {
                channel.counter = 0;
                channel.mode = (val as u32 & 0xfff) | (1 << 10);
            }
            8 => channel.target = val,
            12 => {}
            _ => unreachable!(),
        };
    }

    pub fn sync(&mut self, cycles: u32) -> u32 {
        let mut irqs: u32 = 0;

        for n in 0..3 {
            for _ in 0..cycles {
                if self.channels[n].tick() {
                    irqs |= 1 << n;
                }
            }
        }

        irqs
    }

    fn channel(&self, index: u32) -> &TimerChannel {
        &self.channels[index as usize]
    }

    fn channel_mut(&mut self, index: u32) -> &mut TimerChannel {
        &mut self.channels[index as usize]
    }
}

struct TimerChannel {
    pub base: u32,
    pub counter: u16,
    pub mode: u32,
    pub target: u16,
}

impl TimerChannel {
    pub fn new(index: u32) -> Self {
        let base = 0x1f801100 + index * 0x10;

        TimerChannel {
            base,
            counter: 0,
            mode: 0,
            target: 0,
        }
    }

    fn tick(&mut self) -> bool {
        let zret = self.mode & (1 << 3) != 0;
        let cmp = self.mode & (1 << 4) != 0;
        let ovfl = self.mode & (1 << 5) != 0;
        let repeat = self.mode & (1 << 6) != 0;
        let intf = (self.mode >> 10) & 1;

        let target = self.target & 0xffff;
        self.counter = self.counter.wrapping_add(1);

        let hit_target = self.counter == target;
        let hit_overflow = self.counter == 0xffff;

        let mut raised = false;
        if intf == 1 && ((cmp && hit_target) || (ovfl && hit_overflow)) {
            self.mode &= !(1 << 10);
            if repeat {
                self.mode |= 1 << 10;
            }
            raised = true;
        }

        if hit_overflow || (zret && hit_target) {
            self.counter = 0;
        }

        raised
    }
}
