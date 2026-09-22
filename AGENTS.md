# ServoKit Agent Guide

ServoKit is a Servo embedding toolkit with Rust-owned browser/controller semantics on Servo-backed paths and thin platform adapters. The React Native iOS adapter maps the shared Fabric surface through the Servo-free portable Rust controller to WKWebView and owns WebKit objects natively.

Start architecture changes from [ARCHITECTURE.md](ARCHITECTURE.md). When implementing, reviewing, or submitting changes, use [the development skill](.agents/skills/development/SKILL.md) to pick the owning layer and validation slice.

Use `bun` for JavaScript/TypeScript package work. Rust workspace commands usually target `crates/Cargo.toml`.

Keep diffs scoped to the approved ticket. Do not turn UBRN, Nitro, TurboModules, handwritten JSI, popup policy, or distribution mechanics into the architecture unless the ticket explicitly asks for that.

Before implementing public ServoKit methods, props, events, commands, or host-control surfaces, lock exact names, provenance (upstream Servo, ServoKit-owned, or adapter-only), non-goals, and owning-layer acceptance evidence in the approved tracker item. Cite upstream names for Servo mappings; adapter-only names must not imply Servo support or app-owned tabs, windows, and popup presentation.

Public docs describe validated architecture and APIs. Keep private planning and scratch work local; `.gitignore` owns the excluded paths.

Use [CONTRIBUTING.md](CONTRIBUTING.md) for RFCs, labels, milestone conventions,
issue granularity, and proof-artifact retention.

Investigate unresolved design questions before implementation. Keep investigation in the feature issue unless it has an independently useful outcome. Repository guidance is authoritative; the Wiki provides supplementary recipes and troubleshooting.

## Agent Cadence

Keep at most one xhigh implementation or review subagent active. Keep at most one native-building implementation stream active. Run sequential work in the main checkout; never create manual worktrees under `/private/tmp`. When isolation is genuinely required, use only the built-in Codex-managed agent workspace.

Run important infrastructure, toolchain, signing, CI, or hardware work autonomously through at most one xhigh subagent at a time in the main checkout. Capture commands and evidence; clean only disposable outputs or agent workspaces after their evidence is consumed, and never delete user work. Release workflow design remains open; implementation approval does not authorize release promotion.

## Approval and delivery

When assigned an approved change, carry it through implementation, independent
review, and merge unless the assignment explicitly limits your scope. Give
subagents the reference for their assigned phase and a clear stopping point.

- Proposed changes normally start as `rfc`; a maintainer's explicit approval of
  the concrete scope is recorded in the issue before it becomes `accepted`.
  Explicit human invocation authorizes implementation, PR submission, and merge
  within that scope once the definition of done is met. Small typos and obvious,
  mechanically verifiable corrections that preserve behavior and contracts
  have standing authorization when invoked.
- Classify risk by changed guarantees, not diff size. Refactoring preserves
  behavior, contracts, ownership, lifecycle, and threading. Changes to public
  contracts, ownership, lifecycle/threading guarantees, security/FFI
  boundaries, platform support, or distribution require independent
  architectural review and human approval before implementation.
- Keep unresolved review concerns, missing required evidence, and scope changes
  in the issue discussion for human steering. A local approved ticket or explicit
  session agreement may stand in for an issue; publish only authorized content.
- Deliver one feature or engineering outcome per PR, across all required layers.
  Use one commit by default, at most five meaningful scoped commits. Fold tests
  and review corrections into the owning commit unless a distinct concern merits
  its own. Revisit delivery scope with humans if five commits cannot express it.
- Keep coauthor attribution truthful; commitlint owns message-format checks.
  Name branches `<type>/<kebab-case-topic>` and PRs `type(scope): imperative summary`; append
  `(KEPL-###)` only for a real ticket. Target `main`.
- Rebase onto the current target before landing and preserve a linear history
  with the curated commits. Code changes require an independent reviewer; the
  implementer cannot self-certify. Use the pull-request phase definition of done
  to merge without a routine final human approval.
- Preserve doc comments unless their removal is explicitly requested.
