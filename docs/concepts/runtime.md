# Runtime, views, and the engine

This page explains how ExplorerKit runs Servo: one engine per process, a
`Runtime` that owns your web views, and the rules for threads, profiles,
popups, and shutdown.

## The short version

- Servo runs **once per process**, on **one UI thread**. ExplorerKit starts it
  the first time you need it and keeps it alive.
- A **`Runtime`** owns your web views and routes commands and events to the
  right one. Your app owns layout, selection, and presentation.
- A **web view** (`WebViewHandle`) is one browsing context: a page, its
  history, and its pending prompts. It can exist without a surface.
- **Shutdown is final.** Servo cannot restart in the same process.

## One engine per process

The first successful Servo start keeps one engine on a long-lived UI thread.
Servo's handle cannot move between threads, so ExplorerKit never moves the
engine and never restarts it after that thread exits. Web views and surfaces
live shorter lives and come and go independently.

The host keeps its engine connection even when no views exist. Closing views
or dropping a host leaves the engine running for reuse. Only an explicit
`Runtime::shutdown()` stops it. See
[closing views and shutting down](surfaces.md#closing-views-and-shutting-down).

Servo owns its own internals: IPC, networking, storage, and engine
coordination. ExplorerKit does not reach into them.

## Runtime, sessions, and web views

```rust
let mut runtime = Runtime::new(host);
let session = runtime.create_session();
let view = runtime.create_webview(session)?;
```

- The `Runtime` manages which views exist and routes each handle to its view.
- A `SessionHandle` groups views logically. It does **not** separate storage:
  all views in a process share one profile.
- Each Servo view has its own `WebView`, delegate, controller state, pending
  prompts, event queue, and render target.

Your app decides which view is selected, where each one appears, and how they
are presented. The runtime never models tabs or windows.

## Profiles and persistent storage

Native Rust hosts can choose where Servo keeps cookies, logins, HSTS data, and
web storage:

```rust
let options = SurfaceHostOptions::new(waker, clipboard)
    .with_config_directory("/path/to/profile");
```

Set it before Servo first starts. ExplorerKit passes it straight to Servo's
`Opts::config_dir`, so Servo owns the files and their format. The directory is
process-wide. Once the engine is running, it only accepts the same directory.
To switch profiles, start a new process.

## Threads

Call ExplorerKit from the thread that owns the engine, usually your UI thread.
The private desktop C boundary checks this and rejects calls from other
threads. On Android, the host runs updates on the UI thread from
`Choreographer` frames. The one exception is `GpuFrameCompletion`, which you
may send to another thread.

## Web view lifetime on Servo-backed hosts

A web view's identity (its controller) and its surface have separate
lifetimes:

1. A controller can exist before its surface is ready.
2. Navigation and commands sent before a surface exists wait in Rust until a
   surface is attached.
3. Detaching a surface removes the render target, but does not invalidate the
   controller.
4. Disposing the host invalidates the controller. Later commands fail with an
   explicit error instead of acting on a recycled object.

This is why Android and React Native views can be recycled or resized without
losing the page.

## Popups and new windows

When a page calls `window.open` or follows a `target="_blank"` link, Servo
asks the embedder through `WebViewDelegate::request_create_new`. A
`PopupRequestPolicy` decides what happens:

- `DefaultDeny` (the default): nothing opens. The request is still reported as
  an event with the URL of the page that asked. Servo 0.6 doesn't expose the
  popup's target URL, so the event's `targetUrl` and `windowFeatures` are
  always null for now.
- `ManagedChild`: ExplorerKit creates a child web view under the view that asked
  (the root). Your app attaches a surface to the child with
  `attach_managed_child_surface`, and the child's events carry its ID. The
  child must keep a live handle while it exists.

Set the policy per view with `Runtime::set_popup_policy`. Managed children
belong to one root. They are not a way to create more top-level views (use
`create_webview` for that), and they do not add a multi-view React Native
API.

The React Native Android adapter always denies popups and sends
`onCreateNewWebViewRequested` as information only. iOS does not send that
event. Navigation allow/deny for normal links is a different request
(`request_navigation`), and `load_web_resource` is HTTP(S) resource
interception, not popup routing.

## Native macOS: Servo or system WebKit per view

On macOS, a native Rust host can use `MacOsViewHost` and pick an engine for
each view when it creates it: `Servo`, or `SystemWebView` (a `WKWebView` owned
by [WRY](https://github.com/tauri-apps/wry)). The runtime still owns every
view's lifetime, and your app still owns the window, layout, and selection.
Servo and WebKit do not share cookies, storage, or credentials. Details:
[macOS system web view](../reference/macos-system-webview.md).

## Related

- [Surfaces](surfaces.md): drawing, attaching, multiple views, and shutdown.
- [Commands, events, and page prompts](controller.md): how apps control views.
