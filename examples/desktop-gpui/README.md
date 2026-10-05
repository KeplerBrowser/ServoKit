# GPUI example (macOS)

A [GPUI](https://www.gpui.rs) desktop app that shows a Servo page in one slot
of its layout. GPUI owns the window, chrome, and layout. ServoKit owns the
Servo web view, which draws into a child `NSView` placed over that slot.

This is the "part of a layout" example. For the simpler whole-window case,
see [`desktop-winit`](../desktop-winit/README.md).

## How it works

- The app follows the usual GPUI shape: `Application::new().run(...)`,
  `App::open_window`, and a root `Render` entity. It uses crates.io `gpui`
  0.2.2 with `runtime_shaders`, so it runs from this repository without a Zed
  checkout.
- `servokit::surface::macos::AppKitChildSurface` creates and updates the
  child `NSView`. The app passes the GPUI window's parent view, the slot's
  bounds from layout, and the scale factor.
- A GPUI task runs ServoKit updates. In-slot pointer, wheel, keyboard, and
  focus events are forwarded as `HostInputEvent`s.
- The footer shows the URL, load status, title, and the latest ServoKit event.

Only the public `servokit` crate is used.

## Run

Start the test pages from the repository root:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Run the example (the environment variables just speed up the build):

```sh
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml
```

Pass a URL after `--` to open another page, for example
`-- https://servo.org/`.

With the test pages running, the Servo slot shows **Smoke fixtures ready**.
Resize the window: the Servo view should stay aligned with its slot, with
GPUI's chrome around it. The footer updates as you navigate, focus, type,
resize, or hit errors.

## Shutdown check

The `shutdown` example loads a page, then closes its window or quits the app.
Because this test app exits when its only window closes, both paths end its
use of Servo. That is this example's choice, not a ServoKit rule: an app that
keeps running can close a window's views and keep the engine for later views.

The example cancels its update task and calls `Runtime::shutdown` while the
native view and logging are still alive. Run both paths from the repository
root:

```sh
for exit_path in --close-window --app-quit; do
  RUST_LOG=warn RUST_BACKTRACE=1 RUSTC_WRAPPER=sccache \
  CARGO_PROFILE_DEV_DEBUG=0 FREETYPE2_NO_PKG_CONFIG=1 \
  cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml \
    --example shutdown -- "$exit_path"
done
```

Each run must exit successfully and print that the task was cancelled, the
runtime finished while the view was alive, the wake target was released, and
the Rust host destructor ran. AppKit may terminate without returning through
Rust's `main`, so a `main-returned` line is not required.

Repeat each path with these options, alone and together:

- `--logger-after-page`: set up logging after the page loads instead of
  before startup.
- `--unattached-replacement`: drop the first runtime normally, then shut down
  through a fresh runtime with no surface attached. This checks that a
  leftover engine is retired.

`--retain-runtime` is a control that is meant to fail: it leaves engine
cleanup to thread-local teardown and reproduces a tracing `AccessError`. Keep
it out of passing checks, and don't hide it by silencing logs or skipping
destructors.

The shutdown rules are in
[Surfaces](../../docs/concepts/surfaces.md#closing-views-and-shutting-down).

## Notes

- The app calls `servokit::runtime::ensure_default_rustls_crypto_provider()`
  at startup, and Servo creation checks it again. To use a different `rustls`
  provider, call `servokit::runtime::install_rustls_crypto_provider(...)`
  before creating any Servo surface or web view.
- `AppKitChildSurface` is a macOS helper for examples and proofs. It is not a
  GPUI component crate or a `servokit-gpui` SDK.
- This folder is its own Cargo root with its own `Cargo.lock`.
- The manifest applies two temporary patches, `stylo_derive` and
  `zed-font-kit`, to work around a `ToCss` derive ambiguity and a FreeType
  version conflict with Servo. Your own GPUI app needs both in its own Cargo
  root, because Cargo does not inherit patches. See
  [Dependencies](../../docs/reference/dependencies.md#local-patches).
