# React Native macOS prototype

A React Native macOS app that renders `<ServoView>` with Servo, through the
package's AppKit adapter and ServoKit's private desktop C boundary.

> [!WARNING]
> This is a prototype. It is not supported, not packaged, and has no
> distribution plan.

It shares its browser UI with the Android and iOS example through
[`examples/react-native-example-app`](../react-native-example-app).

> [!NOTE]
> `ServoView.tsx` currently throws for any platform other than Android and
> iOS, and React Native macOS reports its platform as `macos`. Confirm on a
> Mac whether this example still renders before relying on it.

## Run

You need a Mac prepared to build Servo, plus Xcode and CocoaPods, and a clean
checkout (the build checks its source identity). The framework build needs a
lot of temporary disk space.

From the repository root:

```sh
bun install
SERVOKIT_BUILD_FROM_SOURCE=1 SERVOKIT_SOURCE_DIR="$PWD" \
  bun run --cwd examples/react-native-macos-app pods:macos
bun run --cwd examples/react-native-macos-app start   # Metro, in its own terminal
bun run --cwd examples/react-native-macos-app macos
```

Checks for this path are in
[Testing and validation](../../docs/reference/testing.md#react-native-on-macos-prototype).
