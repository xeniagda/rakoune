use std::io::Read;
use std::convert::TryInto;
use std::collections::HashMap;

use harfbuzz_rs::{Face as HBFace, Font as HBFont, UnicodeBuffer, shape};
use rusttype::{Font as RTFont, Scale, GlyphId, Point};

const ASCII_GRAD: &str = " '!M#";

fn other_error(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, msg)
}

fn main() -> std::io::Result<()> {
    let path = "/Users/jonathanloov/Library/Fonts/FiraCode-Regular.ttf";
    // let path = "/Users/jonathanloov/Library/Fonts/linja-pona-4.1.otf";

    let index = 0; //< face index in the font file
    let face = HBFace::from_file(path, index)?;
    let mut hb_font = HBFont::new(face);

    let buffer = UnicodeBuffer::new().add_str(">>=<-");
    let output = shape(&hb_font, buffer, &[]);

    let (width, height) = hb_font.scale();
    let (width, height) = (width as f32, height as f32);

    println!("Scale: {:?}", (width, height));

    let wanted_font_height = 30.0;

    let wanted_font_width = width * wanted_font_height / height;

    // The results of the shaping operation are stored in the `output` buffer.

    let mut f = std::fs::File::open(path)?;
    let mut data_vec = Vec::new();
    f.read_to_end(&mut data_vec)?;
    let rt_font = RTFont::try_from_vec(data_vec)
        .ok_or(other_error("RT could not parse font"))?;

    let positions = output.get_glyph_positions();
    let infos = output.get_glyph_infos();

    assert_eq!(positions.len(), infos.len());


    let mut at_x = wanted_font_width; // Some margin
    let mut at_y = wanted_font_height;

    let mut out_pixels: HashMap<(i32, i32), f32> = HashMap::new();

    // iterate over the shaped glyphs
    for (position, info) in positions.iter().zip(infos) {
        let gid = info.codepoint;
        let cluster = info.cluster;

        let x_advance = position.x_advance as f32 / width * wanted_font_width;
        let y_advance = position.y_advance as f32 / height * wanted_font_height;

        let x_offset = position.x_offset as f32 / width * wanted_font_width;
        let y_offset = position.y_offset as f32 / height * wanted_font_height;

        println!("({:.3}, {:.3}) += ({:.3}, {:.3}) + ({:.3}, {:.3}): gid{:?} = {:?}", at_x, at_y, x_advance, y_advance, x_offset, y_offset, gid, hb_font.get_glyph_name(gid));

        if let Some(extents) = hb_font.get_glyph_extents(gid) {
            println!("{:?}", extents);
            let x_bearing = extents.x_bearing as f32 / width * wanted_font_width;
            let y_bearing = extents.y_bearing as f32 / height * wanted_font_height;

            let x_size = extents.width as f32 / width * wanted_font_width;
            let y_size = extents.height as f32 / height * wanted_font_height;

            println!("bearing = ({:.3}, {:.3})", x_bearing, y_bearing);
            println!("size = ({:.3}, {:.3})", x_size, y_size);

            let glyph_id = GlyphId(gid.try_into().map_err(|_| other_error("GID too large!"))?);
            let glyph = rt_font.glyph(glyph_id);
            let glyph = glyph.scaled(Scale::uniform(wanted_font_height as f32));

            let pos = Point { x: at_x + x_offset, y: at_y + y_offset};
            println!("Rendering at {:?}", pos);
            let glyph = glyph.positioned(pos);
            println!("{:?}", glyph);
            println!("bounding: {:?}", glyph.pixel_bounding_box());

            if let Some(bb) = glyph.pixel_bounding_box() {
                glyph.draw(|rx, ry, v| {
                    let x = rx as i32 + bb.min.x;
                    let y = ry as i32 + bb.min.y;
                    let brightness: &mut f32 = out_pixels.entry((x, y)).or_insert(0.);
                    *brightness = 1. - (1. - *brightness) * (1. - v);
                });
            }
        }

        at_x += x_advance;
        at_y += y_advance;
    }

    let min_x = out_pixels.keys().map(|&(x, _y)| x).min().unwrap_or(0);
    let max_x = out_pixels.keys().map(|&(x, _y)| x).max().unwrap_or(0);

    let min_y = out_pixels.keys().map(|&(_x, y)| y).min().unwrap_or(0);
    let max_y = out_pixels.keys().map(|&(_x, y)| y).max().unwrap_or(0);

    println!("({}, {}) - ({}, {})", min_x, min_y, max_x, max_y);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let here = out_pixels.entry((x, y)).or_insert(0.);
            let grad_idx = *here * (ASCII_GRAD.len() as f32 - 1.);
            let grad_idx = grad_idx as usize;
            print!("{}", ASCII_GRAD.chars().nth(grad_idx).unwrap());
        }
        println!();
    }

    Ok(())
}
