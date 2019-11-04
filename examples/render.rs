//! Example program for testing rendering with skribo.

use std::fs::File;
use std::io::Write;
use std::ops::Range;

use euclid::{Point2D, Size2D};
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use skribo::*;

#[cfg(target_family = "windows")]
const DEVANAGARI_FONT_POSTSCRIPT_NAME: &str = "NirmalaUI";
#[cfg(target_os = "macos")]
const DEVANAGARI_FONT_POSTSCRIPT_NAME: &str = "DevanagariUI";
#[cfg(target_os = "linux")]
const DEVANAGARI_FONT_POSTSCRIPT_NAME: &str = "NotoSerifDevanagari";

struct SimpleSurface {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

fn composite(a: u8, b: u8) -> u8 {
    let y = ((255 - a) as u16) * ((255 - b) as u16);
    let y = (y + (y >> 8) + 0x80) >> 8; // fast approx to round(y / 255)
    255 - (y as u8)
}

// A simple drawing surface, just because it's easier to implement such things
// directly than pull in dependencies for it.
impl SimpleSurface {
    fn new(width: usize, height: usize) -> SimpleSurface {
        let pixels = vec![0; width * height];
        SimpleSurface {
            width,
            height,
            pixels,
        }
    }

    fn paint_from_canvas(&mut self, canvas: &Canvas, x: i32, y: i32) {
        let (cw, ch) = (canvas.size.width as i32, canvas.size.height as i32);
        let (w, h) = (self.width as i32, self.height as i32);
        let y = y - ch;
        let xmin = 0.max(-x);
        let xmax = cw.min(w - x);
        let ymin = 0.max(-y);
        let ymax = ch.min(h - y);
        for yy in ymin..(ymax.max(ymin)) {
            for xx in xmin..(xmax.max(xmin)) {
                let pix = canvas.pixels[(cw * yy + xx) as usize];
                let dst_ix = ((y + yy) * w + x + xx) as usize;
                self.pixels[dst_ix] = composite(self.pixels[dst_ix], pix);
            }
        }
    }

    fn write_pgm(&self, filename: &str) -> Result<(), std::io::Error> {
        let mut f = File::create(filename)?;
        write!(f, "P5\n{} {}\n255\n", self.width, self.height)?;
        f.write(&self.pixels)?;
        Ok(())
    }

    fn paint_layout_session<S: AsRef<str>>(
        &mut self,
        layout: &mut LayoutSession<S>,
        x: i32,
        y: i32,
        range: Range<usize>,
    ) {
        for run in layout.iter_substr(range) {
            let font = run.font();
            let size = 32.0; // TODO: probably should get this from run
            println!("run, font = {:?}", font);
            for glyph in run.glyphs() {
                let glyph_id = glyph.glyph_id;
                let glyph_x = (glyph.offset.x as i32) + x;
                let glyph_y = (glyph.offset.y as i32) + y;
                let bounds = font
                    .font
                    .raster_bounds(
                        glyph_id,
                        size,
                        &Point2D::zero(),
                        HintingOptions::None,
                        RasterizationOptions::GrayscaleAa,
                    )
                    .unwrap();
                println!(
                    "glyph {}, bounds {:?}, {},{}",
                    glyph_id, bounds, glyph_x, glyph_y
                );
                if !bounds.is_empty() {
                    let origin_adj = bounds.origin.to_f32();
                    let neg_origin = Point2D::new(-origin_adj.x, -origin_adj.y);
                    let mut canvas = Canvas::new(
                        // Not sure why we need to add the extra pixel of height, probably a rounding isssue.
                        // In any case, seems to get the job done (with CoreText rendering, anyway).
                        &Size2D::new(bounds.size.width as u32, 1 + bounds.size.height as u32),
                        Format::A8,
                    );
                    font.font
                        .rasterize_glyph(
                            &mut canvas,
                            glyph_id,
                            // TODO(font-kit): this is missing anamorphic and skew features
                            size,
                            &neg_origin,
                            HintingOptions::None,
                            RasterizationOptions::GrayscaleAa,
                        )
                        .unwrap();
                    self.paint_from_canvas(
                        &canvas,
                        glyph_x + bounds.origin.x,
                        glyph_y - bounds.origin.y,
                    );
                }
                println!("glyph {} @ {:?}", glyph.glyph_id, glyph.offset);
            }
        }
    }
}

fn make_collection() -> FontCollection {
    let mut collection = FontCollection::new();
    let source = SystemSource::new();
    let font = source
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    collection.add_family(FontFamily::new_from_font(font));

    let font = source
        .select_by_postscript_name(DEVANAGARI_FONT_POSTSCRIPT_NAME)
        .expect("failed to select Devanagari font")
        .load()
        .unwrap();
    collection.add_family(FontFamily::new_from_font(font));

    collection
}

fn main() {
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    let data = font.copy_font_data();
    println!("font data: {:?} bytes", data.map(|d| d.len()));

    let style = TextStyle { size: 32.0 };
    let glyph_id = font.glyph_for_char('O').unwrap();
    println!("glyph id = {}", glyph_id);
    println!(
        "glyph typo bounds: {:?}",
        font.typographic_bounds(glyph_id).unwrap()
    );
    println!(
        "glyph raster bounds: {:?}",
        font.raster_bounds(
            glyph_id,
            32.0,
            &Point2D::zero(),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa
        )
    );
    let mut canvas = Canvas::new(&Size2D::new(32, 32), Format::A8);
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        // TODO(font-kit): this is missing anamorphic and skew features
        style.size,
        &Point2D::zero(),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();
    // TODO(font-kit): FreeType is top-aligned, CoreText is bottom-aligned, and FT seems to ignore origin
    font.rasterize_glyph(
        &mut canvas,
        glyph_id,
        style.size,
        &Point2D::new(16.0, 16.0),
        HintingOptions::None,
        RasterizationOptions::GrayscaleAa,
    )
    .unwrap();

    let mut args = std::env::args();
    args.next();
    let text = args.next().unwrap_or("Hello हिन्दी".to_string());
    let collection = make_collection();
    let mut layout = LayoutSession::create(&text, &style, &collection);
    let mut surface = SimpleSurface::new(200, 50);
    surface.paint_layout_session(&mut layout, 0, 35, 0..text.len());
    surface.write_pgm("out.pgm").unwrap();
}
