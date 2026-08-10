# Upstream Servo

## Why this repo tracks upstream closely

This package is intentionally a thin host around Servo. When changing the native runtime, upstream Servo should be the source of truth before introducing local workarounds.

Start with these upstream areas:

1. `ports/servoshell` for app-level embedding, lifecycle, and embedder-control handling
2. `support/android/apk/servoview` and `support/android/apk/servoapp` for Android host and demo-app patterns
3. `components/shared/embedder/embedder_controls.rs` for the engine-owned embedder-control request and response model
4. `components/servo/webview_delegate.rs` for the wider delegate surface beyond embedder controls
5. Published crate documentation such as [`servo` on docs.rs](https://docs.rs/servo/latest/servo/) and [crates.io](https://crates.io/crates/servo)

## Submodule policy

The repository keeps `upstream/servo` as a **pinned git submodule**.

That submodule exists so contributors can:

- inspect the exact upstream codebase we are comparing against
- keep low-level Android and desktop-host wiring close to current Servo patterns
- diff local runtime decisions against a stable upstream commit instead of a moving target

The submodule is a **reference checkout**, not the primary build dependency surface for this package. The runtime still embeds Servo through the Rust crate dependency path rather than compiling this repo directly against the submodule tree.

## Published Rust baseline

The current ServoKit release baseline is the crates.io `servo` crate pinned to
`=0.3.0` in the Rust workspace manifests. ServoKit is therefore claiming a
published crates.io baseline, not a temporary git/tag dependency and not a build
against the `upstream/servo` submodule. The submodule remains reference-only.

If ServoKit intentionally moves to a different Servo baseline later, update the
workspace pins, lockfiles, and baseline docs together; see
[`rust-dependency-baseline.md`](./rust-dependency-baseline.md).

## Why pinned instead of floating

Keeping a pinned upstream revision makes native debugging and architecture review easier:

- code review can refer to a known upstream tree
- local docs can talk about a concrete upstream snapshot
- runtime refactors can be audited against the same baseline across multiple commits

When we intentionally realign with a newer upstream snapshot, update the submodule revision and any docs that describe the comparison points.

## Common commands

```sh
git submodule update --init --recursive upstream/servo
git submodule status
```

If you are only consuming the package, you usually do not need to work inside the submodule. It matters most when touching the native runtime, embedder controls, or architectural docs.

## Reuse boundary

The current project direction is:

- **reuse upstream patterns and low-level host wiring**
- **avoid delegating policy-heavy embedder behavior wholesale to `servoview`**

That means upstream support is a strong reference for:

- surface ownership
- render loop and wakeup patterns
- IME baseline behavior
- JNI and host lifecycle wiring

But this repo should keep ownership of higher-policy embedder controls such as:

- file picker flows
- context menus
- permissions
- dialog requests
- React Native-facing policy decisions
