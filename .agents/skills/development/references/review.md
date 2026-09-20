# Review

Read the proposal and discussion, or the approved scope for an implementation,
and [ARCHITECTURE.md](../../../../ARCHITECTURE.md).

## RFC review

Assess the outcome, scope, changed guarantees, compatibility, failure modes,
alternatives, and whether acceptance evidence tests the owning layers.
Return one of these outcomes to the agent handling the issue, or to the user
when directly assigned a review:

- **Needs investigation:** gather missing facts from code, upstream, or tests.
- **Needs human decision:** state the unresolved tradeoff, consequences, options,
  and recommendation. Routine implementation uncertainty is research.
- **Ready for acceptance:** scope and evidence are concrete and architectural
  findings are resolved. Recommend acceptance and await maintainer approval.

The agent handling the issue records the outcome in the existing discussion,
keeping `rfc` until a maintainer explicitly approves the reviewed scope, then
replacing it with `accepted`. A clean review is not human acceptance;
implementation still requires invocation.

## Implementation review

Inspect the diff and affected callers against the approved scope; use
[implementation](implementation.md) when evaluating platform evidence and docs.
Check whether existing documentation becomes inaccurate, rather than requiring
new docs for every change. Check commit scope, count, linear history, and truthful
attribution against AGENTS.md.

Report findings without editing the implementation. Each blocker needs a concrete
failure scenario, source location, and missing correction or evidence. Separate
optional suggestions. Return the reviewed revision, checks, and limitations;
state explicitly when no blockers were found. Verify resolved blockers and
substantive changes since the reviewed revision before completing review.
Unresolved decisions or unavailable required validation return to the issue for
human steering; disclosure alone does not make the change ready.
