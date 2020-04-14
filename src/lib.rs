#[macro_use]
extern crate log;

use font_kit::loaders::default::Font;
use pathfinder_geometry::vector::Vector2F;

mod collection;
mod hb_layout;
mod session;
mod tables;
mod unicode_funcs;

pub use crate::collection::{FontCollection, FontFamily, FontRef};
pub use crate::hb_layout::layout_run;
pub use crate::session::LayoutSession;

#[derive(Clone)]
pub struct TextStyle {
    // This should be either horiz and vert, or a 2x2 matrix
    pub size: f32,
}

// TODO: remove this (in favor of LayoutSession, which might take over this name)
#[derive(Debug)]
pub struct Layout {
    pub size: f32,
    pub glyphs: Vec<Glyph>,
    pub advance: Vector2F,
}

// TODO: remove this (in favor of GlyphInfo as a public API)
#[derive(Debug)]
pub struct Glyph {
    pub font: FontRef,
    pub glyph_id: u32,
    pub offset: Vector2F,
    // TODO: more fields for advance, clusters, etc.
}

impl Layout {
    pub(crate) fn new() -> Layout {
        Layout {
            size: 0.0,
            glyphs: Vec::new(),
            advance: Vector2F::default(),
        }
    }

    pub(crate) fn push_layout(&mut self, other: &Layout) {
        self.size = other.size;
        for glyph in &other.glyphs {
            self.glyphs.push(Glyph {
                font: glyph.font.clone(),
                glyph_id: glyph.glyph_id,
                offset: self.advance + glyph.offset,
            });
        }
        self.advance += other.advance;
    }
}

// This implementation just uses advances and doesn't do fallback.
pub fn make_layout(style: &TextStyle, font: &FontRef, text: &str) -> Layout {
    let scale = style.size / (font.font.metrics().units_per_em as f32);
    let mut pos = Vector2F::default();
    let mut glyphs = Vec::new();
    for c in text.chars() {
        if let Some(glyph_id) = font.font.glyph_for_char(c) {
            if let Ok(adv) = font.font.advance(glyph_id) {
                // TODO(font-kit): this doesn't get hinted advance (hdmx) table
                let adv_f = adv * scale;
                debug!("{:?}", adv);
                let glyph = Glyph {
                    font: font.clone(),
                    glyph_id,
                    offset: pos,
                };
                glyphs.push(glyph);
                pos += adv_f;
            }
        }
    }
    Layout {
        size: style.size,
        glyphs,
        advance: pos,
    }
}

pub fn layout(style: &TextStyle, collection: &FontCollection, text: &str) -> Layout {
    let mut result = Layout::new();
    for (range, font) in collection.itemize(text) {
        result.push_layout(&layout_run(style, font, &text[range]));
    }
    result
}
