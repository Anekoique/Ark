# `task-concurrency-control` PLAN `02`

> Status: Draft
> Feature: `task-concurrency-control`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: none

---

## Summary

PLAN 02 keeps the architecture from PLAN 01 but resolves the four REVIEW 01 findings:

- **GC ownership moves to `state_file::reconcile`.** The `session` module becomes a true leaf that exposes only stateless cache primitives (`cache_matches`, `cache_file_path`, `parent_id`). Pruning of `[sessions.*]` entries by liveness probe lives next to the other StateFile mutations in `state_file::reconcile`. The "`state_file` and `session` MUST NOT import each other" rule survives intact and now matches the public API.
- **`task new` warn semantics fixed.** After two-way reconcile, the `state_mutate` closure computes `had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug)`, warns iff that is true, then ensures the new slug is in `active` exactly once via `if !contains(&opts.slug) { push }`. V-IT-2 splits into "first task is silent" and "second distinct task warns" — both explicit acceptance criteria.
- **Archive ordering reverts to rename-first** for deep-tier SPEC safety. T-3 is rewritten: SPEC extract/register and the `task.toml.phase = Archived` save still happen post-rename (current behavior preserved), and the `state_mutate` cleanup happens AFTER the rename. If state_mutate fails post-rename, two-way reconcile recovers: `task.toml.phase = Archived` lives at `archive_path`; reconcile's add-pass excludes `archive/`, so the slug is not re-added; reconcile's drop-pass removes it from any stale active entry. Active-set integrity is recoverable; promoted SPEC files stay durable.
- **Session-provider gains a test seam.** `session::ppid::Ppid` becomes a small trait with `RealPpid` (production) and `StubPpid(u32)` (tests). `resolve_session_id` takes `&dyn Ppid`; CLI passes `RealPpid::new()`; integration tests pass `StubPpid(12345)` and `StubPpid(67890)` to drive deterministic multi-session scenarios. V-IT-1 is now executable without process-topology tricks; V-F-5 can mock the Windows failure path through a dedicated `WindowsToolhelpFailure` stub variant.

The architecture diagram, data structures, error variants, and CLI surface are otherwise unchanged from PLAN 01.

## Log

[**Added**]

- `state_file::reconcile::prune_dead_sessions` — moved from `session::gc` per R-001.
- `session::cache::cache_matches(layout, pid, uuid) -> bool` — stateless predicate replacing `gc::prune_dead_sessions` from the `session` side.
- `Ppid` trait + `RealPpid` + `StubPpid` in `session::ppid` per R-004.
- New constraint C-21 (`task_new` warn semantics: post-reconcile `had_other_active` filter) per R-002.
- New constraint C-22 (archive ordering: rename-first; state_mutate cleanup after) per R-003.
- New trade-off T-9 (session-provider trait shape: trait+stub vs. thread-local override).
- New tests V-UT-30 (had_other_active false on first task), V-UT-31 (had_other_active true on second distinct task), V-UT-32 (`Ppid` stub injection round-trip), V-IT-9 (deep archive failure-flow: state_mutate fail post-rename → reconcile recovers).

[**Changed**]

- C-19 (reconcile order) extended: GC of dead sessions is now step (5) of the reconcile pass, after the active-set add/drop and orphan-focus drop. Add-then-drop-then-prune order documented.
- T-3 (archive ordering) reverted: rename-first; state_mutate cleanup after rename. Two-way reconcile still provides the recovery primitive for state-mutate failure post-rename, but SPEC promotion happens at archive_path post-rename (the original design's invariant).
- Architecture diagram: `session/gc.rs` removed; `state_file/reconcile.rs` doc updated to mention session pruning. `session/ppid.rs` doc updated to mention the trait.
- `[**API Surface**]`: `session::gc::prune_dead_sessions` replaced with `state_file::reconcile::prune_dead_sessions`. `session::cache::cache_matches` added. `Ppid` trait added.
- C-1 (deps): unchanged.
- Module coupling: `state_file` now imports `session::cache` for the `cache_matches` predicate (one-way). `session` continues to import nothing from `state_file`. The "MUST NOT import each other" rule is replaced with "session MUST NOT import state_file."
- Phase 1 implementation order: `session::ppid` and `Ppid` trait still first; then `session::cache` (with `cache_matches`); then `state_file::reconcile` (which imports the predicate).
- Validation: V-IT-2 split into V-IT-2a (first task silent) and V-IT-2b (second distinct task warns); V-F-5 spec'd to use `Ppid` stub for Windows-failure simulation.
- Acceptance Mapping table updated for C-21, C-22, and the new V-IDs.

[**Removed**]

- `session/gc.rs` from the architecture diagram (folded into `state_file/reconcile.rs`).
- `session::gc::prune_dead_sessions` from the API surface and re-exports (folded into `state_file::reconcile::prune_dead_sessions`).
- The PLAN 01 archive-ordering reversal (T-3 state-mutate-first). T-3's "Choice" line is now "rename-first; state_mutate after rename."

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID    | Decision | Resolution |
|--------|-------|----------|------------|
| Review | R-001 | Accepted | Moved `prune_dead_sessions` from `session::gc` to `state_file::reconcile`. `session::cache` now exposes `cache_matches(layout, pid, uuid) -> bool` as a stateless predicate that `reconcile` calls per session entry. Module coupling rule updated: `session` MUST NOT import `state_file`; `state_file` MAY import `session::cache` (one-way). API surface, architecture diagram, and Phase 1 implementation order all updated. |
| Review | R-002 | Accepted | New C-21 specifies `task_new`'s post-reconcile filter: `had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug)`. Warn iff `had_other_active`. The closure's push is guarded: `if !state.tasks.active.contains(&opts.slug) { push }`. Validation V-IT-2 split into V-IT-2a (`first_task_emits_no_warning`) and V-IT-2b (`second_distinct_task_warns`). Both are acceptance gates. |
| Review | R-003 | Accepted | T-3 reverted to rename-first. New C-22 specifies the archive ordering: (1) load TaskToml + check_transition, (2) `task_dir.rename_to(archive_path)`, (3) `toml.phase = Archived; toml.save(archive_path)`, (4) deep-tier SPEC promotion from archive_path (unchanged from current code), (5) `state_mutate` cleanup (drop from active, clear focuses, release own session id), (6) `record_task`. If step (5) fails after (2)-(4) succeed, two-way reconcile recovers because (a) the task dir is now under `archive/` which reconcile's add-pass excludes, and (b) any stale active entry gets dropped by reconcile's drop-pass on next load. SPEC files and INDEX.md are durable post-(4); state cleanup is recoverable. New test V-IT-9 covers this failure-flow. |
| Review | R-004 | Accepted | `session::ppid::Ppid` becomes a trait: `pub trait Ppid { fn parent_id(&self) -> u32; }`. Production: `RealPpid` (UnitStruct) delegates to the `cfg`-gated platform shim. Tests: `StubPpid(pub u32)` returns the held value. `resolve_session_id` takes `&dyn Ppid`; CLI dispatch constructs `RealPpid` once at startup. New test V-UT-32 round-trips through `StubPpid(12345)`. V-IT-1 (multi-session) uses `StubPpid(12345)` and `StubPpid(67890)` to deterministically simulate two sessions. V-F-5 (Windows toolhelp failure) uses a dedicated `WindowsToolhelpFailure` stub. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec

[**Goals**]

- G-1: A single per-checkout `.ark/.state.toml` file carries identity, the active-task set, and a per-session focus map. Truth still lives in `.ark/tasks/<slug>/task.toml`; the state file is an index reconciled (two-way: add + drop + prune-sessions) on every read.
- G-2: Concurrent CLI invocations from independent shells in the same `.ark/` each get an independent `[sessions.<uuid>]` entry and an independent focused task. No session can clobber another's focus.
- G-3: Two new agent ops — `ark agent task resume <slug>` and `ark agent task discard <slug>` — extend the agent task verb set (extends `ark-agent-namespace` SPEC G-3).
- G-4: The identity API (`read_developer_name`, `require_developer_name`, `write_developer_file` in `commands/agent/workspace/identity.rs`) preserves its public signatures; bodies delegate to the new state-file API. The 4 existing call sites are unchanged.
- G-5: Legacy `.ark/.developer` and `.ark/tasks/.current` are auto-migrated on the first state-file mutation and then deleted. The reader tolerates either layout indefinitely so a pure read in a not-yet-migrated install does not change the on-disk state.
- G-6: State-file mutations are atomic across crash *and* across concurrent CLI invocations on Linux, macOS, and Windows. Crash mid-write leaves no partial `.state.toml`. Concurrent writers serialize via an OS-level file lock with bounded backoff.
- G-7: Dead sessions are pruned transparently on the next state read (in-memory; persisted on the next mutation). A session is "dead" when its temp-dir cache file is missing or mismatched. Pruning lives in `state_file::reconcile`, called via `session::cache::cache_matches` predicate.
- G-8: `task new` warns (does not refuse) when there are other active tasks before the new one is appended. The warn check excludes the just-created slug (added to `active` by the prior two-way reconcile pass). First task is silent; second distinct task warns.
- G-9: Each git worktree's `.ark/` owns its own `.state.toml`. `task new --worktree` writes only to the worktree's state file. `task worktree list` and `task worktree cleanup` enumerate via each worktree's `state.tasks.active`, **not** via session focus.
- G-10: `--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask`.
- G-11: `task discard <slug>` refuses without `--force` when seeded files (PRD/PLAN/etc.) differ from their templates. Always refuses if the task is already archived.
- G-12: `.ark/.state.toml`, `.ark/.state.toml.lock`, and any `.ark/.state.toml.tmp.*` orphan files are skipped by `unload.rs` in both walk sites. Identity stays per-machine and is never captured into `.ark.db`. Active task state is recoverable on `load` via two-way reconcile.
- G-13: Cross-platform parent-id via the `Ppid` trait. Production `RealPpid` delegates to a `cfg`-gated shim: Unix uses `std::os::unix::process::parent_id`; Windows uses `windows-sys`'s `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW`. Failure on Windows returns the calling process's own PID (`std::process::id()`). Tests use `StubPpid(u32)` for deterministic injection.
- G-14: Deep-tier archive remains rename-first to preserve SPEC promotion durability. `state_mutate` cleanup runs AFTER rename and after SPEC promotion. If cleanup fails, two-way reconcile recovers state-file integrity on next read; SPEC files stay correct (the slug they reference IS archived).

- NG-1: Workflow-phase model changes — ROADMAP item #2.
- NG-2: SessionStart hook integration.
- NG-3: Explicit `session end` / `session list` ops.
- NG-4: Cross-host coordination (NFS-shared `.ark/`).
- NG-5: Heartbeats / `last_seen` timestamps.
- NG-6: Identity rename.
- NG-7: Removal of `Layout::tasks_current()` / `Layout::developer_file()` accessors.
- NG-8: Multi-task focus per session.
- NG-9: Capturing `.state.toml` into `.ark.db` snapshots.
- NG-10: `ark upgrade` migration step.
- NG-11: A pure-Rust Windows process-tree walker without `windows-sys`.
- NG-12: A test-only mutable-global session-provider override (e.g. `thread_local!`). The trait+stub approach (T-9) is preferred.

[**Architecture**]

New module tree under `crates/ark-core/src/`:

```
crates/ark-core/src/
├── state_file/                        (NEW)
│   ├── mod.rs                         pub use; invariants doc
│   ├── model.rs                       StateFile, Identity, Tasks, Session + serde
│   ├── io.rs                          load_state, state_mutate, lock acquire/release, atomic write
│   ├── reconcile.rs                   two-way: add_missing + drop_stale + prune_dead_sessions
│   └── migrate.rs                     synthesize_from_legacy + delete_legacy_files
├── session/                           (NEW; LEAF — does not import state_file)
│   ├── mod.rs                         pub use; cache-file naming convention
│   ├── ppid.rs                        Ppid trait + RealPpid (cfg-gated) + StubPpid
│   └── cache.rs                       resolve_session_id, release_session_id, cache_matches,
│                                      project_hash, cache_file_path
├── commands/agent/task/
│   ├── resume.rs                      (NEW) task_resume
│   ├── discard.rs                     (NEW) task_discard, --force, template-diff guard
│   ├── new.rs                         MOD: post-reconcile had_other_active filter (C-21)
│   ├── archive.rs                     MOD: rename-first; state_mutate cleanup after (C-22)
│   └── mod.rs                         MOD: pub mod resume; pub mod discard;
├── commands/agent/workspace/
│   └── identity.rs                    MOD: bodies delegate to state_file
├── commands/context/gather.rs         MOD: focused slug via state_file (this session only)
├── commands/agent/task/worktree/
│   ├── discovery.rs                   MOD: enumerate via state.tasks.active per worktree
│   └── list.rs                        MOD: same
├── commands/unload.rs                 MOD: skip set adds state_file/state_lock_file/tmp glob
├── layout.rs                          ADD state_file(), state_lock_file()
├── error.rs                           ADD StateTomlCorrupt, StateLockContended, TaskStillActive
└── lib.rs                             ADD pub use for state_file, session, resume, discard
```

CLI plumbing in `crates/ark-cli/src/`:

```
crates/ark-cli/src/
└── agent_cli.rs                       MOD: add Resume, Discard subcommands;
                                       MOD: resolve_slug body delegates to state_file;
                                       MOD: dispatch constructs RealPpid once at startup
```

Templates and Cargo manifest changes are unchanged from PLAN 01.

Module coupling (revised per R-001):

```
state_file → io::PathExt, io::hash_bytes, layout, error, session::cache (cache_matches predicate)
session    → io::PathExt, io::hash_bytes, layout, error
                                     (LEAF: does NOT import state_file)
commands/agent/task/{new,archive,resume,discard} → state_file, session, state (existing TaskToml)
commands/agent/workspace/identity → state_file
commands/context/gather → state_file, session (this session's focus)
commands/agent/task/worktree/{discovery,list} → state_file
commands/unload → state_file (path constants only)
```

The dependency direction is one-way: `state_file → session`. This matches the public API: `state_file::reconcile::prune_dead_sessions(layout, &mut StateFile)` calls `session::cache::cache_matches(layout, pid, uuid) -> bool` per session entry.

Call graph: `task new --slug a --tier quick` (revised per R-002, C-21):

```
task_new(opts)
  ├── validate_slug(&opts.slug)
  ├── if task_dir.exists() → Error::TaskAlreadyExists
  ├── task_dir.ensure_dir()
  ├── copy_template("PRD", &task_dir.join("PRD.md"))
  ├── build_task_toml(&opts).save(&task_dir)
  └── state_mutate(&layout, &ppid, |state| {
        // After two-way reconcile, `state.tasks.active` may already include opts.slug
        // (because the dir we just created is now visible to reconcile's add-pass).
        // The "non-empty before this command" check must filter it out.
        let had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug);
        if had_other_active {
            eprintln!("warn: {n} active task(s); see `ark agent task resume`");
        }
        if !state.tasks.active.contains(&opts.slug) {
            state.tasks.active.push(opts.slug.clone());     // dedup belt-and-braces
        }
        let id = resolve_session_id(&layout, &ppid)?;
        let pid = ppid.parent_id();
        state.sessions.insert(id.0, Session { focus: opts.slug.clone(), pid });
        Ok(())
      })?
```

Call graph: `task archive --slug a` (reverted per R-003, C-22; rename-first):

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── task_dir.rename_to(&archive_path)?              // (1) rename FIRST
  ├── toml.phase = Archived; toml.archived_at = now;
  │     toml.save(&archive_path)                      // (2) save Archived metadata
  ├── if tier == Deep:                                // (3) SPEC promotion (unchanged from current)
  │     ├── spec_extract(SpecExtractOptions { task_dir_override: Some(&archive_path), ... })
  │     └── spec_register(SpecRegisterOptions { ... })
  ├── state_mutate(&layout, &ppid, |state| {           // (4) state cleanup
  │     state.tasks.active.retain(|s| s != &opts.slug);
  │     for sess in state.sessions.values_mut() { /* clear if focus matches */ }
  │     let id = resolve_session_id(&layout, &ppid)?;
  │     if state.sessions.get(&id.0).map(|s| &s.focus) == Some(&opts.slug) {
  │         state.sessions.remove(&id.0);
  │         release_session_id(&layout, &id)?;
  │     }
  │     Ok(())
  │   })?
  └── record_task(...)?                               // (5) workspace bridge (unchanged)
```

Failure recovery for step (4): if state_mutate fails after (1)-(3) succeed, the next `load_state` runs reconcile: (a) add-pass enumerates `.ark/tasks/` (excluding `archive/`), so the archived task is NOT re-added; (b) drop-pass removes the slug from any stale `state.tasks.active` entry because no `tasks/<slug>/task.toml` exists anymore. State integrity is restored. SPEC files and INDEX.md correctly reference an archived task. No corruption.

Call graph: `task worktree list` (unchanged from PLAN 01):

```
worktree_list()
  ├── git_worktree_list_porcelain()
  └── for each <wt> under <root>/<cfg.worktree_dir>/:
        ├── wt_layout = Layout::new(&wt)
        ├── state = state_file::load_state(&wt_layout, &ppid)?     // includes reconcile + prune
        └── for slug in &state.tasks.active:
              ├── task_dir = wt_layout.task_dir(slug)
              ├── toml = TaskToml::load(&task_dir)?
              └── emit row
```

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

/// Stateless predicate. Returns true iff the cache file at
/// `cache_file_path(layout, pid)` exists AND its trimmed contents
/// equal `uuid`. Used by `state_file::reconcile::prune_dead_sessions`
/// to judge each `[sessions.*]` entry's liveness.
pub fn cache_matches(layout: &Layout, pid: u32, uuid: &str) -> bool;
```

```rust
// crates/ark-core/src/session/ppid.rs

/// Source of parent-id for the current process. Trait-shaped to allow
/// deterministic test injection. Production code constructs `RealPpid`
/// once at CLI startup; tests pass `StubPpid(u32)`.
pub trait Ppid {
    fn parent_id(&self) -> u32;
}

/// Production implementation. Unix delegates to
/// `std::os::unix::process::parent_id`. Windows walks the toolhelp
/// snapshot to find the parent of `GetCurrentProcessId()`. Returns
/// the calling process's own PID on Windows toolhelp failure
/// (per C-20).
#[derive(Debug, Default, Clone, Copy)]
pub struct RealPpid;

impl Ppid for RealPpid {
    fn parent_id(&self) -> u32 { /* cfg-gated body */ }
}

/// Test stub. `StubPpid(12345).parent_id() == 12345`.
#[derive(Debug, Clone, Copy)]
pub struct StubPpid(pub u32);

impl Ppid for StubPpid {
    fn parent_id(&self) -> u32 { self.0 }
}
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
pub use state_file::{
    Identity, Session, StateFile,
    load_state, state_mutate,
    reconcile::{prune_dead_sessions, reconcile_against_disk},
};
pub use session::{
    SessionId,
    cache::{cache_file_path, cache_matches, release_session_id, resolve_session_id},
    ppid::{Ppid, RealPpid, StubPpid},
};
pub use commands::agent::task::{
    TaskDiscardOptions, TaskDiscardSummary,
    TaskResumeOptions, TaskResumeSummary,
    task_discard, task_resume,
};
```

Core public functions (signatures revised to thread `Ppid` through):

```rust
// state_file/io.rs
pub fn load_state(layout: &Layout, ppid: &dyn Ppid) -> Result<StateFile>;
pub fn state_mutate<F>(layout: &Layout, ppid: &dyn Ppid, edit: F) -> Result<()>
where F: FnOnce(&mut StateFile) -> Result<()>;

// state_file/reconcile.rs
/// Two-way: enumerate task dirs, add missing actives, drop stale, prune dead sessions.
/// Order: add-pass → drop-pass → orphan-focus drop → prune_dead_sessions.
pub fn reconcile_against_disk(layout: &Layout, state: &mut StateFile) -> Result<()>;

/// Prune `[sessions.*]` entries whose cache file is missing or mismatched.
/// Calls `session::cache::cache_matches` per entry.
pub fn prune_dead_sessions(layout: &Layout, state: &mut StateFile);

// session/cache.rs
pub fn resolve_session_id(layout: &Layout, ppid: &dyn Ppid) -> Result<SessionId>;
pub fn release_session_id(layout: &Layout, id: &SessionId) -> Result<()>;
pub fn cache_matches(layout: &Layout, pid: u32, uuid: &str) -> bool;

// commands/agent/task/resume.rs
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary>;
//   - constructs RealPpid internally (or accepts via opts in a future revision; not this task)

// commands/agent/task/discard.rs
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary>;
//   - same
```

The two task-op entry points (`task_resume`, `task_discard`, plus existing `task_new`/`task_archive`) construct `RealPpid` internally as a default. A follow-up could thread a `Ppid` argument through `*Options` for advanced testing, but that is not required for this task — integration tests construct `RealPpid` and rely on the OS PPID being stable within one test process; unit tests in `state_file::*` and `session::cache::*` exercise the Ppid trait directly via `StubPpid`.

Identity API and CLI subcommand additions are unchanged from PLAN 01.

[**Constraints**]

- C-1: Locking primitive is stdlib `File::try_lock` (stable Rust 1.89+). Cross-platform parent-id via the `Ppid` trait + `RealPpid`'s `cfg`-gated shim. New ark-core deps: `uuid` (any host), `windows-sys` under `[target.'cfg(windows)'.dependencies]` only.
- C-2: Lock file is `.ark/.state.toml.lock`. Backoff: 5 attempts at 10/20/40/80/160 ms (≤ 320 ms cumulative).
- C-3: State file write is atomic: `.state.toml.tmp.<pid>` then `rename_to(.state.toml)`.
- C-4: `state_mutate` unlinks `.state.toml.tmp.*` orphans on lock acquire.
- C-5: Cache file naming: `<temp_dir>/ark-session-<project_hash>-<ppid>.id`. PPID source is the `Ppid` trait passed at the call site.
- C-6: No `.unwrap()` in production code. Single allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`.
- C-7: All filesystem access in `state_file/`, `session/`, and the new task ops routes through `io::PathExt`.
- C-8: All `.ark/`-relative paths route through `Layout` helpers. New constants: `STATE_FILE`, `STATE_LOCK_FILE`. New accessors: `Layout::state_file()`, `Layout::state_lock_file()`. Legacy `tasks_current()` / `developer_file()` accessors kept for migration.
- C-9: Reconcile drops a `tasks.active` entry when the corresponding `task.toml` is missing OR `phase == Archived`. Drops a `sessions.*` entry when its slug-focus is no longer in `tasks.active` after the active reconcile.
- C-10: Session pruning (in `state_file::reconcile::prune_dead_sessions`) drops a `sessions.*` entry when `cache_matches(layout, session.pid, session_id) == false`.
- C-11: A read (`load_state`) does NOT delete legacy files. Migration's delete step happens only on the next successful `state_mutate` save.
- C-12: `state_mutate` is the sole path that mutates the state file.
- C-13: `task_discard`'s template-diff guard reads each seeded file and compares against the embedded template; first divergence → `Error::TaskStillActive`. `--force` skips the scan.
- C-14: Legacy accessors carry doc-comment "legacy migration accessor; remove after migration window."
- C-15: `[tasks].active` is sorted+deduped on every save.
- C-16: No SessionStart hook integration.
- C-17: Each worktree's state file is independent.
- C-18: `unload.rs` skip set in BOTH walk sites: `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file(), layout.developer_file()]`. `walk_files_excluding` extended to skip `.state.toml.tmp.*` orphans under `<root>/.ark/`.
- C-19: `state_file::reconcile::reconcile_against_disk` runs in this order: (1) enumerate `.ark/tasks/<slug>/task.toml` excluding `archive/`; for each found slug whose `phase != Archived`, push to `state.tasks.active` if not already present; (2) drop `state.tasks.active` entries whose `task.toml` is missing or `phase == Archived`; (3) sort+dedup active per C-15; (4) drop `state.sessions.*` entries whose `focus` is no longer in active; (5) `prune_dead_sessions(layout, state)` removes sessions whose cache file is missing/mismatched. Order matters: add must precede drop so a brief inconsistent state never collapses an active slug; prune-sessions must come last so newly-inactive sessions (from step 4) are pruned in the same pass.
- C-20: `RealPpid::parent_id()` on Windows returns the calling process's own PID (`std::process::id()`) when toolhelp snapshot creation or walk fails. Unix path has no failure mode.
- C-21 (NEW per R-002): `task_new`'s `state_mutate` closure computes `had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug)` AFTER reconcile (which may have added the just-created slug). Warn iff `had_other_active`. The push is guarded: `if !state.tasks.active.contains(&opts.slug) { state.tasks.active.push(opts.slug.clone()) }`. C-15's sort+dedup is the belt-and-braces second line; C-21's contains-check is the user-visible-correctness first line.
- C-22 (NEW per R-003): `task_archive` ordering is rename-first: (1) `task_dir.rename_to(archive_path)`; (2) save `phase = Archived` to `archive_path`; (3) deep-tier `spec_extract + spec_register` operate on `archive_path`; (4) `state_mutate` cleanup (drop slug from active, clear focuses, release own session); (5) `record_task`. Steps (1)-(3) preserve current archive code's invariant that promoted SPEC files reference an archived task. Step (4) failure is recoverable via two-way reconcile on next `load_state`: add-pass excludes `archive/` so the slug is not re-added; drop-pass removes any stale active entry. SPEC integrity is durable; state integrity is recoverable.

---

## Runtime

[**Main Flow**]

Scenario A — `task new --slug a --tier quick` (single session, no prior active tasks):

1. CLI dispatch → constructs `RealPpid` → `task_new(opts, &ppid)`.
2. Validate slug; check for collision at `.ark/tasks/a/`.
3. Scaffold task dir (PRD.md, task.toml).
4. Open `state_mutate(&layout, &ppid, |state| { ... })`:
   - Acquire `.ark/.state.toml.lock` exclusive (try_lock; backoff if contended).
   - Unlink any `.state.toml.tmp.*` orphans.
   - Read `.state.toml` (or synthesize from legacy if absent).
   - `reconcile_against_disk`: enumerate `.ark/tasks/`; the just-created `a` is added to `state.tasks.active`. Drop stale entries; orphan-focus drop; prune dead sessions.
   - Closure runs: `had_other_active = state.tasks.active.iter().any(|s| s != "a")` → false (only "a" present). **No warning.** Slug `a` already in active → contains-check skips push. Insert/update `sessions.<self> { focus: "a", pid: ppid.parent_id() }`.
   - Serialize state to `.state.toml.tmp.<pid>`. `rename_to(.state.toml)`.
   - Unlink legacy `.developer` and `tasks/.current` if present.
   - Lock guard drops; OS releases.
5. Print summary.

Scenario B — `task new --slug b` from a second shell, same `.ark/`, with `a` already active:

Identical to Scenario A, except step 4's reconcile sees `active = ["a", "b"]` (both dirs exist on disk), and the closure's `had_other_active` filter sees `"a"` ≠ `"b"` → true. **Warning emits.** Slug `b` already in active (from add-pass) → contains-check skips push. Two `sessions.<id>` entries with distinct PPIDs and distinct focuses.

Scenario C — `task execute` with no `--slug`:

1. CLI dispatch → `RealPpid` → `resolve_slug(root, None, &ppid)`.
2. `resolve_slug_via_state`:
   - `let id = resolve_session_id(&layout, &ppid)?;`
   - `let state = load_state(&layout, &ppid)?;` (includes reconcile + prune)
   - `state.sessions.get(&id.0).map(|s| s.focus.clone()).ok_or(NoCurrentTask { path: layout.state_file() })`
3. Phase transition runs against the resolved slug.

Scenario D — `task archive --slug a` (this session focused on `a`, deep tier):

1. Load TaskToml; check_transition.
2. `task_dir.rename_to(archive_path)` — atomic move.
3. `toml.phase = Archived; toml.save(&archive_path)`.
4. Deep-tier: `spec_extract(task_dir_override = Some(&archive_path))`; `spec_register(...)`.
5. `state_mutate(&layout, &ppid, |state| { drop a from active; clear focuses; if mine, release_session_id })`.
6. `record_task(...)` (workspace journal).

Scenario E — `task worktree list` from the parent shell:

1. `git worktree list --porcelain` enumerates worktrees.
2. For each worktree under `.ark/worktrees/`:
   - `wt_layout = Layout::new(<wt_path>)`.
   - `state = load_state(&wt_layout, &ppid)?` (reconcile populates active from disk).
   - For each `slug` in `state.tasks.active`: load `task.toml`; emit row.

[**Failure Flow**]

- **Lock contention beyond backoff**: `Error::StateLockContended { path }`.
- **Corrupt state file**: `Error::StateTomlCorrupt { path, source }`. Recovery: hand-delete; next mutation re-synthesizes (or two-way reconcile rebuilds active from disk).
- **Crash mid-write**: rename atomicity preserves either pre- or post-write state. `.state.toml.tmp.<pid>` orphan reaped by next `state_mutate` (C-4).
- **`task discard` on archived task**: `Error::TaskNotFound { slug }`.
- **`task discard` without --force, PRD edited**: `Error::TaskStillActive { slug, file: "PRD.md" }`.
- **`task resume <bogus>`**: `Error::TaskNotFound { slug }`.
- **`task archive` rename fails**: archive aborts at step 2 (rename returned error). State and SPEC are untouched. User retries.
- **`task archive` SPEC promotion fails post-rename**: task dir is at `archive_path` with `phase = Archived` saved (step 3 succeeded); SPEC files may be partially written. User runs `ark agent spec extract` / `register` manually to complete promotion. State `active` still contains slug; next `load_state` reconcile drops it (no `tasks/<slug>/`). State self-heals; SPEC requires manual completion (matches current behavior per `commands/agent/task/archive.rs:113-117`).
- **`task archive` state_mutate cleanup fails post-SPEC-promotion**: archive succeeded (rename + SPEC), state `active` still contains stale slug. Next `load_state` reconcile drops it (no `tasks/<slug>/`). User-visible: nothing, state self-heals on next read.
- **GC drops own session by mistake**: cannot happen — `resolve_session_id` recreates the cache file before any state read inside `state_mutate`.
- **PPID recycling on Unix**: rare; mitigated by cache-file deletion on archive/discard.
- **Windows toolhelp snapshot fails**: `RealPpid::parent_id()` returns own PID per C-20. Documented limitation; tests cover via dedicated stub.
- **`unload` after migration**: `.developer` no longer exists; skip-set entry is a no-op. `.state.toml` skipped per C-18. Snapshot captures `.ark/tasks/<*>/`; `load` reconstructs `.state.toml` via two-way reconcile on first read.

[**State Transitions**]

```
[no .state.toml, no legacy]      ──load_state──>      reconcile-from-disk → may be non-empty if task dirs exist
[no .state.toml, has legacy]     ──load_state──>      synthesize from legacy → reconcile-from-disk (in-memory only)
[has .state.toml]                ──load_state──>      parse → reconcile-from-disk → prune sessions
[any of above]                   ──state_mutate──>    closure on reconciled state
[state_mutate save success]      ──finalize──>        delete legacy files if present
[unload + load round-trip]       ──first load_state──> reconcile-from-disk repopulates active

session lifecycle:
  unregistered ──task new / task resume──> registered with focus
  registered  ──task archive (own slug)──> released (entry dropped, cache deleted)
  registered  ──task discard (own slug)──> released
  registered  ──cache file disappears──> dropped by prune_dead_sessions on next load_state

archive ordering (rename-first per C-22):
  Active task → rename → Archived metadata saved → SPEC promoted → state cleared → journal recorded
```

---

## Implementation

[**Phase 1 — Foundations (no behavior change for users)**]

- `crates/ark-core/src/session/ppid.rs` — implement `Ppid` trait + `RealPpid` (`cfg`-gated body) + `StubPpid`. Unit test `parent_id_via_real_is_nonzero` (smoke) + `stub_ppid_returns_held_value`.
- `crates/ark-core/src/session/cache.rs` — `resolve_session_id`, `release_session_id`, `cache_matches`, `cache_file_path`, `project_hash`. Functions accept `&dyn Ppid` where they need parent-id.
- `crates/ark-core/src/state_file/{mod,model,io,reconcile,migrate}.rs` — `reconcile.rs` implements `reconcile_against_disk` per C-19 (add-then-drop-then-orphan-then-prune-sessions). `prune_dead_sessions` calls `session::cache::cache_matches`.
- `crates/ark-core/src/error.rs` — add `StateTomlCorrupt`, `StateLockContended`, `TaskStillActive`.
- `crates/ark-core/src/layout.rs` — add `STATE_FILE`/`STATE_LOCK_FILE` constants and `state_file()`/`state_lock_file()` accessors.
- `crates/ark-core/src/lib.rs` — `pub mod state_file; pub mod session;` plus the new re-exports per API Surface.
- `crates/ark-core/Cargo.toml` — add `uuid = { version = "1", features = ["v4"] }`. Add `windows-sys` under `[target.'cfg(windows)'.dependencies]`. Pin `rust-version = "1.89"` if not already pinned.

Unit tests in this phase:

- `state_file::io::tests` — load default + legacy + pure-read + atomic write + legacy cleanup + orphan-tmp unlink + lock-contended + concurrent serialize.
- `state_file::reconcile::tests` — drops_missing/archived/orphan-session; **adds_missing_active_from_disk** (V-UT-24); **recovers_active_from_disk_when_state_file_deleted** (V-UT-25); **archive_rename_failure_recovery_via_reconcile** (V-UT-26); **add_then_drop_order_is_idempotent** (V-UT-27); **prune_dead_sessions_runs_after_orphan_drop** (V-UT-33 — NEW per C-19 step ordering).
- `state_file::migrate::tests` — synthesize variants.
- `session::ppid::tests` — `real_ppid_returns_nonzero_on_current_platform` (V-UT-12); `stub_ppid_returns_held_value` (V-UT-32).
- `session::cache::tests` — `resolve_round_trips_within_same_ppid_via_stub` (V-UT-13); `cache_file_path_uses_project_hash_and_ppid` (V-UT-14); `release_removes_cache_file_idempotently`; `cache_matches_returns_true_for_matching_uuid` (V-UT-15-replacement); `cache_matches_returns_false_when_file_missing_or_mismatched` (V-UT-16-replacement).

[**Phase 2 — Rewire Callers (behavior preserved, internals switched)**]

- `commands/agent/workspace/identity.rs` — replace bodies with `state_file` delegations. Construct `RealPpid` internally (identity reads/writes don't expose Ppid in their public signatures).
- `commands/agent/workspace/init.rs:78` — unchanged.
- `commands/agent/task/new.rs:135-137, 262` — replace direct `tasks_current().write_bytes(...)` with `state_mutate` closure per Scenario A. Apply C-21's `had_other_active` filter and contains-check guard.
- `commands/agent/task/archive.rs` — preserve current rename-first ordering. Insert `state_mutate` cleanup AFTER existing SPEC-promotion block (around line 134) and BEFORE the existing `.current` removal block (lines 136-141 — entire block is now a `state_mutate` closure that drops slug, clears focuses, releases own session). The existing `record_task(...)` call remains last.
- `commands/context/gather.rs:314` — replace `.current` read with `resolve_session_id` + `load_state` + this-session-focus lookup.
- `commands/agent/task/worktree/discovery.rs:94, list.rs:87` — replace `.current` reads with `state.tasks.active` enumeration per Scenario E.
- `commands/unload.rs:87, 171` — extend skip set per C-18.
- `crates/ark-cli/src/agent_cli.rs:384-394` — `resolve_slug` constructs `RealPpid` and calls `resolve_slug_via_state`. Function signature unchanged.

Tests adjusted in this phase:

- `agent_lifecycle.rs:48`, `archive.rs:218` — `!.current.exists()` → `!active.contains(&slug)`.
- `init.rs:580, 590` — `.developer` existence → `state.identity.is_some()` and `state.identity.unwrap().name == "alice"`.
- `context/gather.rs:490, 524` — mock `.current` writes → `state_mutate` calls in test setup.
- `unload.rs::tests::unload_excludes_worktree_contents` — extend to also assert `.state.toml` excluded (V-IT-7 base).

[**Phase 3 — New Ops + CLI Surface + Docs**]

- `commands/agent/task/resume.rs` and `discard.rs` — implement per signatures above.
- `commands/agent/task/mod.rs` — `pub mod resume; pub mod discard;` and re-exports.
- `crates/ark-cli/src/agent_cli.rs`: add `Resume(TaskSlugArgs)` and `Discard(TaskDiscardCliArgs)` variants; dispatch arms call the new functions; pass `RealPpid` through.
- `crates/ark-core/src/lib.rs` — add the new pub-uses.
- `templates/ark/.gitignore` — add `.state.toml`, `.state.toml.lock`, `.state.toml.tmp.*` lines.
- `templates/ark/workflow.md` — brief subsection under §6 Mechanics: multi-session focus, `task resume`, `task discard`, `.state.toml` per-checkout per-worktree, gitignored, skipped by unload.
- `.claude/commands/ark/quick.md` and `design.md` — minor updates: warn-on-other-active behavior; quick-tier `--worktree` opt-in.

Integration tests in `crates/ark-cli/tests/`:

- New file `agent_session.rs`:
  - `multi_session_focus_isolation` (V-IT-1) — uses `StubPpid(12345)` and `StubPpid(67890)` to drive two simulated sessions.
  - `gc_drops_dead_session_on_load` (V-IT-2-old → renamed; covered by V-UT testing of `prune_dead_sessions`).
  - `lock_contention_succeeds_after_backoff` (V-IT-3).
  - `migration_synthesizes_from_legacy_and_deletes` (V-IT-4).
  - `discard_force_removes_edited_task` (V-IT-5).
  - `resume_invalid_slug_errors`.
  - `worktree_isolation_state_file` (V-IT-6).
  - `unload_excludes_state_file` (V-IT-7).
  - `worktree_list_works_after_session_cache_loss` (V-IT-8).
  - `first_task_emits_no_warning` (V-IT-2a — NEW per R-002).
  - `second_distinct_task_warns` (V-IT-2b — NEW per R-002).
  - `deep_archive_state_cleanup_failure_recovers_via_reconcile` (V-IT-9 — NEW per R-003).
- Extensions to `agent_lifecycle.rs`: post-archive assertion update.

---

## Trade-offs

- T-1: Cross-platform parent-id source. Choice: bespoke `windows-sys` shim (vs. `sysinfo`).
- T-2: One `.state.toml` vs. two files. Choice: one file.
- T-3 (REVISED per R-003): **Archive ordering — rename-first vs. state_mutate-first.** PLAN 01 chose state_mutate-first based on "active set always reflects what's in `tasks/`" being a stronger invariant. REVIEW 01 correctly noted that this invariant cannot extend to SPEC promotion: deep-tier extract/register write to `specs/features/<slug>/SPEC.md` and upsert `specs/features/INDEX.md`, neither of which is recoverable by reconcile if the rename later fails. **Choice (revised): rename-first.** SPEC promotion is durable post-rename; state cleanup happens after, and is recoverable via two-way reconcile if it fails. Two-way reconcile (PLAN 01's R-002 fix) makes state recovery cheap; SPEC recovery would require its own state machine and is out of scope.
- T-4: Discard guard — template-diff vs. always-`--force`. Choice: template-diff.
- T-5: Project-hash length — 16 hex chars vs. full 64. Choice: 16.
- T-6: Deduplication of `tasks.active` — Vec+sort/dedup. Choice: Vec+sort/dedup; C-21 adds a contains-check guard at the push site as the first line of correctness.
- T-7: Identity API preservation. Choice: preserve names, swap bodies.
- T-8: Windows `parent_id()` failure — degrade vs. abort. Choice: degrade.
- T-9 (NEW per R-004): **Session-provider test seam — trait+stub vs. thread-local override.** A `thread_local!` mutable global (e.g. `OVERRIDE_PPID: Cell<Option<u32>>`) would avoid changing function signatures, but introduces hidden state, complicates concurrent-test parallelism, and conflicts with NG-12. The trait approach (`Ppid` + `RealPpid` + `StubPpid`) requires threading `&dyn Ppid` through `load_state` / `state_mutate` / `resolve_session_id` — a one-line change to each signature, plus one `RealPpid::default()` construction at each CLI dispatch site. **Choice: trait+stub.** Reasoning: explicit dependency injection matches Rust's dependency-via-trait idiom; threads cleanly through the API; avoids global state; aligns with NG-12. Cost: ~10 sites take a `&dyn Ppid` argument.

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
- V-UT-12: `real_ppid_returns_nonzero_on_current_platform` — cross-platform smoke; runs on whichever CI OS executes.
- V-UT-13: `session_resolve_round_trips_within_same_ppid_via_stub` — uses `StubPpid(12345)`; second call returns the same UUID.
- V-UT-14: `session_cache_file_path_uses_project_hash_and_ppid`.
- V-UT-15: `cache_matches_returns_true_for_matching_uuid` (replaces PLAN 01's `gc_drops_session_with_missing_cache_file`).
- V-UT-16: `cache_matches_returns_false_when_file_missing_or_mismatched` (replaces PLAN 01's `gc_drops_session_with_mismatched_uuid`).
- V-UT-17: `prune_dead_sessions_keeps_session_with_matching_cache`.
- V-UT-18: `task_resume_invalid_slug_returns_task_not_found`.
- V-UT-19: `task_resume_sets_session_focus_idempotently`.
- V-UT-20: `task_discard_template_unchanged_succeeds_without_force`.
- V-UT-21: `task_discard_edited_prd_refuses_without_force`.
- V-UT-22: `task_discard_with_force_removes_edited_task`.
- V-UT-23: `task_discard_archived_task_returns_task_not_found`.
- V-UT-24: `reconcile_adds_missing_active_from_disk`.
- V-UT-25: `recovers_active_from_disk_when_state_file_deleted`.
- V-UT-26: `archive_rename_failure_recovery_via_reconcile`.
- V-UT-27: `add_then_drop_order_is_idempotent`.
- V-UT-28: `unload_skip_set_includes_state_file_and_lock`.
- V-UT-29: `walk_files_excluding_skips_state_toml_tmp_orphans`.
- V-UT-30 (NEW per R-002): `task_new_had_other_active_false_when_only_self` — pre-seed empty state; create task dir; reconcile populates `active = ["self"]`; assert closure-internal `had_other_active == false`.
- V-UT-31 (NEW per R-002): `task_new_had_other_active_true_when_other_present` — pre-seed `active = ["other"]`; create new task dir; reconcile yields `active = ["other", "self"]`; assert `had_other_active == true`.
- V-UT-32 (NEW per R-004): `stub_ppid_returns_held_value` — `StubPpid(12345).parent_id() == 12345`; `StubPpid(67890).parent_id() == 67890`.
- V-UT-33 (NEW per C-19 ordering): `prune_dead_sessions_runs_after_orphan_drop` — pre-seed session whose focus is missing AND whose cache is dead; assert single reconcile pass drops both correctly.

[**Integration Tests**]

- V-IT-1: `multi_session_focus_isolation_across_two_sessions` — two distinct `StubPpid` values drive two simulated sessions; each `--slug`-less command resolves to its own focus. **Now executable deterministically** thanks to `Ppid` trait.
- V-IT-2a (NEW per R-002): `first_task_emits_no_warning` — fresh `.ark/`; `task new --slug a`; assert stderr does NOT contain "active task".
- V-IT-2b (NEW per R-002): `second_distinct_task_warns` — after V-IT-2a's setup, `task new --slug b`; assert stderr contains "active task".
- V-IT-3: `migration_e2e_via_task_new`.
- V-IT-4: `worktree_isolation_state_file`.
- V-IT-5: `archive_clears_focus_and_releases_cache`.
- V-IT-6: `lock_contention_resolves_within_backoff_window`.
- V-IT-7: `unload_excludes_state_file`.
- V-IT-8: `worktree_list_works_after_session_cache_loss`.
- V-IT-9 (NEW per R-003): `deep_archive_state_cleanup_failure_recovers_via_reconcile` — create deep task; archive it but inject failure into the post-SPEC `state_mutate` (e.g. by holding the lock past backoff in a sibling thread); assert (a) archive returns `Error::StateLockContended`, (b) `archive_path/task.toml.phase == Archived` (durable), (c) `specs/features/<slug>/SPEC.md` exists and `INDEX.md` row present (durable), (d) on next `load_state`, reconcile drops the stale active entry, (e) re-running `task archive` is a no-op or returns "already archived."

[**Failure / Robustness Validation**]

- V-F-1: `crash_mid_write_leaves_either_old_or_new_state_never_partial`.
- V-F-2: `lock_contention_resolves_within_backoff_window`.
- V-F-3: `lock_contention_beyond_backoff_returns_state_lock_contended`.
- V-F-4: `archive_rename_failure_state_says_inactive_dir_remains_until_reconcile` — covered by V-UT-26.
- V-F-5 (NEW per C-20, refined per R-004): `windows_parent_id_failure_falls_back_to_own_pid` — Windows-only test using a `WindowsToolhelpFailure` stub variant of `Ppid` that simulates the failure path; verify `parent_id()` returns `std::process::id()` and CLI does not abort. (If the stub is in `cfg(test)` only, the production `RealPpid` keeps its actual cfg-gated body unchanged.)

[**Edge Case Validation**]

- V-E-1: `state_mutate_with_concurrent_load_state_does_not_corrupt`.
- V-E-2: `empty_active_set_serializes_to_omitted_field_per_skip_serializing_if`.
- V-E-3: `dedup_preserves_first_occurrence_order_after_sort`.
- V-E-4: `prune_handles_session_with_pid_that_belongs_to_unrelated_process`.
- V-E-5: `discard_with_no_seeded_files_present_succeeds_without_force`.
- V-E-6: `reconcile_with_archive_subdir_does_not_double_count` — verify enumeration excludes `.ark/tasks/archive/` correctly.
- V-E-7: `unload_load_round_trip_preserves_active_via_reconcile_not_capture`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1               | V-UT-1, V-UT-4, V-UT-5, V-UT-25 |
| G-2               | V-IT-1, V-IT-4, V-UT-13 |
| G-3               | V-UT-18..V-UT-23 |
| G-4               | (existing identity tests still pass) |
| G-5               | V-UT-2, V-UT-3, V-UT-5, V-IT-3 |
| G-6               | V-UT-4, V-UT-6, V-UT-7, V-F-1, V-F-2, V-F-3 |
| G-7               | V-UT-15, V-UT-16, V-UT-17, V-UT-33 |
| G-8               | V-UT-30, V-UT-31, V-IT-2a, V-IT-2b |
| G-9               | V-IT-4, V-IT-8 |
| G-10              | V-UT-18, dispatch test in `agent_lifecycle` |
| G-11              | V-UT-20..V-UT-23 |
| G-12              | V-IT-7, V-E-7, V-UT-28, V-UT-29 |
| G-13              | V-UT-12, V-UT-32, V-F-5 |
| G-14              | V-IT-9 |
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
| C-19              | V-UT-24..V-UT-27, V-UT-33, V-E-6 |
| C-20              | V-F-5 |
| C-21              | V-UT-30, V-UT-31, V-IT-2a, V-IT-2b |
| C-22              | V-IT-9, V-UT-26 |
