//! A HarfBuzz shaping back-end.

use pathfinder_geometry::vector::{vec2i, Vector2F};
use std::cell::RefCell;
use std::collections::HashMap;

use harfbuzz::sys::{
    hb_buffer_get_glyph_infos, hb_buffer_get_glyph_positions, hb_font_create, hb_font_destroy,
    hb_position_t, hb_shape,
};
use harfbuzz::sys::{
    hb_glyph_info_get_glyph_flags, hb_script_t, HB_GLYPH_FLAG_UNSAFE_TO_BREAK, HB_SCRIPT_DEVANAGARI,
};
use harfbuzz::{self as hb, Owned};

use crate::collection::FontId;
use crate::session::{GlyphInfo, LayoutFragment};
use crate::unicode_funcs::install_unicode_funcs;
use crate::{FontRef, Glyph, Layout, TextStyle};

thread_local! {
    static HB_THREAD_DATA: RefCell<HbThreadData> = RefCell::new(HbThreadData::new());
}

/// Per-thread data for HarfBuzz.
///
/// Fonts are cached by HarfBuzz, and this object manages the ownership and lifecycle of the cached
/// objects.
struct HbThreadData {
    hb_face_cache: HashMap<FontId, hb::Face<Owned>>,
}

impl HbThreadData {
    /// Create an empty cache.
    fn new() -> HbThreadData {
        HbThreadData {
            hb_face_cache: HashMap::new(),
        }
    }

    /// Add the given font to this cache.
    fn create_hb_face_for_font(&mut self, font: &FontRef) -> hb::Face<Owned> {
        (*self
            .hb_face_cache
            .entry(FontId::from_font(font))
            .or_insert_with(|| {
                let data = font.font.copy_font_data().expect("font data unavailable");
                hb::Face::from(data)
            }))
        .clone()
    }
}

// TODO: Scheduled for demolition.
pub fn layout_run(style: &TextStyle, font: &FontRef, text: &str) -> Layout {
    HB_THREAD_DATA.with(|hb_thread_data| {
        let mut hb_thread_data = hb_thread_data.borrow_mut();
        let mut b = hb::Buffer::new();
        install_unicode_funcs(&mut b);
        b.add_str(text);
        b.set_direction(hb::Direction::LTR);
        // TODO: set this based on detected script
        b.set_script(HB_SCRIPT_DEVANAGARI);
        b.set_language(hb::Language::from_string("en_US"));
        let hb_face = hb_thread_data.create_hb_face_for_font(font);
        unsafe {
            let hb_font = hb_font_create(hb_face.as_raw());
            hb_shape(hb_font, b.as_ptr(), std::ptr::null(), 0);
            hb_font_destroy(hb_font);
            let mut n_glyph = 0;
            let glyph_infos = hb_buffer_get_glyph_infos(b.as_ptr(), &mut n_glyph);
            log::debug!("number of glyphs: {}", n_glyph);
            let glyph_infos = std::slice::from_raw_parts(glyph_infos, n_glyph as usize);
            let mut n_glyph_pos = 0;
            let glyph_positions = hb_buffer_get_glyph_positions(b.as_ptr(), &mut n_glyph_pos);
            let glyph_positions = std::slice::from_raw_parts(glyph_positions, n_glyph_pos as usize);
            let mut total_adv = Vector2F::zero();
            let mut glyphs = Vec::new();
            let scale = style.size / (font.font.metrics().units_per_em as f32);
            for (glyph, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
                debug!("{:?} {:?}", glyph.codepoint, (pos.x_offset, pos.y_offset));
                let adv = vec2i(pos.x_advance, pos.y_advance);
                let adv_f = adv.to_f32() * scale;
                let offset = vec2i(pos.x_offset, pos.y_offset).to_f32() * scale;
                let g = Glyph {
                    font: font.clone(),
                    glyph_id: glyph.codepoint,
                    offset: total_adv + offset,
                };
                total_adv += adv_f;
                glyphs.push(g);
            }

            Layout {
                size: style.size,
                glyphs: glyphs,
                advance: total_adv,
            }
        }
    })
}

fn hb_face_from_font(font: FontRef) -> hb::Face<Owned> {
    let data = font.font.copy_font_data().expect("font data unavailable");
    hb::Face::from(data)
}

pub(crate) fn layout_fragment(
    style: &TextStyle,
    font: &FontRef,
    script: hb_script_t,
    text: &str,
) -> LayoutFragment {
    let mut b = hb::Buffer::new();
    install_unicode_funcs(&mut b);
    b.add_str(text);
    b.set_direction(hb::Direction::LTR);
    b.set_script(script);
    b.set_language(hb::Language::from_string("en_US"));
    let hb_face = hb_face_from_font(font.clone());
    unsafe {
        let hb_font = hb_font_create(hb_face.as_raw());
        hb_shape(hb_font, b.as_ptr(), std::ptr::null(), 0);
        hb_font_destroy(hb_font);
        let mut n_glyph = 0;
        let glyph_infos = hb_buffer_get_glyph_infos(b.as_ptr(), &mut n_glyph);
        log::trace!("number of glyphs: {}", n_glyph);
        let glyph_infos = std::slice::from_raw_parts(glyph_infos, n_glyph as usize);
        let mut n_glyph_pos = 0;
        let glyph_positions = hb_buffer_get_glyph_positions(b.as_ptr(), &mut n_glyph_pos);
        let glyph_positions = std::slice::from_raw_parts(glyph_positions, n_glyph_pos as usize);
        let mut total_adv = Vector2F::zero();
        let mut glyphs = Vec::new();
        // TODO: we might want to store this size-invariant.
        let scale = style.size / (font.font.metrics().units_per_em as f32);
        for (glyph, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            let adv = vec2i(pos.x_advance, pos.y_advance);
            let adv_f = adv.to_f32() * scale;
            let offset = vec2i(pos.x_offset, pos.y_offset).to_f32() * scale;
            let flags = hb_glyph_info_get_glyph_flags(glyph);
            let unsafe_to_break = flags & HB_GLYPH_FLAG_UNSAFE_TO_BREAK != 0;
            log::trace!(
                "{:?} {:?} {} {}",
                glyph.codepoint,
                (pos.x_offset, pos.y_offset),
                glyph.cluster,
                unsafe_to_break
            );
            let g = GlyphInfo {
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

/*
struct FontFuncs(*mut hb_font_funcs_t);

lazy_static! {
    static ref HB_FONT_FUNCS: FontFuncs = unsafe {
        let hb_funcs = hb_font_funcs_create();
    }
}
*/

/*
// Callback to access table data in a font
unsafe extern "C" fn font_table_func(
    _: *mut hb_face_t,
    tag: hb_tag_t,
    user_data: *mut c_void,
) -> *mut hb_blob_t {
    let font = user_data as *const Font;
    unimplemented!()
}
*/
