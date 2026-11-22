#version 330 core

layout(location = 0) in ivec2 vertex_position;
layout(location = 1) in uvec3 vertex_color;

out vec3 color;

void main() {
    float xpos = (float(vertex_position.x) / 512.0) - 1.0;
    float ypos = 1.0 - (float(vertex_position.y) / 256.0);

    gl_Position = vec4(xpos, ypos, 0.0, 1.0);

    color = vec3(float(vertex_color.r) / 255.0,
                 float(vertex_color.g) / 255.0,
                 float(vertex_color.b) / 255.0);
}