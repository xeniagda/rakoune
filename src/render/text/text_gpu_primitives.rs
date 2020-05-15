use std::mem::size_of;

use wgpu::{
    VertexBufferDescriptor,
    VertexAttributeDescriptor,
    VertexFormat,
};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 2],
    pub fontdata_uv: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub fn create_quad(xy_0: [f32; 2], xy_1: [f32; 2], uv_0: [f32; 2], uv_1: [f32; 2]) -> [Vertex; 6] {
        let tl = Vertex { position: [xy_0[0], xy_0[1]], fontdata_uv: [uv_0[0], uv_0[1]]};
        let tr = Vertex { position: [xy_1[0], xy_0[1]], fontdata_uv: [uv_1[0], uv_0[1]]};
        let bl = Vertex { position: [xy_0[0], xy_1[1]], fontdata_uv: [uv_0[0], uv_1[1]]};
        let br = Vertex { position: [xy_1[0], xy_1[1]], fontdata_uv: [uv_1[0], uv_1[1]]};

        [
            tl, bl, tr,
            br, tr, bl,
        ]
    }

    pub fn desc<'a>() -> VertexBufferDescriptor<'a> {
        VertexBufferDescriptor {
            stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                VertexAttributeDescriptor { // position: [f32; 2]
                    offset: 0,
                    format: VertexFormat::Float2,
                    shader_location: 0,
                },
                VertexAttributeDescriptor { // fontdata_uv: [f32; 2]
                    offset: size_of::<[f32; 2]>() as u64,
                    format: VertexFormat::Float2,
                    shader_location: 1,
                },
            ],
        }
    }
}
