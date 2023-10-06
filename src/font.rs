use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parsing ttf failed: {0:?}")]
    ParseFailTTF(#[from] ttf_parser::FaceParsingError),
    #[error("Parsing ttf failed: {0:?}")]
    ParseFailFontdue(String),
    #[error("Font index {0} out of range")]
    FontIndexOutOfRange(u32),
    #[error("Could not open font {0}: {1:?}")]
    CouldNotRead(PathBuf, std::io::Error),
}

pub struct FontStack {
    pub faces: Vec<Face>,
}

#[derive(Debug)]
pub struct ShapedCodepoint<'a> {
    pub face: &'a Face,
    pub glyph: u16,
    pub at: harfbuzz_rs::GlyphPosition,
}

impl FontStack {
    pub fn new(primary: &Path) -> Result<FontStack, Error> {
        Ok(FontStack {
            faces: Face::load_all_indices(primary)?
        })
    }

    pub fn add_fallback(&mut self, at: &Path) -> Result<(), Error> {
        self.faces.extend(Face::load_all_indices(at)?);
        Ok(())
    }

    pub fn add_face(&mut self, face: Face) {
        self.faces.push(face)
    }

    pub fn shape<'a>(&'a self, text: &str) -> Vec<(Option<ShapedCodepoint<'a>>, std::ops::Range<usize>)> {
        self.shape_with_index(text, 0, 0)
    }

    fn shape_with_index<'a>(&'a self, text: &str, text_offset: usize, font_index: usize) -> Vec<(Option<ShapedCodepoint<'a>>, std::ops::Range<usize>)> {
        let face = &self.faces[font_index];

        let buffer = harfbuzz_rs::UnicodeBuffer::new().add_str(text);
        let glyphbuf = harfbuzz_rs::shape(&face.hb_font, buffer, &[]);

        let mut shaped: Vec<_> = glyphbuf
            .get_glyph_infos()
            .into_iter()
            .zip(glyphbuf.get_glyph_positions().into_iter())
            .zip(glyphbuf.get_glyph_infos().into_iter().map(|x| x.cluster as usize).skip(1).chain(std::iter::once(text.len())))
            .map(
                |((info, pos), next)| if info.codepoint == 0 {
                    (None, text_offset + info.cluster as usize..text_offset + next)
                } else {
                    (Some(ShapedCodepoint {
                        face: face,
                        glyph: info.codepoint as u16,
                        at: pos.clone(),
                    }), text_offset + info.cluster as usize..text_offset + next)
                }
            )
            .collect();

        // no more fallback fonts
        if font_index == self.faces.len() - 1 {
            return shaped;
        }

        let mut out = Vec::with_capacity(shaped.len());
        let mut unshaped_start: Option<usize> = None;
        for (shape, range) in shaped.drain(..) {
            if shape.is_some() {
                if let Some(start) = unshaped_start.take() {
                    let unshaped_subsequence = &text[start - text_offset..range.start-text_offset];
                    let shaped = self.shape_with_index(unshaped_subsequence, start, font_index + 1);
                    out.extend(shaped);
                }
                out.push((shape, range));
            } else {
                if unshaped_start.is_none() {
                    unshaped_start = Some(range.start);
                }
            }
        }
        if let Some(start) = unshaped_start.take() {
            let unshaped_subsequence = &text[start-text_offset..];
            let shaped = self.shape_with_index(unshaped_subsequence, start, font_index + 1);
            out.extend(shaped);
        }
        out
    }
}

pub struct Face {
    pub name: String,
    pub hb_font: harfbuzz_rs::Owned<harfbuzz_rs::Font<'static>>, // TODO: Proper memory management :3
    pub fontdue_font: fontdue::Font,
    pub ttf_face: ttf_parser::Face<'static>,
    pub n_glyphs: u16,
    pub italic: bool,
    pub bold: bool,
}

impl std::fmt::Debug for Face {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Face")
            .field("name", &self.name)
            .field("n_glyphs", &self.n_glyphs)
            .field("italic", &self.italic)
            .field("bold", &self.bold)
            .finish_non_exhaustive()
    }
}

impl Face {
    pub fn load_all_indices(at: &Path) -> Result<Vec<Face>, Error> {
        use std::io::Read;
        let mut f = std::fs::File::open(at).map_err(|e| Error::CouldNotRead(at.to_owned(), e))?;
        let mut data = Vec::new();
        f.read_to_end(&mut data).map_err(|e| Error::CouldNotRead(at.to_owned(), e))?;
        let static_data: &'static [u8] = data.leak(); // :3

        let mut faces = Vec::new();
        for i in 0.. {
            match Face::from_data_index(static_data, i) {
                Ok(f) => faces.push(f),
                Err(Error::FontIndexOutOfRange(_)) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(faces)
    }

    pub fn from_data_index(data: &'static [u8], index: u32) -> Result<Face, Error> {
        let ttf_face = match ttf_parser::Face::parse(data, index) {
            Ok(x) => x,
            Err(ttf_parser::FaceParsingError::FaceIndexOutOfBounds) => return Err(Error::FontIndexOutOfRange(index)),
            Err(e) => Err(e)?,
        };
        let italic = ttf_face.is_italic();
        let bold = ttf_face.is_bold();
        let n_glyphs = ttf_face.number_of_glyphs();

        let name = get_name_by_id(&ttf_face, NAME_ID_FULL_NAME).unwrap_or("(unknown name)".to_string());

        let hb_font = harfbuzz_rs::Font::new(harfbuzz_rs::Face::from_bytes(data, index));
        let fontdue_font = fontdue::Font::from_bytes(
            data,
            fontdue::FontSettings {
                collection_index: index,
                ..Default::default()
            }
        ).map_err(|e| Error::ParseFailFontdue(e.to_string()))?;

        Ok(Face {
            name,
            hb_font, fontdue_font, ttf_face,
            italic, bold, n_glyphs,
        })
    }
}

#[allow(unused)]
pub const NAME_ID_FAMILY_NAME: u16 = 1;
#[allow(unused)]
pub const NAME_ID_SUBFAMILY_NAME: u16 = 2;
#[allow(unused)]
pub const NAME_ID_UNIQUE_NAME: u16 = 3;
#[allow(unused)]
pub const NAME_ID_FULL_NAME: u16 = 4;

// Get the field, preferring English
pub fn get_name_by_id(ttf_face: &ttf_parser::Face, id: u16) -> Option<String> {
    fn get_name(name: ttf_parser::name::Name) -> Option<String> {
        if let Some(x) = name.to_string() {
            Some(x)
        } else if let Ok(x) = String::from_utf8(name.name.to_vec()) {
            Some(x)
        } else {
            None
        }
    }
    for name in ttf_face.names().into_iter() {
        if name.name_id == id && name.language().primary_language() == "English" {
            if let Some(text) = get_name(name) {
                return Some(text)
            }
        }
    }
    // Try again, not checking for language
    for name in ttf_face.names().into_iter() {
        if name.name_id == id {
            if let Some(text) = get_name(name) {
                return Some(text)
            }
        }
    }
    None
}
