//! The font collection type.

use std::ops::Range;
use std::sync::Arc;

use crate::Font;

/// A collection of fonts
pub struct FontCollection {
    families: Vec<FontFamily>,
}

pub struct FontFamily {
    // TODO: multiple weights etc
    fonts: Vec<FontRef>,
}

#[derive(Clone)]
pub struct FontRef {
    font: Arc<Font>,
}

pub struct Itemizer<'a> {
    text: &'a str,
    collection: &'a FontCollection,
    ix: usize,
}

impl FontFamily {
    pub fn new() -> FontFamily {
        FontFamily {
            fonts: Vec::new(),
        }
    }

    pub fn add_font(&mut self, font: Font) {
        let font_ref = FontRef { font: Arc::new(font) };
        self.fonts.push(font_ref);
    }

    pub fn supports_codepoint(&self, c: char) -> bool {
        if let Some(font) = self.fonts.first() {
            font.font.glyph_for_char(c).is_some()
        } else {
            false
        }
    }
}

impl FontCollection {
    pub fn new() -> FontCollection {
        FontCollection {
            families: Vec::new(),
        }
    }

    pub fn add_family(&mut self, family: FontFamily) {
        self.families.push(family);
    }

    pub fn itemize<'a>(&'a self, text: &'a str) -> Itemizer<'a> {
        Itemizer {
            text,
            collection: self,
            ix: 0,
        }
    }

    // TODO: other style params, including locale list
    fn choose_font(&self, c: char) -> usize {
        self.families.iter().position(|family| family.supports_codepoint(c)).unwrap_or(0)
    }
}

impl<'a> Iterator for Itemizer<'a> {
    type Item = (Range<usize>, FontRef);

    fn next(&mut self) -> Option<(Range<usize>, FontRef)> {
        let start = self.ix;
        let mut chars_iter = self.text[start..].chars();
        if let Some(c) = chars_iter.next() {
            let mut end = start + c.len_utf8();
            let font_ix = self.collection.choose_font(c);
            while let Some(c) = chars_iter.next() {
                if font_ix != self.collection.choose_font(c) {
                    break;
                }
                end += c.len_utf8();
            }
            self.ix = end;
            Some((start..end, self.collection.families[font_ix].fonts[0].clone()))
        } else {
            None
        }
    }
}
