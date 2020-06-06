//! The font collection type.
//!
//! A font collection is an ordered list of font families, where preference is given to families
//! earlier in the list. The reason for specifying multiple families is that it may be that a
//! particular font does not support a particular unicode string, in which case it would be
//! necessary to try the next font in the list.

use std::fmt;
use std::ops::Range;
use std::sync::Arc;

use crate::Font;

/// A collection of font families.
///
/// See module level docs for a more detailed description of what a `FontCollection` is.
pub struct FontCollection {
    pub(crate) families: Vec<FontFamily>,
}

/// A collection of fonts, all of which have the same *typeface*.
///
/// The same typeface (e.g. "Times Roman") may have multiple fonts, for different styles (e.g.
/// normal, italic), weight (e.g. bold, 500) and size. When printing to a screen, size is less of
/// an issue because text can be scaled (although scaling may affect pixel alignment etc.), but
/// weight and style require different glyphs.
///
/// Fonts added to a font family should support the same set of code points.
pub struct FontFamily {
    // TODO: multiple weights etc
    pub(crate) fonts: Vec<FontRef>,
}

/// An immutable shared reference to a font-kit `Font`.
// Design question: deref to Font?
#[derive(Clone)]
pub struct FontRef {
    pub font: Arc<Font>,
}

impl fmt::Debug for FontRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FontRef({})", self.font.full_name())
    }
}

/// An iterator over `(Range<usize>, &FontRef)` pairs. Use the given font for the given range. When
/// rendering.
pub struct Itemizer<'a> {
    text: &'a str,
    collection: &'a FontCollection,
    ix: usize,
}

impl FontRef {
    /// Helper method to wrap a font in an `Arc` for `FontRef`.
    pub fn new(font: Font) -> FontRef {
        FontRef {
            font: Arc::new(font),
        }
    }
}

impl FontFamily {
    /// Create an empty font family.
    pub fn new() -> FontFamily {
        FontFamily { fonts: Vec::new() }
    }

    /// Adda a font to an existing family.
    ///
    /// It is the user's responsibility to check that the coverage of the added font matches those
    /// previously added.
    pub fn add_font(&mut self, font: FontRef) {
        self.fonts.push(font);
    }

    /// Create a collection consisting of a single font.
    pub fn new_from_font(font: Font) -> FontFamily {
        let mut result = FontFamily::new();
        result.add_font(FontRef::new(font));
        result
    }

    /// Checks if this font family supports the given code point.
    ///
    /// This is implemented by checking the first font in the family, meaning it will not detect if
    /// later fonts support different code points (see `FontFamily::add_font`).
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
    /// Create a new empty `FontCollection`.
    pub fn new() -> FontCollection {
        FontCollection {
            families: Vec::new(),
        }
    }

    /// Append a font family to the collection, such that it will have lower preference than any
    /// previously added font families.
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
            log::debug!("{}: {}", c, font_ix);
            while let Some(c) = chars_iter.next() {
                if font_ix != self.collection.choose_font(c) {
                    break;
                }
                end += c.len_utf8();
            }
            self.ix = end;
            Some((start..end, &self.collection.families[font_ix].fonts[0]))
        } else {
            None
        }
    }
}
