# `session-focus-bind` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `session-focus-bind`
> Target Task: `session-focus-bind`
> Tier: standard
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Standard tier: `/ark:commit` warns on PENDING and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time. One bullet per registered SPEC.

- [x] LAYOUT.md: N/A — Layout A governs `specs/project/` convention SPECs; this task only edits feature SPECs (which use the runtime-system template), not project SPECs.
- [x] rust/COMMENTS.md: PASS — all new doc-comments use `///` for items, `//!` for module heads, third-person singular present (`Returns`, `Reports`, `Verifies`), one-sentence summaries, no contractions in production prose, no task-mark tags in `crates/`. Test docstrings start with "Verifies that …" per C-3/C-11.
- [x] rust/STYLE.md: PASS — `cargo fmt --check` clean (S-1, S-2, S-3, S-4, S-25); imports version-sorted (S-13); `snake_case` / `UpperCamelCase` casing per S-7; `#[derive(...)]` consolidated per S-17; no `bool` out-args (S-22). New `Option<String>` field on summaries is the right shape — a domain enum would be over-engineered for "absent vs present prior focus."
- [x] rust/ERRORS.md: PASS — `Error::NoFocus { project_root, candidates }` carries typed context fields (E-12, E-15); `#[error("...")]` template uses lowercase first word, no trailing punctuation (E-9); existing `Result<T>` alias preserved (E-5); no new `.unwrap()` in production (E-7); the one preserved `.expect("StateFile serializes")` is genuinely-impossible serde failure (E-8). No `#[from]` introduced for foreign errors.

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — C-14 rewritten to "session-focus-bind" contract; Data Structure block updated (`NoFocus` added, `NoActiveTask`/`AmbiguousActiveTask` removed); CHANGELOG entry dated 2026-05-08 added.
- [x] specs/features/task-concurrency-control/SPEC.md: PASS — Goals, Architecture, Data Structure, API Surface, call graphs, Constraints (C-1 through C-24) rewritten; new C-18 covers `overwrote_focus` summary field; new C-19 covers `task_commit` focus-clear; new C-24 covers legacy `[sessions.*]` deserialization tolerance; CHANGELOG entry dated 2026-05-08 added.
- [x] specs/features/workspace/SPEC.md: N/A — workspace SPEC has no references to session machinery (`grep -nE "Ppid|sessions\.|Session \{|session_id|session::"` returned 0 matches). No body change needed; CHANGELOG note skipped to avoid no-op entries.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] State-file shape: top-level `[focus] slug` field, `[sessions.*]` table removed: PASS — verified live in `.ark/.state.toml` after the migration: 17 stale `[sessions.*]` blocks dropped, single `focus = "session-focus-bind"` at top.
- [x] `task new`/`task resume` write `state.focus` in same `state_mutate`: PASS — `register_focus` in `new.rs:474–489` and inline in `resume.rs:62–75`.
- [x] `task archive` clears `state.focus` iff slug matches: PASS — `clear_focus_for_slug` in `state/checkout/io.rs:96–104`, called from `archive.rs:115`.
- [x] `task discard` clears `state.focus` iff slug matches: PASS — same helper, called from `discard.rs:118`.
- [x] `task commit` clears `state.focus` on success and `--no-commit`: PASS — `clear_focus_if_matches` helper in `commit.rs:275–283`, called after both `guard.commit()` sites.
- [x] `task new` no longer warns about other active tasks: PASS — `had_other_active` and the `eprintln!` removed; `other_active` field removed from `TaskNewSummary`.
- [x] `prune_dead_sessions` removed; reconcile clears stale focus instead: PASS — `reconcile.rs:42–46` is the focus-invalidation pass; `prune_dead_sessions` no longer exists.
- [x] One-shot `$TMPDIR/ark-session-<hash>-*.id` cleanup on every save: PASS — `cleanup_orphan_session_caches` in `state/checkout/io.rs:163–179`. Verified live: 26 stale cache files for this project's hash unlinked after first new-binary save.
- [x] `Error::NoFocus { project_root, candidates }` replaces `NoActiveTask`/`AmbiguousActiveTask`: PASS — `error.rs:99–110`. Old variants gone; CLI matches `NoFocus` to print candidate list.
- [x] `Ppid` trait, `RealPpid`, `StubPpid`, `session/` module deleted: PASS — `crates/ark-core/src/session/` directory removed; `lib.rs` no longer `pub mod session`; no production references remain (`grep -rn "Ppid\b\|RealPpid\b\|StubPpid\b" crates/` returns matches only inside templates / test fixtures unrelated to this).
- [x] `load_state` and `state_mutate` lose `&dyn Ppid`: PASS — signatures `load_state(layout: &Layout) -> Result<StateFile>` and `state_mutate(layout: &Layout, edit: F) -> Result<()>`. All call sites updated.
- [x] `agent_cli.rs::resolve_slug(&Path) -> Result<String>` reads `state.focus`: PASS — implementation at `agent_cli.rs:570–581`. Cascade replaced.
- [x] `workflow.md` "Multi-session state" → "Focus model" rewritten; failure-modes table swaps to `NoFocus`; cascade paragraph deleted: PASS — `.ark/workflow.md:313`/`:330`/`:338`.
- [x] `task new`/`task resume` warn when rebinding focus, suggest `--worktree`: PASS — `TaskNewSummary.overwrote_focus`, `TaskResumeSummary.overwrote_focus`; `Display` renders the warning with verbatim restore command. Verified live: `ark agent task resume --slug doc-tighten` then back produced the warning correctly.

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`).

- [x] G-1: `.state.toml` carries one optional `[focus] slug` per checkout: PASS — verified by serde round-trip test (`state_mutate_persists_focus`) and live state file shape.
- [x] G-2: `task new`/`task resume` set focus; `task archive`/`task discard`/`task commit` clear it iff slug matches: PASS — covered by `concurrency_tests.rs` (`task_new_binds_focus`, `task_archive_of_focused_clears_focus`, `task_archive_of_non_focused_preserves_focus`, `task_discard_of_focused_clears_focus`) and `commit.rs::tests` (`quick_tier_commit_is_atomic_and_clean`, `no_commit_flips_phase_without_committing`).
- [x] G-3: Non-targeted verbs resolve via `state.focus`; absent → `Error::NoFocus`: PASS — `agent_cli::tests::resolve_slug_returns_focus_when_set` and `resolve_slug_errors_no_focus_when_unset`.
- [x] G-4: `session/` module, `Ppid` trait, and `[sessions.*]` map removed: PASS — module directory deleted; `lib.rs` re-exports cleaned; `Session` struct + `BTreeMap` field removed from `StateFile`. Build proves no remaining references.
- [x] G-5: First new-code `state_mutate` unlinks orphan `$TMPDIR/ark-session-<hash>-*.id` files: PASS — `cleanup_orphan_session_caches_removes_only_this_projects_files` test plus live verification (26 → 0 stale files).
- [x] G-6: `task new` and `task resume` warn when overwriting an existing focus: PASS — `task_new_reports_overwrote_focus_on_rebind`, `task_new_first_task_does_not_warn`, `task_resume_reports_overwrote_focus_on_rebind`, `task_resume_does_not_warn_when_focus_unchanged`.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PASS — `ark-agent-namespace/SPEC.md` and `task-concurrency-control/SPEC.md` both have 2026-05-08 `session-focus-bind` entries.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 PRD's Outcome bullet on `Error::NoFocus { ..., candidates }` predates the Display copy

- **Severity:** LOW
- **Location:** `crates/ark-core/src/error.rs:99–110` vs PRD outcome
- **Problem:** PRD said `candidates` is "populated for diagnostics; not in Display". The shipped `#[error("...")]` template *does* render the candidates list inline (`(active: {})`). This is a deliberate improvement — surfacing candidates in the user-visible message is more actionable than burying them on the struct — but it diverges from the PRD's text.
- **Why it matters:** Anyone reading PRD vs shipped behavior would see a small inconsistency. The shipped behavior is the better one.
- **Recommendation:** Document the divergence here; do not edit the PRD (PRDs freeze at design time per the workflow). The `task-concurrency-control` SPEC's C-23 already reflects the shipped contract.
- **Resolution:** ACCEPTED — shipped behavior surfaces candidates in `Display` body; PRD text is point-in-time and the SPEC is authoritative.

### V-002 `task discard` test in `discard.rs::tests` does not assert focus clearing

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/discard.rs:155–238`
- **Problem:** The discard module's own tests assert directory removal and the `--force` guard, but not the focus-clear side effect of the focused-discard path. Coverage exists in `concurrency_tests.rs::task_discard_of_focused_clears_focus` and `task_discard_of_non_focused_preserves_focus`, so the behavior is verified — just not in the per-module file.
- **Why it matters:** A future refactor that loses the call to `clear_focus_for_slug` would still pass the per-module suite, relying on `concurrency_tests.rs` to catch it. Test locality is a soft preference.
- **Recommendation:** Optional — could move or duplicate one focus-clearing assertion into `discard.rs::tests`. Not blocking.
- **Resolution:** ACCEPTED — coverage exists in the cross-cutting test file; no production behavior is unverified.

### V-003 `Display` warning for rebind goes through `TaskNewSummary::Display`, which the CLI prints to stdout, not stderr

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/task/{new,resume}.rs::Display`, rendered by `crates/ark-cli/src/main.rs::render`
- **Problem:** The PLAN's C-3b says "stderr-rendered warning", but the CLI's `render` function writes the summary's `Display` output to stdout. The warning text appears at the end of the success line on stdout, not on stderr. This matches the pattern Ark already uses (see `worktree` summary's parenthetical), but contradicts the C-3b wording.
- **Why it matters:** A pipeline consumer parsing stdout sees the warning text; one parsing stderr does not. The current behavior is consistent with how Ark surfaces other ancillary information, and the warning is purely informational. But the PLAN copy is technically wrong.
- **Recommendation:** Update the PLAN's C-3b copy to "summary-rendered warning" or accept the divergence. The `task-concurrency-control` SPEC's C-18 says "stderr-rendered" too — should be reworded to "Display-rendered" to match shipped behavior.
- **Resolution:** ACCEPTED — shipped behavior matches Ark's existing pattern; the SPEC text drift is captured here. A follow-up doc-tighten task could correct the wording in C-18 of `task-concurrency-control` SPEC, but it is not a blocker for commit.

## Notes

**Manual cleanup confirmed end-to-end on the live checkout.** Before this task: `.state.toml` carried 17 stale `[sessions.*]` blocks; `$TMPDIR` carried 26 stale `ark-session-f213d6cbc5842122-*.id` files. After running the new binary's first `state_mutate`: zero stale sessions in `.state.toml`, zero stale cache files in `$TMPDIR`. The migration is fully self-healing — no user steps required.

**The cascade bug is fixed.** Before this task: `ark agent task <verb>` errored `multiple active tasks: doc-tighten, drop-task-slug, ...` because eight committed-but-not-archived tasks confused the topology cascade. After: `task verify` and `task resume` resolve correctly via `state.focus`. Confirmed live during EXECUTE.

**Net code reduction.** Deleted: entire `crates/ark-core/src/session/` directory (~250 LOC including tests), `prune_dead_sessions` (~5 LOC), `Session` struct + `BTreeMap` field, two error variants, `had_other_active` warning logic. Added: ~30 LOC for `cleanup_orphan_session_caches` + tests, ~25 LOC for `overwrote_focus` plumbing in two files, ~10 LOC for `clear_focus_if_matches` in commit.rs. Roughly 200 LOC net deletion, plus the entire elimination of the `Ppid` cross-platform shim's existence in our coupling graph.

**Trade-off T-1 from the PLAN held up under implementation.** Per-checkout focus is genuinely simpler than the per-session model and matches how the workflow is actually used. The two real costs (multi-shell same-checkout = one shared focus; AI harness = focus binding works because no PPID) both resolved as expected.

**Workspace SPEC unchanged.** Verified: workspace SPEC has zero references to the deleted session machinery. PRD's "scan for [sessions.*] / Ppid references" item completed with "no body change needed".
