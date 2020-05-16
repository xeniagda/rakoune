use std::io::Result as IOResult;
use std::convert::TryInto;

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
use super::super::{RenderBackend, RichTexture};
use super::text_gpu_primitives::Vertex;

const FONT_SIZE_PX: f32 = 40.0; // For UV-rendering
const FONT_DATA: &[u8] = include_bytes!("../../../resources/linja-pona-4.1.otf");

pub struct Glypher {
    hb_font: Owned<HBFont<'static>>,
    rt_font: RTFont<'static>, // TODO: Change this to support dynamic fonts
    text_rendered_cache: String,
    window_size: (f32, f32),
}

impl Glypher {
    pub fn new() -> IOResult<Self> {
        let rt_font = RTFont::try_from_bytes(FONT_DATA).ok_or(into_ioerror("Invalid font data!"))?;

        let hb_face = harfbuzz_rs::Face::from_bytes(FONT_DATA, 0);
        let hb_font = HBFont::new(hb_face);

        Ok(Self {
            hb_font,
            rt_font,
            text_rendered_cache: "".to_string(),
            window_size: (1., 1.),
        })
    }

    pub(super) fn resize(&mut self, backend: &mut RenderBackend, into_size: winit::dpi::PhysicalSize<u32>) -> IOResult<()> {
        self.text_rendered_cache = "".to_string();
        self.window_size = (into_size.width as f32, into_size.height as f32);

        Ok(())
    }

    pub(super) async fn upload(
        &mut self,
        backend: &mut RenderBackend,
        state: &State,
        glyph_vertex_buffer: &mut Buffer, // Let's just assume everything fits :)
        glyph_canvas: &mut RichTexture,
    ) -> IOResult<Option<u32>> {
        if state.content == self.text_rendered_cache {
            return Ok(None);
        }

        let mut encoder = backend.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Texture upload encoder"),
            }
        );

        let canvas_buf_mapped = backend.device.create_buffer_mapped(
            &wgpu::BufferDescriptor {
                label: Some("Canvas staging buffer"),
                size: (glyph_canvas.extent.width * glyph_canvas.extent.height * 4) as u64,
                usage: BufferUsage::COPY_SRC,
            },
        );

        // Render text
        let mut verticies: Vec<Vertex> = Vec::new();

        let h2px = FONT_SIZE_PX / self.hb_font.scale().1 as f32;
        let h2u_x = FONT_SIZE_PX * 2. / (self.hb_font.scale().1 as f32 * self.window_size.0);
        let h2u_y = FONT_SIZE_PX * 2. / (self.hb_font.scale().1 as f32 * self.window_size.1);

        let uni_buf =
            harfbuzz_rs::UnicodeBuffer::new()
            .add_str(&state.content);

        let mut current_xy_position: [f32; 2] = [0., 0., ];
        let mut current_u: usize = 0;
        let mut current_v: usize = 0;

        let glyph_buffer = harfbuzz_rs::shape(&self.hb_font, uni_buf, &[]);
        for (&gl_info, &gl_pos) in glyph_buffer.get_glyph_infos().iter().zip(glyph_buffer.get_glyph_positions().iter()) {
            let render_pos = [
                current_xy_position[0] + gl_pos.x_offset as f32 * h2u_x,
                current_xy_position[1] + gl_pos.y_offset as f32 * h2u_y,
            ];
            current_xy_position[0] += gl_pos.x_advance as f32 * h2u_x;
            current_xy_position[1] += gl_pos.y_advance as f32 * h2u_y;

            let ext = if let Some(ext) = self.hb_font.get_glyph_extents(gl_info.codepoint) {
                ext
            } else {
                eprintln!("No extents found for g{} = {:?}", gl_info.codepoint, self.hb_font.get_glyph_name(gl_info.codepoint));
                continue;
            };


            eprintln!("Rendering {:?} @ ({:.3}, {:.3})", self.hb_font.get_glyph_name(gl_info.codepoint), render_pos[0], render_pos[1]);

            let glyph = self.rt_font.glyph(rusttype::GlyphId(gl_info.codepoint.try_into().map_err(into_ioerror)?));
            let glyph = glyph.scaled(rusttype::Scale::uniform(FONT_SIZE_PX));
            let glyph = glyph.positioned(rusttype::Point { x: current_u as f32, y: current_v as f32 });
            let bounds = if let Some(pbb) = glyph.pixel_bounding_box() {
                pbb
            } else {
                eprintln!("No bounding box found for g{} = {:?}", gl_info.codepoint, self.hb_font.get_glyph_name(gl_info.codepoint));
                continue;
            };

            let current_u_frac = current_u as f32 / glyph_canvas.extent.width as f32;
            let current_v_frac = current_v as f32 / glyph_canvas.extent.height as f32;
            let u_width_frac = bounds.width() as f32 / glyph_canvas.extent.width as f32;
            let v_height_frac = bounds.height() as f32 / glyph_canvas.extent.height as f32;

            if current_u_frac + u_width_frac > 1. || current_v_frac + v_height_frac > 1. {
                eprintln!("Doesn't fit!");
                break;
            }

            let x_bearing = ext.x_bearing as f32 * h2u_x;
            let y_bearing = ext.y_bearing as f32 * h2u_y;
            let ext_width = ext.width as f32 * h2u_x;
            let ext_height = ext.height as f32 * h2u_y;

            verticies.extend(
                &Vertex::create_quad(
                    [render_pos[0] + x_bearing, render_pos[1] + y_bearing],
                    [render_pos[0] + x_bearing + ext_width, render_pos[1] + y_bearing + ext_height],
                    [current_u_frac, current_v_frac],
                    [current_u_frac + u_width_frac, current_v_frac + v_height_frac],
                ),
            );

            eprintln!("Placing at UV {:?} .. {:?}", bounds.min, bounds.max);

            glyph.draw(|rx, ry, v| {
                let x = rx + current_u as u32;
                let y = ry + current_v as u32;

                let i = 4 * (x + y * glyph_canvas.extent.width) as usize;
                canvas_buf_mapped.data[i] = 255;
                canvas_buf_mapped.data[i + 1] = 255;
                canvas_buf_mapped.data[i + 2] = 255;
                canvas_buf_mapped.data[i + 3] = (v * 255.) as u8;
            });

            current_u = (current_u as isize + bounds.width() as isize) as usize;
            current_u += 1;
        }

        let canvas_buf = canvas_buf_mapped.finish();

        // Upload UV-canvas
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &canvas_buf,
                offset: 0,
                bytes_per_row: glyph_canvas.extent.width * 4,
                rows_per_image: glyph_canvas.extent.height,
            },
            wgpu::TextureCopyView {
                texture: &glyph_canvas.content,
                mip_level: 0,
                array_layer: 0,
                origin: Default::default(),
            },
            glyph_canvas.extent,
        );

        // Upload vertex data
        let raw_data: &[u8] = bytemuck::cast_slice(&verticies);

        if raw_data.len() != 0 {
            let mut mapped_write_fut = glyph_vertex_buffer.map_write(0, raw_data.len() as u64);
            backend.device.poll(wgpu::Maintain::Wait);
            let mut mapped_write = mapped_write_fut.await.map_err(|_| into_ioerror("Write sync error"))?;

            mapped_write.as_slice().copy_from_slice(raw_data);

            glyph_vertex_buffer.unmap();
        }

        backend.queue.submit(&[encoder.finish()]);

        self.text_rendered_cache = state.content.clone();

        Ok(Some(verticies.len() as u32))
    }
}
