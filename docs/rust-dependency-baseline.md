# Rust dependency baseline

Servokit currently has **one Rust workspace root** plus **standalone Rust example roots** with checked-in lockfiles:

- `crates/Cargo.toml` + `crates/Cargo.lock` for the ServoKit workspace crates and the Android host crate.
- `examples/desktop-winit/Cargo.toml` + `examples/desktop-winit/Cargo.lock` for the standalone desktop `winit` example.
- `examples/desktop-gpui/Cargo.toml` + `examples/desktop-gpui/Cargo.lock` for the standalone desktop GPUI example.

There is intentionally **no repository-root `Cargo.toml`** and no shared
`examples/Cargo.toml` workspace. Cargo applies `[patch.crates-io]` from the
active manifest/workspace root, so local patches live only on the graphs that
need them.

## Current Servo baseline

The current ServoKit release baseline targets **crates.io `servo` `=0.3.0`**.

That means:

- ServoKit claims the published Servo v0.3 crates.io baseline.
- ServoKit does **not** build against the `upstream/servo` git submodule.
- ServoKit does **not** use a temporary Servo git/tag dependency for the
  release baseline.

Changing the Servo baseline is an explicit follow-up decision, not a routine
lockfile refresh.

## Where direct pins live

Shared direct dependency pins for Servokit crates live in the crate workspace
root instead of being repeated across member manifests:

- `crates/Cargo.toml` centralizes the Servo-facing crate pins used by
  `servokit`, `servokit-embedder`, `servokit-host`, and
  `servokit-host-android`, including the exact `servo = "=0.3.0"` baseline,
  the shared `rustls` provider helper pin, and shared
  `raw-window-handle`/desktop host versions.

Standalone example manifests own their app-facing pins because they are no
longer members of a shared examples workspace:

- `examples/desktop-winit/Cargo.toml` depends on repo-local `servokit` with the
  `servo` feature and pins its direct `winit` dependency.
- `examples/desktop-gpui/Cargo.toml` depends on repo-local `servokit` with the
  `servo` feature and pins its direct GPUI, `raw-window-handle`, and `naga`
  dependencies.

## Local patch strategy

Cargo patches are active-root scoped, so each Cargo root declares only the local
patches that are actually active for its graph. The current active
`[patch.crates-io]` entries are:

- `tikv-jemalloc-sys` → local path patch in `crates/Cargo.toml` and in the
  Servo-backed desktop example manifests (`examples/desktop-winit/Cargo.toml`
  and `examples/desktop-gpui/Cargo.toml`) so Servo's Android/Jemalloc workaround
  stays aligned anywhere those Servo graphs are built directly.
- `stylo_derive` → temporary local path patch in
  `examples/desktop-gpui/Cargo.toml` only. The GPUI dependency graph brings in
  `serde_fmt` through its structured logging stack, which makes Servo/Stylo's
  generated `ToCss` `fmt::Result` propagation ambiguous under Servo `=0.3.0`.

The `stylo_derive` patch is intentionally GPUI-only. Core crates and
`examples/desktop-winit` do not patch
`stylo_derive`; non-GPUI Servo graphs use the crates.io `stylo_derive` package.
The local patch replaces ambiguous `?` conversions in `to_css.rs` with explicit
`match`/`return Err(error)` handling and should be removed when the Servo/Stylo
crates.io graph no longer needs that workaround for the GPUI example.

## Reproducible `--locked` workflow

Readiness commands keep using `--locked`, and each manifest reads the lockfile
next to the root that owns it:

- `cargo test --manifest-path crates/Cargo.toml ... --locked` uses
  `crates/Cargo.lock`.
- `cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked` uses
  `examples/desktop-winit/Cargo.lock`.
- `cargo check --manifest-path examples/desktop-gpui/Cargo.toml --locked` uses
  `examples/desktop-gpui/Cargo.lock`.

To refresh lockfiles intentionally:

```sh
cargo generate-lockfile --manifest-path crates/Cargo.toml
cargo generate-lockfile --manifest-path examples/desktop-winit/Cargo.toml
cargo generate-lockfile --manifest-path examples/desktop-gpui/Cargo.toml
```

To update a non-Servo dependency in a controlled way, update the relevant
manifest and then use `cargo update -p ... --precise ...` against that root, for
example:

```sh
cargo update --manifest-path crates/Cargo.toml -p rustls --precise 0.23.40
cargo update --manifest-path examples/desktop-gpui/Cargo.toml -p naga@26.0.0 --precise 26.0.0
cargo update --manifest-path examples/desktop-winit/Cargo.toml -p winit --precise 0.30.13
cargo update --manifest-path crates/Cargo.toml -p raw-window-handle --precise 0.6.2
```

If the lockfile already contains multiple versions of the same crate,
disambiguate `cargo update -p` with `name@version` so Cargo updates the intended
package.

When a dependency is intentionally shared across multiple roots, update each
relevant manifest and regenerate or refresh the corresponding lockfiles
together.

## Servo baseline changes are special

If ServoKit intentionally moves off crates.io `servo` `=0.3.0`, do all of the
following together:

1. update the Servo pin in `crates/Cargo.toml`;
2. regenerate or refresh `crates/Cargo.lock` and the lockfiles for Servo-backed
   example roots (`examples/desktop-winit/Cargo.lock` and
   `examples/desktop-gpui/Cargo.lock`), plus any other example lockfile whose
   graph changed;
3. rerun the `--locked` readiness commands for crates and desktop examples; and
4. restate any API fallout before claiming the new baseline.

Under the current `=0.3.0` baseline, future Servo API fallout stays deferred
until ServoKit intentionally changes its Servo pin and refreshes the affected
lockfiles with matching validation.
