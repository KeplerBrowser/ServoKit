# Servokit desktop GPUI example

A GPUI desktop app that embeds Servo through Servokit inside a GPUI layout slot.
GPUI owns the native window and surrounding layout; `servokit::surface::macos::AppKitChildSurface` supplies the AppKit child `NSView` for the slot, and Servokit owns the Servo webview attached to that native child surface.

Template alignment: this example follows the `create-gpui-app`/GPUI app shape (`Application::new().run(...)`, `App::open_window`, root `Render` entity), but uses crates.io `gpui` 0.2.2 with `runtime_shaders` instead of the Zed git workspace template so users can run it from this repository without a second GPUI workspace. It intentionally keeps GPUI as the app/layout framework and uses only the public `servokit` facade for Servo embedding.

## Usage

Start the fixture server from the repository root:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Run the GPUI example:

```sh
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml
```

You can pass a custom URL:

```sh
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml -- https://servo.org/
```

## Expected macOS smoke result

With the fixture server running, use the smoke fixture URL for the manual macOS
proof:

```sh
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml -- http://127.0.0.1:8481/smoke/index.html
```

The expected visual marker is `Smoke fixtures ready` inside the Servo-rendered
browser slot. Resize the GPUI window and confirm the Servo child `NSView` stays
aligned to the GPUI layout slot, with GPUI chrome still surrounding it. The
footer is the event output surface for this example: it should update with the
current URL, load status/title, and latest `ServokitEvent` status as navigation,
focus/input, resize, or error/crash events arrive.

This example is the AppKit child-view proof path for macOS. The simpler
[`desktop-winit`](../../docs/desktop-winit.md) example is the whole-window
native-child proof path; both use the same ServoKit `NativeChildSurface` facade.

## Shutdown regression

The separate `shutdown` example loads a self-contained page, then requests native
window close or application quit. It cancels its update task and calls
`Runtime::shutdown` while the native surface and logging remain alive. Run both
paths from the repository root:

```sh
for exit_path in --close-window --app-quit; do
  RUST_LOG=warn RUST_BACKTRACE=1 RUSTC_WRAPPER=sccache \
  CARGO_PROFILE_DEV_DEBUG=0 FREETYPE2_NO_PKG_CONFIG=1 \
  cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml \
    --example shutdown -- "$exit_path"
done
```

Each process must exit successfully. Expect output confirming task cancellation,
runtime finalization with the surface alive, wake-target release, and execution
of the Rust host destructor. AppKit termination need not return through Rust
`main`, so a `main-returned` marker is not required.

Repeat each exit path with these options, individually and together:

- `--logger-after-page` initializes tracing/log forwarding after Servo has loaded
  the page, instead of before application startup. Neither mode emits a deliberate
  logger warmup event.
- `--unattached-replacement` drops the original runtime normally, then finalizes a
  fresh runtime without attaching another surface. This checks retirement of the
  retained process engine.

`--retain-runtime` is a separate, deliberately failing control: ordinary runtime
drop leaves final engine destruction to TLS teardown and reproduces the tracing
`AccessError`. Keep it out of successful smoke gates; do not suppress logs, warm
up formatter TLS, or bypass destructors to make it pass.

The [surface lifecycle contract](../../docs/surface-modes.md#view-destruction-and-final-shutdown)
defines the public shutdown behavior and native resource ordering.

## Notes

- The visible chrome, layout, focus target, and input hooks are GPUI-owned; Servokit owns the Servo webview lifecycle.
- The app now calls `servokit::runtime::ensure_default_rustls_crypto_provider()` at startup, and Servo creation also re-checks it as a safety net. Embedders that need a different rustls provider must call `servokit::runtime::install_rustls_crypto_provider(...)` before creating any Servo-backed surfaces or webviews.
- The macOS child-surface glue comes from `servokit::surface::macos::AppKitChildSurface`: the host supplies the parent view handle plus logical bounds/scale, and ServoKit keeps the browser child `NSView` aligned for native child-surface embedding. Keep this helper framed as a macOS proof-surface utility in `servokit::surface::macos`, not as a stable production component crate or a `servokit-gpui` SDK.
- `examples/desktop-gpui` is a standalone Cargo root with its own `Cargo.lock` and no longer needs its own direct `rustls` dependency just to install the default provider.
- The GPUI manifest owns the temporary `stylo_derive` 0.18.0 compatibility patch because GPUI's dependency graph brings in `serde_fmt` and triggers the Servo/Stylo `ToCss` derive ambiguity. Core crates and non-GPUI examples do not use this patch; see [`../../docs/rust-dependency-baseline.md`](../../docs/rust-dependency-baseline.md).
