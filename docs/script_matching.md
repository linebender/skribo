One central task for skribo is choosing a font most suitable for the requested script. The requested script is a combination of an explicit lang tag and locale system settings.

Part of what makes this requirement tricky is that there is a fuzzy line between the scope of skribo and the scope of [font-kit]. Conceptually, skribo should be a consumer of font metadata, and font-kit is a provider. At a finer grain, there are decisions about exactly what that interface should look like. The main purpose of this document is to describe the problems to be solved.

## Background

Selection of font is not strictly based on Unicode coverage, but is also affected by locale. The biggest impact is on Han unification, but there are other examples. Suitability of a font for a script is *not* encoded in the font's tables, but is metadata about the system's font configuration. Mechanisms for accessing this metadata vary widely from platform to platform.

A significant amount of research for this topic was reading through the [Skia] code, as that's the platform abstraction used by Chromium, so can be used as a rough first draft of requirements for correct cross-platform text layout for the Web. I also used Gecko and Qt as sources.

The main focus of this document is font fallback, but a related platform-specific concern is resolution of generic aliases (system UI font, generic serif, etc). This may be out of scope for skribo itself, but likely within the scope of font-kit. More clarity on requirements for each crate is needed.

TODO: write a clear requirement, link to this section of requirements doc.

## BCP-47

I strongly recommend the use of BCP-47 as the identifier for language, script, and other locale metadata. This is an easy decision for web use cases, as it is the standard for the [lang] tag. The main challenge is that mechanisms for system font metadata in general predate BCP-47, so there will be some impedance matching.

TODO: investigate Rust ecosystem for common BCP-47 tag representation.

## Platform-specific considerations

Each platform handles access to metadata (including script) in a different way. Platforms generally treat their text stack as an opaque black box, and vary in how much low-level access they give.

### Fontconfig

The [Fontconfig] configuration file format specifies "langset" as an an "RFC-3066-style" language. [RFC 3066] is a predecessor to BCP-47 (dated 2001), and basically specifies language and country, with no provision for explicit script or variant. For the purpose of Han unification, the convention is to infer script from country. For example, "zh-CN" could be translated to "zh-Hans", "zh-TW" to "zh-Hant". However, after a little investigation, it's not clear to me how useful it is to do sophisticated processing here, as the default fontconfig on a clean Debian 9 install lists doesn't specify "langset" attributes, but just has a few informal descriptions in comments.

The fontconfig API includes [lang tags](https://www.freedesktop.org/software/fontconfig/fontconfig-devel/fcpatternadd-type.html) in its query interface (see [Qt source](https://github.com/qt/qtbase/blob/5.12/src/platformsupport/fontdatabases/fontconfig/qfontconfigdatabase.cpp#L715) for a good example of the usage). It seems that using this interface could do font matching based on script metadata, but I wasn't able to find evidence that Linux distros actuall ship useful config data. I did find a [blog post](https://spin.atomicobject.com/2015/10/01/localization-font-selection-fontconfig/) on how users can write config files themselves, and [this blog](https://utcc.utoronto.ca/~cks/space/blog/linux/LinuxXTermFreeTypeCJKFonts) has a snippet of a font config from Fedora that suggests it might be a bit more CJK aware.

I request more input from CJK Linux users about what's going on here and the best way to handle it going forward.

Also note that fontconfig *almost* provides accurate aliases for system fonts, with the exception of kde. In those cases, kde does not update fontconfig, but rather stores the preference in `.config/kdeglobals`. Refer to the [Qt code theme code](https://github.com/qt/qtbase/blob/5733dfbd90fd059e7310786faefb022b00289592/src/platformsupport/themes/genericunix/qgenericunixthemes.cpp#L385) for a reference of how to resolve these.

### Windows

Windows is a tricky case. The [DWrite font enumeration] functions don't provide detailed metadata on script coverage. The recommended platform-native way to resolve font matching is to use [IDWriteFontFallback] to map characters, with an [IDWriteTextAnalysisSource] to specify locale and other details. The [Skia Windows font manager] uses this interface.

It's not clear to me whether Blink actually goes through SkFontMgr in this case, or whether it uses [its own logic](https://cs.chromium.org/chromium/src/ui/gfx/font_fallback_win.cc); there seems to be considerable duplication of concerns in the code base. In any case, the Blink code follows generally the same strategy, creating a TextAnalysisSource object containing locale information and then using a FontFallback object to map the characters.

The current font-kit code uses the font enumeration and not the text analysis APIs. Thus, getting full functionality will likely require significant architectural rework.

As far as I can tell, there are no Rust wrappers for these fallback and text analysis winapi functions.

As an alternative approach, Qt seems to have [hardcoded lists of fonts](https://github.com/qt/qtbase/blob/5.12/src/platformsupport/fontdatabases/windows/qwindowsfontdatabase.cpp#L1686) for Han unification. My evaluation of the Qt code is that it's fairly crufty and probably not a great reference.

(Side note: font-kit uses [dwrote], while the driud/piet infra uses the [directwrite crate]. We should dedup this or at the very least make interop painless)

### macOS

The current font-kit code uses the availableFontFamilies call in [NSFontManager], which reports just the names without any additional metadata.

Skia's macOS implementation of font selection (the onMatchFamilyStyleCharacter) is based on [CTFontCreateForString], architecturally a similar approach as on Windows. However, that doesn't seem to be sensitive to the bcp47 argument.

Based on some more investigation, I think it's likely that Blink uses [CTFontCopyDefaultCascadeListForLanguages] as the primary mechanism to determine fallback fonts on macOS (the call seems to be in [ui/gfx/font_fallback_mac.mm](https://cs.chromium.org/chromium/src/ui/gfx/font_fallback_mac.mm?l=42)). Based on this, it seems like the SkFontMgr interface is not consistently used to determine fallback fonts on all platforms, and specifically on macOS it's done in Blink instead. Another useful trail of breadcrumbs to follow might be [chromium#586517].

The Gecko codebase currently uses a very different approach. It stores a list of known platform fonts along with lang+script metadata in the Firefox preferences system, with platform-specific [https://searchfox.org/mozilla-central/source/modules/libpref/init/all.js#4124] initial values, but also the flexibility for users to override. There are other places in the code (starting from [gfxFontGroup::FindFontForChar]) where there is font-specific additional logic. There's a bit more detail in the comments [of the PR landing this document](https://github.com/linebender/skribo/pull/1).

Gecko also explored [mozilla bug 1212731](https://bugzilla.mozilla.org/show_bug.cgi?id=1212731) a CTFontCopyDefaultCascadeListForLanguages based approach, but reverted it because of performance and compatibility concerns. (See also [mozilla bug 1418724](https://bugzilla.mozilla.org/show_bug.cgi?format=default&id=1418724) for more breadcrumbs)

Qt [uses](https://github.com/qt/qtbase/blob/5.12/src/platformsupport/fontdatabases/mac/qcoretextfontdatabase.mm) the CTFontCopyDefaultCascadeListForLanguages approach.

It thus seems an open question which strategy is best.

### Android

Android has a complete and up-to-date listing of font metadata, including BCP-47 language tags, in its [fonts.xml] file. This format is quasi-standard; the only officially supported way to access fonts is through the Java APIs, but that's not adequate for low-level needs such as a Web browser, so the Android frameworks team has historically made some effort not to break this file format. Note that the format evolves a bit over time, in particular Lollipop was a significant difference. The file also has a threatening sounding notice suggesting that it will go away in an imminent release; I'm not sure what they have in mind to replace it.

The best reference for parsing fonts.xml is [Skia], as that's used to configure fonts in Chrome. In particular, its parsing logic is in [SkFontMgr_android_parser].

A major additional possible complication on Android is [downloadable fonts], which manifest as additional fonts that are not listed in fonts.xml. Access to these is through Java-language interfaces only, and the resulting [Typeface] object does not give documented access to the underlying font files. I was not able to find evidence that Chromium supports this feature, so likely it is out of scope, at least for an initial implementation.

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
[gfxFontGroup::FindFontForChar]: https://searchfox.org/mozilla-central/rev/92d11a33250a8e368c8ca3e962e15ca67117f765/gfx/thebes/gfxTextRun.cpp#2667
[CTFontCopyDefaultCascadeListForLanguages]: https://developer.apple.com/documentation/coretext/1509992-ctfontcopydefaultcascadelistforl
[downloadable fonts]: https://developer.android.com/guide/topics/ui/look-and-feel/downloadable-fonts
[Typeface]: https://developer.android.com/reference/android/graphics/Typeface.html
