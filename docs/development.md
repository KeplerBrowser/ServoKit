# Development Guide

This is the workflow router for changing ServoKit. Architecture truth lives in
[`ARCHITECTURE.md`](../ARCHITECTURE.md); detailed behavior belongs in the
topic docs below.

## Pick The Owner First

| Change | Owning layer | Start with |
| --- | --- | --- |
| Shared controller command validation, request identity, pending semantics, fallback policy, and Servo-backed runtime integration | Rust ServoKit crates | [`docs/servokit.md`](servokit.md), [`docs/runtime-and-module-map.md`](runtime-and-module-map.md), [`docs/controller-seam.md`](controller-seam.md) |
| React Native component API, props, events, refs, presentation hooks | `react-native-servokit` | [`docs/react-native-servokit.md`](react-native-servokit.md), [`docs/react-native-rust-control-seam.md`](react-native-rust-control-seam.md) |
| Android view, surface, lifecycle, IME, JNI, native host module | Android host adapter | [`docs/android-build.md`](android-build.md), [`docs/embedded-surface-contract.md`](embedded-surface-contract.md) |
| iOS WKWebView behavior, native completions and timers, Rust-effect execution, recycling, and native objects | WKWebView/UIKit adapter above the portable Rust controller | [`docs/react-native-servokit.md`](react-native-servokit.md), [`docs/host-control-capabilities.md`](host-control-capabilities.md) |
| macOS React Native view, AppKit handles/input/scheduling, and private C transport | macOS adapter above Rust ServoKit | [`docs/react-native-servokit.md`](react-native-servokit.md), [`docs/react-native-rust-control-seam.md`](react-native-rust-control-seam.md) |
| Surface modes and embedder boundaries | Surface contract | [`docs/embedded-surface-contract.md`](embedded-surface-contract.md), [`docs/surface-modes.md`](surface-modes.md) |
| Capability coverage or deferred behavior | Host-control references | [`docs/host-control-capabilities.md`](host-control-capabilities.md), [`docs/feature-coverage.md`](feature-coverage.md) |
| Build or smoke validation | Validation matrix | [`docs/readiness-checks.md`](readiness-checks.md) |

Apps own product UI, tabs, windows, chrome, and final presentation policy. Do
not move those decisions into ServoKit unless an approved issue scopes them.

## Name Public Surfaces Deliberately

Before implementing or changing a public ServoKit method, prop, event, command,
or host-control surface:

- Lock the exact public names in an approved issue before implementation.
- State each name's provenance: upstream Servo API, ServoKit-owned vocabulary,
  or platform adapter-only mapping.
- For Servo-backed mappings, use Servo-aligned terms and cite the Servo source
  name when applicable, such as `WebViewDelegate::request_create_new` or
  `CreateNewWebViewRequest`.
- For adapter-only mappings, avoid names or docs that imply the surface exists
  in Servo.
- State non-goals when a name could imply unsupported product behavior, such as
  app-owned popup presentation, chrome, tabs, windows, opener lifecycle, or
  React Native/adapter-managed child presentation or adoption.

## Keep Changes Small

- Change the smallest layer that owns the behavior.
- Prefer an existing seam over a new abstraction.
- Keep detached/headless controllers, preload/user scripts, popup presentation,
  distribution, accessibility, and multi-window lifecycle as separate product
  decisions unless an approved issue scopes them in.
- Do not claim support in docs until implementation and validation have landed.
- Public docs describe durable architecture and API behavior, not work in
  progress.

## Validate The Narrow Slice

Run the smallest check that can fail for the touched layer:

- Docs only: `git diff --check`.
- TypeScript package work: `bun run --cwd packages/react-native-servokit typecheck`.
- Rust workspace work: `cargo test --manifest-path crates/Cargo.toml -p <crate> <focused filter>`.
- React Native Android native work: package typecheck plus the narrow Android
  codegen/Kotlin/unit slice from `examples/react-native-app/android`.
- Shared Android host module changes: validate both `examples/react-native-app`
  and `examples/android-native-example`.
- Public surface changes must test the owning layer that changed, not only a
  lower shared bridge. Servo-backed controller commands, validation, pending
  control, fallback policy, and Servo integration need Rust checks; portable
  iOS command validation, request identity, pending semantics, and fallback
  policy need Rust checks, while native completions, timers, recycling, and
  Rust-effect execution need iOS adapter checks; React Native `ServoView`
  props/events/refs need package checks; platform view, lifecycle, input, and
  adapter mappings need their platform slice.

If a relevant check is skipped, say why in the PR or final response.
