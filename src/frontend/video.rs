use std::{error::Error, num::NonZeroU32};

use glow::HasContext;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{
        ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext,
        PossiblyCurrentContext, Version,
    },
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SwapInterval, WindowSurface},
};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use winit::{dpi::PhysicalSize, event_loop::ActiveEventLoop, window::Window};

pub struct Renderer {
    pub window: Window,
    pub surface: Surface<WindowSurface>,
    pub context: PossiblyCurrentContext,
    pub gl: glow::Context,
    program: glow::Program,
    vao: glow::VertexArray,
    position_buffer: glow::Buffer,
    color_buffer: glow::Buffer,
    vram_program: glow::Program,
    vram_vao: glow::VertexArray,
    vram_texture: glow::Texture,
    vram_buffer: glow::Buffer,
    vram_location: Option<glow::UniformLocation>,
    src_offset_location: Option<glow::UniformLocation>,
    src_size_location: Option<glow::UniformLocation>,
}

impl Renderer {
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Renderer, Box<dyn Error>> {
        let window_attr = Window::default_attributes()
            .with_title("PSX")
            .with_inner_size(PhysicalSize::new(1024, 512));

        let (window, gl_config) = DisplayBuilder::new()
            .with_window_attributes(Some(window_attr))
            .build(event_loop, ConfigTemplateBuilder::new(), |mut configs| {
                configs.next().expect("no_opengl_config_available")
            })?;

        let window = window.expect("no_window_created");

        let raw_window_handle = window.window_handle()?.as_raw();

        let context_attr = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .with_profile(GlProfile::Core)
            .build(Some(raw_window_handle));

        let gl_display = gl_config.display();

        let not_current_context = unsafe { gl_display.create_context(&gl_config, &context_attr)? };

        let surface_attr = window.build_surface_attributes(Default::default())?;

        let surface = unsafe { gl_display.create_window_surface(&gl_config, &surface_attr)? };

        let context = not_current_context.make_current(&surface)?;

        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|s| gl_display.get_proc_address(s).cast())
        };

        surface
            .set_swap_interval(&context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
            .expect("failed_to_enable_vsync");

        let vertex_shader = compile_shader(&gl, include_str!("vertex.glsl"), glow::VERTEX_SHADER);
        let fragment_shader =
            compile_shader(&gl, include_str!("fragment.glsl"), glow::FRAGMENT_SHADER);

        let program = link_program(&gl, &([vertex_shader, fragment_shader]));

        let vao;
        let position_buffer;
        let color_buffer;

        unsafe {
            gl.detach_shader(program, vertex_shader);
            gl.delete_shader(vertex_shader);
            gl.detach_shader(program, fragment_shader);
            gl.delete_shader(fragment_shader);

            gl.use_program(Some(program));

            vao = gl.create_vertex_array().expect("failed_to_create_vao");
            gl.bind_vertex_array(Some(vao));

            position_buffer = gl
                .create_buffer()
                .expect("failed_to_create_position_buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buffer));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_i32(0, 2, glow::SHORT, 0, 0);

            color_buffer = gl.create_buffer().expect("failed_to_create_color_buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(color_buffer));
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_i32(1, 3, glow::UNSIGNED_BYTE, 0, 0);

            let size = window.inner_size();
            gl.viewport(0, 0, size.width as i32, size.height as i32);
        }

        let vram_vertex = compile_shader(&gl, include_str!("vram.vert"), glow::VERTEX_SHADER);
        let vram_fragment = compile_shader(&gl, include_str!("vram.frag"), glow::FRAGMENT_SHADER);
        let vram_program = link_program(&gl, &[vram_vertex, vram_fragment]);

        let vram_vao;
        let vram_buffer;
        let vram_texture;
        let vram_location;
        let src_offset_location;
        let src_size_location;

        unsafe {
            gl.detach_shader(vram_program, vram_vertex);
            gl.delete_shader(vram_vertex);
            gl.detach_shader(vram_program, vram_fragment);
            gl.delete_shader(vram_fragment);

            gl.use_program(Some(vram_program));

            vram_location = gl.get_uniform_location(vram_program, "vram");
            src_offset_location = gl.get_uniform_location(vram_program, "src_offset");
            src_size_location = gl.get_uniform_location(vram_program, "src_size");
            gl.uniform_1_i32(vram_location.as_ref(), 0);

            vram_vao = gl.create_vertex_array().expect("failed_to_create_vram_vao");
            gl.bind_vertex_array(Some(vram_vao));

            vram_buffer = gl.create_buffer().expect("failed_to_create_vram_buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vram_buffer));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);

            let quad: [f32; 12] = [
                -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
            ];
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&quad), glow::STATIC_DRAW);

            vram_texture = gl.create_texture().expect("failed_to_create_vram_texture");
            gl.bind_texture(glow::TEXTURE_2D, Some(vram_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                1024,
                512,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
        }

        unsafe {
            let size = window.inner_size();
            gl.viewport(0, 0, size.width as i32, size.height as i32);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        Ok(Renderer {
            window,
            surface,
            context,
            gl,
            program,
            vao,
            position_buffer,
            color_buffer,
            vram_program,
            vram_vao,
            vram_buffer,
            vram_texture,
            vram_location,
            src_offset_location,
            src_size_location,
        })
    }

    pub fn display(&mut self, triangles: &[Triangle]) {
        let mut positions = Vec::with_capacity(triangles.len() * 3);
        let mut colors = Vec::with_capacity(triangles.len() * 3);

        for triangle in triangles {
            positions.extend_from_slice(&triangle.positions);
            colors.extend_from_slice(&triangle.colors);
        }

        unsafe {
            self.gl.use_program(Some(self.program));
            self.gl.bind_vertex_array(Some(self.vao));

            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.position_buffer));
            self.gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                as_bytes(&positions),
                glow::STREAM_DRAW,
            );

            self.gl
                .bind_buffer(glow::ARRAY_BUFFER, Some(self.color_buffer));
            self.gl
                .buffer_data_u8_slice(glow::ARRAY_BUFFER, as_bytes(&colors), glow::STREAM_DRAW);

            self.gl
                .draw_arrays(glow::TRIANGLES, 0, positions.len() as i32);
        }

        self.present();
    }

    pub fn resize(&self) {
        self.window.resize_surface(&self.surface, &self.context);

        let size = self.window.inner_size();
        unsafe {
            self.gl
                .viewport(0, 0, size.width as i32, size.height as i32);
        }
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn display_vram(&mut self, vram: &[u16], src_x: u16, src_y: u16, w: u32, h: u32) {
        let mut rgba = vec![0u8; 1024 * 512 * 4];
        for (i, &p) in vram.iter().enumerate() {
            let o = i * 4;
            rgba[o] = ((p & 0x1f) << 3) as u8;
            rgba[o + 1] = (((p >> 5) & 0x1f) << 3) as u8;
            rgba[o + 2] = (((p >> 10) & 0x1f) << 3) as u8;
            rgba[o + 3] = 255;
        }

        unsafe {
            self.gl
                .bind_texture(glow::TEXTURE_2D, Some(self.vram_texture));
            self.gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                1024,
                512,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&rgba)),
            );

            self.gl.use_program(Some(self.vram_program));
            self.gl.bind_vertex_array(Some(self.vram_vao));
            self.gl.uniform_2_f32(
                self.src_offset_location.as_ref(),
                src_x as f32,
                src_y as f32,
            );
            self.gl
                .uniform_2_f32(self.src_size_location.as_ref(), w as f32, h as f32);
            self.gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }

        self.present();
    }

    fn present(&self) {
        self.window.pre_present_notify();
        self.surface
            .swap_buffers(&self.context)
            .expect("failed_to_swap_buffers");
    }
}

fn compile_shader(gl: &glow::Context, src: &str, kind: u32) -> glow::Shader {
    unsafe {
        let shader = gl.create_shader(kind).expect("failed_to_create_shader");

        gl.shader_source(shader, src);
        gl.compile_shader(shader);

        if !gl.get_shader_compile_status(shader) {
            panic!(
                "shader_compilation_failed:\n{}",
                gl.get_shader_info_log(shader)
            );
        }

        shader
    }
}

fn link_program(gl: &glow::Context, shaders: &[glow::Shader]) -> glow::Program {
    unsafe {
        let program = gl.create_program().expect("failed_to_create_program");

        for &shader in shaders {
            gl.attach_shader(program, shader);
        }

        gl.link_program(program);

        if !gl.get_program_link_status(program) {
            panic!(
                "program_linking_failed:\n{}",
                gl.get_program_info_log(program)
            );
        }

        program
    }
}

fn as_bytes<T>(data: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn from_gp0(val: u32) -> Self {
        let r = val as u8;
        let g = (val >> 8) as u8;
        let b = (val >> 16) as u8;

        Color { r, g, b }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

impl Position {
    pub fn from_gp0(val: u32) -> Self {
        let x = val as i16;
        let y = (val >> 16) as i16;

        Position { x, y }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub positions: [Position; 3],
    pub colors: [Color; 3],
}
