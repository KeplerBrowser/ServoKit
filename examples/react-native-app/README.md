# React Native example

A small browser app built with `react-native-servokit`. On Android it renders
with Servo. On iOS it renders with WebKit, through the same `<ServoView>`.

The browser UI lives in
[`examples/react-native-example-app`](../react-native-example-app), and is
shared with the macOS prototype. This folder holds the Android and iOS app
projects.

## Run it

From the repository root:

```sh
bun install
bun run --cwd examples/react-native-app start   # Metro, in its own terminal
```

**iOS** (no Rust needed):

```sh
(cd examples/react-native-app && bundle install)
bun run --cwd examples/react-native-app pods:ios
bun run --cwd examples/react-native-app ios
```

**Android** needs the Servo host library first. See
[Get started](../../docs/getting-started.md#android-react-native-or-kotlin)
for the toolchain, then:

```sh
examples/react-native-app/android/gradlew \
  -p examples/react-native-app/android \
  :servokit-android-host:stageReactNativeServokitReleaseAar
bun run --cwd examples/react-native-app android
```

To build the Android app without launching it:

```sh
bun run --cwd examples/react-native-app build:android
```

## Use it

Type a URL in the address bar at the bottom, or use the back, forward, home,
and reload buttons.

## Open in an IDE

- Android: open `examples/react-native-app/android` in Android Studio.
- iOS: open the workspace in `examples/react-native-app/ios` that `pod install`
  creates, or `ServoExample.xcodeproj`.
