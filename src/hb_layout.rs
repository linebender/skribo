//! A HarfBuzz shaping back-end.

use euclid::Vector2D;

use harfbuzz::sys::*;
use harfbuzz::{Blob, Buffer, Direction, Language};

use crate::session::{FragmentGlyph, LayoutFragment};
use crate::unicode_funcs::install_unicode_funcs;
use crate::FontRef;
use crate::TextStyle;

pub(crate) struct HbFace {
    hb_face: *mut hb_face_t,
}

impl HbFace {
    pub fn new(font: &FontRef) -> HbFace {
        let data = font.font.copy_font_data().expect("font data unavailable");
        let blob = Blob::new_from_arc_vec(data);
        unsafe {
            let hb_face = hb_face_create(blob.into_raw(), 0);
            HbFace { hb_face }
        }
    }
}

impl Clone for HbFace {
    fn clone(&self) -> HbFace {
        unsafe {
            HbFace {
                hb_face: hb_face_reference(self.hb_face),
            }
        }
    }
}

impl Drop for HbFace {
    fn drop(&mut self) {
        unsafe {
            hb_face_destroy(self.hb_face);
        }
    }
}

pub(crate) fn layout_fragment(
    style: &TextStyle,
    font: &FontRef,
    hb_face: &HbFace,
    script: hb_script_t,
    text: &str,
) -> LayoutFragment {
    let mut b = Buffer::new();
    install_unicode_funcs(&mut b);
    b.add_str(text);
    b.set_direction(Direction::LTR);
    b.set_script(script);
    b.set_language(Language::from_string("en_US"));
    unsafe {
        let hb_font = hb_font_create(hb_face.hb_face);
        hb_shape(hb_font, b.as_ptr(), std::ptr::null(), 0);
        hb_font_destroy(hb_font);
        let mut n_glyph = 0;
        let glyph_infos = hb_buffer_get_glyph_infos(b.as_ptr(), &mut n_glyph);
        println!("number of glyphs: {}", n_glyph);
        let glyph_infos = std::slice::from_raw_parts(glyph_infos, n_glyph as usize);
        let mut n_glyph_pos = 0;
        let glyph_positions = hb_buffer_get_glyph_positions(b.as_ptr(), &mut n_glyph_pos);
        let glyph_positions = std::slice::from_raw_parts(glyph_positions, n_glyph_pos as usize);
        let mut total_adv = Vector2D::zero();
        let mut glyphs = Vec::new();
        // TODO: we might want to store this size-invariant.
        let scale = style.size / (font.font.metrics().units_per_em as f32);
        for (glyph, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let adv = Vector2D::new(pos.x_advance, pos.y_advance);
            let adv_f = adv.to_f32() * scale;
            let offset = Vector2D::new(pos.x_offset, pos.y_offset).to_f32() * scale;
            let flags = hb_glyph_info_get_glyph_flags(glyph);
            let unsafe_to_break = flags & HB_GLYPH_FLAG_UNSAFE_TO_BREAK != 0;
            println!(
                "{:?} {:?} {} {}",
                glyph, pos, glyph.cluster, unsafe_to_break
            );
            let g = FragmentGlyph {
                cluster: glyph.cluster,
                advance: adv_f,
                glyph_id: glyph.codepoint,
                offset: total_adv + offset,
                unsafe_to_break,
            };
            total_adv += adv_f;
            glyphs.push(g);
        }

        LayoutFragment {
            //size: style.size,
            substr_len: text.len(),
            script,
            glyphs: glyphs,
            advance: total_adv,
            hb_face: hb_face.clone(),
            font: font.clone(),
        }
    }
}

#[allow(unused)]
fn float_to_fixed(f: f32) -> i32 {
    (f * 65536.0 + 0.5).floor() as i32
}

#[allow(unused)]
fn fixed_to_float(i: hb_position_t) -> f32 {
    (i as f32) * (1.0 / 65536.0)
}
