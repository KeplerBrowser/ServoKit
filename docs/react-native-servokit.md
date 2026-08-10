# react-native-servokit

Experimental React Native Fabric adapter for ServoKit, the Servo embedding
toolkit. Android is Servo-backed through ServoKit's Rust runtime, the reusable
`crates/servokit-host-android/android` Gradle module, and the mounted
`ServoView` command path. The same local package contains the
WKWebView/WebKit-backed iOS adapter and its Servo-free portable Rust controller.
The experimental Servo-backed macOS adapter remains source-side and is not
packaged or supported. Servo-on-iOS is deferred while Servo lacks official iOS
platform support.

## Status

| Platform family | Status | Notes |
| --- | --- | --- |
| android | experimental local package | Servo-backed Kotlin -> JNI -> Rust path. The package-local AAR contains `arm64-v8a` and `x86_64` and has been proven in a clean external React Native Android consumer. |
| ios | experimental local package | CocoaPods autolinks `ios/ServoView.mm`, WebKit, and `ServoKitController.xcframework`. Exact-package Release builds pass for device arm64 and simulator arm64/x86_64. The current baseline passed one attended ARM64 simulator run; each future candidate still needs its own manual runtime gate. Servo-on-iOS is deferred. |
| macos | source-side experiment, not supported | Fabric/AppKit and package-private desktop C implementation over Rust ServoKit; not packaged, with no runtime or distribution contract claimed. |
| windows | deferred | No Windows runtime adapter, package integration, or distribution is supported. |

Package APIs and local packaging remain unstable. No npm release exists yet;
`0.1.0` is still local and pre-public. The exact locally packed artifact
contains the thin Android/iOS adapters, the dual-ABI Android AAR, and the iOS
controller XCFramework. Consumer builds do not run Cargo, download native
artifacts, or refer to the ServoKit workspace.

For current capability coverage and the remaining Servo embedder surfaces, see
[`feature-coverage.md`](./feature-coverage.md). For the platform-specific
host-control matrix, see
[`host-control-capabilities.md`](./host-control-capabilities.md).

## iOS WKWebView baseline matrix

This table summarizes the packaged iOS adapter. Fixture pages that load on iOS
do not by themselves establish a host-control support matrix or claim native/
default/customizable support. The full matrix lives in
[`host-control-capabilities.md`](./host-control-capabilities.md).

| Surface | iOS baseline posture |
| --- | --- |
| Engine identity | WKWebView/WebKit above the packaged Servo-free portable Rust controller. |
| Platform policy | The current path fits the Apple/WebKit-constrained iOS baseline. Servo-on-iOS is deferred. See Open Web Advocacy's [Apple Browser Ban](https://open-web-advocacy.org/apple-browser-ban/) summary for context. |
| Render/navigation | `url`, `loadUrl`, `reload`, `goBack`, `goForward` map to WKWebView. |
| Focus | `focus`, `blur`, and `onFocusChanged` are mapped where WebKit/UIKit exposes equivalent behavior. |
| Events | URL, title, load status, history, and focus are emitted; WebKit-specific request details are not exposed. |
| Navigation policy | `onShouldStartLoadWithRequest({ url })` maps to `WKNavigationDelegate` and defaults to allow on missing/throwing/rejecting/timeout callbacks. |
| JavaScript evaluation | Mounted `ServoViewHandle.evaluateJavaScript(script)` maps to `WKWebView.evaluateJavaScript` and uses WKWebView/lifecycle error categories. |
| JavaScript dialogs | `onJavaScriptDialog` maps WebKit alert/confirm/prompt callbacks to the shared RN request shape; without an RN handler, alert completes and confirm/prompt cancel. |
| Deferred/not claimed on iOS | Context menus, cursor/fullscreen/crash events, Servo-specific controls, preload/user scripts, secure web-content IPC, accessibility parity, Servo-on-iOS, and publication. Future release candidates still require an attended runtime gate. |

## iOS package path

The package includes `Servokit.podspec`, `ios/ServoView.mm`, and
`ios/ServoKitController.xcframework`. CocoaPods autolinks the Objective-C++
adapter, WebKit, and the XCFramework. The XCFramework contains the Servo-free
portable Rust controller for device arm64 and simulator arm64/x86_64; it does
not contain Servo. Consumer builds do not compile Rust or download native
artifacts.

The portable Rust controller owns command validation, request identity, pending
semantics, fallback policy, and response validation. Objective-C++ owns the
`WKWebView`, WebKit delegate completion and timer objects, KVO/recycling, effect
execution, native engine handles, and main-thread scheduling.

## macOS source path

The macOS pod projection uses `macos/ServoView.mm` plus AppKit. The adapter owns
the `NSView`, native handles, geometry, input translation, native presentation,
and main-queue scheduling; it sends bounded commands and events through the
package-private desktop C boundary to the Rust ServoKit runtime. Local
macOS consumers select the repository-only `macos/ServokitMacOS.podspec` so its
Servo dependency never enters the published iOS pod. Local verification
can use a prepared `ServoKit.xcframework` with
`SERVOKIT_BUILD_FROM_SOURCE=1` or provide the matching local
`ServoKitMacOSBinary` pod. This is an experimental implementation, not a
supported runtime or published distribution contract.

## Control capability contract

Public control docs should describe capabilities per platform rather than
upstream enum names or transport details. The shared capability vocabulary uses
`dialog`, `navigationPolicy`, `contextMenu`, picker/file flows, permissions,
fullscreen, cursor, focus, and crash/error reporting.

Android maps its supported capabilities to Servo-backed controls through Rust
controller seams plus native platform behavior. The experimental macOS path
uses the same ownership split without establishing a supported runtime
contract. The iOS path maps only the native equivalents exposed by
WKWebView/WebKit and UIKit. Rust owns request identity, pending semantics,
fallback policy, and response validation on both packaged platforms; the iOS
adapter owns WebKit's native completion/timer lifetime and effect execution.
React Native hooks customize presentation and answers without becoming either
engine's state owner.

## Quick usage

The current local package exposes this component on Android and iOS. The macOS
mapping described below remains a repository source experiment.

```tsx
import { useRef } from 'react';
import { ServoView, type ServoViewHandle } from 'react-native-servokit';

function Browser() {
  const servoRef = useRef<ServoViewHandle>(null);

  return (
    <ServoView
      ref={servoRef}
      style={{ flex: 1 }}
      url="https://servo.org"
      onShouldStartLoadWithRequest={async ({ url }) => !url.includes('blocked')}
      onUrlChanged={(event) => console.log(event.nativeEvent.url)}
      onPageTitleChanged={(event) => console.log(event.nativeEvent.title)}
      onFocusChanged={(event) => console.log(event.nativeEvent.isFocused)}
      onCursorChanged={(event) => console.log(event.nativeEvent.cursor)}
      onFullscreenChanged={(event) => console.log(event.nativeEvent.isFullscreen)}
      onCreateNewWebViewRequested={(event) => console.log(event.nativeEvent.targetUrl)}
      onLoadStatusChanged={(event) =>
        console.log(event.nativeEvent.status)
      }
    />
  );
}
```

`ServoViewHandle` currently exposes:

- `loadUrl(url: string)`
- `reload()`
- `goBack()`
- `goForward()`
- `focus()`
- `blur()`
- `evaluateJavaScript(script: string): Promise<string>`

On Android, `evaluateJavaScript` follows Servo `WebView::evaluate_javascript` on
the mounted view path. The experimental macOS adapter maps the same command to
Servo. The React Native adapter resolves with a JSON string serialization of
Servo's `JSValue` result
such as `{"type":"string","value":"Example title"}` or `{"type":"null"}`.
Promise rejection surfaces Servo evaluation error categories such as
`DocumentNotFound`, `CompilationFailure`, `EvaluationFailure`, `InternalError`,
`WebViewNotReady`, and `SerializationError`. On iOS, the same mounted
`ServoViewHandle.evaluateJavaScript(script): Promise<string>` API is
WKWebView-backed through `WKWebView.evaluateJavaScript`; it uses the same tagged
JSON-string convention where WebKit returns serializable `string`, `number`,
`boolean`, `null`, array, or object results. Native failures use
`EvaluationFailure`, `InternalError`, `WebViewNotReady`, or
`SerializationError`; unmounting an unsettled request produces the JavaScript
`LifecycleError` category. This mounted API is not preload/user scripts,
RN-WebView-style `injectJavaScript`, per-webview JavaScript/security policy,
detached/headless scripting, or web-content-to-native messaging.

On Android, these mounted baseline ref methods and the current navigation/dialog/
context-menu responses travel through mounted Fabric `ServoView` commands, the
shared Android host module's Kotlin/JNI wrapper, and the generic JNI
controller-command envelope documented in
[`react-native-rust-control-seam.md`](./react-native-rust-control-seam.md). On
iOS, the baseline routes supported commands, navigation-policy decisions, and
JavaScript alert/confirm/prompt dialogs through the portable Rust controller
before Objective-C++ applies effects to WKWebView/WebKit. On macOS, the mounted
Fabric command travels through the AppKit adapter and package-private desktop C
boundary to the Rust controller envelope.

`onShouldStartLoadWithRequest` is the shared React Native-facing name for
Android Servo `WebViewDelegate::request_navigation` decisions and iOS WebKit
`WKNavigationDelegate` policy decisions. It returns `boolean | Promise<boolean>`
because the React Native decision is resolved asynchronously. If the callback is
absent, throws, rejects, or never resolves, both platform paths explicitly fall
back to allowing the navigation so neither Servo nor WebKit wedges on a pending
request. The iOS callback receives the shared `{ url }` request shape only; no
WebKit-specific request fields are exposed in this baseline.

When `onJavaScriptDialog` is present, Rust owns the pending dialog identity and
semantics while the React Native callback only supplies UI and policy. iOS
mirrors the same React Native request shape for WebKit `WKUIDelegate`
alert/confirm/prompt callbacks; Objective-C++ retains the native completion
object while the portable controller remains the source of truth. Without an
RN handler, Android falls back to host-native dialogs; iOS uses
a no-UI WebKit-safe default: alerts complete, while confirm and prompt cancel.
Dismissal events are emitted for RN-owned dialogs when the platform reports or
settles the pending dialog. The Rust-owned controller command/control seams are
documented in
[`react-native-rust-control-seam.md`](./react-native-rust-control-seam.md).

Servo-backed Android policy notifications currently include `onFocusChanged`,
`onCursorChanged`, `onFullscreenChanged`, and
`onCreateNewWebViewRequested`, so apps can keep native chrome in sync without
relying on placeholder APIs. The iOS WKWebView baseline currently emits focus
but not cursor/fullscreen/crash/create-new-webview parity.
`onCreateNewWebViewRequested` is an Android-only React Native adapter event. It
maps Servo `WebViewDelegate::request_create_new` /
`CreateNewWebViewRequest` into host-routed popup/new-window intent and emits
`{ parentWebViewId, parentUrl, targetUrl, windowFeatures, policy }`, with
`parentUrl`, `targetUrl`, and `windowFeatures` nullable. The prop itself is not
a Servo crate API. React Native Android uses default-deny; a host can ignore the
informational intent or interpret a surfaced URL in its own UI, but this adapter
does not create, present, or adopt managed child views. iOS does not emit this
event.

Lower-level Rust ServoKit separately supports root-scoped
`PopupRequestPolicy::ManagedChild` and managed-child surface lifecycle. Those
children do not allow a second root or provide a general N-root pool/public
React Native multi-view API. See
[Architecture](../ARCHITECTURE.md#current-servo-runtime-limit).

## Android host module boundary

The current Android package shape is intentionally split:

- `react-native-servokit` owns Fabric `ServoView`, React Native props/events/refs,
  generated RN glue, and adapter-only ergonomics.
- `crates/servokit-host-android/android` is the crate-owned reusable Android Gradle
  library boundary below the RN package. It packages `libservokit_host_android.so` and
  holds the current proof-only Kotlin/JNI host wrapper plus shared event bridge.
- `crates/servokit-host-android` owns the Rust Android host implementation and
  JNI exports consumed by that shared Android module.

`crates/servokit-host-android/android` remains the crate-owned producer source
module, not a stable Kotlin SDK. It builds and verifies one release AAR with
`arm64-v8a` and `x86_64`; the staging task depends on the exact-AAR verifier
before copying that artifact into `react-native-servokit`.

The packed React Native package contains that AAR and the thin Android adapter,
not the host Gradle project or Rust workspace. It also contains the iOS podspec,
Objective-C++ adapter, and portable-controller XCFramework. Normal React Native
autolinking resolves the installed package's native projects: Android depends
on the package-local AAR, while CocoaPods links the iOS adapter, WebKit, and
XCFramework. Consumer builds do not invoke Cargo, download a native artifact,
or refer back to the ServoKit checkout. Maven publication is not part of this
distribution path.

## Exact-package iOS readiness

From `packages/react-native-servokit`:

```sh
node scripts/validate-packed-consumer.mjs --ios-only
```

The command packs the exact local tgz, installs it into a clean external React
Native consumer, and proves Release builds for device arm64, simulator arm64,
and simulator x86_64. It does not perform the separate simulator runtime gate,
publish the package, or promote a release.

The current baseline separately passed one attended ARM64 simulator run. That
acceptance does not replace the manual runtime gate for future package
candidates.

## Documentation

Durable package, architecture, and readiness context lives in the checked-in
docs below. Tactical progress stays in the active tracker.

| Doc | Purpose |
| --- | --- |
| [`../README.md`](../README.md) | Repo overview and documentation index |
| [`../ARCHITECTURE.md`](../ARCHITECTURE.md) | Canonical architecture front door and fast system mental model |
| [`runtime-and-module-map.md`](./runtime-and-module-map.md) | Detailed ownership split, runtime flow, and desktop vs mobile embedding tradeoffs |
| [`host-control-capabilities.md`](./host-control-capabilities.md) | Android Servo-backed vs iOS WKWebView/WebKit-backed host-control capability matrix |
| [`react-native-rust-control-seam.md`](./react-native-rust-control-seam.md) | Rust controller command transports for Servo-backed hosts and the portable iOS controller |
| [`servokit.md`](./servokit.md) | Target Servokit module map for embedder, host, binding, and platform crates |
| [`readiness-checks.md`](./readiness-checks.md) | Build and smoke-check matrix for Rust, desktop, React Native Android/iOS, and native Android proof paths |
| [`surface-modes.md`](./surface-modes.md) | Surface vocabulary and lifecycle ADR for native-child, CPU-offscreen, and future GPU-layer modes |
| [`rust-dependency-baseline.md`](./rust-dependency-baseline.md) | Servo 0.3.0 baseline, shared lockfile roots, and dependency update workflow |
| [`embedded-surface-contract.md`](./embedded-surface-contract.md) | App-owned layout/window embedded-surface contract shared by GPUI, `winit`, React Native Android hosts, and the iOS WKWebView baseline |
| [`android-build.md`](./android-build.md) | Android prerequisites, local example flow, and build notes |
| [`feature-coverage.md`](./feature-coverage.md) | Current Servo embedder-control coverage and React Native package status |
| [`upstream-servo.md`](./upstream-servo.md) | How this repo follows upstream Servo and why `upstream/servo` is a pinned submodule |

## License

MIT
