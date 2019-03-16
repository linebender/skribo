//! A HarfBuzz shaping back-end.

use std::os::raw::{c_char, c_uint, c_void};
use std::sync::Arc;

use euclid::Vector2D;

use harfbuzz::sys::{
    hb_buffer_get_glyph_infos, hb_buffer_get_glyph_positions,
    hb_blob_create, hb_blob_destroy, hb_blob_t, hb_face_create, hb_face_destroy, hb_face_reference, hb_face_t,
    hb_font_create, hb_font_destroy, hb_shape, hb_position_t,
};
use harfbuzz::sys::{HB_MEMORY_MODE_READONLY, HB_SCRIPT_LATIN};
use harfbuzz::{Buffer, Direction, Language};

use font_kit::loaders::default::Font;

use crate::{Glyph, Layout, TextStyle};
use crate::{FontCollection, FontRef};

struct HbFace {
    hb_face: *mut hb_face_t,
}

impl HbFace {
    pub fn new(font: &FontRef) -> HbFace {
        let data = font.font.copy_font_data().expect("font data unavailable");
        let blob = ArcVecBlob::new(data);
        unsafe {
            let hb_face = hb_face_create(blob.into_raw(), 0);
            HbFace { hb_face }
        }
    }
}

impl Clone for HbFace {
    fn clone(&self) -> HbFace {
        unsafe {
            HbFace { hb_face: hb_face_reference(self.hb_face) }
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

pub fn layout_run(style: &TextStyle, font: &FontRef, text: &str) -> Layout {
    let mut b = Buffer::new();
    b.add_str(text);
    b.set_direction(Direction::LTR);
    b.set_script(HB_SCRIPT_LATIN);
    b.set_language(Language::from_string("en_US"));
    let hb_face = HbFace::new(font);
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
        let scale = style.size / (font.font.metrics().units_per_em as f32);
        for (glyph, pos) in glyph_infos.iter().zip(glyph_positions.iter()) {
            //println!("{:?} {:?}", glyph, pos);
            let adv = Vector2D::new(pos.x_advance, pos.y_advance);
            let adv_f = adv.to_f32() * scale;
            let offset = Vector2D::new(pos.x_offset, pos.y_offset).to_f32() * scale;
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

/// A HarfBuzz blob that's backed by an `Arc<Vec>`.
///
/// Note: this can probably be merged with `Blob` in the harfbuzz crate.
struct ArcVecBlob(*mut hb_blob_t);

impl ArcVecBlob {
    pub fn new(data: Arc<Vec<u8>>) -> ArcVecBlob {
        let len = data.len();
        assert!(len <= c_uint::max_value() as usize);
        unsafe {
            let data_ptr = data.as_ptr();
            let ptr = Arc::into_raw(data);
            let hb_blob = hb_blob_create(
                data_ptr as *const c_char,
                len as c_uint,
                HB_MEMORY_MODE_READONLY,
                ptr as *mut c_void,
                Some(arc_vec_blob_destroy),
            );
            ArcVecBlob(hb_blob)
        }
    }

    pub fn into_raw(self) -> *mut hb_blob_t {
        let ptr = self.0;
        std::mem::forget(self);
        ptr
    }
}

// Can implement Clone, Deref as needed; impls similar to harfbuzz crate

impl Drop for ArcVecBlob {
    fn drop(&mut self) {
        unsafe {
            hb_blob_destroy(self.0);
        }
    }
}

// This has type hb_destroy_func_t
unsafe extern "C" fn arc_vec_blob_destroy(user_data: *mut c_void) {
    std::mem::drop(Arc::from_raw(user_data as *const Vec<u8>))
}
