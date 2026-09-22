# Implementation

Read the approved scope and trace the affected callers. Consult the ownership
model in [ARCHITECTURE.md](../../../../ARCHITECTURE.md) and follow its links for
the affected platform. For Servo integration, compare upstream servoshell and
Servo APIs before designing an adapter.

Implement in the owning layer, keeping tests and documentation with the feature.
Update existing docs when public behavior, API contracts, ownership, setup, or
platform support changes. Add a document only for a durable topic with no existing
home. Internal refactors that preserve documented behavior need no documentation
change; investigation and progress belong in the issue or PR.

Apply [CONTRIBUTING's](../../../../CONTRIBUTING.md) proof-artifact retention rule before adding a tracked
executable, fixture, or readiness check. For a one-time proof, preserve the
evidence in the issue or PR and remove its machinery before merge. A durable
artifact must protect the accepted guarantee at its owning layer and have an
explicit rerun trigger.

Use [readiness checks](../../../../docs/readiness-checks.md) for platform commands.
Choose evidence for the changed behavior:

- Documentation: changed links and `git diff --check`.
- React Native contract: focused package tests, typecheck, and package build.
- Rust semantics: focused tests in the owning crate with the lockfile preserved.
- Native behavior: the affected adapter/build/runtime slice. Shared Android host
  changes must cover both React Native and native Android consumers.

A lower bridge test does not prove a public-surface contract. On iOS, reducer
semantics need portable Rust tests; completion/timer/recycling behavior needs
WKWebView adapter checks. Test each changed owner across a cross-layer feature.

For delegated implementation, return the changed behavior, commands/results,
and missing evidence to the delegating agent and stop at the assigned boundary.
Otherwise, obtain independent review and follow the
[pull-request phase](pull-request.md) through merge.
