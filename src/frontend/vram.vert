#version 330 core
layout(location = 0) in vec2 position;
out vec2 uv;
void main() {
    uv = vec2(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
    gl_Position = vec4(position, 0.0, 1.0);
}