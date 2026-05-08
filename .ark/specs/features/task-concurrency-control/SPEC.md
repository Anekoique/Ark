[**Goals**]

- G-1: Per-checkout `.ark/.state.toml` carries the active-task set and a per-session focus map.
- G-2: Concurrent CLI invocations from independent shells each get an independent session entry; no session clobbers another's focus.
- G-3: `ark agent task resume <slug>` and `ark agent task discard <slug>` extend the agent task verb set.
- G-4: Legacy `.ark/tasks/.current` is auto-migrated on first state-file mutation and then deleted.
- G-5: State-file writes are atomic across crash and concurrent writers (file lock + temp+rename).
- G-6: Each git worktree owns its own `.state.toml`; worktree enumeration goes through state, not session focus.

[**Non-goals**]

- NG-1: No SessionStart hook integration in this task.
- NG-2: No cross-host coordination (NFS-shared `.ark/`).
- NG-3: No `.state.toml` capture into `.ark.db` snapshots.

[**Architecture**]

```
crates/ark-core/src/
├── state_file/                       (NEW)
│   ├── mod.rs                        (pub use; invariants doc)
│   ├── model.rs                      (StateFile, Tasks, Session + serde)
│   ├── io.rs                         (load_state, state_mutate; lock acquire/release;
│   │                                   atomic temp+rename write)
│   ├── reconcile.rs                  (two-way: add_missing + drop_stale +
│   │                                   prune_dead_sessions)
│   └── migrate.rs                    (synthesize_from_legacy + delete_legacy_files)
├── session/                          (NEW; LEAF — does NOT import state_file)
│   ├── mod.rs                        (cache-file naming convention)
│   ├── ppid.rs                       (Ppid trait + RealPpid (cfg-gated) + StubPpid)
│   └── cache.rs                      (resolve_session_id, release_session_id,
│                                       cache_matches, project_hash, cache_file_path)
├── commands/agent/task/
│   ├── resume.rs                     (NEW; task_resume)
│   ├── discard.rs                    (NEW; task_discard, --force, template-diff guard)
│   ├── new.rs                        (post-reconcile had_other_active filter — C-19)
│   ├── archive.rs                    (rename-first; state_mutate cleanup after — C-20)
│   └── mod.rs                        (pub mod resume; pub mod discard)
├── commands/context/gather.rs        (focused slug via state_file — this session only)
├── commands/agent/task/worktree/
│   ├── discovery.rs                  (enumerate via state.tasks.active per worktree)
│   └── list.rs                       (same)
├── commands/unload.rs                (skip set adds state_file/state_lock_file/tmp glob)
├── layout.rs                         (state_file(), state_lock_file();
│                                       STATE_FILE, STATE_LOCK_FILE consts)
├── error.rs                          (StateTomlCorrupt, StateLockContended,
│                                       TaskStillActive, NoActiveTask, AmbiguousActiveTask)
└── lib.rs                            (re-exports state_file, session, resume, discard)

crates/ark-cli/src/agent_cli.rs       (Resume/Discard subcommands; resolve_slug delegates
                                        to state_file; dispatch constructs RealPpid once)
```

Module coupling (one-way):

```
state_file → io::PathExt, io::hash_bytes, layout, error, session::cache::cache_matches
session    → io::PathExt, io::hash_bytes, layout, error                  (LEAF)
commands/agent/task/{new, archive, resume, discard} → state_file, session, state
commands/context/gather                              → state_file, session
commands/agent/task/worktree/{discovery, list}       → state_file
commands/unload                                       → state_file (path constants only)
```

Call graph for `task new --slug a --tier quick`:

```
task_new(opts)
  ├── validate_slug(&opts.slug)
  ├── if task_dir.exists() → Error::TaskAlreadyExists
  ├── task_dir.ensure_dir()
  ├── copy_template("PRD", &task_dir.join("PRD.md"))
  ├── build_task_toml(&opts).save(&task_dir)
  └── state_mutate(&layout, &ppid, |state| {
        // After two-way reconcile, state.tasks.active may already include
        // opts.slug (the dir we just created is now visible to add-pass).
        // The "other-active before this command" check filters it out.
        let had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug);
        if had_other_active {
            eprintln!("warn: {n} active task(s); see `ark agent task resume`");
        }
        if !state.tasks.active.contains(&opts.slug) {
            state.tasks.active.push(opts.slug.clone());
        }
        let id = resolve_session_id(&layout, &ppid)?;
        let pid = ppid.parent_id();
        state.sessions.insert(id.0, Session { focus: opts.slug.clone(), pid });
        Ok(())
      })?
```

Call graph for `task archive --slug a` (rename-first):

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── task_dir.rename_to(&archive_path)              (1) rename FIRST
  ├── toml.phase = Archived; toml.archived_at = now;
  │     toml.save(&archive_path)                     (2) save Archived metadata
  ├── if tier == Deep:                               (3) SPEC promotion
  │     ├── spec_extract(SpecExtractOptions { task_dir_override: Some(&archive_path), ... })
  │     └── spec_register(SpecRegisterOptions { ... })
  └── state_mutate(&layout, &ppid, |state| {          (4) state cleanup
        state.tasks.active.retain(|s| s != &opts.slug);
        for sess in state.sessions.values_mut() { /* clear if focus matches */ }
        let id = resolve_session_id(&layout, &ppid)?;
        if state.sessions.get(&id.0).map(|s| &s.focus) == Some(&opts.slug) {
            state.sessions.remove(&id.0);
            release_session_id(&layout, &id)?;
        }
        Ok(())
      })?
```

Recovery for step (4) failure: next `load_state` runs reconcile — add-pass excludes `archive/` so the archived slug is not re-added; drop-pass removes any stale active entry. SPEC integrity is durable; state integrity is recoverable. No corruption.

Call graph for `task worktree list`:

```
worktree_list()
  ├── git_worktree_list_porcelain()
  └── for each <wt> under <root>/<cfg.worktree_dir>/:
        ├── wt_layout = Layout::new(&wt)
        ├── state = state_file::load_state(&wt_layout, &ppid)?    (includes reconcile + prune)
        └── for slug in &state.tasks.active:
              ├── task_dir = wt_layout.task_dir(slug)
              ├── toml = TaskToml::load(&task_dir)?
              └── emit row
```

[**Data Structure**]

```rust
// crates/ark-core/src/state_file/model.rs
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

// crates/ark-core/src/session/cache.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

pub fn cache_file_path(layout: &Layout, ppid: u32) -> PathBuf;

/// Stateless predicate. Returns true iff the cache file at
/// `cache_file_path(layout, pid)` exists AND its trimmed contents
/// equal `uuid`. Used by `prune_dead_sessions` per session entry.
pub fn cache_matches(layout: &Layout, pid: u32, uuid: &str) -> bool;

// crates/ark-core/src/session/ppid.rs
/// Source of parent-id for the current process. Trait-shaped to allow
/// deterministic test injection. Production constructs `RealPpid` once
/// at CLI startup; tests pass `StubPpid(u32)`.
pub trait Ppid {
    fn parent_id(&self) -> u32;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealPpid;
// Unix: std::os::unix::process::parent_id()
// Windows: walks toolhelp snapshot to find the parent of GetCurrentProcessId();
//          returns calling process's own PID on toolhelp failure (C-18).

#[derive(Debug, Clone, Copy)]
pub struct StubPpid(pub u32);
impl Ppid for StubPpid { fn parent_id(&self) -> u32 { self.0 } }

// crates/ark-core/src/error.rs (additions)
#[error("state file `{path}` is corrupt: {source}")]
StateTomlCorrupt { path: PathBuf, #[source] source: toml::de::Error },

#[error("state file `{path}` is locked by another process; gave up after backoff")]
StateLockContended { path: PathBuf },

#[error("task `{slug}` has user content in {file}; pass --force to discard anyway")]
TaskStillActive { slug: String, file: String },

#[error("no active task in `{}`; run `ark agent task new` first", project_root.display())]
NoActiveTask { project_root: PathBuf },

#[error(
    "multiple active tasks: {}; run `ark agent task resume --slug <one-of>` to focus this session",
    candidates.join(", ")
)]
AmbiguousActiveTask { candidates: Vec<String> },
```

[**API Surface**]

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
pub fn prune_dead_sessions(layout: &Layout, state: &mut StateFile);

// session/cache.rs
pub fn resolve_session_id(layout: &Layout, ppid: &dyn Ppid) -> Result<SessionId>;
pub fn release_session_id(layout: &Layout, id: &SessionId) -> Result<()>;
pub fn cache_matches(layout: &Layout, pid: u32, uuid: &str) -> bool;

// commands/agent/task/resume.rs
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary>;

// commands/agent/task/discard.rs
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary>;

// Library re-exports from crates/ark-core/src/lib.rs
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

The two new task ops (`task_resume`, `task_discard`, plus existing `task_new`/`task_archive`) construct `RealPpid` internally as a default. Integration tests rely on the OS PPID being stable within one test process; unit tests in `state_file::*` and `session::cache::*` exercise the `Ppid` trait via `StubPpid`.

CLI subcommand additions:

```
ark agent task resume  --slug <s>
ark agent task discard --slug <s> [--force]
```

[**Constraints**]

- C-1: Locking primitive is stdlib `File::try_lock` (Rust 1.89+). Cross-platform PPID via the `Ppid` trait + `RealPpid`'s `cfg`-gated shim. New deps: `uuid` (any host); `windows-sys` under `[target.'cfg(windows)'.dependencies]` only.
- C-2: Lock file is `.ark/.state.toml.lock`; backoff: 5 attempts at 10/20/40/80/160 ms (≤ 320 ms cumulative).
- C-3: State write is atomic: `.state.toml.tmp.<pid>` then `rename_to(.state.toml)`.
- C-4: `state_mutate` unlinks `.state.toml.tmp.*` orphans on lock acquire.
- C-5: Cache file naming: `<temp_dir>/ark-session-<project_hash>-<ppid>.id`. PPID source is the `Ppid` trait passed at the call site.
- C-6: No `.unwrap()` in production; one allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`.
- C-7: All filesystem access in `state_file/` and `session/` routes through `io::PathExt`.
- C-8: All `.ark/`-relative paths route through `Layout` helpers. New consts `STATE_FILE`, `STATE_LOCK_FILE`. Legacy `tasks_current()` accessor kept for migration.
- C-9: Reconcile drops a `tasks.active` entry when its `task.toml` is missing or `phase == Archived`. Drops a session when its slug-focus is no longer active.
- C-10: `prune_dead_sessions` drops a session when `cache_matches(layout, pid, uuid) == false`.
- C-11: `load_state` does NOT delete legacy files; migration deletion happens on the next successful `state_mutate`.
- C-12: `state_mutate` is the sole path that mutates the state file.
- C-13: `task_discard` reads each seeded file and compares to its template; first divergence → `Error::TaskStillActive`. `--force` skips the scan.
- C-14: Legacy accessors carry a doc comment "legacy migration accessor; remove after migration window."
- C-15: `[tasks].active` is sorted+deduped on every save.
- C-16: No SessionStart hook integration.
- C-17: Each worktree's state file is independent.
- C-18: `RealPpid::parent_id()` on Windows returns the calling process's own PID on toolhelp snapshot failure. Unix path has no failure mode.
- C-19: `task_new` filters `had_other_active = state.tasks.active.iter().any(|s| s != &opts.slug)` AFTER reconcile (which may have already added the new slug). Push is `if !contains { push }`. C-15 sort+dedup is the belt-and-braces second line.
- C-20: `task_archive` ordering is rename-first: (1) rename to archive path; (2) save Archived metadata; (3) deep-tier `spec_extract + spec_register`; (4) `state_mutate` cleanup. State integrity recoverable via reconcile if (4) fails.
- C-21: `unload.rs` skip set in BOTH walk sites: `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file()]`. `walk_files_excluding` extended to skip `.state.toml.tmp.*` orphans.
- C-22: `reconcile_against_disk` runs in order: (1) enumerate `.ark/tasks/<slug>/task.toml` excluding `archive/`, push to `state.tasks.active` if absent; (2) drop `tasks.active` entries whose `task.toml` is missing or `phase == Archived`; (3) sort+dedup per C-15; (4) drop sessions whose `focus` is no longer active; (5) `prune_dead_sessions`. Order matters: add precedes drop so a transient inconsistency never collapses an active slug; prune-sessions comes last so newly-inactive sessions (from step 4) are pruned in the same pass.
- C-23: `--slug` is required only on `task new`, `task resume`, `task discard`. Other verbs resolve via topology cascade: worktree path with active-set membership → single active task → session focus. Empty active set → `NoActiveTask`. Ambiguous → `AmbiguousActiveTask`.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
