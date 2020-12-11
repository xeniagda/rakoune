use std::io::Result as IOResult;
use std::convert::TryInto;

use wgpu::{
    Buffer,
    BufferUsage,
};

use harfbuzz_rs::{
    Font as HBFont,
    Owned,
    GlyphPosition,
};

use rusttype::{
    Font as RTFont,
};

use crate::into_ioerror;
use crate::state::State;
use super::super::{RenderBackend, RichTexture};
use super::text_gpu_primitives::Vertex;

const FONT_SIZE_PX: f32 = 24.0; // For UV-rendering
const FONT_DATA: &[u8] = include_bytes!("../../../resources/firacode-regular.ttf");

struct Glyph<'a> {
    from_ref: &'a str,
    byte_span: std::ops::Range<usize>,

    position: GlyphPosition,
    glyph_id: u32,
}

impl <'a> Glyph<'a> {
    // TODO: Maybe pass a unicode buffer? Kinda cessary though...
    fn create_glyph_iter(text: &'a str, font: &HBFont) -> Vec<Glyph<'a>> {
        let unicode_buffer = harfbuzz_rs::UnicodeBuffer::new().add_str(text);
        let glyph_buffer = harfbuzz_rs::shape(font, unicode_buffer, &[]);

        let infos = glyph_buffer.get_glyph_infos();
        let mut spans = Vec::with_capacity(infos.len());

        for i in 0..infos.len() {
            let start = infos[i].cluster as usize;
            let next = infos.get(i+1).map(|x| x.cluster as usize).unwrap_or(text.len());
            spans.push(start..next);
        }

        let positions = glyph_buffer.get_glyph_positions();
        positions
            .iter()
            .enumerate()
            .map(|(i, &pos)| Glyph {
                from_ref: text,
                byte_span: spans[i].clone(),
                position: pos,
                glyph_id: infos[i].codepoint,
            })
            .collect()
    }

    fn get_content(&'a self) -> &'a str {
        &self.from_ref[self.byte_span.clone()]
    }
}

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

    pub(super) fn resize(&mut self, backend: &mut RenderBackend) -> IOResult<()> {
        self.text_rendered_cache = "".to_string();
        self.window_size = (backend.sc_desc.width as f32, backend.sc_desc.height as f32);

        Ok(())
    }

    pub(super) async fn upload(
        &mut self,
        backend: &mut RenderBackend,
        state: &State,
        glyph_vertex_buffer: &mut Buffer, // Let's just assume everything fits :)
        glyph_canvas: &mut RichTexture,
    ) -> IOResult<Option<u32>> {
        // if state.content == self.text_rendered_cache {
        //     return Ok(None);
        // }

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

        // h = harfbuzz, u = unit position for gpu
        let h2u_x = FONT_SIZE_PX * 2. / (self.hb_font.scale().1 as f32 * self.window_size.0);
        let h2u_y = FONT_SIZE_PX * 2. / (self.hb_font.scale().1 as f32 * self.window_size.1);

        let glyphs = Glyph::create_glyph_iter(&state.content, &self.hb_font);

        let mut current_xy_position: [f32; 2] = [0., 0., ];
        let mut current_u: usize = 0;
        let mut current_v: usize = 0;

        for glyph_info in glyphs {
            let gl_pos = glyph_info.position;
            let in_selection = state.cursor_range.contains(&glyph_info.byte_span.start);

            // Special case for newline
            // TODO: I'm not 100% sure how fonts handle newlines. For top-to-bottom fonts, should we step right?
            // Look this up and make a proper solution.

            if glyph_info.get_content() == "\n" {
                current_xy_position[0] = 0.;
                current_xy_position[1] += -self.hb_font.scale().1 as f32 * h2u_y; // negative = down
                continue;
            }

            let render_pos = [
                current_xy_position[0] + gl_pos.x_offset as f32 * h2u_x,
                current_xy_position[1] + gl_pos.y_offset as f32 * h2u_y,
            ];
            current_xy_position[0] += gl_pos.x_advance as f32 * h2u_x;
            current_xy_position[1] += gl_pos.y_advance as f32 * h2u_y;

            let ext = if let Some(ext) = self.hb_font.get_glyph_extents(glyph_info.glyph_id) {
                ext
            } else {
                continue;
            };


            let glyph = self.rt_font.glyph(rusttype::GlyphId(glyph_info.glyph_id.try_into().map_err(into_ioerror)?));
            let glyph = glyph.scaled(rusttype::Scale::uniform(FONT_SIZE_PX));
            let glyph = glyph.positioned(rusttype::Point { x: current_u as f32, y: current_v as f32 });
            let bounds = if let Some(pbb) = glyph.pixel_bounding_box() {
                pbb
            } else {
                continue;
            };

            let current_u_frac = current_u as f32 / glyph_canvas.extent.width as f32;
            let current_v_frac = current_v as f32 / glyph_canvas.extent.height as f32;
            let u_width_frac = bounds.width() as f32 / glyph_canvas.extent.width as f32;
            let v_height_frac = bounds.height() as f32 / glyph_canvas.extent.height as f32;

            if current_u_frac + u_width_frac > 1. || current_v_frac + v_height_frac > 1. {
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

            let width = bounds.width() as usize + 4; // 4 margin
            let height = bounds.height() as usize + 4;

            // Clear the area + margin
            for x in current_u..current_u+width {
                for y in current_v..current_v+height {
                    let i = 4 * (x + y * glyph_canvas.extent.width as usize);
                    for c in 0..4 {
                        canvas_buf_mapped.data[i + c] = 0;
                    }
                }
            }

            glyph.draw(|rx, ry, v| {
                let x = rx + current_u as u32;
                let y = ry + current_v as u32;

                let i = 4 * (x + y * glyph_canvas.extent.width) as usize;
                canvas_buf_mapped.data[i] = 255;
                canvas_buf_mapped.data[i + 1] = 255;
                canvas_buf_mapped.data[i + 2] = 255;
                if in_selection {
                    canvas_buf_mapped.data[i] = 0;
                }
                canvas_buf_mapped.data[i + 3] = (v * 255.) as u8;
            });

            current_u += width;
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
            let mapped_write_fut = glyph_vertex_buffer.map_write(0, raw_data.len() as u64);
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
