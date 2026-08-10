# Desktop winit example

`examples/desktop-winit` is the desktop `winit` proof app. It is an example app,
not a Servokit-provided host crate: the app owns `EventLoop::run_app`, its
`ApplicationHandler`, the native `winit::Window`, resize/input handling, and the
stdout event trace. It uses the public `servokit` surface facade with the
`servo` feature enabled, so Servokit owns the Servo
webview/delegate/embedder-control/render integration.

The example opens a native window, attaches it to a Servokit webview through a
`HostSurface` and `SurfaceViewport`, loads an initial URL, pumps updates from the
winit loop, forwards basic pointer/keyboard/wheel/IME/focus input, and prints
shared URL/load/focus/surface events.

On macOS this is the whole-window native-child proof path: `winit` owns the
native AppKit-backed window, while ServoKit receives borrowed display/window
handles through `servokit::surface::NativeChildSurface`. The separate
[`desktop-gpui`](../examples/desktop-gpui/README.md) example proves the AppKit
child `NSView` layout-slot path through
`servokit::surface::macos::AppKitChildSurface`; both paths exercise the same
host-neutral ServoKit surface facade.

## Run commands

From the repository root, optional fixture server:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Run the desktop example:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- http://127.0.0.1:8481/smoke/index.html
```

If no URL is supplied, the example uses the fixture smoke index URL above.
Passing another URL is supported:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- https://servo.org/
```

Run deterministic smoke mode:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- --smoke http://127.0.0.1:8481/smoke/index.html
```

Smoke mode opens the same native window and uses the same Servokit surface/event
path as interactive mode, but it exits after the first `Complete` load status or
a timeout. The default timeout is 30 seconds; override it with
`--smoke-timeout-ms <milliseconds>`. The process exits `0` when a page reaches
`Complete` and exits `1` on timeout, Servo error, Servo crash, or early window
close. Normal interactive behavior is unchanged when `--smoke` is not supplied.

## Lockfile and update workflow

`examples/desktop-winit` is a standalone Cargo root, so `--locked` uses the
example-local `examples/desktop-winit/Cargo.lock` file. Its manifest depends on
repo-local `servokit` via `servokit = { path = "../../crates/servokit" }` with
the `servo` feature, and it carries only the local `tikv-jemalloc-sys` patch
needed by the Servo graph. It does not patch `stylo_derive`; that temporary
patch is GPUI-only. The canonical Servo 0.3.0 baseline, patch strategy, and
`cargo update -p ... --precise ...` workflow live in
[`rust-dependency-baseline.md`](./rust-dependency-baseline.md).

## Expected result

On a supported desktop machine, the example opens a native `winit` window and
renders the initial URL as Servo web content. With the fixture server running,
the page body should visibly show the shared smoke fixture, including `Smoke
fixtures ready`.

The event flow is printed to stdout, for example:

```text
webview=1 surface attached 960x640
webview=1 load=Started
webview=1 url=http://127.0.0.1:8481/smoke/index.html
webview=1 focus=true
webview=1 load=Complete
```

Smoke mode prints the same event trace plus a deterministic start/result summary:

```text
smoke mode=enabled platform=macos/aarch64 url=http://127.0.0.1:8481/smoke/index.html timeout_ms=30000
webview=1 surface attached 960x640
webview=1 load=Complete
webview=1 surface detached
smoke result=pass platform=macos/aarch64 target_url=http://127.0.0.1:8481/smoke/index.html last_url=http://127.0.0.1:8481/smoke/index.html load_status=Complete surface_attached=960x640 surface_resized=none elapsed_ms=1234 errors=0 reason=none
```

If Servo emits an error or crash event, smoke mode keeps the event detail in the
stdout trace and adds `smoke error[...]` lines before exiting `1`.

Links, scrolling, focus changes, text input, and window resize are routed through
the public Servokit surface facade,
`servokit::runtime::Runtime<servokit::surface::SurfaceHost<_>>`. The example
also calls `servokit::runtime::ensure_default_rustls_crypto_provider()` at
startup so it does not rely on every app rediscovering rustls' process-default
provider requirement; apps that need a different provider should call
`servokit::runtime::install_rustls_crypto_provider(...)` before creating Servo surfaces or webviews. The example-local glue stays thin: it supplies
borrowed `winit` raw-window handles through
`servokit::surface::NativeChildSurface`, dispatches `HostInputEvent` values,
calls `Runtime::perform_updates`, and drains `ServokitEvent` values.

## Resize reflow smoke

The smoke index includes a `Viewport reflow` card for manual resize checks. Run
the fixture server and desktop example, then drag the native window narrower and
wider across the breakpoint. Passing means the visible `Viewport <width> x
<height>` text changes, the breakpoint label switches between `narrow` and
`wide`, and stdout prints matching `PageTitleChanged` values from the fixture's
resize handler. `SurfaceResized` stdout events alone are not a pass signal for
content reflow.
