One central task for skribo is choosing a font most suitable for the requested script. The requested script is a combination of an explicit lang tag and locale system settings.

Part of what makes this requirement tricky is that there is a fuzzy line between the scope of skribo and the scope of [font-kit]. Conceptually, skribo should be a consumer of font metadata, and font-kit is a provider. At a finer grain, there are decisions about exactly what that interface should look like. The main purpose of this document is to describe the problems to be solved.

## Background

Selection of font is not strictly based on Unicode coverage, but is also affected by locale. The biggest impact is on Han unification, but there are other examples. Suitability of a font for a script is *not* encoded in the font's tables, but is metadata about the system's font configuration. Mechanisms for accessing this metadata vary widely from platform to platform.

A significant amount of research for this topic was reading through the [Skia] code, as that's the platform abstraction used by Chromium, so can be used as a rough first draft of requirements for correct cross-platform text layout for the Web.

TODO: write a clear requirement, link to this section of requirements doc.

## BCP-47

I strongly recommend the use of BCP-47 as the identifier for language, script, and other locale metadata. This is an easy decision for web use cases, as it is the standard for the [lang] tag. The main challenge is that mechanisms for system font metadata in general predate BCP-47, so there will be some impedance matching.

TODO: investigate Rust ecosystem for common BCP-47 tag representation.

## Platform-specific considerations

Each platform handles access to metadata (including script) in a different way. Platforms generally treat their text stack as an opaque black box, and vary in how much low-level access they give.

### Fontconfig

The [Fontconfig] configuration file format specifies "langset" as an an "RFC-3066-style" language. [RFC 3066] is a predecessor to BCP-47 (dated 2001), and basically specifies language and country, with no provision for explicit script or variant. For the purpose of Han unification, the convention is to infer script from country. For example, "zh-CN" could be translated to "zh-Hans", "zh-TW" to "zh-Hant". However, after a little investigation, it's not clear to me how useful it is to do sophisticated processing here, as the default fontconfig on a clean Debian 9 install lists doesn't specify "langset" attributes, but just has a few informal descriptions in comments.

I request more input from CJK Linux users about what's going on here and the best way to handle it going forward.

### Windows

Windows is a tricky case. The [DWrite font enumeration] functions don't provide detailed metadata on script coverage. The recommended platform-native way to resolve font matching is to use [IDWriteFontFallback] to map characters, with an [IDWriteTextAnalysisSource] to specify locale and other details. The [Skia Windows font manager] uses this interface.

The current font-kit code uses the font enumeration and not the text analysis APIs. Thus, getting full functionality will likely require significant architectural rework.

As far as I can tell, there are no Rust wrappers for these fallback and text analysis winapi functions.

(Side note: font-kit uses [dwrote], while the driud/piet infra uses the [directwrite crate]. We should dedup this or at the very least make interop painless)

### macOS

The current font-kit code uses the availableFontFamilies call in [NSFontManager], which reports just the names without any additional metadata.

Skia's macOS implementation of font selection (the onMatchFamilyStyleCharacter) is based on [CTFontCreateForString], architecturally a similar approach as on Windows. However, that doesn't seem to be sensitive to the bcp47 argument. I did a little investigation and wasn't able to figure out exactly where Han unification gets handled on macOS, but found some signs it is special-cased. For further research, [chromium#586517] may be a good breadcrumb trail to follow.

### Android

Android has a complete and up-to-date listing of font metadata, including BCP-47 language tags, in its [fonts.xml] file. This format is quasi-standard; the only officially supported way to access fonts is through the Java APIs, but that's not adequate for low-level needs such as a Web browser, so the Android frameworks team has historically made some effort not to break this file format. Note that the format evolves a bit over time, in particular Lollipop was a significant difference. The file also has a threatening sounding notice suggesting that it will go away in an imminent release; I'm not sure what they have in mind to replace it.

The best reference for parsing fonts.xml is [Skia], as that's used to configure fonts in Chrome. In particular, its parsing logic is in [SkFontMgr_android_parser].

[BCP 47]: https://tools.ietf.org/html/bcp47
[lang]: https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang
[Fontconfig]: https://www.freedesktop.org/software/fontconfig/fontconfig-user.html
[RFC 3066]: https://www.ietf.org/rfc/rfc3066.txt
[font-kit]: https://github.com/pcwalton/font-kit
[fonts.xml]: https://android.googlesource.com/platform/frameworks/base/+/master/data/fonts/fonts.xml
[Skia]: https://github.com/google/skia
[SkFontMgr_android_parser]: https://github.com/google/skia/blob/master/src/ports/SkFontMgr_android_parser.cpp
[dwrote]: https://crates.io/crates/dwrote
[directwrite crate]: https://crates.io/crates/directwrite
[DWrite font enumeration]: https://docs.microsoft.com/en-us/windows/desktop/directwrite/font-enumeration
[Skia Windows font manager]: https://github.com/google/skia/blob/master/src/ports/SkFontMgr_win_dw.cpp
[IDWriteFontFallback]: https://docs.microsoft.com/en-us/windows/desktop/api/dwrite_2/nn-dwrite_2-idwritefontfallback
[IDWriteTextAnalysisSource]: https://docs.microsoft.com/en-us/windows/desktop/api/dwrite/nn-dwrite-idwritetextanalysissource
[NSFontManager]: https://developer.apple.com/documentation/appkit/nsfontmanager
[CTFontCreateForString]: https://developer.apple.com/documentation/coretext/1509506-ctfontcreateforstring
[chromium#586517]: https://bugs.chromium.org/p/chromium/issues/detail?id=586517
