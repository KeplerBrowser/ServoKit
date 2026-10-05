# Bare Android example

An Android app that embeds Servo in a plain `SurfaceView`, without React
Native. It has native Android UI for every kind of page prompt, so it doubles
as a hands-on test app for ServoKit's Android host.

The Kotlin classes here are example code, not a Kotlin SDK or a reusable
library. [`android-kotlin-browser`](../android-kotlin-browser/README.md)
builds this same code under a different name.

## What it shows

- A native Android app can embed Servo without loading React Native or any
  React Native classes.
- The app depends on the shared `:servokit-android-host` Gradle module, which
  builds the Rust library.
- It attaches a `Surface` through `ServoViewBinding` and
  `ServoSurfaceLifecycleCoordinator`, loads a URL, runs updates from
  `Choreographer`, and reads ServoKit's typed events.
- Its toolbar drives load, reload, back, forward, focus, and blur.
- It answers every prompt type through the same `ServoViewBinding` calls the
  React Native adapter uses: navigation policy, JavaScript dialogs, text input
  and IME, `<select>`, date, time, and color pickers, file pickers,
  permissions, and context menus.

## Build

You need the Android setup from
[Get started](../../docs/getting-started.md#android-react-native-or-kotlin):
JDK 17, the Android SDK, NDK `28.2.13676358` and `27.1.12297006`, both Rust
Android targets, and `cargo-ndk`.

From the repository root:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-native-example \
  :app:assembleDebug
```

The `:servokit-android-host` module runs `cargo ndk` on
`crates/servokit-host-android` before the app is packaged. The app ships
`arm64-v8a` only.

## Run

On an arm64 device or emulator:

```sh
adb install -r examples/android-native-example/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.servo.servokit.androidexample/.MainActivity
adb logcat -s ServokitNativeExample NativeServoView ExampleFixtureServer
```

You should see a header, an address field, reload, back, forward, focus, and
blur buttons, two rows of test page shortcuts, the Servo page, and an event
footer. The app opens `http://127.0.0.1:8481/smoke/index.html`, served from a
copy of [`examples/fixtures`](../fixtures/README.md) packaged in the app. The
same server provides the prompt test pages under `/controls/`.

The footer and logcat show ServoKit events as you browse, such as
`surfaceAttached`, `loadStatusChanged`, `urlChanged`, `historyChanged`,
`focusChanged`, `navigationRequested`, `simpleDialogRequested`,
`inputMethodRequested`, `selectElementRequested`, `permissionRequested`, and
`contextMenuRequested`.

To open a different URL:

```sh
adb shell am start \
  -n org.servo.servokit.androidexample/.MainActivity \
  --es org.servo.servokit.androidexample.INITIAL_URL https://servo.org/
```

## Manual checks

Each check names the shortcut to tap, what to do, and what you should see.

### Navigation policy

1. Tap **Policy**.
2. Tap **Allowed same-origin link**. The footer shows `navigationRequested`
   then `policy=allow`, and the page moves to
   `history-start.html?from=policy-allowed`.
3. Go back, then tap **Denied HTTP(S) link** and **Denied custom scheme**.
   Each shows `policy=deny`, and the policy page stays.

### JavaScript dialogs

1. Tap **Dialogs**.
2. Tap **Trigger alert**, **Trigger confirm**, and **Trigger prompt**. An
   Android dialog appears for each. Your answer goes through
   `ServoViewBinding.resolveSimpleDialog`, the page shows the result, and the
   footer shows `simpleDialogRequested` and `simpleDialogDismissed`.

### Text input and IME

1. Tap **Form**, then tap the text and email inputs. The footer shows
   `inputMethodRequested` and the soft keyboard opens. Typing, backspace, and
   delete edit the field. Enter goes through
   `ServoViewBinding.dispatchKeyboardKey`.
2. Tap the textarea. Multi-line input works, including new lines where the
   keyboard offers them.
3. Close the keyboard with Back, or tap **Blur**. The footer shows
   `inputMethodDismissed`.
4. Repeat on **Text input** (`controls/ime-form.html`). This app commits to
   text inputs and textareas only; `contenteditable` and fields lower on the
   page are extra checks.

### Select menus

1. Tap **Select**, then **Basic single select**. An Android dialog lists the
   options. OK updates the page's value and title through
   `ServoViewBinding.resolveSelectElement`.
2. Try **Select with optgroups** and **Disabled option handling**. Group
   labels show as headers, and disabled options can't be picked.
3. Try **Multiple select**. The dialog uses checkboxes and commits every
   checked option.
4. Change a choice, then tap **Cancel** or Back. The page value stays the same.
   There is no separate "dismiss select" call yet, so cancel resubmits the
   original selection.

### Date, time, and color pickers

1. Tap **Pickers**, then the **Date** field. The footer shows an
   `inputMethodRequested` event with `type=date`, an Android date picker
   opens, and OK
   commits a `yyyy-MM-dd` value through
   `ServoViewBinding.dispatchImeComposition`. Cancel or Back changes nothing.
2. **Time** commits `HH:mm`.
3. **Datetime local** opens a date picker, then a time picker, and commits
   `yyyy-MM-ddTHH:mm`. Cancelling either step commits nothing.
4. **Color** uses a small RGB slider dialog and commits `#rrggbb`. **Month**
   commits `yyyy-MM`. **Week** picks a day and commits its ISO week as
   `yyyy-Www`.
5. **Range**, **checkbox**, and **radio** are handled by the page itself, and
   **number** uses the normal keyboard. None of them should crash the app.
   File inputs are covered below.

### Context menus

1. Tap **Context**, then long-press the **Servo** link (or right-click with a
   mouse). The footer shows `contextMenuRequested`, and a dialog shows the
   link details and Servo's actions. Picking an action goes through
   `ServoViewBinding.resolveContextMenu`.
2. Long-press **Generic target**. The dialog reports a page target. Cancel or
   Back goes through `ServoViewBinding.dismissContextMenu`.
3. Extra checks: long-press the **Image** target, long-press inside the
   **Input** or **Textarea**, and select text in the paragraph first. The
   actions offered depend on what Servo reports.

### File pickers

1. Tap **File**, then **Single file**, and pick a small document. The
   selected `content://` item is copied into the app's cache, its path goes
   to `ServoViewBinding.resolveFilePicker`, and the page shows the file name.
2. Open it again and cancel. The value stays the same, and the footer shows
   `reason=cancelled` through `ServoViewBinding.dismissFilePicker`.
3. Tap **Multiple text/markdown files** and pick several, if the document
   provider allows multiple selection. All files are copied and listed. A
   provider that ignores multiple selection is a provider limit, not a
   ServoKit bug.
4. Tap **Image files** and **Capture-hinted image**. The picker is filtered by
   MIME type (for example `image/*`). This app does not prompt for the camera.
   Extension filters are converted to MIME types where Android knows them, so
   some providers may not enforce unusual extensions.

### Permissions

1. Tap **Permissions**, then **Request notifications**. A dialog shows the
   origin and `notifications`. On Android 13 or newer, **Allow** may also show
   the system notification prompt. The answer goes through
   `ServoViewBinding.resolvePermission`. **Deny**, Back, or closing the
   dialog answers no.
2. **Request geolocation** asks for Android's location permission when
   needed, and allows only if Android grants it. The page may still report an
   error if the device has no location provider.
3. **Request camera** and **Request microphone** ask for `CAMERA` and
   `RECORD_AUDIO` first. If Servo has no capture backend on the device, the
   page should show an error, not crash.
4. **Query permission states** shows the page's own results. The app answers
   only `permissionRequested` events and invents no query results.

About permissions in this app:

- The manifest declares `ACCESS_FINE_LOCATION`, `ACCESS_COARSE_LOCATION`,
  `CAMERA`, `RECORD_AUDIO`, and `POST_NOTIFICATIONS`. Camera and microphone
  hardware are optional, so the app installs on emulators.
- Android permissions are requested for `geolocation`, `camera`,
  `microphone`, `notifications`, and the aliases `location`,
  `video-capture`, and `audio-capture`. Other permission kinds still show the
  dialog but request no Android permission.
- An Android grant is needed but may not be enough: location providers,
  capture devices, notification delivery, and secure-context rules can still
  make the page report an error.
