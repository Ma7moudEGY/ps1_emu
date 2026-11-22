#version 330 core
in vec2 uv;
uniform sampler2D vram;
uniform vec2 src_offset;
uniform vec2 src_size;
out vec4 fragcolor;
void main() {
    fragcolor = texture(vram, (src_offset + uv * src_size) / vec2(1024.0, 512.0));
}