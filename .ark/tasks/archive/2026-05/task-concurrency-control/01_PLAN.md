# `task-concurrency-control` PLAN `01`

> Status: Draft
> Feature: `task-concurrency-control`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: none

---

## Summary

Iteration 01 keeps the same overall architecture but resolves the four blockers from REVIEW 00:

- **Session-id provider becomes a real cross-platform shim.** New module `session::ppid` with `cfg(unix)` and `cfg(windows)` implementations, fronted by a single `parent_id() -> u32` function. Adds `windows-sys = { version = "0.59", features = ["Win32_System_Threading", "Win32_System_Diagnostics_ToolHelp", "Win32_Foundation"] }` to ark-core's `[target.'cfg(windows)'.dependencies]`. Unix path uses `std::os::unix::process::parent_id`. Constraint C-1 ("no external crate") is dropped; replaced with a narrower constraint that pinpoints exactly which deps are required and why.

- **Reconcile becomes two-way.** `state_file::reconcile::reconcile_against_disk` now enumerates `.ark/tasks/<slug>/task.toml` (excluding `archive/`), adds any non-archived slug not present in `state.tasks.active`, then drops archived/missing entries and orphan session focuses. State file becomes a true index — losing it is recoverable from `task.toml`s.

- **`.state.toml` gains an explicit unload/snapshot policy.** It is treated identically to how `.developer` was: skipped in both `unload.rs` walk sites along with `.state.toml.lock` and any `.state.toml.tmp.*` orphans. Identity stays per-machine. Active-task state is intentionally ephemeral in snapshots — `load` re-derives it from the captured `task.toml`s via two-way reconcile on first read. Migration retires `.developer` from the skip set after migration; both files are skipped during the migration window.

- **Worktree inventory ops switch to `state.tasks.active` (not session focus).** `discovery.rs` and `list.rs` read each worktree's `state.tasks.active` and load each corresponding `task.toml` to enumerate active worktree-backed tasks. Session focus is not consulted for inventory. Legacy `.current` remains a migration fallback inside the per-worktree state load (already covered by `migrate.rs`).

The state-mutate-first archive ordering (T-3) becomes safe under R-002's two-way reconcile: if rename fails after the active entry is dropped, the next `load_state` either re-adds the slug (if dir still has non-Archived `task.toml`) or correctly drops it (if `task.toml.phase == Archived` was already saved before the failed rename), so the user can re-run archive or discard. TR-3's required action is satisfied as a downstream consequence of the R-002 fix.

## Log

[**Added**]

- New `session::ppid` module with `cfg(unix)` / `cfg(windows)` parent-id shim.
- New ark-core dep: `windows-sys = "0.59"` under `[target.'cfg(windows)'.dependencies]`.
- New `reconcile.rs::reconcile_against_disk` two-way pass.
- New constraints C-18 (`unload.rs` skip set), C-19 (two-way reconcile order), C-20 (Windows parent_id failure fallback).
- New goals G-12 (state-file unload/snapshot policy), G-13 (cross-platform parent_id provider).
- New trade-off T-8 (Windows `parent_id` failure: abort vs. degrade).
- New tests V-UT-12, V-UT-24..V-UT-29, V-IT-7, V-IT-8, V-F-5, V-E-6, V-E-7 covering parent-id portability, two-way reconcile, unload exclusion, worktree inventory after session-cache loss.

[**Changed**]

- C-1: was "stdlib `File::try_lock` is the *only* lock primitive; no external crate." Now: "stdlib `File::try_lock` for locking; `windows-sys` for parent-id under `cfg(windows)`. No other deps."
- T-1: trade-off rewritten — choice between `sysinfo` (heavy, multi-purpose) and bespoke `windows-sys` shim (≈30 LOC, single-purpose). Choice: bespoke shim.
- Architecture diagram: adds `session::ppid`, `commands/unload.rs` row.
- Phase 1 implementation order: implement parent-id shim first; everything else stays.
- Phase 2 implementation: worktree discovery/list rewrite uses `state.tasks.active` enumeration, not session-focus scan; `unload.rs` skip set extended.
- Validation: V-UT-15..V-UT-17 (GC tests) extended with cross-platform notes; V-UT-8..V-UT-10 (reconcile-drops) joined by V-UT-24..V-UT-26 (reconcile-adds).
- Acceptance Mapping table updated for G-12, G-13, and the new V-IDs.

[**Removed**]

- The constraint "no external crate." Replaced with the narrow C-1 above.
- Mention of pinning `rust-version = "1.89"` solely to avoid an external lock crate.

[**Unresolved**]

- None. All four R-001..R-004 findings have concrete fixes inside this iteration.

[**Response Matrix**]

| Source | ID    | Decision | Resolution |
|--------|-------|----------|------------|
| Review | R-001 | Accepted | Replaced `std::process::parent_id()` with a `cfg`-gated `session::ppid::parent_id()` shim. Unix uses `std::os::unix::process::parent_id`; Windows uses `windows-sys`'s `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW` to look up the calling process's parent. Added `windows-sys` dep under `cfg(windows)`. New tests V-UT-12 (cross-platform smoke) and V-IT-1 (multi-session) explicitly run on the CI Windows job. C-20 specifies graceful degradation on Windows toolhelp failure. |
| Review | R-002 | Accepted | `reconcile.rs` now does add-then-drop in a single pass (C-19): enumerate `.ark/tasks/<slug>/task.toml` excluding `archive/`, push any non-archived slug missing from `tasks.active`, then drop archived/missing actives, then drop orphan session focuses. New tests V-UT-24, V-UT-25, V-UT-26 cover "state missing but task dir exists", "state active omits existing task dir", and the archive-rename-failure recovery path. |
| Review | R-003 | Accepted | Added G-12 + C-18 specifying that `unload.rs`'s file-level skip set extends to `[layout.state_file(), layout.state_lock_file(), layout.developer_file()]` plus a `.state.toml.tmp.*` orphan glob in both walk sites. During the migration window both `.developer` and `.state.toml` are skipped. Active task state is recoverable on `load` via two-way reconcile (R-002) — capture is unnecessary and identity-leak risk is removed. New tests V-IT-7, V-UT-28, V-UT-29. |
| Review | R-004 | Accepted | `discovery.rs` and `list.rs` enumerate `state.tasks.active` for each worktree (per-worktree `Layout`, then `load_state`), and load each corresponding `task.toml`. Session focus is not consulted. Legacy `.current` fallback is automatic via `migrate::synthesize_from_legacy` which `load_state` always runs. New test V-IT-8 covers "list works after deleting all session cache files." |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec

[**Goals**]

- G-1: A single per-checkout `.ark/.state.toml` file carries identity, the active-task set, and a per-session focus map. Truth still lives in `.ark/tasks/<slug>/task.toml`; the state file is an index reconciled (two-way: add + drop) on every read.
- G-2: Concurrent CLI invocations from independent shells in the same `.ark/` each get an independent `[sessions.<uuid>]` entry and an independent focused task. No session can clobber another's focus.
- G-3: Two new agent ops — `ark agent task resume <slug>` and `ark agent task discard <slug>` — extend the agent task verb set (extends `ark-agent-namespace` SPEC G-3).
- G-4: The identity API (`read_developer_name`, `require_developer_name`, `write_developer_file` in `commands/agent/workspace/identity.rs`) preserves its public signatures; bodies delegate to the new state-file API. The 4 existing call sites are unchanged.
- G-5: Legacy `.ark/.developer` and `.ark/tasks/.current` are auto-migrated on the first state-file mutation and then deleted. The reader tolerates either layout indefinitely so a pure read in a not-yet-migrated install does not change the on-disk state.
- G-6: State-file mutations are atomic across crash *and* across concurrent CLI invocations on Linux, macOS, and Windows. Crash mid-write leaves no partial `.state.toml`. Concurrent writers serialize via an OS-level file lock with bounded backoff.
- G-7: Dead sessions are GC'd transparently on the next state read (in-memory; persisted on the next mutation). A session is "dead" when its temp-dir cache file is missing or mismatched.
- G-8: `task new` warns (does not refuse) when `[tasks].active` is non-empty before the new task is appended. Replaces the silent `.current` clobber that orphaned in-flight tasks.
- G-9: Each git worktree's `.ark/` owns its own `.state.toml`. `task new --worktree` writes only to the worktree's state file; the parent's state file is untouched. (Mirrors `worktree` SPEC G-9.) `task worktree list` and `task worktree cleanup` enumerate via each worktree's `state.tasks.active`, **not** via session focus, so inventory works from any shell.
- G-10: `--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask` — outside-Ark's-contract usage is rejected, not papered over.
- G-11: `task discard <slug>` refuses without `--force` when seeded files (PRD/PLAN/etc.) differ from their templates ("PRD has user content" guard). Always refuses if the task is already archived (use the archive flow's removal path instead).
- G-12: `.ark/.state.toml`, `.ark/.state.toml.lock`, and any `.ark/.state.toml.tmp.*` orphan files are skipped by `unload.rs` in both walk sites (Stage A snapshot capture + Stage B file enumeration). Identity stays per-machine and is never captured into `.ark.db`. Active task state is recoverable on `load` via two-way reconcile (G-1, C-19) — capture is unnecessary. The legacy `.developer` skip stays in place during the migration window.
- G-13: Cross-platform parent-id via a `cfg`-gated `session::ppid::parent_id()` shim. Unix delegates to `std::os::unix::process::parent_id`; Windows uses `windows-sys`'s `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW` to find the parent of `GetCurrentProcessId()`. Both code paths return `u32`. Failure on Windows (e.g. snapshot creation fails) returns the calling process's own PID as a fallback so session machinery degrades gracefully rather than aborting the CLI invocation.

- NG-1: Workflow-phase model changes (drop VERIFY, postpone ARCHIVE) — ROADMAP item #2.
- NG-2: SessionStart hook integration — sessions register only on `task new` / `task resume`.
- NG-3: Explicit `session end` / `session list` ops — GC is the only reaper.
- NG-4: Cross-host coordination (NFS-shared `.ark/`).
- NG-5: Heartbeats / `last_seen` timestamps. Liveness is purely cache-file-presence.
- NG-6: Identity rename. Re-init still requires hand-deleting state file's `[identity]` (or the legacy `.developer`).
- NG-7: Removal of `Layout::tasks_current()` / `Layout::developer_file()` accessors. Migration's reader needs them; remove in a follow-up task once a release ships with migration.
- NG-8: Multi-task focus per session.
- NG-9: Capturing `.state.toml` into `.ark.db` snapshots. Skipped per G-12.
- NG-10: `ark upgrade` migration step. Migration is read-on-load, write-on-mutate, delete-legacy-on-write. Self-healing.
- NG-11: A pure-Rust Windows process-tree walker without `windows-sys`. The `windows-sys` crate is the maintained, low-overhead path.

[**Architecture**]

New module tree under `crates/ark-core/src/`:

```
crates/ark-core/src/
├── state_file/                        (NEW)
│   ├── mod.rs                         pub use of the surface; invariants doc
│   ├── model.rs                       StateFile, Identity, Tasks, Session + serde
│   ├── io.rs                          load_state, state_mutate, lock acquire/release, atomic write
│   ├── reconcile.rs                   two-way: add_missing_active_from_disk + drop_stale
│   └── migrate.rs                     synthesize_from_legacy + delete_legacy_files
├── session/                           (NEW)
│   ├── mod.rs                         pub use; cache-file naming convention
│   ├── ppid.rs                        cfg-gated cross-platform parent_id() -> u32
│   ├── cache.rs                       resolve_session_id, release_session_id, project_hash
│   └── gc.rs                          prune_dead_sessions (in-memory)
├── commands/agent/task/
│   ├── resume.rs                      (NEW) task_resume
│   ├── discard.rs                     (NEW) task_discard, --force, template-diff guard
│   ├── new.rs                         MOD: append to active + register session via state_mutate
│   ├── archive.rs                     MOD: state_mutate first, then rename
│   └── mod.rs                         MOD: pub mod resume; pub mod discard;
├── commands/agent/workspace/
│   └── identity.rs                    MOD: bodies delegate to state_file (API stable)
├── commands/context/gather.rs         MOD: focused slug via state_file (this session only)
├── commands/agent/task/worktree/
│   ├── discovery.rs                   MOD: enumerate via state.tasks.active per worktree
│   └── list.rs                        MOD: same
├── commands/unload.rs                 MOD: skip set adds state_file/state_lock_file/tmp glob;
│                                      keeps developer_file during migration window
├── layout.rs                          ADD state_file(), state_lock_file(); KEEP legacy accessors
├── error.rs                           ADD StateTomlCorrupt, StateLockContended, TaskStillActive
└── lib.rs                             ADD pub use for state_file, session, resume, discard
```

CLI plumbing in `crates/ark-cli/src/`:

```
crates/ark-cli/src/
└── agent_cli.rs                       MOD: add Resume(TaskSlugArgs) and Discard(TaskDiscardCliArgs);
                                       MOD: resolve_slug body delegates to resolve_slug_via_state
```

Templates:

```
templates/ark/
└── .gitignore                         MOD: add .state.toml, .state.toml.lock, .state.toml.tmp.*
                                       (additive; legacy .developer line stays for migration window)
```

Cargo manifest:

```
crates/ark-core/Cargo.toml             MOD: add uuid = { version = "1", features = ["v4"] };
                                       MOD: add [target.'cfg(windows)'.dependencies] windows-sys
                                       with features Win32_System_Threading,
                                       Win32_System_Diagnostics_ToolHelp, Win32_Foundation
```

Module coupling:

```
state_file → io::PathExt, io::hash_bytes, layout, error
session    → io::PathExt, io::hash_bytes, layout, error
commands/agent/task/{new,archive,resume,discard} → state_file, session, state (existing TaskToml)
commands/agent/workspace/identity → state_file (replaces direct file I/O)
commands/context/gather → state_file, session (focused slug for *this* session)
commands/agent/task/worktree/{discovery,list} → state_file (per-worktree Layout, enumerate active)
commands/unload → state_file (path constants only; not state_mutate)
```

`state_file` and `session` MUST NOT import each other.

Call graph: `task new --slug a --tier quick`

```
task_new(opts)
  ├── validate_slug(&opts.slug)
  ├── if task_dir.exists() → Error::TaskAlreadyExists
  ├── task_dir.ensure_dir()
  ├── copy_template("PRD", &task_dir.join("PRD.md"))
  ├── build_task_toml(&opts).save(&task_dir)
  └── state_mutate(&layout, |state| {
        if !state.tasks.active.is_empty() {
            eprintln!("warn: {n} active task(s); see `ark agent task resume`");
        }
        state.tasks.active.push(opts.slug.clone());
        let id = resolve_session_id(&layout)?;
        let pid = session::ppid::parent_id();
        state.sessions.insert(id.0, Session { focus: opts.slug.clone(), pid });
        Ok(())
      })?
```

Call graph: `task archive --slug a` (state-mutate first; safe under two-way reconcile)

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── if tier == Deep: spec_extract + spec_register (unchanged)
  ├── state_mutate(&layout, |state| {
  │     state.tasks.active.retain(|s| s != &opts.slug);
  │     for sess in state.sessions.values_mut() { /* clear if focus matches */ }
  │     let id = resolve_session_id(&layout)?;
  │     if state.sessions.get(&id.0).map(|s| &s.focus) == Some(&opts.slug) {
  │         state.sessions.remove(&id.0);
  │         release_session_id(&layout, &id)?;
  │     }
  │     Ok(())
  │   })?
  ├── toml.phase = Archived; toml.archived_at = Some(now); toml.save(&task_dir)
  ├── task_dir.rename_to(&archive_path)?     // if this fails, reconcile drops it on next load
  └── record_task(...)?
```

Call graph: `task worktree list` (post-rewrite per R-004)

```
worktree_list()
  ├── git_worktree_list_porcelain()
  └── for each <wt> under <root>/<cfg.worktree_dir>/:
        ├── wt_layout = Layout::new(&wt)
        ├── state = state_file::load_state(&wt_layout)?     // two-way reconcile fills active
        └── for slug in &state.tasks.active:
              ├── task_dir = wt_layout.task_dir(slug)
              ├── toml = TaskToml::load(&task_dir)?
              └── emit row "{slug} {branch} {wt_path} (updated_at={toml.updated_at})"
```

If a worktree's state file is missing AND no legacy `.current` exists, `load_state` returns a default `StateFile`; two-way reconcile populates `active` from the worktree's `.ark/tasks/` directly. Inventory works regardless of session-cache state.

[**Data Structure**]

```rust
// crates/ark-core/src/state_file/model.rs

use std::collections::BTreeMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,

    #[serde(default)]
    pub tasks: Tasks,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sessions: BTreeMap<String, Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub initialized_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tasks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub focus: String,
    pub pid: u32,
}
```

```rust
// crates/ark-core/src/session/cache.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

pub fn cache_file_path(layout: &Layout, ppid: u32) -> PathBuf;
```

```rust
// crates/ark-core/src/session/ppid.rs

/// Returns the calling process's parent process id.
///
/// On Unix: delegates to `std::os::unix::process::parent_id`.
/// On Windows: walks the toolhelp snapshot to find the parent of
/// `GetCurrentProcessId()`. Returns the calling process's own PID
/// (`std::process::id()`) on snapshot/walk failure so session
/// machinery degrades gracefully.
pub fn parent_id() -> u32;
```

New error variants in `crates/ark-core/src/error.rs`:

```rust
#[error("state file `{path}` is corrupt: {source}")]
StateTomlCorrupt {
    path: PathBuf,
    #[source]
    source: toml::de::Error,
},

#[error("state file `{path}` is locked by another process; gave up after backoff")]
StateLockContended { path: PathBuf },

#[error("task `{slug}` has user content in {file}; pass --force to discard anyway")]
TaskStillActive { slug: String, file: String },
```

[**API Surface**]

Library re-exports added to `crates/ark-core/src/lib.rs`:

```rust
pub use state_file::{Identity, Session, StateFile, load_state, state_mutate};
pub use session::{SessionId, cache_file_path, parent_id, release_session_id, resolve_session_id};
pub use commands::agent::task::{
    TaskDiscardOptions, TaskDiscardSummary, TaskResumeOptions, TaskResumeSummary,
    task_discard, task_resume,
};
```

Core public functions:

```rust
// state_file/io.rs
pub fn load_state(layout: &Layout) -> Result<StateFile>;
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
where F: FnOnce(&mut StateFile) -> Result<()>;

// state_file/reconcile.rs
/// Two-way: enumerate task dirs, add missing actives, then drop archived/missing
/// actives, then drop sessions whose focus is no longer in active.
pub fn reconcile_against_disk(layout: &Layout, state: &mut StateFile) -> Result<()>;

// session/ppid.rs
pub fn parent_id() -> u32;

// session/cache.rs
pub fn resolve_session_id(layout: &Layout) -> Result<SessionId>;
pub fn release_session_id(layout: &Layout, id: &SessionId) -> Result<()>;

// session/gc.rs
pub fn prune_dead_sessions(layout: &Layout, state: &mut StateFile);

// commands/agent/task/resume.rs
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary>;

// commands/agent/task/discard.rs
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary>;
```

Identity API and CLI subcommand additions are unchanged from PLAN 00.

[**Constraints**]

- C-1: Locking primitive is stdlib `File::try_lock` (stable Rust 1.89+). Cross-platform parent-id is provided via `session::ppid::parent_id()`, a `cfg`-gated shim. New ark-core deps: `uuid` (any host), `windows-sys` under `[target.'cfg(windows)'.dependencies]` only. No other deps for this feature.
- C-2: Lock file is `.ark/.state.toml.lock`. Backoff is exactly 5 attempts at 10/20/40/80/160 ms (cumulative ≤ 320 ms). On exhaustion → `Error::StateLockContended { path }`. Lock auto-released on `File` drop.
- C-3: State file write is atomic: write `.state.toml.tmp.<pid>`, then `rename_to(.state.toml)`. Cross-platform via `std::fs::rename`.
- C-4: At the start of every `state_mutate` (after lock acquired), enumerate and unlink any `.state.toml.tmp.*` orphan files. Safe under lock.
- C-5: Cache file naming: `<std::env::temp_dir()>/ark-session-<project_hash>-<ppid>.id`. `project_hash` is the first 16 hex chars of `hash_bytes(layout.root().to_string_lossy().as_bytes())`. Cache file body is the bare UUID v4 hex string, no trailing newline. PPID source is `session::ppid::parent_id()`.
- C-6: No `.unwrap()` in production code. The single allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`'s pattern.
- C-7: All filesystem access in `state_file/`, `session/`, and the new task ops routes through `io::PathExt`. No bare `std::fs::*`.
- C-8: All `.ark/`-relative paths route through `Layout` helpers. New constants: `STATE_FILE = ".ark/.state.toml"`, `STATE_LOCK_FILE = ".ark/.state.toml.lock"`. New accessors: `Layout::state_file()`, `Layout::state_lock_file()`. Legacy `tasks_current()` and `developer_file()` accessors are kept for migration.
- C-9: Reconcile drops a `tasks.active` entry when the corresponding `task.toml` is missing OR `phase == Archived`. Drops a `sessions.*` entry when its slug-focus is no longer in `tasks.active` after the active reconcile.
- C-10: GC drops a `sessions.*` entry when `cache_file_path(layout, session.pid)` does not exist OR exists but its trimmed contents do not equal the session UUID.
- C-11: A read (`load_state`) does NOT delete legacy files. Migration's delete step happens only on the next successful `state_mutate` save.
- C-12: `state_mutate` is the sole path that mutates the state file. Direct `StateFile::save` from outside the module is private.
- C-13: `task_discard`'s template-diff guard reads each seeded file (`PRD.md`, `*_PLAN.md`, `*_REVIEW.md`, `VERIFY.md` — whichever exist) and compares against the embedded template via byte equality after stripping the first-line `{Feature Name}` placeholder substitution. First file with divergence → `Error::TaskStillActive { slug, file }`. `--force` skips the entire scan.
- C-14: The `Layout::tasks_current()` and `Layout::developer_file()` accessors carry a doc-comment "legacy migration accessor; remove after migration window once .state.toml is universal."
- C-15: State file's `[tasks].active` is deduped on every save: `state.tasks.active.sort(); state.tasks.active.dedup();`.
- C-16: No SessionStart hook integration. Sessions register only on `task new` / `task resume`.
- C-17: Each worktree's state file is independent. `Layout::new(<worktree_path>)` returns a Layout pointing at the worktree's own `.ark/`; `state_file::*` operations never reach across worktrees.
- C-18: `unload.rs` file-level skip set in BOTH walk sites (Stage A snapshot capture lines around 87, Stage B file enumeration around 171) is `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file(), layout.developer_file()]`. The `.state.toml.tmp.*` orphan glob is also excluded by extending `walk_files_excluding` (or adding a wrapper) to skip any file under `<root>/.ark/` matching `.state.toml.tmp.*`. Migration window: keeping `developer_file()` in the skip set is harmless after migration (the file is gone) and necessary before migration (legacy installs).
- C-19: `state_file::reconcile::reconcile_against_disk` runs in this order: (1) enumerate `.ark/tasks/<slug>/task.toml` excluding the `archive/` subdirectory; for each found slug whose `task.toml.phase != Archived`, push it to `state.tasks.active` if not already present; (2) drop `state.tasks.active` entries whose `task.toml` is missing or `phase == Archived`; (3) sort+dedup active per C-15; (4) drop `state.sessions.*` entries whose `focus` is no longer in `state.tasks.active`. Order matters: add must precede drop so a brief inconsistent intermediate state never collapses an active slug. The full reconcile is idempotent.
- C-20: `session::ppid::parent_id()` on Windows MUST return the calling process's own PID (`std::process::id()`) when toolhelp snapshot creation or walk fails. This degrades the multi-invocation-same-shell-shares-focus property to "single invocation = single session" on broken Windows environments, but never aborts the CLI invocation. Unix path has no failure mode.

---

## Runtime

[**Main Flow**]

Scenario A — `task new --slug a --tier quick` (single session):

1. CLI parses → `task_new(opts)`.
2. Validate slug; check for collision at `.ark/tasks/a/`.
3. Scaffold task dir (PRD.md, task.toml).
4. Open `state_mutate`:
   - Acquire `.ark/.state.toml.lock` exclusive (try_lock; backoff if contended).
   - Unlink any `.state.toml.tmp.*` orphans.
   - Read `.state.toml`. If missing, try legacy `.developer` + `tasks/.current`; synthesize in-memory state. Else parse TOML.
   - Two-way reconcile against on-disk task dirs (add missing actives; then drop archived/missing; then drop orphan sessions).
   - GC dead sessions (cache-file probe per C-10).
   - Run closure: warn if `active` non-empty; push `a`; insert/update `sessions.<self> { focus: "a", pid: parent_id() }`.
   - Serialize state to `.state.toml.tmp.<pid>`. `rename_to(.state.toml)`.
   - Unlink `.developer` and `tasks/.current` if present (migration finalize).
   - Lock guard drops; OS releases.
5. Print summary.

Scenario B — `task new --slug b` from a second shell, same `.ark/`:

Identical to Scenario A, except step 4's reconcile sees `active = ["a"]` (preserved from previous invocation OR re-derived from disk by C-19) and the closure's `eprintln!` fires. Result: `active = ["a", "b"]`; two `sessions.<id>` entries with distinct PPIDs and distinct focuses.

Scenario C — `task execute` with no `--slug`:

1. CLI dispatch → `resolve_slug(root, None)`.
2. `resolve_slug` calls `resolve_slug_via_state(layout)`:
   - `let id = resolve_session_id(layout)?;`
   - `let state = load_state(layout)?;` (includes two-way reconcile + GC)
   - `state.sessions.get(&id.0).map(|s| s.focus.clone()).ok_or(Error::NoCurrentTask { path: layout.state_file() })`
3. Phase transition runs against the resolved slug.

Scenario D — `task archive --slug a` (this session focused on `a`):

1. Load TaskToml; check_transition.
2. Deep-tier SPEC promotion (unchanged).
3. `state_mutate`:
   - Drop `"a"` from `active`.
   - For each session: if `focus == "a"`, drop the session entry entirely.
   - Resolve own session id; if it was just dropped, `release_session_id(layout, &id)`.
4. Mark TaskToml as Archived; save.
5. `task_dir.rename_to(archive_path)`.
6. `record_task(...)`.

Scenario E — `task worktree list` from the parent shell with no per-worktree session:

1. `git worktree list --porcelain` enumerates worktrees.
2. For each worktree under `.ark/worktrees/`:
   - `wt_layout = Layout::new(<wt_path>)`.
   - `state = state_file::load_state(&wt_layout)?` — runs two-way reconcile, so `state.tasks.active` reflects on-disk reality regardless of session presence.
   - For each `slug` in `state.tasks.active`: load `task.toml`; emit row.
3. No session focus consulted.

[**Failure Flow**]

- **Lock contention beyond backoff**: `Error::StateLockContended { path }`. CLI renders friendly message; user retries.
- **Corrupt state file**: `Error::StateTomlCorrupt { path, source }`. Recovery: hand-delete `.state.toml`; next mutation re-synthesizes from legacy if present, else creates fresh — and the two-way reconcile populates active from on-disk `task.toml`s.
- **Crash mid-write**: `.state.toml` is either pre-write or post-write (rename atomic). `.state.toml.tmp.<pid>` orphan reaped by next `state_mutate` (C-4). Lock released by OS on process death.
- **`task discard` on archived task**: `Error::TaskNotFound { slug }`.
- **`task discard` without --force, PRD edited**: `Error::TaskStillActive { slug, file: "PRD.md" }`.
- **`task resume <bogus>`**: `Error::TaskNotFound { slug }`.
- **`task archive` rename fails after state mutate succeeds**: state file says inactive; `task.toml.phase = Archived` (saved before rename); dir at `tasks/<slug>/`. Next `load_state` reconcile: enumerate finds `tasks/<slug>/task.toml` with `phase == Archived` → reconcile drops it from any (re-added) active. Net effect: active correctly excludes the slug; user re-runs archive or discard to complete the dir move.
- **GC drops own session by mistake**: cannot happen — `resolve_session_id` always (re)creates the cache file before any state read inside `state_mutate`.
- **PPID recycling on Unix**: rare; mitigated by cache-file deletion on archive/discard.
- **Windows toolhelp snapshot fails**: `parent_id()` returns own PID per C-20. Session-id cache keyed on own-PID; same shell's next CLI invocation re-derives a different own-PID (different process) → cache miss → new UUID → effective single-invocation session. Documented limitation.
- **`unload` after migration**: `.developer` no longer exists; skip-set entry is a no-op. `.state.toml` skipped per C-18. Snapshot captures `.ark/tasks/<*>/` so `load` can two-way-reconcile a fresh `.state.toml` on first read.

[**State Transitions**]

```
[no .state.toml, no legacy]      ──load_state──>      reconcile-from-disk → may be non-empty if task dirs exist
[no .state.toml, has legacy]     ──load_state──>      synthesize from legacy → reconcile-from-disk (in-memory only)
[has .state.toml]                ──load_state──>      parse → reconcile-from-disk → GC sessions
[any of above]                   ──state_mutate──>    closure on reconciled state
[state_mutate save success]      ──finalize──>        delete legacy files if present
[unload + load round-trip]       ──first load_state──> reconcile-from-disk repopulates active

session lifecycle:
  unregistered ──task new / task resume──> registered with focus
  registered  ──task archive (own slug)──> released (entry dropped, cache deleted)
  registered  ──task discard (own slug)──> released
  registered  ──cache file disappears──> dropped by GC on next load_state
```

---

## Implementation

[**Phase 1 — Foundations (no behavior change for users)**]

Files added; no existing call site changes; existing tests still pass.

- `crates/ark-core/src/session/ppid.rs` — implement first. Unix path is one-liner; Windows path is the toolhelp snapshot walk. Add ark-core dep `windows-sys` under `[target.'cfg(windows)'.dependencies]` with features `Win32_System_Threading`, `Win32_System_Diagnostics_ToolHelp`, `Win32_Foundation`. Unit test `parent_id_returns_nonzero_on_current_platform` validates compile + run on whichever OS CI executes.
- `crates/ark-core/src/state_file/{mod,model,io,reconcile,migrate}.rs` — `reconcile.rs` implements `reconcile_against_disk` per C-19 (add-then-drop).
- `crates/ark-core/src/session/{mod,cache,gc}.rs` — `cache.rs` calls `ppid::parent_id()` instead of `std::process::parent_id`.
- `crates/ark-core/src/error.rs` — add `StateTomlCorrupt`, `StateLockContended`, `TaskStillActive`.
- `crates/ark-core/src/layout.rs` — add `STATE_FILE`/`STATE_LOCK_FILE` constants and `state_file()`/`state_lock_file()` accessors.
- `crates/ark-core/src/lib.rs` — `pub mod state_file; pub mod session;` plus the new re-exports (including `parent_id`).
- `crates/ark-core/Cargo.toml` — add `uuid = { version = "1", features = ["v4"] }`. Add `windows-sys` under `[target.'cfg(windows)'.dependencies]`. Pin `rust-version = "1.89"` if not already pinned.

Unit tests in this phase (in-module `#[cfg(test)]`):

- `state_file::io::tests` — load_state default+legacy+pure-read; state_mutate atomic+legacy-cleanup+orphan-tmp-unlink+lock-contended+concurrent-serialize.
- `state_file::reconcile::tests` — drops_missing/archived/orphan-session; **adds_missing_active_from_disk**, **recovers_active_from_disk_when_state_file_deleted**, **add_then_drop_order_is_idempotent**, **archive_rename_failure_recovery_via_reconcile**.
- `state_file::migrate::tests` — synthesize variants.
- `session::ppid::tests` — `parent_id_is_nonzero` (cross-platform smoke).
- `session::cache::tests` — round-trip, project_hash naming, release idempotent.
- `session::gc::tests` — prune-missing/mismatched/keep-matching.

[**Phase 2 — Rewire Callers (behavior preserved, internals switched)**]

- `commands/agent/workspace/identity.rs` — replace bodies with `state_file` delegations.
- `commands/agent/workspace/init.rs:78` — `write_developer_file` call site unchanged.
- `commands/agent/task/new.rs:135-137, 262` — replace direct `tasks_current().write_bytes(...)` with `state_mutate` closure.
- `commands/agent/task/archive.rs:103-141` — reverse the rename-first ordering. Update line-103 comment block to document the new invariant (refers to C-19).
- `commands/context/gather.rs:314` — replace `tasks_current().read_text_optional()` with `resolve_session_id` + `load_state` + this-session-focus lookup.
- `commands/agent/task/worktree/discovery.rs:94, list.rs:87` — replace `.current` reads with `state.tasks.active` enumeration per Scenario E.
- `commands/unload.rs:87, 171` — extend the skip set per C-18: replace the two-element array with `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file(), layout.developer_file()]`. Extend `walk_files_excluding` (or add a wrapper) to skip `.state.toml.tmp.*` orphans.
- `crates/ark-cli/src/agent_cli.rs:384-394` — replace `resolve_slug` body to delegate to `resolve_slug_via_state`. Keep the function name and signature.

Tests adjusted in this phase:

- `agent_lifecycle.rs:48`, `archive.rs:218` — `!.current.exists()` → `!active.contains(&slug)`.
- `init.rs:580, 590` — `.developer` existence assertion → state file's `identity.is_some()`.
- `context/gather.rs:490, 524` — mock `.current` writes → `state_mutate` calls in test setup.
- `unload.rs::tests::unload_excludes_worktree_contents` — extend to also assert `.state.toml` is excluded (V-IT-7 base).

[**Phase 3 — New Ops + CLI Surface + Docs**]

- `commands/agent/task/resume.rs` and `discard.rs` — implement per signatures above.
- `commands/agent/task/mod.rs` — `pub mod resume; pub mod discard;` and re-exports.
- `crates/ark-cli/src/agent_cli.rs`: add `Resume(TaskSlugArgs)` and `Discard(TaskDiscardCliArgs)` variants; dispatch arms call the new functions and `render(...)`.
- `crates/ark-core/src/lib.rs` — add the two new pub-uses.
- `templates/ark/.gitignore` — add `.state.toml`, `.state.toml.lock`, `.state.toml.tmp.*` lines.
- `templates/ark/workflow.md` — brief subsection under §6 Mechanics: multi-session focus, `task resume`, `task discard`, `.state.toml` per-checkout per-worktree, gitignored, skipped by unload.
- `.claude/commands/ark/quick.md` and `design.md` — minor updates: warn-on-active-nonempty; quick-tier `--worktree` opt-in.

Integration tests in `crates/ark-cli/tests/`:

- New file `agent_session.rs`:
  - `multi_session_focus_isolation` (V-IT-1)
  - `gc_drops_dead_session_on_load` (V-IT-2)
  - `lock_contention_succeeds_after_backoff` (V-IT-3)
  - `migration_synthesizes_from_legacy_and_deletes` (V-IT-4)
  - `discard_force_removes_edited_task` (V-IT-5)
  - `resume_invalid_slug_errors`
  - `worktree_isolation_state_file` (V-IT-6)
  - `unload_excludes_state_file` (V-IT-7)
  - `worktree_list_works_after_session_cache_loss` (V-IT-8)
- Extensions to `agent_lifecycle.rs`: post-archive assertion update.

---

## Trade-offs

- T-1: **Cross-platform parent-id source — `sysinfo` crate vs. bespoke `windows-sys` shim.** `sysinfo` (≈10 deps, ~1MB) is multi-purpose, well-maintained, gives `Pid::from(parent_id)` cross-platform. The bespoke shim is ~30 LOC of `cfg(windows)` code calling `windows-sys`'s toolhelp APIs directly. **Choice: bespoke shim.** Reasoning: ark-core already exercises low-overhead-deps discipline; pulling `sysinfo` for one function is over-budget. The shim is small and isolated to one file (`session/ppid.rs`).
- T-2: One `.state.toml` vs. two files. Unchanged choice: one file. (TR-2 raised a privacy concern; addressed by C-18/G-12.)
- T-3: Archive ordering — state_mutate-first vs. rename-first. Unchanged choice (state_mutate-first), now safe under C-19's two-way reconcile.
- T-4: Discard guard — template-diff vs. always-`--force`. Unchanged: template-diff.
- T-5: Project-hash length — 16 hex chars vs. full 64. Unchanged: 16.
- T-6: Deduplication of `tasks.active` — BTreeSet vs. Vec+sort/dedup. Unchanged: Vec+sort/dedup.
- T-7: Identity API preservation. Unchanged: preserve names, swap bodies.
- T-8: **Windows `parent_id()` failure behavior — abort vs. degrade.** When toolhelp snapshot fails on Windows, the session machinery has no good answer. Aborting on a one-time WinAPI failure would be hostile (especially for Codex/OpenCode users who never asked about session machinery). Degrading to `std::process::id()` (own PID) means the same shell's next CLI invocation gets a different PID (different process), so no two invocations share a session — effectively "one Ark CLI call = one session." Focus is lost between invocations on broken Windows. **Choice: degrade.** The cost falls on the rare-broken-Windows user who probably won't notice (they'd just type `--slug` more often).

---

## Validation

[**Unit Tests**]

- V-UT-1: `load_state_returns_default_when_missing_and_no_legacy_and_no_task_dirs`.
- V-UT-2: `load_state_synthesizes_from_legacy_developer_and_current`.
- V-UT-3: `load_state_does_not_delete_legacy_on_pure_read`.
- V-UT-4: `state_mutate_round_trips_atomically`.
- V-UT-5: `state_mutate_deletes_legacy_files_on_first_save`.
- V-UT-6: `state_mutate_unlinks_orphan_tmp_files_on_acquire`.
- V-UT-7: `state_mutate_returns_lock_contended_after_backoff`.
- V-UT-8: `reconcile_drops_active_slug_with_missing_dir`.
- V-UT-9: `reconcile_drops_active_slug_with_phase_archived`.
- V-UT-10: `reconcile_drops_session_whose_focus_no_longer_active`.
- V-UT-11: `migrate_synthesize_handles_missing_developer_or_missing_current_independently`.
- V-UT-12: `parent_id_is_nonzero_on_current_platform` (NEW per R-001 — cross-platform smoke; runs on whichever CI OS executes; pair with cross-OS CI matrix).
- V-UT-13: `session_resolve_round_trips_within_same_ppid`.
- V-UT-14: `session_cache_file_path_uses_project_hash_and_ppid`.
- V-UT-15: `gc_drops_session_with_missing_cache_file`.
- V-UT-16: `gc_drops_session_with_mismatched_uuid`.
- V-UT-17: `gc_keeps_session_with_matching_cache`.
- V-UT-18: `task_resume_invalid_slug_returns_task_not_found`.
- V-UT-19: `task_resume_sets_session_focus_idempotently`.
- V-UT-20: `task_discard_template_unchanged_succeeds_without_force`.
- V-UT-21: `task_discard_edited_prd_refuses_without_force`.
- V-UT-22: `task_discard_with_force_removes_edited_task`.
- V-UT-23: `task_discard_archived_task_returns_task_not_found`.
- V-UT-24 (NEW per R-002): `reconcile_adds_missing_active_from_disk` — pre-seed `task.toml` with `phase != Archived`; state.active empty; after `load_state`, slug appears in active.
- V-UT-25 (NEW per R-002): `recovers_active_from_disk_when_state_file_deleted` — populate `.ark/tasks/foo/task.toml`; delete `.state.toml`; `load_state` returns `active = ["foo"]`.
- V-UT-26 (NEW per R-002): `archive_rename_failure_recovery_via_reconcile` — set `task.toml.phase = Archived` but leave dir at `tasks/<slug>/`; state.active still contains slug; after `load_state`, reconcile drops it.
- V-UT-27 (NEW per C-19): `add_then_drop_order_is_idempotent` — call `reconcile_against_disk` twice; second is a no-op.
- V-UT-28 (NEW per C-18): `unload_skip_set_includes_state_file_and_lock` — unit test on the skip-set builder verifying `state_file()`, `state_lock_file()`, `developer_file()` all present.
- V-UT-29 (NEW per C-18): `walk_files_excluding_skips_state_toml_tmp_orphans` — pre-seed `.state.toml.tmp.<n>` files; assert excluded.

[**Integration Tests**]

- V-IT-1: `multi_session_focus_isolation_across_two_sessions` — two distinct simulated PPIDs each run `task new`; each `--slug`-less command resolves to its own focus.
- V-IT-2: `task_new_warns_on_non_empty_active_set`.
- V-IT-3: `migration_e2e_via_task_new` — pre-seed `.developer` + `.current`; run `task new <other>`; verify `.state.toml` shape and legacy files gone.
- V-IT-4: `worktree_isolation_state_file` — `task new --worktree --slug w1` from parent; parent's `.state.toml` has `active=[]`; worktree's has `active=["w1"]`.
- V-IT-5: `archive_clears_focus_and_releases_cache`.
- V-IT-6: `lock_contention_resolves_within_backoff_window`.
- V-IT-7 (NEW per R-003): `unload_excludes_state_file` — `ark init --developer X`; `task new --slug a`; `ark unload`; assert `.ark.db` does not contain `state.toml` payload, identity, or session UUIDs. Then `ark load` into fresh tempdir; assert `state.toml` re-derives `active=["a"]` from captured `tasks/a/`.
- V-IT-8 (NEW per R-004): `worktree_list_works_after_session_cache_loss` — create worktree-backed task; delete the worktree's session cache file under temp dir; `task worktree list` from the parent shell still enumerates the active task.

[**Failure / Robustness Validation**]

- V-F-1: `crash_mid_write_leaves_either_old_or_new_state_never_partial`.
- V-F-2: `lock_contention_resolves_within_backoff_window`.
- V-F-3: `lock_contention_beyond_backoff_returns_state_lock_contended`.
- V-F-4: `archive_rename_failure_state_says_inactive_dir_remains_until_reconcile` — covered by V-UT-26.
- V-F-5 (NEW per C-20): `windows_parent_id_failure_falls_back_to_own_pid` — Windows-only test; mock toolhelp failure (or skip if mocking is hard); verify `parent_id()` returns `std::process::id()` and CLI does not abort.

[**Edge Case Validation**]

- V-E-1: `state_mutate_with_concurrent_load_state_does_not_corrupt`.
- V-E-2: `empty_active_set_serializes_to_omitted_field_per_skip_serializing_if`.
- V-E-3: `dedup_preserves_first_occurrence_order_after_sort`.
- V-E-4: `gc_handles_session_with_pid_that_belongs_to_unrelated_process`.
- V-E-5: `discard_with_no_seeded_files_present_succeeds_without_force`.
- V-E-6 (NEW per C-19): `reconcile_with_archive_subdir_does_not_double_count` — verify enumeration excludes `.ark/tasks/archive/` correctly.
- V-E-7 (NEW per G-12): `unload_load_round_trip_preserves_active_via_reconcile_not_capture` — round-trip; new `.state.toml` materializes from disk truth.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1               | V-UT-1, V-UT-4, V-UT-5, V-UT-25 |
| G-2               | V-IT-1, V-IT-4, V-UT-13 |
| G-3               | V-UT-18..V-UT-23 |
| G-4               | (existing identity tests still pass) |
| G-5               | V-UT-2, V-UT-3, V-UT-5, V-IT-3 |
| G-6               | V-UT-4, V-UT-6, V-UT-7, V-F-1, V-F-2, V-F-3 |
| G-7               | V-UT-15, V-UT-16, V-UT-17 |
| G-8               | V-IT-2 |
| G-9               | V-IT-4, V-IT-8 |
| G-10              | V-UT-18, dispatch test in `agent_lifecycle` |
| G-11              | V-UT-20..V-UT-23 |
| G-12              | V-IT-7, V-E-7, V-UT-28, V-UT-29 |
| G-13              | V-UT-12, V-F-5 |
| C-1, C-2          | V-UT-7, V-F-2, V-F-3 |
| C-3               | V-UT-4, V-F-1 |
| C-4               | V-UT-6 |
| C-5               | V-UT-14 |
| C-9               | V-UT-8, V-UT-9, V-UT-10 |
| C-10              | V-UT-15, V-UT-16, V-UT-17 |
| C-11              | V-UT-3 |
| C-13              | V-UT-20..V-UT-22, V-E-5 |
| C-15              | V-E-3 |
| C-17              | V-IT-4 |
| C-18              | V-IT-7, V-UT-28, V-UT-29 |
| C-19              | V-UT-24, V-UT-25, V-UT-26, V-UT-27, V-E-6 |
| C-20              | V-F-5 |
