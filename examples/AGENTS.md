# Examples guide for agents

Examples prove that ExplorerKit's layers work. They are not products, and they
can change without notice.

## Rules

- Keep examples thin. Reusable logic belongs in a crate or in the shared
  Android module, not in an example.
- Desktop examples use only the public `explorerkit` crate. Don't import
  `explorerkit-embedder` or `servo` directly. The `profile-branch-proof`
  binary in `desktop-winit` still does; don't copy that pattern.
- Android examples use the shared Gradle module at
  `crates/explorerkit-host-android/android`. Add shared Android behavior there.
- Don't add public APIs in examples, and don't present example code as an SDK.

## Which app is which

| Folder | Role |
| --- | --- |
| `react-native-app` | Public React Native demo for Android and iOS |
| `react-native-macos-app` | React Native macOS prototype |
| `react-native-example-app` | Browser UI shared by both React Native apps |
| `../packages/react-native-explorerkit/example` | The package's test app, used by the Maestro flows and Android unit tests |
| `android-native-example` | Kotlin app without React Native, with native UI for every prompt |
| `android-kotlin-browser` | Builds `android-native-example`'s sources under another name; has no code of its own |
| `desktop-winit` | Rust app owning a `winit` window |
| `desktop-gpui` | Rust GPUI app embedding Servo in one layout slot (macOS) |
| `fixtures` | Shared test pages |

## Test pages

- Pages in `fixtures/` are static HTML, CSS, and small inline scripts. They
  must not depend on a particular host or framework.
- Add a row to [fixtures/README.md](fixtures/README.md) for every new page,
  with what it covers and the expected result.
- Android apps copy the pages at build time with their `syncFixtureAssets`
  Gradle task. Don't commit copies. Known gap: the package test app also
  bundles committed copies from
  `packages/react-native-explorerkit/example/fixtures/`, which differ from
  `fixtures/controls/`, alongside the shared pages.

## Cargo roots

`desktop-winit` and `desktop-gpui` are separate Cargo roots, each with its own
`Cargo.lock`. Build them with `--locked`. Only `desktop-gpui` uses the
`stylo_derive` and `zed-font-kit` patches. See
[Dependencies](../docs/reference/dependencies.md).

## Validate

Use the commands in [Testing and validation](../docs/reference/testing.md).
Manual checks need a device, simulator, or desktop session. Record what you
ran, or "not run", in the pull request.
