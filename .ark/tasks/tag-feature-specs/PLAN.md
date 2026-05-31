# PLAN: tag feature SPECs with actuators

> Standard tier — single PLAN. No REVIEW loop. Fill every section.

## Summary

Tag the `[**Constraints**]` bullets of all 14 feature SPECs under `.ark/specs/features/` with inline
actuator tags `⟨@<kind>: <arg>⟩`, deriving each from the source task PLAN's `[**Acceptance Mapping**]`
as a *pointer*, then grep-confirming the real covering test fn / lint in current `crates/` before
writing the tag. Doc-only edits; no code changes. A `/ark:spec-audit` baseline opens the work and a
clean audit closes it.

## Spec

> This section is the durable contract. On standard tier it is **not** auto-promoted, but write it
> as if it were — it is the reference for VERIFY.

[**Goals**]

- G-1: Every concretely-enforceable feature-SPEC constraint carries a well-formed actuator tag.
- G-2: Each `test-binding` / `tool` / `source-scan` arg resolves to a real enforcer in current code.
- G-3: Constraints with no concrete enforcer stay untagged (judgment-by-default), never fabricated.
- G-4: The `/ark:spec-audit` skill is dogfooded: baseline report before, clean report after.

[**Non-goals**]

- NG-1: No changes to the actuator grammar, the spec-audit skill, or any `crates/` code.
- NG-2: No tagging of project SPECs (already tagged by the spec-actuators task) — features only.
- NG-3: No new tests authored; this task binds to existing tests, it does not write them.

[**Constraints**]

- C-1: Tags are added only to `[**Constraints**]` bullets in `.ark/specs/features/*/SPEC.md`; no other file changes.
- C-2: A tag's arg is derived from the source PLAN's Acceptance Mapping, then the cited concept is grep-confirmed to a test fn that exists in `crates/` at HEAD; on mismatch, rebind to the real fn or downgrade to untagged.
- C-3: `test-binding` arg is a real `#[test] fn` name; `tool` arg is a runnable command (e.g. `clippy`); `source-scan` arg is `<pattern> @ <glob>`.
- C-4: No `V-*` / `C-N` / `G-N` workflow label appears in any tag arg.
- C-5: A constraint whose PLAN cites no concrete test, or whose cited test no longer exists with no equivalent, is left untagged.
- C-6: Each tag ends the constraint bullet's first line; the bullet's prose and `C-N` id are unchanged.
- C-7: `cargo test --workspace` and `cargo fmt --all -- --check` stay green after the edits.
- C-8: A post-tag `/ark:spec-audit` reports zero malformed and zero mismatch tags across feature SPECs.

## Runtime

N/A — pure-doc task. No runtime behavior, no error paths in shipped code.

## Implementation

Per-SPEC, the same pipeline (14 SPECs; `ark-context` and `ark-upgrade` each have two source tasks —
merge both Acceptance Mappings). SPEC → source PLAN map:

| Feature SPEC | Source PLAN(s) |
|---|---|
| ark-agent-namespace | archive/2026-04/ark-agent-namespace |
| ark-context | archive/2026-04/ark-context, archive/2026-04/improve-ark-context |
| ark-research | archive/2026-05/ark-research |
| ark-sandbox | tasks/ark-sandbox |
| ark-upgrade | archive/2026-04/ark-upgrade, tasks/improve-ark-upgrade |
| codex-support | archive/2026-04/codex-support |
| detachable-feature-spec | archive/2026-04/detachable-feature-spec |
| opencode-support | archive/2026-04/opencode-support |
| project-spec | archive/2026-04/project-spec |
| subagent-support | archive/2026-05/subagent-support |
| task-concurrency-control | archive/2026-05/task-concurrency-control |
| workspace | archive/2026-05/workspace |
| worktree | archive/2026-04/worktree-support |

**Phase 1 — Baseline audit.** Run `/ark:spec-audit` read-only across `.ark/specs/features/`; record
per-SPEC untagged/malformed/mismatch counts as the starting point (write into VERIFY later).

**Phase 2 — Derive + bind, per SPEC.** For each constraint `C-N`:
1. Read its prose. Find the matching row in the source PLAN's Acceptance Mapping.
2. Pick the kind: `tool` if a lint/format/build covers it; `source-scan` if it's a "no X literal in Y"
   rule; `test-binding` if a named test asserts it; else leave untagged.
3. For `test-binding`: take the PLAN's cited fn as a pointer, then `grep -rn 'fn <name>' crates/`. If
   absent, locate the real covering test by reading the module's `#[cfg(test)]` block and rebind to its
   actual name; if none exists, leave untagged (C-5). The PLAN's names are illustrative — the live
   test name wins (verified: e.g. ark-agent-namespace PLAN says `commit_is_atomic`, real fn is
   `commit_rolls_back_task_toml_on_git_failure`).
4. Append `⟨@<kind>: <arg>⟩` to the end of the bullet's first line.

**Phase 3 — Verify.** `cargo test --workspace`, `cargo fmt --all -- --check`; re-run `/ark:spec-audit`
and confirm zero malformed/mismatch. Capture the before/after counts in VERIFY.

## Validation

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1  | V-M-1: every feature SPEC's concretely-enforceable constraints carry a tag (manual count vs. baseline audit). |
| G-2  | V-G-1: `grep -rn 'fn <name>' crates/` resolves for every `test-binding` arg; every `tool` arg is a runnable command. |
| G-3  | V-M-2: each untagged constraint is justified (PLAN cites no enforcer / no live test). |
| G-4  | V-C-1: `/ark:spec-audit` post-run reports 0 malformed + 0 mismatch (C-8). |
| —    | V-UT-1: `cargo test --workspace` green; V-F-1: `cargo fmt --all -- --check` clean (C-7). |
| —    | V-S-1: `git diff` touches only `.ark/specs/features/*/SPEC.md` (C-1); no `V-*`/`C-N`/`G-N` in any arg (C-4). |

## Trade-offs

- **PLAN-as-pointer, code-as-truth:** the PLAN's cited test names have drifted from current code, so a
  literal copy would produce dead bindings — the exact failure actuators exist to prevent. Binding is
  therefore grep-confirmed against HEAD, accepting the slower per-constraint pass for correct tags.
- **Untagged over fabricated:** leaving genuinely-judgment constraints untagged is preferred to inventing
  a weak proxy; honesty of the audit signal beats coverage percentage.
