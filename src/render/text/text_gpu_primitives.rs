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
