# `drop-task-slug` VERIFY

> Status: Closed.
> Feature: `drop-task-slug`
> Target Task: `drop-task-slug`
> Tier: standard

---

## Project Spec Compliance

- [x] LAYOUT.md: PASS — no SPEC files added; only existing feature SPECs edited.
- [x] rust/COMMENTS.md: PASS — new doc-comments are third-person present, summary on first line, no contractions, no SPEC labels in source. `slug_from_worktree_root` and `resolve_slug` follow C-21 ("Returns ..."). No `// G-N`-style tags.
- [x] rust/STYLE.md: PASS — `cargo fmt` clean. Imports grouped + version-sorted. New `Layout::slug_from_worktree_root` uses `match`/`?` idiomatically; no `unwrap` outside tests.
- [x] rust/ERRORS.md: PASS — `NoActiveTask` and `AmbiguousActiveTask` are `thiserror`-derived variants with structured fields (E-12), lowercase messages without trailing punctuation (E-9), and contextual fields naming the resource (E-15: `project_root`, `candidates`).

## Related Feature Spec Compliance

- [x] .ark/specs/features/ark-agent-namespace/SPEC.md: PASS — table updated, fallback paragraph rewritten, error variants list updated. Phase 5.1.1 grep gate confirms no `[--slug <s>]` or `NoCurrentTask` references remain.
- [x] .ark/specs/features/task-concurrency-control/SPEC.md: PASS — G-9 rewritten, `NoActiveTask` + `AmbiguousActiveTask` added to error variants block, `NoCurrentTask` removed.
- [x] .ark/specs/features/worktree/SPEC.md: PASS — G-4 signature dropped `[--slug <s>]`, `WorktreeCleanupCliArgs` SPEC code-snippet updated.

## PRD Constraints

- [x] Worktree topology resolution: PASS — `Layout::slug_from_worktree_root` + `task_dir.is_dir()` + active-set membership combined check. V-UT-5 + manual repro confirm.
- [x] Single active task: PASS — V-UT-7 covers it.
- [x] Multiple active tasks with valid session focus: PASS — V-UT-9 (no focus → AmbiguousActiveTask) + V-UT-10 (valid focus → returns it).

## Plan Fidelity

- [x] G-1: PASS — `slug` field removed from `TaskSlugArgs`, `TaskCommitCliArgs`, `TaskArchiveCliArgs`, `TaskPromoteCliArgs`, `WorktreeCleanupCliArgs`, `SpecExtractCliArgs`. Verified: `cargo build` clean; passing `--slug X` would now produce clap's "unexpected argument" error.
- [x] G-2: PASS — `TaskResumeCliArgs.slug: String` (unchanged). `TaskDiscardCliArgs.slug` changed from `Option<String>` to `String` (required).
- [x] G-3: PASS — `resolve_slug(root: &Path, ppid: &dyn Ppid) -> anyhow::Result<String>` implemented per Data Structure block. Cascade: worktree → single-active → focus → split error. `lookup_session_id(...).ok().flatten()` handles cache IO failures gracefully.
- [x] G-4: PASS — REMOVED in PLAN 01; `discard` does not cascade.
- [x] G-5: PASS — `Error::NoCurrentTask` removed; `NoActiveTask { project_root }` and `AmbiguousActiveTask { candidates }` added with the specified message text.
- [x] G-6: PASS — `resolve_slug_regression_pr_repro` (V-UT-13) replays the exact bug fixture and asserts `Ok("b")`. Manual repro on the release binary also confirms (got `IllegalPhaseTransition` post-resolution, not `NoCurrentTask`).
- [x] G-7: PASS — three feature SPECs revised in lockstep. Phase 5.1.1 grep gate confirms.
- [x] G-8: PASS — both copies of `workflow.md` edited at lines 141, 165, 191, 195. Phase 5.1.1 grep gate confirms.
- [x] G-9: PASS — `cargo build --workspace`, `cargo test --workspace` (409 passed), `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` all green.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — standard tier; deep-tier CHANGELOG convention does not apply. The standard-tier task does not promote a SPEC, but it does revise three existing feature SPECs. Per workflow §5, that's allowed without a CHANGELOG block (CHANGELOG applies to deep-tier promoted SPECs).

## Findings

### V-001 PLAN 5.1.1 grep regex was over-broad

- **Severity:** LOW
- **Location:** `02_PLAN.md` Phase 5.1.1
- **Problem:** Regex `^ark agent task discard([[:space:]]|$|#)` matches `ark agent task discard --slug <slug>` (whitespace immediately follows `discard`), causing the gate to fire on legitimate kept-form lines.
- **Why it matters:** A regression gate that fires on legitimate content is worthless — future maintainers will disable it. The intent was to catch the BARE form (no `--slug`), not all `discard` invocations.
- **Recommendation:** Tightened to `^ark agent task discard($|[[:space:]]+#)` — anchors on EOL or whitespace-then-comment. Ran inline; gate now passes cleanly.
- **Resolution:** FIXED inline during Phase 5 verification.

### V-002 Reconciler-driven test fixtures need real `task.toml`

- **Severity:** LOW
- **Location:** `crates/ark-cli/src/agent_cli.rs::tests`
- **Problem:** First pass of resolver tests created empty `<task-dir>/` directories, expecting `state.tasks.active` to be honored verbatim. But `load_state` runs `reconcile_against_disk`, which enumerates `.ark/tasks/<slug>/task.toml` and drops slugs without valid TOML. Half the tests failed.
- **Why it matters:** Surfaces the correct contract: `state.tasks.active` is reconciled against on-disk truth at every read. Tests must therefore plant a real `task.toml`, not just create the directory. Documenting it here so future test authors don't trip on the same wire.
- **Recommendation:** `make_task_dir` helper in tests now writes a minimal valid `task.toml`. The one test that needs a "task dir present but slug not in active" semantics (V-UT-6) creates an empty dir intentionally to suppress reconciler pickup.
- **Resolution:** FIXED inline during Phase 3.

## Notes

- The bug-reproduction's first manifestation in this very session (when running `ark agent task plan` from `/Users/anekoique/Agent/Ark` with 6 active tasks) provided live validation of the `AmbiguousActiveTask` error and its message text. That is mathematically distinct from the PRD's bug, but it confirms the new error variants render correctly.
- The two reviewer rounds caught everything that mattered before EXECUTE began. Round-3 approval with two LOW findings (both fixed inline) was the right termination point — no further iteration would have produced different code.
- N-6 (`worktree/SPEC.md` G-4 step 1's stale `.current` reference) is **not** addressed by this task. Deferred to follow-up `worktree-spec-current-cleanup`.
