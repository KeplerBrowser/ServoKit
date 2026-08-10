# Feature coverage

This document tracks two related but different things:

1. **Servo embedder controls** — capabilities Servo exposes to the embedder, where the host app is expected to provide UI, policy, or platform integration.
2. **React Native package readiness** — the practical feature set app developers expect when using Servo through this React Native adapter.

Many of the items below are Servo **embedder-owned** features. Servo exposes the request or callback, but this port still has to surface it through Android and React Native.

## Control contract

ServoKit's control posture is platform-native default plus app-customizable
presentation where the platform supports it. Public docs should describe
capabilities, not transport names: `dialog`, `navigationPolicy`, `contextMenu`,
picker/file flows, permissions, fullscreen, cursor, focus, and crash/error
reporting.

On Servo-backed Android and macOS, ServoKit owns request creation, request IDs,
target ownership, expiry/timeout policy, fallback behavior, invalid-response
handling, response validation, and command names. The Servo-free portable Rust
controller owns the same shared semantics on iOS; Objective-C++ owns WKWebView,
native delegate completions/timers, KVO/recycling, effect execution, and
main-thread scheduling. React Native hooks can override presentation and
answers; they do not own pending request state.

Fallback policy is safe native default first, bounded non-wedging fallback
second, and deny-by-default for risk-sensitive capabilities.

The platform-specific host-control matrix is
[`host-control-capabilities.md`](./host-control-capabilities.md). Treat it as
the source of truth for Android Servo-backed support versus iOS
WKWebView/WebKit-backed support.

## Current position

`react-native-servokit` already has a solid Android Servo-backed baseline for:

- real rendering and navigation
- load/title/history/error events
- JavaScript dialogs
- IME and viewport resizing
- `<select>`, file, and date/color/time picker flows
- permissions
- context menus, including React Native item injection
- host-routed popup/new-window intent events
- clipboard-backed edit actions
- Android back-button and focus policy

The default iOS baseline is intentionally different: it keeps the shared React
Native `ServoView` ergonomics while using WKWebView/WebKit. Servo-on-iOS is
deferred rather than exposed as a package path.
Today the WKWebView baseline covers:

- initial render and navigation through the `url` prop and load/reload/back/forward commands
- URL, title, load-status, history, and focus events where WebKit exposes equivalent state
- `onShouldStartLoadWithRequest` navigation policy with allow-on-fallback behavior
- mounted `evaluateJavaScript` using WKWebView serialization/error categories
- JavaScript alert/confirm/prompt through the shared RN dialog callback

The React Native macOS path is an experimental implementation of the same
Fabric `ServoView` surface through an AppKit adapter and the package-private
desktop C boundary into Rust ServoKit. It is not a supported or distributed
runtime contract and does not claim full Android feature parity.

Servo-backed paths retain the process runtime on its owning UI thread and allow
one active runtime lease/live root. Dropping or destroying its owning
webview/host releases the lease so a later root can reuse the runtime; surface
detach does not. See
[Architecture](../ARCHITECTURE.md#current-servo-runtime-limit).

The mounted React Native JavaScript evaluation API covers app-initiated
`ServoView.evaluateJavaScript(script): Promise<string>` on a mounted `ServoView`:
Android routes through Servo `WebView::evaluate_javascript`; iOS routes through
`WKWebView.evaluateJavaScript`. It is not a preload/user-script system, a
web-content-to-native messaging bridge, per-webview JavaScript or security
policy, RN-WebView-style `injectJavaScript`, detached/headless controller
scripting, or popup/new-window behavior.

Important non-baseline gaps remain:

1. **secure web-content IPC and preload/user-script policy**
2. **advanced auth / interception hooks**
3. **accessibility integration**

## Servo baseline note

This coverage document describes the current crates.io `servo` `=0.3.0`
baseline. Popup/new-window intent language maps to Servo
`WebViewDelegate::request_create_new`: content asks for a new `WebView`, such
as `window.open`; ignored requests open nothing, and embedders that create a
child must retain a live handle. `WebViewDelegate::request_navigation` is
navigation allow/deny for main-frame or iframe loads.
`WebViewDelegate::load_web_resource` is HTTP/HTTPS resource interception or
alternate response loading, not popup routing. Future Servo API fallout stays
deferred until ServoKit intentionally changes its Servo baseline; see
[`rust-dependency-baseline.md`](./rust-dependency-baseline.md).

## Coverage table

| Area | Kind | Status | App impact | Notes |
| --- | --- | --- | --- | --- |
| Rendering, navigation, surface lifecycle | Core runtime | Available on Android; experimental on macOS; WKWebView-backed on iOS | High | The macOS Servo-backed path is an experimental implementation without a supported or distributed runtime contract; iOS uses WKWebView/WebKit for the baseline `ServoView` adapter |
| Load, title, history, crash, and error events | Core runtime | Available on Android; experimental on macOS; partial on iOS | High | The macOS event wiring belongs to the experimental runtime path; iOS currently exposes URL/title/load/history/focus and does not claim crash/cursor/fullscreen parity |
| Imperative commands (`loadUrl`, `reload`, `goBack`, `goForward`, `focus`, `blur`) | Public API | Available on Android/iOS; experimental on macOS | High | Android and experimental macOS target Servo-backed Rust controllers; iOS routes through the portable Rust controller before Objective-C++ executes effects against WKWebView/UIKit |
| Touch input | Core runtime | Implemented on Android; WKWebView-owned on iOS | High | Android touch events stay on the native Servo path; iOS uses WKWebView/UIKit behavior |
| Dialog (`alert`, `confirm`, `prompt`) | Embedder control | Implemented | Medium | Public capability name is `dialog`. RN-owned dialog UI is supported on Android Servo and iOS WKWebView; Android-native fallback remains, while iOS no-handler default completes alert and cancels confirm/prompt |
| IME bridge and viewport resizing | Embedder/platform control | Implemented on Android; WKWebView/UIKit-owned on iOS | High | Android has keyboard-aware viewport updates through the Servo host path; iOS does not claim the shared ServoKit IME bridge |
| `<select>` picker | Embedder control | Implemented on Android; deferred/not claimed on iOS | Medium | Android host owns the picker UI; see `host-control-capabilities.md` for platform scope |
| File picker | Embedder control | Implemented on Android; deferred/not claimed on iOS | Medium | Android document-picker flow is wired; see `host-control-capabilities.md` for platform scope |
| Color/date/time/datetime pickers | Embedder control | Implemented on Android; deferred/not claimed on iOS | Medium | Android host owns these picker dialogs; see `host-control-capabilities.md` for platform scope |
| Permissions | Delegate / embedder policy | Implemented on Android; deferred/not claimed on iOS | Medium | Android covers Servo-supported permission prompts currently surfaced here; see `host-control-capabilities.md` for platform scope |
| Context menu baseline | Embedder control | Implemented on Android; deferred/not claimed on iOS | Medium | Android long-press trigger and Servo actions are wired; see `host-control-capabilities.md` for platform scope |
| React Native context-menu injection | Port policy surface | Implemented on Android; deferred/not claimed on iOS | Medium | RN can add dynamic items before showing the Android menu; see `host-control-capabilities.md` for platform scope |
| Clipboard/edit actions (`cut`, `copy`, `paste`, `select all`) | Embedder/platform control | Implemented on Android; deferred/not claimed on iOS | Medium | Android uses `ClipboardManager` instead of Servo's Android fallback clipboard store; iOS WebKit/UIKit behavior is not a shared ServoKit clipboard/edit claim |
| Back-button and focus synchronization | Port policy surface | Implemented on Android; focus partial on iOS | Medium | Android IME dismisses first and focused webview consumes back for history when possible; iOS maps focus where WebKit/UIKit expose equivalent behavior |
| Navigation request policy (`WebViewDelegate::request_navigation`) | Delegate / port policy | Implemented | High | Current RN-facing prop is `onShouldStartLoadWithRequest`; Android maps Servo delegate decisions, iOS maps `WKNavigationDelegate`, and unresolved RN callbacks auto-allow after a bounded timeout |
| Mounted JavaScript evaluation (`ServoView.evaluateJavaScript`) | Public React Native API / delegate surface | Implemented | High | App-initiated mounted `ServoView.evaluateJavaScript(script): Promise<string>` calls Servo `WebView::evaluate_javascript` on Android and `WKWebView.evaluateJavaScript` on iOS; both resolve with tagged JSON strings |
| Popup / new-window intent | Delegate / embedder policy | Managed children in lower-level Rust; Android RN informational event; not emitted on iOS | Medium to high | Rust ServoKit maps Servo `WebViewDelegate::request_create_new` / `CreateNewWebViewRequest` and supports root-scoped `PopupRequestPolicy::ManagedChild` lifecycle. React Native Android uses default-deny and emits `onCreateNewWebViewRequested` for host-routed intent only; it does not create, present, or adopt managed children. Managed children do not allow another root or provide a general N-root pool/public RN multi-view API. iOS does not emit the RN event. |
| Preload / user scripts | Web-content scripting | Backlog | Medium | Not covered by mounted evaluation; requires per-webview script ordering plus JavaScript/security policy |
| RN-WebView-style `injectJavaScript` | Web-content scripting | Backlog | Medium | Separate from app-initiated mounted evaluation; no detached/headless or post-load injection API is promised |
| Secure web-content-to-native messaging | Web-content IPC | Backlog | Medium to high | Requires an explicit bridge capability and origin/security model before any adapter API |
| Custom schemes / virtual asset loader | Resource loading | Backlog | Medium | Useful for app assets/offline content, but not part of the current runtime baseline |
| GPU layer surfaces / zero-copy texture handoff | Surface/compositor mode | Backlog | Medium | Future `GpuLayerSurface` work is separate from the current native-child and CPU-offscreen surface modes |
| Local prebuilt native package | Packaging | Implemented locally | Medium | The exact tgz contains the dual-ABI Android AAR and Servo-free iOS controller XCFramework; consumer builds run no Cargo or native download and do not reference the workspace. npm publication, CI, and release promotion remain deferred. |
| Fullscreen notifications and policy | Delegate / embedder policy | Implemented on Android; deferred on iOS | Medium | Servo-backed `onFullscreenChanged` notifies RN on Android; Android system-ui policy stays app-owned; iOS fullscreen parity is not in the WKWebView baseline |
| Richer focus/cursor notifications | Delegate surface | Implemented on Android; partial on iOS | Low to medium | Servo-backed `onFocusChanged` and `onCursorChanged` on Android; iOS currently exposes focus only and no separate caret-position stream |
| Bluetooth device selection | Embedder control | Not yet implemented | Low | Useful for specific media/device flows, not baseline embedding |
| Auth hooks and resource interception | Advanced embedder policy | Not yet implemented | Medium | Important for enterprise/proxy/custom-network use cases |
| Screenshot utilities | Advanced utility surface | Not yet implemented | Low | Helpful app utility, but not baseline Servo embedder work |
| Media session integration | Advanced platform integration | Not yet implemented | Low | Nice to have, not baseline embedding |
| Accessibility integration | Advanced platform integration | Not yet implemented | High | Product-quality requirement rather than basic API coverage |

## Notes on terminology

### Servo embedder controls

In upstream Servo, "embedder controls" usually refers to flows like:

- file pickers
- select pickers
- context menus
- dialog requests

These are modeled explicitly in upstream code such as `components/shared/embedder/embedder_controls.rs`.

### Broader package surfaces

Not every package-relevant feature is literally an upstream `EmbedderControl` variant. Some important gaps are exposed through delegates or other embedder-owned integration points instead, such as:

- navigation interception
- JavaScript evaluation
- host-routed popup / new-window intent policy
- fullscreen state handling

So when evaluating this package, it is useful to think in terms of **embedder-owned surfaces**, not only the narrow upstream `EmbedderControl` enum.
