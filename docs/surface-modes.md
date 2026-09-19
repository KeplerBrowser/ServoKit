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
| `SurfaceViewport` | Physical-pixel origin and size plus display scale factor. |
| `SurfaceTarget` | A constructible `NativeChild` or `CpuOffscreen` render target. |
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
`SurfaceFrame` additionally exposes frame metadata and CPU RGBA readback.

## NativeChildSurface

`NativeChildSurface` borrows `raw-window-handle` display and window handles from
the host. The host keeps those handles valid while attached and remains
responsible for the top-level window, child view placement, input routing, and
event/frame loop. ServoKit renders and presents into that native target.

This is the current path for whole-window `winit`, AppKit child-view embedding,
and Android native surfaces.

## CpuOffscreenSurface

`CpuOffscreenSurface` describes an offscreen Servo target associated with a
host-owned parent native surface and parent size. ServoKit paints into Servo's
offscreen rendering context. The host can read the resulting frame through
`SurfaceFrame::read_rgba` and composite those pixels into its layout.

CPU readback is suitable for proofs, screenshots, and compatibility paths. It
is not a zero-copy GPU handoff.

## Attach And Detach

- A logical webview must exist before a surface is attached.
- A webview can have at most one attached surface.
- Attach creates the Servo-backed surface webview on first use; reattach reuses
  that browser/controller identity.
- Viewport updates carry origin, physical size, and scale together before the
  next update/present cycle.
- Detach removes the render target while preserving browser/controller state
  for later reattachment.
- `Runtime::destroy_webview` detaches and closes one view, invalidates its handle,
  and discards its queued events. Sibling views and the runtime remain usable.
- The host retains its shared engine connection even with zero views. Ordinary
  host drop releases that connection after detaching/dropping all views, allowing
  a later host to reuse the process engine.
- `Runtime::shutdown` consumes the runtime, closes its views, and permanently
  shuts down the engine. Call it on the owning UI thread before native parent
  windows or logging are destroyed. Servo 0.3 does not support engine restart.
  Final cleanup still runs if an individual detach fails, and returns the first
  error. An unattached final host can also retire an engine retained after an
  earlier host was dropped; it cannot shut down another live host's engine.

## Multiple Native Views

Create multiple `WebViewHandle`s in the same runtime and attach a distinct
`HostSurface`/native child to each. A `SurfaceDelegate` selects its target by
`HostSurface` identity. Views share the engine, not their delegate state or
rendering target; creation does not require popup policy or an opener view.

Use `Runtime::perform_all_updates` from the host event loop. It runs per-view
`before_update` hooks, spins Servo once, then presents pending frames and drains
all view events. `RuntimeError::WebView` identifies a failed view while sibling
events remain available through `drain_events`; `RuntimeError::Host` denotes an
owner-wide failure. The existing `perform_updates(view)` remains available for
single-view callers. Multi-view hosts must service all views because a Servo
spin can invoke any view's delegate.

Native handles remain owned by the host. Destroy or detach the renderer before
removing a child, and dispose of all renderers before destroying their parent
window. The AppKit helper's visibility/removal API remains separate work; this
contract does not add native clipping, stacking, or GPU export.

See [Architecture](../ARCHITECTURE.md#current-servo-runtime-limit) for process
ownership and adapter limits, and [readiness checks](readiness-checks.md) for the
runnable native proof.

## Future GpuLayerSurface

`SurfaceMode::GpuLayer` reserves vocabulary for a future host-consumable GPU
surface, but no `GpuLayerSurface` target or exported-handle API is implemented.
Any such mode depends first on upstream Servo exporting a supported external
surface contract, including platform handle, synchronization, resize, and
retirement semantics. ServoKit should not invent a parallel compositor API in
advance of that support.
