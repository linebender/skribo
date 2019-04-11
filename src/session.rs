//! Retained layout that supports substring queries.

use harfbuzz::sys::{hb_script_t, HB_SCRIPT_COMMON, HB_SCRIPT_INHERITED, HB_SCRIPT_UNKNOWN};

use euclid::Vector2D;

use crate::hb_layout::{layout_fragment, HbFace};
use crate::unicode_funcs::lookup_script;
use crate::{FontCollection, FontRef, Glyph, TextStyle};

pub struct LayoutSession<'a> {
    text: &'a str,
    fragments: Vec<LayoutFragment>,
}

pub(crate) struct LayoutFragment {
    // Length of substring covered by this fragment.
    pub(crate) substr_len: usize,
    pub(crate) script: hb_script_t,
    pub(crate) advance: Vector2D<f32>,
    pub(crate) glyphs: Vec<Glyph>,
    pub(crate) hb_face: HbFace,
    pub(crate) font: FontRef,
}

pub struct LayoutRangeIter<'a> {
    // This probably wants to be a mut ref so we can stash resources in the session.
    session: &'a LayoutSession<'a>,
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

impl<'a> LayoutSession<'a> {
    pub fn create(
        text: &'a str,
        style: &TextStyle,
        collection: &FontCollection,
    ) -> LayoutSession<'a> {
        let mut i = 0;
        let mut fragments = Vec::new();
        while i < text.len() {
            let (script, script_len) = get_script_run(&text[i..]);
            let script_substr = &text[i..i + script_len];
            for (range, font) in collection.itemize(script_substr) {
                let fragment = layout_fragment(style, font, script, &script_substr[range]);
                fragments.push(fragment);
            }
            i += script_len;
        }
        LayoutSession { text, fragments }
    }

    pub fn iter_all(&self) -> LayoutRangeIter {
        LayoutRangeIter {
            offset: Vector2D::zero(),
            session: &self,
            fragment_ix: 0,
        }
    }

    // TODO: similar function as iter_all but takes a range (maybe subsumes iter_all, as
    // it has the same behavior with [0..text.len()]).
}

impl<'a> Iterator for LayoutRangeIter<'a> {
    type Item = LayoutRun<'a>;

    fn next(&mut self) -> Option<LayoutRun<'a>> {
        if self.fragment_ix == self.session.fragments.len() {
            None
        } else {
            let fragment = &self.session.fragments[self.fragment_ix];
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
