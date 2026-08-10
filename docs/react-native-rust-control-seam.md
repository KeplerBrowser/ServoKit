# React Native Rust control seam

## Status

This document records how the mounted React Native `ServoView` command reaches
Rust. Android and experimental macOS target Servo-backed controllers. iOS
targets the packaged Servo-free portable controller, whose effects and
observations are mapped to WKWebView by Objective-C++.

It is intentionally split by stability:

| Layer | Status | Notes |
| --- | --- | --- |
| React Native `ServoView` props, callbacks, and ref methods | Public package API | Experimental package surface, but this is the app-facing API |
| Rust `ControllerCommand` and portable-controller semantics in `servokit-embedder` | Internal durable foundation | Rust owns command names, validation, request IDs, pending semantics, fallback policy, response validation, and invalid-controller behavior |
| JSON controller command envelope over C/JNI/package-private desktop C | Internal transport contract | Stable enough for repo adapters to target, but not yet a semver-stable public SDK/ABI |
| Portable controller C boundary and packaged XCFramework | Internal package contract | Four-function Servo-free boundary used by the iOS adapter; not a public stable C SDK |
| Mounted `evaluateJavaScript` async result event | Internal transport contract | Available on packaged Android/iOS; macOS transport exists only as part of the experimental, unsupported runtime path |
| Popup/new-webview or richer async replies | Out of scope | No managed child-view lifecycle is implied by this transport |

For the higher-level controller overview, see
[`controller-seam.md`](./controller-seam.md). For the broader runtime/module
split, see [`runtime-and-module-map.md`](./runtime-and-module-map.md).

## Decision

ServoKit's React Native adapters keep controller semantics in Rust: React Native
provides a Fabric view and idiomatic JavaScript ergonomics, while platform
adapters own native views and engine handles. This does not move WKWebView or
WebKit delegate objects into Rust.

The mounted baseline flow is:

```text
RN JS ref/callback
  -> Fabric view command on the mounted ServoView
      -> Android Kotlin/JNI or macOS Objective-C++/private C transport
          -> Rust controller command envelope by opaque controller token
              -> HostHandle / command queue / pending controls
                  -> ServoKit / Servo

RN JS ref/callback
  -> Fabric view command on the mounted ServoView
      -> iOS Objective-C++ / portable controller C boundary
          -> Rust validation / pending state / effects
              -> Objective-C++ effect execution and WebKit observations
                  -> WKWebView / WebKit
```

## Ownership

Across the Servo-backed and portable controller paths, Rust owns:

- controller identity and invalid-controller behavior
- command names and payload validation
- pending request IDs and target controller ownership
- expiry/timeout and fallback policy ownership
- invalid-response handling and response validation
- status mapping for invalid input, stale controller, busy, and internal-error
  cases
- mounted JavaScript evaluation request identity, completion validation, and
  result event semantics

Servo-backed controllers additionally own the live Servo host and call Servo's
JavaScript evaluation API. The portable controller never includes Servo.

TypeScript and the native adapters own transport and platform
ergonomics:

- React Native props, refs, callbacks, and app-facing async UX
- Fabric/native command dispatch plumbing
- native view lifecycle, engine handles, input, geometry, scheduling, and
  platform presentation
- native string/JSON marshalling into Rust

They do **not** own browser/control command names, request identity, pending
semantics, fallback policy, or response validation. On iOS, Objective-C++ does
own WKWebView, delegate completion and timer objects, KVO/recycling, effect
execution, and main-thread scheduling.

## Separation from host lifecycle/input APIs

The controller seam is intentionally narrower than the full host API.

These stay **outside** the controller command envelope:

- surface attach/detach and resize
- native-window ownership
- touch forwarding
- IME and keyboard dispatch
- update/frame pumping

Those are host/view lifecycle concerns, not browser/controller semantics.
Mounted `ServoView` commands transport controller envelopes for the current
baseline, but they must not absorb these unrelated APIs.

## Rust envelope

The current command envelope and its schema parser live in
`crates/servokit-embedder/src/controller_command.rs` as `ControllerCommand`.
The JNI, package-private desktop C, and portable-controller C transports pass
opaque JSON into Rust and validate only their own string or bounded-byte
transport concerns. The portable controller additionally accepts lifecycle and
native-observation envelopes and returns effects/events.

That transport-only rule does not remove existing behavior from the mounted
Android view adapter. It still owns mounted-controller readiness and JavaScript
evaluation completion handling while forwarding the command envelope without
owning its command names or payload schema.

### Envelope shape

The controller-command shape is a single JSON object with:

- required `version: 1`
- required Rust-owned `command` string
- command-specific payload fields

Examples:

```json
{"version":1,"command":"loadUrl","url":"https://servo.org"}
{"version":1,"command":"reload"}
{"version":1,"command":"goBack"}
{"version":1,"command":"evaluateJavaScript","evaluationId":"evaluation-1","script":"document.title"}
{"version":1,"command":"resolveNavigationRequest","navigationId":"navigation-1","allow":false}
{"version":1,"command":"resolveSimpleDialog","dialogId":"dialog-1","confirmed":true,"promptValue":"Servo"}
{"version":1,"command":"resolveContextMenu","contextMenuId":"context-menu-1","action":"copy-link"}
{"version":1,"command":"dismissContextMenu","contextMenuId":"context-menu-1"}
```

### Baseline commands

| Command | Required fields | Rust behavior |
| --- | --- | --- |
| `loadUrl` | `url: string` | Normalizes/validates through `NavigationRequest::new(...)`; Servo-backed hosts queue `WebViewCommand::LoadUrl`, while the portable controller emits a `loadUrl` effect |
| `reload` | none | Queues the Servo command or emits the portable effect |
| `goBack` | none | Queues the Servo command or emits the portable effect |
| `goForward` | none | Queues the Servo command or emits the portable effect |
| `focus` | none | Queues the Servo command or emits the portable effect |
| `blur` | none | Queues the Servo command or emits the portable effect |
| `evaluateJavaScript` | `evaluationId: string`, `script: string` | Servo-backed hosts queue Servo evaluation; the portable controller records pending identity and emits an evaluation effect. Both return a correlated async result event. |

### Current control responses

| Command | Required fields | Rust behavior |
| --- | --- | --- |
| `resolveNavigationRequest` | `navigationId: string`, `allow: boolean` | Resolves Rust-owned pending navigation state and queues or emits the engine-specific resolution |
| `resolveSimpleDialog` | `dialogId: string`, `confirmed: boolean`, `promptValue?: string \| null` | Resolves Rust-owned pending dialog state and queues or emits the engine-specific resolution |
| `resolveContextMenu` | `contextMenuId: string`, `action: string` | Validates `ContextMenuAction` in Rust and queues `ResolveContextMenu` on Servo-backed hosts; unsupported by the portable controller |
| `dismissContextMenu` | `contextMenuId: string` | Queues `DismissContextMenu` on Servo-backed hosts; unsupported by the portable controller |

The Android JNI token path, package-private desktop C ABI, and portable iOS C
boundary carry the applicable response envelopes through their generic
controller transports. Rust retains request-ID, action, and command validation.

Validation rules:

- `version` must be the integer `1`
- `command` must be recognized by Rust
- request IDs such as `navigationId`, `dialogId`, and `contextMenuId` must be
  non-empty strings
- `loadUrl.url` must parse as a valid navigation request
- context-menu `action` must be a known Rust `ContextMenuAction`

## Status mappings

The Android token transport reports Rust-owned status codes through
`ServoStatus`:

| Android condition | `ServoStatus` |
| --- | --- |
| command queued successfully | `Ok` |
| bad/malformed controller token | `InvalidControllerHandle` |
| token for a disposed host | `ExpiredControllerHandle` |
| invalid `loadUrl` payload | `InvalidUrl` |
| malformed JSON, unsupported command, unsupported context-menu action, or other envelope validation failure | `BackendError` |

The package-private desktop C ABI reports its own
`ServokitDesktopPrivateStatus` values:

| Desktop condition | Private status |
| --- | --- |
| command or response accepted | `SERVOKIT_DESKTOP_PRIVATE_OK` |
| null host pointer | `SERVOKIT_DESKTOP_PRIVATE_NULL_POINTER` |
| unknown, disposed, or mismatched host token | `SERVOKIT_DESKTOP_PRIVATE_STALE_TOKEN` |
| null, empty, oversized, non-UTF-8, malformed, unsupported, or otherwise invalid command envelope | `SERVOKIT_DESKTOP_PRIVATE_INVALID_ARGUMENT` |
| call made off the owning UI thread | `SERVOKIT_DESKTOP_PRIVATE_WRONG_THREAD` |
| reentrant call while the host is busy | `SERVOKIT_DESKTOP_PRIVATE_BUSY` |
| accepted command fails in the runtime or host | `SERVOKIT_DESKTOP_PRIVATE_RUNTIME_ERROR` |
| panic contained at the private C boundary | `SERVOKIT_DESKTOP_PRIVATE_PANIC` |

Both transports keep controller and command validation in Rust, but their
status vocabularies are not interchangeable.

## Generic transports

The current Servo-backed controller transports are:

- typed Rust entrypoint:
  `servo_host_send_controller_command_with_token(token, ControllerCommand)`
- generic JSON Rust entrypoint:
  `servo_host_send_controller_command_json_with_token(token, command_json)`
- generic C ABI:
  `servo_host_send_controller_command_with_token_ffi(const char* token, const char* command_json)`
- generic JNI entrypoint:
  `nativeSendControllerCommandWithController(String controllerHandle, String commandJson)`
- package-private desktop C ABI:
  `servokit_desktop_private_dispatch_controller_command(host, token, command_bytes, command_length)`

The packaged iOS portable-controller ABI is deliberately smaller:

- `servokit_controller_create()`
- `servokit_controller_dispatch(handle, bytes, length)`
- `servokit_controller_destroy(handle)`
- `servokit_controller_result_free(result)`

Its exact-length result bytes carry effects/events back to Objective-C++; the
XCFramework contains this controller only, not Servo.

Existing token-scoped Rust helpers such as `servo_host_load_url_with_token(...)`
and the current context-menu/navigation/dialog token FFI shims now route through
the same `ControllerCommand` path where reasonable.

## Current React Native mapping

| React Native concern | Android Servo path | iOS WKWebView path |
| --- | --- | --- |
| `ServoViewHandle.loadUrl/reload/goBack/goForward/focus/blur` | Fabric command reaches the Servo-backed controller through Kotlin/JNI. | Fabric command reaches the portable controller; Objective-C++ executes its effect against WKWebView/UIKit. |
| `ServoViewHandle.evaluateJavaScript(script): Promise<string>` | Rust calls Servo `WebView::evaluate_javascript` and returns the correlated result event. | Rust allocates/validates pending identity; Objective-C++ evaluates in WKWebView and returns the observation for Rust to settle. |
| `onShouldStartLoadWithRequest` response | Rust resolves the pending Servo navigation request. | Rust resolves the pending portable request; Objective-C++ invokes the matching WebKit completion. |
| `onJavaScriptDialog` response | Rust resolves the pending Servo dialog request. | Rust resolves the pending portable request; Objective-C++ invokes the matching WebKit completion. |
| Context-menu select/dismiss | Uses the Servo-backed controller and generic JNI path. | Not part of the current iOS baseline. |
| Surface/view lifecycle and native engine objects | Separate Android host APIs. | Objective-C++ ownership; lifecycle changes are observed by the portable controller but native objects never cross into Rust. |

The macOS adapter uses the same mounted Fabric command and forwards its bounded
bytes through `servokit_desktop_private_dispatch_controller_command`. Its AppKit
view, native handle, geometry, input, and main-queue pumping remain outside the
controller envelope. The macOS runtime path is experimental and is not a
supported or distributed contract. Consumer-facing iOS behavior belongs in
[`react-native-servokit.md`](./react-native-servokit.md).

## Fallback policy

Fallback policy ownership stays in Rust controller logic, even when app UX is
in React Native or native platform UI:

- Rust owns the existence and identity of pending navigation/dialog/context-menu
  requests.
- React Native may decide the UI and chosen answer only.
- Android may provide host-native fallback UI when no RN handler is installed.
- On iOS, Objective-C++ owns native completion/timer objects and executes the
  fallback effect selected by the portable controller.
- The returned response still resolves against Rust-owned pending state.
- Timeout policy, invalid-response handling, and fallback behavior remain
  coordinator concerns rather than React Native state.

That means adapter-local state may guard duplicate callback completion, but it is
not the source of truth for pending request lifetime.

## Mounted Fabric contract

The mounted baseline:

1. sends `ServoView` imperative commands through mounted Fabric view commands;
2. has each native view transport call its Rust controller boundary;
3. keeps the React Native control surface on the mounted Fabric path without a
   separate TurboModule.

It still does **not** move controller command semantics into TypeScript,
Kotlin, or C++.

For mounted JavaScript evaluation on Servo-backed hosts, Rust serializes Servo
`JSValue` results into JSON strings for React Native, maps Servo
`JavaScriptEvaluationError` variants into the mounted adapter's rejection
categories, and uses `evaluationId` only as transport correlation back to the
React Native promise.
