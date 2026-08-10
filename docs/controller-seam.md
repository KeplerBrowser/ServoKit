# Controller Seam

## Purpose

The ServoKit controller seam keeps browser-control semantics in Rust while
platform adapters own native views and engine handles. On Servo-backed hosts,
the controller handle/token refers to a Rust-owned `HostHandle`; commands keep
targeting that host when a mounted Fabric view or render surface changes. On
iOS, a separate portable controller handle owns the shared command and pending
semantics without embedding Servo, while Objective-C++ applies its effects to
WKWebView.

The durable React Native controller-command record now lives in
[`react-native-rust-control-seam.md`](./react-native-rust-control-seam.md).
Read that document for the command envelope, ownership rules, current RN
mapping, and the mounted-`ServoView` transport direction.

## Ownership

Across the Servo-backed and portable controller paths, Rust owns:

- controller identity and invalid-controller behavior
- controller command names, payload validation, and status mapping
- request identity, expiry/timeout policy, target ownership, fallback
  semantics, and response validation

Servo-backed controllers additionally own the live host's browser state,
pending browser commands, and Servo integration.

Android and macOS own platform views/handles, rendering and input primitives,
mounting, geometry, scheduling, and native presentation.

React Native owns the package API, callbacks, refs, and app-facing policy UI.
Responses return through the Rust-owned controller seam on both paths. On iOS,
Objective-C++ retains the native WebKit delegate completion and timer objects,
but the portable controller remains the source of truth for pending identity
and fallback semantics.

## Command path

Controller commands are the durable path for host control. Adapters may use
multiple transports to reach that path:

- the mounted React Native baseline now uses mounted Fabric `ServoView`
  commands plus the generic JNI transport on Android and the package-private C
  transport on macOS;
- iOS uses the packaged four-function `servokit-controller-ffi` C boundary to
  dispatch commands, lifecycle changes, and WebKit observations through its
  portable controller handle;
- Android/JNI exposes a generic controller-command transport for opaque
  controller tokens.

That means mounted Fabric view commands are acceptable as an adapter transport,
while the controller token plus command envelope remain the browser/control
seam.

The Servo-backed policy flow is:

1. Servo raises a delegate request or embedder control.
2. Rust records the pending request against the controller-owned host state.
3. The adapter surfaces the request to React Native or another shell.
4. The caller responds with the controller handle/token and request id.
5. Rust validates the controller, payload, and request before applying the
   response.

The iOS flow uses the same ownership rule with a different engine boundary:
Rust validates an input and emits effects; Objective-C++ applies those effects
to WKWebView and returns native observations to Rust; Rust validates and emits
the resulting events or resolution effects.

## Implemented baseline envelope

The Rust-owned controller command envelope lives in
`crates/servokit-embedder/src/controller_command.rs`. Android, desktop, and the
portable controller adapt the parsed command to their engine without
duplicating its JSON schema.

The current baseline covers:

- browser commands: `loadUrl`, `reload`, `goBack`, `goForward`, `focus`,
  `blur`
- control responses: navigation request allow/deny, dialog
  confirm/dismiss/prompt, context-menu select, and context-menu dismiss
- generic transports: Rust typed command sending,
  `servo_host_send_controller_command_with_token_ffi`, and the JNI entrypoint
  `nativeSendControllerCommandWithController(...)`; desktop package adapters
  use bounded opaque bytes through
  `servokit_desktop_private_dispatch_controller_command(...)` for controller
  commands and responses; iOS uses `servokit_controller_create`,
  `servokit_controller_dispatch`, `servokit_controller_destroy`, and
  `servokit_controller_result_free`

Existing token-scoped Rust functions now route through the same typed command
parsing and queueing path so command names, validation, and status mapping stay
centralized in Rust.

## Adjacent boundaries

Servo-backed host events cross one structured bridge. Keep event additions
localized to that bridge shape and its adapters rather than adding new
per-field extraction paths.

Surface lifecycle, native-window attachment, touch/input forwarding, and update
pump APIs stay separate from controller commands. A mounted `ServoView` now
transports controller commands for the baseline RN control path, but surface
attach/detach, resize, `ANativeWindow`, touch, IME, keyboard, and frame pumping
are still distinct host APIs.

## New work checklist

- Is the target Servo-backed or the portable iOS controller? Keep native engine
  state and delegate completion objects in the platform adapter either way.
- Does a Servo-backed response cross the controller seam through an adapter
  transport while keeping command semantics Rust-owned?
- Does Rust own the pending request id, expiry/timeout policy, target engine,
  fallback behavior, and response validation?
- Does invalid controller behavior fail explicitly?
- Are names aligned with Servo delegate and embedder-control terminology?
- Are surface lifecycle, native-window, touch, input, and update pumping kept
  separate from controller commands?
