# ServoKit Surface Modes

ServoKit separates browser/controller identity from the render surface supplied
by an application. The application owns its window, layout, event loop, and
surface resources. ServoKit owns Servo integration and attaches it to the
host-provided target.

## Current Contract

The public surface vocabulary is host-neutral:

| Type | Current role |
| --- | --- |
| `HostSurface` | Stable host-provided identity for a layout slot or native surface. |
| `SurfaceViewport` | Host placement/size plus display scale. Exportable offscreen targets interpret size as logical and allocate `size × scale` physically. |
| `SurfaceTarget` | A constructible `Native` or `Offscreen` render target. |
| `SurfaceFrameInfo` | Viewport and mode metadata for a painted frame. |
| `SurfaceDelegate` | Host callbacks that select/update the render target and present or composite frames. |

`SurfaceDelegate` uses an associated `Frame<'a>: SurfaceFrameLike`. Its current
callbacks are:

- `render_target` for initial attachment;
- `update_render_target` when the attached target or viewport changes;
- `before_update` before Servo update work; and
- `present_frame` when Servo produces a frame.

The default `update_render_target` calls `render_target`. The default
`present_frame` calls `SurfaceFrameLike::present`. The `servokit` facade's
`SurfaceFrame` additionally exposes frame metadata and optional CPU RGBA readback.

## NativeSurface

`NativeSurface` borrows `raw-window-handle` display and window handles from
the host. The host keeps those handles valid while attached and remains
responsible for the top-level window, child view placement, input routing, and
event/frame loop. ServoKit renders and presents into that native target.

This is the current path for whole-window `winit`, AppKit child-view embedding,
and Android native surfaces.

## OffscreenSurface

`OffscreenSurface::new(parent, parent_size)` preserves the parent-backed local
framebuffer path. The host can read the resulting frame through
`SurfaceFrame::read_rgba` and composite those pixels into its layout. CPU
readback is a frame operation, not a surface mode.

CPU readback is suitable for proofs, screenshots, and compatibility paths. It
is not a zero-copy GPU handoff.

On macOS, `OffscreenSurface::exportable()` creates a per-webview surfman
`GPUOnly` surface pool. `SurfaceFrame::take_gpu_frame()` publishes an opaque,
single-plane 32BGRA `CVPixelBuffer` without ordinary CPU readback. The exported
`GpuFrameInfo` identifies the webview, physical generation, pool slot, frame
serial, and exact physical extent. The GL image is vertically oriented as in
the accepted #8 contract, so a Metal consumer flips it when sampling.

The pool has three slots. If every slot is held by a consumer, ServoKit keeps
the paint request pending and does not overwrite a slot. A `GpuFrameCompletion`
is one-shot and `Send`; complete it only after the consumer's final GPU sample.
Dropping it incomplete never releases the slot. This bounded misuse path retains
the affected IOSurface until process exit rather than risking use-after-free.

## Attach And Detach

- A logical webview must exist before a surface is attached.
- A webview can have at most one attached surface.
- Attach creates the Servo-backed surface webview on first use; reattach reuses
  that browser/controller identity.
- Viewport updates carry origin, size, and scale together before the next
  update/present cycle. Exportable offscreen size is logical; native and local
  offscreen targets keep their existing physical-size convention until #16.
- Detach removes the render target while preserving browser/controller state
  for later reattachment.
- `MacOsViewHost` system views retain a ServoKit-owned container while detached,
  not the old app parent. Reattach moves that container and its existing WRY
  browser view to the supplied live AppKit parent. Focus and blur requested
  while detached replay after reattachment. Ordinary host drop removes the
  system children before dropping the parent-view delegate.

## Multiple Native Views

In the native Rust `Runtime<SurfaceHost<_>>`, create multiple `WebViewHandle`s
and attach a distinct `HostSurface`/native child to each. A `SurfaceDelegate`
selects its target by `HostSurface` identity. Each view has its own delegate state
and rendering target under the shared engine; creation does not require popup
policy or an opener view.

Use `Runtime::perform_all_updates` from the host event loop. It runs per-view
`before_update` hooks, spins Servo once, then presents pending frames and drains
all view events. `RuntimeError::WebView` identifies a failed view while sibling
events remain available through `drain_events`; `RuntimeError::Host` denotes an
owner-wide failure. The existing `perform_updates(view)` remains available for
single-view callers. Multi-view hosts must service all views because a Servo
spin can invoke any view's delegate.

## View destruction and final shutdown

The native Rust surface host distinguishes closing a view from retiring the engine.

The embedding host decides the engine's lifetime. ServoKit does not observe window
closure or automatically shut down the engine when a window, surface, or final view
is removed. A host can keep the runtime alive with no windows or views and attach
new views later. Call terminal shutdown only when the host is finished using Servo
for the remainder of the process.

- `Runtime::destroy_webview` detaches and closes one view, invalidates its handle,
  and discards its queued events. Siblings remain usable. A detach error leaves
  the view registered so destruction can be retried. Exportable offscreen views
  return this retriable error while consumer-held frames await completion.
- The host retains its shared engine connection even with zero views. Ordinary
  host drop releases that connection after detaching/dropping all views, allowing
  a later host to reuse the process engine.
- `Runtime::shutdown` consumes the runtime, closes its views, and permanently
  shuts down the engine. Call it on the owning UI thread before native parent
  windows or logging are destroyed. Servo 0.3 does not support engine restart.
  Final cleanup still runs if an individual detach fails, and returns the first
  error. A lost GPU completion quarantines only its retained bounded IOSurface;
  macOS reclaims it when the app process exits. An unattached final host can also
  retire an engine retained after an earlier host was dropped; it cannot shut
  down another live host's engine.

Native handles remain owned by the host. Destroy or detach a view's renderer
before removing its native child or destroying its parent window.
The AppKit helper's visibility/removal API remains separate work; exportable
offscreen rendering does not add native clipping or stacking.

See [Architecture](../ARCHITECTURE.md#servo-runtime-ownership) for process
ownership and adapter limits, and [readiness checks](readiness-checks.md) for the
runnable native proof.

## Platform scope

The exported resource payload is intentionally under `servokit::surface::macos`.
Windows shared-texture and Linux dma-buf payloads remain separate platform work;
they can reuse the same `Offscreen` target and completion semantics without
pretending that `CVPixelBuffer` is cross-platform.
