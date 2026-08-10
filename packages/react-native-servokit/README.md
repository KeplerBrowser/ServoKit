# react-native-servokit

Experimental React Native Fabric view backed by Servo on Android and
WKWebView/WebKit on iOS.

## Installation

```sh
npm install react-native-servokit
```

React Native autolinking discovers the Android library and iOS pod. For iOS,
install pods after adding the package:

```sh
npx pod-install ios
```

The New Architecture must be enabled. Consumer builds use the native artifacts
already carried by the package; they do not run Cargo or download native code.

## Validated compatibility

| Target | Experimentally validated baseline |
| --- | --- |
| React Native | React Native 0.85 with the New Architecture enabled |
| Android | minSdk 24, compileSdk 36, Java 17; `arm64-v8a` and `x86_64` |
| iOS | React Native 0.85 / iOS 15.1 deployment baseline; device arm64 and simulator arm64/x86_64 |

The peer dependency ranges permit other React and React Native versions, but
only the baseline above has been validated.

## Package contents

| Platform | Packaged native path | Engine and ownership |
| --- | --- | --- |
| Android | `android/libs/servokit-android-host-release.aar`, with `arm64-v8a` and `x86_64` native hosts | Servo-backed. Rust ServoKit owns browser/controller semantics; Kotlin owns the Fabric view and Android platform glue. |
| iOS | `ios/ServoView.mm` and `ios/ServoKitController.xcframework`, autolinked through CocoaPods | WKWebView/WebKit-backed. The XCFramework contains only the Servo-free portable Rust controller; Objective-C++ owns WKWebView and native platform objects. |

The API and native packaging remain experimental.

Servo-on-iOS is deferred. macOS remains an experimental source-side path and is
not packaged or supported by this npm package.

## Usage

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
      onLoadStatusChanged={(event) =>
        console.log(event.nativeEvent.status)
      }
    />
  );
}
```

`ServoViewHandle` exposes:

- `loadUrl(url: string)`
- `reload()`
- `goBack()`
- `goForward()`
- `focus()`
- `blur()`
- `evaluateJavaScript(script: string): Promise<string>`

Android evaluation follows Servo `WebView::evaluate_javascript`; iOS uses
`WKWebView.evaluateJavaScript`. Both return the package's tagged JSON-string
result shape, while failures retain engine-specific categories.

On both platforms, Rust owns command validation, request identity, pending
semantics, fallback policy, and response validation. Android resolves those
semantics through the Servo-backed host. On iOS, the portable Rust controller
emits effects that Objective-C++ applies to WKWebView; Objective-C++ owns native
delegate completions and timers, KVO/recycling, engine handles, effect
execution, and main-thread scheduling.

Navigation policy maps to Servo `WebViewDelegate::request_navigation` on
Android and `WKNavigationDelegate` on iOS. Dialog callbacks likewise preserve
the shared React Native request shape while each engine uses its native
completion path.

See the
[platform capability matrix](https://github.com/KeplerBrowser/ServoKit/blob/main/docs/host-control-capabilities.md)
for event and control availability.

## Development validation

From this package directory:

```sh
node scripts/validate-packed-consumer.mjs --ios-only
```

This packs the exact local tgz, installs it in a clean external React Native
consumer, and proves Release builds for device arm64, simulator arm64, and
simulator x86_64. It does not perform simulator runtime acceptance, publish to
npm, or promote a release.
