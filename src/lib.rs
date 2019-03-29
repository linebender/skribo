use euclid::Vector2D;
use font_kit::loaders::default::Font;

mod harfbuzz;

pub use crate::harfbuzz::layout_run;

pub struct TextStyle {
    // This should be either horiz and vert, or a 2x2 matrix
    pub size: f32,
}

#[derive(Debug)]
pub struct Layout {
    // TODO: reference to fonts
    pub size: f32,
    pub glyphs: Vec<Glyph>,
    pub advance: Vector2D<f32>,
}

#[derive(Debug)]
pub struct Glyph {
    // TODO: index to font
    pub glyph_id: u32,
    pub offset: Vector2D<f32>,
    // TODO: more fields for advance, clusters, etc.
}

// This implementation just uses advances and doesn't do fallback.
pub fn make_layout(style: &TextStyle, font: &Font, text: &str) -> Layout {
    let scale = style.size / (font.metrics().units_per_em as f32);
    let mut pos = Vector2D::zero();
    let mut glyphs = Vec::new();
    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_for_char(c) {
            if let Ok(adv) = font.advance(glyph_id) {
                // TODO(font-kit): this doesn't get hinted advance (hdmx) table
                let adv_f = adv * scale;
                println!("{:?}", adv);
                let glyph = Glyph {
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
