# ServoKit Agent Guide

ServoKit is a Servo embedding toolkit with Rust-owned browser/controller semantics on Servo-backed paths and thin platform adapters. The React Native iOS adapter maps the shared Fabric surface through the Servo-free portable Rust controller to WKWebView and owns WebKit objects natively.

Start architecture changes from [ARCHITECTURE.md](ARCHITECTURE.md). Before editing code or docs, use [docs/development.md](docs/development.md) to pick the owning layer and validation slice.

Use `bun` for JavaScript/TypeScript package work. Rust workspace commands usually target `crates/Cargo.toml`.

Keep diffs scoped to the approved ticket. Do not turn UBRN, Nitro, TurboModules, handwritten JSI, popup policy, or distribution mechanics into the architecture unless the ticket explicitly asks for that.

Before implementing public ServoKit methods, props, events, commands, or host-control surfaces, lock the names in the approved tracker item and route provenance, non-goals, and owning-layer tests through [docs/development.md](docs/development.md).

Public docs are for durable architecture/API truth, not progress tracking. An approved tracker item may be a GitHub issue or a local ticket. Keep local PM state and scratch files uncommitted unless explicitly approved: `CONTEXT.md`, `PLAN.md`, `TODO.md`, `.scratch/`, `plans/`, `context/`, `research/`, and subagent output.

Use the public planning terms and contribution flow in [CONTRIBUTING.md](CONTRIBUTING.md). The active milestone is the descriptive sprint goal. A local Goal may cover one issue or a few related issues within that milestone, or the local equivalent, and may persist across working sessions. A local ticket may stand in for an issue.

Use Wayfinder only when the route to a large destination is unclear. Resolve one decision ticket per working session, except research tickets, then hand the clear route to implementation issues. Generic ServoKit Wayfinder work may use the public tracker; keep private-product and preliminary context local. The [GitHub Wiki](https://github.com/KeplerBrowser/ServoKit/wiki) is the living process handbook, while repository documentation remains authoritative for version-coupled truth.

## Agent Cadence

Keep at most one xhigh implementation or review subagent active. Keep at most one native-building implementation stream active. Run sequential work in the main checkout; never create manual worktrees under `/private/tmp`. When isolation is genuinely required, use only the built-in Codex-managed agent workspace.

Run important infrastructure, toolchain, signing, CI, or hardware work autonomously through at most one xhigh subagent at a time in the main checkout. Capture commands and evidence; clean only disposable outputs or agent workspaces after their evidence is consumed, and never delete user work. Keep final merge and release promotion human-gated.
