# Rust crates guide for agents

This guide covers `crates/`. Read [Crates and packages](../docs/reference/crates.md)
for each crate's job, and [ARCHITECTURE.md](../ARCHITECTURE.md) for the
principles every change must respect.

## Put code in the right crate

| Change | Crate |
| --- | --- |
| Public Rust API for native apps (curated re-exports, documented items) | `servokit` |
| Servo delegate translation, pending prompt state, the command parser, events, the portable controller | `servokit-embedder` |
| Host-neutral surface types and traits | `servokit-host` |
| Android rendering, JNI, and Android platform services | `servokit-host-android` |
| The private desktop C boundary for React Native macOS | `servokit-host-desktop` |
| The iOS controller C boundary | `servokit-controller-ffi` |

Dependency rules:

- `servokit-host` must not depend on any other ServoKit crate.
- Only desktop hosts and apps may depend on `servokit`.
- Servo code must sit behind the `servo` feature. `servokit-controller-ffi`
  must never turn it on, and the always-compiled modules of
  `servokit-embedder` (`controller_command`, `host_event_bridge`,
  `portable_controller`, `runtime`, `servo_adapter`) must not import Servo
  types.
- Don't add crates named `servokit-core`, `servokit-protocol`, or
  `servokit-jni` without an accepted decision.
- Don't `pub use` an internal crate wholesale from `servokit`. Every
  re-export is public API. `servokit` uses `#![deny(missing_docs)]`.

Before designing Servo integration, compare with upstream servoshell and the
published `servo` API. See [Dependencies](../docs/reference/dependencies.md#the-upstreamservo-submodule).

## Commands and events

- Add or change browser commands and prompt answers only in `ControllerCommand`
  (`servokit-embedder/src/controller_command.rs`). It is the one parser for
  every platform. Update [the controller doc](../docs/concepts/controller.md).
- Add events to `HostEvent` and to the encoder in `host_event_bridge.rs`. Then
  update every decoder that must understand them: the Kotlin
  `ServoHostEventBridge.kt` and the iOS and macOS adapters in
  `packages/react-native-servokit`.
- Request IDs, pending prompts, and answer validation belong in Rust, not in
  adapters. Don't add timers or made-up answers anywhere: a prompt nobody
  handles ends with the engine's own default.

## FFI boundaries

- Catch panics at every exported C function, as the `catch_boundary` helpers
  in `servokit-host-desktop` and `servokit-controller-ffi` do. Known gap: the
  JNI and C exports in `servokit-host-android` don't catch panics yet.
- Check null pointers, lengths, and UTF-8 before use. Never pass Servo types
  across a C boundary.
- Keep the C headers in `include/` in sync with the exported functions.
- Keep ABI changes separate from behavior changes.

## Threads and the engine

Servo starts once per process and cannot move between threads. Don't move or
restart the engine. Real-Servo tests share one process, so run them with
`--test-threads=1`.

## Validate

```sh
# Quick: no Servo build
cargo test --manifest-path crates/Cargo.toml -p servokit-embedder -p servokit-host -p servokit-controller-ffi -p servokit --locked
cargo fmt --manifest-path crates/Cargo.toml --all --check

# Lints for the crate you changed
cargo clippy --manifest-path crates/Cargo.toml -p <crate> --all-targets --locked -- -D warnings
```

Servo-backed, Android, and C boundary checks are in
[Testing and validation](../docs/reference/testing.md). Keep `crates/Cargo.lock`
unchanged unless the change is a dependency update.
