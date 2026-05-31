# `workspace` PRD

---

[**What**]

Re-introduce workspace support: per-developer journal trees under `.ark/workspace/<dev>/` that record AI-Agent task and manual sessions. Journal entries written at `/ark:commit` time carry a `**Closing Commit**: <PENDING:<slug>>` sentinel; `ark archive` patches the sentinel to the real closing SHA via a slug-anchored `git log -S` lookup, atomic with the archive move commit. Layout and entry shape are ported from Trellis's workspace pattern: top-level `workspace/index.md` + per-developer `<dev>/index.md` + sequential `<dev>/journal-N.md`. Both index files use the `ark agent` registrar pattern with `<!-- ARK:...:START -->` / `<!-- ARK:...:END -->` markers.

[**Why**]

Workspace was shipped (PR #9, 2026-04-29) and removed (commit `73d46ba`, 2026-05-02) because the closing-commit-SHA recording was unsolved: the three options on the table — amend after journal write (stale hash), independent chore commit (trellis-style noise), or omit the SHA (unclear messages) — were all unacceptable. The `ark-workflow-refactor` task (commit `7ed2b8b`, awaiting bulk archive) landed two enabling primitives: `task.toml.start_head` captured at `task new` time, and an atomic `/ark:commit` that bundles work + task.toml + (deep) SPEC + features INDEX into a single commit. Those primitives plus a deferred-slot mechanism that `ark archive` fills make a fourth option viable: the journal entry is written at commit time with everything that's *knowable* before the closing commit exists (start_head, base_branch, slug, commits-in-range table from `git log <start_head>..HEAD`), and the closing SHA is filled in by archive — which already runs in a later commit allowed to know earlier SHAs. No amend, no chore commits, no vague messages.

Beyond fixing the SHA problem, the previous workspace shape had two further gaps the user surfaced when reviewing: per-developer entries were too prose-heavy (long sentences instead of compact tables), and there was no top-level `workspace/index.md` aggregating active developers. The Trellis workspace layout (compact tables, sequential journals, two-level index) is the target shape.

[**Outcome**]

A user running `ark init --developer alice` on a fresh install, then completing a deep-tier task and running `ark archive`, observes the following end-to-end:

1. **Layout.** `.ark/workspace/index.md` (top-level), `.ark/workspace/<dev>/index.md` (personal), `.ark/workspace/<dev>/journal-N.md` (sequential, starts at `journal-1.md`). `.ark/.developer` (gitignored) carries identity. No `tasks/*.json` subdirectory under workspace — Ark's project-wide `.ark/tasks/` is untouched.

2. **Top-level `workspace/index.md`** is auto-maintained. Static preface (purpose, layout diagram, getting-started prose) + an Active Developers table managed between `<!-- ARK:DEVELOPERS:START -->` / `<!-- ARK:DEVELOPERS:END -->` markers. Columns: `Developer | Last Active | Sessions | Active Journal`. Rows are upserted by `ark agent workspace developer register --name <dev>` (called automatically on first journal write for that developer) and refreshed on every journal append by `ark agent workspace developer touch --name <dev>`. Hand-edits outside the markers are preserved across `ark upgrade`.

3. **Per-developer `<dev>/index.md`** is auto-maintained. Static preface + a Session History table managed between `<!-- ARK:SESSIONS:START -->` / `<!-- ARK:SESSIONS:END -->` markers. Columns: `# | Date | Title | Slug | Branch | Closing Commit | Journal`. Each appended entry adds a row; archive's slot-patch step also rewrites the matching row's `Closing Commit` cell from `<PENDING:<slug>>` to the real short SHA. The `# | Slug` columns let archive locate the row deterministically.

4. **Journal entry shape (task-driven, written by `/ark:commit`)** — compact, table-first, no long sentences:

   ```
   ## Session N: <title>

   **Date**: YYYY-MM-DD
   **Slug**: <slug>
   **Branch**: `<branch>`
   **Base Branch**: `<base_branch>`
   **Start Head**: `<start_head_short>`
   **Closing Commit**: <PENDING:<slug>>

   ### Summary

   <one-line summary, agent-filled>

   ### Main Changes

   | Area | Description |
   |------|-------------|
   | <area> | <description> |

   ### Git Commits

   | Hash | Message |
   |------|---------|
   | `<short>` | <subject> |

   ### Status

   **Committed** — awaiting archive.
   ```

   Auto-populated by `ark agent workspace record`: `Date`, `Slug`, `Branch`, `Base Branch`, `Start Head`, `Closing Commit` sentinel, `Git Commits` table from `git log <start_head>..HEAD --oneline`, `Status` literal. Agent-filled before invoking the recorder: `Title`, `Summary`, `Main Changes` table rows. Removed from the previous workspace shape: prose sub-bullets, `Files Created` (redundant with diff), `Testing` (covered by VERIFY.md), `Next Steps` (covered by follow-up tasks), `Package` (Trellis-monorepo concept).

5. **Journal entry shape (manual, written by `/ark:record`)** — same shape, agent-filled fields only:

   ```
   ## Session N: <title>

   **Date**: YYYY-MM-DD
   **Slug**: -
   **Branch**: `<current-branch>`

   ### Summary

   <agent-filled>

   ### Main Changes

   | Area | Description |
   |------|-------------|
   | <area> | <description> |

   ### Status

   **Recorded** — manual entry.
   ```

   Manual entries omit `Base Branch`, `Start Head`, `Closing Commit`, and `Git Commits` (no task range to render). `**Slug**: -` ensures the slug-anchored pickaxe used by archive never matches a manual entry. Agent fills `Title`, `Summary`, `Main Changes` before invoking the recorder; recorder auto-populates `Date`, `Branch`, `Status` literal.

6. **`ark agent workspace record` is the single journal-append primitive.** Both modes consume an `--entry-file <path>` payload (the agent-filled draft Markdown): `--task <slug> --entry-file <path>` (task mode — reads `task.toml` for `slug`/`branch`/`base_branch`/`start_head`, writes `<PENDING:<slug>>` sentinel) and `--manual --entry-file <path>` (manual mode — no sentinel, no `Git Commits` table, `**Slug**: -`). Both modes append to the developer's active `journal-N.md`, rotate to `journal-{N+1}.md` if the file would exceed `journal_max_lines`, and refresh the personal index's Session History table + the top-level Active Developers table in the same write batch. The slash commands (`/ark:commit`, `/ark:record`) render the draft entry to a temp file, let the agent edit the empty `Summary` and `Main Changes` sections, then invoke the CLI with the edited file. `/ark:commit` is the user-facing closure step; `ark agent task commit --entry-file <path>` consumes the draft and folds the journal write into the same atomic git commit as the task work.

7. **`ark archive` patches the sentinel.** For each task it's about to move from `phase = committed` to `.ark/tasks/archive/YYYY-MM/<slug>/`:

   a. Read `task.toml` to get `slug` and the journal path the entry was written to (`task.toml.journal_path`, captured at commit time so archive doesn't have to re-derive the developer or journal-N).
   b. Resolve the closing SHA via **collect-then-classify** pickaxe: `git log -S '**Slug**: <slug>' --format=%H -- <journal-path>` (no `-n` cap), collect all matching full SHAs, classify by count — error on 0 (`SlotResolveNoMatch`) or >1 (`SlotResolveAmbiguous { candidates }`), derive 12-character short SHA (`git rev-parse --short=12 <sha>`) only on unambiguous success. Slug uniqueness per journal is structural (each task records exactly once; manual entries use `**Slug**: -`); pickaxe matches net-count-change so amend/revert sequences that re-add the same string don't false-match. The collect-then-classify form is required because `-n 1` would silently mask the ambiguous case.
   c. Idempotency check: read the journal, search for `<PENDING:<slug>>`. If absent → slot already filled, skip patch step (re-archive of an already-archived task is a no-op). If present → continue.
   d. Patch the journal in place: `<PENDING:<slug>>` → `<closing-sha-short>` (12-char short SHA, link-friendly format). Patch the personal `<dev>/index.md` Session History row's `Closing Commit` cell in the same write.
   e. Move the task dir to archive.
   f. Single git commit covers the journal patch + index patch + archive move. Commit message: `chore(archive): bulk-archive <N> task(s)` with a body listing slugs and resolved SHAs.

   Failure modes:
   - **Pickaxe returns no commits** (e.g., journal write was hand-deleted between commit and archive) → archive errors with a clear message naming the slug + journal path; user fixes manually or passes `--skip-slot-patch <slug>` to bypass for that task only.
   - **Pickaxe returns multiple commits** (cannot happen given slug uniqueness, but defensive) → archive errors and refuses; same `--skip-slot-patch` escape.
   - **Journal file no longer exists at the path `task.toml.journal_path` recorded** (e.g., user moved it) → archive errors with the recorded path; user fixes manually.

8. **`task.toml` gains `journal_path`** captured at `/ark:commit` time so archive doesn't have to re-resolve the developer or active journal-N. Type: `Option<String>` (None for tasks that committed with `--no-commit` or pre-workspace tasks). Archive's slot-patch step is skipped when None — degrades gracefully.

9. **Squash-merge SHA is out of scope.** The slot records the *local closing SHA* (the commit `/ark:commit` produced on `feat/<slug>`). If the merge to main is a squash, the SHA on main differs. That's the `task-finalize` problem (queued separately) — `task-finalize` can later overwrite the slot with the merged SHA via the same patch primitive. Workspace records what it can know at archive time.

10. **Identity bootstrap is consolidated.** `ark init --developer <name>` / `--no-developer` flags + interactive prompt (the surface from PR #9). Drop the standalone `ark agent workspace init` — `ark init` covers it, and `ark agent workspace developer register` is the lower-level primitive for tooling. Identity stored in `.ark/.developer` (gitignored), single line containing the name.

11. **Configuration in `.ark/config.toml`'s `[workspace]` section.** Keys: `journal_max_lines` (default `2000`), `developer` (optional override, normally read from `.ark/.developer`). Section is preserved across `ark upgrade` (existing `[worktree]` policy). No `workspace.toml`.

12. **Migration on `ark upgrade`** — clean slate: workspace was just removed, no in-flight workspace state to preserve. Upgrade scaffolds `.ark/workspace/index.md` if absent, scaffolds the developer dir if `.ark/.developer` exists, but does not synthesize entries for tasks committed before workspace was re-introduced. Pre-workspace tasks archive normally with the slot-patch step skipped (item 8).

13. **Across all three platforms in lockstep.** Claude `/ark:record`, Codex `ark-record` skill, OpenCode `/ark:record` — all re-added. Each slash command renders a draft entry to `.ark/.commit-draft.md` (or a per-platform equivalent), pauses for the agent to fill the empty `Summary` and `Main Changes` sections, then invokes `ark agent workspace record --manual --entry-file <path>` (or `--task <slug> --entry-file <path>` for the `/ark:commit` path). The atomic `/ark:commit` flow becomes: render draft → agent edits → `ark agent task commit --entry-file <path>` runs the full closure (workspace journal write, deep-tier SPEC extract, single git commit covering work + task.toml + (deep) SPEC + features INDEX + workspace files).

14. **Dogfooding.** This task creates `.ark/.developer` and `.ark/workspace/Anekoique/` during its own EXECUTE phase, and the journal entry for *this very task* is the first entry written by the new `record` primitive. The slot-patch is exercised by the eventual `ark archive` of this task.

Out of scope:
- Squash-merge / as-merged-SHA recording (deferred to `task-finalize`).
- Multi-developer concurrent-write coordination beyond what filesystem `O_APPEND` already provides (existing `PathExt::append_text` is sufficient).
- Cross-project workspace aggregation (each `.ark/` is its own workspace).
- `--worktree` cleanup post-archive (deferred to `task-finalize`).
- Any UI / web rendering of journals.

[**Related Specs**]

- `.ark/specs/features/ark-workflow-refactor/SPEC.md` — `task.toml.start_head`, `Phase::Committed`, atomic `/ark:commit`, the `**Start Head**` / `**Base Branch**` journal fields. This task consumes those primitives and adds the `**Closing Commit**` slot + slug-anchored `git log -S` recovery + archive-time patch. Adds `task.toml.journal_path` captured at commit time.
- `.ark/specs/features/ark-agent-namespace/SPEC.md` — `ark agent` verb set. This task adds `ark agent workspace record` (with `--task <slug> --entry-file <path>` / `--manual --entry-file <path>` modes), `ark agent workspace developer register|touch`, and the `--entry-file` flag on `ark agent task commit`. The hidden, non-semver-stable nature is preserved.
- `.ark/specs/features/ark-context/SPEC.md` — `ark context` projections. This task adds a new `--scope record` projection (additive on the existing `Scope` enum — `--for record` was rejected because `--for` is reserved for the `Phase` scope) that bundles developer identity + active journal path + workspace config + branch as a `RecordProjection` payload inside the existing `ProjectedContext` envelope. The slash commands consume the JSON to seed agent-filled fields.
- `.ark/specs/features/ark-upgrade/SPEC.md` — template re-render and migration policy. This task adds `.ark/workspace/index.md` scaffolding to the upgrade flow and adds the `[workspace]` section to the `.ark/config.toml` template. Existing non-destructive patching policy is preserved.
- `.ark/specs/features/codex-support/SPEC.md` and `.ark/specs/features/opencode-support/SPEC.md` — platform-parity SPECs. This task adds `/ark:record` (or platform equivalent) across all three platforms in lockstep.
- `.ark/specs/features/worktree/SPEC.md` — worktree mechanics. This task does not change worktree creation, but `record --task <slug> --entry-file <path>` runs from the worktree's `.ark/` and writes to `.ark/workspace/<dev>/` *within the worktree's tree* (so the journal append is part of the same commit as the work, not the parent checkout's tree). The journal file lives on the task's branch, not on `main` — same lesson as PR #9 (no parent-resolution).
- `.ark/specs/features/project-spec/SPEC.md` — project-spec layout. This task does not modify the layout but consumes `.ark/specs/project/INDEX.md` at VERIFY-seed time per the standard rule.
- `.ark/specs/features/task-concurrency-control/SPEC.md` — `.ark/.state.toml` and `Phase`. This task does not change phases or transitions; it only writes journals during the existing `Verify → Committed` and `Execute → Committed` transitions and reads `phase = committed` during `Committed → Archived`.
