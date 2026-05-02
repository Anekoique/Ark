
[**Goals**]

- G-1: A single per-checkout `.ark/.state.toml` file carries the active-task set and a per-session focus map. Truth still lives in `.ark/tasks/<slug>/task.toml`; the state file is an index reconciled (two-way: add + drop + prune-sessions) on every read.
- G-2: Concurrent CLI invocations from independent shells in the same `.ark/` each get an independent `[sessions.<uuid>]` entry and an independent focused task. No session can clobber another's focus.
- G-3: Two new agent ops — `ark agent task resume <slug>` and `ark agent task discard <slug>` — extend the agent task verb set (extends `ark-agent-namespace` SPEC G-3).
- G-4: Legacy `.ark/tasks/.current` is auto-migrated on the first state-file mutation and then deleted. The reader tolerates either layout indefinitely so a pure read in a not-yet-migrated install does not change the on-disk state.
- G-5: State-file mutations are atomic across crash *and* across concurrent CLI invocations on Linux, macOS, and Windows. Crash mid-write leaves no partial `.state.toml`. Concurrent writers serialize via an OS-level file lock with bounded backoff.
- G-6: Dead sessions are pruned transparently on the next state read (in-memory; persisted on the next mutation). A session is "dead" when its temp-dir cache file is missing or mismatched. Pruning lives in `state_file::reconcile`, called via `session::cache::cache_matches` predicate.
- G-7: `task new` warns (does not refuse) when there are other active tasks before the new one is appended. The warn check excludes the just-created slug (added to `active` by the prior two-way reconcile pass). First task is silent; second distinct task warns.
- G-8: Each git worktree's `.ark/` owns its own `.state.toml`. `task new --worktree` writes only to the worktree's state file. `task worktree list` and `task worktree cleanup` enumerate via each worktree's `state.tasks.active`, **not** via session focus.
- G-9: `--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask`.
- G-10: `task discard <slug>` refuses without `--force` when seeded files (PRD/PLAN/etc.) differ from their templates. Always refuses if the task is already archived.
- G-11: `.ark/.state.toml`, `.ark/.state.toml.lock`, and any `.ark/.state.toml.tmp.*` orphan files are skipped by `unload.rs` in both walk sites. Active task state is recoverable on `load` via two-way reconcile.
- G-12: Cross-platform parent-id via the `Ppid` trait. Production `RealPpid` delegates to a `cfg`-gated shim: Unix uses `std::os::unix::process::parent_id`; Windows uses `windows-sys`'s `CreateToolhelp32Snapshot` + `Process32FirstW`/`Process32NextW`. Failure on Windows returns the calling process's own PID (`std::process::id()`). Tests use `StubPpid(u32)` for deterministic injection.
- G-13: Deep-tier archive remains rename-first to preserve SPEC promotion durability. `state_mutate` cleanup runs AFTER rename and after SPEC promotion. If cleanup fails, two-way reconcile recovers state-file integrity on next read; SPEC files stay correct (the slug they reference IS archived).

- NG-1: Workflow-phase model changes — ROADMAP item #2.
- NG-2: SessionStart hook integration.
- NG-3: Explicit `session end` / `session list` ops.
- NG-4: Cross-host coordination (NFS-shared `.ark/`).
- NG-5: Heartbeats / `last_seen` timestamps.
- NG-6: Multi-task focus per session.
- NG-7: Capturing `.state.toml` into `.ark.db` snapshots.
- NG-8: `ark upgrade` migration step.
- NG-9: A pure-Rust Windows process-tree walker without `windows-sys`.
- NG-10: A test-only mutable-global session-provider override (e.g. `thread_local!`). The trait+stub approach (T-9) is preferred.

[**Architecture**]

New module tree under `crates/ark-core/src/`:

```text
crates/ark-core/src/
├── state_file/                        (NEW)
│   ├── mod.rs                         pub use; invariants doc
│   ├── model.rs                       StateFile, Tasks, Session + serde
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

```text
crates/ark-cli/src/
└── agent_cli.rs                       MOD: add Resume, Discard subcommands;
                                       MOD: resolve_slug body delegates to state_file;
                                       MOD: dispatch constructs RealPpid once at startup
```

Templates and Cargo manifest changes are unchanged from PLAN 01.

Module coupling (revised per R-001):

```text
state_file → io::PathExt, io::hash_bytes, layout, error, session::cache (cache_matches predicate)
session    → io::PathExt, io::hash_bytes, layout, error
                                     (LEAF: does NOT import state_file)
commands/agent/task/{new,archive,resume,discard} → state_file, session, state (existing TaskToml)
commands/context/gather → state_file, session (this session's focus)
commands/agent/task/worktree/{discovery,list} → state_file
commands/unload → state_file (path constants only)
```

The dependency direction is one-way: `state_file → session`. This matches the public API: `state_file::reconcile::prune_dead_sessions(layout, &mut StateFile)` calls `session::cache::cache_matches(layout, pid, uuid) -> bool` per session entry.

Call graph: `task new --slug a --tier quick` (revised per R-002, C-21):

```text
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

```text
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
```

Failure recovery for step (4): if state_mutate fails after (1)-(3) succeed, the next `load_state` runs reconcile: (a) add-pass enumerates `.ark/tasks/` (excluding `archive/`), so the archived task is NOT re-added; (b) drop-pass removes the slug from any stale `state.tasks.active` entry because no `tasks/<slug>/task.toml` exists anymore. State integrity is restored. SPEC files and INDEX.md correctly reference an archived task. No corruption.

Call graph: `task worktree list` (unchanged from PLAN 01):

```text
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
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default)]
    pub tasks: Tasks,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sessions: BTreeMap<String, Session>,
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
    Session, StateFile,
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

CLI subcommand additions are unchanged from PLAN 01.

[**Constraints**]

- C-1: Locking primitive is stdlib `File::try_lock` (stable Rust 1.89+). Cross-platform parent-id via the `Ppid` trait + `RealPpid`'s `cfg`-gated shim. New ark-core deps: `uuid` (any host), `windows-sys` under `[target.'cfg(windows)'.dependencies]` only.
- C-2: Lock file is `.ark/.state.toml.lock`. Backoff: 5 attempts at 10/20/40/80/160 ms (≤ 320 ms cumulative).
- C-3: State file write is atomic: `.state.toml.tmp.<pid>` then `rename_to(.state.toml)`.
- C-4: `state_mutate` unlinks `.state.toml.tmp.*` orphans on lock acquire.
- C-5: Cache file naming: `<temp_dir>/ark-session-<project_hash>-<ppid>.id`. PPID source is the `Ppid` trait passed at the call site.
- C-6: No `.unwrap()` in production code. Single allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`.
- C-7: All filesystem access in `state_file/`, `session/`, and the new task ops routes through `io::PathExt`.
- C-8: All `.ark/`-relative paths route through `Layout` helpers. New constants: `STATE_FILE`, `STATE_LOCK_FILE`. New accessors: `Layout::state_file()`, `Layout::state_lock_file()`. The legacy `tasks_current()` accessor is kept for migration.
- C-9: Reconcile drops a `tasks.active` entry when the corresponding `task.toml` is missing OR `phase == Archived`. Drops a `sessions.*` entry when its slug-focus is no longer in `tasks.active` after the active reconcile.
- C-10: Session pruning (in `state_file::reconcile::prune_dead_sessions`) drops a `sessions.*` entry when `cache_matches(layout, session.pid, session_id) == false`.
- C-11: A read (`load_state`) does NOT delete legacy files. Migration's delete step happens only on the next successful `state_mutate` save.
- C-12: `state_mutate` is the sole path that mutates the state file.
- C-13: `task_discard`'s template-diff guard reads each seeded file and compares against the embedded template; first divergence → `Error::TaskStillActive`. `--force` skips the scan.
- C-14: Legacy accessors carry doc-comment "legacy migration accessor; remove after migration window."
- C-15: `[tasks].active` is sorted+deduped on every save.
- C-16: No SessionStart hook integration.
- C-17: Each worktree's state file is independent.
- C-18: `unload.rs` skip set in BOTH walk sites: `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file()]`. `walk_files_excluding` extended to skip `.state.toml.tmp.*` orphans under `<root>/.ark/`.
- C-19: `state_file::reconcile::reconcile_against_disk` runs in this order: (1) enumerate `.ark/tasks/<slug>/task.toml` excluding `archive/`; for each found slug whose `phase != Archived`, push to `state.tasks.active` if not already present; (2) drop `state.tasks.active` entries whose `task.toml` is missing or `phase == Archived`; (3) sort+dedup active per C-15; (4) drop `state.sessions.*` entries whose `focus` is no longer in active; (5) `prune_dead_sessions(layout, state)` removes sessions whose cache file is missing/mismatched. Order matters: add must precede drop so a brief inconsistent state never collapses an active slug; prune-sessions must come last so newly-inactive sessions (from step 4) are pruned in the same pass.
- C-20: `RealPpid::parent_id()` on Windows returns the calling process's own PID (`std::process::id()`) when toolhelp snapshot creation or walk fails. Unix path has no failure mode.
- C-21 (NEW per R-002): `task_new`'s `state_mutate` closure computes `had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug)` AFTER reconcile (which may have added the just-created slug). Warn iff `had_other_active`. The push is guarded: `if !state.tasks.active.contains(&opts.slug) { state.tasks.active.push(opts.slug.clone()) }`. C-15's sort+dedup is the belt-and-braces second line; C-21's contains-check is the user-visible-correctness first line.
- C-22 (NEW per R-003): `task_archive` ordering is rename-first: (1) `task_dir.rename_to(archive_path)`; (2) save `phase = Archived` to `archive_path`; (3) deep-tier `spec_extract + spec_register` operate on `archive_path`; (4) `state_mutate` cleanup (drop slug from active, clear focuses, release own session); (5) `record_task`. Steps (1)-(3) preserve current archive code's invariant that promoted SPEC files reference an archived task. Step (4) failure is recoverable via two-way reconcile on next `load_state`: add-pass excludes `archive/` so the slug is not re-added; drop-pass removes any stale active entry. SPEC integrity is durable; state integrity is recoverable.

---
