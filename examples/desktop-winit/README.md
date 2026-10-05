# winit example

A Rust desktop app that owns a `winit` window and event loop, and shows a
Servo page across the whole window through the `explorerkit` crate. It is the
simplest way to see Servo running inside ExplorerKit.

## Run

Optionally start the test pages from the repository root:

```sh
python3 -m http.server 8481 --directory examples/fixtures
```

Then run:

```sh
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml
cargo run --locked --manifest-path examples/desktop-winit/Cargo.toml -- https://servo.org/
```

Add `--smoke` before the URL for a scripted check that exits with code 0 once
the page loads.

## Read more

The [Desktop guide](../../docs/platforms/desktop.md) walks through this
example step by step, including smoke mode, expected output, and the resize
check. The `profile-branch-proof` binary in this folder is a separate proof
tool, described in
[Testing and validation](../../docs/reference/testing.md#profile-copy-proof).
