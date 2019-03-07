//! Example program for testing rendering with skribo.

use std::fs::File;
use std::io::Write;

use euclid::{Point2D, Size2D};
use font_kit::canvas::{Canvas, Format, RasterizationOptions};
use font_kit::family_name::FamilyName;
use font_kit::hinting::HintingOptions;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;

use skribo::{make_layout, TextStyle};

fn write_canvas_pgm(canvas: &Canvas, filename: &str) -> Result<(), std::io::Error> {
    let mut f = File::create(filename)?;
    write!(f, "P5\n{} {}\n255\n", canvas.size.width, canvas.size.height)?;
    f.write(&canvas.pixels)?;
    Ok(())
}


fn main() {
    println!("render test");
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap();
    let style = TextStyle {
        size: 32.0,
    };
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

    println!("{:?}", make_layout(&style, &font, "hello world"));
    write_canvas_pgm(&canvas, "out.pgm").unwrap();
}
