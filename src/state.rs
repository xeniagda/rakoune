use std::ops::Range;

pub struct State {
    pub content: String,
    pub cursor_range: Range<usize>,
}

#[derive(Debug)]
pub enum Key {
    Typed(char),
    Backspace,
}

fn char_index_before(st: &str, ch_idx: usize) -> Option<usize> {
    if ch_idx == 0 {
        return None;
    }
    for ch_byte_len in 1..ch_idx {
        if st.get(ch_idx - ch_byte_len .. ch_idx).is_some() {
            return Some(ch_idx - ch_byte_len);
        }
    }
    None
}

#[test]
fn test_char_index_before() {
    let st = "hellå wörld";
    let space_idx = 6;
    assert_eq!(st.get(space_idx..space_idx+1), Some(" "));

    assert_eq!(char_index_before(st, space_idx), Some(space_idx - 2)); // å is two bytes
    assert_eq!(char_index_before(st, space_idx - 2), Some(space_idx - 3)); // l is one byte
}

impl State {
    pub fn new() -> State {
        State {
            content: "ni li ilo pi pana sitelen".to_string(),
            cursor_range: 3..5,
        }
    }

    pub fn step(&mut self, dt: f32) {
        println!(
            "{}|{}|{}",
            &self.content[..self.cursor_range.start],
            &self.content[self.cursor_range.clone()],
            &self.content[self.cursor_range.end..]
        );
    }

    pub fn received_key(&mut self, key: Key) {
        match key {
            Key::Typed(ch) => {
                self.content.insert(self.cursor_range.start, ch);
                self.cursor_range.start += ch.len_utf8();
                self.cursor_range.end += ch.len_utf8();
            }
            Key::Backspace => {
                if let Some(idx_before) = char_index_before(&self.content, self.cursor_range.start) {
                    let removed = self.content.remove(idx_before);

                    self.cursor_range.start -= removed.len_utf8();
                    self.cursor_range.end -= removed.len_utf8();
                }
            }
        }
    }
}
