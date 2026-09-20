# Contributing

ServoKit uses agent-led development. Contributors describe problems, discuss
tradeoffs, and approve designs; agents investigate, implement, test, review, and merge.
You do not need to write code or prepare a pull request to contribute.
Keep discussion friendly, focused, and constructive.

## Start with an issue

Use the feature request form for a feature, refactor, or architectural proposal.
Describe the desired outcome and why it matters. Start with `rfc`; an
agent helps develop scope, alternatives, non-goals, and acceptance evidence in
the issue. You do not need an architectural plan before starting the discussion.
Discussion depth should match the change's consequences.

Report reproducible failures with the bug form. Small typos and obvious,
behavior-preserving corrections can proceed under standing authorization without
a manufactured proposal. Significant fixes still need agreement on their scope.

## Discuss, accept, then invoke

```text
rfc -> discussion and human approval -> accepted -> explicit agent invocation
                                                   -> PR and independent review -> merge
```

A proposal is ready for acceptance when its scope and acceptance evidence are
concrete and architectural findings are resolved; a clean agent review does not
replace human approval. A maintainer approves the proposal in the discussion. The agent records
that decision and replaces `rfc` with `accepted`. Acceptance permits that scope;
it does not automatically launch development. Explicitly ask an agent to
implement the accepted issue when you want work to start.

Agents choose implementation details within the agreement. Foundational changes
receive an independent architectural review before approval. New tradeoffs,
expanded scope, unresolved review concerns, and missing required validation
return to the same issue, where humans steer the next step.

Use familiar labels for the kind of contribution: `bug`, `enhancement`,
`documentation`, or `question`. `help wanted` and `good first issue` can invite
reproduction, investigation, or design participation as well as implementation.
Closed issues and linked PRs provide completion and delivery status.

## Keep work meaningful

An issue represents an independently acceptable feature or engineering outcome.
Tests, file edits, and reviewer corrections are part of that outcome, not separate
administrative tickets. Split only when another outcome can be accepted or deferred
independently. A PR delivers a feature across whichever layers it requires.

Milestones describe shared sprint goals, with Outcome, Done when, and Not included
boundaries. They can span working sessions and are separate from releases.
Investigate unresolved design questions in the issue; contributors need no
special planning tools.

## Implementation and merge

Once invoked, the implementing agent owns the change through PR submission, independent review,
and merge. Agents verify the agreed acceptance criteria, resolve review findings,
and pass required checks before merging; no final human sign-off is required.
The PR records the result and evidence. Humans return to the discussion when
scope or design decisions change or a concern cannot be resolved within the
agreement. Release workflow design remains open and separate from this authorization.

Agents follow [AGENTS.md](AGENTS.md) and the repository development skill.
[ARCHITECTURE.md](ARCHITECTURE.md) explains the technical ownership model.
