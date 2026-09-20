# Pull request

For invoked, approved work, carry the PR through merge within your assigned
scope. Respect local-only or publication constraints attached to the task.

Prepare commits according to AGENTS.md, fetch the current target, and rebase.
Rerun checks affected by conflict resolution. Run
`bun run commitlint --from <base> --to HEAD` over the final commit range; rebasing
can rewrite messages without running the commit hook. Create or update the PR against
`main` with a concise summary, related issue and approval, validation results,
and independent review evidence. Keep incomplete work in draft.

Resolve in-scope review findings and check failures, fold corrections into their
owning commits, and obtain review of substantive changes. Bring unresolved
scope or design decisions back to the issue discussion.

## Definition of done

Before merging, verify:

- The approved outcome and acceptance criteria are met; documentation matches.
- Relevant local validation and required PR checks pass for the final revision.
- Independent review is complete and blockers are resolved.
- Commitlint passes; review confirms meaningful commit scope and truthful attribution.
- The PR is current with the target and repository merge requirements are met.

Merge with rebase to preserve the curated commits, using the reviewed head SHA
when the merge tool supports it. If the head or target changes, reassess affected
checks before retrying. Respect branch protection; an unavailable permission
or required approval is a blocker, not a reason to bypass it.

Verify the merge, close the related issue only if its outcome is complete, and
report the PR URL and result. Release promotion is outside this phase.
