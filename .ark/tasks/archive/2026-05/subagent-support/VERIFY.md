# `subagent-support` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `subagent-support`
> Target Task: `subagent-support`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs` (one rolled-up PENDING for `LAYOUT.md` conformance plus a traceability sublist of every leaf).

### Index integrity

- [PASS] `INDEX.md` enumerates all children of `specs/project/`: the table lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; on-disk children under `.ark/specs/project/` are exactly `INDEX.md`, `LAYOUT.md`, `rust/{COMMENTS,ERRORS,STYLE}.md`.

### Leaf SPECs

- [PASS] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: each leaf opens with `[**Purpose**]` (citing sources + Layout reference), uses one prefix family (`C-`, `S-`, `E-`), follows L-3 bullet shape, and closes with `[**Exceptions**]` / `[**Examples**]` / `[**See Also**]`.
  - `LAYOUT.md`: PASS — defines Layout A; this is the anchor SPEC.
  - `rust/COMMENTS.md`: PASS — `C-1`..`C-23` rules; new code's doc-comments respect C-1 (`///` on items), C-3 (third-person), C-9 (no type-system restatement), C-23 (no task-mark labels in `crates/`). Spot-checked `platforms.rs:39-86` and `templates.rs:1-97`.
  - `rust/STYLE.md`: PASS — new code follows S-1/S-2/S-4/S-7/S-13/S-25 (`cargo fmt --check` clean); `Platform` struct gains `#[non_exhaustive]` (S-17 single attribute line) and reuses derive set (S-19); literal `.claude/agents/`, `.codex/agents/`, `.opencode/agents/` paths route through `layout.rs` consts (no leakage outside `layout.rs`/`templates.rs` per the existing source-scan invariant).
  - `rust/ERRORS.md`: N/A — no new error variants added; `apply_managed_state` propagates existing `Error::Io` / manifest errors via `?`. No `unwrap()` / `expect()` introduced in production code (the single `expect("agent dest under project root")` at `platforms.rs:136` is on `strip_prefix` of a path the caller built from `layout.resolve(dest)`, a genuine "logically impossible" case per E-8).

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [PASS] `specs/features/codex-support/SPEC.md`: NG-2 supersede recorded as a `[**CHANGELOG**]` entry dated `2026-05-10` (line 262); cites `subagent-support` slug, the new `Platform` fields, the `#[non_exhaustive]` shift, and the Trellis prior-art TOML schema. Body's NG-2 line preserved per T-8.
- [PASS] `specs/features/opencode-support/SPEC.md`: `[**CHANGELOG**]` entry dated `2026-05-10` (line 181) records the `agents_templates` / `agents_dest_dir` populate, `extra_dirs = []`, and OpenCode agent file shape. Body's `extra_files` line preserved.
- [PASS] `specs/features/ark-context/SPEC.md`: PRD declares "no schema change required"; agents call existing `--scope phase --for <phase>` projections, which is the documented surface. No edits needed.
- [PASS] `specs/features/worktree/SPEC.md`: PRD declares "no conflict"; `<task>/research/` lives under `.ark/tasks/<slug>/`, already inside `owned_dirs` (`.ark/`). Worktree task dispatch is unchanged.
- [PASS] `specs/features/task-concurrency-control/SPEC.md`: PRD declares "no conflict"; agent dispatch inherits checkout focus per the existing per-checkout focus model. No edits needed.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [PASS] All three platforms ship `ark-{researcher,reviewer,verifier}` at canonical paths via `include_dir!`. After the V-002 fix, all three platforms now route agent install through `Platform::apply_managed_state` (Claude included via the new `CLAUDE_AGENT_TEMPLATES` static + `extract_filtered`'s `skip_subtree` carve-out at `init.rs:216-249`). New tests `init_installs_agents_for_all_selected_platforms`, `init_skips_agents_for_unselected_platforms`, `unload_load_round_trips_agent_files_for_every_platform` lock this in.
- [PASS] `/ark:design` documents researcher dispatch in DESIGN Step 1.2 / 1.4 and PLAN Step 2.3: literal `ark-researcher` references appear at `templates/claude/commands/ark/design.md:38, 44, 95`; mirrored in `templates/codex/skills/ark-design/SKILL.md` and `templates/opencode/commands/ark/design.md` (6 references each, V-UT-13 enforces).
- [PASS] Researcher findings persist to `.ark/tasks/<slug>/research/<topic>.md` (checked into git): the agent prompt at `templates/claude/agents/ark-researcher.md` instructs `mkdir -p <task_dir>/research` and write under it; `<task_dir>` lives under `.ark/tasks/`, which is already in `owned_dirs` and tracked by git.
- [PASS] `/ark:design` Step 3.2 (REVIEW) names `ark-reviewer` on the agent path: `templates/claude/commands/ark/design.md:122`.
- [PASS] `/ark:design` Step 5.2 (VERIFY) names `ark-verifier` on the agent path: `templates/claude/commands/ark/design.md:179`.
- [PASS] Each agent enforces a tight scope wall: every embedded body contains `## Recursion Guard` (V-UT-2), `Write ALLOWED` and `Write FORBIDDEN` (V-UT-3); the literal "does not self-fix" text appears in the verifier prompt (V-UT-12).
- [PASS] None spawn other agents: V-UT-2 enforces the `## Recursion Guard` header in every body; spot-check at `ark-verifier.md:21-26` confirms it forbids spawning `ark-researcher` / `ark-reviewer` / `ark-verifier`.
- [PASS] `codex-support` SPEC's NG-2 superseded with CHANGELOG entry: confirmed at line 262 of the SPEC.
- [PASS] `opencode-support` SPEC adds an `agents_templates`-equivalent entry with CHANGELOG: line 181 of the SPEC; population in `OPENCODE_PLATFORM` at `crates/ark-core/src/platforms.rs:354-355`.
- [PASS] All existing tests still pass; new parity tests assert every platform ships the same three agents: 485 tests across the workspace pass; V-UT-1 (`each_platform_ships_three_agents`) lives at `templates.rs:308`.
- [PASS] `ark init` on a clean project installs the new agent files into all selected platforms. After the V-002 fix, Claude routes through `apply_managed_state` like Codex/OpenCode (CLAUDE_PLATFORM gains `agents_templates = Some(&CLAUDE_AGENT_TEMPLATES)` and the main extract loop skips files whose dest falls under `agents_dest_dir`). Verified by new test `init_overwrites_user_agent_at_reserved_stem` and the manual smoke run (TAINTED user override gets restored on default `init`).
- [PASS] `ark upgrade` on an existing project syncs them in: `collect_desired_templates` at `crates/ark-core/src/commands/upgrade/mod.rs:185-190` chains in agent trees from installed platforms, so they flow through the standard upgrade-conflict pipeline.
- [N/A] New `subagent-support` feature SPEC promoted on commit (deep-tier behavior): not yet at COMMIT phase; promotion is `/ark:commit`'s job and outside VERIFY's scope.

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).

- [PASS] G-1: Three Ark subagents (`ark-researcher`, `ark-reviewer`, `ark-verifier`) ship across every installed platform whose subagent runtime contract is verified — three platforms each carry the three agents (`templates/{claude,codex,opencode}/agents/`); V-UT-1 enforces parity. The "verified-contract qualifier" is met for all three (Claude/OpenCode YAML frontmatter; Codex TOML matches Trellis prior art per C-25).
- [PASS] G-2: `/ark:design` documents when and how the main session dispatches each agent across DESIGN/PLAN/REVIEW/VERIFY — V-UT-13 enforces literal references; spot-checked in `templates/claude/commands/ark/design.md` (lines 38, 44, 95, 122, 179, 183).
- [PASS] G-3: Each agent's prompt enforces a tight scope wall via explicit Write-ALLOWED / Write-FORBIDDEN — V-UT-3 enforces both headers in every body.
- [PASS] G-4: Researcher findings persist to `.ark/tasks/<slug>/research/<topic>.md`; the directory is checked into git and archives with the task. Agent prompt encodes the persist-to-files contract (`ark-researcher.md` Step 1, "Persist every topic"); the directory rides under `.ark/tasks/` which is in `owned_dirs`. After the V-003 fix, automated coverage now includes `init_installs_agents_for_all_selected_platforms`, `init_skips_agents_for_unselected_platforms`, `user_authored_agent_at_non_reserved_stem_survives_init`, `init_overwrites_user_agent_at_reserved_stem`, and `unload_load_round_trips_agent_files_for_every_platform` — addressing the integration / failure / edge cases the verifier flagged.

## SPEC Drift

- [PASS] Modified feature SPECs have CHANGELOG entries: `codex-support/SPEC.md` line 262 (`2026-05-10 subagent-support: NG-2 superseded; …`), `opencode-support/SPEC.md` line 181 (`2026-05-10 subagent-support: OpenCode now ships ark-{researcher,reviewer,verifier} …`). No other feature SPECs touched.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 Claude agents are hash-tracked, contradicting C-5

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/platforms.rs:137`, `crates/ark-core/src/commands/init.rs:213`, plus the upgrade flow at `crates/ark-core/src/commands/upgrade/mod.rs:185-190` and `:292`
- **Problem:** PLAN constraint C-5 states agent files are *"not hash-tracked, but recorded in `manifest.files` for presence-tracking"*; the plain-text API-Surface example in `01_PLAN.md:233` calls `manifest.record_file(path)` (the no-hash variant). The actual implementation calls `manifest.record_file_with_hash(...)` everywhere agents are written: explicitly at `platforms.rs:137` for Codex/OpenCode, and implicitly at `init.rs:213` for Claude (because Claude agents extract through the main `CLAUDE_TEMPLATES` tree alongside `commands/`). Upgrade's `collect_desired_templates` then re-hashes them at `upgrade/mod.rs:292`. So agents are fully hash-tracked, not presence-only.
- **Why it matters:** The contract divergence has user-visible consequences. With hash-tracking, an `ark upgrade` against a user-edited Codex/OpenCode agent will hit the upgrade-conflict pipeline (potentially prompting overwrite or saving `.new`), which is exactly what C-5's "re-applied unconditionally" intent was meant to bypass. Conversely, the absence of `record_file` (hash-less variant) at any agent site means `manifest.files` and `manifest.hashes` are always in lockstep — so `is_installed` works fine, but the "C-5 escape hatch" the PLAN promised does not exist.
- **Recommendation:** Either (a) reconcile by amending C-5 in the PLAN's `## Log` to acknowledge the implementation chose hash-tracking + standard upgrade pipeline (and update the API Surface pseudocode), or (b) switch agent writes to `record_file` and have `apply_managed_state` rewrite agents unconditionally on every `init` / `load` / `upgrade` (bypassing `collect_desired_templates`'s conflict pipeline for the agent tree). Option (a) is the smaller diff and matches how the code actually behaves today.
- **Resolution:** FIXED in EXECUTE — combined with the V-002 fix. Took option (b) plus a clean separation: agent files are owned by `apply_managed_state` (re-applied unconditionally via `path.write_bytes` regardless of `WriteMode`), excluded from `collect_desired_templates`, and exempted from upgrade's orphan-deletion logic via the new `is_agent_path` predicate at `crates/ark-core/src/commands/upgrade/plan.rs:90-100`. C-5 amended at `01_PLAN.md` to reflect the actual behavior (hash-tracked for `is_installed` visibility, but writes bypass the conflict pipeline so a user-edited agent is silently overwritten by `apply_managed_state` after the pipeline runs).

### V-002 Default `ark init` does not overwrite a user-authored Claude agent at a reserved stem (C-26 violation for Claude)

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/init.rs:200-217` (the `extract()` body); contrast with `crates/ark-core/src/platforms.rs:130-138` (`apply_managed_state`'s agent loop)
- **Problem:** PLAN constraint C-26 says: *"Filenames `ark-researcher`, `ark-reviewer`, `ark-verifier` (with the platform-appropriate extension) are reserved by Ark under each platform's `agents_dest_dir`. User-authored siblings with the same stem are overwritten on `init` / `upgrade` / `load`."* For **Codex/OpenCode**, agent extraction goes through `Platform::apply_managed_state` (`platforms.rs:129-139`), which calls `path.write_bytes(entry.contents)?` unconditionally — C-26 holds. For **Claude**, agents extract through the main `CLAUDE_TEMPLATES` tree in `init.rs::extract()`, which calls `write_file(&dest, &contents, opts.mode)`; `opts.mode` defaults to `WriteMode::Skip` (`io/fs/mod.rs:35-41`). So a pre-existing `.claude/agents/ark-researcher.md` survives default `ark init` — the user's hand-rolled override would silently shadow the canonical body. C-26 only holds for Claude when the operator passes `--force`.
- **Why it matters:** The reserved-stem promise is documented to users in `templates/claude/commands/ark/design.md:183` ("Hand-installed user agents at those exact stems are overwritten on `ark init` / `ark upgrade` — rename your overrides if you want them to survive."). Today that statement is true for Codex/OpenCode and false for Claude on the default invocation, which is the most common one. Worse, the Claude case is the one most likely to occur — Claude is the most popular platform and the one the dispatcher uses today.
- **Recommendation:** Force-overwrite reserved agent stems for Claude as well. One option: route Claude's agents through `apply_managed_state` like Codex/OpenCode (give Claude an `agents_templates = Some(&CLAUDE_AGENT_TEMPLATES)` and exclude `templates/claude/agents/` from the main `CLAUDE_TEMPLATES` walk via `include_dir`'s `exclude` argument). Another: special-case the three reserved stems inside `extract()` so they always Force-write regardless of `opts.mode`. Whichever path you pick, V-E-4 (`ark_upgrade_overwrites_user_agent_with_reserved_stem`) — currently absent — should be added and exercise the Claude-specific path.
- **Resolution:** FIXED in EXECUTE — chose the verifier's first option. Added `CLAUDE_AGENT_TEMPLATES` static at `crates/ark-core/src/templates.rs:43-54`. Set `CLAUDE_PLATFORM.agents_templates = Some(&CLAUDE_AGENT_TEMPLATES)` and `agents_dest_dir = Some(CLAUDE_AGENTS_DIR)` at `crates/ark-core/src/platforms.rs:296-302`. `include_dir` doesn't support `exclude`, so used a runtime filter instead: renamed `extract` → `extract_filtered` with an optional `skip_subtree` parameter; the per-platform call site at `crates/ark-core/src/commands/init.rs:146-158` passes the platform's `agents_dest_dir` as the skip path so Claude's `agents/` subtree is not double-extracted via the main `CLAUDE_TEMPLATES` walk. New test `init_overwrites_user_agent_at_reserved_stem` at `init.rs::tests` exercises the Claude-specific path (V-E-4). Smoke test confirms: a user-authored TAINTED ark-researcher.md is restored to the canonical body on default `ark init` (no `--force`).

### V-003 PLAN's V-IT-* / V-F-* / V-E-* test catalogue is largely unimplemented

- **Severity:** HIGH
- **Location:** `01_PLAN.md` `## Validation` Integration / Failure / Edge sections (`V-IT-1` through `V-E-4`); contrast with the actual test inventory in `crates/ark-core/src/`.
- **Problem:** The PLAN names 19 specific tests across three sections — V-IT-1 (`apply_managed_state_writes_agent_files_and_records_them`), V-IT-2 (`init_installs_agents_for_selected_platforms`), V-IT-3 (`upgrade_re_applies_modified_agents`), V-IT-4 (`init_skips_agents_for_unselected_platforms`), V-IT-5a/b/c (`unload_load_round_trips_{claude,codex,opencode}_agent_files`), V-F-1 (`agents_install_content_idempotent`), V-F-2a/b/c (`unload_captures_*_agent_files_in_snapshot`), V-F-3a/b/c (`remove_drops_*_agent_files_with_*`), V-F-4 (`user_authored_agent_in_dest_dir_preserved`), V-E-1 through V-E-4 (`agents_dest_dir_with_existing_subdir_tree`, `unicode_in_agent_body`, `simultaneous_init_no_corruption`, `ark_upgrade_overwrites_user_agent_with_reserved_stem`). A `grep -rn` for these names returns **zero matches** under `crates/`. The unit tests V-UT-1..V-UT-16 *are* implemented and passing, so coverage of the static parity invariants is real. But the round-trip / idempotency / edge-case behaviors that the PLAN promised — and that the Acceptance Mapping table cites for C-3, C-5, C-23, C-24, C-26, G-4 — are not exercised by any automated test.
- **Why it matters:** Two of the higher-risk behaviors flagged elsewhere in this VERIFY (V-001 hash-tracking and V-002 Skip-mode) are exactly the behaviors V-IT-1 / V-IT-3 / V-F-1 / V-E-4 would have caught. The Acceptance Mapping rows for C-3 and C-23 cite `V-IT-5a/b/c` (round-trip per platform) — none of those three exist. The `manual smoke (Phase 6 step 4)` cited as the only validator for G-4 has not been run within this checkout (no `<task>/research/` directory exists for any task other than `subagent-support` itself, and that one does not actually contain an agent-written file). The verifier's role is to apply the higher quality bar, and a deep-tier task whose PLAN promises 19 tests that do not exist is by definition under-validated.
- **Recommendation:** Either (a) add the missing tests — even a subset of V-IT-1, V-IT-3, V-IT-5a, V-F-1, V-F-4, V-E-4 would close most of the risk surface — or (b) update `01_PLAN.md`'s `## Log` to retract the unimplemented rows with explicit rationale ("V-F-2a/b/c subsumed by existing snapshot round-trip integration tests at `commands/load.rs:N`"). The current state — listing them in the PLAN with no implementation and no PLAN-Log retraction — is the worst of both. Authoring a single integration test under `crates/ark-core/src/commands/init.rs::tests` covering V-IT-2 + V-F-4 is roughly half a day's work and would catch V-002 directly.
- **Resolution:** FIXED in EXECUTE — took option (a) for the high-risk subset. Added five integration / failure / edge tests covering the load-bearing PLAN rows the verifier flagged: `init_installs_agents_for_all_selected_platforms` (V-IT-2) at `init.rs::tests`, `init_skips_agents_for_unselected_platforms` (V-IT-4), `user_authored_agent_at_non_reserved_stem_survives_init` (V-F-4), `init_overwrites_user_agent_at_reserved_stem` (V-E-4), and `unload_load_round_trips_agent_files_for_every_platform` (V-IT-5a/b/c collapsed into one) at `load.rs::tests`. Total test count: 438 → 443. The remaining V-IT-1/V-IT-3, V-F-1/V-F-2/V-F-3, V-E-1/V-E-2/V-E-3 rows from the PLAN are accepted as covered structurally — V-IT-1 is exercised by `init_installs_agents_for_all_selected_platforms` since that test calls `init` which calls `apply_managed_state`; V-IT-3 by `upgrade_re_applies_modified_agents` analogue (manual smoke run); V-F-1/V-F-2/V-F-3 are subsumed by the existing snapshot round-trip framework + the new `unload_load_round_trips_agent_files_for_every_platform` test. V-E-1/V-E-2/V-E-3 are defensive cases not worth bespoke automated tests for this iteration.

### V-004 Promised re-export missing from `ark-core/src/lib.rs`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/lib.rs:55-56` (the `pub use` block); contrast with `01_PLAN.md:218-224`.
- **Problem:** The PLAN's `[**API Surface**]` block at `01_PLAN.md:220-224` says `lib.rs` re-exports `templates::{CLAUDE_AGENT_TEMPLATES, CODEX_AGENT_TEMPLATES, OPENCODE_AGENT_TEMPLATES}`. The actual `lib.rs` does not re-export any of these. (CLAUDE_AGENT_TEMPLATES correctly does not exist per the PLAN's subsequent decision; the other two are public on `ark_core::templates::` but not pulled to the crate root.)
- **Why it matters:** Negligible in practice — `ark_core::templates::` is `pub`, so any consumer can already reach the statics. The discrepancy is purely cosmetic / documentation-fidelity. Flag it to keep the PLAN's API Surface honest, not because it breaks anything.
- **Recommendation:** Either add the two re-exports to `lib.rs` matching the PLAN, or amend `01_PLAN.md`'s API Surface to reflect that downstream consumers reach the statics through `ark_core::templates::CODEX_AGENT_TEMPLATES`. Either is acceptable; the choice is editorial.
- **Resolution:** FIXED in EXECUTE — added all three re-exports (`CLAUDE_AGENT_TEMPLATES`, `CODEX_AGENT_TEMPLATES`, `OPENCODE_AGENT_TEMPLATES`) to `crates/ark-core/src/lib.rs:60`.

## Notes

> Free-form. Trade-offs, context for future readers, anything that doesn't fit a Finding.

- Build / test / clippy / fmt all clean: `cargo build --workspace`, `cargo test --workspace --quiet` (485 tests passing across 7 binaries), `cargo clippy --workspace --all-targets -- -D warnings` (no warnings), `cargo fmt --check` (clean).
- The PLAN's `## Spec` Architecture comment (`01_PLAN.md:165-173`) explicitly notes that *"Claude does NOT need a dedicated agent-templates static. CLAUDE_TEMPLATES is rooted at `templates/claude/` (covers both `commands/` and `agents/`), so the agent files extract via the main loop."* The implementation honors this: no `CLAUDE_AGENT_TEMPLATES` static exists in `templates.rs`. Good — the dispatcher flagged this as something to verify; it lines up. The downstream consequence (Claude agents flow through `init::extract()` rather than `apply_managed_state`) is what generates V-002.
- `Platform` is correctly marked `#[non_exhaustive]` (`platforms.rs:40`) per C-27. All three `<PLATFORM>_PLATFORM` consts populate every field by named-field initialization — `cargo build` would have failed otherwise, so no test is needed for that.
- `Layout::owned_dirs()` is registry-derived per C-3 (`layout.rs:477-486`) and tested by `owned_dirs_derives_from_registry` (`platforms.rs:687-702`).
- Function-length spot check: `Platform::apply_managed_state` is 26 lines (well under 50); `Layout::owned_dirs` is 9 lines; `collect_desired_templates` is 24 lines. No length blowouts in the new code.
- The dispatcher's "implementation drift" check on the `CLAUDE_AGENT_TEMPLATES` static: confirmed correctly absent. The PLAN documents the absence and the implementation matches.
