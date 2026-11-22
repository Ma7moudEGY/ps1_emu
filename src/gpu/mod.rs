use std::cell::RefCell;

use crate::{
    frontend::video::{Color, Position, Renderer, Triangle},
    interconnect::{AccessWidth, Addressable},
};

mod commands;
mod renderer;

pub struct Gpu {
    page_base_x: u8,
    page_base_y: u8,
    semi_transparency: u8,
    texture_depth: TextureDepth,
    dithering: bool,
    draw_to_diplay: bool,
    force_set_mask_bit: bool,
    preserve_masked_pixels: bool,
    field: Field,
    texture_disable: bool,
    hres: HorizontalRes,
    vres: VerticalRes,
    vmode: VMode,
    display_depth: DisplayDepth,
    interlaced: bool,
    display_disabled: bool,
    interrupt: bool,
    rectangle_texture_x_flip: bool,
    rectangle_texture_y_flip: bool,
    dma_dir: DmaDirection,
    texture_window_x_mask: u8,
    texture_window_y_mask: u8,
    texture_window_x_offset: u8,
    texture_window_y_offset: u8,
    drawing_area_left: u16,
    drawing_area_top: u16,
    drawing_area_right: u16,
    drawing_area_bottom: u16,
    drawing_x_offset: i16,
    drawing_y_offset: i16,
    display_vram_x_start: u16,
    display_vram_y_start: u16,
    display_horiz_start: u16,
    display_horiz_end: u16,
    display_line_start: u16,
    display_line_end: u16,
    gp0_command: CommandBuffer,
    gp0_words_remaining: u32,
    gp0_command_method: fn(&mut Gpu),
    gp0_mode: Gp0Mode,
    pending_triangles: Vec<Triangle>,
    cycles: u32,
    scanline: u32,
    pub in_vblank: bool,
    frame: u32,
    vram: Box<[u16]>,
    transfer: RefCell<Transfer>,
}

impl Gpu {
    pub fn new() -> Self {
        Gpu {
            page_base_x: 0,
            page_base_y: 0,
            semi_transparency: 0,
            texture_depth: TextureDepth::T4Bit,
            dithering: false,
            draw_to_diplay: false,
            force_set_mask_bit: false,
            preserve_masked_pixels: false,
            field: Field::Top,
            texture_disable: false,
            hres: HorizontalRes::from_fields(0, 0),
            vres: VerticalRes::Y240Lines,
            vmode: VMode::Ntsc,
            display_depth: DisplayDepth::D15Bits,
            interlaced: false,
            display_disabled: true,
            interrupt: false,
            dma_dir: DmaDirection::Off,
            rectangle_texture_x_flip: false,
            rectangle_texture_y_flip: false,
            texture_window_x_mask: 0,
            texture_window_y_mask: 0,
            texture_window_x_offset: 0,
            texture_window_y_offset: 0,
            drawing_area_left: 0,
            drawing_area_top: 0,
            drawing_area_right: 0,
            drawing_area_bottom: 0,
            drawing_x_offset: 0i16,
            drawing_y_offset: 0i16,
            display_vram_x_start: 0,
            display_vram_y_start: 0,
            display_horiz_start: 0,
            display_horiz_end: 0,
            display_line_start: 0,
            display_line_end: 0,
            gp0_command: CommandBuffer::new(),
            gp0_words_remaining: 0,
            gp0_command_method: Gpu::gp0_nop,
            gp0_mode: Gp0Mode::Command,
            pending_triangles: vec![],
            cycles: 0,
            scanline: 0,
            in_vblank: false,
            frame: 0,
            vram: vec![0u16; 1024 * 512].into_boxed_slice(),
            transfer: RefCell::new(Transfer::new()),
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        renderer.display(&self.pending_triangles);
        self.pending_triangles.clear();
    }

    pub fn status(&self) -> u32 {
        let mut r = 0u32;

        r |= (self.page_base_x as u32) << 0;
        r |= (self.page_base_y as u32) << 4;
        r |= (self.semi_transparency as u32) << 5;
        r |= (self.texture_depth as u32) << 7;
        r |= (self.dithering as u32) << 9;
        r |= (self.draw_to_diplay as u32) << 10;
        r |= (self.force_set_mask_bit as u32) << 11;
        r |= (self.preserve_masked_pixels as u32) << 12;
        r |= (self.field as u32) << 13;
        r |= (self.texture_disable as u32) << 15;
        r |= self.hres.into_status();
        // r |= (self.vres as u32) << 19; to avoid lock
        r |= (self.vmode as u32) << 20;
        r |= (self.display_depth as u32) << 21;
        r |= (self.interlaced as u32) << 22;
        r |= (self.display_disabled as u32) << 23;
        r |= (self.interrupt as u32) << 24;

        r |= 1 << 26;
        r |= 1 << 27;
        r |= 1 << 28;

        r |= (self.dma_dir as u32) << 29;
        r |= (self.in_vblank as u32) << 31;

        let dma_request = match self.dma_dir {
            DmaDirection::Off => 0,
            DmaDirection::Fifo => 1,
            DmaDirection::CpuToGp0 => (r >> 28) & 1,
            DmaDirection::VramToCpu => (r >> 27) & 1,
        };

        r |= dma_request << 25;

        r
    }

    pub fn gp0(&mut self, val: u32) {
        if self.gp0_words_remaining == 0 {
            let opcode = (val >> 24) & 0xff;

            let (len, method) = match opcode {
                0x00 => (1, Gpu::gp0_nop as fn(&mut Gpu)),
                0x01 => (1, Gpu::gp0_clear_cache as fn(&mut Gpu)),
                0x02 => (3, Gpu::gp0_fill as fn(&mut Gpu)),
                0x28 => (5, Gpu::gp0_quad_mono_opaque as fn(&mut Gpu)),
                0x2c => (9, Gpu::gp0_quad_texture_blend_opaque as fn(&mut Gpu)),
                0x30 => (6, Gpu::gp0_triangle_shaded_opaque as fn(&mut Gpu)),
                0x38 => (8, Gpu::gp0_quad_shaded_opaque as fn(&mut Gpu)),
                0xa0 => (3, Gpu::gp0_image_load as fn(&mut Gpu)),
                0xc0 => (3, Gpu::gp0_image_store as fn(&mut Gpu)),
                0xe1 => (1, Gpu::gp0_draw_mode as fn(&mut Gpu)),
                0xe2 => (1, Gpu::gp0_texture_window as fn(&mut Gpu)),
                0xe3 => (1, Gpu::gp0_drawing_area_top_left as fn(&mut Gpu)),
                0xe4 => (1, Gpu::gp0_drawing_area_bottom_right as fn(&mut Gpu)),
                0xe5 => (1, Gpu::gp0_drawing_offset as fn(&mut Gpu)),
                0xe6 => (1, Gpu::gp0_mask_bit_setting as fn(&mut Gpu)),
                _ => todo!("unhandled_GP0_command_{:08x}", val),
            };

            self.gp0_words_remaining = len;
            self.gp0_command_method = method;

            self.gp0_command.clear();
        }

        self.gp0_words_remaining -= 1;

        match self.gp0_mode {
            Gp0Mode::Command => {
                self.gp0_command.push_word(val);

                if self.gp0_words_remaining == 0 {
                    (self.gp0_command_method)(self)
                }
            }
            Gp0Mode::ImageLoad => {
                if let Some(offset) = self.transfer_offset() {
                    self.vram[offset] = val as u16;
                }
                self.advance_transfer();

                if let Some(offset) = self.transfer_offset() {
                    self.vram[offset] = (val >> 16) as u16;
                }
                self.advance_transfer();

                if self.gp0_words_remaining == 0 {
                    self.gp0_mode = Gp0Mode::Command;
                }
            }
        }
    }

    fn gp0_nop(&mut self) {}

    fn gp0_draw_mode(&mut self) {
        let val = self.gp0_command[0];

        self.page_base_x = (val & 0xf) as u8;
        self.page_base_y = ((val >> 4) & 1) as u8;
        self.semi_transparency = ((val >> 5) & 3) as u8;

        self.texture_depth = match (val >> 7) & 3 {
            0 => TextureDepth::T4Bit,
            1 => TextureDepth::T8Bit,
            2 => TextureDepth::T15Bit,
            n => todo!("unhandled_texture_depth_{}", n),
        };

        self.dithering = ((val >> 9) & 1) != 0;
        self.draw_to_diplay = ((val >> 10) & 1) != 0;
        self.texture_disable = ((val >> 11) & 1) != 0;
        self.rectangle_texture_x_flip = ((val >> 12) & 1) != 0;
        self.rectangle_texture_y_flip = ((val >> 13) & 1) != 0;
    }

    fn gp0_drawing_area_top_left(&mut self) {
        let val = self.gp0_command[0];

        self.drawing_area_top = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_left = (val & 0x3ff) as u16;
    }

    fn gp0_drawing_area_bottom_right(&mut self) {
        let val = self.gp0_command[0];

        self.drawing_area_bottom = ((val >> 10) & 0x3ff) as u16;
        self.drawing_area_right = (val & 0x3ff) as u16;
    }

    fn gp0_drawing_offset(&mut self) {
        let val = self.gp0_command[0];

        let x = (val & 0x7ff) as u16;
        let y = ((val >> 11) & 0x7ff) as u16;

        self.drawing_x_offset = ((x << 5) as i16) >> 5;
        self.drawing_y_offset = ((y << 5) as i16) >> 5;
    }

    fn gp0_texture_window(&mut self) {
        let val = self.gp0_command[0];

        self.texture_window_x_mask = (val & 0x1f) as u8;
        self.texture_window_y_mask = ((val >> 5) & 0x1f) as u8;
        self.texture_window_x_offset = ((val >> 10) & 0x1f) as u8;
        self.texture_window_y_offset = ((val >> 15) & 0x1f) as u8;
    }

    fn gp0_mask_bit_setting(&mut self) {
        let val = self.gp0_command[0];

        self.force_set_mask_bit = (val & 1) != 0;
        self.preserve_masked_pixels = (val & 2) != 0;
    }

    fn gp0_quad_mono_opaque(&mut self) {
        let positions_1 = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[2]),
            Position::from_gp0(self.gp0_command[3]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let positions_2 = [
            Position::from_gp0(self.gp0_command[2]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[4]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let colors_1 = [Color::from_gp0(self.gp0_command[0]); 3];
        let colors_2 = [Color::from_gp0(self.gp0_command[0]); 3];

        self.pending_triangles.push(Triangle {
            positions: positions_1,
            colors: colors_1,
        });
        self.pending_triangles.push(Triangle {
            positions: positions_2,
            colors: colors_2,
        });
    }

    fn gp0_clear_cache(&mut self) {
        println!("Clearing_cache");
    }

    fn gp0_image_load(&mut self) {
        let pos = self.gp0_command[1];
        let size = self.gp0_command[2];

        self.start_transfer(pos, size);
        self.gp0_words_remaining = self.transfer.borrow().total / 2;
        self.gp0_mode = Gp0Mode::ImageLoad;
    }

    fn gp0_image_store(&mut self) {
        let pos = self.gp0_command[1];
        let size = self.gp0_command[2];

        self.start_transfer(pos, size);
    }

    fn gp0_quad_shaded_opaque(&mut self) {
        let positions_1 = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let positions_2 = [
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
            Position::from_gp0(self.gp0_command[7]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let colors_1 = [
            Color::from_gp0(self.gp0_command[0]),
            Color::from_gp0(self.gp0_command[2]),
            Color::from_gp0(self.gp0_command[4]),
        ];

        let colors_2 = [
            Color::from_gp0(self.gp0_command[2]),
            Color::from_gp0(self.gp0_command[4]),
            Color::from_gp0(self.gp0_command[6]),
        ];

        self.pending_triangles.push(Triangle {
            positions: positions_1,
            colors: colors_1,
        });
        self.pending_triangles.push(Triangle {
            positions: positions_2,
            colors: colors_2,
        });
    }

    fn gp0_triangle_shaded_opaque(&mut self) {
        let positions = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let colors = [
            Color::from_gp0(self.gp0_command[0]),
            Color::from_gp0(self.gp0_command[2]),
            Color::from_gp0(self.gp0_command[4]),
        ];

        self.pending_triangles.push(Triangle { positions, colors });
    }

    fn gp0_quad_texture_blend_opaque(&mut self) {
        let positions_1 = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let positions_2 = [
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
            Position::from_gp0(self.gp0_command[7]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let colors = [Color {
            r: 0x80,
            g: 0x00,
            b: 0x00,
        }; 3];

        self.pending_triangles.push(Triangle {
            positions: positions_1,
            colors,
        });
        self.pending_triangles.push(Triangle {
            positions: positions_2,
            colors,
        });
    }

    fn gp0_fill(&mut self) {
        let color = self.gp0_command[0];
        let color = (((color >> 3) & 0x1f)
            | (((color >> 11) & 0x1f) << 5)
            | (((color >> 19) & 0x1f) << 10)) as u16;

        let pos = self.gp0_command[1];
        let size = self.gp0_command[2];

        let x = pos & 0x3f0;
        let y = (pos >> 16) & 0x1ff;
        let w = ((size & 0x3ff) + 0xf) & !0xf;
        let h = (size >> 16) & 0x1ff;

        if w == 0 || h == 0 {
            return;
        }

        for dy in 0..h {
            for dx in 0..w {
                let i = self.vram_index((x + dx) as u16, (y + dy) as u16);
                self.vram[i] = color;
            }
        }
    }

    pub fn gp1(&mut self, val: u32) {
        let opcode = (val >> 24) & 0xff;

        match opcode {
            0x00 => self.gp1_reset(val),
            0x01 => self.gp1_reset_command_buffer(),
            0x02 => self.gp1_ack_irq(),
            0x03 => self.gp1_display_enable(val),
            0x04 => self.gp1_dma_dir(val),
            0x05 => self.gp1_display_vram_start(val),
            0x06 => self.gp1_display_horizontal_range(val),
            0x07 => self.gp1_display_vertical_range(val),
            0x08 => self.gp1_display_mode(val),
            _ => todo!("unhandled_GP1_command_{:08x}", val),
        }
    }

    fn gp1_reset(&mut self, _: u32) {
        self.page_base_x = 0;
        self.page_base_y = 0;
        self.semi_transparency = 0;
        self.texture_depth = TextureDepth::T4Bit;
        self.dithering = false;
        self.draw_to_diplay = false;
        self.force_set_mask_bit = false;
        self.preserve_masked_pixels = false;
        self.field = Field::Top;
        self.texture_disable = false;
        self.hres = HorizontalRes::from_fields(0, 0);
        self.vres = VerticalRes::Y240Lines;
        self.vmode = VMode::Ntsc;
        self.display_depth = DisplayDepth::D15Bits;
        self.interlaced = true;
        self.display_disabled = true;
        self.interrupt = false;
        self.dma_dir = DmaDirection::Off;
        self.rectangle_texture_x_flip = false;
        self.rectangle_texture_y_flip = false;
        self.texture_window_x_mask = 0;
        self.texture_window_y_mask = 0;
        self.texture_window_x_offset = 0;
        self.texture_window_y_offset = 0;
        self.drawing_area_left = 0;
        self.drawing_area_top = 0;
        self.drawing_area_right = 0;
        self.drawing_area_bottom = 0;
        self.drawing_x_offset = 0i16;
        self.drawing_y_offset = 0i16;
        self.display_vram_x_start = 0;
        self.display_vram_y_start = 0;
        self.display_horiz_start = 0x200;
        self.display_horiz_end = 0xc00;
        self.display_line_start = 0x10;
        self.display_line_end = 0x100;
    }

    fn gp1_display_mode(&mut self, val: u32) {
        let hr1 = (val & 3) as u8;
        let hr2 = ((val >> 6) & 1) as u8;

        self.hres = HorizontalRes::from_fields(hr1, hr2);

        self.vres = match val & 0x4 != 0 {
            false => VerticalRes::Y240Lines,
            true => VerticalRes::Y480Lines,
        };

        self.vmode = match val & 0x8 != 0 {
            false => VMode::Ntsc,
            true => VMode::Pal,
        };

        self.display_depth = match val & 0x10 != 0 {
            false => DisplayDepth::D24Bits,
            true => DisplayDepth::D15Bits,
        };

        self.interlaced = val & 0x20 != 0;

        if val & 0x80 != 0 {
            panic!("unsupported_display_mode_{:08x}", val)
        }
    }

    fn gp1_dma_dir(&mut self, val: u32) {
        self.dma_dir = match val & 3 {
            0 => DmaDirection::Off,
            1 => DmaDirection::Fifo,
            2 => DmaDirection::CpuToGp0,
            3 => DmaDirection::VramToCpu,
            _ => unreachable!(),
        }
    }

    fn gp1_display_vram_start(&mut self, val: u32) {
        self.display_vram_x_start = (val & 0x3fe) as u16;
        self.display_vram_y_start = ((val >> 10) & 0x1ff) as u16;
    }

    fn gp1_display_horizontal_range(&mut self, val: u32) {
        self.display_horiz_start = (val & 0xfff) as u16;
        self.display_horiz_end = ((val >> 12) & 0x3ff) as u16;
    }

    fn gp1_display_vertical_range(&mut self, val: u32) {
        self.display_line_start = (val & 0x3ff) as u16;
        self.display_line_end = ((val >> 10) & 0x3ff) as u16;
    }

    fn gp1_display_enable(&mut self, val: u32) {
        self.display_disabled = val & 1 != 0;
    }

    fn gp1_ack_irq(&mut self) {
        self.interrupt = false;
    }

    fn gp1_reset_command_buffer(&mut self) {
        self.gp0_command.clear();
        self.gp0_words_remaining = 0;
        self.gp0_mode = Gp0Mode::Command;
    }

    fn apply_draw_offset(&self, p: Position) -> Position {
        Position {
            x: p.x.wrapping_add(self.drawing_x_offset),
            y: p.y.wrapping_add(self.drawing_y_offset),
        }
    }

    fn read(&self) -> u32 {
        let lo = match self.transfer_offset() {
            Some(i) => {
                let v = self.vram[i] as u32;
                self.advance_transfer();
                v
            }
            None => 0,
        };

        let hi = match self.transfer_offset() {
            Some(i) => {
                let v = self.vram[i] as u32;
                self.advance_transfer();
                v
            }
            None => 0,
        };

        lo | (hi << 16)
    }

    pub fn load<T: Addressable>(&self, offset: u32) -> T {
        if T::width() != AccessWidth::Word {
            panic!("unhandled_{:?}_GPU_load", T::width());
        }

        let r = match offset {
            0 => self.read(),
            4 => self.status(),
            _ => unreachable!(),
        };

        Addressable::from_u32(r)
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        if T::width() != AccessWidth::Word {
            panic!("unhandled_{:?}_GPU_load", T::width());
        }

        let val = val.as_u32();

        match offset {
            0 => self.gp0(val),
            4 => self.gp1(val),
            _ => unreachable!(),
        }
    }

    pub fn sync(&mut self, cycles: u32) {
        self.cycles += cycles;

        let scanline_limit = match self.vmode {
            VMode::Ntsc => SCANLINE_PER_NTSC,
            VMode::Pal => SCANLINE_PER_PAL,
        };
        let vblank_start = scanline_limit - 22;

        while self.cycles >= CYCLES_PER_SCANLINE {
            self.cycles -= CYCLES_PER_SCANLINE;
            self.scanline += 1;

            if self.scanline >= scanline_limit {
                self.scanline = 0;
                self.frame = self.frame.wrapping_add(1);
            }

            if self.interlaced {
                self.field = if self.frame % 2 == 0 {
                    Field::Top
                } else {
                    Field::Bottom
                };
            } else {
                self.field = Field::Top;
            }
        }

        self.in_vblank = self.scanline >= vblank_start;
    }

    fn vram_index(&self, x: u16, y: u16) -> usize {
        ((y & 0x1ff) as usize) * 1024 + (x & 0x3ff) as usize
    }

    fn start_transfer(&self, pos: u32, size: u32) {
        let mut t = self.transfer.borrow_mut();

        t.x = (pos & 0x3ff) as u16;
        t.y = ((pos >> 16) & 0x1ff) as u16;

        let sw = size & 0x3ff;
        let sh = (size >> 16) & 0x1ff;
        t.w = (if sw == 0 { 0x400 } else { sw }) as u16;
        let h = if sh == 0 { 0x200 } else { sh };

        t.total = (t.w as u32 * h + 1) & !1;
        t.index = 0;
    }

    fn transfer_offset(&self) -> Option<usize> {
        let t = self.transfer.borrow();
        if t.index < t.total {
            Some(t.offset())
        } else {
            None
        }
    }

    fn advance_transfer(&self) {
        self.transfer.borrow_mut().index += 1;
    }
}

#[derive(Clone, Copy)]
enum TextureDepth {
    T4Bit = 0,
    T8Bit = 1,
    T15Bit = 2,
}

#[derive(Clone, Copy)]
enum Field {
    Top = 1,
    Bottom = 0,
}

#[derive(Clone, Copy)]
struct HorizontalRes(u8);

impl HorizontalRes {
    fn from_fields(hr1: u8, hr2: u8) -> HorizontalRes {
        let hr = (hr2 & 1) | ((hr1 & 3) << 1);

        HorizontalRes(hr)
    }

    fn into_status(self) -> u32 {
        let HorizontalRes(hr) = self;

        (hr as u32) << 16
    }
}

#[derive(Clone, Copy)]
enum VerticalRes {
    Y240Lines = 0,
    Y480Lines = 1,
}

#[derive(Clone, Copy)]
enum VMode {
    Ntsc = 0,
    Pal = 1,
}

const SCANLINE_PER_NTSC: u32 = 263;
const SCANLINE_PER_PAL: u32 = 314;
const CYCLES_PER_SCANLINE: u32 = 2147;

#[derive(Clone, Copy)]
enum DisplayDepth {
    D15Bits = 0,
    D24Bits = 1,
}

#[derive(Clone, Copy)]
enum DmaDirection {
    Off = 0,
    Fifo = 1,
    CpuToGp0 = 2,
    VramToCpu = 3,
}

#[derive(Clone, Copy)]
enum Gp0Mode {
    Command,
    ImageLoad,
}

#[derive(Clone, Copy)]
struct CommandBuffer {
    buffer: [u32; 12],
    len: u8,
}

impl CommandBuffer {
    fn new() -> Self {
        CommandBuffer {
            buffer: [0; 12],
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }

    fn push_word(&mut self, word: u32) {
        self.buffer[self.len as usize] = word;

        self.len += 1;
    }
}

impl ::std::ops::Index<usize> for CommandBuffer {
    type Output = u32;

    fn index<'a>(&'a self, index: usize) -> &'a u32 {
        if index >= self.len as usize {
            panic!(
                "command_buffer_index_out_of_range:_{}_({})",
                index, self.len
            );
        }

        &self.buffer[index]
    }
}

#[derive(Clone, Copy)]
struct Transfer {
    x: u16,
    y: u16,
    w: u16,
    total: u32,
    index: u32,
}

impl Transfer {
    pub fn new() -> Self {
        Transfer {
            x: 0,
            y: 0,
            w: 1,
            total: 0,
            index: 0,
        }
    }

    fn offset(&self) -> usize {
        let n = self.index;
        let x = (self.x as u32 + n % self.w as u32) & 0x3ff;
        let y = (self.y as u32 + n / self.w as u32) & 0x1ff;
        (y as usize) * 1024 + x as usize
    }
}
