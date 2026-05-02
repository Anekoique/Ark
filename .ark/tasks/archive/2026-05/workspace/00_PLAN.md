# `workspace` PLAN `00`

> Status: Draft
> Feature: `workspace`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Re-introduce workspace support on top of the `ark-workflow-refactor` primitives (`task.toml.start_head`, `Phase::Committed`, atomic `/ark:commit`). New module `crates/ark-core/src/commands/agent/workspace/` with five files: `mod.rs`, `identity.rs`, `config.rs`, `record.rs`, `developer.rs`. The journal-write primitive (`workspace::record`) renders an entry with a `<PENDING:<slug>>` sentinel for the Closing Commit field, then the bulk-archive primitive (`commands::archive`) patches the sentinel to the real short SHA via a slug-anchored `git log -S` lookup atomic with the archive move commit. Two-level index (top-level Active Developers, per-developer Session History) maintained via the same managed-block pattern as `spec_register`. New CLI verbs: `ark agent workspace record`, `ark agent workspace developer register|touch`. New slash command: `/ark:record`. `ark init` gains `--developer <name>` / `--no-developer` flags + interactive prompt. `task.toml` gains `journal_path: Option<String>` captured at `/ark:commit` time so archive doesn't re-derive. `.ark/config.toml` gains `[workspace]` section with `journal_max_lines` (default 2000).

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: Per-developer journal trees under `.ark/workspace/<dev>/` with sequential `journal-N.md` files, written at `/ark:commit` time as part of the same atomic commit as the work.
- G-2: Closing Commit SHA recorded in the journal entry, resolved deterministically without amending or chore-committing. Mechanism: write `<PENDING:<slug>>` sentinel at commit time; `ark archive` patches the sentinel via `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>`.
- G-3: Top-level `.ark/workspace/index.md` with auto-maintained Active Developers table inside `<!-- ARK:DEVELOPERS:START/END -->` markers.
- G-4: Per-developer `.ark/workspace/<dev>/index.md` with auto-maintained Session History table inside `<!-- ARK:SESSIONS:START/END -->` markers; archive patches the matching row's `Closing Commit` cell in lockstep with the journal patch.
- G-5: Compact, table-first journal entry shape (Trellis-derived): no long sentences, no `Files Created`, no `Testing`, no `Next Steps`, no `Package`. Auto-populated structural fields, agent-filled content fields.
- G-6: Manual `/ark:record` entries use the same shape with `**Slug**: -` (so the slug-anchored pickaxe never matches them) and omit `Base Branch`, `Start Head`, `Closing Commit`, `Git Commits`.
- G-7: `task.toml` gains `journal_path: Option<String>` captured at `/ark:commit` time; `archive` reads it directly to locate the journal file (no re-derivation).
- G-8: Idempotent archive — re-running `ark archive` on a task whose slot is already filled is a no-op (sentinel-presence check). Re-running on a partially-archived state resumes safely.
- G-9: Identity bootstrap consolidated to `ark init --developer <name>` / `--no-developer` + interactive prompt. Identity stored in `.ark/.developer` (gitignored).
- G-10: Configuration in `.ark/config.toml`'s `[workspace]` section: `journal_max_lines` (default 2000), `developer` (optional override).
- G-11: Across all three platforms in lockstep — Claude `/ark:record`, Codex `ark-record` skill, OpenCode `/ark:record`.
- G-12: Failure modes are explicit: pickaxe returns 0 commits → error with `--skip-slot-patch <slug>` escape; pickaxe returns >1 commits → error (defensive); journal file moved/missing → error with recorded path.

- NG-1: Squash-merge / as-merged-SHA recording — deferred to `task-finalize`. Slot records the local closing SHA on `feat/<slug>`.
- NG-2: Multi-developer concurrent-write coordination beyond `O_APPEND`.
- NG-3: Cross-project workspace aggregation (each `.ark/` is its own workspace).
- NG-4: Worktree cleanup post-archive — deferred to `task-finalize`.
- NG-5: UI / web rendering of journals.
- NG-6: Backfill of pre-workspace task entries — clean slate, pre-workspace tasks archive normally with slot-patch skipped (G-7's None branch).

[**Architecture**]

```
crates/ark-core/src/commands/agent/workspace/
├── mod.rs              # public surface (re-exports)
├── identity.rs         # `.ark/.developer` read/write/prompt; Identity newtype
├── config.rs           # `[workspace]` section in `.ark/config.toml`
├── developer.rs        # register/touch developer in top-level index
└── record.rs           # journal append + personal-index upsert
                          (consumes identity + config + developer registrar)

crates/ark-core/src/commands/
├── archive.rs          # *modified* — adds slot-patch step before move
├── init.rs             # *modified* — wires --developer / --no-developer / prompt
└── agent/task/
    ├── new.rs          # *unchanged* (start_head already captured)
    └── commit.rs       # *modified* — calls workspace::record after work commit
                          and writes task.toml.journal_path

crates/ark-core/src/commands/agent/state.rs
└── *modified* — TaskToml gains `journal_path: Option<String>`

templates/ark/
├── config.toml         # *modified* — adds [workspace] section
├── .gitignore          # *modified* — adds `.developer`
└── workspace/
    └── index.md        # *new* — top-level template

templates/{claude,codex,opencode}/
└── commands/skills/ark-record  # *new* — slash command / skill bodies
```

Atomic-commit boundary at `/ark:commit`:

```
ark agent task commit
  ├── deep tier: spec_extract (existing)
  ├── workspace::record(--task <slug>)              # NEW — appends journal,
  │     ├── identity::resolve()                     #     upserts personal index,
  │     ├── developer::touch(<dev>)                 #     refreshes Active Devs.
  │     ├── journal::append(<dev>, entry)           #
  │     └── personal_index::upsert(<row>)           #
  ├── set task.toml.journal_path = <path>           # NEW
  ├── set task.toml.phase = Committed
  └── git commit (covers work + task.toml + (deep) SPEC + features INDEX
                  + workspace files)
```

Atomic-commit boundary at `ark archive`, per-task:

```
ark archive
  for each task with phase = committed:
    ├── read task.toml.journal_path                 # G-7
    ├── if journal_path is None: skip slot-patch    # pre-workspace tasks
    ├── git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>
    │     ├── 0 results → error (G-12)
    │     ├── >1 results → error (G-12, defensive)
    │     └── 1 result → continue
    ├── if sentinel <PENDING:<slug>> absent in journal: skip (G-8 idempotent)
    ├── patch journal: <PENDING:<slug>> → <closing-sha-short>
    ├── patch personal index row: <PENDING:<slug>> → <closing-sha-short>
    ├── move task dir to archive/YYYY-MM/<slug>/
  git commit (single commit, covers all journal patches + index patches + moves)
```

[**Data Structure**]

```rust
// crates/ark-core/src/commands/agent/workspace/identity.rs
pub struct Identity {
    pub name: String,
}

pub struct ResolveOptions<'a> {
    pub project_root: &'a Path,
    pub override_name: Option<&'a str>, // from --developer or [workspace].developer
}

// crates/ark-core/src/commands/agent/workspace/config.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WorkspaceConfig {
    pub journal_max_lines: usize,        // default: 2000
    pub developer: Option<String>,       // optional override
}

impl WorkspaceConfig {
    pub fn load_or_default(project_root: &Path) -> Result<Self>;
}

// crates/ark-core/src/commands/agent/workspace/record.rs
pub enum RecordMode<'a> {
    Task { slug: &'a str },
    Manual { title: &'a str },
}

pub struct RecordOptions<'a> {
    pub project_root: &'a Path,
    pub mode: RecordMode<'a>,
    pub identity: Option<&'a Identity>,  // resolved by caller; None → resolve internally
    pub summary: Option<&'a str>,        // agent-filled
    pub main_changes: &'a [(String, String)], // (area, description) pairs
}

pub struct RecordSummary {
    pub journal_path: PathBuf,
    pub session_number: u32,
    pub rotated: bool,
}

// crates/ark-core/src/commands/agent/workspace/developer.rs
pub struct DeveloperRegisterOptions<'a> {
    pub project_root: &'a Path,
    pub name: &'a str,
    pub active_journal: &'a Path,        // relative to .ark/workspace/<name>/
}
pub struct DeveloperTouchOptions<'a> {
    pub project_root: &'a Path,
    pub name: &'a str,
    pub session_count: u32,
    pub active_journal: &'a Path,
}

// crates/ark-core/src/commands/agent/state.rs (modified)
pub struct TaskToml {
    // ... existing fields ...
    pub start_head: Option<String>,      // existing
    pub journal_path: Option<String>,    // NEW — relative to project root
}
```

[**API Surface**]

```rust
// public re-exports from workspace::mod.rs
pub use config::WorkspaceConfig;
pub use developer::{
    DeveloperRegisterOptions, DeveloperTouchOptions, developer_register, developer_touch,
};
pub use identity::{Identity, ResolveOptions, identity_resolve, identity_write};
pub use record::{RecordMode, RecordOptions, RecordSummary, workspace_record};

// archive.rs (modified) — internal helpers
fn resolve_closing_sha(
    project_root: &Path,
    journal_path: &Path,
    slug: &str,
) -> Result<String>; // returns short SHA or errors per G-12

fn patch_slot(
    journal_path: &Path,
    personal_index_path: &Path,
    slug: &str,
    short_sha: &str,
) -> Result<bool>; // bool: whether a patch happened (false → already filled)
```

CLI surface additions (in `crates/ark-cli/src/agent_cli.rs`):

```
ark agent workspace record --task <slug>           # task-driven
ark agent workspace record --manual --title <t>    # manual
ark agent workspace developer register --name <n>  # internal
ark agent workspace developer touch --name <n>     # internal
ark init --developer <n> | --no-developer          # bootstrap
```

[**Constraints**]

- C-1: Journal append uses `PathExt::append_text` (existing primitive); `O_APPEND` semantics are documented as the atomicity guarantee.
- C-2: Sentinel format is exactly `<PENDING:<slug>>` — slug embedded so multi-task journals don't false-match each other during archive.
- C-3: `**Slug**: <slug>` line in the journal entry is the pickaxe anchor. Each task records exactly once; manual entries use `**Slug**: -`.
- C-4: Archive's pickaxe lookup runs *before* archive's own commit so the closing commit is reachable in `git log` history.
- C-5: Short SHA format is 12 chars (`git rev-parse --short=12`), link-friendly and consistent across the journal.
- C-6: No parent-resolution. Journal lives on the task's branch in the worktree's tree; the entry rides with the task commit. (Lesson from PR #9.)
- C-7: Managed-block markers follow the existing `spec_register` convention exactly: `<!-- ARK:DEVELOPERS:START -->` / `<!-- ARK:DEVELOPERS:END -->` and `<!-- ARK:SESSIONS:START -->` / `<!-- ARK:SESSIONS:END -->`. Hand-edits outside markers preserved.
- C-8: `ark upgrade` scaffolds top-level `.ark/workspace/index.md` if absent and adds the `[workspace]` config section if missing; never overwrites existing developer journals or personal indices.
- C-9: `task.toml.journal_path` is project-relative POSIX-style; archive resolves to absolute by joining with project root.
- C-10: `--no-commit` mode of `/ark:commit` does NOT call `workspace::record` (user takes full responsibility). `task.toml.journal_path` stays `None`; archive's slot-patch is skipped per G-7.
- C-11: Workspace files must be staged before `/ark:commit`'s git commit step — `commit.rs` adds the workspace paths to its existing staging set.
- C-12: Identity prompt re-prompts on blank input + missing `USER`/`USERNAME` env (PR #9 bug fix preserved).
- C-13: All journal scans (`scan_session_count`, `index::rerender`) sort journals descending and return max-session-number, not count (PR #9 bug fix preserved).
- C-14: `WorkspaceConfig::load_or_default` reads only its own `[workspace]` section via a private `RawConfig { workspace: Option<...> }` (existing `[worktree]` policy).

## Runtime `runtime logic`

[**Main Flow**] — `/ark:commit` → archive end-to-end

1. User runs `/ark:commit -m "<msg>"` in a deep-tier worktree.
2. `ark agent task commit` (existing) runs deep-tier `spec_extract`.
3. `ark agent task commit` calls `workspace::record(RecordMode::Task { slug })`:
   a. `identity::resolve()` reads `.ark/.developer`; errors if absent (or honors `[workspace].developer` override).
   b. `developer::register_if_absent(<dev>, <active_journal>)` upserts the top-level Active Developers row.
   c. `journal::append(<dev>, entry)` writes the rendered entry (with `<PENDING:<slug>>` sentinel) to the active `journal-N.md`. Rotates to `journal-{N+1}.md` if append would exceed `journal_max_lines`.
   d. `personal_index::upsert_session_row(<dev>, row)` adds the new session row inside the SESSIONS markers.
   e. `developer::touch(<dev>, <session_count>, <active_journal>)` refreshes the top-level row.
   f. Returns `RecordSummary { journal_path, session_number, rotated }`.
4. `ark agent task commit` writes `task.toml.journal_path = summary.journal_path`.
5. `ark agent task commit` writes `task.toml.phase = Committed`.
6. `ark agent task commit` runs `git add` on (work + task.toml + (deep) SPEC + features INDEX + workspace files), then `git commit -m "<msg>"`. Single commit.
7. Later, manager runs `ark archive`:
   a. Scans `.ark/tasks/*/task.toml` for `phase = committed`.
   b. For each task: reads `journal_path`; if None → skip slot-patch (move only).
   c. `resolve_closing_sha(project_root, journal_path, slug)` runs `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal_path>`.
   d. Reads journal text; if `<PENDING:<slug>>` absent → already filled, skip patch (G-8); else patch journal + personal index in memory.
   e. Writes patched files; moves task dir to archive.
   f. After all tasks processed, single git commit covers all journal patches + index patches + archive moves.

[**Failure Flow**]

1. `identity::resolve()` finds no `.ark/.developer` and no `[workspace].developer` → error `MissingIdentity` with hint to run `ark init --developer <name>`. `/ark:commit` aborts before any write; no partial state.
2. `journal::append` rotation fails (e.g., disk full mid-rotation) → error propagates; `/ark:commit` aborts before `git add`. Atomicity preserved by deferring `git commit` to step 6.
3. `resolve_closing_sha` returns 0 commits → archive errors `SlotResolveNoMatch { slug, journal_path }` with hint: rerun with `--skip-slot-patch <slug>` to bypass for that task only (move proceeds, slot stays as `<PENDING:<slug>>`).
4. `resolve_closing_sha` returns >1 commits → archive errors `SlotResolveAmbiguous { slug, candidates: [<sha>...] }`; same `--skip-slot-patch` escape.
5. Journal file at `task.toml.journal_path` does not exist → archive errors `JournalMissing { recorded_path }`; user fixes manually (move journal back) or passes `--skip-slot-patch`.
6. Sentinel present in journal but absent in personal index (or vice versa) → archive errors `SlotMismatch { slug }` to surface drift; user fixes manually.

[**State Transitions**]

- `task.toml.phase`: existing transitions unchanged. `Verify → Committed` and `Execute → Committed` now also write `task.toml.journal_path` as a side effect (unless `--no-commit`).
- `<dev>/index.md` Session History: append-only during `record`; one cell (Closing Commit) mutated during archive.
- Top-level `index.md` Active Developers: row upserted on first record; `Last Active` + `Sessions` + `Active Journal` cells refreshed on every `developer::touch`.
- Slot lifecycle: `<PENDING:<slug>>` (written at commit) → `<closing-sha-short>` (patched at archive) → terminal.

## Implementation `split task into phases`

[**Phase 1** — Identity, Config, Module Skeleton (~200 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/{mod,identity,config}.rs`.
2. Port identity logic from PR #9: `Identity` newtype, `.ark/.developer` read/write/prompt, env fallback (`USER` then `USERNAME`), reprompt-on-blank fix.
3. Port config logic from PR #9: `WorkspaceConfig`, `[workspace]` section, `RawConfig { workspace: Option<...> }`. Keys: `journal_max_lines` (default 2000), `developer` (optional).
4. Update `templates/ark/config.toml` to include `[workspace]` block.
5. Update `templates/ark/.gitignore` to include `.developer`.
6. Wire `ark init --developer <n>` / `--no-developer` flags + interactive prompt in `crates/ark-core/src/commands/init.rs` and `crates/ark-cli/src/main.rs`.
7. Add error variants: `Error::MissingIdentity`, `Error::DeveloperWriteFailed`, `Error::WorkspaceConfigInvalid`.
8. Unit tests: identity round-trip, blank-reprompt, env fallback, config load with/without `[workspace]` section.

[**Phase 2** — Developer Registrar + Indices (~250 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/developer.rs`.
2. Port the managed-block upsert pattern from `spec_register.rs` — generalize the marker scanner so it works for `ARK:DEVELOPERS` (top-level) and `ARK:SESSIONS` (personal). Either a private helper module shared between `spec_register` and `developer`, or a copy if generalization complicates the existing code (decide during implementation; T-1).
3. `developer_register`: scaffolds top-level `index.md` from template if missing, upserts the dev row inside markers.
4. `developer_touch`: refreshes the dev row (`Last Active`, `Sessions`, `Active Journal`).
5. Personal-index upsert: scaffolds `<dev>/index.md` from template if missing, appends a row inside `ARK:SESSIONS` markers.
6. Add `templates/ark/workspace/index.md` (top-level template with static prose + empty markers).
7. Add `templates/ark/workspace/personal-index.md` (per-dev template).
8. Wire `ark agent workspace developer register|touch` CLI verbs.
9. Unit tests: register-fresh, register-idempotent, touch-updates-cells, marker-corruption-rejected, hand-edits-outside-markers-preserved.

[**Phase 3** — Record Primitive + Slot Sentinel (~300 LOC)]

1. Create `crates/ark-core/src/commands/agent/workspace/record.rs`.
2. Implement `workspace_record`:
   a. Resolve identity + config.
   b. Discover active journal (`scan_session_count` returns max-N; new entry goes in `journal-N.md` unless rotation needed).
   c. Render entry using format strings keyed off `RecordMode`. Task mode includes `**Slug**: <slug>`, `**Branch**: <branch>`, `**Base Branch**: <base_branch>`, `**Start Head**: <start_head_short>`, `**Closing Commit**: <PENDING:<slug>>`, and a `Git Commits` table from `git log <start_head>..HEAD --oneline`. Manual mode includes `**Slug**: -`, `**Branch**: <current-branch>`, no Closing Commit / Start Head / Base Branch / Git Commits.
   d. Append via `PathExt::append_text` (rotates first if would exceed config).
   e. Call `personal_index::upsert_session_row` with the new row.
   f. Call `developer_touch` to refresh top-level.
3. Wire `ark agent workspace record --task <slug>` and `--manual --title <t>` CLI verbs.
4. Edge: when `start_head` is None (pre-refactor task), the Git Commits table falls back to `git log -n 20 --oneline` (existing fallback mentioned in `ark-workflow-refactor` SPEC).
5. Unit tests: task-mode renders all fields, manual-mode renders subset, sentinel format is exact, rotation triggers at threshold, descending-sort scan returns correct N.

[**Phase 4** — Wire `record` into `/ark:commit` (~150 LOC)]

1. Modify `crates/ark-core/src/commands/agent/state.rs::TaskToml` — add `pub journal_path: Option<String>`.
2. Modify `crates/ark-core/src/commands/agent/task/commit.rs`:
   a. After `spec_extract` (deep) and before staging, call `workspace::workspace_record(RecordMode::Task { slug })` with summary + main_changes provided by the slash command via the existing CLI param machinery (or, if no agent-supplied content, render with placeholder summary/changes that the agent edits before staging — decision during implementation; T-2).
   b. Set `task.toml.journal_path = summary.journal_path`.
   c. Add the workspace files to the `git add` set (the journal file + personal index + top-level index).
3. Honor `--no-commit`: skip workspace::record entirely; `journal_path` stays None.
4. Update `templates/{claude,codex,opencode}/commands/skills/ark/commit.md` (and `.codex/skills/ark-commit/SKILL.md`) to reflect the new step.
5. Integration test: end-to-end `task new → execute → verify → commit` produces the expected journal entry with sentinel + `task.toml.journal_path` set.

[**Phase 5** — Slot Patch in `ark archive` (~200 LOC)]

1. Modify `crates/ark-core/src/commands/archive.rs`:
   a. Add `resolve_closing_sha(project_root, journal_path, slug) -> Result<String>` running `git log -S '**Slug**: <slug>' --format=%H -n 1 -- <journal-path>`.
   b. Add `patch_slot(journal_path, personal_index_path, slug, short_sha) -> Result<bool>` that reads file, returns false if sentinel absent (already filled), otherwise rewrites both files in memory and returns true.
   c. Modify the per-task archive loop: read `task.toml.journal_path` → if Some, run resolve + patch *before* `mv`; if None, skip.
   d. Add `--skip-slot-patch <slug>` flag (repeatable) to bypass patch for specific tasks (failure-mode escape per G-12).
   e. Single `git commit` after all tasks processed covers patches + moves.
2. Add error variants: `Error::SlotResolveNoMatch`, `Error::SlotResolveAmbiguous`, `Error::JournalMissing`, `Error::SlotMismatch`.
3. Unit tests: resolve-success, resolve-zero-error, resolve-ambiguous-error, patch-idempotent, skip-slot-patch-flag, journal-missing-error.
4. Integration test: end-to-end `commit → archive` → journal sentinel replaced with real short SHA, personal index Closing Commit cell updated to match.

[**Phase 6** — Slash Commands + Migration (~150 LOC + docs)]

1. Add `templates/claude/commands/ark/record.md`, `templates/codex/skills/ark-record/SKILL.md`, `templates/opencode/commands/ark/record.md` — thin wrappers calling `ark agent workspace record --manual --title "<t>"`.
2. Mirror to `.claude/commands/ark/record.md`, `.codex/skills/ark-record/SKILL.md`, `.opencode/commands/ark/record.md`.
3. Update `ark upgrade` (`crates/ark-core/src/commands/upgrade/mod.rs`):
   a. Scaffold top-level `.ark/workspace/index.md` if absent.
   b. Add `[workspace]` config section if missing (non-destructive patch).
   c. Re-render slash-command templates.
4. Update `.ark/workflow.md`, `AGENTS.md`, `README.md`, `docs/book/*` to mention workspace + `/ark:record` again (mirror PR #9's doc edits).
5. Identity dogfood: this task creates `.ark/.developer` for `Anekoique` during EXECUTE; the workspace task records its own commit through the new primitive (the first journal entry).

## Trade-offs `ask reviewer for advice`

- T-1: **Marker-scanner sharing.** Generalize `spec_register`'s managed-block helper into a shared module (`crates/ark-core/src/io/managed_block.rs`?), or copy the pattern into `developer.rs`. *Adv. of share*: single source of truth, easier to fix marker bugs once. *Adv. of copy*: keeps `spec_register` stable during this refactor; no risk of breaking the existing feature-INDEX.md flow. Lean: extract to shared module — the pattern is identical and the refactor is small.
- T-2: **Agent-content delivery to `record` at commit time.** Two options. Option A: slash-command renders a draft entry with placeholder summary/changes, agent edits before staging, `commit` reads the journal file as-is. Option B: slash-command passes `--summary` / `--main-changes` flags through to `record` directly. *Adv. of A*: agent has full Markdown freedom, no flag-escaping headaches. *Adv. of B*: structurally cleaner, no "edit then commit" two-step. Lean: A — agent edits the journal entry as a regular file, then `/ark:commit` stages it. Matches the existing pattern where agent edits PRD/PLAN/VERIFY before phase advance.
- T-3: **Sentinel format.** `<PENDING:<slug>>` vs. `<<closing:<slug>>>` vs. literal space placeholder. *Adv. of `<PENDING:<slug>>`*: human-readable, slug-embedded so multi-task journals can't false-match, easy to grep. *Adv. of literal space*: invisible to readers. *Adv. of `<<closing:<slug>>>`*: distinctive bracket style. Lean: `<PENDING:<slug>>` — readable + unique + greppable.
- T-4: **Archive failure UX.** When pickaxe returns 0 results, archive can either (a) error and require `--skip-slot-patch`, or (b) auto-fall-back to `git log -n 1 -- <journal-path>` (touch-based lookup). *Adv. of (a)*: surfaces real bugs (e.g., journal hand-deleted) early. *Adv. of (b)*: more permissive, fewer manual interventions. Lean: (a) — the `ark-workflow-refactor` REVIEW R-201 already concluded path-only `git log` is unreliable; fail loud.

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `identity_resolve` returns from `.ark/.developer`, falls back to `[workspace].developer`, errors on missing both with `MissingIdentity`.
- V-UT-2: `identity` prompt reprompts on blank + missing env (PR #9 fix preserved).
- V-UT-3: `WorkspaceConfig::load_or_default` reads `journal_max_lines` from `[workspace]` section, returns 2000 default when section absent.
- V-UT-4: `developer_register` upserts a row inside `ARK:DEVELOPERS` markers; idempotent on re-register.
- V-UT-5: `developer_touch` refreshes `Last Active`, `Sessions`, `Active Journal` cells; preserves hand-edits outside markers.
- V-UT-6: `personal_index::upsert_session_row` appends a row with `<PENDING:<slug>>` Closing Commit cell.
- V-UT-7: `workspace_record(Task)` renders all expected fields and the exact sentinel `<PENDING:<slug>>`.
- V-UT-8: `workspace_record(Manual)` renders `**Slug**: -` and omits Closing Commit / Base Branch / Start Head / Git Commits.
- V-UT-9: Journal rotation triggers when append would exceed `journal_max_lines`; new file is `journal-{N+1}.md`.
- V-UT-10: `scan_session_count` sorts journals descending and returns max-N (PR #9 fix preserved).
- V-UT-11: `resolve_closing_sha` happy path returns short SHA.
- V-UT-12: `resolve_closing_sha` returns `SlotResolveNoMatch` for unknown slug.
- V-UT-13: `patch_slot` returns false (skipped) when sentinel absent — idempotency.
- V-UT-14: `patch_slot` returns true and rewrites both journal + index when sentinel present.

[**Integration Tests**]

- V-IT-1: `task new --tier deep --worktree → /ark:commit` produces a journal entry with sentinel and `task.toml.journal_path` populated.
- V-IT-2: `/ark:commit → ark archive` end-to-end: sentinel replaced with real short SHA; personal index Closing Commit cell matches.
- V-IT-3: `ark archive` is idempotent — re-running on already-archived task is a no-op (sentinel-presence check passes through).
- V-IT-4: `ark init --developer alice` followed by `/ark:record` produces a manual entry with `**Slug**: -`, no Closing Commit field.
- V-IT-5: `ark upgrade` on a workspace-less repo scaffolds top-level `.ark/workspace/index.md` and adds `[workspace]` config section.
- V-IT-6: Three-platform parity — `/ark:record` works identically across Claude / Codex / OpenCode templates.

[**Failure / Robustness Validation**]

- V-F-1: `resolve_closing_sha` returns >1 commits → `SlotResolveAmbiguous` error with candidate list.
- V-F-2: Journal file moved between commit and archive → `JournalMissing` error with recorded path.
- V-F-3: `--skip-slot-patch <slug>` bypasses the patch for the named slug; archive proceeds with move only; sentinel left in journal.
- V-F-4: Sentinel present in journal but missing in personal index (or vice versa) → `SlotMismatch` error.
- V-F-5: `--no-commit` mode of `/ark:commit` does not write a journal entry; `journal_path` stays None; archive proceeds without slot-patch.
- V-F-6: `MissingIdentity` aborts `/ark:commit` before any file write; working tree unchanged.

[**Edge Case Validation**]

- V-E-1: Slug containing characters that would break the pickaxe search (e.g., spaces, regex metachars) — slug grammar already restricts to lowercase + hyphen + ASCII (existing constraint from `task new`); no escaping needed but document the constraint.
- V-E-2: Journal at exactly `journal_max_lines` triggers rotation on next append.
- V-E-3: Manual entries interleaved with task entries in the same `journal-N.md` — pickaxe for task slug correctly ignores manual `**Slug**: -` lines.
- V-E-4: Multiple task entries in the same `journal-N.md` (same developer, multiple tasks committed to the same branch) — each task's pickaxe uniquely matches its own slug line.
- V-E-5: Concurrent record from two processes — `O_APPEND` semantics give line-level atomicity for typical entry sizes; document the limitation per PR #9.
- V-E-6: `--developer` flag overrides `.ark/.developer` for a single invocation; `.ark/.developer` is unchanged.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1  | V-IT-1, V-UT-7 |
| G-2  | V-IT-2, V-UT-11, V-UT-13, V-UT-14 |
| G-3  | V-UT-4, V-UT-5 |
| G-4  | V-UT-6, V-IT-2 |
| G-5  | V-UT-7, V-UT-8 |
| G-6  | V-UT-8, V-E-3 |
| G-7  | V-IT-1, V-F-5 |
| G-8  | V-IT-3, V-UT-13 |
| G-9  | V-IT-4, V-UT-1, V-UT-2 |
| G-10 | V-UT-3, V-IT-5 |
| G-11 | V-IT-6 |
| G-12 | V-UT-12, V-F-1, V-F-2, V-F-3, V-F-4 |
| C-1  | V-UT-9 |
| C-2  | V-UT-7, V-E-4 |
| C-3  | V-UT-8, V-E-3 |
| C-4  | V-IT-2 |
| C-5  | V-IT-2 |
| C-6  | V-IT-1 |
| C-7  | V-UT-4, V-UT-5 |
| C-8  | V-IT-5 |
| C-9  | V-IT-1, V-IT-2 |
| C-10 | V-F-5 |
| C-11 | V-IT-1 |
| C-12 | V-UT-2 |
| C-13 | V-UT-10 |
| C-14 | V-UT-3 |
