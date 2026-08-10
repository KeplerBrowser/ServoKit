# Contributing

Contributions are welcome, no matter how large or small. Keep discussion
friendly and respectful.

Keep contributions focused and constructive. Start with the repository guide
in [`README.md`](./README.md), then use
[`ARCHITECTURE.md`](./ARCHITECTURE.md) and
[`docs/development.md`](./docs/development.md) to identify the owning layer and
the smallest relevant validation slice.

## Planning work

ServoKit uses these public planning terms:

- A **milestone** is a descriptive, goal-based body of related work: the public
  sprint goal. It may span several working sessions and need not be named after
  a release version. Examples include `Initial release`, `macOS support`,
  `Windows support`, and `GPU surface`.
- An **issue** is an independently acceptable outcome.
- A **Wayfinder ticket** is an issue that resolves one decision or investigation
  for maintainers planning a larger change.
- A **pull request** is a reviewable delivery that may complete an issue or one
  meaningful slice.
- A **commit** is one coherent implementation step.

Every milestone description states its **Outcome**, **Done when**, and **Not
included** boundaries. Issue titles use concise sentence case and name a
concrete deliverable or outcome; mechanisms and acceptance details belong in
the body. Avoid artificial prefixes and vague titles such as `Implement
support`, `Improve`, or `Investigate` when a stronger outcome exists.

A release is separate from a milestone. A version may appear in a milestone
description without becoming its title.

Maintainers may use Wayfinder when the route through a large change is unclear.
Contributors do not need Wayfinder or any local planning tools to report a bug,
propose a change, or work from an approved issue.

An issue may produce zero, one, or many commits. Its body should make the
outcome, motivation, scope, acceptance evidence, and non-goals clear, adding
dependencies, durable references, and public-surface provenance when relevant.
Split work only when ownership, sequencing, infrastructure, or failure domain
genuinely differs.

Small fixes, documentation, and obvious bugs may go directly to a pull request.
Public API, architecture, platform support, and distribution changes require an
approved issue before implementation. Keep required tests, documentation, and
validation with the implementation unless they are independently deliverable.
Reference related issues without implying closure; use a closing keyword only
when the pull request genuinely completes the issue. Routine planning,
editorial maintenance, and tiny supporting changes do not need manufactured
issues.

## Development workflow

This project is a Servo embedding workspace. Its main contributor surfaces are:

- ServoKit Rust crates in `crates/`.
- The React Native package in `packages/react-native-servokit/` and its
  package-owned contract example in `packages/react-native-servokit/example/`.
- The shared Android/iOS React Native app in `examples/react-native-app/`.
- The React Native macOS verification app in
  `examples/react-native-macos-app/`, which exercises the experimental,
  unsupported AppKit/private-C/Rust path.
- Native Android and Rust desktop examples in `examples/`.

To get started with the project, install a current [Node.js](https://nodejs.org/) release compatible with the checked-in Bun/React Native workspace.

Run `bun install` in the root directory to install the required JavaScript
dependencies:

```sh
bun install
```

Use the example that owns the platform path you changed. The
[shared React Native app](./examples/react-native-app/) covers Android and the
packaged iOS WKWebView baseline. The
[React Native macOS app](./examples/react-native-macos-app/) is its separate
runtime verification surface.

Before changing native Servo integration, look upstream first:

1. `ports/servoshell` in the `servo/servo` repository for the closest supported embedding example.
2. The Servo repository itself for current platform behavior, build logic, and runtime conventions.
3. The published `servo` crate docs and crates.io metadata for API-level expectations.

On Servo-backed Android and macOS paths, Rust owns controller/browser semantics
while the native adapters own platform views, handles, geometry, input, and
scheduling. On iOS, the portable Rust controller owns command validation,
request identity, pending semantics, fallback policy, and response validation;
Objective-C++ owns WKWebView, native completion/timer objects, KVO/recycling,
effect execution, and main-thread scheduling. Keep the public Fabric
`ServoView` interface shared across those engine-specific paths.

The React Native example is configured to use the local package, so changes you
make under `packages/react-native-servokit` are reflected in the example app.
JavaScript changes are reflected without a native rebuild, but native code
changes require rebuilding the example app.

If you want to use Android Studio to edit the native code, open
`examples/react-native-app/android` and find the package sources
under `react-native-servokit`.

You can use various commands from the root directory to work with the project.

To start the packager:

```sh
bun run --cwd examples/react-native-app start
```

To run the example app on Android:

```sh
bun run --cwd examples/react-native-app android
```

To confirm that the app is running with the new architecture, you can check the Metro logs for a message like this:

```sh
Running "ServoExample" with {"fabric":true,"initialProps":{"concurrentRoot":true},"rootTag":1}
```

Note the `"fabric":true` and `"concurrentRoot":true` properties.

For React Native package changes, run the focused tests for the changed contract
and these package checks:

```sh
bun test packages/react-native-servokit/src/__tests__/<focused-test>.test.ts
bun run --cwd packages/react-native-servokit typecheck
bun run --cwd packages/react-native-servokit prepare
```

For Rust changes, target the owning package instead of defaulting to the whole
workspace:

```sh
cargo test --manifest-path crates/Cargo.toml -p <package> --locked
```

Run a native build or runtime gate only for the platform path you changed. The
current commands and prerequisites live in
[`docs/readiness-checks.md`](./docs/readiness-checks.md).

### Scripts

The `package.json` file contains various scripts for common tasks:

- `bun install`: set up the project dependencies.
- `bun run --cwd packages/react-native-servokit typecheck`: type-check package files with TypeScript.
- `bun run --cwd packages/react-native-servokit prepare`: build the package output.
- `bun run --cwd examples/react-native-app start`: start the Metro server for the example app.
- `bun run --cwd examples/react-native-app android`: run the example app on Android.

### Sending a pull request

> **Working on your first pull request?** You can learn how from this _free_ series: [How to Contribute to an Open Source Project on GitHub](https://app.egghead.io/playlists/how-to-contribute-to-an-open-source-project-on-github).

When you're sending a pull request:

- Prefer small pull requests focused on one change.
- Verify that linters and tests are passing.
- Review the documentation to make sure it looks good.
- Follow the pull request template when opening a pull request.
- Public API or architecture changes require maintainer agreement and an
  approved issue before implementation.
