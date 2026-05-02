# `ark-workflow-refactor` VERIFY

> Status: Closed. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `ark-workflow-refactor`
> Target Task: `ark-workflow-refactor`
> Tier: deep
>
> Every checklist item is non-PENDING; every finding's Resolution is
> non-PENDING. `/ark:commit` deep-tier gate accepts.

---

## Project Spec Compliance

> Audited each ruleset against the new code (commit.rs, archive.rs,
> verify_seed.rs, rollback handling) and the rewritten templates.

- [x] LAYOUT.md: PASS — no convention-SPEC layout changes; this refactor adds runtime modules and templates only.
- [x] rust/COMMENTS.md: PASS — new modules carry C-N-compliant doc comments (third-person summary; `# Errors` sections on `Result`-returning fns).
- [x] rust/STYLE.md: PASS — `cargo fmt --check` clean; `cargo clippy --all-targets` clean (collapsible-if warnings auto-fixed); newtype + builder patterns used appropriately; public types derive `Debug`.
- [x] rust/ERRORS.md: PASS — six new `Error` variants (`NothingStaged`, `VerifyIncomplete`, `GitCommitFailed`, `CommitMessageRequired`, `CommittedAtMissing`, `TemplateMarkerMissing`) all carry context fields per E-15; `?` propagation throughout; no `unwrap()` outside tests.

## Related Feature Spec Compliance

- [x] .ark/specs/features/workspace/SPEC.md: PASS — `record_task` threads `start_head` through; `JournalEntry` carries `**Start Head**` + `**Base Branch**` for task entries (manual entries omit). `record_task`'s `archive_path → task_dir`, `archived_at → recorded_at` rename matches the post-refactor invocation site.
- [x] .ark/specs/features/task-concurrency-control/SPEC.md: PASS — `Phase::Committed` slots in; reconcile keeps `Committed` slugs in `tasks.active` (existing predicate unchanged because `Committed != Archived`); state-file behavior unchanged.
- [x] .ark/specs/features/ark-agent-namespace/SPEC.md: PASS — verb set preserved (`task archive` retained, mechanics now side-effect-free; `task commit` added). Namespace stays hidden + non-semver per the SPEC.
- [x] .ark/specs/features/ark-context/SPEC.md: PASS — `PhaseFilter::Commit` projection is body-free; serialization test asserts no `verify_md_body` / `plan_body` fields.
- [x] .ark/specs/features/ark-upgrade/SPEC.md: PASS — `ark upgrade` gains `verify_migrated` counter; migration runs after manifest write so a migration error cannot block template refresh; idempotency test included.
- [x] .ark/specs/features/worktree/SPEC.md: PASS — worktree creation/cleanup unchanged; `task_commit` operates on the worktree's cwd when `task.toml.worktree_path` is set, matching existing per-phase behavior.
- [x] .ark/specs/features/codex-support/SPEC.md: PASS — Codex skill `ark-commit/SKILL.md` shipped; `ark-archive/SKILL.md` deleted; `ark-design`/`ark-quick`/`ark-record` updated to point at `ark-commit`.
- [x] .ark/specs/features/project-spec/SPEC.md: PASS — Layout A unchanged. The new `read_project_specs` parser reads the `## Index` markdown table per the project's actual INDEX shape (corrected during smoke test).

## PRD Constraints

- [x] Lifecycle is design then plan/review then execute then verify then commit then bulk-archived: PASS — `Phase::Committed` variant added; transition table updated; `archived_only_reachable_from_committed` test enforces the new shape.
- [x] /ark:commit [-m] [--no-commit] is the new closure slash command: PASS — three platforms ship `commit.md` / `ark-commit/SKILL.md` with identical body modulo platform frontmatter.
- [x] /ark:commit requires staged work: PASS — `Error::NothingStaged` returned when `git diff --cached --quiet` succeeds; `commit_errors_when_nothing_staged` test exercises the path.
- [x] VERIFY.md is a living checklist + findings document: PASS — six fixed sections; four dynamically seeded from project state; PENDING-completion criterion enforced by `parse_verify_md` and `task_commit`'s gate.
- [x] /ark:archive removed; ark archive bulk CLI added: PASS — three slash-command platforms drop `archive.md`; new top-level `ark archive` CLI visible in `ark --help`; `task_archive_move` is side-effect-free; tests `archive_writes_no_journal_entry` and `archive_writes_no_spec_files` enforce.
- [x] Phase enum gains Committed: PASS — V-UT-1, V-UT-2, V-UT-3, `archived_only_reachable_from_committed` cover.
- [x] Recoverable journal commit range, anchored to slug: PASS — three end-to-end tests in `commit::e2e` exercise the slug-anchored `git log -S` recovery primitive.
- [x] Migration on ark upgrade: PASS — `migrate_in_flight_verify_files` regenerates legacy verdict-shaped VERIFY.md files; preserves prior V-NNN findings verbatim; idempotent on re-run.
- [x] Workflow doc, AGENTS.md, slash-command templates updated: PASS — lockstep updates committed across `templates/ark/workflow.md`, `.ark/workflow.md`, `AGENTS.md`, three platforms.

## Plan Fidelity

- [x] G-1 (Phase::Committed + transition table): PASS — `commands/agent/state.rs`; tests in same file.
- [x] G-2 (start_head + committed_at fields): PASS — `commands/agent/state.rs::TaskToml`; `task_new::capture_start_head`; round-trip test.
- [x] G-3 (task_commit atomic closure with RollbackGuard): PASS — `commands/agent/task/commit.rs`; end-to-end git tests cover happy paths + pre-commit hook rollback + targeted unstage preserving user index.
- [x] G-4 (VERIFY living document): PASS — `templates/ark/templates/VERIFY.md` rewritten; `task_verify` overlays markers via `verify_seed::render_seeded_verify`.
- [x] G-5 (journal entry shape + slug-anchored recovery): PASS — `JournalEntry.start_head` + `.base_branch` fields; render_entry emits when Some; three e2e tests exercise the recovery primitive.
- [x] G-6 (ark archive top-level CLI; archive_move side-effect-free): PASS — `commands/archive.rs::ark_archive`; `commands/agent/task/archive.rs::task_archive_move`; six tests cover skip-non-committed, committed_at-month, --month filter, --dry-run, no-journal, no-spec.
- [x] G-7 (slash command surface lockstep): PASS — three platforms × `commit.md` exist; `archive.md` deleted across all three; design.md + quick.md updated. Existing parity tests pass.
- [x] G-8 (workflow.md + AGENTS.md): PASS — both updated; tier table reflects new path-through-states; lifecycle ASCII shows COMMIT block.
- [x] G-9 (CLI subcommands `commit` + `archive`): PASS — `agent_cli.rs` adds Commit + replaces Archive with `--month` defaulting via `derive_archive_month`.
- [x] G-10 (ark upgrade migration): PASS — `commands/upgrade/verify_migration.rs`; legacy verdict heuristic; idempotency + skip-non-verify tests.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — this task introduces a new feature SPEC (`ark-workflow-refactor/SPEC.md`) on closure rather than modifying existing ones. The deep-tier `task_commit` will extract the SPEC from `02_PLAN.md`'s `## Spec` section and register it in features INDEX. No CHANGELOG-on-existing-SPEC scenario applies.

## Findings

### V-001 Plan-goals parser tolerates two bullet styles

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/verify_seed.rs::read_plan_goals`
- **Problem:** Post-refactor PLAN style uses bolded bullets (`- **G-N: ...**`); pre-existing PLANs use bare bullets (`- G-N: ...`).
- **Why it matters:** Without tolerating both, the seed protocol would mis-populate Plan Fidelity sections for older tasks.
- **Recommendation:** Accept both forms in the parser; add a regression test.
- **Resolution:** FIXED in this iteration; test `read_plan_goals_accepts_bolded_bullets` covers.

### V-002 Project-INDEX parser uses the table convention, not H2 headings

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/verify_seed.rs::read_project_specs`
- **Problem:** Initial draft assumed `## <spec-name>` H2 headings per registered SPEC; the actual project INDEX uses a markdown table under `## Index`.
- **Why it matters:** Discovered during the smoke test — without this fix the Project Spec Compliance section would seed with `Index` and `How to Use` instead of the four real SPECs.
- **Recommendation:** Parse the `## Index` section's first-column backtick-wrapped names.
- **Resolution:** FIXED in this iteration; smoke test confirmed correct seeding.

### V-003 RollbackGuard restore order lives in comments, not types

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/task/commit.rs::RollbackGuard::restore`
- **Problem:** The guard records snapshots in step order, but the restore order in `Drop` is hard-coded reverse-of-recording. The ordering invariant lives in code comments rather than in the type system.
- **Why it matters:** Future contributors editing the guard could break restore correctness silently.
- **Recommendation:** Either capture the snapshot sequence in a `Vec<RollbackOp>` so restore is literally reverse-iteration, or document the invariant more strongly. Defer until a real bug surfaces.
- **Resolution:** ACCEPTED — the current shape passes all rollback tests (pre-commit hook, partial SPEC writes); the comment in `restore()` documents the order. A hardening refactor is a follow-up if the failure mode actually appears.

### V-004 Smoke test exposed parser robustness gaps

- **Severity:** LOW
- **Location:** smoke-test path against this very task
- **Problem:** Two parser bugs (V-001, V-002) only surfaced when the seed ran against the real project INDEX and PRD. Unit tests with synthetic fixtures missed them.
- **Why it matters:** Real-world fixtures catch what synthetic ones don't.
- **Recommendation:** When future PRs touch verify_seed parsers, run the binary against a real project to validate.
- **Resolution:** FIXED in this iteration. The smoke test of this very refactor closing itself is the proof.

## Notes

The slug-anchored `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal>` recovery primitive is the cleanest replacement for the impossible inline-SHA invariant from `00_PLAN`. Three iterations of plan revision — and the codex review that flagged path-only `git log` as unreliable — were necessary to converge on it. The chain R-001 to R-104 to R-201 is preserved in the PRD for posterity.

The `RollbackGuard` RAII pattern is a good fit for Rust's drop semantics. A future refactor could promote it to a more general utility (e.g. `commands::common::FileSnapshotSet`) if other commands need similar atomicity.

Bulk archive happens out-of-band of slash commands now. Manager workflow: run `ark archive --dry-run` to inspect, then `ark archive` to commit. The month bucket comes from each task's `committed_at`, so historical placement is deterministic.

Concurrent `/ark:commit` on the same checkout is unsupported (NG-12). The workspace journal's session-number assignment is best-effort; two simultaneous commits can produce duplicate session numbers. Documented in `--help` text and the workflow doc. A future task can add a per-checkout commit lock if the failure surfaces.
