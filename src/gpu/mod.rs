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
    cycles: u32,
    scanline: u32,
    pub in_vblank: bool,
    frame: u32,
    vram: Box<[u16]>,
    transfer: RefCell<Transfer>,
    pending_draws: Vec<PendingDraw>,
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
            cycles: 0,
            scanline: 0,
            in_vblank: false,
            frame: 0,
            vram: vec![0u16; 1024 * 512].into_boxed_slice(),
            transfer: RefCell::new(Transfer::new()),
            pending_draws: vec![],
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        self.flush();

        renderer.display_vram(
            &self.vram,
            self.display_vram_x_start,
            self.display_vram_y_start,
            1024,
            512,
        );
        self.pending_draws.clear();
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
                0x24 => (7, Gpu::gp0_triangle_texture_blend_opaque as fn(&mut Gpu)),
                0x26 => (7, Gpu::gp0_triangle_texture_blend_opaque as fn(&mut Gpu)),
                0x28 => (5, Gpu::gp0_quad_mono_opaque as fn(&mut Gpu)),
                0x2c => (9, Gpu::gp0_quad_texture_blend_opaque as fn(&mut Gpu)),
                0x2e => (9, Gpu::gp0_quad_texture_blend_opaque as fn(&mut Gpu)),
                0x30 => (6, Gpu::gp0_triangle_shaded_opaque as fn(&mut Gpu)),
                0x34 => (9, Gpu::gp0_gouraud_texture_triangle as fn(&mut Gpu)),
                0x36 => (9, Gpu::gp0_gouraud_texture_triangle as fn(&mut Gpu)),
                0x38 => (8, Gpu::gp0_quad_shaded_opaque as fn(&mut Gpu)),
                0x3c => (12, Gpu::gp0_gouraud_texture_quad as fn(&mut Gpu)),
                0x3e => (12, Gpu::gp0_gouraud_texture_quad as fn(&mut Gpu)),
                0x40 => (3, Gpu::gp0_line_flat as fn(&mut Gpu)),
                0x50 => (4, Gpu::gp0_line_gouraud as fn(&mut Gpu)),
                0x60 => (3, Gpu::gp0_rectangle as fn(&mut Gpu)),
                0x68 => (2, Gpu::gp0_rectangle as fn(&mut Gpu)),
                0x70 => (2, Gpu::gp0_rectangle as fn(&mut Gpu)),
                0x78 => (2, Gpu::gp0_rectangle as fn(&mut Gpu)),
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

        let tri_1 = Triangle {
            positions: positions_1,
            colors: colors_1,
        };

        let tri_2 = Triangle {
            positions: positions_2,
            colors: colors_2,
        };

        self.pending_draws.push(PendingDraw::Triangle(tri_1));
        self.pending_draws.push(PendingDraw::Triangle(tri_2));
    }

    fn gp0_clear_cache(&mut self) {
        println!("Clearing_cache");
    }

    fn gp0_image_load(&mut self) {
        self.flush();
        let pos = self.gp0_command[1];
        let size = self.gp0_command[2];

        self.start_transfer(pos, size);
        self.gp0_words_remaining = self.transfer.borrow().total / 2;
        self.gp0_mode = Gp0Mode::ImageLoad;
    }

    fn gp0_image_store(&mut self) {
        self.flush();
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

        let tri_1 = Triangle {
            positions: positions_1,
            colors: colors_1,
        };

        let tri_2 = Triangle {
            positions: positions_2,
            colors: colors_2,
        };

        self.pending_draws.push(PendingDraw::Triangle(tri_1));
        self.pending_draws.push(PendingDraw::Triangle(tri_2));
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

        let tri = Triangle { positions, colors };

        self.pending_draws.push(PendingDraw::Triangle(tri));
    }

    fn gp0_gouraud_texture_quad(&mut self) {
        let semi_transparent = self.gp0_command[0] & (1 << 25) != 0;

        let colors_1 = [
            Color::from_gp0(self.gp0_command[0]),
            Color::from_gp0(self.gp0_command[3]),
            Color::from_gp0(self.gp0_command[6]),
        ];

        let colors_2 = [
            Color::from_gp0(self.gp0_command[3]),
            Color::from_gp0(self.gp0_command[6]),
            Color::from_gp0(self.gp0_command[9]),
        ];

        let positions_1 = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[4]),
            Position::from_gp0(self.gp0_command[7]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let positions_2 = [
            Position::from_gp0(self.gp0_command[4]),
            Position::from_gp0(self.gp0_command[7]),
            Position::from_gp0(self.gp0_command[10]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let uvs_1 = [
            Uv::from_gp0(self.gp0_command[2]),
            Uv::from_gp0(self.gp0_command[5]),
            Uv::from_gp0(self.gp0_command[8]),
        ];

        let uvs_2 = [
            Uv::from_gp0(self.gp0_command[5]),
            Uv::from_gp0(self.gp0_command[8]),
            Uv::from_gp0(self.gp0_command[11]),
        ];

        let clut = self.gp0_command[2];
        let page = self.gp0_command[5];
        let raw = self.gp0_command[0] & (1 << 24) != 0;

        let texture_attrs = self.texture_attr(clut, page, raw);

        self.pending_draws
            .push(PendingDraw::TexturedTriangle(TexturedTriangle {
                positions: positions_1,
                colors: colors_1,
                uvs: uvs_1,
                texture: texture_attrs,
                semi_transparent,
            }));

        self.pending_draws
            .push(PendingDraw::TexturedTriangle(TexturedTriangle {
                positions: positions_2,
                colors: colors_2,
                uvs: uvs_2,
                texture: texture_attrs,
                semi_transparent,
            }));
    }

    fn gp0_gouraud_texture_triangle(&mut self) {
        let semi_transparent = self.gp0_command[0] & (1 << 25) != 0;

        let colors = [
            Color::from_gp0(self.gp0_command[0]),
            Color::from_gp0(self.gp0_command[3]),
            Color::from_gp0(self.gp0_command[6]),
        ];

        let positions = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[4]),
            Position::from_gp0(self.gp0_command[7]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let uvs = [
            Uv::from_gp0(self.gp0_command[2]),
            Uv::from_gp0(self.gp0_command[5]),
            Uv::from_gp0(self.gp0_command[8]),
        ];

        let clut = self.gp0_command[2];
        let page = self.gp0_command[5];
        let raw = self.gp0_command[0] & (1 << 24) != 0;

        let texture_attrs = self.texture_attr(clut, page, raw);

        self.pending_draws
            .push(PendingDraw::TexturedTriangle(TexturedTriangle {
                positions,
                colors,
                uvs,
                texture: texture_attrs,
                semi_transparent,
            }));
    }

    fn gp0_triangle_texture_blend_opaque(&mut self) {
        let semi_transparent = self.gp0_command[0] & (1 << 25) != 0;

        let color = Color::from_gp0(self.gp0_command[0]);

        let positions = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let uvs = [
            Uv::from_gp0(self.gp0_command[2]),
            Uv::from_gp0(self.gp0_command[4]),
            Uv::from_gp0(self.gp0_command[6]),
        ];

        let clut = self.gp0_command[2];
        let page = self.gp0_command[4];
        let raw = self.gp0_command[0] & (1 << 24) != 0;

        let texture_attrs = self.texture_attr(clut, page, raw);

        self.pending_draws
            .push(PendingDraw::TexturedTriangle(TexturedTriangle {
                positions,
                colors: [color; 3],
                uvs,
                texture: texture_attrs,
                semi_transparent,
            }));
    }

    fn texture_attr(&self, clut: u32, page: u32, raw: bool) -> TextureAttributes {
        let clut = clut >> 16;
        let clut_x = (clut & 0x3f) * 16;
        let clut_y = (clut >> 6) & 0x1ff;

        let page = page >> 16;
        let page_x = (page & 0xf) * 64;
        let page_y = ((page >> 4) & 1) * 256;

        let depth = (page >> 7) & 3;
        let depth = match depth {
            0 => TextureDepth::T4Bit,
            1 => TextureDepth::T8Bit,
            2 | 3 => TextureDepth::T15Bit,
            n => todo!("unhandled_texture_depth_{}", n),
        };

        let abr = ((page >> 5) & 3) as u8;

        TextureAttributes {
            page_x: page_x as u16,
            page_y: page_y as u16,
            clut_x: clut_x as u16,
            clut_y: clut_y as u16,
            depth,
            raw,
            abr,
        }
    }

    fn gp0_quad_texture_blend_opaque(&mut self) {
        let semi_transparent = self.gp0_command[0] & (1 << 25) != 0;

        let clut = self.gp0_command[2];
        let page = self.gp0_command[4];
        let raw = self.gp0_command[0] & (1 << 24) != 0;

        let texture_attrs = self.texture_attr(clut, page, raw);

        let positions_1 = [
            Position::from_gp0(self.gp0_command[1]),
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let uvs_1 = [
            Uv::from_gp0(self.gp0_command[2]),
            Uv::from_gp0(self.gp0_command[4]),
            Uv::from_gp0(self.gp0_command[6]),
        ];

        let positions_2 = [
            Position::from_gp0(self.gp0_command[3]),
            Position::from_gp0(self.gp0_command[5]),
            Position::from_gp0(self.gp0_command[7]),
        ]
        .map(|p| self.apply_draw_offset(p));

        let uvs_2 = [
            Uv::from_gp0(self.gp0_command[4]),
            Uv::from_gp0(self.gp0_command[6]),
            Uv::from_gp0(self.gp0_command[8]),
        ];

        let color = Color::from_gp0(self.gp0_command[0]);

        let tri_1 = TexturedTriangle {
            positions: positions_1,
            uvs: uvs_1,
            colors: [color; 3],
            texture: texture_attrs,
            semi_transparent,
        };

        let tri_2 = TexturedTriangle {
            positions: positions_2,
            uvs: uvs_2,
            colors: [color; 3],
            texture: texture_attrs,
            semi_transparent,
        };

        self.pending_draws
            .push(PendingDraw::TexturedTriangle(tri_1));
        self.pending_draws
            .push(PendingDraw::TexturedTriangle(tri_2));
    }

    fn fetch_texel(&self, u: u32, v: u32, tex: &TextureAttributes) -> u16 {
        match tex.depth {
            TextureDepth::T4Bit => {
                let word =
                    self.vram[self.vram_index(tex.page_x + (u / 4) as u16, tex.page_y + v as u16)];
                let index = ((word >> ((u & 3) * 4)) & 0xf) as u16;
                self.vram[self.vram_index(tex.clut_x + index, tex.clut_y)]
            }
            TextureDepth::T8Bit => {
                let word =
                    self.vram[self.vram_index(tex.page_x + (u / 2) as u16, tex.page_y + v as u16)];
                let index = ((word >> ((u & 1) * 8)) & 0xff) as u16;
                self.vram[self.vram_index(tex.clut_x + index, tex.clut_y)]
            }
            TextureDepth::T15Bit => {
                self.vram[self.vram_index(tex.page_x + u as u16, tex.page_y + v as u16)]
            }
        }
    }

    fn rasterize_textured_triangle(&mut self, tri: &TexturedTriangle) {
        let p = tri.positions.map(|v| (v.x as i32, v.y as i32));

        let area = self.edge(p[0].0, p[0].1, p[1].0, p[1].1, p[2].0, p[2].1);
        if area == 0 {
            return;
        }

        let min_x = p
            .iter()
            .map(|v| v.0)
            .min()
            .unwrap()
            .max(self.drawing_area_left as i32)
            .max(0);
        let max_x = p
            .iter()
            .map(|v| v.0)
            .max()
            .unwrap()
            .min(self.drawing_area_right as i32)
            .min(1023);
        let min_y = p
            .iter()
            .map(|v| v.1)
            .min()
            .unwrap()
            .max(self.drawing_area_top as i32)
            .max(0);
        let max_y = p
            .iter()
            .map(|v| v.1)
            .max()
            .unwrap()
            .min(self.drawing_area_bottom as i32)
            .min(511);

        let x_mask = self.texture_window_x_mask as u32 * 8;
        let y_mask = self.texture_window_y_mask as u32 * 8;
        let x_offset = self.texture_window_x_offset as u32 * 8;
        let y_offset = self.texture_window_y_offset as u32 * 8;

        let u0 = tri.uvs[0].u as i64;
        let u1 = tri.uvs[1].u as i64;
        let u2 = tri.uvs[2].u as i64;
        let v0 = tri.uvs[0].v as i64;
        let v1 = tri.uvs[1].v as i64;
        let v2 = tri.uvs[2].v as i64;

        let abr = tri.texture.abr;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let w0 = self.edge(p[1].0, p[1].1, p[2].0, p[2].1, x, y);
                let w1 = self.edge(p[2].0, p[2].1, p[0].0, p[0].1, x, y);
                let w2 = self.edge(p[0].0, p[0].1, p[1].0, p[1].1, x, y);

                let inside = if area > 0 {
                    w0 >= 0 && w1 >= 0 && w2 >= 0
                } else {
                    w0 <= 0 && w1 <= 0 && w2 <= 0
                };
                if !inside {
                    continue;
                }

                let mut u =
                    ((u0 * w0 as i64 + u1 * w1 as i64 + u2 * w2 as i64) / area as i64) as u32;
                let mut v =
                    ((v0 * w0 as i64 + v1 * w1 as i64 + v2 * w2 as i64) / area as i64) as u32;

                u &= 0xff;
                v &= 0xff;
                u = (u & !x_mask) | (x_offset & x_mask);
                v = (v & !y_mask) | (y_offset & y_mask);

                let texel = self.fetch_texel(u, v, &tri.texture);
                if texel == 0 {
                    continue;
                }

                let tr = (texel & 0x1f) as i64;
                let tg = ((texel >> 5) & 0x1f) as i64;
                let tb = ((texel >> 10) & 0x1f) as i64;

                let mut r = (tr << 3) | (tr >> 2);
                let mut g = (tg << 3) | (tg >> 2);
                let mut b = (tb << 3) | (tb >> 2);

                if !tri.texture.raw {
                    let vr = (tri.colors[0].r as i64 * w0 as i64
                        + tri.colors[1].r as i64 * w1 as i64
                        + tri.colors[2].r as i64 * w2 as i64)
                        / area as i64;
                    let vg = (tri.colors[0].g as i64 * w0 as i64
                        + tri.colors[1].g as i64 * w1 as i64
                        + tri.colors[2].g as i64 * w2 as i64)
                        / area as i64;
                    let vb = (tri.colors[0].b as i64 * w0 as i64
                        + tri.colors[1].b as i64 * w1 as i64
                        + tri.colors[2].b as i64 * w2 as i64)
                        / area as i64;

                    r = (r * vr / 128).clamp(0, 255);
                    g = (g * vg / 128).clamp(0, 255);
                    b = (b * vb / 128).clamp(0, 255);
                }

                let mut color = ((r as u16 >> 3) & 0x1f)
                    | (((g as u16 >> 3) & 0x1f) << 5)
                    | (((b as u16 >> 3) & 0x1f) << 10);

                let i = self.vram_index(x as u16, y as u16);

                if self.preserve_masked_pixels && self.vram[i] & 0x8000 != 0 {
                    continue;
                }

                if tri.semi_transparent && texel & 0x8000 != 0 {
                    let dst = self.vram[i];
                    let br = dst & 0x1f;
                    let bg = (dst >> 5) & 0x1f;
                    let bb = (dst >> 10) & 0x1f;
                    let fr = color & 0x1f;
                    let fg = (color >> 5) & 0x1f;
                    let fb = (color >> 10) & 0x1f;

                    let blend = |back: u16, front: u16| -> u16 {
                        match abr {
                            0 => (back + front) / 2,
                            1 => (back + front).min(31),
                            2 => back.saturating_sub(front),
                            _ => (back + front / 4).min(31),
                        }
                    };

                    color = blend(br, fr) | (blend(bg, fg) << 5) | (blend(bb, fb) << 10);
                }

                let mask = if self.force_set_mask_bit {
                    0x8000
                } else {
                    texel & 0x8000
                };

                self.vram[i] = (color & 0x7fff) | mask;
            }
        }
    }

    fn gp0_fill(&mut self) {
        self.flush();
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

    fn gp0_rectangle(&mut self) {
        let cmd = self.gp0_command[0];
        let siz = (cmd >> 27) & 3;
        let pos = self.gp0_command[1];

        let x = (pos & 0xffff) as i32 + self.drawing_x_offset as i32;
        let y = (pos >> 16) as i32 + self.drawing_y_offset as i32;

        let (w, h) = match siz {
            0 => {
                let size = self.gp0_command[2];
                ((size & 0xffff) as i32, (size >> 16) as i32)
            }
            1 => (1, 1),
            2 => (8, 8),
            _ => (16, 16),
        };

        let color = (((cmd >> 3) & 0x1f)
            | (((cmd >> 11) & 0x1f) << 5)
            | (((cmd >> 19) & 0x1f) << 10)) as u16;

        if w > 0 && h > 0 {
            self.pending_draws
                .push(PendingDraw::Rect(Rect { x, y, w, h, color }));
        }
    }

    fn rasterize_rect(&mut self, rect: Rect) {
        let x0 = rect.x.max(self.drawing_area_left as i32).max(0);
        let y0 = rect.y.max(self.drawing_area_top as i32).max(0);
        let x1 = (rect.x + rect.w - 1)
            .min(self.drawing_area_right as i32)
            .min(1023);
        let y1 = (rect.y + rect.h - 1)
            .min(self.drawing_area_bottom as i32)
            .min(511);

        if x0 > x1 || y0 > y1 {
            return;
        }

        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = self.vram_index(x as u16, y as u16);
                if self.preserve_masked_pixels && self.vram[i] & 0x8000 != 0 {
                    continue;
                }
                self.vram[i] = rect.color;
            }
        }
    }

    fn gp0_line_flat(&mut self) {
        let color = Color::from_gp0(self.gp0_command[0]);

        let pos0 = Position::from_gp0(self.gp0_command[1]);
        let pos0 = self.apply_draw_offset(pos0);

        let pos1 = Position::from_gp0(self.gp0_command[2]);
        let pos1 = self.apply_draw_offset(pos1);

        self.pending_draws.push(PendingDraw::Line(Line {
            x0: pos0.x as i32,
            y0: pos0.y as i32,
            x1: pos1.x as i32,
            y1: pos1.y as i32,
            colors: [color, color],
        }));
    }

    fn gp0_line_gouraud(&mut self) {
        let color0 = Color::from_gp0(self.gp0_command[0]);

        let pos0 = Position::from_gp0(self.gp0_command[1]);
        let pos0 = self.apply_draw_offset(pos0);

        let color1 = Color::from_gp0(self.gp0_command[2]);

        let pos1 = Position::from_gp0(self.gp0_command[3]);
        let pos1 = self.apply_draw_offset(pos1);

        self.pending_draws.push(PendingDraw::Line(Line {
            x0: pos0.x as i32,
            y0: pos0.y as i32,
            x1: pos1.x as i32,
            y1: pos1.y as i32,
            colors: [color0, color1],
        }));
    }

    fn rasterize_line(&mut self, line: Line) {
        let dx = (line.x1 - line.x0).abs();
        let dy = (line.y1 - line.y0).abs();
        let steps = dx.max(dy);

        let sx = if line.x0 < line.x1 { 1 } else { -1 };
        let sy = if line.y0 < line.y1 { 1 } else { -1 };

        let mut x = line.x0;
        let mut y = line.y0;
        let mut err = dx - dy;

        for step in 0..=steps {
            let denom = steps.max(1);
            let t = step;

            let r = (line.colors[0].r as i32 * (denom - t) + line.colors[1].r as i32 * t) / denom;
            let g = (line.colors[0].g as i32 * (denom - t) + line.colors[1].g as i32 * t) / denom;
            let b = (line.colors[0].b as i32 * (denom - t) + line.colors[1].b as i32 * t) / denom;

            let color = (((r as u16) >> 3) & 0x1f)
                | ((((g as u16) >> 3) & 0x1f) << 5)
                | ((((b as u16) >> 3) & 0x1f) << 10);

            let inside_vram = (0..1024).contains(&x) && (0..512).contains(&y);
            let inside_area = x >= self.drawing_area_left as i32
                && x <= self.drawing_area_right as i32
                && y >= self.drawing_area_top as i32
                && y <= self.drawing_area_bottom as i32;

            if inside_vram && inside_area {
                let i = self.vram_index(x as u16, y as u16);
                if !self.preserve_masked_pixels || self.vram[i] & 0x8000 == 0 {
                    self.vram[i] = color;
                }
            }

            if x == line.x1 && y == line.y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
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

    pub fn load<T: Addressable>(&mut self, offset: u32) -> T {
        if T::width() != AccessWidth::Word {
            panic!("unhandled_{:?}_GPU_load", T::width());
        }

        let r = match offset {
            0 => {
                self.flush();
                self.read()
            }
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

    fn edge(&self, ax: i32, ay: i32, bx: i32, by: i32, px: i32, py: i32) -> i32 {
        (bx - ax) * (py - ay) - (by - ay) * (px - ax)
    }

    fn rasterize_triangle(&mut self, tri: &Triangle) {
        let p = tri.positions.map(|v| (v.x as i32, v.y as i32));

        let area = self.edge(p[0].0, p[0].1, p[1].0, p[1].1, p[2].0, p[2].1);
        if area == 0 {
            return;
        }

        // clip bounding box to drawing area and VRAM
        let min_x = p
            .iter()
            .map(|v| v.0)
            .min()
            .unwrap()
            .max(self.drawing_area_left as i32)
            .max(0);
        let max_x = p
            .iter()
            .map(|v| v.0)
            .max()
            .unwrap()
            .min(self.drawing_area_right as i32)
            .min(1023);
        let min_y = p
            .iter()
            .map(|v| v.1)
            .min()
            .unwrap()
            .max(self.drawing_area_top as i32)
            .max(0);
        let max_y = p
            .iter()
            .map(|v| v.1)
            .max()
            .unwrap()
            .min(self.drawing_area_bottom as i32)
            .min(511);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let w0 = self.edge(p[1].0, p[1].1, p[2].0, p[2].1, x, y);
                let w1 = self.edge(p[2].0, p[2].1, p[0].0, p[0].1, x, y);
                let w2 = self.edge(p[0].0, p[0].1, p[1].0, p[1].1, x, y);

                let inside = if area > 0 {
                    w0 >= 0 && w1 >= 0 && w2 >= 0
                } else {
                    w0 <= 0 && w1 <= 0 && w2 <= 0
                };
                if !inside {
                    continue;
                }

                let r = (tri.colors[0].r as i64 * w0 as i64
                    + tri.colors[1].r as i64 * w1 as i64
                    + tri.colors[2].r as i64 * w2 as i64)
                    / area as i64;
                let g = (tri.colors[0].g as i64 * w0 as i64
                    + tri.colors[1].g as i64 * w1 as i64
                    + tri.colors[2].g as i64 * w2 as i64)
                    / area as i64;
                let b = (tri.colors[0].b as i64 * w0 as i64
                    + tri.colors[1].b as i64 * w1 as i64
                    + tri.colors[2].b as i64 * w2 as i64)
                    / area as i64;

                let color = (((r as u16 >> 3) & 0x1f)
                    | (((g as u16 >> 3) & 0x1f) << 5)
                    | (((b as u16 >> 3) & 0x1f) << 10)) as u16;

                let i = self.vram_index(x as u16, y as u16);
                self.vram[i] = color;
            }
        }
    }

    fn flush(&mut self) {
        for i in 0..self.pending_draws.len() {
            let draw = self.pending_draws[i];
            match draw {
                PendingDraw::Triangle(t) => self.rasterize_triangle(&t),
                PendingDraw::Rect(r) => self.rasterize_rect(r),
                PendingDraw::Line(l) => self.rasterize_line(l),
                PendingDraw::TexturedTriangle(tt) => self.rasterize_textured_triangle(&tt),
            }
        }
        self.pending_draws.clear();
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

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: u16,
}

#[derive(Clone, Copy)]
struct Line {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    colors: [Color; 2],
}

#[derive(Clone, Copy)]
enum PendingDraw {
    Triangle(Triangle),
    Rect(Rect),
    Line(Line),
    TexturedTriangle(TexturedTriangle),
}

#[derive(Clone, Copy)]
struct Uv {
    u: u8,
    v: u8,
}

impl Uv {
    pub fn from_gp0(val: u32) -> Uv {
        let u = (val & 0xff) as u8;
        let v = ((val >> 8) & 0xff) as u8;

        Uv { u, v }
    }
}

#[derive(Clone, Copy)]
struct TextureAttributes {
    page_x: u16,
    page_y: u16,
    clut_x: u16,
    clut_y: u16,
    depth: TextureDepth,
    raw: bool,
    abr: u8,
}

#[derive(Clone, Copy)]
struct TexturedTriangle {
    positions: [Position; 3],
    colors: [Color; 3],
    uvs: [Uv; 3],
    texture: TextureAttributes,
    semi_transparent: bool,
}
