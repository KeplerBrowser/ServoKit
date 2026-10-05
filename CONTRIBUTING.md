# Contributing

ExplorerKit is built by AI coding agents. People decide what to build; agents
investigate, write the code, test it, review it, and merge it. **You don't need
to write code to contribute.** Describing a real problem clearly is the most
valuable thing you can do.

Please keep discussions friendly, focused, and constructive.

## How a change happens

```text
1. Open an issue        Describe the problem or idea. New proposals get the `rfc` label.
2. Discuss              An agent helps work out scope, options, non-goals, and how to prove it works.
3. Approve              A maintainer approves the concrete scope. The label becomes `accepted`.
4. Invoke               A contributor explicitly asks an agent to build the accepted issue.
5. Build and review     The agent implements and tests it, and an independent reviewer checks it.
6. Merge                The agent merges once the definition of done is met.
```

Acceptance allows the work but does not start it. Work starts only when a
contributor explicitly asks an agent to build the accepted issue.

## Start with an issue

- **Ideas, features, refactors, and design proposals:** use the feature
  request form. Describe the outcome you want and why it matters. You don't
  need an architecture plan to start. The depth of discussion should match how
  much the change could break.
- **Bugs:** use the bug report form with steps to reproduce.
- **Small fixes:** typos and obvious corrections that don't change behavior or
  contracts can go ahead without a proposal. Bigger fixes still need
  agreement on scope.

Labels describe the kind of contribution: `bug`, `enhancement`,
`documentation`, or `question`. `help wanted` and `good first issue` invite
reproduction, investigation, and design help as well as code. Closed issues
and linked pull requests show what is done and delivered.

## Getting to "accepted"

A proposal is ready when its scope and its acceptance evidence are concrete,
and any architectural concerns are resolved. A clean review by an agent does
not replace human approval. A maintainer approves in the issue, then the agent
records the decision and swaps `rfc` for `accepted`.

Within the approved scope, agents choose the implementation details.
Foundational changes, such as public contracts, ownership, lifecycle or
threading guarantees, security or FFI boundaries, platform support, or
distribution, get an independent architectural review before approval. New
trade-offs, bigger scope, unresolved review concerns, and missing evidence go
back to the same issue, where people decide the next step.

Investigate open design questions in the issue itself. Contributors don't
need special planning tools.

## Keep work meaningful

An issue is one outcome that could be accepted or deferred on its own. Tests,
file edits, and review fixes are part of that outcome, not separate tickets.
Split an issue only when part of it could be accepted or deferred
independently. A pull request delivers one feature or fix across every layer
it touches.

Write the outcome as the behavior or guarantee that becomes true. Cleanup,
investigation, evidence gathering, and file moves are how you deliver an
outcome, not outcomes themselves. Fold them into the nearest active feature or
fix. When maintenance has no active outcome to belong to, a maintainer can
approve one scoped change directly. Open an issue for maintenance only when it
needs its own decision or could be deferred on its own.

## Keep proof tools temporary

This is the proof-artifact retention rule. Scripts written to research or
prove something once are temporary by default.
Record their revision, commands, results, and limits in the issue or pull
request, then remove them before merging.

Keep a proof tool or check in the repository only when the accepted scope
names three things: the guarantee it protects, the layer that owns it, and
the event that requires running it again. When a later change replaces a
tracked proof, remove the old one in that change and keep its results in the
history.

## Milestones

Milestones are shared goals for a stretch of work. Each one states an
**Outcome**, **Done when**, and **Not included**. A milestone can span many
working sessions, and it is separate from releases.

## Implementation and merge

Once invoked, the implementing agent owns the change through the pull request,
independent review, and merge. It checks the agreed acceptance criteria,
resolves review findings, and passes the required checks before merging. No
final human sign-off is needed. The pull request records the result and the
evidence. People come back in when scope or design changes, or when a concern
can't be resolved within the agreement. How releases work is still undecided
and is not covered by this approval.

## For agents and maintainers

- [AGENTS.md](AGENTS.md): the rules agents follow.
- [Development skill](.agents/skills/development/SKILL.md): the playbooks for
  implementing, reviewing, and merging.
- [How ExplorerKit works](ARCHITECTURE.md): the layers and the principles every
  change must respect.
- [Testing and validation](docs/reference/testing.md): the checks for each
  area.
