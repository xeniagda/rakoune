#version 450

layout(set=0, binding=0) uniform Uniform { uint screen_width; uint screen_height; };

layout(location=0) out vec2 uv_pos;

const float LOGO_SIZE = 200.0;

const vec2 uvs[6] = vec2[6](
    vec2(0., 0.),
    vec2(1., 0.),
    vec2(0., 1.),
    vec2(0., 1.),
    vec2(1., 0.),
    vec2(1., 1.)
);

void main() {
    float one_h_pixel = 0.5 / screen_width;
    float one_v_pixel = 0.5 / screen_height;

    vec2 point = vec2(0., 0.);

    if (gl_VertexIndex == 0) {
        point = vec2(-1., 1.);
    }
    if (gl_VertexIndex == 1 || gl_VertexIndex == 4) {
        point = vec2(-1., 1. - LOGO_SIZE * one_v_pixel);
    }
    if (gl_VertexIndex == 2 || gl_VertexIndex == 3) {
        point = vec2(-1. + LOGO_SIZE * one_h_pixel, 1.);
    }
    if (gl_VertexIndex == 5) {
        point = vec2(-1. + LOGO_SIZE * one_h_pixel, 1. - LOGO_SIZE * one_v_pixel);
    }
    uv_pos = uvs[gl_VertexIndex];
    gl_Position = vec4(point, 0., 1.);
}
