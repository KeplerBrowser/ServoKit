# Servokit shared fixtures

This directory contains platform-neutral fixture pages for cross-host smoke
checks. The pages use only static HTML, CSS, and small inline JavaScript so they
can be reused by React Native Android, desktop examples, and native Android proof
apps without depending on React Native or a specific host API.

## Serving the fixtures

Serve this directory as the fixture root. For local desktop checks, one simple
option is:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Then load the smoke or embedder-control indexes:

```text
http://127.0.0.1:8481/smoke/index.html
http://127.0.0.1:8481/controls/index.html
```

The desktop winit and GPUI examples use that URL by default:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml
RUSTC_WRAPPER=sccache \
CARGO_TARGET_DIR=/tmp/servokit-gpui-target \
CARGO_PROFILE_DEV_DEBUG=0 \
cargo run --locked --manifest-path examples/desktop-gpui/Cargo.toml
```

The desktop examples validate live desktop rendering through the public
`servokit` facade, basic interaction, and URL/load/focus/surface tracing: the
page content should render in the native window or GPUI layout slot, links
should be clickable, fixture form fields should accept focus/text input,
scrolling should work, and the host UI/logs should show the shared event stream.

Android hosts can either embed the same files in app assets and serve them from
an app-local loopback server, or point to a development-machine server using the
emulator/device address that is appropriate for that environment. The React
Native Android example copies these shared files into its existing app-local
fixture server and exposes the smoke index at `/smoke/index.html` and the shared
embedder-control index at `/controls/index.html`.

The pages are also safe to load with `file://` for hosts that support file
navigation, except that the expected network-error target on `error.html` needs
HTTP navigation to exercise the unreachable-port case and some permission/media
APIs may require a secure loopback or HTTPS origin.

## Smoke pages and expected results

| Page | What it covers | Expected host result |
| --- | --- | --- |
| `smoke/index.html` | Initial load, fixture index, and viewport reflow signal | URL/load-status events are delivered, the page renders `Smoke fixtures ready`, and the `Viewport reflow` card plus page title update while the host window is resized. |
| `smoke/title-change.html` | Title changes | Initial title is `Servokit Smoke - Title Pending`, then changes to `Servokit Smoke - Title Changed`. |
| `smoke/history-start.html` and `smoke/history-next.html` | Link activation and back/forward history | Activating `Go to history next` changes the URL; host back returns to start and forward returns to next. |
| `smoke/reload.html` | Reload-visible content | Host reload changes the visible `Loaded at` timestamp and increments `Reload count` when storage is available. |
| `smoke/form.html` | Focusable fields | Text input, email input, textarea, select, checkbox, and button can receive focus/activation; focus status updates visibly. |
| `smoke/error.html` | Expected error behavior | Unreachable loopback and missing-page links produce host error/failed-load reporting without crashing the webview. |
| `smoke/policy.html` | Navigation-policy checks | Same-origin link is allowed; URLs containing `blocked-by-policy` and custom schemes should be denied by hosts that wire policy callbacks. |

## Embedder-control pages and expected results

The `controls/` pages are host-neutral triggers for Android parity and future
host-control slices. They do not implement host UI or assume a specific adapter;
unsupported controls should report unavailable/unsupported or fail gracefully
without crashing the webview.

| Page | What it covers | Expected host result |
| --- | --- | --- |
| `controls/index.html` | Control fixture index and coverage map | Page renders `Embedder-control fixtures ready` and links to each control fixture. |
| `controls/dialogs.html` | JavaScript `alert()`, `confirm()`, and `prompt()` | Host receives/presents simple dialog requests where supported; resolving the dialog updates the visible result and page title. |
| `controls/ime-form.html` | Text input, search/email/url/tel/password modes, textarea, contenteditable, and lower-page fields | Text focus/editing works, IME or keyboard appears where supported, and lower-page focused fields remain usable. |
| `controls/select-elements.html` | Single select, optgroups, disabled options, preselected value, and multiple select | Activating a select presents enabled options/labels where supported and committed selections update the visible value. |
| `controls/pickers.html` | Color, date, time, datetime-local, month, week, number, range, checkbox, and radio controls | Supported picker/control UI opens and committed values update the visible status; unsupported picker types degrade without crashing. |
| `controls/file-input.html` | Single file, multiple files, image accept filters, and capture-hinted file input | Activating a file input opens the host file picker where supported; selected files update the visible summary and cancellation leaves the value unchanged. |
| `controls/permissions.html` | Notification, geolocation, camera, microphone, and Permissions API query triggers | Supported APIs surface a host permission request and resolve to granted/denied/prompt or an error/unavailable message on the page. |
| `controls/context-menu-demo.html` | Link, selected text, image, editable text, and generic context-menu targets | Long-press/right-click produces target metadata and default actions where supported; unsupported context menus fail gracefully. |

## Suggested host smoke flow

1. Start or embed a fixture server rooted at `examples/fixtures`.
2. Load `smoke/index.html` as the initial URL.
3. Visit each smoke page from the smoke index and each embedder-control page
   from `controls/index.html`, then compare the observed host events and UI with
   the expected results above.
4. Use this fixture-specific navigation policy when a host exposes policy
   callbacks: allow HTTP(S) URLs unless the URL contains `blocked-by-policy`, and
   deny unsupported custom schemes.
5. Keep host-specific UI, screenshots, and automation outside this directory so
   the fixture set remains reusable across examples.
