# react-native-servokit example

React Native example app for the local `react-native-servokit` package. Android
tests the Servo-backed adapter against the monorepo copy of ServoKit and Servo;
iOS tests the WKWebView/WebKit-backed baseline adapter for the same `ServoView`
component.

## Run locally

Install dependencies from the repository root:

```sh
bun install
```

Start Metro:

```sh
bun run --cwd examples/react-native-app start
```

Run the Android app:

```sh
bun run --cwd examples/react-native-app android
```

Run the iOS app after installing Ruby gems and pods:

```sh
cd examples/react-native-app && bundle install && cd ../..
bun run --cwd examples/react-native-app pods:ios
bun run --cwd examples/react-native-app ios
```

Build the Android app without launching it:

```sh
bun run --cwd examples/react-native-app build:android
```

The app is intentionally a thin native host for the shared public browser shell.
Enter a URL in the bottom address field or use the back, forward, home, and
reload controls to exercise the embedded `ServoView`.

## Native code

Open `examples/react-native-app/android` in Android Studio when you need IDE
support for the Android app or included package sources. Open
`examples/react-native-app/ios/ServoExample.xcodeproj` or the generated
workspace after `pod install` when you need Xcode support for the iOS WKWebView
baseline.
