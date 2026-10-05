# React Native package guide for agents

This guide covers `packages/react-native-servokit`. The package is a thin
adapter: it maps one `<ServoView>` API to each platform's engine. Read
[React Native](../../docs/platforms/react-native.md) for the public API and
[ARCHITECTURE.md](../../ARCHITECTURE.md) for the principles.

## What belongs here

| Layer | Owns |
| --- | --- |
| TypeScript (`src/`) | Props, events, refs, and callback ergonomics. Builds command envelopes and sends them unchanged. |
| Android (`android/`) | The Fabric view, `SurfaceView` lifecycle, input and IME translation, native prompt UI, and frame scheduling above the shared Android module |
| iOS (`ios/ServoView.mm`) | The Fabric view, `WKWebView`, WebKit delegates, native completion handlers and timers, KVO, and main-thread work |
| macOS (`macos/`) | The AppKit view and input translation above the private desktop C boundary. Not shipped in the npm package. |

What does not belong here:

- Command names and payload validation beyond building the envelope. Rust's
  `ControllerCommand` owns them.
- Request identity and answer validation. The Rust controller owns them.
  Adapters may only guard against completing a native callback twice.
- Timers and made-up answers. A prompt nobody handles ends with the engine's
  own default. Known gap: the navigation timer in
  `NavigationPolicyCoordinator.ts`, the no-handler navigation answer in
  `ServoView.kt`, and the iOS timers still exist. Don't add more.
- Android host or JNI code. It belongs in `crates/servokit-host-android/android`.
- A detached TurboModule, or example app behavior.
- WebKit objects inside Rust.

## Changing the public API

- Lock exact names, provenance, non-goals, and acceptance evidence first, as
  described in the root [AGENTS.md](../../AGENTS.md#rules-for-changes).
- Props, events, and commands are declared once in the Codegen spec,
  `src/ServoViewNativeComponent.ts`. The mounted command is
  `sendControllerCommand`.
- Map each new event on every platform that supports it: `ServoViewEvents.kt`
  on Android, the `emit*` methods in `ios/ServoView.mm`, and
  `macos/ServoView.mm`.
- Update [React Native](../../docs/platforms/react-native.md) and
  [What works where](../../docs/reference/capabilities.md) in the same change.

## Native artifacts

- The Android AAR in `android/libs/` comes only from
  `:servokit-android-host:stageReactNativeServokitReleaseAar`. Never copy it in
  by hand. It is not committed.
- `ios/ServoKitController.xcframework` is built by
  `distribution/ios/build-controller-xcframework.sh`.
- The packed package must not contain the Rust workspace or the host Gradle
  project. `scripts/validate-packed-consumer.mjs` enforces this.

## Validate

```sh
bun run --cwd packages/react-native-servokit typecheck
bun run --cwd packages/react-native-servokit prepare
bun test packages/react-native-servokit/src/__tests__
```

TypeScript tests use `bun:test` and live in `src/__tests__/*.test.ts`. Android
unit tests, the packed-package iOS build, and the Maestro end-to-end flows are
in [Testing and validation](../../docs/reference/testing.md). The end-to-end
flows run against the package's own test app in `example/`.
