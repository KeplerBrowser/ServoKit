# ServoKit Kotlin Android browser example

This is an experimental Kotlin Android app-facing host example for ServoKit. It
embeds Servo through the crate-owned repo-local `crates/servokit-host-android/android`
Gradle module and does **not** load the React Native runtime.

It is not a stable Kotlin SDK, public AAR, or distribution promise. The Android
host module remains a crate-owned repo-local boundary while ServoKit's Android
packaging story is still experimental.

## What this proves

ServoKit's Android shape mirrors a facade/host-backend split:

```text
Kotlin Android browser example
  -> shared Android ServoKit host/control coordinator
      -> crates/servokit-host-android/android
          -> servokit-host-android
              -> servokit-embedder
                  -> Servo

react-native-servokit
  -> Fabric ServoView + RN bridge ergonomics
      -> shared Android ServoKit host/control coordinator
          -> crates/servokit-host-android/android
              -> servokit-host-android
                  -> servokit-embedder
                      -> Servo
```

React Native owns Fabric props/events, ref commands, optional JavaScript
callbacks, and app-specific bridge ergonomics. The default Android host/control
path is shared below it: `ServoViewBinding`, `ServoSurfaceLifecycleCoordinator`,
the typed `ServoHostEvent` bridge, and picker value helpers live in
`crates/servokit-host-android/android` and are used by both the RN Android adapter and
this Kotlin example.

The app exposes browser chrome rather than a low-level fixture harness: a URL
field, load/reload/back/forward/focus/blur controls, shortcut buttons for the
packaged smoke fixtures, a Servo `SurfaceView`, and an event footer. The
fixture shortcuts demonstrate the current Android control matrix: navigation
policy, JavaScript dialogs, text IME, select controls, non-text pickers, file
picker, permissions, and context menu fallback.

The app currently reuses the repository's proof-only native Android view/source
wrapper while the shared coordinator extraction deepens. That wrapper now routes
browser, surface, and embedder-control commands through the shared
`ServoViewBinding` and `ServoSurfaceLifecycleCoordinator`; follow-up work can
move more Android fallback UI presentation out of the example layer if the
project chooses to harden a Kotlin API.

## Build

Prerequisites match the other Android examples:

- Android SDK with NDK `27.1.12297006`
- Rust target `aarch64-linux-android`
- `cargo ndk`

From the repository root:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-kotlin-browser \
  :app:assembleDebug
```

The build includes `:servokit-android-host`, which builds/packages the existing
`servokit-host-android` native library for `arm64-v8a`. There are no React
Native dependencies in this Gradle project.

## Run manually

Install and launch on an arm64 Android device or emulator:

```sh
adb install -r examples/android-kotlin-browser/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.servo.servokit.androidkotlinbrowser/org.servo.servokit.androidexample.MainActivity
adb logcat -s ServokitNativeExample NativeServoView ExampleFixtureServer
```

Expected smoke:

- The activity shows ServoKit Android host browser chrome around a native
  `SurfaceView`.
- The default URL is `http://127.0.0.1:8481/smoke/index.html`, served from the
  app's packaged copy of `examples/fixtures`.
- The smoke shortcut row can load smoke/title/history/reload/form/error/
  policy pages.
- The embedder-control shortcut row can load dialog, IME/text input, select,
  picker, file-input, permission, and context-menu fixtures.
- Control prompts use Android UI and resolve through the shared ServoKit Android
  host/control path, not through React Native JavaScript.

If no device/emulator is attached, record manual smoke as not run.
