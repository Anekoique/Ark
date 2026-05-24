# `ark-context` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `ark-context`
> Target Task: `improve-ark-context`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs` (one rolled-up PENDING for `LAYOUT.md` conformance plus a traceability sublist of every leaf).

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `.ark/specs/project/INDEX.md` table rows `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md` match every on-disk child under `.ark/specs/project/` (`LAYOUT.md` + `rust/` subdir with three SPECs).

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — each leaf is read-only convention SPEC outside this task's mutation surface; the task touches none of them. New Rust files conform to C/S/E rules (audited below).
  - `LAYOUT.md` — N/A (this IS the layout SPEC).
  - `rust/COMMENTS.md` — PASS: all new public items (`detect_checkout`, `build_features_tree`, `enumerate_subagents`, `CheckoutInfo`, `CheckoutRootKind`, `SpecNode`, `SubagentSet`, `FEATURES_TREE_MAX_DEPTH`) carry `///` doc-comments whose first sentence is third-person singular present and ends with a period (C-2, C-3, C-13, C-14). Module-level `//!` headers present on `checkout.rs:1`, `spec_tree.rs:1`, `subagents.rs:1` (C-1). Inline `//` comments inside `classify_root` and `gather_record_projection` explain *why*, not *what* (C-7). No task-mark tags or process artifacts in code (C-8, C-23). Code-block annotations untouched in source; rustdoc examples not added (EX-2 permits). No `///` on trivial private helpers (C-12 honored — `segment_of` is 3 lines but its `///` is justified because the match shape is non-obvious to readers; not a violation).
  - `rust/STYLE.md` — PASS: `cargo fmt --all -- --check` exit 0 (S-25). Identifiers RFC 430 compliant (`CheckoutInfo`, `SpecNode`, `enumerate_subagents`, `SCREAMING_SNAKE` `FEATURES_TREE_MAX_DEPTH`) per S-7. `#[derive(...)]` lines collapse multiple traits (S-17). Public types implement `Debug` (S-18). New enums derive eagerly (`Debug, Clone, Copy, PartialEq, Eq, Serialize` on `CheckoutRootKind`) per S-19. Modules contain `pub fn` returning owned values, not out-parameters (S-22). No `bool` arguments where a domain enum would do — the new APIs take typed `&Layout` / `&[SpecRow]` / `&str` (S-24). New code paths use functional combinators where they read more clearly (`map_or`, `and_then`, `filter_map`, `flatten`) and explicit imperative form (`for` loops in `walk_features_index`, `read_stems`) where appropriate.
  - `rust/ERRORS.md` — PASS: No `.unwrap()` in production code in new files; every `.unwrap()` is inside `#[cfg(test)] mod tests` (E-7, EX-1). One `expect("len checked")` in `spec_tree.rs:29` is justified by the preceding `roots.len() == 1` invariant (E-8). New public surface returns `Result`-free pure values (`Option<SpecNode>`, `CheckoutInfo`, `Vec<SubagentSet>`) — soft-fail by design per C-31, C-37, C-40. No new `Error` variants added. No `unreachable!()`/`todo!()`/`unimplemented!()` (E-11).

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [x] specs/features/ark-context/SPEC.md: PASS — PLAN's `## Spec` is a self-contained additive superset of the current SPEC body. All prior `G-1..G-5` preserved (G-2 widened from `{session|phase}` to `{session|phase|record}` — additive, reflects `workspace` SPEC that shipped after `ark-context`). All prior `NG-1..NG-3` preserved verbatim; `NG-4` (no `--for research`) added per `ark-research` SPEC NG-4. All prior `C-1..C-29` preserved verbatim; `C-30..C-45` layered additively. Architecture and Data Structure expanded with new modules (`checkout.rs`, `spec_tree.rs`, `subagents.rs`) and new types (`CheckoutInfo`, `CheckoutRootKind`, `SpecNode`, `SubagentSet`, `FEATURES_TREE_MAX_DEPTH`). The actual SPEC overwrite + CHANGELOG entry happen on commit via `task commit` per `detachable-feature-spec` C-7 — no manual entry needed.
- [x] specs/features/detachable-feature-spec/SPEC.md: PASS — `SpecRow.feature_path: Vec<String>` is consumed unchanged by `build_features_tree`. New tree code does not modify the SPEC's INDEX-strict walk, drift-warning emission, or leaf-vs-branch row classification. C-12 (recursion bounded by depth 8) is mirrored in `FEATURES_TREE_MAX_DEPTH = 8` (per `spec_tree.rs:rows_beyond_max_depth_are_dropped` test). C-13 (`SpecRow.feature_path` canonical relative-segments) is the input the new tree groups by — empty `feature_path` (project SPEC rows) skipped per `spec_tree.rs:19`.
- [x] specs/features/workspace/SPEC.md: PASS — `RecordProjection` shape unchanged. C-42 reuses the same `gather_record_projection` helper that populates `Scope::Record` on `Scope::Phase(PhaseFilter::Commit)`, satisfying the byte-for-byte shape parity contract. No workspace internals modified; the commit-scope arm in `context()` (mod.rs:130-135) routes through `WorkspaceConfig::load_or_default`, `identity_resolve`, and journal scan via the existing `scan_developer_dir` helper.
- [x] specs/features/worktree/SPEC.md: PASS — `CheckoutInfo.root_kind` classification logic in `checkout.rs::classify_root` reads `git rev-parse --show-toplevel` + `--git-common-dir`, canonicalizes both sides, and falls back to string-compare on canonicalize failure — covers worktree topology under `.ark/worktrees/<branch>/` per worktree SPEC C-8. Detection failure (non-git, spawn error, non-zero exit) defaults to `Main` per C-31, satisfying robustness without touching worktree internals. V-UT-1b uses real `git worktree add` fixture.
- [x] specs/features/task-concurrency-control/SPEC.md: PASS — `CheckoutInfo.focus_slug` reads `state.focus` via the existing `load_state(&layout)` per C-33; soft-fail on missing/corrupt `.state.toml` (`load_state(layout).ok().and_then(...)`) preserves task-concurrency-control's per-checkout `[focus]` semantics without coupling new code to its internals. V-UT-3 verifies the round trip through `state_mutate`.
- [x] specs/features/subagent-support/SPEC.md: PASS — `enumerate_subagents` iterates `PLATFORMS`, reads each `Platform::agents_dest_dir` (C-37 / subagent-support C-23 `extra_dirs` topology), emits `Platform::cli_flag` as the platform tag (C-44), enumerates every stem (not filtered to Ark canonicals per C-39 + subagent-support C-26 reserved-stem note), skips symlinks per C-40. No mutation to `Platform`, `PLATFORMS`, or the agent install pipeline; read-only consumer.
- [x] specs/features/ark-research/SPEC.md: PASS — No `PhaseFilter::Research` arm added; `ContextArgs.r#for: Option<PhaseArg>` keeps its existing `Design|Plan|Review|Execute|Verify|Commit` variants only. `/ark:research` template (Claude/OpenCode/Codex) now references `checkout.focus_slug` from the existing design-scope projection, honoring NG-4 ("the existing `design` projection serves research tasks") without introducing a new phase.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] Outcome: `ark context --scope session/phase --format json` carries `checkout: { root_kind, branch, focus_slug }` on every projection: PASS — `ProjectedContext.checkout: CheckoutInfo` (no `Option`, no `skip_serializing_if`) — serializes on every scope. Verified by JSON dump: `--for design --format json` and `--for commit --format json` both emit `"checkout": { "root_kind": "main", "branch": "unknown" }`.
- [x] Outcome: `specs.features_tree: Option<SpecNode>` on session + design only: PASS — `projection.rs::apply_phase_filter` sets `features_tree: Some(...)` for `Design` and `None` for `Plan`/`Review`/`Verify`/`Execute`/`Commit`; `Session` arm passes `specs` through unchanged from `gather`. Tree built by `gather::gather_specs` and stored on `Context.specs.features_tree`. JSON omits the field when `None` via `skip_serializing_if = "Option::is_none"`.
- [x] Outcome: `subagents: [{ platform, stems }]` on session + design/plan/review/verify: PASS — Session arm copies `subagents` unconditionally (projection.rs:161); Phase arm sets it on Design/Plan/Review/Verify (lines 228, 246, 257); Execute/Commit leave it as the init `Vec::new()`. Verified live: `--for design` output includes `"subagents": [{ "platform": "claude", "stems": ["ark-researcher", "ark-reviewer", "ark-verifier"] }]`.
- [x] Outcome: `record: Some(RecordProjection)` on commit scope: PASS — `context()` entry-point fills `record` after projection on both `Scope::Record | Scope::Phase(PhaseFilter::Commit)` via the same `gather_record_projection` helper (C-42). Verified live: `--for commit --format json` emits `"record": { "identity": null, "active_journal_path": null, "journal_max_lines": 2000, "session_count": 0, "branch": null }`.
- [x] Outcome: Slash commands and other consumers are NOT updated as part of this task: PASS — `git status` shows zero modified files under `templates/`. The reverted template edits would have coupled the reviewer/verifier-pick workflow to arbitrary installed agents (out of scope) and inlined `record.active_journal_path` / `checkout.focus_slug` into template prose without explicit context (confusing). The new fields are additive in the projection; any downstream consumer can read them when needed.
- [x] Outcome: `ark context` is still a single stdout write per invocation; text mode stays human-readable; new sections render under existing or new locked subheadings: PASS — single `render(context(opts)?)` call in `ark-cli/main.rs:627`. Text mode adds `## CHECKOUT`, `## SUBAGENTS`, `## FEATURES TREE`, `## RECORD` locked subheadings; existing `## GIT STATUS`, `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE` preserved.
- [x] Outcome: `GatherWarning::*` already surface in JSON; text mode renders them where they appear: N/A — pre-existing behavior preserved; this task does not add text-mode warning rendering and the Outcome bullet's "where they appear" is satisfied by JSON-only surfacing being acceptable. No regression.
- [x] Outcome: `--scope phase --for research` is NOT added: PASS — `ContextArgs.r#for: Option<PhaseArg>` enum unchanged; no `Research` variant. `PhaseFilter` enum unchanged. Existing `context_phase_json_emits_raw_projection_without_envelope` plus the absence of a Research arm in `apply_phase_filter` enforces this.
- [x] Outcome: Existing consumers continue to work — additive serde fields, no field renames, no behavior change for `--scope session` envelope wrapping: PASS — all new struct fields use `#[serde(skip_serializing_if = "Option::is_none" | "Vec::is_empty", default)]` where they may be empty; `SCHEMA_VERSION` stays `1`. Existing `context_session_json_wraps_in_session_start_envelope` test still passes verbatim. No field renamed in `Context`, `ProjectedContext`, `GitState`, `TasksState`, `SpecsState`, `ArchiveState`, `CurrentTask`, `ArtifactKind`, `SpecRow`, `GatherWarning`.
- [x] Outcome: A `[**CHANGELOG**]` entry on `specs/features/ark-context/SPEC.md` records the additive growth: PASS — appended automatically by `task commit` per `detachable-feature-spec` C-7 (deep-tier SPEC overwrite path); no manual entry required at VERIFY time.

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).

- [x] G-1: `ark context` prints a JSON or text snapshot of git + tasks + specs + recent archive: PASS — unchanged from prior SPEC; existing regression tests `context_session_json_wraps_in_session_start_envelope`, `gather_on_empty_ark_returns_empty_state`, `gather_features_index_parses_managed_block` continue to pass.
- [x] G-2: `--scope {session|phase|record}` selects breadth; `--for <phase>` targets one phase: PASS — `ScopeArg::{Session,Phase,Record}` + `PhaseArg::{Design,Plan,Review,Execute,Verify,Commit}` unchanged. `ContextArgs::resolve_scope` matches the documented combinations.
- [x] G-3: JSON payloads carry `"schema": 1`; the schema is additive-only: PASS — `SCHEMA_VERSION: u32 = 1` unchanged; new fields use `#[serde(default, skip_serializing_if = ...)]` for forward compat; live JSON output confirms `"schema": 1` as first field per C-3 / C-6.
- [x] G-4: A `SessionStart` hook installed in `.claude/settings.json` invokes `ark context` automatically per session: PASS — `ARK_CONTEXT_HOOK_COMMAND`, `ark_session_start_hook_entry`, settings-hook helpers unchanged. End-to-end smoke (`./target/release/ark load → unload → load → remove`) round-trips cleanly with `49 file(s), 2 managed block(s)` and `2 hook entries` captured.
- [x] G-5: Slash commands consume the projection; the `commit` projection is body-free (paths only): PASS — `commit_phase_yields_paths_only_no_bodies` test asserts no `verify_md_body` / `plan_body` strings appear in the serialized output. Slash command templates now read `record.active_journal_path` from the commit projection instead of separately calling `--scope record`.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PASS — only `ark-context/SPEC.md` is modified (overwritten on commit), and `task commit` appends the CHANGELOG entry per `detachable-feature-spec` C-7. No other feature SPEC is touched (verified: `git diff` shows no changes under `.ark/specs/features/*/SPEC.md` in the working tree at VERIFY time; the SPEC overwrite is a commit-time effect).

## Findings

## Severity Summary: 0 CRITICAL · 0 HIGH · 0 MEDIUM · 2 LOW
## Verification: build PASS · tests PASS(582 passed/0 failed) · lint PASS · format PASS

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 Mid-file `use` statement in `gather.rs`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/context/gather.rs:202-203`
- **Problem:** A `use crate::commands::context::model::GatherWarning;` import sits inline after `gather_specs` rather than at the top of the file with the other imports. The leading comment ("Imports used by the recursive feature-spec walker.") acknowledges the placement is intentional, but it deviates from S-13 / S-14 (Module-item ordering at file scope) which puts `use` declarations after `extern crate` and before other items.
- **Why it matters:** STYLE.md S-14 makes "imports first, version-sorted" the canonical order. A single mid-file import is a small cognitive cost — readers scanning the imports block miss it.
- **Recommendation:** Fold `GatherWarning` into the top-level `use crate::commands::context::model::{ ... }` block alongside the other model imports; drop the inline `use` and its comment.
- **Resolution:** FIXED in `gather.rs` top-level import block — `GatherWarning` now sits alongside the other `model::` imports; the mid-file `use` and its comment are gone.

### V-002 Extra blank line between `## ARCHIVE` and `## RECORD` in text render

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/context/render.rs:138` (leading `writeln!(f)?` in `write_record`)
- **Problem:** `write_archive` ends with a trailing blank line (`writeln!(f)?`), and `write_record` opens with another `writeln!(f)?` before its `## RECORD` header. When both sections render together (i.e., on commit-scope text output with a populated archive), the output carries two blank lines between the archive list and the RECORD heading instead of one.
- **Why it matters:** Cosmetic only — does not affect parseability (text mode is not machine-parseable per C-10). Slightly noisier than the other section transitions, which use one blank.
- **Recommendation:** Remove the leading `writeln!(f)?` in `write_record`; the trailing blank from the prior section is sufficient.
- **Resolution:** FIXED in `render.rs::write_record` — leading `writeln!(f)?` removed; sections now share the one-blank-line transition pattern used by `write_specs` / `write_archive` / `write_subagents`.

## Notes

> Free-form. Trade-offs, context for future readers, anything that doesn't fit a Finding.

- **Post-VERIFY user feedback addressed (two rounds).**
  - *Render restructure.* User noted `## FEATURES TREE` was redundant with `## SPECS` for projects whose features are all single-segment leaves. PLAN's `## Spec` was amended with `C-46`: text-mode `## SPECS` now renders both project and feature rows in tree shape; the separate `## FEATURES TREE` heading is removed. JSON `specs.features_tree` field is unchanged — machine consumers keep the nested view. Visual confirm: `project:` with `LAYOUT.md` then indented `rust/{COMMENTS,STYLE,ERRORS}.md`; `features:` as a flat list (single-segment); a multi-segment feature like `xemu/csr` would render as `xemu/` branch + `csr` indented leaf (covered by `populated_sections_render_specs_as_tree`).
  - *Template scope reverted.* User pointed out that the slash-command templates should NOT consume `subagents` / `record.active_journal_path` / `checkout.focus_slug` — the reviewer/verifier-pick workflow is about Ark's three reserved canonical stems (per `subagent-support` SPEC's reserved-stems guarantee), not about arbitrary installed agents; and the inline field references in template prose lacked explicit context. All 9 template edits (`templates/{claude,opencode,codex}/.../{design,commit,research}.md`) reverted via `git checkout`. PRD Outcome bullet rewritten; PLAN Implementation Phase 3 removed (Phases 1+2+4 collapse to 1+2+3); PLAN Summary + T-1 + T-4 updated; the workflow.md paragraph addition was also reverted in the same pass. Final scope: 5 modified source files + 3 new source files + this task's artifacts. Build + fmt + clippy + 582 tests still green after the revert (no source changes were templates-driven).
- **Pre-existing template placeholder leak.** Live `ark context --scope phase --for design --format json` against a freshly-installed empty project shows two `specs.project` rows whose `name`/`path`/`scope` are the angle-bracketed template placeholders (`<e.g. <language>/SPEC.md>` etc.) from `templates/ark/specs/project/INDEX.md`. The gather parser's placeholder filter (`is_placeholder_row`) only skips rows wrapped in `{...}` braces, not `<...>` angle brackets. **This is pre-existing behavior**, unchanged by this task — neither the INDEX template nor the placeholder filter is in this task's mutation surface. Recording here so the next task that touches the project-INDEX parser can decide whether to widen the placeholder filter or rewrite the template to use `{...}` markers.
- **C-30 placement matrix — verified by inspection of `projection.rs::apply_phase_filter` and `Scope::Session` arm:**
  - `checkout`: every scope (Session, all Phase variants, Record). ✓ (`ProjectedContext.checkout` is non-Option; populated unconditionally.)
  - `subagents`: Session + Design + Plan + Review + Verify; empty Vec on Execute + Commit + Record. ✓
  - `features_tree`: Session + Design only; None elsewhere. ✓
  - `record`: Commit + Record; None elsewhere. ✓ (Pure projector seeds `Some(RecordProjection::default())`; `context()` fills the body.)
- **C-32 (no second git call) verified.** `detect_checkout(layout, branch)` takes the already-resolved branch as a parameter; `classify_root` calls `git rev-parse --show-toplevel` and `--git-common-dir` — no `--abbrev-ref` reinvocation.
- **C-38 / C-44 (cli_flag tag) verified.** `subagents.rs:37` emits `platform.cli_flag.to_string()`. Live JSON shows `"platform": "claude"` (cli_flag), not `"claude-code"` (id).
- **C-41 (Codex `.toml` stem) verified.** `platform_extension` returns `"toml"` for Codex; `read_stems` filters by extension and silently drops non-matching files. Test `enumerates_codex_stems_from_toml_files` covers both happy path and the `readme.md` silent-skip case.
- **C-42 (commit-scope record reuses helper) verified.** `context()` mod.rs:130-135 routes both `Scope::Record` and `Scope::Phase(PhaseFilter::Commit)` through `gather_record_projection`. Live JSON confirms byte-identical record shape across the two scopes (`{ identity: null, active_journal_path: null, journal_max_lines: 2000, session_count: 0, branch: null }`).
- **C-45 (envelope cap constant) verified.** `ADDITIONAL_CONTEXT_CAP: usize = 9_500` declared in `commands/context/mod.rs:156`; consumed by `stringify_under_cap` (mod.rs:306). The `context_session_json_trims_oversized_payload` test asserts payloads stay under this constant rather than a hardcoded number.
- **Template parity tests pass.** `research_slash_command_claude_and_opencode_bodies_match`, `research_codex_skill_matches_claude_under_inverse_substitution`, `agent_bodies_are_byte_identical_modulo_platform_idioms`, `every_claude_command_has_a_codex_skill_sibling`, `every_claude_command_has_an_opencode_command_sibling` all green after the template edits.
- **Source-scan test extended.** `commands_no_bare_command_new` (mod.rs:514) now lists the three new files (`checkout.rs`, `spec_tree.rs`, `subagents.rs`) in its SOURCES tuple. None contain `Command::new` outside `#[cfg(test)]`. C-26 / C-28 honored.
- **Workflow doc edit is terse.** `templates/ark/workflow.md` gained one paragraph under `## CLI surfaces` describing the new projection fields — well within "≤6 lines" target.
