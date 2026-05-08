# `extract-spec-cmd` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `extract-spec-cmd`
> Target Task: `extract-spec-cmd`
> Tier: standard

---

## Project Spec Compliance

- [x] LAYOUT.md: N/A — Layout A governs convention SPECs under `specs/project/`. This task adds no convention SPEC; the runtime-system template (`.ark/templates/SPEC.md`) governs the *output* of `spec import`, which is unaffected here.
- [x] rust/COMMENTS.md: PASS — `import.rs` doc-comments use third-person summaries, single-sentence first lines, `# Errors` on `spec_import` (C-3, C-4, C-13). No task-mark labels in source (C-23). One inline `// SAFETY`-shaped why-comment on `sanitize_field`'s duplication (C-7). Concision over completeness throughout (C-10). Spot check: every `///` summary opens with `Imports`, `Validates`, `Composes`, `Locates`, `Inserts`, `Sentinel` — verb-led per C-21.
- [x] rust/STYLE.md: PASS — `cargo fmt --check` clean (S-25). Imports grouped + version-sorted (S-13). Newtype-style summary structs with private fields would be ideal, but `SpecImportOptions` follows the existing pub-fields-Options-struct convention used by `SpecRegisterOptions` and `SpecExtractOptions` — S-21 carve-out for "intentionally-transparent types" applies (a clap-shaped options bag is not a domain type with invariants).
- [x] rust/ERRORS.md: PASS — `Error::SpecAlreadyExists { feature: String, path: PathBuf }` carries context per E-15 (no bare `#[from]`). `Display` is lowercase, no trailing punctuation per E-9. `?` propagation throughout, no `unwrap()` in non-test code (E-7). Single canonical `Error` enum unchanged (E-2). All filesystem access at boundaries returns `Result` (E-10).

## Related Feature Spec Compliance

- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — new `Spec::Import` verb is added under hidden `ark agent` (G-1, C-1). `Display` summary is one stdout write (G-5, C-3). All filesystem access routes through `io::PathExt` (C-4). All `.ark/`-relative paths via `Layout` (C-5). INDEX upsert uses `update_managed_block` with `ARK:FEATURES` marker via the shared `upsert_index_row` helper (C-7). `Error::SpecAlreadyExists` is new and slots into the existing pattern. **No SPEC amendment needed**: the SPEC's listed `spec` verbs (lines 168–169) only enumerated `extract` / `register`, but the SPEC's Goals don't enumerate verbs — G-4 says "Feature-SPEC ops (`spec extract` / `spec register`)", which is a non-exhaustive example. Adding `import` does not contradict the SPEC contract; the `## API Surface` listing is illustrative, not enumerative. Reviewer should confirm.
- [x] specs/features/project-spec/SPEC.md: N/A — project-spec governs the convention layer (`specs/project/`), not the feature layer. Output of `spec import` writes to `specs/features/`, not `specs/project/`.
- [x] specs/features/ark-context/SPEC.md: N/A — `spec import` is invoked directly by the slash command and does not consume a context envelope (it operates outside the design/plan/review/execute/verify lifecycle).

## PRD Constraints

- [x] `.ark/specs/features/copy-on-write/SPEC.md` body matches feature-SPEC template: PASS — Phase 5 smoke verified body wrote verbatim with the authored `[**Goals**]`, `[**Non-goals**]`, `[**Architecture**]`, `[**Data Structure**]`, `[**API Surface**]`, `[**Constraints**]` sections, and `spec import` did not parse or restructure them (it only spliced the CHANGELOG). Smoke target was `spec-extract-smoke` rather than `copy-on-write` (per PLAN Phase 5), but the path shape is identical.
- [x] CHANGELOG entry stamped on extracted SPEC: PASS — smoke artifact's tail showed `` - `2026-05-08` `extracted`: initial extraction from codebase at `e169239`. ``. Format matches PLAN's C-5.
- [x] INDEX row registered with `from-task = "extracted"`: PASS — smoke artifact's INDEX line: `` | `spec-extract-smoke` | PLAN-based feature SPEC promotion (smoke) | 2026-05-08 from task `extracted` | ``. Sentinel preserved per PLAN's C-7.

## Plan Fidelity

- [x] G-1: `ark agent spec import` writes a registered feature SPEC: PASS — V-IT-1, V-IT-2, V-F-1 all green; smoke test end-to-end successful.
- [x] G-2: `/ark:extract-spec` drives discover → confirm → synthesize → import: PASS in shape — slash command published in three platforms with full flow doc. **Caveat:** the discover→confirm→synthesize phases are AI-side narrative, not code; their *fidelity* depends on the AI executing the skill correctly. The smoke test exercised Phase 4 (Import) directly; Phases 1–3 are exercised the next time a user invokes `/ark:extract-spec`. This is intrinsic to a slash command — no test in this repo can verify the AI follows the skill body.
- [x] G-3: CHANGELOG provenance entry: PASS — V-UT-1, V-UT-2, V-IT-1; smoke confirmed.
- [x] G-4: Shared managed-block path: PASS — `upsert_index_row` factored out of `spec_register` and called by both `spec_register` and `spec_import` (V-IT-3 is the parity test `register_then_import_preserves_existing_row`). The two call sites pass the same args in the same order with the same date type; only `from_task` differs.
- [x] G-5: Three-platform parity: PASS — `every_claude_command_has_a_codex_skill_sibling` and `every_claude_command_has_an_opencode_command_sibling` both pass with the new template added. Materialized copies under `.claude/` and `.opencode/` exist; codex skill exists under `templates/codex/skills/ark-extract-spec/SKILL.md`. Live skill list shows `ark:extract-spec` (the materialized copy is being read by the harness).

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no existing feature SPEC was modified by this task. The new path *writes* feature SPECs but does not amend any pre-existing one. (`spec import` will eventually amend SPECs in some flow, but per PLAN's NG-1/NG-3 and the `SpecAlreadyExists` refusal, it does not in this task.)

## Findings

### V-001 `sanitize_field` duplicated between `register.rs` and `import.rs`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/spec/register.rs:98` and `crates/ark-core/src/commands/agent/spec/import.rs:113`
- **Problem:** The `sanitize_field` helper is byte-identical in both files (16 lines each, same trim-then-reject pattern for empty/`|`/newline inputs). The `import.rs` doc-comment explicitly notes the duplication as deliberate ("reproduced here to keep the import path independent of `register`'s private helpers"), but PLAN C-10 calls for shared helpers between `register` and `import`, and `upsert_index_row` already crosses that boundary.
- **Why it matters:** Drift risk — if `register::sanitize_field` evolves (e.g. adds rejection of `\t`, or surfaces a different `Error` variant) and `import::sanitize_field` doesn't, the two import paths produce subtly different validation. The "independence" rationale in the comment is weak: `import` already imports `upsert_index_row` from `register`, so the boundary is already crossed.
- **Recommendation:** Promote `sanitize_field` to `pub(crate) fn sanitize_field` in `register.rs` (or a small `spec/util.rs`), import it from `import.rs`, delete the duplicate. ~10 LOC delta. Test coverage stays the same (`sanitize_rejects_pipe_and_empty` in `import.rs` still validates the `import.rs` callers do call into the validator).
- **Resolution:** FIXED — `sanitize_field` promoted to `pub(crate) fn sanitize_table_field` in `register.rs`; `import.rs` imports and calls it; the duplicate body and its standalone test are deleted (the existing `sanitize_rejects_pipe_and_empty` test now exercises the shared helper). Verified clean: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --workspace` all green (418 tests pass).

### V-002 `ark agent spec` SPEC's `## API Surface` does not list `import`

- **Severity:** LOW
- **Location:** `.ark/specs/features/ark-agent-namespace/SPEC.md:168–169`
- **Problem:** The feature SPEC's `[**API Surface**]` block enumerates `ark agent spec extract` and `ark agent spec register` but not `ark agent spec import`. The G-4 goal line names extract and register as examples ("Feature-SPEC ops (`spec extract` / `spec register`)"), and no Constraint freezes the verb set. So adding `import` does not contradict the SPEC, but the API Surface block is now incomplete.
- **Why it matters:** Future readers consulting `ark-agent-namespace/SPEC.md` for the full verb inventory will miss `import`. The SPEC's CHANGELOG convention (deep-tier amendments) is the right tool, but this task is standard-tier and the namespace SPEC is a deep-tier-promoted artifact — amending it is awkward without a deep-tier task.
- **Recommendation:** Either (a) add a one-line CHANGELOG entry to `ark-agent-namespace/SPEC.md` noting the new verb (low-friction, mirrors how the SPEC's existing CHANGELOG entries handle small additions like the `2026-05-06` `drop-task-slug` entry), or (b) accept and let the next deep-tier task that touches the namespace fold it in. Reviewer's call.
- **Resolution:** FIXED — added a CHANGELOG entry to `.ark/specs/features/ark-agent-namespace/SPEC.md` under date `2026-05-08` for `extract-spec-cmd`, naming the new verb, the shared helper, the sentinel, and the new `Error::SpecAlreadyExists` variant. Mirrors the prior `drop-task-slug` entry's style.

### V-003 `--from-commit` required vs optional

- **Severity:** LOW
- **Location:** `crates/ark-cli/src/agent_cli.rs:286–293` (CliArgs definition)
- **Problem:** PLAN initially specified `--from-commit` as optional (defaulting to `git rev-parse --short HEAD`), but during EXECUTE I switched to required after discovering `io::git` is `pub(crate)` and re-implementing the lookup in `ark-cli` would have duplicated `run_git`. The PLAN's `[**API Surface**]` and `## Spec` were not updated to reflect this — they still say "defaults: `git rev-parse HEAD` short SHA".
- **Why it matters:** PLAN/code drift. Per the EXECUTE-phase contract ("If implementation reveals design gaps, **update the latest PLAN's `## Spec`**"), I should have updated PLAN.md when the design changed. I did not.
- **Recommendation:** Edit PLAN.md's `[**API Surface**]` block: change `[--from-commit <sha>]   # defaults: git rev-parse HEAD short SHA` to `--from-commit <sha>` (required), and update the slash command flow note.
- **Resolution:** FIXED — updated PLAN.md's `[**API Surface**]` to mark `--from-commit` as required with a short rationale paragraph; updated `## Runtime` step 5 (now "Validate `--from-commit` (required, sanitized via `sanitize_table_field`)" rather than "Resolve … default"); updated `## Implementation` Phase 3 bullet to drop the defaulting language; rewrote V-E-1 to reflect that clap rejects missing required args before logic runs.

## Notes

**Smoke test cleanup:** Phase 5 ran `ark agent spec import` against the live repo to produce `.ark/specs/features/spec-extract-smoke/`, then `rm -rf` and `git checkout INDEX.md` to revert. Tree was clean after cleanup; no smoke artifacts persist in the diff.

**`ROADMAP.md` and `reference/`:** SessionStart listed both as dirty before this task started. They are unrelated to this work and remain unstaged — the user is the one who edited `ROADMAP.md` (with notes about the multi-active-tasks issue we hit during PLAN→EXECUTE) and added `reference/`. Do not include either in the commit for this task.

**Active-task warning:** Both `task plan` and `task execute` failed with `multiple active tasks` until I called `task resume --slug extract-spec-cmd` first. The other 7 actives are all in `phase = committed` (not yet archived). The topology cascade currently treats committed-but-not-archived tasks as "active" for ambiguity-detection purposes. Worth a follow-up task to either auto-resume on `task new` or exclude committed tasks from the cascade — out of scope here.

**Test coverage:** Added 8 unit/integration tests (3 unit, 5 integration). Total workspace test count rose from 410 to 418 (8 new) — counted via `test result: ok. 418 passed`. Per-module: `commands::agent::spec::import` now has 8 tests; `commands::agent::spec::register` retains its 7 (parity preserved by the refactor).
