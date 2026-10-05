# Kotlin browser example

An Android app written in Kotlin, without React Native, that embeds Servo
through ExplorerKit's shared Android layer.

> [!NOTE]
> This folder has no Kotlin code of its own. Its Gradle build compiles the
> sources of [`android-native-example`](../android-native-example/README.md)
> under a different app name and application ID
> (`com.kepler.explorerkit.androidkotlinbrowser`). Both folders produce the same
> app. To change the app, edit `android-native-example`.

It is an example, not a Kotlin SDK or a published library.

## What it shows

```text
Kotlin app (examples/android-native-example sources)
  → ExplorerViewBinding, ExplorerSurfaceLifecycleCoordinator, ExplorerHostEvent
    (shared Kotlin layer in crates/explorerkit-host-android/android)
  → explorerkit-host-android (Rust)
  → explorerkit-embedder (Rust)
  → Servo
```

The React Native adapter uses the same shared Kotlin layer. Only the app on
top differs.

## Build

You need the Android setup from
[Get started](../../docs/getting-started.md#android-react-native-or-kotlin):
JDK 17, the Android SDK, NDK `28.2.13676358` and `27.1.12297006`, both Rust
Android targets, and `cargo-ndk`.

From the repository root:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-kotlin-browser \
  :app:assembleDebug
```

The build includes the `:explorerkit-host-android` module, which compiles the
Rust library with `cargo ndk`. The app has no React Native dependencies and
ships `arm64-v8a` only.

## Run

On an arm64 device or emulator:

```sh
adb install -r examples/android-kotlin-browser/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.kepler.explorerkit.androidkotlinbrowser/com.kepler.explorerkit.androidexample.MainActivity
adb logcat -s ExplorerKitNativeExample NativeExplorerView ExampleFixtureServer
```

What to expect, and a step-by-step check of every prompt type, are in the
[`android-native-example` README](../android-native-example/README.md#run).
