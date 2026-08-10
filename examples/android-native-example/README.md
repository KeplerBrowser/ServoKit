# Native Android ServoKit proof app

This example is an experimental proof app for ServoKit's Android host path. It
embeds Servo content in a native Android `SurfaceView` without React Native,
consumes the shared crate-owned `crates/servokit-host-android/android` Gradle
library, exposes proof-app navigation controls, forwards basic touch, text IME
input, select control resolution, file input fallback, permission prompt fallback, and
context-menu fallback for fixture interaction, and logs the shared Servokit
URL/load/history/focus/navigation/dialog/input-method/select/file-picker/
permission/context-menu event stream.

It is **not** a stable Kotlin SDK, reusable Android package, or public Android
API contract. The Kotlin classes in this example are proof-app glue only.

## What it proves

- A native Android app can create a ServoKit-backed Android view without loading
  the React Native runtime or depending on React Native UI classes.
- The app depends on the reusable `:servokit-android-host` Gradle module, which
  builds/packages the existing `servokit-host-android` native library for
  `arm64-v8a`.
- The app attaches an Android `Surface` through the shared `ServoViewBinding`
  and `ServoSurfaceLifecycleCoordinator`, loads an initial URL, pumps host
  updates from `Choreographer`, and drains the shared typed event stream.
- The native proof UI can drive load URL, reload, back, forward, focus, blur,
  navigation-policy resolution, JavaScript-dialog resolution, basic touch input,
  text IME composition/keyboard/dismissal, HTML `<select>` resolution, and
  file input resolve/dismiss, permission prompt allow/deny, and context-menu
  trigger/resolve/dismiss commands through the same shared Android host module
  and `ServoViewBinding` command path used below the RN adapter.
- The select proof uses proof-local Android dialog UI for single-select and
  multiple-select payloads, keeps optgroup labels visible, disables disabled
  options, and resolves committed selections through
  `ServoViewBinding.resolveSelectElement`. The current shared host API exposes no
  explicit `dismissSelectElement` call, so Cancel/Back resolves the unchanged
  selection rather than inventing a proof-only dismiss command.
- The context-menu proof uses long-press and secondary-button/right-click
  equivalents on the native `SurfaceView`, shows Servo-provided context-menu
  items plus element metadata in proof-local Android dialog UI, resolves chosen
  Servo actions through `ServoViewBinding.resolveContextMenu`, and routes Cancel/Back
  through `ServoViewBinding.dismissContextMenu`.
- The file-input proof decodes `filePickerRequested`/`filePickerDismissed`, opens
  Android's system document picker from the native proof activity, copies returned
  `content://` selections into the app cache, resolves paths through
  `ServoViewBinding.resolveFilePicker`, and routes Cancel/Back, empty selections,
  copy failures, or unavailable picker UI through `ServoViewBinding.dismissFilePicker`.
- The permission proof decodes `permissionRequested`, shows the requested origin
  and permission kind in proof-local Android UI, optionally requests the narrow
  Android runtime permission needed for geolocation/camera/microphone/
  notifications, and resolves Allow/Deny through `ServoViewBinding.resolvePermission`.
- The default URL is the cross-host smoke fixture at
  `http://127.0.0.1:8481/smoke/index.html`, served from the app's packaged copy
  of `examples/fixtures`. The same packaged server also exposes shared
  embedder-control fixtures under `http://127.0.0.1:8481/controls/`.

## Build

Prerequisites match the React Native Android package build:

- Android SDK with NDK `27.1.12297006`
- Rust target `aarch64-linux-android`
- `cargo ndk`

From the repository root:

```sh
examples/react-native-app/android/gradlew \
  -p examples/android-native-example \
  :app:assembleDebug
```

The Gradle build pulls in `:servokit-android-host`, which runs `cargo ndk` from
`crates/servokit-host-android` before the Android app is packaged.

## Run manually

Install and launch the debug APK on an arm64 Android device or emulator:

```sh
adb install -r examples/android-native-example/app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.servo.servokit.androidexample/.MainActivity
adb logcat -s ServokitNativeExample NativeServoView ExampleFixtureServer
```

Expected behavior:

- The activity shows a native Android header, a load-URL field, reload/back/
  forward/focus/blur buttons, fixture shortcut buttons, a `SurfaceView`
  containing the rendered Servo page, and a small event status footer.
- The smoke fixture shortcut row can navigate to the shared smoke, title,
  history, reload, form, error, and policy pages served from the packaged
  fixture assets.
- The embedder-control fixture shortcut row can navigate to the shared controls
  index plus dialog, text-input/IME, select, picker, file-input, permission, and
  context-menu fixture pages. Dialogs, text-input/IME, select controls,
  representative non-text pickers, file inputs, permission prompts, and context
  menus have proof-local Android UI/input handling in this app.
- Logcat and the footer include shared Servokit events such as `surfaceAttached`,
  `loadStatusChanged`, `urlChanged`, `historyChanged`, `navigationRequested`,
  `simpleDialogRequested`, `simpleDialogDismissed`, `inputMethodRequested`,
  `inputMethodDismissed`, `selectElementRequested`, `selectElementDismissed`,
  `permissionRequested`, `contextMenuRequested`, `contextMenuDismissed`, and
  `focusChanged` for
  `http://127.0.0.1:8481/smoke/index.html` and subsequent fixture navigations.

Manual navigation-policy and dialog smoke:

1. Tap **Policy** in the smoke fixture row.
2. Tap **Allowed same-origin link** in the rendered page. Expected: the footer and
   logcat show `navigationRequested` followed by `policy=allow`, the page
   navigates to `history-start.html?from=policy-allowed`, and history updates.
3. Go back to the policy page, then tap **Denied HTTP(S) link** and **Denied
   custom scheme**. Expected: each request shows `policy=deny` in the footer and
   logcat, and the policy page remains visible.
4. Tap **Dialogs** in the embedder-control fixture row.
5. Tap **Trigger alert**, **Trigger confirm**, and **Trigger prompt** in the
   rendered page. Expected: Android `AlertDialog` UI appears for each request,
   OK/Cancel/prompt values resolve through `ServoViewBinding.resolveSimpleDialog`,
   the fixture result text updates, and the footer/logcat show
   `simpleDialogRequested` plus the corresponding `simpleDialogDismissed` event.

Manual text-input and IME smoke:

1. Tap **Form** in the smoke fixture row.
2. Tap the rendered text input and email input. Expected: the footer/logcat show
   `inputMethodRequested`, the Android soft keyboard appears when the fixture
   allows it, typed text updates the rendered field, backspace/delete remove
   characters, and the hardware/software Enter action is routed through
   `ServoViewBinding.dispatchKeyboardKey`.
3. Tap the rendered textarea. Expected: the same text-entry path works for
   multiline input, including newline entry where the active Android keyboard
   exposes it.
4. Dismiss the keyboard with Back or press **Blur** after focusing a field.
   Expected: dismissal routes through `ServoViewBinding.dismissInputMethod`, the
   keyboard hides, and the footer/logcat show `inputMethodDismissed`.
5. Tap **Text input** in the embedder-control fixture row and repeat the text,
   textarea, backspace/delete, Enter, and dismissal checks for
   `controls/ime-form.html`. Contenteditable and lower-page fields are useful
   extra smoke checks, but this proof slice only commits to text input and
   textarea handling.

Manual select-element smoke:

1. Tap **Select** in the embedder-control fixture row to load
   `controls/select-elements.html`.
2. Tap **Basic single select** in the rendered page. Expected: logcat/footer show
   `selectElementRequested`, an Android dialog lists Apple/Banana/etc. as radio
   choices, OK resolves through `ServoViewBinding.resolveSelectElement`, the page's
   selected-value text and title update, and logcat/footer show the resolved
   selection plus `selectElementDismissed` when Servo emits it.
3. Tap **Select with optgroups** and **Disabled option handling**. Expected:
   optgroup labels appear as group headers, disabled options are visible but not
   enabled for selection, and enabled choices commit normally.
4. Tap **Multiple select**. Expected: the Android dialog uses checkboxes, allows
   multiple enabled options to be checked, and OK updates the page with all
   selected values.
5. Reopen a select, change a choice, then press **Cancel** or Back. Expected:
   because the current shared host API has no explicit select-dismiss command,
   the native proof resolves the original `selectedOptions` through
   `ServoViewBinding.resolveSelectElement`, leaving the page value unchanged.

Manual picker smoke:

1. Tap **Pickers** in the embedder-control fixture row to load
   `controls/pickers.html`.
2. Tap the **Date** field. Expected: the footer/logcat show
   `inputMethodRequested type=date`, an Android date picker opens, OK commits a
   `yyyy-MM-dd` value through `ServoViewBinding.dispatchImeComposition`, the page's
   visible date value/title update, and `ServoViewBinding.dismissInputMethod`
   triggers `inputMethodDismissed`. Reopen the field and press Cancel or Back;
   expected: no page value change and `inputMethodDismissed` appears.
3. Tap the **Time** field. Expected: an Android time picker opens and OK commits
   an `HH:mm` value through the same shared IME composition path.
4. Tap the **Datetime local** field. Expected: the proof app opens a date picker
   followed by a time picker, then commits a `yyyy-MM-ddTHH:mm` value through the
   shared IME composition path. Canceling either step dismisses the active input
   method without committing a new value.
5. Supported/degraded picker types in this proof: **date**, **time**, and
   **datetime-local** use native Android picker dialogs; **color** uses a small
   proof-local RGB slider dialog and commits `#rrggbb`; **month** uses Android's
   date picker with the day spinner hidden when the platform exposes that field
   and commits `yyyy-MM`; **week** uses Android's date picker to choose a day and
   commits that ISO week as `yyyy-Www`.
6. Explicit unsupported/degraded controls on `pickers.html`: **file** inputs are
   covered by the separate file fixture smoke below; **range**, **checkbox**, and
   **radio** are page/touch controls rather than host picker dialogs; **number**
   continues to use the text-like IME path rather than a native numeric picker
   dialog. These should not crash the app.

Manual context-menu smoke:

1. Tap **Context** in the embedder-control fixture row to load
   `controls/context-menu-demo.html`.
2. Long-press the rendered **Servo** link, or right-click it with a mouse/trackpad
   on an emulator/device that supports secondary-button input. Expected: the
   footer/logcat show `contextMenuRequested`, an Android dialog opens with the
   payload's link metadata and Servo-provided actions, and selecting one enabled
   action resolves through `ServoViewBinding.resolveContextMenu`. Follow any
   navigation/result expected by Servo for the selected action.
3. Long-press the **Generic target** block. Expected: the same proof-local
   dialog appears without React Native, reports a page/generic target in the
   metadata rows, and Cancel/Back routes through
   `ServoViewBinding.dismissContextMenu`; the footer/logcat should then show the
   native dismiss line and any `contextMenuDismissed` event emitted by Servo.
4. Practical extended coverage on the same fixture: long-press the **Image**
   target to verify image metadata and image actions; tap into the **Input** or
   **Textarea** and long-press to verify editable-text metadata/actions; select
   text in the selectable paragraph where the device interaction allows it, then
   open the context menu to verify `hasSelection`/selected-text metadata and
   text actions. These target-specific actions depend on Servo's current payload,
   but the native proof should always use the shared `triggerContextMenu`,
   `resolveContextMenu`, and `dismissContextMenu` host paths.

Manual file-input smoke:

1. Tap **File** in the embedder-control fixture row to load
   `controls/file-input.html`.
2. Tap **Single file** and choose a small local document from Android's system
   picker. Expected: footer/logcat show `filePickerRequested` and a native
   "file picker launching" line, the selected `content://` item is copied into
   this app's cache, `ServoViewBinding.resolveFilePicker` receives the copied path,
   and the fixture updates the visible selected-file summary.
3. Reopen **Single file** and press Back or Cancel in the system picker.
   Expected: the value remains unchanged and footer/logcat show
   `file picker dismissed by native ... reason=cancelled`, routed through
   `ServoViewBinding.dismissFilePicker`.
4. Tap **Multiple text/markdown files** and select more than one item where the
   installed Android document provider supports multiple selection. Expected:
   `allowSelectMultiple=true` is passed to `Intent.EXTRA_ALLOW_MULTIPLE`, all
   returned items are copied into the app cache, and the fixture displays all
   selected file names. If a provider ignores multiple selection, record that as
   a provider limitation rather than a ServoKit API limitation.
5. Tap **Image files** and **Capture-hinted image**. Expected: the picker is
   launched with MIME filters derived from Servo's `filterPatterns` (for example
   `image/*`) and selected images resolve through `ServoViewBinding.resolveFilePicker`.
   The proof intentionally does not implement camera/capture prompting; Android's
   document picker only receives MIME filters, so raw extension filters are mapped
   through `MimeTypeMap` when possible (with a small markdown fallback for this
   fixture) and unknown extensions may not be enforced by every provider.

Manual permission smoke:

1. Tap **Permissions** in the embedder-control fixture row to load
   `controls/permissions.html`.
2. Tap **Request notifications** where Servo exposes notifications. Expected:
   footer/logcat show `permissionRequested` with origin and `notifications`, an
   Android proof dialog shows the same details, Android 13+ may show the system
   notification runtime permission after **Allow**, and the decision resolves
   through `ServoViewBinding.resolvePermission`. **Deny**, Back, or dialog dismissal
   should resolve false through the same host path.
3. Tap **Request geolocation**. Expected: the proof dialog shows origin and
   `geolocation`; **Allow** requests Android location runtime permission when it
   is not already granted, then resolves allow only if Android grants fine or
   coarse location. **Deny** or an Android denial resolves false. The page may
   still report an unavailable/error result if the current Servo/Android proof
   stack has no location provider behind the granted permission.
4. Tap **Request camera** and **Request microphone** where `getUserMedia` is
   available. Expected: the same proof dialog appears for `camera`/
   `microphone`; **Allow** requests `CAMERA`/`RECORD_AUDIO` runtime permission
   before resolving allow. If Servo lacks a media capture backend on the device,
   the fixture should report an API/backend error rather than crashing the host.
5. Tap **Query permission states**. Expected: the fixture displays states for
   supported permissions or explicit unsupported/query errors on the page. The
   native proof app does not synthesize query results; it only handles host
   `permissionRequested` events.

Runtime permission and backend limitations:

- The native proof manifest declares `ACCESS_FINE_LOCATION`,
  `ACCESS_COARSE_LOCATION`, `CAMERA`, `RECORD_AUDIO`, and `POST_NOTIFICATIONS`,
  with camera and microphone hardware marked optional so the proof can install on
  emulators/devices without those features.
- Runtime permission prompts are mapped narrowly for `geolocation`, `camera`,
  `microphone`, `notifications`, plus `location`, `video-capture`, and
  `audio-capture` aliases. Unknown Servo permission kinds are still shown in the
  proof dialog and resolve through `ServoViewBinding.resolvePermission`, but no
  additional Android runtime permission is requested for them.
- Android runtime permission grant is necessary but may not be sufficient for a
  successful web API result. Current Servo/device support for location providers,
  media capture devices, notification delivery, secure-context requirements, and
  permission query APIs can still produce `unavailable`, `denied`, or backend
  errors visible in the fixture.

To load a different URL, pass the proof-app extra:

```sh
adb shell am start \
  -n org.servo.servokit.androidexample/.MainActivity \
  --es org.servo.servokit.androidexample.INITIAL_URL https://servo.org/
```
