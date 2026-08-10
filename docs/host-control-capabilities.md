# Host control capabilities

This matrix is the durable reference for host-control capability claims.
Android rows describe the Servo-backed path through the Rust ServoKit runtime
and shared Android host. iOS rows describe the WKWebView/WebKit-backed React
Native baseline only. Servo-on-iOS is deferred, so this matrix scopes the
WKWebView capability claims rather than collapsing a future Servo path into
WebKit.

Use public capability names here. The capability is `dialog`; internal command
names are transport details and should appear only when a doc is quoting the
actual command envelope.

Fixture pages are smoke evidence. If an iOS row says a fixture only loads, that
load is not a claim that ServoKit owns native default behavior, app/RN
customization, or complete support for the capability.

React Native callbacks provide presentation and answers only. They do not own
request IDs, pending lifetime, timeout/fallback policy, invalid-response
handling, response validation, or command names. On iOS, those semantics belong
to the Servo-free portable Rust controller; Objective-C++ retains native WebKit
completion and timer objects and executes controller effects.

Fallback posture is safe native default first, bounded non-wedging fallback
second, and deny by default for risk-sensitive capabilities.

Popup/new-window intent maps to Servo
`WebViewDelegate::request_create_new`, not `load_web_resource`.
`request_navigation` remains normal navigation allow/deny for main-frame or
iframe loads; `load_web_resource` is HTTP/HTTPS resource interception or
alternate response loading.

## Android Servo-backed path

| Capability | Fixture page loads | Platform-native default behavior | ServoKit-owned default/fallback | App/RN customization hook | Unsupported | Deferred/not claimed |
| --- | --- | --- | --- | --- | --- | --- |
| `navigationPolicy` | `smoke/policy.html` exercises allow/deny smoke. | No Android OS policy UI; normal navigation is allowed when no app hook is installed. | Rust records `navigationId`, validates allow/deny responses, and falls back to allow after a bounded timeout so navigation does not wedge. | `onShouldStartLoadWithRequest`. | - | Rich request interception and auth/resource policy are separate future surfaces. |
| `dialog` | `controls/dialogs.html` exercises alert, confirm, and prompt. | Android host-native dialogs are the no-RN-handler default. | Rust owns pending dialog identity and response validation; invalid or missing app responses resolve through bounded fallback behavior. | `onJavaScriptDialog` and `onJavaScriptDialogDismissed`. | - | Custom product dialog systems are app UI, not a new ServoKit control owner. |
| `contextMenu` | `controls/context-menu-demo.html` exercises menu targets. | Android menu presentation is provided by the host adapter. | Rust owns pending menu identity, default Servo actions, action validation, selection, and dismissal. | `onBeforeShowContextMenu` and `onContextMenuItemSelected`. | - | Cross-platform parity and richer menu policy remain separate decisions. |
| Select picker | `controls/select-elements.html`. | Android host selection UI. | Rust owns pending select IDs and selected-option validation; closing the webview drops pending state through Servo's default response. | None documented. | - | RN-owned select picker UI is not claimed. |
| File picker | `controls/file-input.html`. | Android document picker flow. | Rust owns pending file-picker IDs; selected paths or dismissal resolve the Servo request, and unavailable/cancelled UI dismisses safely. | None documented. | - | RN-owned file picker replacement is not claimed. |
| Date/time/color pickers | `controls/pickers.html` for color, date, time, datetime-local, month, and week. | Android host picker dialogs for supported non-text input methods. | ServoKit maps input-method requests to picker UI, commits values, and dismisses unsupported/cancelled picker requests without wedging. | None documented. | Unsupported picker input types degrade without crashing. | RN-owned picker UI is not claimed. |
| Permissions | `controls/permissions.html`. | Android permission UI is host/app presentation. | Rust tracks the pending permission request; replaced or dropped requests deny by default, and explicit responses validate allow/deny. | None documented as a public RN hook. | - | Public RN permission customization is not claimed. |
| Fullscreen | No dedicated fixture. | Android system-UI/chrome policy stays app-owned. | Servo-backed fullscreen state is surfaced as an event; ServoKit does not take over app chrome. | `onFullscreenChanged`. | - | Final fullscreen product policy is not claimed. |
| Focus | `smoke/form.html` and `controls/ime-form.html`. | Android `View` focus and keyboard behavior. | `focus`/`blur` commands and focus events stay on the controller/event seam. | `ServoViewHandle.focus`, `ServoViewHandle.blur`, and `onFocusChanged`. | - | - |
| Cursor | No dedicated fixture. | No Android OS cursor policy is claimed for touch-only hosts. | Servo cursor changes are mapped into the shared event stream. | `onCursorChanged`. | - | Rich pointer/caret-position APIs are not claimed. |
| IME | `controls/ime-form.html`. | Android `InputMethodManager` and soft keyboard. | Input-method request/dismissal, composition dispatch, and viewport resizing are Android host glue around ServoKit events. | Focus/blur APIs only; no custom RN IME hook. | - | Custom keyboard/IME products are not claimed. |
| Clipboard/edit actions | `controls/context-menu-demo.html` and editable fields in `controls/ime-form.html`. | Android `ClipboardManager` and edit menu behavior. | Servo edit/context actions resolve through the Android host path instead of Servo's fallback clipboard store. | `onContextMenuItemSelected` can observe selected menu items; app-injected menu items stay app-owned. | - | A standalone clipboard API surface is not claimed. |
| Crash/error reporting | `smoke/error.html` exercises load-error smoke; no crash fixture is claimed. | Platform logging may exist, but it is not the public contract. | Android emits Servo-backed load, error, and crash events through the shared event bridge. | `onLoadStatusChanged`, `onError`, and `onCrashed`. | - | Deterministic crash fixtures and recovery policy are not claimed. |
| Popup/new-window intent | Future fixture candidate only. | React Native Android creates no native popup window. | The adapter uses default-deny for `request_create_new`; lower-level Rust `PopupRequestPolicy::ManagedChild` is not adopted by this path. | `onCreateNewWebViewRequested` is an informational event for app-owned routing; it surfaces `parentWebViewId`, nullable `parentUrl`, nullable `targetUrl`, nullable `windowFeatures`, and `policy`. | Managed popup/window presentation is not provided. | The Rust embedder's root-scoped managed children do not imply multi-root or public RN multi-view support. |

## iOS WKWebView/WebKit-backed path

| Capability | Fixture page loads | Platform-native default behavior | ServoKit-owned default/fallback | App/RN customization hook | Unsupported | Deferred/not claimed |
| --- | --- | --- | --- | --- | --- | --- |
| `navigationPolicy` | `smoke/policy.html` can exercise the shared policy smoke. | `WKNavigationDelegate` supplies the native decision point. | The portable Rust controller owns request identity, response validation, and bounded allow fallback; Objective-C++ retains and invokes the matching native completion. | `onShouldStartLoadWithRequest({ url })`. | WebKit-specific request fields are not exposed. | Rich interception/auth/resource policy is not claimed. |
| `dialog` | `controls/dialogs.html` can exercise alert, confirm, and prompt. | `WKUIDelegate` supplies alert/confirm/prompt callbacks. | The portable Rust controller owns pending dialog identity and fallback semantics; Objective-C++ retains the native completion and applies WebKit-safe defaults: alerts complete, confirm and prompt cancel. | `onJavaScriptDialog`. | - | Custom product dialog systems are app UI, not a new ServoKit control owner. |
| `contextMenu` | `controls/context-menu-demo.html` may load only. | Not claimed as ServoKit support. | None. | None documented. | Context-menu support is not in the iOS baseline. | Native/default/customizable context-menu support is deferred. |
| Select picker | `controls/select-elements.html` may load only. | Not claimed as ServoKit support. | None. | None documented. | Shared select-picker support is not in the iOS baseline. | Native/default/customizable select support is deferred. |
| File picker | `controls/file-input.html` may load only. | Not claimed as ServoKit support. | None. | None documented. | Shared file-picker support is not in the iOS baseline. | Native/default/customizable file-picker support is deferred. |
| Date/time/color pickers | `controls/pickers.html` may load only. | Not claimed as ServoKit support. | None. | None documented. | Shared picker support is not in the iOS baseline. | Native/default/customizable date/time/color support is deferred. |
| Permissions | `controls/permissions.html` may load only. | Not claimed as shared ServoKit permission support. | None. | None documented. | Shared permission prompts are not in the iOS baseline. | Permission policy and RN customization are deferred. |
| Fullscreen | No dedicated fixture. | Not claimed as ServoKit support. | None. | None documented. | Fullscreen event/policy parity is not in the iOS baseline. | Deferred. |
| Focus | `smoke/form.html` can exercise focusable content. | WKWebView/UIKit own native focus behavior. | The iOS adapter maps `focus`, `blur`, and focus events where WebKit/UIKit expose equivalent behavior. | `ServoViewHandle.focus`, `ServoViewHandle.blur`, and `onFocusChanged`. | - | Rich focus/caret-position APIs are not claimed. |
| Cursor | No dedicated fixture. | Not claimed as ServoKit support. | None. | None documented. | Cursor event parity is not in the iOS baseline. | Deferred. |
| IME | `controls/ime-form.html` may load and text input may use WKWebView/UIKit behavior. | WKWebView/UIKit own keyboard and text editing behavior. | No shared ServoKit IME bridge or pending input-method request is claimed on iOS. | Focus/blur APIs only. | Shared IME control is not in the iOS baseline. | Custom keyboard/IME products are not claimed. |
| Clipboard/edit actions | Editable fixture fields may load and use WebKit/UIKit behavior. | WKWebView/UIKit may provide native edit behavior. | None. | None documented. | Shared clipboard/edit action control is not in the iOS baseline. | A standalone clipboard API surface is not claimed. |
| Crash/error reporting | `smoke/error.html` may exercise load-status behavior; no crash fixture is claimed. | WKWebView supplies native loading state. | The iOS baseline exposes URL, title, load status, history, and focus events; crash/error parity is not claimed. | `onLoadStatusChanged` only for the documented baseline. | `onCrashed`/`onError` parity is not in the iOS baseline. | Crash/error reporting parity is deferred. |
| Popup/new-window intent | Future fixture candidate only. | Not claimed as ServoKit support. | None. | No `onCreateNewWebViewRequested` event is emitted. | Popup/new-window support is not in the iOS baseline. | Multi-tab/window/view lifecycle, `WindowProxy`/opener communication, and public managed child-surface adoption are not claimed. |
