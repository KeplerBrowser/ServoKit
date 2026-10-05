# Get started

This page shows how to build and run an ExplorerKit example. Pick the path that
matches what you want to see:

| Path | Engine | You need | First build |
| --- | --- | --- | --- |
| [iOS simulator](#ios-simulator-react-native) | WebKit | A Mac with Xcode | Minutes |
| [Desktop window](#desktop-window-rust) | Servo | Rust and Servo's build tools | Long (compiles Servo) |
| [Android](#android-react-native-or-kotlin) | Servo | Rust, the Android SDK, and two NDKs | Long (compiles Servo for Android) |

All commands run from the repository root.

> [!NOTE]
> ExplorerKit is experimental and nothing is published yet, so you build
> everything from this repository.

## Set up the repository

You need [Bun](https://bun.sh) and Node.js 22.11 or newer.

```sh
git clone https://github.com/KeplerBrowser/ServoKit.git
cd ServoKit
bun install
```

`bun install` also turns on the repository's commit-message check.

## iOS simulator (React Native)

This is the quickest path. On iOS, `<ExplorerView>` uses Apple's WebKit, and the
ExplorerKit Rust controller ships as a prebuilt binary in the repository, so you
don't need Rust.

You need a Mac with Xcode, an iOS simulator, and Ruby with Bundler for
CocoaPods.

```sh
(cd examples/react-native-app && bundle install)
bun run --cwd examples/react-native-app pods:ios
bun run --cwd examples/react-native-app ios
```

The app opens a small browser. Type a URL in the address bar at the bottom, or
use the back, forward, home, and reload buttons.

## Desktop window (Rust)

This path shows Servo itself drawing a page in a native window. It is tested
on macOS.

You need:

- Rust, installed with [rustup](https://rustup.rs).
- Servo's system build dependencies. Follow the
  [Servo book](https://book.servo.org/building/building.html) for your OS.
- Python 3, for the local test pages.

Start the local test pages in one terminal:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Run the example in another terminal:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml
```

A window opens and shows **Smoke fixtures ready**. Without a URL, the example
loads `http://127.0.0.1:8481/smoke/index.html`. You can pass any URL instead:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- https://servo.org
```

Next: [Desktop guide](platforms/desktop.md).

## Android (React Native or Kotlin)

On Android, ExplorerKit runs Servo. The first build compiles Servo for Android,
which takes a while. The React Native example uses the result packaged as an
Android library (an AAR); the Kotlin examples build the Rust library directly
through the Gradle module.

You need:

- JDK 17 and the Android SDK with API level 36.
- Android NDK `28.2.13676358` to build the Servo host library.
- Android NDK `27.1.12297006`, which the example apps use for their own
  native code.
- Rust with both Android targets and `cargo-ndk`:

  ```sh
  rustup target add aarch64-linux-android x86_64-linux-android
  cargo install cargo-ndk
  ```

- `uv`, or Python 3.11 or newer, for Servo's code generation.
- An arm64 Android device or emulator.

Build the Servo host library and copy it into the React Native package:

```sh
examples/react-native-app/android/gradlew \
  -p examples/react-native-app/android \
  :explorerkit-host-android:stageReactNativeExplorerKitReleaseAar
```

Then run the React Native example. Start Metro in one terminal:

```sh
bun run --cwd examples/react-native-app start
```

Install and launch the app in another terminal:

```sh
bun run --cwd examples/react-native-app android
```

To try ExplorerKit from Kotlin without React Native, build the
[bare Android example](../examples/android-native-example/README.md):

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-native-example \
  :app:assembleDebug
```

Next: [Android guide](platforms/android.md) or
[React Native guide](platforms/react-native.md).

## All examples

| Example | What it shows |
| --- | --- |
| [`examples/react-native-app`](../examples/react-native-app/README.md) | React Native browser app for Android (Servo) and iOS (WebKit) |
| [`examples/react-native-macos-app`](../examples/react-native-macos-app) | React Native on macOS with Servo. A prototype, not supported. |
| [`examples/android-native-example`](../examples/android-native-example/README.md) | Kotlin app without React Native, with native UI for every page prompt |
| [`examples/android-kotlin-browser`](../examples/android-kotlin-browser/README.md) | The same Kotlin app, built under a different name |
| [`examples/desktop-winit`](../examples/desktop-winit) | Rust app that owns a `winit` window |
| [`examples/desktop-gpui`](../examples/desktop-gpui/README.md) | Rust GPUI app with Servo in one part of its layout (macOS) |
| [`crates/explorerkit/examples/multiple-native-views`](../crates/explorerkit/examples/multiple-native-views) | Two independent pages side by side (macOS) |
| [`examples/fixtures`](../examples/fixtures/README.md) | Shared test pages used by every example |

`examples/react-native-example-app` holds the browser UI shared by the React
Native examples. `packages/react-native-explorerkit/example` is the package's
own test app, used by its native unit tests and end-to-end tests.

## Something broke?

Check [Testing and validation](reference/testing.md) for the exact checks
each example should pass, then
[open a bug report](https://github.com/KeplerBrowser/ServoKit/issues/new/choose).
