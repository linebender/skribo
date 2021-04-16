//! A library to help convert from a unicode string and attributes to a list of glyphs and
//! positions.
//!
//! Converting unicode strings to glyphs is a complex process, requiring knowledge of
//!  - which fonts to use (and choosing an appropriate font given a size, script, typeface, weight,
//!    etc.),
//!  - a description of how to render a font (e.g. a truetype font file)
//!  - the space available for drawing and what to do if it's not enough,
//!  - and information on the user's locale.
//!
//! # Font terms
//!
//! The terminology of printing can be confusing. Partly this is because printing has been so
//! important for so long, and so has historical baggage, but it is also because unifying the
//! written methods of many different cultures requires a complicated system with many parts.
//! Unfortunately, a consequence of this is that most of the terms used in typesetting have
//! different meanings to different people. The unicode standard is useful in that it precisely
//! defines the terms it uses, but these meanings aren't necessarily the same as those that are in
//! common usage.  Below are some important terms in font rendering and the meanings some ascribe
//! to them:
//!
//! This is a work in progress and will be wrong in places (for now).
//!
//!  - *typography* - The study of drawing text to paper, screen, or some other medium. Drawing
//!    text is also known as *typesetting*, or *rendering*.
//!  - *symbol* - An image drawn or printed on a canvas, sheet of paper, or screen. Physically the
//!    same as a picture.
//!  - *glyph* - A symbol from a collection of symbols with some semantic meaning. The collection
//!    could be a script (like latin - `ABCDE....abcde...`, or it could be from the set of
//!    mathematical symbols (`+−×÷`). In the case of languages, another word more commonly used to
//!    mean *glyph* is *letter*, but "letter" can also mean grapheme (because in latin script all
//!    of the meanings are equivalent, they are often used interchangeably).
//!  - *font* - A set of glyphs that can be printed to paper or a screen. Sometimes the word
//!    font is used to mean typeface, and this can cause some confusion. In computing, a font can
//!    also refer to 1 or more font descriptions, which contain the information required to draw
//!    glyphs. These descriptions could be stored in a file on disk, or in memory, or somewhere
//!    else. "OpenType" has emerged as a consensus standard for the font file format, but
//!    tranditionally other formats existed, including "TrueType", various bitmap formats, and
//!    "PostScript". Container formats also exist for compression, like the "Web Open Font Format"
//!    (WOFF, WOFF2).
//!  - *typeface* - A collection of fonts that all share the same "look", but have different sizes,
//!    weights, and styles.
//!  - *size* An indication of the size of a font. A font's size is often given as the width of a
//!    latin `m` character. Sizes are often given in pixels on a computer, but screen pixels do not
//!    have a fixed physical width. To see this think about the fact that a 14" screen with the
//!    same number of pixels as a 21" screen will not have pixels of the same size.
//!  - *weight* The thickness of glyphs in the font. Thicker weighted fonts are known as *bold*
//!    whilst thinner weighted fonts are sometimes known as *light*. Weight can also be expressed as
//!    a number.
//!  - *style* - Variations of a specific typeface other than weight and size. Examples include a
//!    more rounded, slanted form (*italic*), simply slanting text (*oblique*) or using small upper
//!    case letters instead of lower case letters (`small-caps` in CSS). *Italic* is sometimes
//!    synthesized using an affine transformation to affect a slant.
//!  - *character* - The word "character" is often used informally, but is ambiguous and has no
//!    clear formal meaning. In Rust, the `char` type actually represents a unicode code point. In
//!    simple contexts (such as ASCII-only text), a "character" can be considered identical to a
//!    code point, but in a general context it should generally be avoided if precise meaning is
//!    desired. (The truth is slightly more nuanced than even this: `char` in Rust is actually a
//!    [unicode scalar](https://stackoverflow.com/questions/48465265/what-is-the-difference-between-unicode-code-points-and-unicode-scalars),
//!    but it's usually OK to ignore this distinction.
//!  - *unicode* - When computers were first created in America, they all used the latin alphabet,
//!    which fits into a byte. The *ASCII* encoding was developed to store latin characters in a
//!    byte of data. As other countries began to use computers, they created their own encodings to
//!    include their non-latin letters (for example É in French). Asian languages had to widen the
//!    character to 2 or more bytes since there are more than 256 glyphs in these languages.
//!    Unicode was developed as a (successful as it turns out) attempt to unify all languages and
//!    scripts into a single encoding scheme. Unicode characters are 4 bytes wide, meaning that
//!    there is space for 4_294_967_296 different characters, which is expected to be enough. (It's
//!    not quite this many in practice - some bit patterns are reserved).
//!  - *utf-8* - Unicode was a very useful in making it easier to use the same software in
//!    different parts of the world, but if all your text is latin then it takes 4 bytes per
//!    character instead of 1 for ASCII. This would be quite wasteful, so the most popular way to
//!    store unicode strings (utf-8) uses a continuation-bit style scheme to compress unicode
//!    characters where possible. Another older scheme is utf-16, which works similarly, but uses 2
//!    bytes (16 bits) as its smallest size.
//!  - *grapheme, grapheme cluster* - Outside of computer typesetting, graphemes can have multiple
//!    meanings in linguistics (see wikipedia). Within computer typesetting (and unicode in
//!    particular), a grapheme cluster (also sometimes simplified to grapheme despite the ambiguity
//!    this introduces) is a collection of code points whose meaning is different to those code
//!    points in isolation. It is often, but by no means always, represented by a single glyph.
//!    Exceptions to this include a rainbow flag (`U+1F3F3 (white flag), U+200D (zwj), U+1F308
//!    (rainbow)`) which produces a single rainbow flag glyph, "ﬁ", which is 1 glyph but 2 grapheme
//!    clusters, and "न्दी", which is 1 grapheme cluster but often 3 glyphs. In Han unification (see
//!    below), the same grapheme cluster may be represented by different glyphs depending on
//!    *locale*.
//!  - *script* - TODO write this using https://unicode.org/reports/tr24/, and UAX
//!    (https://github.com/linebender/skribo/pull/24),
//!    also explore the differences and similarities between scripts and writing systems:
//!    https://en.wikipedia.org/wiki/Script_(Unicode) and
//!    https://en.wikipedia.org/wiki/Writing_system.
//!  - *language* - A context for deriving meaning from a script. Collections of graphemes from a
//!    script will have different meanings in different languages, for example the word "pain"
//!    means different things in English and French ("bread" in french).
//!  - *locale* - Contextual information that defines how certain properties of the user interface
//!    should look. Examples are whether text should be rendered left-to-right, or right-to-left
//!    (as is the case in Hebrew, for example), and which varaint of Han characters to use.
//!    Non-text locale information includes date format and timezone.
//!  - *Han unification, CJK* - A number of Asian languages derive from ancient Chinese. The
//!    acronym CJK expands to Chinese, Japanese, Korean. Traditional Vietnamese also has this
//!    property, but since Vietnam has now adopted the Latin script it is less of an issue in
//!    practice. These languages use different glyphs for the same unicode code point (the code
//!    points themselves can be thought of as representing ancient Han Chinese).
//!  - *RTL, LTR, BiDi* - Some scripts are read from the right to the left (RTL), the opposite of latin
//!    which is read left-to-right (LTR). If a span of text contains code points from both LTR and
//!    RTL languages, then multiple directions of text will be used, and this is known as
//!    bi-directional (BiDi) text. For example, if an english sentence contains a hebrew word, a
//!    reader would expect that the order of letters in that word to be flipped.
//!  - *cursive* - todo (text rendering hates you says the word cursive describes the situation
//!    where the position of text in a word affects which glyph is used).
//!  - *font shaping* - The act of selecting particular glyphs to represent code points based on
//!    contextual information. In a number of languages (and in non-language unicode scripts like
//!    emoji, flags), the actual glyphs used to represent text depends on glyphs surrounding
//!    it. In Indic languages (e.g. Devanagari), graphemes from the same word are joined together,
//!    and this means that the glyphs used in the middle of a word are different to those at the
//!    edge, even for the same grapheme. A popular open-source library for converting graphemes to
//!    glyphs is [*HarfBuzz*](https://harfbuzz.github.io/), which has a [good explanation of text
//!    shaping](https://harfbuzz.github.io/what-is-harfbuzz.html#what-is-text-shaping) in its
//!    documentation.
//!  - *rendering* - todo (define this in context as drawing glyphs on a computer, then discuss CPU/GPU,
//!    vector/rasterized etc.
//!  - *antialiasing* - todo
//!  - *subpixel rendering* - todo
//!

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
pub use font_kit::properties::Style;

#[derive(Clone)]
pub struct TextStyle {
    // This should be either horiz and vert, or a 2x2 matrix
    pub size: f32,
    pub style: Style,
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

impl TextStyle {
    pub fn from_size(size: f32) -> Self {
        TextStyle {
            size,
            style: Style::Normal,
        }
    }
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
                log::debug!("{:?}", adv);
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
