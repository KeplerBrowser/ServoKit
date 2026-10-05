# Test pages

Static pages that every ExplorerKit example uses for manual checks. They use
only HTML, CSS, and small inline scripts, so they work in any host: desktop,
Android, and React Native.

## Serve them

From the repository root:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Then open one of the index pages:

```text
http://127.0.0.1:8481/smoke/index.html
http://127.0.0.1:8481/controls/index.html
```

The desktop examples open the smoke index by default. The Kotlin Android
examples and the React Native package's test app copy these pages into the
app at build time and serve them on the same port from inside the app, so you
don't need a server for them on Android.

Most pages also work over `file://`. Two exceptions: the unreachable-port link
in `error.html` needs HTTP, and some permission and media APIs need a secure
loopback or HTTPS origin.

A page that loads proves only that it loads. Support claims live in
[What works where](../../docs/reference/capabilities.md).

## Basic pages (`smoke/`)

| Page | Tests | What you should see |
| --- | --- | --- |
| `smoke/index.html` | First load and resize | URL and load events arrive, the page shows **Smoke fixtures ready**, and the **Viewport reflow** card and title change as you resize the window. |
| `smoke/title-change.html` | Title changes | The title changes from `ExplorerKit Smoke - Title Pending` to `ExplorerKit Smoke - Title Changed`. |
| `smoke/history-start.html`, `smoke/history-next.html` | Links and history | **Go to history next** changes the URL. Back returns to the start page, and forward returns to the next page. |
| `smoke/reload.html` | Reload | Reloading changes **Loaded at** and increases **Reload count** when storage works. |
| `smoke/form.html` | Focus | Text, email, textarea, select, checkbox, and button take focus, and the page shows focus changes. |
| `smoke/error.html` | Errors | The unreachable-port and missing-page links report a failed load without crashing the view. |
| `smoke/policy.html` | Navigation policy | The same-origin link is allowed. URLs containing `blocked-by-policy`, and custom schemes, are denied by hosts that implement policy. |

## Prompt pages (`controls/`)

These pages trigger the questions a page can ask its app. They don't assume
any particular UI. Hosts that don't support a prompt should fail cleanly,
without crashing.

| Page | Tests | What you should see |
| --- | --- | --- |
| `controls/index.html` | Index | **Embedder-control fixtures ready** and links to every page below. |
| `controls/dialogs.html` | `alert()`, `confirm()`, `prompt()` | The host shows each dialog where supported, and the answer updates the page and title. |
| `controls/ime-form.html` | Text input modes, textarea, `contenteditable`, lower-page fields | Editing works, the keyboard appears where supported, and fields lower on the page stay usable. |
| `controls/select-elements.html` | Single, grouped, disabled, preselected, and multiple selects | The host lists enabled options, and picks update the page. |
| `controls/pickers.html` | Color, date, time, `datetime-local`, month, week, number, range, checkbox, radio | Supported pickers open and commit values. Unsupported types fail without crashing. |
| `controls/file-input.html` | Single, multiple, image-filtered, and capture-hinted file inputs | The host file picker opens where supported. Picks update the page, and cancel changes nothing. |
| `controls/permissions.html` | Notifications, geolocation, camera, microphone, Permissions API queries | Supported APIs ask the host, and the page shows granted, denied, prompt, or an error. |
| `controls/context-menu-demo.html` | Links, selected text, images, editable text, generic targets | Long-press or right-click reports the target and offers actions where supported. |

## A suggested check

1. Serve or embed the pages.
2. Open `smoke/index.html` first.
3. Visit each basic page and each prompt page, and compare what your host does
   with the tables above.
4. If your host has a navigation policy hook, allow HTTP(S) URLs unless they
   contain `blocked-by-policy`, and deny unknown custom schemes.
5. Keep host-specific UI, screenshots, and automation outside this folder, so
   the pages stay reusable.
