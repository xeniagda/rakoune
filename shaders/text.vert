#version 450

layout(location=0) in vec2 xy_pos;
layout(location=1) in vec2 uv_pos;

layout(location=0) out vec2 uv_out;

void main() {
    gl_Position = vec4(xy_pos, 0.0, 1.0);
    uv_out = uv_pos;
}
