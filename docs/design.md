# Skribo design and roadmap

My work on skribo has been to get a working prototype and flesh out the design. I have other commitments coming up soon, and will transition from focusing on skribo to handing it off to the community. The main purpose of this document is to document the work remaining to be done.

Some of the work is in issues, and I’ll point to those. In other cases, I’ll point to existing open source code bases as guidance for how to complete the remaining features.

## font-kit issues

The line between font-kit and skribo is more than a bit blurry, as certain things such as resolving system fallback fonts involve both.

In general, this is how things *should* be: font-kit should be an abstraction over platform-specific font enumeration and loading, and should contain only the platform-independent logic needed to support such an abstraction. To this end, it would be good to move font matching out of font-kit and into skribo.

A major potential performance issue is handling the fonts that come back from the `get_fallbacks` call. As of now, as these fonts are wrapped for HarfBuzz use, the font data is copied into a `Vec` each time. The solution is not trivial, and likely involves caching the wrapped fonts. In order for the cache to work, a reliable font id is needed for the cache key, captured in [font-kit#40](https://github.com/pcwalton/font-kit/issues/40). It might also be possible to reduce the wrapping overhead. For example, on macOS it’s almost certainly going to be better to wire up the [HarfBuzz CoreText functions](https://github.com/harfbuzz/harfbuzz/blob/master/src/hb-coretext.h) rather than rely on `Vec<u8>` font data (this might be done in skribo, we could probably just use the existing `NativeFont` interface.

Implementations of `get_fallbacks` are also required for macOS and Linux.

## substring layout

It’s possible to reuse an existing layout using the unsafe_for_breaks flags. This should be strictly a performance improvement, which is why I did not prioritize it.

## script-based font fallback

We need to apply the given locale. Go from locale list to single locale based on script detected in script run.

Also set HB language based on matching script - [Minikin source](https://android.googlesource.com/platform/frameworks/minikin/+/refs/heads/master/libs/minikin/Layout.cpp#652)

## Additional functions

The current skribo codebase is focused on iterating through glyphs, which is important for rendering, but a lot of the time spent will be measurement for the purpose of paragraph layout. In particular, there should be an advance_substr in analogy to iter_substr. This function has the potential to be implemented very efficiently, as in the fast case it doesn’t need to allocate. It should also use binary search, using the assumption that clusters are monotonic. Following https://harfbuzz.github.io/working-with-harfbuzz-clusters.html we should be able to work at level 0 or level 1, but level 2 will not work.

Based on [Minikin][Minikin Layout.h], here are additional useful functions:

* get_bounds: bounding box for all glyphs

* get_offset_for_advance: take an advance value and return the closest character offset

* get_advance: return the advance for the given offset

These functions should probably all take a range for substring queries (and maybe strings to append, for hyphenation).

## letterspacing

One feature applied by skribo after HarfBuzz is letterspacing. A good model to follow in general is [Minikin](https://android.googlesource.com/platform/frameworks/minikin/+/refs/heads/master/libs/minikin/Layout.cpp#633). There are a few considerations.

First, not all scripts support letterspacing. Bascially, any script with joined or continuous glyphs will fare poorly. (As an advanced possible future feature, kashida could be added, but this is not supported by any current browser).

Second, ligatures like “fi” look bad when letterspacing is more than a small amount. Minikin [disables it](https://android.googlesource.com/platform/frameworks/minikin/+/refs/heads/master/libs/minikin/Layout.cpp#570) when the absolute value of letterSpacing is more than 0.03 (note that in Android, letter spacing is provided as a fraction of an em, which I think is a good idea).

Note also that if we’re at level 1 clustering there will likely be zero-advance clusters. The current logic in Minikin (which is at level 0 clustering) applies it to all clusters.

[Minikin Layout.h]: https://android.googlesource.com/platform/frameworks/minikin/+/refs/heads/master/include/minikin/Layout.h

