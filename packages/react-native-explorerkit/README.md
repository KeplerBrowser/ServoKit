# react-native-explorerkit

A React Native web view backed by [Servo](https://servo.org) on Android and
by WebKit on iOS, with one API on both. Part of
[ExplorerKit](https://github.com/KeplerBrowser/ServoKit).

> [!WARNING]
> Experimental. The API and packaging may change. The package is not published
> to npm yet.

## Install

The package is not on npm yet. Build the Android AAR in an ExplorerKit checkout
(see [Get started](https://github.com/KeplerBrowser/ServoKit/blob/main/docs/getting-started.md#android-react-native-or-kotlin)),
pack the package, and install the archive in your app:

```sh
# in the ExplorerKit checkout
cd packages/react-native-explorerkit && npm pack

# in your app
npm install /path/to/react-native-explorerkit-0.1.0.tgz
npx pod-install ios
```

The New Architecture must be on. Autolinking finds the Android library and the
iOS pod. Your app's build uses the native binaries inside the package: it does
not run Cargo or download anything.

| Target | Tested with |
| --- | --- |
| React Native | 0.85, New Architecture |
| Android | minSdk 24, compileSdk 36, Java 17; `arm64-v8a` and `x86_64` |
| iOS | iOS 15.1 or later; device arm64, simulator arm64 and x86_64 |

The peer dependency ranges allow other versions, but only these are tested.

## Usage

```tsx
import { useRef } from 'react';
import { ExplorerView, type ExplorerViewHandle } from 'react-native-explorerkit';

export function Browser() {
  const servo = useRef<ExplorerViewHandle>(null);

  return (
    <ExplorerView
      ref={servo}
      style={{ flex: 1 }}
      url="https://servo.org"
      onShouldStartLoadWithRequest={async ({ url }) => !url.includes('blocked')}
      onUrlChanged={(e) => console.log(e.nativeEvent.url)}
      onPageTitleChanged={(e) => console.log(e.nativeEvent.title)}
      onLoadStatusChanged={(e) => console.log(e.nativeEvent.status)}
    />
  );
}
```

`ExplorerViewHandle` methods: `loadUrl(url)`, `reload()`, `goBack()`,
`goForward()`, `focus()`, `blur()`, and `evaluateJavaScript(script)`, which
returns a `Promise<string>` of tagged JSON such as
`{"type":"string","value":"Example"}`.

Navigation is allowed if `onShouldStartLoadWithRequest` is missing, fails, or
takes longer than 5 seconds. Without `onJavaScriptDialog`, Android shows
native dialogs, and iOS completes alerts and cancels `confirm` and `prompt`.

## What's inside

| Platform | Ships | Engine |
| --- | --- | --- |
| Android | `android/libs/explorerkit-host-android-release.aar` | Servo. Rust owns the browser logic; Kotlin owns the view and Android integration. |
| iOS | `ios/ExplorerView.mm`, `ios/ServoKitController.xcframework` | WebKit. The XCFramework holds only the Servo-free Rust controller; Objective-C++ owns `WKWebView`. |

Apple requires WebKit for most iOS apps, and Servo doesn't support iOS yet, so
iOS uses WebKit. macOS is a prototype built from source and is not part of
this package.

## Learn more

- [Full API guide](https://github.com/KeplerBrowser/ServoKit/blob/main/docs/platforms/react-native.md):
  every prop, event, and method, with platform differences.
- [What works where](https://github.com/KeplerBrowser/ServoKit/blob/main/docs/reference/capabilities.md):
  capability support on each platform.
- [How ExplorerKit works](https://github.com/KeplerBrowser/ServoKit/blob/main/ARCHITECTURE.md).

## Development

From this folder, check that the packed package builds in a clean iOS app
(device arm64, simulator arm64 and x86_64):

```sh
node scripts/validate-packed-consumer.mjs --ios-only
```

This builds Release only. It does not run the app or publish anything. Other
checks are in
[Testing and validation](https://github.com/KeplerBrowser/ServoKit/blob/main/docs/reference/testing.md).

## License

MIT
