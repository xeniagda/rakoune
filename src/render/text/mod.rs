use std::io::{Result as IOResult, Cursor};

use wgpu::{
    Buffer,
};

use winit::dpi::PhysicalSize;

use super::RenderBackend;
use crate::into_ioerror;
use crate::state::State;

mod text_gpu_primitives;

use text_gpu_primitives::Vertex;

pub(super) struct TextRenderer {
}

impl TextRenderer {
    pub async fn new(backend: &mut RenderBackend) -> IOResult<Self> {
        Ok(Self {
        })
    }

    pub fn resize(&mut self, backend: &mut RenderBackend, into_size: PhysicalSize<u32>) -> IOResult<()> {
        Ok(())
    }

    pub fn render(&mut self, backend: &mut RenderBackend, to_view: &wgpu::TextureView, state: &State) -> IOResult<wgpu::CommandBuffer> {
        let mut encoder = backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Text render encoder"),
            }
        );

        Ok(encoder.finish())
    }
}

