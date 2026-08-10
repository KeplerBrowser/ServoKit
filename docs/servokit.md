# ServoKit Target Architecture

## Intent

ServoKit is a reusable Rust runtime and thin-adapter toolkit for embedding
Servo: a slim Rust layer over the upstream `servo` crate and servoshell-style
embedding patterns. It should make Servo usable from native Rust shells,
Servo-backed React Native Android and macOS adapters, and Kotlin/native Android
hosts without copying framework glue across adapters. Its Servo-free portable
controller also keeps shared React Native iOS command and pending semantics in
Rust while WebKit remains native-owned.

The closest analogy is a narrower CEF for Chromium, GeckoView for Gecko, or
WebKit framework surface for WebKit. ServoKit should expose reusable
Servo-shaped runtime, webview, surface, and embedder APIs while product adapters
own their platform and framework ergonomics. The current Android/React Native
paths are proof surfaces and local validation strengths, not the product center;
future IPC, preload/user-script, and broader policy systems still need explicit
APIs and security models before they are claimed as supported.

The canonical module diagram lives in
[`runtime-and-module-map.md`](./runtime-and-module-map.md#servokit-module-map). This document keeps
the responsibilities and design rules behind that diagram precise. The
layout-host embedding contract for app-owned windows and app-owned layout
regions lives in [`embedded-surface-contract.md`](./embedded-surface-contract.md).
The cross-mode surface vocabulary and lifecycle ADR lives in
[`surface-modes.md`](./surface-modes.md).

## Workspace Shape

```text
servokit/
  crates/
    servokit
    servokit-embedder
    servokit-host
    servokit-host-android
    servokit-host-desktop
    servokit-controller-ffi

  packages/
    react-native-servokit/

  examples/
    android-kotlin-browser
    android-native-example
    desktop-gpui
    desktop-winit
    fixtures
    react-native-app
    react-native-macos-app

  upstream/
    servo
```

The current Cargo workspace, native library names, React Native Codegen inputs,
and proof examples use the agreed Servokit names and module map.

## Module Responsibilities

| Target module | Owns | Must not own |
| --- | --- | --- |
| `servokit` | Public Rust facade for native shells and Rust embedders: runtime/session/webview handles, command methods, event/delegate API, a small root happy path, and curated module reexports from lower crates | React Native, platform binding glue, Android framework objects, raw Servo internals that should stay behind the embedder, or wholesale dumps of internal crate APIs |
| `servokit-embedder` | Reusable Servo embedder layer: internal runtime state for sessions/webviews/handles, Servo `WebView`/delegate/embedder-control translation, pending Servo request objects, Servo-to-Servokit event conversion, and canonical control/event/command payloads when they directly reflect Servo delegate APIs | Android lifecycle, React Native callback ergonomics, generated bindings, platform-specific UI, premature stable `servokit` facade APIs, or duplicate public facade modules |
| `servokit-host` | Target home for host-neutral surface and platform seams: geometry, raw-window-handle vocabulary, host-neutral `SurfaceDelegate`-style callbacks that do not depend on `WebViewHandle`, surface targets, and shared platform service traits | Heavy platform implementations, Android framework objects, JNI, generated bindings, React Native package API, Servo implementation internals, or dependency edges back up into `servokit-embedder` |
| `crates/servokit-host-android/android` | Crate-owned repo-local reusable Android Gradle library boundary: packages `libservokit_host_android.so`, carries the first shared Kotlin/JNI host wrapper and event bridge below adapters, and is consumed by the RN adapter, the Kotlin Android browser example, and the native Android proof app | Stable Kotlin SDK/API promises, React Native view ergonomics, or portable embedder policy |
| `servokit-host-android` | Android implementation crate for the host traits: `SurfaceView`/`ANativeWindow`, frame scheduling, IME, clipboard, permissions, intents, picker dialogs, Android fallback UI, native library packaging, and JNI entrypoints consumed by the reusable Android host module | Portable embedder policy, public Rust facade, React Native package API |
| `servokit-host-desktop` | Package-private C boundary over the Rust ServoKit runtime for target-gated desktop framework adapters | Public React Native API, AppKit/WinUI view ownership, stable SDK/ABI promises, or platform UI |
| `servokit-controller-ffi` | Servo-free portable controller C boundary: opaque controller handles, bounded input/result bytes, and panic containment for native engine adapters | Servo engine/runtime binaries, platform views, WebKit objects, React Native API, or public stable ABI promises |
| `react-native-servokit` | NPM package: `ServoView`, Fabric props/events/commands/refs, RN callbacks, Codegen inputs, packaged Android/iOS native dependencies, and thin platform adapters | Servo delegate internals, portable controller semantics, Android host producer glue, detached TurboModules, or example app behavior |
| `desktop-winit` | Thin desktop example app that owns its `winit` event loop/window/input glue and uses the public `servokit` surface facade to render URLs and emit shared events | Reusable Servo integration internals, React Native package behavior, Android/JNI concepts |
| `desktop-gpui` | Thin desktop example app that owns a GPUI window/layout, uses `servokit::surface::macos::AppKitChildSurface` to manage an AppKit child `NSView` for the layout slot, and renders URLs/show shared events in GPUI chrome through the public `servokit` surface facade | Reusable Servo integration internals, a Servokit-owned GPUI component crate, React Native package behavior, Android/JNI concepts |
| `android-native-example` | Proof app that uses the shared `crates/servokit-host-android/android` module from Android without React Native to render fixtures, expose controls, and log shared events | Stable Kotlin SDK shape or reusable Android package API |
| `android-kotlin-browser` | App-facing Kotlin Android browser example that uses the same shared Android host/control coordinator below React Native, with URL chrome and fixture shortcuts | Stable Kotlin SDK shape, public AAR promises, or reusable package API |

## Dependency Direction

Native Rust shells and desktop examples should use the Rust facade directly:

```text
desktop-winit / desktop-gpui -> servokit -> servokit-embedder + servokit-host
```

React Native Android and Kotlin/native Android examples share the Android
host/control coordinator below their app-facing ergonomics:

```text
react-native-servokit (Fabric ServoView + RN ergonomics)
  -> crates/servokit-host-android/android (shared Gradle/Kotlin/JNI host module)
      -> servokit-host-android
          -> servokit-embedder
              -> Servo

examples/android-kotlin-browser (Kotlin browser chrome)
  -> crates/servokit-host-android/android
      -> servokit-host-android
          -> servokit-embedder
              -> Servo

examples/android-native-example (proof Android UI)
  -> crates/servokit-host-android/android
      -> servokit-host-android
          -> servokit-embedder
              -> Servo
```

`servokit-host-android` also depends on `servokit-host` for host-neutral surface
traits and values; the module map shows that side of the backend split. React
Native Codegen generates the mounted Fabric component glue from the package's
`ServoView` specification.

React Native macOS uses the same specification through a thin AppKit adapter and
the package-private desktop C boundary. React Native iOS dispatches through the
Servo-free portable Rust controller, then its Objective-C++ adapter applies
effects and observations to WKWebView/WebKit.

## Public Facade Shape

`servokit` should behave like a conventional Rust facade crate: callers import
stable Servokit paths, while lower crates remain free to reorganize their
implementation. The facade should provide namespaced modules such as
`servokit::runtime`, `servokit::webview`, `servokit::surface`,
`servokit::controls`, `servokit::events`, `servokit::input`, and
`servokit::host`, with only a small happy-path set reexported at the crate root
(for example `Runtime`, handles, core errors/events, and common surface values).
Avoid `pub use servokit_embedder::*`; every reexport is part of the public
contract.

Canonical control payloads should have one implementation home. If a request is
Servo-delegate shaped, such as select elements, dialogs, navigation policy, or
permissions, the canonical translation belongs in `servokit-embedder`; the
public facade may reexport it under `servokit::controls`. Do not maintain two
independent `controls` modules with overlapping types.

For Servo-backed hosts and the portable iOS controller, controller request
creation, request ids,
expiry/timeout policy, fallback policy, invalid-response handling, response
validation, and command names belong with ServoKit browser/control semantics.
Android and macOS adapters map those semantics to Servo-backed hosts. The iOS
Objective-C++ adapter maps portable-controller effects to WKWebView and owns
native delegate completions/timers, KVO/recycling, engine handles, and
main-thread execution.

## Workspace Mapping

| Area | Responsibility |
| --- | --- |
| `crates/servokit` | Public Rust facade prototype reexporting the embedder-owned runtime/session/webview handles, baseline browser commands, host update pumping, and event draining for native Rust callers |
| `crates/servokit-embedder` | Shared Servo-shaped embedder vocabulary, runtime/webview command and host-input paths, host events, navigation values, policy payloads, pure adapter state, reusable Servo delegate/webview helpers, and event bridge encoding |
| `crates/servokit-host` | Host-neutral platform value types and surface seams. `HostSurface`, `SurfacePoint`, `SurfaceSize`, `SurfaceViewport`, `NativeChildSurface`, `CpuOffscreenSurface`, `SurfaceTarget`, `SurfaceMode`, `SurfaceError`, and the host-neutral `SurfaceDelegate` trait live here, with shared geometry reexported by `servokit-embedder` for event payload and runtime compatibility; the runtime `Host` trait currently lives in `servokit-embedder` |
| `crates/servokit-host-android/android` | Crate-owned repo-local reusable Android Gradle module that builds/packages `libservokit_host_android.so` and owns the first shared Kotlin/JNI host wrapper and typed event bridge below adapters |
| `crates/servokit-host-android` | Android host implementation, direct JNI bridge, Android native-window/render backend, clipboard, input, and fallback UI. Its Cargo package is `servokit-host-android`; its native library is `servokit_host_android` |
| `crates/servokit-host-desktop` | Package-private bounded C boundary used by the React Native macOS adapter to drive a Rust ServoKit host |
| `crates/servokit-controller-ffi` | Servo-free four-function C boundary that packages the portable controller for native adapters |
| `packages/react-native-servokit/android/src/main/java/org/servo/servokit/reactnative` | RN adapter code: `ServoView` plus Fabric manager/package classes above the shared Android host module |
| `packages/react-native-servokit/ios` | Packaged UIKit/WKWebView Fabric adapter plus `ServoKitController.xcframework`; CocoaPods autolinks WebKit and the Servo-free portable Rust controller without selecting Servo or consumer-side Rust build hooks |
| `packages/react-native-servokit/macos` | AppKit Fabric adapter over the package-private desktop C boundary |
| `packages/react-native-servokit/src/ServoView.tsx` and `ServoViewNativeComponent.ts` | Public component wrapper and Fabric Codegen specification |
| `examples/react-native-app` | React Native example app glue for the local `react-native-servokit` package across the Android Servo-backed path and iOS WKWebView baseline |
| `examples/react-native-macos-app` | Verification app for the experimental React Native macOS AppKit/private-C/Rust implementation; no supported or distributed runtime contract is claimed |
| `examples/desktop-winit` | Desktop proof example using `servokit`'s surface facade with app-owned `winit` event loop/window/input glue |
| `examples/desktop-gpui` | Desktop proof example using `servokit`'s surface facade inside a GPUI-owned layout slot backed by `servokit::surface::macos::AppKitChildSurface` |
| `examples/android-native-example` | Android proof app using the shared `crates/servokit-host-android/android` module below React Native; proof-only Kotlin glue |
| `examples/android-kotlin-browser` | Kotlin Android browser example using the same shared Android host/control coordinator and no React Native runtime dependency |
| `examples/fixtures` | Shared smoke pages for desktop, React Native Android, and native Android readiness checks |

## Surface facade and current Servo-backed implementation

Servo and Surfman use surface vocabulary for native/window rendering,
offscreen rendering, and eventual texture handoff, so Servokit uses
`Surface` as the public rendering/embedding term.

Today, `servokit` exposes this path behind the `servo` feature. Applications own their windows, event loops, and layout trees;
Servokit owns the Servo webview/delegate/embedder-control/render integration. A
host creates a `servokit::runtime::Runtime<servokit::surface::SurfaceHost<_>>`,
implements the host-neutral `servokit::surface::SurfaceDelegate`, and attaches a
`HostSurface` plus `SurfaceViewport` to an app-owned native or
offscreen/composited surface. The facade reexports `NativeChildSurface`,
`CpuOffscreenSurface`, `SurfaceTarget`, `SurfaceMode`, and `SurfaceError` from
`servokit-host` while keeping Servo rendering-context adaptation in `servokit`.
On macOS, `servokit::surface::macos::AppKitChildSurface` provides the CEF-like
native child-view helper for app-owned AppKit parents: hosts supply the parent
handle, logical bounds, and scale factor; Servokit keeps the child `NSView` and
borrowed handles aligned for `NativeChildSurface`. Browser commands, host
input, update pumping, viewport changes, and event draining stay on the public
runtime facade path.

The public surface vocabulary is now `SurfaceHost`, `SurfaceHostOptions`,
`SurfaceDelegate`, `NativeChildSurface`, `CpuOffscreenSurface`,
`SurfaceTarget`, `SurfaceFrame`, and `SurfaceError`. The old pre-release facade
names are gone rather than kept as migration aliases.
Native child/window surfaces are the initial CEF-like path; CPU offscreen
surfaces are useful for screenshots/debug/tests; future GPU layer surfaces remain
separate strategic work. See [`surface-modes.md`](./surface-modes.md) for the
current mode taxonomy and lifecycle rules.

The surface facade must not expose `winit`, GPUI, React Native, Android
framework, generated binding, or raw Servo rendering types. Hosts provide
viewport origin/size/scale, event-loop wake behavior, present/composite
callbacks, and clipboard/platform services through Servokit traits/options.

## Design Rules

1. `servokit` is the public Rust crate a native shell should import. It should
   expose namespaced modules plus a small root happy path, not wholesale
   internal-crate reexports.
2. `servokit-embedder` owns Servo integration and exports the shared
   embedder-facing ids, events, commands, and policy/control payloads unless a
   separate types crate becomes necessary later.
3. `servokit-host` is the target home for host-neutral surface traits and values.
   Reusable Servokit-owned platform implementations live in `servokit-host-*`
   crates when they are heavy packaging units; thin app-owned proof glue can stay
   in examples.
4. `servokit-host-android` stays separate from `servokit-host` because JNI,
   `ANativeWindow`, `cdylib` packaging, Gradle integration, and Android lifecycle
   are heavy platform implementation details. Shared host-neutral pieces should
   move down into `servokit-host` instead of folding Android upward.
5. The reusable `crates/servokit-host-android/android` Gradle module is the first
   Android packaging boundary below `react-native-servokit` and the Kotlin/native
   Android examples. It may remain repo-local and proof-only while the
   dependency/publication story hardens, but Android host packaging/JNI glue
   should deepen there instead of drifting back up into adapter packages or
   example apps.
6. `react-native-servokit` is the public React Native package name. Its mounted
   view API uses Fabric commands and platform adapters, not a detached
   TurboModule.
7. Do not introduce `servokit-core`, `servokit-protocol`, or `servokit-jni` as
   target crate names unless a later decision explicitly changes this map.

## Current architecture baseline

The current baseline includes:

- Surface lifecycle, host updates, and app-owned native/window surfaces modeled
  through the `servokit` facade and host-neutral surface types.
- On Servo-backed Android and macOS paths, React Native-facing policy responses
  and mounted browser commands resolve through Rust-owned controller seams
  instead of ad hoc Fabric view state. On iOS, the portable Rust controller
  owns the corresponding command and pending semantics while Objective-C++
  executes effects against WKWebView/WebKit.
- Shared host events cross one structured bridge and are decoded by platform
  adapters.
- The Servo adapter boundary owns testable Servo-to-Servokit delegate, command,
  and event translation.
- Android render backend and native-window vocabulary live behind
  `servokit-host-android` and the shared Gradle module, not in shared
  host-neutral crates.
- The React Native package uses its mounted Fabric component as the JavaScript
  boundary and keeps platform adaptation thin.
- Native Rust callers use `crates/servokit`; the desktop `winit` and GPUI
  examples stay thin.
- Servokit package and native library names are used consistently without
  pre-release migration aliases.
- Build and smoke validation for Rust, desktop, packaged React Native
  Android/iOS, and native/Kotlin Android proof surfaces lives in
  [`readiness-checks.md`](./readiness-checks.md).

Architecture evolution should follow these durable rules:

1. Continue deepening the `servokit-embedder` and `servokit-host` split. The
   host-neutral surface vocabulary and `SurfaceDelegate` live in
   `servokit-host`; callbacks that still need `WebViewHandle` or other
   embedder/runtime identity remain in `servokit` as compatibility adapters.
2. Keep `servokit` centered on namespaced facade modules and the stable-enough
   surface vocabulary already exposed through `servokit::surface`.
3. Keep any future ABI-affecting binding changes separate from behavior work.
4. Treat `servokit` facade vocabulary as stable enough for repository examples,
   but not yet a semver-stable external SDK.
5. Keep proof-only surfaces explicit: desktop example UX, native Android Kotlin
   app glue, React Native example UI, and generated Codegen glue can evolve
   without compatibility guarantees.
