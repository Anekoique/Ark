# PRD: tag feature SPECs with actuators

> Standard tier — single PRD. Keep it tight.

## What

Add inline actuator tags `⟨@<kind>: <arg>⟩` to the `[**Constraints**]` bullets of Ark's 14
feature SPECs under `.ark/specs/features/`, deriving each tag from the source task's PLAN
`[**Acceptance Mapping**]` → the real test fn / lint that already covers it. Begin with a
read-only `/ark:spec-audit` baseline, then close the gaps it reports. Repo-internal SPECs only;
nothing ships to users.

## Why

The `spec-actuators` mechanism (committed `53b6c79`) lets a constraint declare how it is enforced,
but Ark's own feature SPECs are still entirely untagged — so the dogfooding is half-done and the
audit reports ~all feature constraints as untagged. Binding each constraint to a concrete enforcer
turns silent prose into checkable contracts and exercises `/ark:spec-audit` on a real corpus.
Memory: `project_tag_feature_specs_pending`.

## Outcome

- A `/ark:spec-audit` baseline report (counts untagged/malformed/mismatch per feature SPEC) is captured.
- Every feature SPEC constraint that maps to a concrete test/lint in its source PLAN carries a
  well-formed tag (`tool` / `source-scan` / `test-binding`).
- Each `test-binding` arg names a test fn that **currently exists** in `crates/` (grep-verified — no drift).
- Constraints with no concrete enforcer in the PLAN are left untagged (judgment-by-default, not fabricated).
- No `V-*` / `C-N` / `G-N` workflow label appears in any tag arg.
- `cargo test --workspace` stays green (SPEC edits are doc-only; no code changes expected).
- A post-tag `/ark:spec-audit` shows zero malformed/mismatch tags.

## Related Specs

- All 14 feature SPECs are edited (constraint bullets tagged): `ark-agent-namespace`, `ark-context`,
  `ark-research`, `ark-sandbox`, `ark-upgrade`, `codex-support`, `detachable-feature-spec`,
  `opencode-support`, `project-spec`, `subagent-support`, `task-concurrency-control`, `workspace`, `worktree`.
- `spec-actuators` (committed this branch) — defines the tag grammar and the `/ark:spec-audit` skill this task uses; this task is its first real application. No grammar change.
