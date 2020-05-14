#version 450

layout(location=0) in vec2 uv_pos;

layout(location=0) out vec4 frag_color;

layout(set=0, binding=0) uniform texture2D logo_texture;
layout(set=0, binding=1) uniform sampler logo_sampler;

void main() {
    frag_color = texture(sampler2D(logo_texture, logo_sampler), uv_pos);
}
