# Requirements for skribo

This document summarizes the requirements for skribo, a crate for low level text rendering. The scope and motivation for the crate are described in the [skribo kickoff blog post], but this document goes into more detail and specifically tries to clarify lines responsibility for the various crates.

## Low level text rendering

The following functions are in scope:

* Shaping of a run of text with a single style.

* Plumbing of locale data, see below.

* Font selection, including:

  - Resolving of fallbacks through both a user-provided and system fallback list

  - Resolving color vs b/w emoji (including handling of variation selectors).

* Correct shaping of complex scripts.

  - Using HarfBuzz as the primary engine, but abstracting over details so other engines can be used.

* Computation of data for faux bold and faux italic (as it's common for different font families in a font collection to have different coverage of styles).

* Handling of unicode-range to effectively subset fonts in the stack. (Note: since this tends not to be directly supported in platform text stacks, it may be optionally supported by skribo)

And these functions are out of scope, as they are in a higher level:

* Paragraph level formatting including line breaking.

* Hyphenation.

* Representation of rich text.

* BiDi.

* Line spacing (but with access to metrics that can be used to compute this).

* Text-decodration (but providing enough metrics, such as underline-position, that the renderer can add them).

## Access to system fonts and metadata

In order to choose an appropriate font from a font collection, skribo needs access to the fallback fonts on the system. In many cases, choice of a fallback font can be determined entirely through Unicode coverage. However, particularly for CJK, there may be multiple fallback fonts with coverage of the same Unicode range (because of [Han unification]). In these cases, additional locale metadata is required. Access to system fonts, and loading custom fonts through files or memory buffers, is generally within the scope of [font-kit]. One of the requirements for skribo is refining the interface between font-kit and its clients so that font selection can be done accurately and performantly.

## Locale-sensitive layout

Text layout is sensitive to locales, entailing a number of requirements. First is representation of locales. The underlying standard is [BCP 47]. We should try to use common Rust infrastructure for the parsing and representation. In addition, there are some utility functions that are likely specific to text, for example inferring script from other locale information (in ICU, this is done with the [`addLikelySubtags`] method).

Locales manifest in two primary ways. First, they affect selection of fonts from a fallback list, especially Han unification. Second, they need to be plumbed to the shaping engine so OpenType [`locl`] features can be selected.

Note that to really do this properly requires a locale *list,* not just a single locale as in some older interfaces. An example would be two users with en_US as their primary locale, but with ja_JP and zh_CN as their secondary locales. In this case, the secondary locale would be responsible for Han unification. In short, Han unification is controlled by the first CJK entry in the locale list. But Han unification is only the most visible example; in general, localization needs to be done from the first relevant entry, and the logic for what's relevant is complex and font-dependent.

It is the responsibility of the caller, not skribo, to determine the locale list for text layout. A typical caller will use a combination of explicit lang tags, possible linguistic analysis of the text, other cues (for example, inferring language from font name), and, likely as the lowest priority, the system locale.

We tentatively plan to use [fluent-locale-rs] for the locale representation. Also, since access to the fallback list is platform-dependent, the current plan is for that functionality to be implemented in font-kit, see [font-kit#37](https://github.com/pcwalton/font-kit/issues/37).

## Performance

Text layout can be slow, and especially as more of the rendering pipeline is moving to the GPU, it can be the limiting factor. A large part of the design goal is performance, and here are some mechanisms to try to achieve that:

* A layout cache. One subtlety is that it should be designed for multithreaded use, so a `Mutex<HashMap>` may replaced at some point with a concurrent hash map.

* A fast representation of styles. Since style information can be expensive, both for storage and for hash-table lookup, we might design an arena, indexed by integer. Alternatively, we might use a small struct that covers the most common cases, with an optional `Box` for fancier style parameters such as OpenType features. In any case, locale is a large part of the puzzle.

* Possibly fast paths for monospace fonts, including CJK. These have to be carefully designed to preserve semantics (unlike the "simple path" in Blink before it was removed).

* Locale represntation can impact performance, so we plan work on [fluent-locale-rs] to reduce the size and allocation burden for locale objects.

## Some open questions

* Do shaping driven itemization, or use coverage data extracted from fonts (as in Minikin)?

* Is layout at this level size-independent, or can it take into account pixel size dependent metrics (see [font-kit#34](https://github.com/pcwalton/font-kit/issues/34)).

* Do we have a dependency on font-kit in all cases, or only cases where we're not using the platform layout? The font-kit dependency seems pretty heavyweight; more empirical measurement is necessary here.

* Is there an abstraction over different shaping engines at the skribo level, or should skribo itself be specialized to cases where it does its own shaping (using HarfBuzz at first but potentially other engines)? In the latter case, there is a need for an abstraction at a higher level.

## Vertical text

Vertical text is now part of the requirements for fully correct Web text rendering, thanks to [CSS writing modes]. Thus, vertical text is part of the *design* for skribo. I'd like more information about whether its implementation should be a priority.

Some of the groundwork can be done just by using proper types (vector rather than scalar for advance, an enum rather than bool for text direction).

## Font variation

A goal for skribo is to support font variation. This is tricky, in part because variation is a relatively new feature, so it probably won't be part of the intial implementation.

Note that font variation support in DWrite is still [work in progress](https://docs.microsoft.com/en-us/windows/desktop/directwrite/opentype-variable-fonts#opentype-variable-font-support-in-directwrite) - there is support for using existing interfaces to set width and weight, and also named instances of variable fonts, but no direct support for general continuous variation.

## Future extensions planned

The following features are slated for the initial release, but the design should extend to accommodating them:

* Advanced justification such as Arabic kashida (and stuff around the [jstf table], which is widely regarded to need rework). There is exciting future work around [justification with variable fonts] as well.

* Optical margin alignment.

* In general, features for advanced text layout, such as in the W3C text layout requirements documents. An excellent analysis of these is Behnam Esfahbod's presentation, [Comparative Analysis of
W3C Text Layout Requirements]. Most of these are not standards-track, and some probably never will be, but in any case it's good to understand needs of high quality layout in various languages.

## Major types and methods

Below is an enumeration of the major types to be provided by the crate, along with discussion and references.

### Engine / Cache

The primary role of the "Engine" type is to serve as a layout cache. It is also responsible for initializing libraries and loading global data as needed.

It is something of a factory for layouts, so similar to [DWriteFactory](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nn-dwrite-idwritefactory). The cache functionality is in LayoutCache in Minikin (which is effectively a singleton, managed by Layout).

### Locale list

The locale list is a prioritized list of BCP-47 locales, likely with some preprocessing (the likely subtags) to make it efficient to use. Creation of locale lists is expected to be infrequent.

This is `LocaleList` in Minikin. In DirectWrite, I think a single locale can be set with [SetLocaleName](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nf-dwrite-idwritetextlayout-setlocalename); it's possible that for multi-locale handling some logic needs to be layered on top (it's also possible there's a newer API that I didn't find on quick searching).

Following direction from [fluent-locale-rs], the main locale type will follow the Unicode BCP 47 Locale Identifier rather than Language Tag. The differences are subtle, see [fluent-locale-rs#8](https://github.com/projectfluent/fluent-locale-rs/pull/8) and linked issue for more details.

### Style

The style type is at heart a representation of CSS-like properties. It includes:

* (Maybe) size, see above.

* Weight.

* Italic / oblique / normal.

* Text direction. (possibly split out as a separate arg)

* Locale list.

* Opentype features.

* Additional font variation axes (eg optical size).

* ... (to be expanded)

Among other things, either style itself or a handle to it is part of the key for the layout cache.

This is [MinikinPaint](https://android.googlesource.com/platform/frameworks/minikin/+/refs/heads/master/include/minikin/MinikinFont.h) in Minikin.

In DirectWrite, most of it is setters on [TextLayout](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nn-dwrite-idwritetextlayout), with help from other types including [IDWriteTypography](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nf-dwrite-idwritetextlayout-settypography).

### Layout

The layout is the primary result of the crate. It contains an (x,y) positioned list of glyph references, where each glyph reference is a font reference and a glyph id.

A layout also supports queries for bounding box, total advance, and cursor positioning.

It corresponds closely to a [`TextLayout`](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nn-dwrite-idwritetextlayout) in DirectWrite, and a Layout in Minikin.

### Font collection

A font collection is a stack of fonts, with additional metadata to help with selection. It corresponds to a font-family stack in CSS, but generally has more granularity into fallbacks (generally system fonts).

There are two paths to font collections. One is system fonts (and access to data for fallback). This is a tricky and platform-dependent implementation, but not a difficult client interface. The other is a builder for custom fonts, which means organizing into families (related fonts with different weight etc.) and providing additional metadata for selection. A specific challenge is support of [unicode-range], which is needed for Web work.

This is FontCollection in Minikin.

The story in DirectWrite is more complicated. There is a [FontCollection](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nn-dwrite-idwritefontcollection) which handles system fonts, but this type doesn't support web-specific functionality. For that, there's a [new](https://docs.microsoft.com/en-us/windows/desktop/directwrite/what-s-new-in-directwrite-for-windows-8-consumer-preview) (Windows 10+) [FontSet](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite_3/nn-dwrite_3-idwritefontset), with an associated [builder](https://docs.microsoft.com/en-us/windows/desktop/api/dwrite_3/nn-dwrite_3-idwritefontsetbuilder). It's worth noting that builder supports adding fonts with script metadata.

Note also that [DirectWrite custom font sets] support a lot of additional functionality, including streaming fonts from the network.

[CSS writing modes]: https://www.w3.org/TR/css-writing-modes-3/
[Han unification]: https://en.wikipedia.org/wiki/Han_unification
[BCP 47]: https://tools.ietf.org/html/bcp47
[skribo kickoff blog post]: https://raphlinus.github.io/rust/skribo/text/2019/02/27/text-layout-kickoff.html
[`addLikelySubtags`]: http://icu-project.org/apiref/icu4j/com/ibm/icu/util/ULocale.html#addLikelySubtags-com.ibm.icu.util.ULocale-
[`locl`]: https://docs.microsoft.com/en-us/typography/opentype/spec/features_ko#a-namelocl-idloclatag-39locl39
[Minikin]: https://android.googlesource.com/platform/frameworks/minikin/
[unicode-range]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/unicode-range
[DirectWrite custom font sets]: https://docs.microsoft.com/en-us/windows/desktop/directwrite/custom-font-sets-win10
[fluent-locale-rs]: https://github.com/projectfluent/fluent-locale-rs
[jstf table]: https://docs.microsoft.com/en-us/typography/opentype/spec/jstf
[justification with variable fonts]: https://twitter.com/simoncozens/status/1096447322295283712
[Comparative Analysis of W3C Text Layout Requirements]: https://behnam.es/scriptology/talks/2017-IUC41-Comparative_Analysis_of_W3C_Text_Layout_Requirements-Behnam_Esfahbod.pdf