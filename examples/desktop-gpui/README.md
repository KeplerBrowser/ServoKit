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

## Notes

- The visible chrome, layout, focus target, and input hooks are GPUI-owned; Servokit owns the Servo webview lifecycle.
- The app now calls `servokit::runtime::ensure_default_rustls_crypto_provider()` at startup, and Servo creation also re-checks it as a safety net. Embedders that need a different rustls provider must call `servokit::runtime::install_rustls_crypto_provider(...)` before creating any Servo-backed surfaces or webviews.
- The macOS child-surface glue comes from `servokit::surface::macos::AppKitChildSurface`: the host supplies the parent view handle plus logical bounds/scale, and ServoKit keeps the browser child `NSView` aligned for native child-surface embedding. Keep this helper framed as a macOS proof-surface utility in `servokit::surface::macos`, not as a stable production component crate or a `servokit-gpui` SDK.
- `examples/desktop-gpui` is a standalone Cargo root with its own `Cargo.lock` and no longer needs its own direct `rustls` dependency just to install the default provider.
- The GPUI manifest owns the temporary `stylo_derive` 0.18.0 compatibility patch because GPUI's dependency graph brings in `serde_fmt` and triggers the Servo/Stylo `ToCss` derive ambiguity. Core crates and non-GPUI examples do not use this patch; see [`../../docs/rust-dependency-baseline.md`](../../docs/rust-dependency-baseline.md).
