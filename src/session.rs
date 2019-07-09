//! Retained layout that supports substring queries.

use std::ops::Range;

use harfbuzz::sys::{hb_script_t, HB_SCRIPT_COMMON, HB_SCRIPT_INHERITED, HB_SCRIPT_UNKNOWN};

use euclid::Vector2D;

use crate::hb_layout::{layout_fragment, HbFace};
use crate::unicode_funcs::lookup_script;
use crate::{FontCollection, FontRef, Glyph, TextStyle};

pub struct LayoutSession<S: AsRef<str>> {
    text: S,
    style: TextStyle,
    fragments: Vec<LayoutFragment>,

    // A separate layout for the substring if needed.
    substr_fragments: Vec<LayoutFragment>,
}

pub(crate) struct LayoutFragment {
    // Length of substring covered by this fragment.
    pub(crate) substr_len: usize,
    pub(crate) script: hb_script_t,
    pub(crate) advance: Vector2D<f32>,
    pub(crate) glyphs: Vec<FragmentGlyph>,
    pub(crate) hb_face: HbFace,
    pub(crate) font: FontRef,
}

// This should probably be renamed "glyph".
//
// Discussion topic: this is so similar to hb_glyph_info_t, maybe we
// should just use that.
pub(crate) struct FragmentGlyph {
    pub cluster: u32,
    pub glyph_id: u32,
    pub offset: Vector2D<f32>,
    pub advance: Vector2D<f32>,
    pub unsafe_to_break: bool,
}

pub struct LayoutRangeIter<'a> {
    fragments: &'a [LayoutFragment],
    offset: Vector2D<f32>,
    fragment_ix: usize,
}

pub struct LayoutRun<'a> {
    // This should potentially be in fragment (would make it easier to binary search)
    offset: Vector2D<f32>,
    fragment: &'a LayoutFragment,
}

pub struct RunIter<'a> {
    offset: Vector2D<f32>,
    fragment: &'a LayoutFragment,
    glyph_ix: usize,
}

pub struct GlyphInfo {
    pub glyph_id: u32,
    pub offset: Vector2D<f32>,
}

impl<S: AsRef<str>> LayoutSession<S> {
    pub fn create(
        text: S,
        style: &TextStyle,
        collection: &FontCollection,
    ) -> LayoutSession<S> {
        let mut i = 0;
        let mut fragments = Vec::new();
        while i < text.as_ref().len() {
            let (script, script_len) = get_script_run(&text.as_ref()[i..]);
            let script_substr = &text.as_ref()[i..i + script_len];
            for (range, font) in collection.itemize(script_substr) {
                let fragment = layout_fragment(style, font, script, &script_substr[range]);
                fragments.push(fragment);
            }
            i += script_len;
        }
        let substr_fragments = Vec::new();
        LayoutSession {
            text,
            // Does this clone mean we should take style arg by-move?
            style: style.clone(),
            fragments,
            substr_fragments,
        }
    }

    /// Iterate through all glyphs in the layout.
    ///
    /// Note: this is redundant with `iter_substr` with the whole string, might
    /// not keep it.
    pub fn iter_all(&self) -> LayoutRangeIter {
        LayoutRangeIter {
            offset: Vector2D::zero(),
            fragments: &self.fragments,
            fragment_ix: 0,
        }
    }

    /// Iterate through the glyphs in the layout of the substring.
    ///
    /// This method reuses as much of the original layout as practical, almost
    /// entirely reusing the itemization, but possibly doing re-layout.
    pub fn iter_substr(&mut self, range: Range<usize>) -> LayoutRangeIter {
        if range == (0..self.text.as_ref().len()) {
            return self.iter_all();
        }
        // TODO: reuse existing layout if unsafe_to_break flag is false at both endpoints.
        let mut fragment_ix = 0;
        let mut str_offset = 0;
        while fragment_ix < self.fragments.len() {
            let fragment_len = self.fragments[fragment_ix].substr_len;
            if str_offset + fragment_len > range.start {
                break;
            }
            str_offset += fragment_len;
            fragment_ix += 1;
        }
        self.substr_fragments.clear();
        while str_offset < range.end {
            let fragment = &self.fragments[fragment_ix];
            let fragment_len = fragment.substr_len;
            let substr_start = range.start.max(str_offset);
            let substr_end = range.end.min(str_offset + fragment_len);
            let substr = &self.text.as_ref()[substr_start..substr_end];
            let font = &fragment.font;
            let script = fragment.script;
            // TODO: we should pass in the hb_face too, just for performance.
            let substr_fragment = layout_fragment(&self.style, font, script, substr);
            self.substr_fragments.push(substr_fragment);
            str_offset += fragment_len;
            fragment_ix += 1;
        }
        LayoutRangeIter {
            offset: Vector2D::zero(),
            fragments: &self.substr_fragments,
            fragment_ix: 0,
        }
    }
}

impl<'a> Iterator for LayoutRangeIter<'a> {
    type Item = LayoutRun<'a>;

    fn next(&mut self) -> Option<LayoutRun<'a>> {
        if self.fragment_ix == self.fragments.len() {
            None
        } else {
            let fragment = &self.fragments[self.fragment_ix];
            self.fragment_ix += 1;
            let offset = self.offset;
            self.offset += fragment.advance;
            Some(LayoutRun { offset, fragment })
        }
    }
}

impl<'a> LayoutRun<'a> {
    pub fn font(&self) -> &FontRef {
        &self.fragment.font
    }

    pub fn glyphs(&self) -> RunIter<'a> {
        RunIter {
            offset: self.offset,
            fragment: self.fragment,
            glyph_ix: 0,
        }
    }
}

impl<'a> Iterator for RunIter<'a> {
    type Item = GlyphInfo;

    fn next(&mut self) -> Option<GlyphInfo> {
        if self.glyph_ix == self.fragment.glyphs.len() {
            None
        } else {
            let glyph = &self.fragment.glyphs[self.glyph_ix];
            self.glyph_ix += 1;
            Some(GlyphInfo {
                glyph_id: glyph.glyph_id,
                offset: self.offset + glyph.offset,
            })
        }
    }
}

/// Figure out the script for the initial part of the buffer, and also
/// return the length of the run where that script is valid.
pub(crate) fn get_script_run(text: &str) -> (hb_script_t, usize) {
    let mut char_iter = text.chars();
    if let Some(cp) = char_iter.next() {
        let mut current_script = lookup_script(cp.into());
        let mut len = cp.len_utf8();
        while let Some(cp) = char_iter.next() {
            let script = lookup_script(cp.into());
            if script != current_script {
                if current_script == HB_SCRIPT_INHERITED || current_script == HB_SCRIPT_COMMON {
                    current_script = script;
                } else if script != HB_SCRIPT_INHERITED && script != HB_SCRIPT_COMMON {
                    break;
                }
            }
            len += cp.len_utf8();
        }
        if current_script == HB_SCRIPT_INHERITED {
            current_script = HB_SCRIPT_COMMON;
        }
        (current_script, len)
    } else {
        (HB_SCRIPT_UNKNOWN, 0)
    }
}

fn debug_script_runs(text: &str) {
    let mut text_substr = text;
    while !text_substr.is_empty() {
        let (script, len) = get_script_run(text_substr);
        println!("text {:?} script {:x}", &text_substr[..len], script);
        text_substr = &text_substr[len..];
    }
}
