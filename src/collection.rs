//! The font collection type.

use crate::Font;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

/// A collection of fonts
pub struct FontCollection {
    pub(crate) families: Vec<FontFamily>,
}

pub struct FontFamily {
    // TODO: multiple weights etc
    pub(crate) fonts: Vec<FontRef>,
}

// Design question: deref to Font?
#[derive(Clone)]
pub struct FontRef {
    pub font: Arc<Font>,
    pub location: HashMap<[u8; 4], f32>,
}

impl fmt::Debug for FontRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FontRef({})", self.font.full_name())
    }
}

pub struct Itemizer<'a> {
    text: &'a str,
    collection: &'a FontCollection,
    ix: usize,
}

impl FontRef {
    pub fn new(font: Font) -> FontRef {
        FontRef {
            font: Arc::new(font),
            location: HashMap::new(),
        }
    }

    pub fn set_axis_location(&mut self, tag: &str, location: f32) -> bool {
        if let Ok(t) = tag.as_bytes().try_into() {
            self.location.insert(t, location);
            true
        } else {
            false
        }
    }
}

impl FontFamily {
    pub fn new() -> FontFamily {
        FontFamily { fonts: Vec::new() }
    }

    pub fn add_font(&mut self, font: FontRef) {
        self.fonts.push(font);
    }

    /// Create a collection consisting of a single font
    pub fn new_from_font(font: Font) -> FontFamily {
        let mut result = FontFamily::new();
        result.add_font(FontRef::new(font));
        result
    }

    pub fn supports_codepoint(&self, c: char) -> bool {
        if let Some(font) = self.fonts.first() {
            let glyph_id = font.font.glyph_for_char(c);
            // TODO(font-kit): We're getting Some(0) for unsupported glyphs on CoreText
            // and DirectWrite
            glyph_id.unwrap_or(0) != 0
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
        self.families
            .iter()
            .position(|family| family.supports_codepoint(c))
            .unwrap_or(0)
    }
}

// This is the PostScript name of the font. Eventually this should be a unique ID.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FontId {
    postscript_name: String,
}

impl FontId {
    pub(crate) fn from_font(font: &FontRef) -> FontId {
        FontId {
            postscript_name: font.font.postscript_name().unwrap_or_default(),
        }
    }
}

impl<'a> Iterator for Itemizer<'a> {
    type Item = (Range<usize>, &'a FontRef);

    fn next(&mut self) -> Option<(Range<usize>, &'a FontRef)> {
        let start = self.ix;
        let mut chars_iter = self.text[start..].chars();
        if let Some(c) = chars_iter.next() {
            let mut end = start + c.len_utf8();
            let font_ix = self.collection.choose_font(c);
            debug!("{}: {}", c, font_ix);
            while let Some(c) = chars_iter.next() {
                if font_ix != self.collection.choose_font(c) {
                    break;
                }
                end += c.len_utf8();
            }
            self.ix = end;

            if &self.collection.families.len() >= &1 {
                Some((start..end, &self.collection.families[font_ix].fonts[0]))
            } else {
                None
            }
        } else {
            None
        }
    }
}
