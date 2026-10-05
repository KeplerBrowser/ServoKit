# Desktop (Rust)

A native Rust app can embed Servo with the `servokit` crate. Your app keeps
its window, event loop, and layout. ServoKit draws pages into a window, a
child view, or an offscreen buffer you provide.

| OS | Status |
| --- | --- |
| macOS | Tested: whole windows, child views in a layout, several views, GPU frame export, and system WebKit views |
| Windows | Should build through `winit`, but not verified: no CI or recorded build yet. |
| Linux | Should build through `winit`, but not verified. No baseline X11 or Wayland setup chosen yet. |

## Add ServoKit to your app

ServoKit is not on crates.io yet. Depend on it by path or Git, with the `servo`
feature:

```toml
[dependencies]
servokit = { path = "../ServoKit/crates/servokit", features = ["servo"] }
```

The examples also apply the repository's `tikv-jemalloc-sys` patch so their
Servo graph matches the workspace. Copy it from
`examples/desktop-winit/Cargo.toml` if you build the same way. See
[Dependencies](../reference/dependencies.md).

Servo uses `rustls`. Call this once at startup so you don't depend on a
process-wide default being found:

```rust
let _ = servokit::runtime::ensure_default_rustls_crypto_provider();
```

If you need a different provider, call
`servokit::runtime::install_rustls_crypto_provider(...)` before creating any
surface or web view.

## Embed a page in five steps

The [`desktop-winit`](../../examples/desktop-winit) example is the full,
working version of these steps.

**1. Give ServoKit your window.** Implement `SurfaceDelegate` and return a
`NativeSurface` built from your window's handles:

```rust
impl SurfaceDelegate for WinitSurface {
    type Frame<'a> = SurfaceFrame<'a>;

    fn render_target(
        &mut self,
        _surface: &HostSurface,
        _viewport: SurfaceViewport,
    ) -> Result<SurfaceTarget<'_>, SurfaceError> {
        let display = self.window.display_handle().map_err(|e| SurfaceError::new(e.to_string()))?;
        let window = self.window.window_handle().map_err(|e| SurfaceError::new(e.to_string()))?;
        Ok(NativeSurface::new(display, window).into())
    }
}
```

**2. Create the runtime.** Pass a waker that wakes your event loop, and a
clipboard:

```rust
let options = SurfaceHostOptions::new(waker, Rc::new(MemoryClipboard::default()));
let mut runtime = Runtime::new(SurfaceHost::new(WinitSurface { window }, options));
```

**3. Create a view, load a page, and attach the window:**

```rust
let session = runtime.create_session();
let view = runtime.create_webview(session)?;
runtime.load_url(view, "https://servo.org")?;
runtime.attach_surface_with_viewport(view, HostSurface::new("main"), viewport)?;
```

A `SurfaceViewport` carries the size in pixels and the scale factor, for
example `SurfaceViewport::new(SurfaceSize::new(w, h), scale)`.

**4. Drive it from your event loop.** Call `perform_updates` when woken and
on redraw, then handle events. Answer every navigation request, or the page
will wait:

```rust
runtime.perform_updates(view)?;
for event in runtime.drain_events() {
    match event.event {
        HostEvent::PageTitleChanged { title } => set_title(title),
        HostEvent::NavigationRequested { navigation_id, .. } => {
            runtime.resolve_navigation_request(event.webview, &navigation_id, true)?;
        }
        _ => {}
    }
}
```

Send input that belongs to the page with `runtime.dispatch_input_event(view,
HostInputEvent::...)`, using coordinates relative to the view. Send a new
viewport when the window resizes.

**5. Clean up.** Detach or destroy views before you close their native
windows. When your app is done with Servo for good, call `runtime.shutdown()`
on the thread that owns the engine (the main thread in these examples), before
you destroy windows or logging. Servo cannot restart in the same process.

The rules behind each step are in [Surfaces](../concepts/surfaces.md) and
[Runtime and views](../concepts/runtime.md).

## Examples

### `desktop-winit`: a whole window

The app owns the `winit` event loop, window, input, and a printed event log.
ServoKit gets the window's handles through `NativeSurface`.

```sh
python3 -m http.server 8481 --directory examples/fixtures   # optional test pages
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- https://servo.org/
```

Without a URL, it loads `http://127.0.0.1:8481/smoke/index.html` and shows
**Smoke fixtures ready**. Links, scrolling, focus, text input, and resizing
all work. The terminal prints events like these:

```text
webview=1 surface attached 960x640
webview=1 load=Started
webview=1 url=http://127.0.0.1:8481/smoke/index.html
webview=1 focus=true
webview=1 load=Complete
```

Add `--smoke` before the URL for a scripted check that exits with code 0 when
the page finishes loading, or 1 on timeout (30 seconds by default, set with
`--smoke-timeout-ms`), error, crash, or early window close. It prints a final
line such as `smoke result=pass ...`.

To check that pages reflow on resize, drag the window across the test page's
**Viewport reflow** breakpoint. The viewport text and the narrow/wide label
should change, and the terminal should print matching `PageTitleChanged`
values. `SurfaceResized` events alone don't prove the page reflowed.

The example is its own Cargo root with its own `Cargo.lock`.

### `desktop-gpui`: one part of a layout (macOS)

[GPUI](https://www.gpui.rs) apps own their window and render a layout tree, so
ServoKit cannot open a window of its own. This example puts Servo in one slot
of a GPUI layout:

- GPUI owns the window, chrome, layout, focus, and input hooks.
- `servokit::surface::macos::AppKitChildSurface` creates a child `NSView` for
  the slot. The example sets its frame from GPUI's layout bounds and scale
  factor during prepaint.
- The example runs ServoKit updates from a GPUI task, forwards in-slot pointer,
  wheel, keyboard, and focus events as `HostInputEvent`s, and shows URL, load,
  title, and event status in a GPUI footer.

It uses crates.io `gpui` 0.2.2 with the `runtime_shaders` feature, and needs
two temporary dependency patches. Run commands and the shutdown check are in
[its README](../../examples/desktop-gpui/README.md).

### `multiple-native-views`: several pages (macOS)

`crates/servokit/examples/multiple-native-views` creates two independent pages
in separate native views, then closes, replaces, and recreates them on one
engine. See [Testing and validation](../reference/testing.md#multiple-views-on-macos).

## More desktop features

- **Offscreen rendering:** draw pages into a buffer and composite them
  yourself, with CPU readback everywhere or zero-copy GPU frames on macOS. See
  [Surfaces](../concepts/surfaces.md#surface-types).
- **System WebKit views on macOS:** pick Servo or `WKWebView` per view, for
  sites Servo can't handle yet. See
  [macOS system web view](../reference/macos-system-webview.md).
- **Persistent profiles:** keep cookies and storage in a directory you choose.
  See [Runtime and views](../concepts/runtime.md#profiles-and-persistent-storage).

## Known limits

- The AppKit child-view helper is meant for examples and proofs. It is not a
  GPUI component crate, and ServoKit has no `servokit-gpui` crate.
- Child views are a working path, but not necessarily the fastest one. Exact
  zero-copy GPUI texture integration is still platform-specific work.
- GPUI is pre-1.0.
- Servo marks `WindowRenderingContext::set_window` and `take_window`, which
  ServoKit uses for attach and detach, as temporary upstream APIs. ServoKit
  will follow Servo when they change.
