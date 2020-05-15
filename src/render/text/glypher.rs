use std::io::Result as IOResult;

use wgpu::{
    Buffer,
    Texture,
    Extent3d,
    BufferUsage,
};

use harfbuzz_rs::{
    Font as HBFont,
    Owned,
};

use rusttype::{
    Font as RTFont,
};

use crate::into_ioerror;
use crate::state::State;
use super::super::RenderBackend;
use super::text_gpu_primitives::Vertex;

const FONT_SIZE_PX: f32 = 40.0;
const FONT_DATA: &[u8] = include_bytes!("../../../resources/linja-pona-4.1.otf");

pub struct Glypher {
    hb_font: Owned<HBFont<'static>>,
    rt_font: RTFont<'static>, // TODO: Change this to support dynamic fonts
}

impl Glypher {
    pub fn new() -> IOResult<Self> {
        let rt_font = RTFont::try_from_bytes(FONT_DATA).ok_or(into_ioerror("Invalid font data!"))?;

        let hb_face = harfbuzz_rs::Face::from_bytes(FONT_DATA, 0);
        let hb_font = HBFont::new(hb_face);

        Ok(Self {
            hb_font,
            rt_font,
        })
    }

    pub(super) async fn upload(
        &self,
        backend: &mut RenderBackend,
        state: &State,
        glyph_vertex_buffer: &mut Buffer, // Let's just assume everything fits :)
        (glyph_canvas, extent): (&mut Texture, Extent3d),
    ) -> IOResult<u32> {
        // Bogus vertex data
        let verticies = vec![
            Vertex { position: [ 0., 0., ], fontdata_uv: [ 0., 0., ] },
            Vertex { position: [ -1., 1., ], fontdata_uv: [ 1., 1., ] },
            Vertex { position: [ -1., 0., ], fontdata_uv: [ 1., 0., ] },
        ];

        let raw_data: &[u8] = bytemuck::cast_slice(&verticies);

        let mut mapped_write_fut = glyph_vertex_buffer.map_write(0, raw_data.len() as u64);
        backend.device.poll(wgpu::Maintain::Wait);
        let mut mapped_write = mapped_write_fut.await.map_err(|_| into_ioerror("sync error"))?;

        mapped_write.as_slice().copy_from_slice(raw_data);

        glyph_vertex_buffer.unmap();
        backend.device.poll(wgpu::Maintain::Poll);

        // Bogus texture stuff
        let staging_buffer_mapped = backend.device.create_buffer_mapped(
            &wgpu::BufferDescriptor {
                label: Some("bogus texture data"),
                size: 512 * 512 * 4,
                usage: BufferUsage::COPY_SRC,
            },
        );
        for i in 0..(extent.width * extent.height * 4) as usize {
            staging_buffer_mapped.data[i] = (255 * ((i / 4) % 2)) as u8;
        }

        let buf = staging_buffer_mapped.finish();

        let mut encoder = backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Texture upload encoder"),
            }
        );
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &buf,
                offset: 0,
                bytes_per_row: 512 * 4,
                rows_per_image: 512,
            },
            wgpu::TextureCopyView {
                texture: &glyph_canvas,
                mip_level: 0,
                array_layer: 0,
                origin: Default::default(),
            },
            wgpu::Extent3d { width: 512, height: 512, depth: 1 },
        );
        backend.queue.submit(&[encoder.finish()]);

        Ok(3)
    }
}
