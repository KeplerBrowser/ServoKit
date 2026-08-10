# ServoKit readiness checks

This matrix records durable build and smoke checks for ServoKit's repo-local
proof surfaces. It separates automation-friendly checks from manual smoke checks
so contributors can choose the narrowest validation path that matches a change.
The matrix is not a tactical tracker; progress belongs in the active approved
tracker, which may be local.

## Shared validation baseline

Repo-local proof surfaces should exercise the same browser-control and
host-lifecycle concepts through Servokit-owned seams:

| Capability | Expected coverage |
| --- | --- |
| Embedded rendering | Render the selected engine into a host-owned native surface, view, or layout region. |
| Initial URL load | Accept an initial URL during example startup and begin loading it through the Servokit host/facade or adapter path. |
| Load URL | Expose a load-url command that navigates an existing webview to a new URL. |
| Reload | Expose a reload command for the current page. |
| Back / forward | Expose back and forward commands that use Servo web history when available. |
| Focus / blur | Forward host focus into the embedded webview and clear it when the host blurs the view. |
| Surface lifecycle | Model host surface create/attach, resize, detach, and destroy without folding the control handle into a platform view lifetime. |
| Host updates | Let the platform host pump Servo/update work from its native event or frame loop so rendering and events keep moving. |
| Shared host events | Deliver the shared Servokit host event stream, including URL, title, status text, load status, history, focus, cursor, fullscreen, crash/error, and surface attach/resize/detach events. Delegate-owned policy events can ride the same stream when implemented. |

Platform-specific UI can differ, but command and event concepts should stay
aligned with the Rust facade and shared embedder model. The shared fixture pages
in [`examples/fixtures`](../examples/fixtures/README.md) cover common smoke
flows without relying on React Native-specific assumptions. Loading a fixture
page proves only the checked smoke path; it is not a claim that ServoKit owns
native defaults, customization hooks, or complete support for that capability.

## Desktop platform readiness

Desktop checks are vertical validation slices for the same reusable ServoKit
embedding layer that the Android and React Native proofs exercise. The Rust
runtime/facade owns the browser-control vocabulary, surface lifecycle, and Servo
integration; each desktop host owns only unavoidable native window/view/event
loop glue. Treat macOS, Windows, Linux, Android, and React Native as proof
surfaces for one architecture, not as separate architectures.

The supported initial desktop surface path is `NativeChildSurface`: the host app
or framework owns the native window or layout slot, then lends ServoKit borrowed
`raw-window-handle` display/window handles through the public surface facade.
`GpuLayerSurface` remains future, experimental, and upstream-dependent; desktop
readiness checks below must not claim IOSurface, shared-D3D, `dmabuf`, or other
exported-GPU-layer support until Servo exports a supported host-consumable GPU
surface contract and ServoKit implements it.

| Desktop platform | Automation-friendly readiness signals | Manual smoke signals | Surface posture | Current limits |
| --- | --- | --- | --- | --- |
| macOS | `cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked` on a macOS desktop Rust host.<br>`RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/servokit-gpui-target CARGO_PROFILE_DEV_DEBUG=0 cargo check --manifest-path examples/desktop-gpui/Cargo.toml --locked` for the GPUI/AppKit proof. | Start the fixture server, then run `examples/desktop-winit` and `examples/desktop-gpui` against `http://127.0.0.1:8481/smoke/index.html`. Confirm the visible `Smoke fixtures ready` marker, resize behavior, focus/input flow, and event output for each example. | `NativeChildSurface` for the whole-window `winit` path and for the GPUI/AppKit child `NSView` layout slot. The AppKit helper lives in `servokit::surface::macos` and produces borrowed native-child handles for the same facade. | Requires an interactive macOS/AppKit session for GUI smoke. The GPUI path is a proof surface, not a stable `servokit-gpui` product crate. `GpuLayerSurface` / IOSurface export is not implemented. |
| Windows | `cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked` on a prepared Windows Rust/Servo workstation or runner. | Start the fixture server, run `examples/desktop-winit`, and confirm a native Windows window renders the smoke fixture while resize, focus, URL/load, and close/detach events appear in stdout. | `NativeChildSurface` through the `winit` / `raw-window-handle` path. Keep any Windows-specific details target-gated and expressed through existing ServoKit facade vocabulary. | No Windows framework adapter is claimed yet. GUI smoke requires an interactive desktop session. `GpuLayerSurface` / shared-D3D export is not implemented. |
| Linux | `cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked` on a selected X11 and/or Wayland baseline. | Under the selected X11/Wayland session, start the fixture server, run `examples/desktop-winit`, and confirm the smoke fixture, resize/focus behavior, URL/load events, and close/detach events. | `NativeChildSurface` through the selected `winit` / `raw-window-handle` display path. Native-child readiness is separate from future `dmabuf` or exported-layer work. | The initial Linux compositor baseline remains undecided, so no cross-compositor parity is claimed. `GpuLayerSurface` / `dmabuf` export is not implemented. |

### macOS native-child proof paths

macOS currently has two source-built proof paths for the same ServoKit
`NativeChildSurface` contract:

1. `examples/desktop-winit` is the whole-window native-child proof. The app owns
   the `winit` window/event loop, lends borrowed `raw-window-handle` display and
   window handles through `servokit::surface::NativeChildSurface`, pumps
   `Runtime::perform_updates`, and prints URL/load/focus/surface events to
   stdout.
2. `examples/desktop-gpui` is the AppKit child-view proof. GPUI owns the window,
   chrome, and layout slot; `servokit::surface::macos::AppKitChildSurface`
   creates/refreshes the child `NSView` for that slot and exposes borrowed
   native-child handles to the same ServoKit surface facade. Keep this helper in
   `servokit::surface::macos` as a proof-surface utility; do not present it as a
   stable `servokit-gpui` product crate.

For a macOS manual smoke pass, start the shared fixture server once:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Then run the whole-window `winit` proof:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- --smoke http://127.0.0.1:8481/smoke/index.html
```

Confirm that a native macOS window opens, the page visibly shows `Smoke fixtures
ready`, resize keeps the Servo content aligned with the window, and stdout shows
surface attach/resize/detach plus URL/load/focus events and a `smoke
result=pass` summary.

Run the GPUI/AppKit child-slot proof separately:

```sh
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml -- http://127.0.0.1:8481/smoke/index.html
```

Confirm that GPUI chrome surrounds the Servo region, the child `NSView` shows
`Smoke fixtures ready`, resizing the GPUI window keeps the child view aligned to
the browser slot, and the GPUI footer reports URL/load/title/status event output
from `ServokitEvent` values. This validates macOS native-child embedding only;
IOSurface / `GpuLayerSurface` export remains future upstream-dependent work.

## Matrix

| Target | Purpose | Automation-friendly command(s) | Manual smoke command(s) | Expected result | Notes / limitations |
| --- | --- | --- | --- | --- | --- |
| Rust workspace / crates | Prove the shared Rust facade, embedder vocabulary, and host-neutral crate tests still compile and pass without platform UI. | `cargo test --manifest-path crates/Cargo.toml -p servokit-embedder --locked`<br>`cargo test --manifest-path crates/Cargo.toml -p servokit-host --locked`<br>`cargo test --manifest-path crates/Cargo.toml -p servokit --locked` | Full local sweep when disk/time allow:<br>`cargo test --manifest-path crates/Cargo.toml --workspace --locked` | Automation-friendly package tests pass. The full workspace sweep should pass on a prepared developer machine with the required Servo/native dependencies. | The lightweight package tests are the cheapest headless signal. Full workspace tests can pull in heavier desktop/Android Servo dependencies and are better suited to prepared local or expanded CI runners. |
| desktop private C boundary | On macOS and Windows, compile the package-private desktop header as C++17, link the real Rust static-library exports, and execute boundary validation without relying on a Rust `rlib` import. | `cargo test --manifest-path crates/Cargo.toml -p servokit-host-desktop --test c_boundary --locked` | None. | On supported macOS/Windows targets, Cargo builds the static library, then the registered target compiles, links, and runs the C++ harness in normal and `-DNDEBUG -O2` modes. Both executions reference all nine exported symbols and validate create/token handling, rejected null native-handle attach with zero generation output, exact bounded command bytes (including a generic response envelope, non-NUL input, excluded trailing bytes, invalid UTF-8/JSON, and oversize rejection), wrong-token behavior, idempotent detach, event draining, and destroy. | Requires the prepared native Servo desktop toolchain and a C++17 compiler. The headless harness deliberately uses only an invalid native handle; it does not claim live attachment. Successful attach/resize/input/detach/reattach remains an AppKit or Win32 adapter smoke gate with a real host view/window. |
| desktop winit example | Prove the app-owned `winit` example builds, opens a native window, renders a Servo page through the public `servokit` surface facade, forwards basic input, pumps updates, and traces shared events. | Compile check on a desktop Rust host:<br>`cargo check --manifest-path examples/desktop-winit/Cargo.toml --locked` | Start fixtures:<br>`python3 -m http.server 8481 --directory examples/fixtures`<br>Run the interactive example:<br>`cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- http://127.0.0.1:8481/smoke/index.html`<br>Run deterministic smoke mode:<br>`cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- --smoke http://127.0.0.1:8481/smoke/index.html` | A native window opens and renders `Smoke fixtures ready`. Stdout shows platform, surface attach/resize state, URL/load status, focus, and any Servo error/crash details. In smoke mode, the process exits `0` after `Complete` or `1` on timeout/error/crash. Links, form focus/text input, scrolling, and window resize work through the shared Servokit facade in interactive mode. | Manual GUI smoke requires an interactive desktop session. The compile check is headless but still builds the Servo desktop graph, so keep it on a prepared desktop runner if automated. Smoke mode is deterministic after a desktop session is available, but it still opens a native window. |
| desktop GPUI example | Prove a GPUI app can keep ownership of its window/layout while embedding Servo in a GPUI layout slot through the public `servokit` facade. | Compile check on macOS:<br>`RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/servokit-gpui-target CARGO_PROFILE_DEV_DEBUG=0 cargo check --manifest-path examples/desktop-gpui/Cargo.toml --locked` | Start fixtures:<br>`python3 -m http.server 8481 --directory examples/fixtures`<br>Run the example:<br>`RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/servokit-gpui-target CARGO_PROFILE_DEV_DEBUG=0 cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml` | A GPUI window opens with GPUI-owned chrome around a Servo region. The Servo region renders `Smoke fixtures ready`; the chrome shows URL/load/title and the latest Servokit event/input. Window resize keeps the Servo child `NSView` aligned to the GPUI layout slot. | Manual GUI smoke requires macOS/AppKit and an interactive session. The example uses the `servokit::surface::macos::AppKitChildSurface` helper for AppKit child-view glue; keep it framed as a proof-surface helper, not a stable GPUI product crate. |
| React Native Android example | Prove the repo-local React Native Fabric adapter path packages the Android example app against the staged ServoKit AAR. | Headless package/build checks:<br>`bun run --cwd packages/react-native-servokit typecheck`<br>`bun run --cwd packages/react-native-servokit prepare`<br>`bun run --cwd examples/react-native-app build:android` | Optional focused Android checks:<br>`bun run --cwd examples/react-native-app fixtures:sync:android`<br>`bun run --cwd examples/react-native-app test:native:android`<br>Manual app launch on a device/emulator:<br>`bun run --cwd examples/react-native-app android` | TypeScript and package build pass. With a verified AAR already staged, the Android example consumes it through the package's ordinary file dependency. Manual launch remains the render and interaction gate. | Requires the staged two-ABI AAR, JDK 17, Bun dependencies, and the normal React Native Android SDK/NDK toolchain. Producing or replacing the AAR is a separate source-side Gradle/Cargo operation. |
| React Native exact packed-package consumer | Prove the exact locally packed `react-native-servokit` tgz installs in an isolated external React Native consumer with both packaged platform paths and no workspace dependency. | From `packages/react-native-servokit`, run the exact iOS package matrix:<br>`node scripts/validate-packed-consumer.mjs --ios-only` | iOS simulator runtime smoke remains a separate manual gate for each candidate. | The exact tgz contains the Android AAR with `arm64-v8a` and `x86_64`, plus `Servokit.podspec`, `ios/ServoView.mm`, and the Servo-free `ServoKitController.xcframework`. The iOS command installs that tgz in a clean React Native 0.85 consumer and proves Release builds for device arm64, simulator arm64, and simulator x86_64. The current baseline separately passed one attended ARM64 simulator run. | Consumer builds run neither Cargo nor native downloads and do not refer to the ServoKit workspace. No npm release exists yet. The Release matrix itself is not simulator runtime acceptance. |
| React Native iOS example | Exercise the packaged WKWebView/WebKit-backed iOS `ServoView` baseline after the exact-package Release matrix passes. | Package checks:<br>`bun run --cwd packages/react-native-servokit typecheck`<br>`bun run --cwd packages/react-native-servokit prepare` | Manual simulator smoke:<br>`python3 -m http.server 8481 --directory examples/fixtures`<br>`bun run --cwd examples/react-native-app start`<br>`bun run --cwd examples/react-native-app ios`<br>`bun run --cwd examples/react-native-app test:e2e:fixtures:ios` | Manual simulator smoke should render through WKWebView and exercise shared controls, events, navigation policy, mounted JavaScript evaluation, and dialogs. | This row does not establish runtime acceptance for the exact package. The path uses WebKit plus the Servo-free portable Rust controller; Servo-on-iOS remains deferred. Picker, permission, and context-menu page loads remain manual smoke only and do not claim ServoKit-owned native/customizable support. |
| React Native macOS example | Validate that the shared Fabric `ServoView` maps through AppKit and the package-private desktop C boundary into the Rust ServoKit runtime. | Cheap package checks:<br>`bun run --cwd packages/react-native-servokit typecheck`<br>`bun run --cwd packages/react-native-servokit prepare`<br>`bun test packages/react-native-servokit/src/__tests__/macos-fabric-source-routing.test.ts`<br>Prepared native-host gate from the repository root:<br>`SERVOKIT_BUILD_FROM_SOURCE=1 SERVOKIT_SOURCE_DIR="$PWD" bun run --cwd examples/react-native-macos-app pods:macos` | Start Metro:<br>`bun run --cwd examples/react-native-macos-app start`<br>Launch separately:<br>`bun run --cwd examples/react-native-macos-app macos` | The package checks pass, CocoaPods selects `macos/ServoView.mm` and the prepared ServoKit framework, and the app mounts one Servo-backed Fabric view. Manual smoke confirms rendering, resize, scroll, pointer interaction, focus/text input, navigation commands, recycling, and event delivery. | Requires clean macOS build inputs because source identity is enforced, a prepared macOS Servo/Xcode/CocoaPods toolchain, and enough temporary disk for a universal framework. This validates the experimental local architecture/runtime path, not a supported runtime, npm publication, or binary-pod distribution. Infrastructure failure is a native-host gate, not evidence for moving AppKit or WebKit ownership into Rust. |
| Kotlin Android browser example | Prove an app-facing Kotlin Android host can build without React Native while sharing the Android host/control coordinator used below the RN adapter. | Android-enabled runner:<br>`examples/react-native-app/android/gradlew -p examples/android-kotlin-browser :app:assembleDebug` | Install and launch on an arm64 device/emulator:<br>`adb install -r examples/android-kotlin-browser/app/build/outputs/apk/debug/app-debug.apk`<br>`adb shell am start -n org.servo.servokit.androidkotlinbrowser/org.servo.servokit.androidexample.MainActivity`<br>`adb logcat -s ServokitNativeExample NativeServoView ExampleFixtureServer` | Debug APK builds with no React Native runtime dependency. Manual launch shows URL chrome, fixture shortcuts, a Servo `SurfaceView`, and Android fallback controls resolving through `crates/servokit-host-android/android`. | Requires the same Android SDK/NDK/Rust target/`cargo ndk` prerequisites as the React Native Android build. The example is experimental and not a stable Kotlin SDK/AAR. |
| native Android proof app | Prove the non-React-Native Android proof app builds against the shared `:servokit-android-host` module, packages the native library, and exercises the Android host path directly. | Android-enabled runner:<br>`examples/react-native-app/android/gradlew -p examples/android-native-example :app:assembleDebug` | Install and launch on an arm64 device/emulator:<br>`adb install -r examples/android-native-example/app/build/outputs/apk/debug/app-debug.apk`<br>`adb shell am start -n org.servo.servokit.androidexample/.MainActivity`<br>`adb logcat -s ServokitNativeExample NativeServoView ExampleFixtureServer` | Debug APK builds. Manual launch shows the native proof UI with a `SurfaceView`, fixture shortcuts, load/reload/back/forward/focus/blur controls, and logcat/footer events such as `surfaceAttached`, `loadStatusChanged`, `urlChanged`, `historyChanged`, and `focusChanged`. | Requires the same Android SDK/NDK/Rust target/`cargo ndk` prerequisites as the React Native Android build. Keep this on an Android-enabled runner if automated. |

## Rust lockfile roots

The Rust validation commands intentionally keep using `--locked`. Checked-in
lockfiles now live next to the Cargo root that owns each graph:

- `crates/Cargo.lock` for `cargo test --manifest-path crates/Cargo.toml ...`
- `examples/desktop-winit/Cargo.lock` for the desktop `winit` example manifest
- `examples/desktop-gpui/Cargo.lock` for the desktop GPUI example manifest

There is no shared `examples/Cargo.toml` workspace or shared
`examples/Cargo.lock`. The temporary `stylo_derive` patch is scoped to the GPUI
example root only; core crates and non-GPUI examples do not use it. The
canonical Servo 0.3.0 baseline, patch locations, and lock/update workflow live
in [`rust-dependency-baseline.md`](./rust-dependency-baseline.md).

## Local fixture smoke recipe

For desktop manual smoke, start the shared fixture server once from the repository
root:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Then run a desktop example or Android example against
`http://127.0.0.1:8481/smoke/index.html` unless the app packages the fixtures
internally. For `examples/desktop-winit`, add `--smoke` before the URL when you
want the deterministic exit-code path instead of an open-ended interactive run.
The visible first-page marker is `Smoke fixtures ready`.

## Future check candidates

Add these checks only when the corresponding feature work has a real
fixture/API to validate:

- Mounted `ServoView.evaluateJavaScript(script): Promise<string>` should have a
  fixture assertion that the promise resolves with the expected serialized Servo
  `JSValue` JSON string and rejects with mapped Servo evaluation errors.
- Popup/new-window intent should add fixtures for `window.open` and
  `target="_blank"`, Servo `request_create_new` event emission, default
  deny/no-op, and host handling of a surfaced popup/navigation URL in the
  current app-owned UI.
- Areas such as custom schemes, secure web-content IPC, preload/user scripts,
  and GPU layer surfaces should gain matrix rows only after concrete APIs and
  fixtures exist. Publication/CI and release promotion remain separate from the
  proven local prebuilt package path.

## Documentation map

Keep tactical status in the active tracker. For durable docs or validation
context, read in this order:

1. [`../README.md`](../README.md), [`../ARCHITECTURE.md`](../ARCHITECTURE.md),
   [`servokit.md`](./servokit.md), and
   [`runtime-and-module-map.md`](./runtime-and-module-map.md) for the repo map,
   module boundaries, and ownership rules.
2. [`surface-modes.md`](./surface-modes.md) and
   [`rust-dependency-baseline.md`](./rust-dependency-baseline.md) for surface
   vocabulary, dependency/lockfile expectations, supported flows, and known
   feature limits.
3. This matrix for the cheapest command that proves the target area.
4. Platform example docs (`desktop-winit.md`, `android-build.md`,
   `../examples/desktop-gpui/README.md`,
   `../examples/android-native-example/README.md`, and
   `../examples/android-kotlin-browser/README.md`) before running manual smoke.

Do not treat manual-only smoke as skipped forever: record whether it was run,
what platform was used, and which fixture URL was loaded. If a host surface is
proof-only, preserve that language in follow-up docs and PRs.
