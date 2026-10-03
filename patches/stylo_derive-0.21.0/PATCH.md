# `stylo_derive` 0.21.0 patch

This directory is a Cargo `[patch.crates-io]` replacement for `stylo_derive`
0.21.0, used by the standalone GPUI example under ServoKit's crates.io `servo`
`=0.6.0` baseline. It carries forward the existing 0.18.0 workaround onto the
matching published source; the GPUI graph still reproduces the ambiguity
without it.

## Why it exists

Combining Servo with GPUI enables structured `log`/`serde_fmt` support in the
dependency graph. That makes some generated `ToCss` implementations from
`stylo_derive` ambiguous around `?` error conversion (`std::fmt::Error` can be
constructed both from itself and from `serde_fmt::Error`). The patch changes the
generated `fmt::Result` propagation in `to_css.rs` to explicit
`match`/`return Err(error)` forms.

## Scope

Only `to_css.rs` differs from the crates.io source. Cargo cannot apply a
standalone diff file as a patch during normal builds, so the replacement must be
a minimal local crate copy.

Regenerate the diff with:

```sh
diff -u \
  ~/.cargo/registry/src/index.crates.io-*/stylo_derive-0.21.0/to_css.rs \
  patches/stylo_derive-0.21.0/to_css.rs
```

Long-term, remove this local patch when the Servo/Stylo crates.io graph no
longer emits ambiguous `ToCss` error propagation for the GPUI example.
