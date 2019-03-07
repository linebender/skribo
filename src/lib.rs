use euclid::Vector2D;
use font_kit::loaders::default::Font;

pub struct TextStyle {
    // This should be either horiz and vert, or a 2x2 matrix
    pub size: f32,
}

#[derive(Debug)]
pub struct Layout {
    // TODO: reference to fonts
    glyphs: Vec<Glyph>,
    advance: Vector2D<f32>,
}

#[derive(Debug)]
struct Glyph {
    // TODO: index to font
    glyph_id: u32,
    offset: Vector2D<f32>,
    // TODO: more fields for advance, clusters, etc.
}

pub fn make_layout(style: &TextStyle, font: &Font, text: &str) -> Layout {
    let scale = style.size / (font.metrics().units_per_em as f32);
    let mut pos = Vector2D::zero();
    let mut glyphs = Vec::new();
    for c in text.chars() {
        if let Some(glyph_id) = font.glyph_for_char(c) {
            if let Ok(adv) = font.advance(glyph_id) {
                // TODO(font-kit): this doesn't get hinted advance (hdmx) table
                let adv_f = adv * scale;
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
        glyphs,
        advance: pos,
    }
}
