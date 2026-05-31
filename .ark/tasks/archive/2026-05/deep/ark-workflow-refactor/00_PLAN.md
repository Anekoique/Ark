# `ark-workflow-refactor` PLAN `00`

> Status: Draft
> Feature: `ark-workflow-refactor`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`
> - Related Specs: `workspace`, `task-concurrency-control`, `ark-agent-namespace`, `ark-context`, `ark-upgrade`, `worktree`, `codex-support`, `opencode-support`, `project-spec`

---

## Summary

Replace the verdict-driven `VERIFY → ARCHIVE` closure with a living-document `VERIFY` and an atomic `/ark:commit` step, demote per-task archive from a slash command to a manager-only `ark archive` bulk CLI, and capture an exact `start_head..HEAD` commit range for the workspace journal at task creation time. The refactor preserves REVIEW (pre-execute, plan-soundness gate) as a separate phase from VERIFY (post-execute, implementation audit). It deliberately scopes out user-defined workflow chains and version-bump-driven archive triggers.

The change is broad but mechanically modest: one new `Phase` variant (`Committed`), one new TOML field (`start_head`), one new public function (`task_commit`), one renamed/relocated archive entry point (top-level `ark archive`), one rewritten template (`VERIFY.md`), one rewritten slash command (`/ark:commit`, replacing `/ark:archive`'s slot), and lock-step updates across Claude / Codex / OpenCode templates plus `workflow.md` / `AGENTS.md`. The journal-write path swaps `git log -n 20` for an exact `start_head..HEAD` range and falls back gracefully on pre-refactor tasks lacking `start_head`. State-file reconcile gains a single rule: `phase = Committed` keeps a slug in `tasks.active` (committed but not yet archived).

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- **G-1: Phase enum gains `Committed`; legal-transition table updated.** `Phase` (in `crates/ark-core/src/commands/agent/state.rs`) gains a single new variant `Committed` (lowercase serde rename `committed`). The legal-transition table in `can_transition` is updated:
  - **Add:** `(Quick, Execute, Committed)`, `(Standard, Verify, Committed)`, `(Deep, Verify, Committed)`, `(Quick, Committed, Archived)`, `(Standard, Committed, Archived)`, `(Deep, Committed, Archived)`.
  - **Remove:** `(Quick, Execute, Archived)`, `(Standard, Verify, Archived)`, `(Deep, Verify, Archived)`. After the refactor, `Archived` is reachable only from `Committed`.
  - **Status derivation** (`TaskToml::status`): `Phase::Committed` → `Status::InProgress` (committed but not yet archived); `Phase::Archived` → `Status::Completed`.
  - The state-file reconcile loop (`state_file::reconcile::reconcile_against_disk`) keeps a slug in `state.tasks.active` when its `task.toml.phase` is anything except `Archived`. `Committed` is included; the existing add-pass and drop-pass logic require no change beyond accepting `Committed` as "active."

- **G-2: `task.toml` gains `start_head`.** New optional field `start_head: Option<String>` on `TaskToml`. Captured at `task new` time as the parent checkout's HEAD SHA via `git rev-parse HEAD` (resolved from `opts.project_root`, **not** from inside the worktree, because the worktree is created later in the flow and its HEAD is the same SHA at the moment of creation). On `task new --worktree`, the same SHA is captured: `start_head` is the SHA of the base branch's HEAD at the moment the worktree was created — `git worktree add -b <branch> <path> <base>` resolves `<base>` to a SHA and the worktree's HEAD starts there. The captured value is stored verbatim. `#[serde(skip_serializing_if = "Option::is_none", default)]` so pre-refactor `task.toml`s deserialize cleanly with `start_head = None` and the migration path is read-tolerant.

- **G-3: `task_commit` is the atomic closure.** New public function `task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>` in `crates/ark-core/src/commands/agent/task/commit.rs`. The function performs five steps, in order, abort-on-first-failure with documented partial-state recovery:
  1. **Phase precondition.** Load `TaskToml` from the active task dir (slug-resolved from `.ark/.state.toml` or `--slug`). Reject unless `(tier, phase)` is one of `(Quick, Execute)`, `(Standard, Verify)`, `(Deep, Verify)`. Wrong phase → `Error::IllegalPhaseTransition`.
  2. **Working-tree precondition.** Run `git status --porcelain` from the task's cwd (worktree path if `task.toml.worktree_path` is set, else `layout.root()`). If output is empty → `Error::NothingToCommit { slug }`. Skipped only when `opts.no_commit == true`. Skipped also when `opts.no_commit == true && tier == Deep` so SPEC extract still runs.
  3. **VERIFY gate (tier-conditional).** Standard/Deep only: parse `VERIFY.md` (see G-4 for the parser) and inspect every checklist item and finding. **Deep tier**: refuse if any item or finding is `PENDING` → `Error::VerifyIncomplete { pending_items, pending_findings }`. **Standard tier**: emit one stderr warning per pending item/finding, do not refuse. **Quick tier**: no VERIFY exists; skip the gate entirely.
  4. **Deep-tier SPEC extraction.** Deep tier only: invoke `spec_extract` + `spec_register` on the *active* task dir (not the archive path; the task is not archived yet). Re-uses the existing functions in `commands/agent/spec/{extract,register}.rs`. The SPEC lands at `specs/features/<slug>/SPEC.md` and the row is upserted in `specs/features/INDEX.md`. Idempotent (existing SPEC gets a `[**CHANGELOG**]` row appended; existing INDEX row is updated in place).
  5. **Atomic commit + journal write.** When `opts.no_commit == false`:
     a. Compute `commit_range = "<start_head>..HEAD"` after the commit. The commit message is `opts.message` (verbatim if `Some`) or generated by the slash-command-side caller and passed in as `Some(...)` (the agent runs `git log -n 5 --oneline` style probe and proposes a conventional-commits message; the CLI does not invent messages itself — see C-3).
     b. Render the journal entry payload first (without the `commit_range` final HEAD), build a `JournalEntry` with `JournalKind::Task { slug }`, and write it to `<dev>/journal-N.md` via the existing `record_task` shared write path. The journal write happens *before* the git commit so the rendered entry is part of the working tree the commit captures.
     c. Run `git commit -m <message>` from the task's cwd. The commit captures both the working-tree work and the just-written journal entry. Pre-commit hooks may reject — surface the failure verbatim.
     d. After the commit lands, re-resolve HEAD (`git rev-parse HEAD`) and patch the `commit_range` field of the just-written journal entry by editing the rendered text in place — see G-5 for the commit-range patching protocol.
     e. Transition `task.toml.phase = Committed`, set `committed_at = now`, `updated_at = now`. Save.
  6. When `opts.no_commit == true`: skip steps 5a–5d entirely. Still run step 4 (SPEC extract) and step 5e (phase transition to `Committed`). Step 2's working-tree-non-empty precondition is **also** skipped under `--no-commit` (the user is explicitly opting out of ark's commit). The user is responsible for their own commit + journal record.

- **G-4: `VERIFY.md` is a living checklist + findings document.** The template at `.ark/templates/VERIFY.md` is rewritten to six sections in this fixed order: `## Project Spec Compliance`, `## Related Feature Spec Compliance`, `## PRD Constraints`, `## Plan Fidelity`, `## SPEC Drift`, `## Findings`, `## Notes`. The first four sections are **dynamically seeded at `ark agent task verify` time** by inspecting (a) `.ark/specs/project/INDEX.md`, (b) the PRD's `[**Related Specs**]` block, (c) the PRD's `[**Outcome**]` block, (d) the latest `NN_PLAN.md`'s `## Spec` Goals (G-N) section. Each seeded bullet renders as `- [ ] <item>: PENDING`. The fifth section (SPEC Drift) is fixed-content (one bullet: `- [ ] Modified feature SPECs have CHANGELOG entries: PENDING`). The sixth section (Findings) is empty at seed time; the implementer adds `### V-NNN — <title>` blocks as they audit. Each finding has Severity / Location / Problem / Why it matters / Recommendation / Resolution. The `## Notes` section is free-form, empty at seed time. **No verdict line, no decision enum.** Document is "complete" iff every checklist item is in `{PASS, FAIL: <reason>, N/A: <reason>}` and every finding's Resolution is in `{FIXED in <commit-or-section>, ACCEPTED — <reason>}` (no `PENDING` remaining). The legacy `VERIFY.md` (verdict-driven) is deleted from `templates/ark/templates/`.

- **G-5: Exact `commit_range` in journal entries.** The per-task journal entry rendered by `record_task` includes a new field `commit_range: Option<String>` rendered as `**Commit Range**: \`<start_head>..<HEAD>\`` between the `**Branch**` field and the `### Summary` heading. When `start_head` is absent (pre-refactor tasks), the field is rendered as `**Commit Range**: \`(unknown — pre-refactor task)\`` and `record_task` falls back to `git log -n 20` for the commits table. When `start_head` is present, the commits table is populated by `git log <start_head>..HEAD --oneline -n 20`. **`task_commit` patches `commit_range`'s HEAD half post-commit** by re-reading the journal file, locating the just-written entry by its `## Session N: <title>` anchor, and replacing the placeholder `<HEAD-PENDING>` token with the real SHA — single regex-substitution write, atomic at the OS level for a single `write_all`. The token is unique per entry by construction (it's literal text, only present in the just-written entry). Manual `/ark:record` path is unchanged: it omits the `**Commit Range**` field entirely (manual entries have no defined start point).

- **G-6: `ark archive` is a top-level manager-only CLI.** New top-level subcommand `ark archive [--dry-run] [--month YYYY-MM]` in `crates/ark-cli/src/main.rs` (peer of `ark init`, `ark unload`, `ark upgrade`, `ark context`). It is **not** under `ark agent` — it is a stable, semver-tracked top-level surface. The implementation lives in `crates/ark-core/src/commands/archive.rs` (new module, peer of `init.rs` / `unload.rs` / `upgrade.rs`). Behavior:
  - Enumerate `.ark/tasks/<slug>/task.toml` excluding `.ark/tasks/archive/`. For each task with `phase = Committed`, derive the YYYY-MM month from `committed_at` (the `task.toml` field set by `task_commit`), and call the existing `task_archive` helper with the slug. `task_archive` already handles the rename + INDEX update + state-file cleanup.
  - `--month YYYY-MM` filters to only archive tasks whose `committed_at` falls in the named month. Default: archive every committed task regardless of month.
  - `--dry-run` lists what would move without performing the move; prints `<slug> -> .ark/tasks/archive/YYYY-MM/<slug>` per row.
  - Idempotent. Tasks not in `Committed` phase are skipped silently. Re-running yields the same result.
  - Exits non-zero if any individual archive fails (partial archive: report each failure; continue processing remaining tasks; final exit code reflects whether any failed).

- **G-7: Slash-command and skill template surface.** Across `templates/claude/commands/ark/`, `templates/codex/skills/`, `templates/opencode/commands/ark/`:
  - **Add** `commit.md` (Claude + OpenCode) and `ark-commit/SKILL.md` (Codex). Body: parse `$ARGUMENTS` for `-m "<msg>"` and `--no-commit`; pull `ark context --scope phase --for commit`; if `-m` absent and `--no-commit` absent, agent generates a conventional-commits message from staged diff + recent `git log` style and shows it for confirmation/edit; invoke `ark agent task commit --message "<m>" [--no-commit]`. Wrap-up reports the commit SHA, journal session number, and (deep tier) the promoted SPEC path.
  - **Remove** `archive.md` (Claude + OpenCode) and `ark-archive/SKILL.md` (Codex). The user-facing slash command `/ark:archive` is deleted across all three platforms in lockstep.
  - **Update** `design.md`, `quick.md` (Claude + OpenCode) and `ark-design/SKILL.md`, `ark-quick/SKILL.md` (Codex). Replace the "tell user to run `/ark:archive`" closure step with "tell user to run `/ark:commit`."
  - **Update** `record.md` and `ark-record/SKILL.md`. No behavioral change; mention only that the manual record path is unaffected by the refactor (i.e. `/ark:record` does not interact with the new `commit_range` field).
  - **Lockstep rule (per `codex-support` and `opencode-support` SPECs):** any change to one platform's command body lands as a parallel edit on the other two. Reviewer enforces parity by diffing the three.

- **G-8: `workflow.md` and `AGENTS.md` updated.** `templates/ark/workflow.md` and the in-repo `.ark/workflow.md` are updated in lockstep:
  - §3 Tiers table: replace `archived` with `committed → archived` in the path-through-states column. Replace the `Path through states` cell for each tier as: `quick: design → execute → commit → archived`; `standard: design → plan → execute → verify → commit → archived`; `deep: design → plan ⇄ review → execute → verify → commit → archived`.
  - §4 Lifecycle ASCII diagram: replace the `ARCHIVE` block at the bottom with a `COMMIT` block (atomic: SPEC extract on deep, commit + journal write), followed by a separate post-block annotation that bulk archive happens later via `ark archive`.
  - §4 stage descriptions: rewrite the VERIFY description to match G-4 (living document, no verdict). Add a new `COMMIT` section. Remove the standalone `ARCHIVE` section; replace with a one-paragraph `Bulk Archive` note pointing at `ark archive`.
  - §6 Mechanics: add `ark archive` to the top-level CLI list. Note that `ark agent task commit` is the structural mutation invoked by `/ark:commit` (hidden, non-semver). Note that the per-task `/ark:archive` slash command is removed.
  - `AGENTS.md`: update the slash-command table — drop `/ark:archive` row, add `/ark:commit` row. Drop any reference to VERIFY's verdict.

- **G-9: `ark agent task commit` CLI subcommand.** In `crates/ark-cli/src/agent_cli.rs`'s `TaskCommand` enum, add a new variant `Commit(TaskCommitCliArgs)` with flags `--message <msg>` (`-m` short alias) and `--no-commit`. Dispatch wires through to `ark_core::task_commit`. The existing `Archive(TaskSlugArgs)` variant **is retained** for use by `ark archive` (it calls the same underlying `task_archive` function); only its discoverability via slash commands is removed (G-7).

- **G-10: Migration on `ark upgrade`.** `crates/ark-core/src/commands/upgrade.rs` gains one migration step:
  - **Slash-command refresh:** the existing template-overwrite pass for `templates/claude/commands/ark/`, `templates/codex/skills/`, `templates/opencode/commands/ark/` already replaces files in lockstep with the embedded templates on each `ark upgrade`. The refactor only changes the embedded set (G-7); the upgrade machinery requires no logic change. Removed slash commands (`archive.md`) leave orphan files on disk; upgrade prints a one-line stderr note `removed obsolete slash command: <path>` and unlinks each.
  - **In-flight `VERIFY.md` regeneration:** for any task with `task.toml.phase ∈ {Verify, Committed}` and a `VERIFY.md` whose top-level structure matches the legacy verdict-driven template (heuristic: contains `## Verdict` heading), rewrite the file using the new template seeded from the live PRD + project specs + plan. **Preserve any V-NNN findings** by parsing the legacy `## Findings` section and re-emitting them under the new `## Findings` section verbatim. The legacy `## Verdict` line is dropped (logged to stderr as `dropped legacy verdict: <task>`). If the legacy `Findings` section is empty or unparseable, regenerate from scratch (no findings carried forward).
  - **No phase rename, no `start_head` backfill.** Pre-refactor tasks keep `start_head = None`; their `task_commit` runs degrade to `git log -n 20` for the commits table (per G-5). No hand-rolled SHA reconstruction.

- **NG-1: User-defined workflow chains.** A `[workflow]` section in `.ark/config.toml` is **out of scope.** The `workflow.md` doc may sketch the future shape so the new defaults don't paint into a corner, but no parsing, validation, or runtime substitution is implemented here.
- **NG-2: Bulk archive triggered by version bump.** `ark archive` is invoked manually only. No `ark version bump` integration, no release-tag triggers.
- **NG-3: REVIEW phase shape.** No changes to REVIEW (pre-execute, plan-soundness gate, deep-only, iterative). Same `NN_REVIEW.md` numbering, same template, same `task review` transition. The locked-design "REVIEW + VERIFY stay separate" rule is preserved.
- **NG-4: REVIEW iteration loop.** No changes to the `Review → Plan` iterate transition (deep tier).
- **NG-5: Commit message authorship.** Ark does **not** generate commit messages itself. The slash command's body instructs the agent to generate one (and show it for confirmation); `task_commit` accepts `message: String` from its caller and uses it verbatim. This keeps `task_commit` deterministic and testable.
- **NG-6: VERIFY verdict preservation in archive.** Once a task is `Committed`, the legacy `## Verdict` header (if it existed pre-refactor) is dropped from the migrated `VERIFY.md`. No archived verdict is preserved.
- **NG-7: Reopen flow for committed tasks.** A task in `phase = Committed` cannot be "reopened" mid-state — it must be either `ark archive`'d to terminal `Archived` first, or its `task.toml` hand-edited back to `Verify` (the existing reopen escape hatch, undocumented but supported). No new reopen API.
- **NG-8: Hooks for commit.** No `pre-commit` / `post-commit` hook integration with `task_commit`. Standard git hooks fire as usual when `git commit` runs in step 5c; ark does not add its own hook layer.
- **NG-9: `--no-commit` for non-deep tiers.** `--no-commit` is accepted on all tiers but its effect on non-deep tiers is limited: it skips the working-tree-non-empty precondition and the commit/journal write, but quick/standard tiers have no SPEC to extract, so the user gets only the `phase = Committed` transition. CLI emits one stderr note `--no-commit on <tier> tier: only phase transition recorded`.
- **NG-10: Tracking PR-review revise commits in the journal.** The journal entry's `commit_range` captures `start_head..HEAD-at-task-commit`. Subsequent revise commits made on the branch (e.g. after PR review) are **not** retroactively folded into the entry; the implementer can run `/ark:record` manually to log a follow-up entry, or accept that the journal records the task's first-merge state. This is a feature, not a bug — the journal is a point-in-time record.
- **NG-11: Concurrent `ark archive` invocations.** No new locking beyond what `task_archive` already does (the state-file lock + per-slug rename atomicity). Two repo managers running `ark archive` concurrently may race on a single slug; the loser sees `TaskAlreadyExists` (in the destination) and aborts that slug; other slugs proceed. Idempotent re-runs converge.

[**Architecture**]

```text
crates/
├── ark-cli/src/
│   ├── main.rs                                  ─ ADD top-level `Archive(ArchiveCliArgs)` variant
│   │                                              under the root `Command` enum + dispatch.
│   └── agent_cli.rs                             ─ ADD `Commit(TaskCommitCliArgs)` to TaskCommand;
│                                                  drop the slash-command-driven path that
│                                                  previously called `task_archive` directly via
│                                                  `Archive(TaskSlugArgs)` — that variant remains
│                                                  in the enum because the new top-level
│                                                  `ark archive` calls into `task_archive` for each
│                                                  committed slug, but the agent-CLI surface is no
│                                                  longer the user-facing path.
└── ark-core/src/
    ├── commands/
    │   ├── archive.rs                           ─ NEW: pub fn ark_archive(opts) -> Result<…>
    │   │                                          enumerates .ark/tasks/, filters phase=Committed,
    │   │                                          calls task::archive::task_archive per slug.
    │   ├── upgrade.rs                           ─ MOD: VERIFY.md migration step (G-10);
    │   │                                          orphan-slash-command unlink + stderr note.
    │   ├── agent/
    │   │   ├── state.rs                         ─ MOD: Phase::Committed variant; can_transition
    │   │   │                                       table updated per G-1; status() handles
    │   │   │                                       Committed = InProgress.
    │   │   ├── task/
    │   │   │   ├── commit.rs                    ─ NEW: pub fn task_commit + TaskCommitOptions
    │   │   │   │                                  + TaskCommitSummary; verify-gate parser;
    │   │   │   │                                  start_head..HEAD range computation; atomic
    │   │   │   │                                  commit + journal write protocol (G-3, G-5).
    │   │   │   ├── new.rs                       ─ MOD: build_task_toml captures start_head via
    │   │   │   │                                  git rev-parse HEAD on opts.project_root; both
    │   │   │   │                                  no-worktree and worktree paths.
    │   │   │   ├── archive.rs                   ─ MOD: precondition check accepts only
    │   │   │   │                                  phase=Committed (was: Verify on standard/deep,
    │   │   │   │                                  Execute on quick); Phase::Archived destination
    │   │   │   │                                  unchanged. The function is now a manager helper
    │   │   │   │                                  invoked by `ark archive`, not by slash commands.
    │   │   │   ├── phase.rs                     ─ MOD: artifact_for(Phase::Committed, _) → None
    │   │   │   │                                  (no template seed for the Committed phase).
    │   │   │   └── mod.rs                       ─ MOD: pub mod commit; pub use commit::*;
    │   │   ├── workspace/
    │   │   │   ├── journal.rs                   ─ MOD: JournalEntry.commit_range:
    │   │   │   │                                  Option<String>; render_entry emits
    │   │   │   │                                  **Commit Range** field for task entries when
    │   │   │   │                                  Some; HEAD-pending placeholder protocol.
    │   │   │   └── record.rs                    ─ MOD: collect_commits_for_task accepts
    │   │   │                                      start_head: Option<&str>; falls back to
    │   │   │                                      base_branch range or -n 20 when None.
    │   │   │                                      record_task signature gains start_head field
    │   │   │                                      on RecordTaskOptions.
    │   │   └── (no other agent modules touched)
    │   └── context/projection.rs                ─ MOD: PhaseFilter::Commit variant;
    │                                              for_phase(Commit) returns latest VERIFY +
    │                                              latest PLAN + project specs + git state.
    └── lib.rs                                   ─ ADD pub re-exports: ark_archive,
                                                   ArchiveOptions, ArchiveSummary, task_commit,
                                                   TaskCommitOptions, TaskCommitSummary.

templates/
├── ark/
│   ├── templates/
│   │   ├── VERIFY.md                            ─ REWRITE: six-section living document
│   │   │                                          (per G-4); no Verdict.
│   │   └── (PRD/PLAN/REVIEW/SPEC unchanged)
│   ├── workflow.md                              ─ MOD: per G-8.
│   └── (config.toml unchanged — no [workflow] yet per NG-1)
├── claude/commands/ark/
│   ├── commit.md                                ─ NEW
│   ├── archive.md                               ─ DELETE
│   └── design.md / quick.md / record.md         ─ MOD: closure step → /ark:commit
├── codex/skills/
│   ├── ark-commit/SKILL.md                      ─ NEW
│   ├── ark-archive/SKILL.md                     ─ DELETE
│   └── ark-design / ark-quick / ark-record      ─ MOD: closure step → /ark:commit
└── opencode/commands/ark/
    ├── commit.md                                ─ NEW
    ├── archive.md                               ─ DELETE
    └── design.md / quick.md / record.md         ─ MOD: closure step → /ark:commit

AGENTS.md                                        ─ MOD: drop /ark:archive row, add /ark:commit row.
```

**Module coupling.**

- `commands/archive.rs` (new top-level) imports `commands::agent::task::archive::task_archive` (existing helper) and `commands::agent::state::{TaskToml, Phase}`. One-way: top-level `archive` → `agent::task::archive`.
- `commands/agent/task/commit.rs` (new) imports `commands::agent::state::{Phase, TaskToml, check_transition}`, `commands::agent::spec::{extract::spec_extract, register::spec_register}`, `commands::agent::workspace::{record_task, RecordTaskOptions}`, `io::{git::run_git, PathExt}`, `layout::Layout`. Does **not** import `task::archive`.
- `commands/agent/workspace/record.rs` gains a new optional `start_head` field on `RecordTaskOptions`; signature change ripples to the two existing call sites (`task::archive::task_archive` for legacy compatibility — passes `None` since archive runs *after* commit and the journal entry was already written; `task::commit::task_commit` for the new path — passes `task.toml.start_head.as_deref()`).
- `commands/upgrade.rs` imports `templates::ARK_TEMPLATES` and `state::{TaskToml, Phase}` for the VERIFY migration step. No new cross-module coupling.

**Call graph: `/ark:commit` → `task_commit`.**

```text
slash command /ark:commit
  ├── (agent generates message if -m absent; shows for confirm)
  └── ark agent task commit --message "<m>" [--no-commit] [--slug <s>]
        └── task_commit(opts)
              ├── slug ← resolve_slug(opts.slug, ppid)        [existing helper]
              ├── layout ← Layout::new(project_root)
              ├── task_dir ← layout.task_dir(&slug)
              ├── toml ← TaskToml::load(&task_dir)?
              ├── check_phase_for_commit(toml.tier, toml.phase)?
              │     ├── Quick + Execute → ok
              │     ├── Standard/Deep + Verify → ok
              │     └── _ → Error::IllegalPhaseTransition
              │
              ├── task_cwd ← worktree_path.unwrap_or(layout.root())
              │
              ├── if !opts.no_commit:
              │     status ← run_git(&["status","--porcelain"], task_cwd)
              │     if status.stdout.is_empty(): return Error::NothingToCommit{ slug }
              │
              ├── if tier in {Standard, Deep}:
              │     verify ← parse_verify_md(&task_dir.join("VERIFY.md"))?
              │     pendings ← collect_pending(&verify)
              │     match (tier, pendings.len()) {
              │         (Deep, n) if n > 0 → Error::VerifyIncomplete { items, findings },
              │         (Standard, n) if n > 0 → eprintln!(warn for each pending),
              │         _ → ok,
              │     }
              │
              ├── if tier == Deep:
              │     spec_extract(SpecExtractOptions {
              │         project_root, slug, plan_override: None,
              │         task_dir_override: Some(task_dir.clone()),
              │     })?
              │     spec_register(SpecRegisterOptions {
              │         project_root, feature: slug.clone(),
              │         scope: toml.title.clone(),
              │         from_task: slug.clone(),
              │         date: now.date_naive(),
              │     })?
              │
              ├── if !opts.no_commit:
              │     // 1. Render journal entry first (with HEAD-PENDING token)
              │     entry ← build_journal_entry(toml, opts.start_head, "<HEAD-PENDING>", commits_so_far)
              │     append entry to <dev>/journal-N.md (via record_task internals)
              │     index::rerender(&layout, &dev)
              │     // 2. Now commit work + journal in one git commit
              │     commit_msg ← opts.message.clone().ok_or(Error::CommitMessageRequired)?
              │     git_commit_out ← run_git(&["commit","-m",&commit_msg], task_cwd)
              │     if !git_commit_out.is_success(): return Error::GitCommitFailed{ stderr }
              │     // 3. Patch HEAD-PENDING with real SHA in the journal file
              │     head_sha ← run_git(&["rev-parse","HEAD"], task_cwd).stdout.trim()
              │     patch_journal_head_pending(&journal_path, &head_sha)?
              │
              ├── toml.phase = Phase::Committed
              ├── toml.committed_at = Some(now)
              ├── toml.updated_at = now
              ├── toml.save(&task_dir)?
              │
              └── return TaskCommitSummary { slug, tier, head_sha, journal_path, deep_spec_promoted }
```

**Call graph: `ark archive` → `task_archive`.**

```text
ark archive [--month YYYY-MM] [--dry-run]
  └── ark_archive(opts)
        ├── layout ← Layout::new(project_root)
        ├── candidates ← enumerate_committed(&layout)?
        │     // walks .ark/tasks/<slug>/task.toml, filters phase=Committed,
        │     // returns Vec<(slug, committed_at)>; tasks without committed_at are skipped.
        │
        ├── filtered ← match opts.month {
        │       Some(m) → candidates.filter(|(_, ca)| ca.format("%Y-%m") == m),
        │       None → candidates,
        │   }
        │
        ├── if opts.dry_run:
        │     for (slug, ca) in filtered:
        │         println!("{slug} -> .ark/tasks/archive/{}/{slug}", ca.format("%Y-%m"))
        │     return Ok(ArchiveSummary { count: 0, dry_run: true, … })
        │
        ├── successes ← Vec::new(); failures ← Vec::new()
        ├── for (slug, _ca) in filtered:
        │     match task_archive(TaskArchiveOptions {
        │         project_root: opts.project_root.clone(), slug: slug.clone(),
        │     }) {
        │         Ok(s) → successes.push(s),
        │         Err(e) → failures.push((slug, e)),
        │     }
        │
        └── return Ok(ArchiveSummary { successes, failures, dry_run: false })
              // Display impl prints one line per success, one line per failure;
              // CLI dispatcher converts non-empty failures into a non-zero exit code.
```

**Call graph: `ark agent task verify` (seeded VERIFY.md).**

```text
task_verify(opts)            // existing entry point; mod-only changes inside
  ├── (existing) check_transition(tier, Execute, Verify)
  ├── (existing) artifact_for(Verify, _) → ("VERIFY", "VERIFY.md")
  ├── if !VERIFY.md exists:
  │     // NEW seed protocol
  │     prd ← parse PRD.md for [**Outcome**] block + [**Related Specs**] block
  │     project_index ← parse .ark/specs/project/INDEX.md for SPEC rows
  │     plan_path ← latest NN_PLAN.md
  │     plan_goals ← parse plan_path's ## Spec section for Goal (G-N) bullets
  │     verify_text ← render_seeded_verify(SeedInputs {
  │         project_specs: project_index, related_specs: prd.related_specs,
  │         outcome: prd.outcome, plan_goals: plan_goals,
  │     })
  │     write VERIFY.md ← verify_text
  └── (existing) toml.phase = Verify; save
```

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Committed,    // NEW
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archived_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub committed_at: Option<DateTime<Utc>>,    // NEW
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub worktree_path: Option<std::path::PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub start_head: Option<String>,             // NEW (G-2)
}
```

```rust
// crates/ark-core/src/commands/agent/task/commit.rs (NEW)

#[derive(Debug, Clone)]
pub struct TaskCommitOptions {
    pub project_root: PathBuf,
    pub slug: String,
    /// Commit message. Required when `no_commit == false`.
    pub message: Option<String>,
    /// When true, skip git commit + journal write; deep tier still extracts SPEC.
    pub no_commit: bool,
}

#[derive(Debug, Clone)]
pub struct TaskCommitSummary {
    pub slug: String,
    pub tier: Tier,
    /// HEAD SHA after the commit, or None when --no-commit.
    pub head_sha: Option<String>,
    /// Journal file written, or None when --no-commit.
    pub journal_path: Option<PathBuf>,
    /// Session number assigned, or None when --no-commit.
    pub session_number: Option<u32>,
    /// True when deep-tier SPEC was extracted/registered.
    pub deep_spec_promoted: bool,
    /// VERIFY pending counts surfaced as a warning on standard tier.
    pub pending_verify: VerifyPendingCounts,
}

#[derive(Debug, Clone, Default)]
pub struct VerifyPendingCounts {
    pub items: u32,
    pub findings: u32,
}

impl fmt::Display for TaskCommitSummary { /* one-line summary */ }

pub fn task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary>;
```

```rust
// crates/ark-core/src/commands/agent/workspace/record.rs

#[derive(Debug, Clone)]
pub struct RecordTaskOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub branch: Option<String>,
    pub base_branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    /// NEW (G-5): exact start_head from task.toml. When Some, journal range is
    /// `<start_head>..HEAD`. When None, falls back to `base_branch..HEAD` then `-n 20`.
    pub start_head: Option<String>,
    /// Renamed: `archive_path` → `task_dir` (commit time the dir is still under
    /// `.ark/tasks/<slug>/`, not yet under `.ark/tasks/archive/`).
    pub task_dir: PathBuf,
    /// Renamed: `archived_at` → `recorded_at` (the function records on commit, not archive).
    pub recorded_at: DateTime<Utc>,
}
```

```rust
// crates/ark-core/src/commands/agent/workspace/journal.rs

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind: JournalKind,
    pub branch: Option<String>,
    /// NEW: rendered as `**Commit Range**: \`<start_head>..<HEAD>\`` for task entries.
    /// For `JournalKind::Manual`, always None (manual entries have no defined range).
    /// During `task_commit`, written first with the literal token `<HEAD-PENDING>` and
    /// patched in place after the git commit.
    pub commit_range: Option<String>,
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}

/// Constant token used by `task_commit` for post-commit HEAD patching.
pub const HEAD_PENDING_TOKEN: &str = "<HEAD-PENDING>";

/// Re-reads the journal file, finds the last `## Session N: <title>` block whose
/// rendered `**Commit Range**` field contains `HEAD_PENDING_TOKEN`, and replaces
/// the token with `head_sha`. Atomic: single read + single write_all on the
/// patched bytes.
pub fn patch_head_pending(path: &Path, head_sha: &str) -> Result<()>;
```

```rust
// crates/ark-core/src/commands/archive.rs (NEW)

#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    pub project_root: PathBuf,
    pub month: Option<String>,    // YYYY-MM filter
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveSummary {
    pub successes: Vec<TaskArchiveSummary>,
    pub failures: Vec<(String, Error)>,
    pub dry_run: bool,
}

impl fmt::Display for ArchiveSummary { /* multi-line: one per success, one per failure */ }

pub fn ark_archive(opts: ArchiveOptions) -> Result<ArchiveSummary>;
```

```rust
// crates/ark-core/src/error.rs (additions)

#[error("task `{slug}` cannot be committed with an empty working tree")]
NothingToCommit { slug: String },

#[error("VERIFY.md has {items} pending item(s) and {findings} pending finding(s); resolve before commit")]
VerifyIncomplete { items: u32, findings: u32 },

#[error("git commit failed: {stderr}")]
GitCommitFailed { stderr: String },

#[error("commit message is required (pass `-m` or generate one before invoking `task commit`)")]
CommitMessageRequired,
```

[**API Surface**]

```rust
// New public re-exports in `crates/ark-core/src/lib.rs`:
pub use commands::archive::{ArchiveOptions, ArchiveSummary, ark_archive};
pub use commands::agent::task::commit::{
    TaskCommitOptions, TaskCommitSummary, VerifyPendingCounts, task_commit,
};
```

```rust
// crates/ark-cli/src/main.rs — top-level Command enum gains:
enum Command {
    // ...existing...
    Archive(ArchiveCliArgs),    // NEW
}

#[derive(clap::Args)]
struct ArchiveCliArgs {
    #[arg(long)] month: Option<String>,
    #[arg(long, default_value_t = false)] dry_run: bool,
}
```

```rust
// crates/ark-cli/src/agent_cli.rs — TaskCommand enum gains:
enum TaskCommand {
    // ...existing variants Plan, Review, Execute, Verify, Archive (retained), New, Promote, Resume, Discard...
    /// Transition: -> Committed; deep tier extracts + registers SPEC; commits work + journal.
    Commit(TaskCommitCliArgs),    // NEW
}

#[derive(clap::Args)]
struct TaskCommitCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(short = 'm', long = "message")] message: Option<String>,
    #[arg(long = "no-commit", default_value_t = false)] no_commit: bool,
}
```

[**Constraints**]

- **C-1 (one-way coupling):** `commands/archive.rs` depends on `commands::agent::task::archive::task_archive`; the reverse is forbidden. `commands/agent/task/commit.rs` depends on `commands::agent::workspace::record_task` and `commands::agent::spec::{extract,register}`; the reverse is forbidden. `commands/agent/workspace/*` MUST NOT import `super::task` — the existing rule from the workspace SPEC is preserved.

- **C-2 (process spawn locality):** All git invocations under `commands/agent/task/commit.rs`, `commands/archive.rs`, and `commands/upgrade.rs`'s migration path route through `io::git::run_git`. `Command::new` MUST NOT appear in these new sites. Extends the existing source-scan test that already enforces this for workspace.

- **C-3 (commit message authorship):** `task_commit` does **not** generate commit messages. When `opts.no_commit == false`, `opts.message` MUST be `Some(_)`; `None` → `Error::CommitMessageRequired`. The slash command's body (Claude / Codex / OpenCode) is responsible for either passing `-m` through or generating a conventional-commits message and showing it for confirmation. This keeps `task_commit` deterministic and test-isolated from prompt-driven message generation.

- **C-4 (atomic commit + journal write ordering):** `task_commit`'s step 5 sequence (render journal entry → append to journal file → git commit → patch HEAD-PENDING) is **append-then-commit-then-patch**. Reordering breaks atomicity:
  - Append-first ensures the journal entry is in the working tree the git commit captures.
  - Patch-last is required because HEAD's SHA is not knowable before the commit lands.
  - If git commit fails (step 5c), the journal file already has the entry but with `<HEAD-PENDING>` unresolved. The implementer either retries `task_commit` (which would append a duplicate entry — the entry's session_number is monotonic, so this produces an explicit duplicate that the user can manually clean up, OR `task_commit` MUST detect a stale `<HEAD-PENDING>` from a prior failed run and refuse with `Error::JournalEntryStale` so the user can hand-clean before retrying. Choose the second: refuse + clean instructions in the error message).
  - If patch-HEAD-PENDING fails (step 5d, e.g. file system error), the commit landed but the journal has a literal `<HEAD-PENDING>` token. Surface as `Error::JournalPatchFailed`; the user can hand-edit the file or re-run `task_commit --no-commit` (which still writes phase=Committed, leaving the patch unfixed — this is acceptable; the journal is a record, not load-bearing for the state machine).

- **C-5 (`start_head` capture):** `task_new` captures `start_head` via `run_git(&["rev-parse","HEAD"], &opts.project_root)`. On unborn HEAD (fresh repo with no commits), `start_head = None`. On `--worktree` paths, the same call is made on `opts.project_root` (parent checkout), **before** `git worktree add`; the resolved SHA is the parent HEAD. The worktree's HEAD-after-add equals `start_head` because `git worktree add ... <base>` defaults to creating the worktree's branch at `<base>` (which is `git rev-parse HEAD` of the parent, by `resolve_base_branch`).

- **C-6 (state-file reconcile semantics):** `state_file::reconcile::reconcile_against_disk`'s drop-pass (existing rule: drop slug from `tasks.active` when `task.toml` missing or `phase == Archived`) keeps `phase == Committed` slugs in `tasks.active`. **No code change is required** because the existing predicate is `phase == Archived`, and `Committed` is a distinct enum value.

- **C-7 (VERIFY parser):** The verify-gate parser in `task_commit` (step 3) reads `VERIFY.md` and counts `PENDING` occurrences. A line is `PENDING` iff it matches `/^- \[ \] .+: PENDING$/` (checklist item) or appears under a `### V-NNN` heading with a `Resolution: PENDING` line. The parser is permissive about extra whitespace and case-sensitive on the literal `PENDING`. Out-of-section text is ignored. Notes section is never gated. The parser lives in `commands/agent/task/commit.rs` as a private helper `parse_verify_md`.

- **C-8 (slash-command lockstep):** Any change to `templates/claude/commands/ark/commit.md` lands as a parallel edit on `templates/codex/skills/ark-commit/SKILL.md` and `templates/opencode/commands/ark/commit.md`. Same body, same flag handling, same wrap-up reporting. Reviewer enforces parity by diffing the three.

- **C-9 (template ordering on disk):** `templates/ark/templates/VERIFY.md` is the source of truth for the new VERIFY shape. `task_verify`'s seed protocol parses this template, then *overlays* dynamic content (Project Spec Compliance, Related Feature Spec Compliance, PRD Constraints, Plan Fidelity bullets) by string-replacing four named markers `{{PROJECT_SPEC_COMPLIANCE}}`, `{{RELATED_FEATURE_COMPLIANCE}}`, `{{PRD_CONSTRAINTS}}`, `{{PLAN_FIDELITY}}` — see G-4 and the new VERIFY template body for the exact marker placement. If a marker is missing in the template (corrupt or hand-edited), seed errors with `Error::TemplateMarkerMissing { marker }`.

- **C-10 (path/io discipline):** All filesystem access in the new modules routes through `io::PathExt`. All `.ark/`-relative paths route through `Layout` helpers. No string concatenation for `.ark/tasks/<slug>` etc.

- **C-11 (CLI hidden-vs-visible split):** `ark archive` is **top-level and visible** in `ark --help` (peer of `init` / `unload` / `upgrade` / `context`). `ark agent task commit` is **hidden** under `ark agent` per the existing `ark-agent-namespace` SPEC: not in `ark --help`, reachable via `ark agent --help`. Help text for `ark archive` includes "Bulk-archives committed tasks; manager-only operation. Run after a release cut or whenever you want to consolidate completed work into the YYYY-MM archive directories."

- **C-12 (migration idempotency):** `ark upgrade`'s VERIFY-migration step is idempotent: re-running on an already-migrated VERIFY.md (one without `## Verdict`) is a no-op. The heuristic for "this is legacy" is the literal substring `## Verdict` near the top of the file; absence → already migrated. Orphan-slash-command unlink is idempotent: re-running silently skips files that no longer exist.

- **C-13 (auto-record absence on `--no-commit`):** When `task_commit` runs with `opts.no_commit = true`, the journal write is **fully skipped**. The user is responsible for invoking `/ark:record` themselves. CLI emits one stderr note `--no-commit: journal not written; run /ark:record manually if you want a session entry`.

- **C-14 (deep-tier `--no-commit` SPEC extraction):** When `task_commit` runs with `opts.no_commit = true && tier == Deep`, SPEC extraction (step 4) **still runs** unconditionally. Rationale: SPEC extraction is the deep-tier-defining mutation; opting out of the commit shouldn't opt out of the SPEC promotion. If the user wants neither, they should not have invoked `task_commit` at all.

- **C-15 (CHANGELOG entry on existing SPEC):** When `task_commit`'s deep-tier SPEC extraction step encounters an existing `specs/features/<slug>/SPEC.md`, it appends a `[**CHANGELOG**]` row. Format reuses the existing `spec_extract` CHANGELOG protocol from the `project-spec` SPEC. `task_commit` does not invent its own CHANGELOG format.

- **C-16 (commit_range absent in manual /ark:record):** The `/ark:record` slash-command path (manual journal entries via `workspace_record`) keeps `commit_range = None` and renders no `**Commit Range**` field. This preserves the `workspace` SPEC's behavior for manual entries.

- **C-17 (json schema for `ark context --for commit`):** New `PhaseFilter::Commit` variant in `commands/context/projection.rs`. The projection returns: current task + latest VERIFY.md (if exists) + latest NN_PLAN.md + project specs + git state. Rendered identically in JSON shape to `Verify` projection but with VERIFY.md content body included rather than just path.

- **C-18 (ark agent task archive retained, hidden):** The existing `ark agent task archive` subcommand and `task_archive` function are **retained** unchanged in their semantics. They are no longer surfaced through any slash command; they are invoked by `ark archive` (top-level CLI) per slug. The `ark agent task archive` CLI surface remains accessible to power users / debugging but its discoverability is reduced — no slash command points at it. This minimizes surface change at the cost of preserving an internal dual-call-path; the alternative (rename `task_archive` → `task_archive_internal` + delete the CLI variant) bloats the diff without changing user-visible behavior, so it is rejected.

- **C-19 (legal-transition table parity):** When G-1's transition table changes land, the `archived_is_terminal` test in `state.rs` is updated to assert that `Archived` remains terminal *and* that `Committed → Archived` is the only entry. New tests `*_committed_is_legal_destination` and `committed_archived_is_legal_destination` are added per tier.

- **C-20 (slash-command parity test):** A new test under `templates/` verifies that `templates/claude/commands/ark/commit.md`, `templates/codex/skills/ark-commit/SKILL.md`, and `templates/opencode/commands/ark/commit.md` exist and share the same hash on a normalized form (front-matter stripped, whitespace collapsed). Cribs from the existing `codex-support` / `opencode-support` parity machinery if present; otherwise added as a fresh assertion.

---

## Runtime `runtime logic`

[**Main Flow**]

1. **`/ark:design --deep <title>`** scaffolds a deep-tier task. `task new` captures `start_head` from parent HEAD into `task.toml.start_head`.
2. **`/ark:plan` → `/ark:review` ↔ `/ark:plan` → `/ark:execute`** unchanged. REVIEW iterates as today (deep only).
3. **`/ark:verify`** seeds `VERIFY.md` with auto-populated checklist sections. Implementer fills sections during EXECUTE → COMMIT, marking each item PASS/FAIL/N/A and adding V-NNN findings as needed.
4. **`/ark:commit -m "<msg>"`** invokes `ark agent task commit`. Atomic sequence:
   a. Load `task.toml`. Verify phase is `Verify` (or `Execute` for quick).
   b. Verify working tree non-empty (unless `--no-commit`).
   c. Parse VERIFY.md. Refuse if any PENDING (deep) or warn (standard).
   d. Deep tier: extract SPEC to `specs/features/<slug>/SPEC.md`, register in INDEX.
   e. Render journal entry with `commit_range = "<start_head>..<HEAD-PENDING>"`. Append to journal-N.md. Re-render index.
   f. Run `git commit -m "<msg>"` from the task's cwd.
   g. Re-resolve HEAD. Patch `<HEAD-PENDING>` token in journal file with real SHA.
   h. Save `task.toml.phase = Committed`, `committed_at = now`.
5. **Time passes.** PR review may add more commits to the branch. Multiple tasks accumulate in `phase = Committed`.
6. **`ark archive`** (manager-invoked, e.g. at release cut): enumerates all `phase = Committed` tasks, calls `task_archive` per slug, moves each to `tasks/archive/YYYY-MM/<slug>/` using the slug's `committed_at` for the YYYY-MM bucket.

[**Failure Flow**]

1. **`task new` git rev-parse fails (unborn HEAD).** `start_head = None` recorded. Task proceeds; `task_commit` later falls back to `git log -n 20`.
2. **`task_commit` step 2: working tree clean.** Hard error `NothingToCommit`. The user either has nothing to commit (workflow violation — they should have invoked `/ark:commit --no-commit` instead) or has not staged work.
3. **`task_commit` step 3: VERIFY incomplete (deep).** Hard error `VerifyIncomplete { items, findings }`. The user resolves PENDING entries and retries.
4. **`task_commit` step 4: spec_extract fails (deep).** SPEC file write fails (FS error, missing PLAN). Hard error; no commit occurs; phase remains `Verify`. User fixes the cause and retries.
5. **`task_commit` step 5b: journal append fails.** Hard error; commit not yet attempted; phase remains `Verify`. User fixes the cause and retries.
6. **`task_commit` step 5c: git commit fails (pre-commit hook rejects, etc.).** Journal entry already appended with `<HEAD-PENDING>` token. Hard error `GitCommitFailed { stderr }`. Phase remains `Verify`. **On retry**, `task_commit` detects the stale `<HEAD-PENDING>` token in the journal file and refuses with `JournalEntryStale { journal_path, token_count }`; the error message tells the user to either resolve the failed commit (e.g. fix the pre-commit hook) so the patch step can complete, or hand-remove the stale entry from the journal file before re-invoking.
7. **`task_commit` step 5d: patch HEAD fails.** Commit landed; journal has literal `<HEAD-PENDING>` token. Hard error `JournalPatchFailed`. Phase **does** transition to `Committed` (the commit is real). User hand-edits the journal file or re-runs `task_commit --no-commit` (which is effectively a no-op since phase is already Committed; emits stderr note).
8. **`ark archive` per-slug failure.** Continues processing remaining slugs; final summary lists each failure. Exit code is non-zero iff any failure occurred.

[**State Transitions**]

- `Phase::Verify → Phase::Committed` when `task_commit` runs successfully on a standard or deep task.
- `Phase::Execute → Phase::Committed` when `task_commit` runs successfully on a quick task.
- `Phase::Committed → Phase::Archived` when `task_archive` runs (invoked by `ark archive`).
- `Phase::Archived` remains terminal; no transitions out.
- All other transitions are unchanged from current.

---

## Implementation `split task into phases`

[**Phase 1 — State machine + start_head capture**]

Goal: land the foundational changes that don't break existing slash commands.

1. Add `Phase::Committed` to `crates/ark-core/src/commands/agent/state.rs`. Update `can_transition` per G-1. Update `archived_is_terminal` test. Add `*_committed_is_legal_destination` tests per tier. (`src/commands/agent/state.rs`)
2. Add `start_head: Option<String>` and `committed_at: Option<DateTime<Utc>>` to `TaskToml`. Update existing test `task_toml_loads_without_worktree_fields` to confirm new fields also default to `None`. (`src/commands/agent/state.rs`)
3. In `task_new::build_task_toml`, capture `start_head` via `run_git(&["rev-parse","HEAD"], opts.project_root)` (with `.ok().filter(is_success).map(stdout.trim)`). Pre-call to satisfy borrow rules; pass through into the struct. (`src/commands/agent/task/new.rs`)
4. Add unit tests: `task_new_captures_start_head`, `task_new_with_unborn_head_records_none`. (`src/commands/agent/task/new_tests.rs`)
5. **Verify CI passes with no slash-command behavior change.** No existing tests should break; only additions.

[**Phase 2 — `task_commit` and atomic-commit protocol**]

Goal: the core new entry point.

1. Create `crates/ark-core/src/commands/agent/task/commit.rs`. Implement `task_commit` with the five-step sequence (G-3, G-5). Use private helpers: `parse_verify_md` (C-7 parser), `build_commit_journal_entry`, `patch_head_pending` (delegates to `journal::patch_head_pending` for actual file edit). (`src/commands/agent/task/commit.rs`, `src/commands/agent/workspace/journal.rs`)
2. Add `pub mod commit;` and `pub use commit::*;` to `src/commands/agent/task/mod.rs`. Add re-exports to `lib.rs`.
3. Add `RecordTaskOptions.start_head: Option<String>`. Update `record_task` to thread `start_head` through to the `JournalEntry.commit_range` field (Some when start_head is Some, omitted otherwise). Adjust the existing `task::archive::task_archive` call site to pass `start_head: None` (archive runs after commit, journal already written). (`src/commands/agent/workspace/record.rs`)
4. Add `JournalEntry.commit_range: Option<String>` and update `render_entry` to emit `**Commit Range**: \`<value>\`` line between `**Branch**` and the first `### Summary` heading when `commit_range.is_some()`. Update the golden tests `render_entry_golden_task` and `render_entry_manual_uses_dash_slug`. (`src/commands/agent/workspace/journal.rs`)
5. Implement `journal::patch_head_pending(path, head_sha)` per the data-structure spec (locate by `## Session N: <title>` anchor + `<HEAD-PENDING>` token; single read+write_all). Add unit tests `patch_head_pending_replaces_token`, `patch_head_pending_idempotent_when_token_absent`, `patch_head_pending_errors_on_corrupt_journal`. (`src/commands/agent/workspace/journal.rs`)
6. Add error variants `NothingToCommit`, `VerifyIncomplete`, `GitCommitFailed`, `CommitMessageRequired`, `JournalEntryStale`, `JournalPatchFailed`, `TemplateMarkerMissing` to `src/error.rs`.
7. Wire `Commit(TaskCommitCliArgs)` into `crates/ark-cli/src/agent_cli.rs`'s `TaskCommand` enum + dispatcher.
8. Add integration tests under `commit.rs` covering each tier × `--no-commit` × VERIFY-pending matrix.
9. **Verify CI passes; the new `ark agent task commit` is callable end-to-end on a fresh task.**

[**Phase 3 — `ark archive` top-level CLI**]

Goal: replace the per-task archive slash command with the bulk manager CLI.

1. Create `crates/ark-core/src/commands/archive.rs`. Implement `ark_archive(opts)` per the architecture call graph. Reuse `task_archive` per slug. (`src/commands/archive.rs`)
2. Add `ArchiveOptions`, `ArchiveSummary` types + `Display` impl. Re-export from `lib.rs`.
3. Add `Command::Archive(ArchiveCliArgs)` to `crates/ark-cli/src/main.rs`'s top-level enum + dispatcher.
4. Add `task_archive`'s precondition update: change the `check_transition(toml.tier, toml.phase, Phase::Archived)` call from "phase must be Verify (or Execute on quick)" to "phase must be Committed." This is the point at which the *direct* `Verify → Archived` and `Execute → Archived` transitions are removed from the table. (`src/commands/agent/task/archive.rs`, `src/commands/agent/state.rs::can_transition`)
5. Add integration tests: `ark_archive_archives_committed_tasks`, `ark_archive_skips_uncommitted_tasks`, `ark_archive_dry_run_lists_only`, `ark_archive_filters_by_month`, `ark_archive_idempotent`.
6. **Verify CI passes; `ark archive` is end-to-end functional.** Existing per-task archive tests need updating to first run `task_commit` before `task_archive`.

[**Phase 4 — VERIFY template, seed protocol, and migration**]

Goal: ship the new VERIFY shape.

1. Rewrite `templates/ark/templates/VERIFY.md` per G-4 with the four marker tokens (`{{PROJECT_SPEC_COMPLIANCE}}`, `{{RELATED_FEATURE_COMPLIANCE}}`, `{{PRD_CONSTRAINTS}}`, `{{PLAN_FIDELITY}}`). Section ordering: Project Spec Compliance → Related Feature Spec Compliance → PRD Constraints → Plan Fidelity → SPEC Drift → Findings → Notes.
2. In `crates/ark-core/src/commands/agent/task/phase.rs`'s `task_verify` function (or its `artifact_for` seed step), inject the seed-time substitution: parse PRD.md, project INDEX, latest plan; call a new private helper `commands::agent::task::verify_seed::render_seeded_verify(...)`. (`src/commands/agent/task/phase.rs`, new helper module `src/commands/agent/task/verify_seed.rs`)
3. The seed helper returns the substituted text; phase.rs writes it via `write_file` with `WriteMode::Force` (replacing the just-copied template). (`src/commands/agent/task/phase.rs`)
4. Add unit tests for `render_seeded_verify` covering: (a) standard tier with two project specs + three related specs + PRD Outcome + plan with three goals; (b) deep tier; (c) missing PRD `[**Related Specs**]` block (renders empty section); (d) missing `{{PROJECT_SPEC_COMPLIANCE}}` marker (errors with `TemplateMarkerMissing`).
5. Update `crates/ark-core/src/commands/upgrade.rs` to migrate legacy in-flight VERIFY.md files per G-10. The migration is gated by detecting `## Verdict` near the top of the file; idempotent. Add `migrate_legacy_verify_md` private helper + unit tests.
6. Delete `templates/claude/commands/ark/archive.md`, `templates/codex/skills/ark-archive/SKILL.md`, `templates/opencode/commands/ark/archive.md`. Update the embedded-templates manifest if any.
7. **Verify CI passes; `ark agent task verify` produces the new VERIFY shape on a fresh task.**

[**Phase 5 — Slash command surface (Claude / Codex / OpenCode)**]

Goal: lockstep ship the user-facing surface.

1. Create `templates/claude/commands/ark/commit.md`, `templates/codex/skills/ark-commit/SKILL.md`, `templates/opencode/commands/ark/commit.md`. Bodies: parse `$ARGUMENTS` for `-m "<msg>"` and `--no-commit`; pull `ark context --scope phase --for commit`; if no `-m` and no `--no-commit`, generate a conventional-commits message from staged diff + recent `git log` style (sample command embedded in body); show for confirmation; invoke `ark agent task commit --message "<m>" [--no-commit]`. Wrap-up reports commit SHA, journal session number, deep-tier promoted SPEC path.
2. Update `templates/claude/commands/ark/design.md` (and Codex/OpenCode peers): replace step 5.2's "tell user to run `/ark:archive`" with "tell user to run `/ark:commit -m \"<message>\"`."
3. Update `templates/claude/commands/ark/quick.md` (and Codex/OpenCode peers): replace step 8 "tell user to run `/ark:archive`" with "tell user to run `/ark:commit -m \"<message>\"`."
4. Update `templates/claude/commands/ark/record.md` (and Codex/OpenCode peers): no behavioral change; brief note that manual record path is unaffected by the refactor.
5. Add `commands::agent::task::commit` to `ark context --for commit`'s projection in `commands/context/projection.rs` (PhaseFilter::Commit variant; same shape as Verify projection but bundles VERIFY.md content body).
6. **Verify CI passes; lockstep diff between the three platforms shows identical bodies (modulo the platform-specific frontmatter).**

[**Phase 6 — Workflow doc + AGENTS.md + cleanup**]

Goal: documentation parity.

1. Update `templates/ark/workflow.md` per G-8. Update `.ark/workflow.md` (the project's own copy) in lockstep.
2. Update `AGENTS.md`: drop `/ark:archive` row, add `/ark:commit` row.
3. Sweep tests across the codebase: any test that called `task_archive` directly (now requires `phase = Committed` precondition) must be updated to run `task_commit` first.
4. Update `concurrency_tests.rs` to verify that `Committed` tasks are kept in `tasks.active` by reconcile.
5. Run full test suite. Run `cargo clippy --all-targets`. Run `cargo fmt --check`.
6. **Verify the entire flow end-to-end on a fresh `tempdir`-backed install: design → plan → review → execute → verify → commit → archive (via `ark archive`) — assert all artifacts at each stage and the journal entry's commit_range field is exact.**

---

## Trade-offs `ask reviewer for advice`

- **T-1: Append-then-commit-then-patch (chosen) vs. amend-after-commit vs. separate journal commit.** Chosen path appends the journal entry to the working tree, commits work + journal in a single git commit, then patches the journal's `<HEAD-PENDING>` token with the real SHA post-commit. **Advantages:** one commit per task close; no amend (preserves remote-pushed safety); no separate "chore: journal" commit cluttering history. **Disadvantages:** the patch step is non-atomic with the commit (if patch fails, the journal has a stale token). **Rejected alternatives:**
  - *Amend after commit*: if the work was already pushed (PR-review revise scenarios), amend forces a `push --force`, which is fragile on shared branches.
  - *Separate journal commit*: clean and atomic, but adds an extra `chore(workspace): record <slug>` commit per task. The user previously rejected this as not elegant.

- **T-2: Phase enum addition (chosen) vs. directory-state-as-truth.** Chosen path adds a `Committed` variant to the `Phase` enum and a `committed_at` timestamp to `TaskToml`. **Advantages:** state-file reconcile already keys off `task.toml.phase`; the new variant slots in cleanly; reopen/migration is trivial because the phase value is durable. **Disadvantages:** one more variant to maintain in the legal-transition table and serde. **Rejected alternative:** treat "committed" as the absence of a `committed_at` timestamp on a `phase = Verify` task. Would couple two truth sources (phase enum + timestamp presence) and complicate reconcile.

- **T-3: Top-level `ark archive` (chosen) vs. `ark agent task archive` retained as user-facing.** Chosen: top-level `ark archive` is the manager-only entry point; the existing `ark agent task archive` is kept as an internal helper called by `ark archive`. **Advantages:** matches the user's model (archive is a manager bulk op, not a per-task slash). The two-tier visibility (top-level visible vs. agent hidden) is consistent with `ark agent`'s namespace conventions. **Disadvantages:** retains the dual call path (top-level → agent). **Rejected alternative:** rename `task_archive` → `task_archive_internal` and delete the agent CLI variant. Bloats the diff without changing user-visible behavior.

- **T-4: Tier-conditional VERIFY gate (deep refuses, standard warns) vs. uniform refuse.** Chosen: deep refuses on any pending; standard warns. **Advantages:** matches the user's intent that deep work demands rigor while standard accepts pragmatic completion. **Disadvantages:** asymmetric — a user moving a task from standard to deep mid-flight changes the gate behavior. **Rejected alternatives:** uniform refuse on all tiers (too rigid for standard); uniform warn on all tiers (loses the gate's teeth on deep, which is the tier most likely to harbor latent quality issues).

- **T-5: Slash command generates message (chosen) vs. CLI generates message vs. require explicit `-m`.** Chosen: slash command body instructs the agent to generate a conventional-commits message from staged diff + recent `git log` style and show it for confirmation; CLI is deterministic and only consumes the final message string. **Advantages:** keeps `task_commit` testable (no LLM dependency in core); the agent can use richer context (PRD, recent commits, conversation history) than a deterministic generator could. **Disadvantages:** the slash command's behavior depends on the agent's implementation; non-LLM tools cannot use the message-generation path and must pass `-m`. **Rejected alternatives:**
  - *CLI generates message*: would require either an embedded template engine or an external LLM call; both bloat the binary or introduce non-determinism.
  - *Require explicit `-m`*: high friction; the user explicitly asked for `-m` to be optional with agent-generated default.

- **T-6: HEAD-PENDING patch (chosen) vs. defer journal write to post-commit vs. commit then write journal in second commit.** Chosen: write the journal first (including a `<HEAD-PENDING>` placeholder), commit, then patch the placeholder. **Advantages:** the journal entry is part of the same commit as the work; the placeholder is a deterministic, single-character-class token unlikely to collide. **Disadvantages:** the patch step is a separate disk write; failure leaves a stale placeholder. **Rejected alternatives:**
  - *Defer journal write to post-commit*: the journal entry would land in a subsequent commit (or be uncommitted).
  - *Two commits*: rejected by the user as not elegant.

---

## Validation `test design`

[**Unit Tests**]

- **V-UT-1:** `Phase::Committed` round-trips through TOML serde (rename `committed`).
- **V-UT-2:** `can_transition(*, Verify, Committed)` and `can_transition(Quick, Execute, Committed)` are true; legacy `(Standard|Deep, Verify, Archived)` and `(Quick, Execute, Archived)` are false; only `(*, Committed, Archived)` reaches `Archived`.
- **V-UT-3:** `archived_is_terminal` test holds for the new transition table; `Committed` is not terminal.
- **V-UT-4:** `task_new` captures `start_head` via `git rev-parse HEAD` from `project_root`; pre-existing repo with one commit yields a non-None SHA.
- **V-UT-5:** `task_new` on an unborn HEAD records `start_head = None`.
- **V-UT-6:** `task.toml` round-trips with `start_head = Some("abc123")` and `committed_at = Some(now)`.
- **V-UT-7:** Pre-refactor `task.toml` (without `start_head` / `committed_at`) deserializes cleanly with both as `None`.
- **V-UT-8:** `task_commit` on standard tier from `Verify` phase with non-empty working tree + complete VERIFY.md transitions to `Committed`, captures `committed_at`, returns `head_sha = Some(...)`.
- **V-UT-9:** `task_commit` on deep tier with PENDING items in VERIFY.md returns `Error::VerifyIncomplete { items > 0 }`.
- **V-UT-10:** `task_commit` on standard tier with PENDING items emits stderr warnings and proceeds (does not error).
- **V-UT-11:** `task_commit` with empty working tree returns `Error::NothingToCommit`.
- **V-UT-12:** `task_commit --no-commit` on deep tier extracts SPEC and transitions to `Committed`, but does NOT run `git commit` and does NOT write the journal.
- **V-UT-13:** `task_commit` without `-m` and `no_commit = false` returns `Error::CommitMessageRequired`.
- **V-UT-14:** `task_commit` writes journal with `**Commit Range**: \`<start_head>..<head_sha>\`` rendered correctly.
- **V-UT-15:** `task_commit` on a pre-refactor task (`start_head = None`) writes journal with `**Commit Range**: \`(unknown — pre-refactor task)\``.
- **V-UT-16:** `journal::patch_head_pending` replaces the unique `<HEAD-PENDING>` token with the provided SHA.
- **V-UT-17:** `journal::patch_head_pending` is idempotent when token is absent (returns Ok without writing).
- **V-UT-18:** Detect-stale: `task_commit` refuses with `JournalEntryStale` when re-invoked after a prior failed commit left a `<HEAD-PENDING>` token in the journal.
- **V-UT-19:** `parse_verify_md` returns 0 pending on a fully-resolved checklist + findings document.
- **V-UT-20:** `parse_verify_md` returns N items when N checklist items remain `PENDING`.
- **V-UT-21:** `parse_verify_md` counts findings whose `Resolution: PENDING` line is present.
- **V-UT-22:** `render_seeded_verify` substitutes `{{PROJECT_SPEC_COMPLIANCE}}` with one bullet per project SPEC.
- **V-UT-23:** `render_seeded_verify` substitutes `{{RELATED_FEATURE_COMPLIANCE}}` from PRD's `[**Related Specs**]`.
- **V-UT-24:** `render_seeded_verify` errors with `TemplateMarkerMissing` if a marker is absent in the template.
- **V-UT-25:** `ark_archive` enumerates only `phase = Committed` tasks from `.ark/tasks/<slug>/task.toml`.
- **V-UT-26:** `ark_archive --month 2026-05` filters out tasks committed in other months.
- **V-UT-27:** `ark_archive --dry-run` does not move any directory; only prints the would-move plan.

[**Integration Tests**]

- **V-IT-1:** End-to-end deep-tier flow on a `tempdir` git repo: `task_new --tier deep --worktree → task_plan → task_review → task_execute → task_verify → task_commit → ark_archive` produces an archived task at `.ark/tasks/archive/YYYY-MM/<slug>/`, a feature SPEC at `.ark/specs/features/<slug>/SPEC.md`, an INDEX row, and a journal entry with exact `commit_range`.
- **V-IT-2:** End-to-end standard-tier flow: same as V-IT-1 minus review and SPEC promotion.
- **V-IT-3:** End-to-end quick-tier flow: `task_new → task_execute → task_commit → ark_archive`. No VERIFY, no SPEC promotion.
- **V-IT-4:** Multi-task `ark_archive`: three tasks committed in three different months; default `ark_archive` archives all three to their respective YYYY-MM buckets.
- **V-IT-5:** `ark_archive --month YYYY-MM` filters correctly.
- **V-IT-6:** Slash-command generation lockstep test: `templates/claude/commands/ark/commit.md`, `templates/codex/skills/ark-commit/SKILL.md`, `templates/opencode/commands/ark/commit.md` all exist; `archive.md` is absent in all three platforms.
- **V-IT-7:** `ark upgrade` migrates a legacy `VERIFY.md` (with `## Verdict`) to the new shape; preserves prior `## Findings` content.
- **V-IT-8:** `ark upgrade` unlinks orphan `archive.md` from a previously-installed `templates/claude/commands/ark/` site.
- **V-IT-9:** `ark context --for commit` returns latest VERIFY.md content + latest plan + project specs in a single JSON projection.
- **V-IT-10:** `state_file` reconcile keeps `phase = Committed` slugs in `tasks.active`; bulk `ark_archive` removes them only after archive.

[**Failure / Robustness Validation**]

- **V-F-1:** `task_commit` with a pre-commit hook that always rejects: returns `Error::GitCommitFailed`; phase remains `Verify`; journal has `<HEAD-PENDING>` token. Re-invoke → `JournalEntryStale`.
- **V-F-2:** `task_commit` with a hand-broken patch step (simulated FS error on second journal write): commit succeeds; phase transitions to `Committed`; returns `Error::JournalPatchFailed`; the journal contains a literal `<HEAD-PENDING>` token in one entry.
- **V-F-3:** `ark_archive` with one slug whose `task.toml` is corrupt: the bad slug is reported in `failures`; remaining slugs proceed to archive; exit code is non-zero.
- **V-F-4:** `task_commit --no-commit` followed by manual user commit + manual `/ark:record`: produces a journal entry without `commit_range` (manual entries have no defined range), and the `phase = Committed` task is still picked up by `ark_archive`.
- **V-F-5:** `ark upgrade` migration on a corrupt `VERIFY.md` (heading missing or duplicated): falls back to fresh-seed regeneration; emits one stderr warning.
- **V-F-6:** `task_commit` on a task whose worktree has been deleted: returns `Error::Io` from `git status` invocation; phase remains `Verify`.

[**Edge Case Validation**]

- **V-E-1:** Two tasks committed in the same month bulk-archive into the same `archive/YYYY-MM/` directory under their own slugs.
- **V-E-2:** A task whose `start_head` equals current HEAD (no commits made — should have been blocked by V-UT-11 earlier, but if a user pushed empty changes through somehow): the journal's `commit_range` is `<sha>..<sha>` (empty range); `git log <sha>..<sha>` returns no commits; commits table is empty.
- **V-E-3:** A standard tier `task_commit` invoked without ever running `task_verify` (skipped a phase): VERIFY.md is absent → parse_verify_md returns 0 pending (the file is empty/missing means no checklist). However, `task_commit` checks `phase = Verify` precondition first, so this scenario errors with `IllegalPhaseTransition` before reaching the parser. Belt-and-suspenders: `parse_verify_md` returns Ok(empty pending) on missing file.
- **V-E-4:** A deep-tier task whose latest `NN_PLAN.md` has zero `## Spec` Goals (shouldn't pass REVIEW, but defensively): `render_seeded_verify` emits an empty Plan Fidelity section. Not an error.
- **V-E-5:** A task with `branch = None` (no worktree): `task_commit`'s `task_cwd` defaults to `layout.root()`; commit lands on the parent checkout's current branch.
- **V-E-6:** Concurrent `task_commit` from two shells on different tasks in the same checkout: no contention because they target different journal files? — actually they target the same `.ark/workspace/<dev>/journal-N.md`. The journal write uses `append_text` (atomic for sub-PIPE_BUF writes), and the index re-render is serialized by the existing rerender lock. Add V-IT test for concurrent two-task commit: both entries land, no corruption.
- **V-E-7:** `--no-commit` on quick tier: skips commit + journal, transitions to `Committed`. Stderr emits the note. No SPEC extraction (quick doesn't promote).

[**Acceptance Mapping**]

| Goal / Constraint | Validation                                                       |
| ----------------- | ---------------------------------------------------------------- |
| G-1               | V-UT-1, V-UT-2, V-UT-3, V-IT-10                                  |
| G-2               | V-UT-4, V-UT-5, V-UT-6, V-UT-7                                   |
| G-3               | V-UT-8 .. V-UT-15, V-UT-18, V-IT-1, V-IT-2, V-IT-3, V-F-1, V-F-2 |
| G-4               | V-UT-22, V-UT-23, V-UT-24, V-IT-9                                |
| G-5               | V-UT-14, V-UT-15, V-UT-16, V-UT-17, V-IT-1, V-F-2                |
| G-6               | V-UT-25, V-UT-26, V-UT-27, V-IT-4, V-IT-5, V-F-3                 |
| G-7               | V-IT-6                                                           |
| G-8               | V-IT-6 (parity test) + manual review of `workflow.md` diffs      |
| G-9               | V-IT-6                                                           |
| G-10              | V-IT-7, V-IT-8, V-F-5                                            |
| C-1               | code review + cargo doc graph (no reverse imports)               |
| C-2               | extends existing source-scan test                                |
| C-3               | V-UT-13                                                          |
| C-4               | V-UT-18, V-F-1, V-F-2                                            |
| C-5               | V-UT-4, V-UT-5                                                   |
| C-6               | V-IT-10                                                          |
| C-7               | V-UT-19, V-UT-20, V-UT-21                                        |
| C-8               | V-IT-6                                                           |
| C-9               | V-UT-22, V-UT-23, V-UT-24                                        |
| C-10              | code review                                                      |
| C-11              | manual `ark --help` smoke + `ark agent --help` smoke             |
| C-12              | V-IT-7 (re-run on already-migrated file)                         |
| C-13              | V-F-4                                                            |
| C-14              | V-UT-12                                                          |
| C-15              | V-IT-1 (CHANGELOG row check on existing SPEC)                    |
| C-16              | V-F-4 (manual record path produces no commit_range)              |
| C-17              | V-IT-9                                                           |
| C-18              | manual `ark agent task archive --help` smoke                     |
| C-19              | V-UT-2, V-UT-3                                                   |
| C-20              | V-IT-6                                                           |
