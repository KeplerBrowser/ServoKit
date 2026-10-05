# React Native

`react-native-servokit` gives React Native apps a `<ServoView>` component: a
web view backed by Servo on Android and by WebKit on iOS, with one API on both.

## Platform support

| Platform | Engine | Status |
| --- | --- | --- |
| Android | Servo, through the ServoKit Rust runtime | Experimental |
| iOS | `WKWebView`, driven by the ServoKit Rust controller | Experimental |
| macOS | Servo, through a private C boundary | Prototype, built from source only, not supported |
| Windows | None | Not started |

Why WebKit on iOS? Apple requires WebKit for most iOS apps (the EU and Japan
allow other engines only with special entitlements), and Servo does not
support iOS yet. ServoKit keeps the same component, commands, and Rust logic
on iOS, and drives `WKWebView` underneath. See Open Web Advocacy's
[Apple Browser Ban](https://open-web-advocacy.org/apple-browser-ban/) summary
for background.

## Install

The package is not on npm yet. Build it from this repository and install the
packed archive, or depend on `packages/react-native-servokit` from a
workspace. Once installed:

- React Native autolinking finds the Android library and the iOS pod. On iOS,
  run `npx pod-install ios`.
- The New Architecture must be on.
- Your app's build does not run Cargo or download native code. The package
  already contains the Android AAR and the iOS controller binary.

Tested with:

| Target | Version |
| --- | --- |
| React Native | 0.85, New Architecture |
| Android | minSdk 24, compileSdk 36, Java 17; `arm64-v8a` and `x86_64` |
| iOS | iOS 15.1 or later; device arm64, simulator arm64 and x86_64 |

Other versions may work, but only these are tested.

## Usage

```tsx
import { useRef } from 'react';
import { ServoView, type ServoViewHandle } from 'react-native-servokit';

export function Browser() {
  const servo = useRef<ServoViewHandle>(null);

  return (
    <ServoView
      ref={servo}
      style={{ flex: 1 }}
      url="https://servo.org"
      onUrlChanged={(e) => console.log('url', e.nativeEvent.url)}
      onPageTitleChanged={(e) => console.log('title', e.nativeEvent.title)}
      onLoadStatusChanged={(e) => console.log('load', e.nativeEvent.status)}
      onShouldStartLoadWithRequest={async ({ url }) => !url.includes('blocked')}
    />
  );
}
```

## Props

`url` (required) is the page to load. All other props are optional.

### Events

Event handlers receive `{ nativeEvent }`.

| Prop | `nativeEvent` | Android | iOS |
| --- | --- | :---: | :---: |
| `onUrlChanged` | `{ url }` | ✓ | ✓ |
| `onPageTitleChanged` | `{ title }` (may be null) | ✓ | ✓ |
| `onLoadStatusChanged` | `{ status }`: `'Started'`, `'HeadParsed'`, or `'Complete'` | ✓ | ✓ |
| `onHistoryChanged` | `{ entries, current, canGoBack, canGoForward }` | ✓ | ✓ |
| `onFocusChanged` | `{ isFocused }` | ✓ | ✓ |
| `onStatusTextChanged` | `{ status }` | ✓ | |
| `onCursorChanged` | `{ cursor }`, a CSS cursor name | ✓ | |
| `onFullscreenChanged` | `{ isFullscreen }` | ✓ | |
| `onClosed` | `{}` | ✓ | |
| `onCrashed` | `{ reason, backtrace }` | ✓ | |
| `onError` | `{ code, message }` | ✓ | |
| `onCreateNewWebViewRequested` | `{ parentWebViewId, parentUrl, targetUrl, windowFeatures, policy }` | ✓ | |

Nullable text fields arrive as `null` on Android and as `""` on iOS.

### Callbacks

These receive plain objects and let you answer the page.

| Prop | Purpose | Android | iOS |
| --- | --- | :---: | :---: |
| `onShouldStartLoadWithRequest({ url })` | Return `true` to allow a navigation, `false` to block it | ✓ | ✓ |
| `onJavaScriptDialog(request)` | Show your own UI for `alert`, `confirm`, and `prompt` | ✓ | ✓ |
| `onJavaScriptDialogDismissed({ dialogId })` | A pending dialog closed: the engine dismissed it, or on iOS, your answer or the 60-second fallback settled it | ✓ | ✓ |
| `onBeforeShowContextMenu({ element, servoItems, show })` | Add your own items to the long-press menu | ✓ | |
| `onContextMenuItemSelected({ item, element })` | A context-menu item was picked | ✓ | |

## Methods

Get a `ServoViewHandle` with a ref:

| Method | What it does |
| --- | --- |
| `loadUrl(url)` | Load a URL |
| `reload()` | Reload the page |
| `goBack()` / `goForward()` | Move through history |
| `focus()` / `blur()` | Give or take keyboard focus |
| `evaluateJavaScript(script)` | Run JavaScript in the page; returns `Promise<string>` |

Each method sends one JSON command to the native view, which hands it to the
Rust controller. The commands and their rules are described in
[Commands, events, and page prompts](../concepts/controller.md).

## Navigation policy

`onShouldStartLoadWithRequest` maps to Servo's `request_navigation` on Android
and to `WKNavigationDelegate` on iOS. It may return a boolean or a promise.

The page is never left waiting. The navigation is **allowed** when:

- you don't pass the callback;
- the callback throws, rejects, or returns something other than a boolean;
- the callback takes longer than 5 seconds;
- the view unmounts while a decision is pending.

The request contains only `{ url }`. WebKit-specific fields are not exposed.

On Android, this timeout runs in the package's JavaScript. On iOS, the Rust
controller sets it, and the native adapter runs the timer.

## JavaScript dialogs

With `onJavaScriptDialog`, you get a request and decide what to show:

```tsx
onJavaScriptDialog={(request) => {
  // request: { dialogId, kind: 'alert' | 'confirm' | 'prompt', message, defaultValue }
  if (request.kind === 'prompt') {
    request.confirm('my answer'); // omit the value to use defaultValue
  } else {
    request.confirm();
  }
  // or request.dismiss()
}}
```

Without the callback:

- **Android** shows a native Android dialog.
- **iOS** shows nothing: alerts complete, and `confirm` and `prompt` are
  cancelled.

If your callback throws, the dialog is dismissed. If the view unmounts, pending
dialogs are dismissed. On iOS, if 60 seconds pass without an answer, the Rust
controller applies the same defaults as above. On Android there is no such
timeout yet, so always call `confirm` or `dismiss`.

## Evaluating JavaScript

```ts
const json = await servo.current?.evaluateJavaScript('document.title');
// '{"type":"string","value":"Example title"}'
```

The promise resolves with tagged JSON: every value carries its type, such as
`{"type":"string","value":"text"}` or `{"type":"null"}`. Arrays and objects
contain tagged members.

On Android (Servo), the promise rejects with one of Servo's error categories:
`DocumentNotFound`, `CompilationFailure`, `EvaluationFailure`,
`InternalError`, `WebViewNotReady`, or `SerializationError`.

On iOS (WebKit), results follow WebKit's conversion of `string`, `number`,
`boolean`, `null`, arrays, and objects. Failures are `EvaluationFailure`,
`InternalError`, `WebViewNotReady`, or `SerializationError`.

On both, unmounting the view rejects pending calls with `LifecycleError`.

This runs a script you choose, on demand. It is not a preload or user-script
system, not `injectJavaScript` from `react-native-webview`, not a way for pages
to message your app, and does not work without a mounted view.

## Context menus (Android)

Long-pressing a page shows a native menu with Servo's actions, such as copy or
open link. `onBeforeShowContextMenu` lets you add items:

```tsx
onBeforeShowContextMenu={({ element, show }) => {
  if (element.isLink) {
    show([{ label: 'Share link', action: 'share-link' }]);
  } else {
    show([]);
  }
}}
onContextMenuItemSelected={({ item, element }) => {
  if (item.type !== 'separator' && item.action === 'share-link') {
    share(element.linkUrl);
  }
}}
```

- Servo's items always come first, and your items are added after them. An
  item whose `action` matches one of Servo's items is ignored.
- If you don't call `show`, or your callback rejects, the menu shows Servo's
  items only.
- Picking one of your items closes the menu. Picking a Servo item runs its
  action. Both call `onContextMenuItemSelected`.

`element` tells you what was pressed: `isLink`, `isImage`, `isEditableText`,
`hasSelection`, `linkUrl`, `imageUrl`, and `contextType`.

## Popups and new windows (Android)

The adapter never opens new windows. When a page asks for one,
`onCreateNewWebViewRequested` reports that a popup from `parentUrl` was
blocked. Servo 0.6 doesn't expose the popup's target URL, so `targetUrl` and
`windowFeatures` are always null for now. iOS does not fire this event.

## Other Android behavior

- Select menus, date, time, and color pickers, file pickers, and permission
  prompts use native Android UI. There are no React Native hooks for these
  yet.
- The back button first closes the keyboard, then goes back in page history
  when the focused page can go back.
- Cut, copy, paste, and select all use Android's real clipboard.

See [What works where](../reference/capabilities.md) for every capability on
every platform.

## How the package is built

| Platform | What ships | Who owns what |
| --- | --- | --- |
| Android | `android/libs/servokit-android-host-release.aar` (`arm64-v8a`, `x86_64`) | Rust owns browser logic. Kotlin owns the view and Android integration. |
| iOS | `ios/ServoView.mm` and `ios/ServoKitController.xcframework` | The XCFramework holds only the Servo-free Rust controller. Objective-C++ owns `WKWebView` and its native objects. |

To check the exact package builds on iOS, run this from
`packages/react-native-servokit`:

```sh
node scripts/validate-packed-consumer.mjs --ios-only
```

More checks are in [Testing and validation](../reference/testing.md).

The macOS adapter (`macos/ServoView.mm`) is not in the npm package. Local
macOS apps use the repository-only `macos/ServokitMacOS.podspec`, either with
a prebuilt `ServoKit.xcframework` and `SERVOKIT_BUILD_FROM_SOURCE=1`, or with a
local `ServoKitMacOSBinary` pod. This keeps Servo out of the iOS pod.
