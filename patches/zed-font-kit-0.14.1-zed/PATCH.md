# `zed-font-kit` 0.14.1-zed compatibility patch

This is the published crates.io `zed-font-kit` 0.14.1-zed source, whose package
metadata identifies upstream revision
[`110523127440aefb11ce0cf280ae7c5071337ec5`](https://github.com/zed-industries/font-kit/commit/110523127440aefb11ce0cf280ae7c5071337ec5).
The original MIT and Apache-2.0 licenses are retained. Cargo's generated lockfile
and cache markers, and upstream CI configuration, are not part of this copy.

## Why it exists

Published GPUI 0.2.2 depends on this font package. Its FreeType 0.20 dependency
conflicts with Servo 0.6's FreeType 0.23 dependency: Cargo allows only one package
with `links = "freetype"`, even when building on macOS, where GPUI uses CoreText.

Only the active `Cargo.toml` differs from the published package. It applies the
three manifest changes from upstream font-kit's
[`868f28a2c60d36092be66e4d83db001267c9d6b4`](https://github.com/servo/font-kit/commit/868f28a2c60d36092be66e4d83db001267c9d6b4):
use `freetype-sys` 0.23 instead of the optional `freetype` wrapper, select that
dependency in `loader-freetype`, and align the non-Apple FreeType dependency to
0.23. `Cargo.toml.orig` is retained unchanged for provenance.

No Rust sources or CoreText/DirectWrite dependencies are changed. ServoKit's
supported GPUI path is macOS; this patch does not establish a non-macOS GPUI
support guarantee.

## Root scope and removal

The standalone GPUI example selects this patch at its Cargo root. Cargo does not
inherit dependency patches, so an external application combining GPUI 0.2.2 and
ServoKit must select this package at its own Cargo root as well. The main
ServoKit workspace and the Android build do not use this font-package patch.

Remove this copy and its root patch entry when the GPUI dependency graph uses a
published font package with a compatible FreeType dependency. Confirm the
unpatched graph resolves and the supported macOS GPUI build and rendering smoke
pass before removing it.
