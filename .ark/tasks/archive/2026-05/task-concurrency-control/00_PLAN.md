# `task-concurrency-control` PLAN `00`

> Status: Draft
> Feature: `task-concurrency-control`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Replace the two single-purpose per-checkout files (`.ark/tasks/.current` and `.ark/.developer`) with one `.ark/.state.toml` that carries developer identity, the **set** of active task slugs, and a per-session focus map. Add `task resume` and `task discard` ops. Introduce session-id derivation (PPID + project-hash + UUID cache in OS temp dir), file-lock-and-atomic-write for cross-process safety, and self-healing migration from the legacy two-file layout. Each `.ark/` (parent and every worktree) owns its own state file.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: A single per-checkout `.ark/.state.toml` file carries identity, the active-task set, and a per-session focus map. Truth still lives in `.ark/tasks/<slug>/task.toml`; the state file is an index reconciled on every read.
- G-2: Concurrent CLI invocations from independent shells in the same `.ark/` each get an independent `[sessions.<uuid>]` entry and an independent focused task. No session can clobber another's focus.
- G-3: Two new agent ops — `ark agent task resume <slug>` and `ark agent task discard <slug>` — extend the agent task verb set (extends `ark-agent-namespace` SPEC G-3).
- G-4: The identity API (`read_developer_name`, `require_developer_name`, `write_developer_file` in `commands/agent/workspace/identity.rs`) preserves its public signatures; bodies delegate to the new state-file API. The 4 existing call sites are unchanged.
- G-5: Legacy `.ark/.developer` and `.ark/tasks/.current` are auto-migrated on the first state-file mutation and then deleted. The reader tolerates either layout indefinitely so a pure read in a not-yet-migrated install does not change the on-disk state.
- G-6: State-file mutations are atomic across crash *and* across concurrent CLI invocations on Linux, macOS, and Windows. Crash mid-write leaves no partial `.state.toml`. Concurrent writers serialize via an OS-level file lock with bounded backoff.
- G-7: Dead sessions are GC'd transparently on the next state read (in-memory; persisted on the next mutation). A session is "dead" when its temp-dir cache file is missing or mismatched.
- G-8: `task new` warns (does not refuse) when `[tasks].active` is non-empty before the new task is appended. Replaces the silent `.current` clobber that orphaned in-flight tasks.
- G-9: Each git worktree's `.ark/` owns its own `.state.toml`. `task new --worktree` writes only to the worktree's state file; the parent's state file is untouched. (Mirrors `worktree` SPEC G-9.)
- G-10: `--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask` — outside-Ark's-contract usage is rejected, not papered over.
- G-11: `task discard <slug>` refuses without `--force` when seeded files (PRD/PLAN/etc.) differ from their templates ("PRD has user content" guard). Always refuses if the task is already archived (use the archive flow's removal path instead).

- NG-1: Workflow-phase model changes (drop VERIFY, postpone ARCHIVE) — ROADMAP item #2.
- NG-2: SessionStart hook integration — sessions register only on `task new` / `task resume`.
- NG-3: Explicit `session end` / `session list` ops — GC is the only reaper. Listing sessions is debug-tier; not justifying CLI surface.
- NG-4: Cross-host coordination (NFS-shared `.ark/`). File-lock semantics on networked filesystems are not specified by the OS; documented as unsupported.
- NG-5: Heartbeats / `last_seen` timestamps. Liveness is purely cache-file-presence.
- NG-6: Identity rename. Still NG-7 from `workspace` SPEC. Re-init still requires hand-deleting state file's `[identity]` (or, transitionally, the legacy `.developer`).
- NG-7: Removal of `Layout::tasks_current()` / `Layout::developer_file()` accessors. Migration's reader needs them; remove in a follow-up task once a release ships with migration.
- NG-8: Multi-task focus per session. One session focuses one slug at a time. Switching is `task resume <other>`.
- NG-9: External lock crate (`fs2`, `fd-lock`). Use stdlib `File::try_lock` (stable Rust 1.89+); pin `rust-version = "1.89"` in `Cargo.toml` if not already.
- NG-10: `ark upgrade` migration step. Migration is read-on-load, write-on-mutate, delete-legacy-on-write. Self-healing.

[**Architecture**]

New module tree under `crates/ark-core/src/`:

```
crates/ark-core/src/
├── state_file/                        (NEW — name avoids collision with existing state/)
│   ├── mod.rs                         pub use of the surface; invariants doc
│   ├── model.rs                       StateFile, Identity, Tasks, Session + serde
│   ├── io.rs                          load_state, state_mutate, lock acquire/release, atomic write
│   ├── reconcile.rs                   drop missing/Archived entries (in-memory)
│   └── migrate.rs                     synthesize_from_legacy + delete_legacy_files
├── session/                           (NEW)
│   ├── mod.rs                         pub use; cache-file naming convention
│   ├── cache.rs                       resolve_session_id, release_session_id, project_hash
│   └── gc.rs                          prune_dead_sessions (in-memory)
├── commands/agent/task/
│   ├── resume.rs                      (NEW) task_resume
│   ├── discard.rs                     (NEW) task_discard, --force handling, template-diff guard
│   ├── new.rs                         MOD: append to active + register session via state_mutate
│   ├── archive.rs                     MOD: state_mutate first, then rename (reverses 103-comment)
│   └── mod.rs                         MOD: pub mod resume; pub mod discard;
├── commands/agent/workspace/
│   └── identity.rs                    MOD: bodies delegate to state_file (API stable)
├── commands/context/gather.rs         MOD: focused slug via state_file
├── commands/agent/task/worktree/
│   ├── discovery.rs                   MOD: per-worktree state_file lookup
│   └── list.rs                        MOD: per-worktree state_file enumeration
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
└── .gitignore                         MOD: add .state.toml and .state.toml.lock (additive;
                                       legacy .developer line stays for migration window)
```

Why a new `state_file/` directory (not a sibling under existing `state/`): the existing `crates/ark-core/src/state/` owns install-manifest and snapshot models. Reusing the name would conflate two different "state" concepts. `state_file/` cleanly maps to `.ark/.state.toml`.

The `session/` module is sibling to `io/` because session identity is a process-runtime concern (PPID, temp dir), not a `.ark/`-managed file. Dependency direction stays one-way: `state_file` does not depend on `session`; the `state_mutate` closure pattern lets callers thread their session id in.

#### Module coupling

```
state_file → io::PathExt, io::hash_bytes, layout, error
session    → io::PathExt, io::hash_bytes, layout, error
commands/agent/task/{new,archive,resume,discard} → state_file, session, state (existing TaskToml)
commands/agent/workspace/identity → state_file (replaces direct file I/O)
commands/context/gather → state_file, session (focused slug for *this* session)
commands/agent/task/worktree/{discovery,list} → state_file (per-worktree Layout::new(wt))
```

`state_file` and `session` MUST NOT import each other. `commands/agent/*` orchestrates the pair.

#### Call graph: `task new --slug a --tier quick`

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
        state.tasks.active.push(opts.slug.clone());           // dedup BTreeSet semantics
        let id = resolve_session_id(&layout)?;
        state.sessions.insert(id.0, Session {
            focus: opts.slug.clone(),
            pid: std::process::parent_id(),
        });
        Ok(())
      })?
```

#### Call graph: `task archive --slug a` (decision per user: state-mutate FIRST, then rename)

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── if tier == Deep: spec_extract + spec_register (unchanged)
  ├── state_mutate(&layout, |state| {
  │     state.tasks.active.retain(|s| s != &opts.slug);
  │     for sess in state.sessions.values_mut() { /* clear if focus matches */ }
  │     // own session: also drop the entry entirely (focus released)
  │     let id = resolve_session_id(&layout)?;
  │     if state.sessions.get(&id.0).map(|s| &s.focus) == Some(&opts.slug) {
  │         state.sessions.remove(&id.0);
  │         release_session_id(&layout, &id)?;  // delete cache file
  │     }
  │     Ok(())
  │   })?
  ├── toml.phase = Archived; toml.archived_at = Some(now); toml.save(&task_dir)
  ├── task_dir.rename_to(&archive_path)?               // atomic on POSIX, atomic on Windows
  └── record_task(...)?                                 // unchanged workspace bridge
```

If the rename fails, the next `load_state` reconcile sees the active set already excludes `<slug>` but the dir is still at `tasks/<slug>/`. Reconcile drops the stale active entry; user re-runs `archive` or `discard` to clean up. (See Trade-offs T-3 for the rationale.)

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

    /// Keyed by session UUID string. BTreeMap for stable serialization order.
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
    /// Active (created-but-not-archived) task slugs.
    /// Stored as Vec for TOML compatibility; semantics are set (deduped).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub focus: String,        // active slug
    pub pid: u32,             // PPID at registration; for GC liveness probe
}
```

```rust
// crates/ark-core/src/session/cache.rs

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);  // UUID v4 hex

/// Returns the cache file path for (project_root, ppid).
/// Format: `<temp_dir>/ark-session-<project_hash>-<ppid>.id`
/// where project_hash = first 16 hex chars of hash_bytes(layout.root().to_string_lossy()).
pub fn cache_file_path(layout: &Layout, ppid: u32) -> PathBuf;
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

`StateTomlCorrupt` mirrors `TaskTomlCorrupt`. `StateLockContended` distinguishes "locked" from generic IO error so the CLI can render a kinder message. `TaskStillActive` is the discard guard.

[**API Surface**]

Library re-exports added to `crates/ark-core/src/lib.rs`:

```rust
pub use state_file::{Identity, Session, StateFile, load_state, state_mutate};
pub use session::{SessionId, cache_file_path, release_session_id, resolve_session_id};
pub use commands::agent::task::{
    TaskDiscardOptions, TaskDiscardSummary, TaskResumeOptions, TaskResumeSummary,
    task_discard, task_resume,
};
```

Core public functions:

```rust
// state_file/io.rs

/// Load (or synthesize from legacy + reconcile + GC) the per-checkout state file.
/// Pure read; never persists. Cheap fallback to defaults if file missing.
pub fn load_state(layout: &Layout) -> Result<StateFile>;

/// Acquire exclusive lock with bounded backoff, load_state, run `edit`, write atomically,
/// release lock. The closure receives an in-memory StateFile that is already reconciled
/// against on-disk truth and GC'd. On successful save, legacy files (`.developer`,
/// `tasks/.current`) are unlinked if still present.
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
where
    F: FnOnce(&mut StateFile) -> Result<()>;
```

```rust
// session/cache.rs

/// Resolve this Ark CLI invocation's session id.
/// 1) Compute cache_file_path(layout, parent_id())
/// 2) If exists and parses as UUID, return it.
/// 3) Else generate UUID v4, write atomically to cache file, return.
pub fn resolve_session_id(layout: &Layout) -> Result<SessionId>;

/// Delete this session's cache file. Idempotent (uses remove_if_exists).
/// Called by task_archive (own session) and task_discard (when discarding own focus).
pub fn release_session_id(layout: &Layout, id: &SessionId) -> Result<()>;
```

```rust
// session/gc.rs

/// Drop [sessions.*] entries whose cache file is missing or mismatched.
/// Pure in-memory mutation; persisting is state_mutate's job.
pub fn prune_dead_sessions(layout: &Layout, state: &mut StateFile);
```

```rust
// commands/agent/task/resume.rs

pub struct TaskResumeOptions {
    pub project_root: PathBuf,
    pub slug: String,
}
pub struct TaskResumeSummary { pub slug: String }
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary>;
//   - validate_slug(&opts.slug)
//   - state_mutate: require active.contains(&slug) else TaskNotFound;
//                   set sessions.<self>.focus = slug

// commands/agent/task/discard.rs

pub struct TaskDiscardOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub force: bool,
}
pub struct TaskDiscardSummary { pub slug: String, pub task_dir: PathBuf }
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary>;
//   - validate_slug
//   - load TaskToml; if phase == Archived → TaskNotFound (use archive removal)
//   - if !opts.force: scan task_dir's seeded files vs. embedded templates;
//                     any divergence → Error::TaskStillActive { slug, file }
//   - state_mutate: drop slug from active; clear focus in any session pointing at it;
//                   if my session's focus matched, release_session_id afterward
//   - task_dir.remove_dir_all()
```

Identity API (preserved verbatim, body swapped):

```rust
// crates/ark-core/src/commands/agent/workspace/identity.rs

pub fn validate_developer_name(name: &str) -> Result<()>;          // unchanged
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>>;
pub fn require_developer_name(layout: &Layout) -> Result<String>;
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()>;
```

New bodies:

- `read_developer_name` → `Ok(state_file::load_state(layout)?.identity.map(|i| i.name))`
- `require_developer_name` → unwrap-or-`Error::DeveloperNotInitialized { path: layout.state_file() }`
- `write_developer_file` → `state_mutate` closure: if existing identity name differs → `DeveloperAlreadyInitialized`; else set `state.identity = Some(Identity { name, initialized_at: now })`.

CLI subcommand additions in `crates/ark-cli/src/agent_cli.rs`:

```rust
enum TaskCommand {
    // ... existing variants ...
    Archive(TaskSlugArgs),
    Resume(TaskSlugArgs),                  // NEW — same shape as Archive
    Discard(TaskDiscardCliArgs),           // NEW — adds --force
    Promote(TaskPromoteArgs),
    Worktree(TaskWorktreeArgs),
}

#[derive(clap::Args)]
struct TaskDiscardCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to this session's focused task in `.ark/.state.toml`.
    #[arg(long)]
    slug: Option<String>,
    /// Force discard even when seeded files have user content.
    #[arg(long)]
    force: bool,
}
```

Dispatch arms call `task_resume(...)` and `task_discard(...)` with `--slug` resolved via `resolve_slug` (whose body now reads from state file).

[**Constraints**]

- C-1: `File::try_lock` (stdlib, stable Rust 1.89+) is the *only* lock primitive. No external crate. Pin `rust-version = "1.89"` in `Cargo.toml` if not already pinned.
- C-2: Lock file is `.ark/.state.toml.lock`. Backoff is **exactly** 5 attempts at 10/20/40/80/160 ms (cumulative ≤ 320 ms). On exhaustion → `Error::StateLockContended { path }`. Lock auto-released on `File` drop (incl. process exit / SIGKILL).
- C-3: State file write is atomic: write `.state.toml.tmp.<pid>`, then `rename_to(.state.toml)`. Cross-platform via `std::fs::rename` (replaces existing on Windows since Rust 1.5).
- C-4: At the start of every `state_mutate` (after lock acquired), enumerate and unlink any `.state.toml.tmp.*` orphan files (left by crashes mid-write). Safe under lock.
- C-5: Cache file naming: `<std::env::temp_dir()>/ark-session-<project_hash>-<ppid>.id`. `project_hash` is the first 16 hex chars of `hash_bytes(layout.root().to_string_lossy().as_bytes())`. Cache file body is the bare UUID v4 hex string, no trailing newline.
- C-6: No `.unwrap()` in production code. The single allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`'s pattern, justified after model is exercised in unit tests.
- C-7: All filesystem access in `state_file/`, `session/`, and the new task ops routes through `io::PathExt`. No bare `std::fs::*` (matches existing `commands/agent/` discipline).
- C-8: All `.ark/`-relative paths route through `Layout` helpers. New constants: `STATE_FILE = ".ark/.state.toml"`, `STATE_LOCK_FILE = ".ark/.state.toml.lock"`. New accessors: `Layout::state_file()`, `Layout::state_lock_file()`. Legacy `tasks_current()` and `developer_file()` accessors are **kept** for migration; documented as "remove in follow-up after migration window."
- C-9: Reconcile drops a `tasks.active` entry when the corresponding `task.toml` is missing OR `phase == Archived`. Drops a `sessions.*` entry when its slug-focus is no longer in `tasks.active` after the active reconcile.
- C-10: GC drops a `sessions.*` entry when `cache_file_path(layout, session.pid)` does not exist OR exists but its trimmed contents do not equal the session UUID.
- C-11: A **read** (`load_state`) does NOT delete legacy files. Migration's delete step happens only on the next successful `state_mutate` save. Pure-read paths in not-yet-migrated installs leave the legacy two files alone.
- C-12: `state_mutate` is the sole path that mutates the state file. Direct `StateFile::save` from outside the module is private.
- C-13: `task_discard`'s template-diff guard reads each seeded file (`PRD.md`, `*_PLAN.md`, `*_REVIEW.md`, `VERIFY.md` — whichever exist) and compares against the embedded template via byte equality after stripping the first-line `{Feature Name}` placeholder substitution. First file with divergence → `Error::TaskStillActive { slug, file }`. `--force` skips the entire scan.
- C-14: The `Layout::tasks_current()` and `Layout::developer_file()` accessors carry a doc-comment "deprecated; remove after migration window once .state.toml is universal." They remain `pub` for `migrate.rs` to use.
- C-15: State file's `[tasks].active` is deduped on every save: `state.tasks.active.sort(); state.tasks.active.dedup();`.
- C-16: No SessionStart hook integration. Sessions register only on `task new` / `task resume`. Documented in `state_file/mod.rs` doc-comment.
- C-17: Each worktree's state file is independent. `Layout::new(<worktree_path>)` returns a Layout pointing at the worktree's own `.ark/`; `state_file::*` operations never reach across worktrees. (Already true by Layout design; documented for clarity.)

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
   - Reconcile against on-disk task dirs (drop stale actives).
   - GC dead sessions.
   - Run closure: warn if `active` non-empty; push `a`; insert/update `sessions.<self> { focus: "a", pid: parent_id() }`.
   - Serialize state to `.state.toml.tmp.<pid>`. `rename_to(.state.toml)`.
   - Unlink `.developer` and `tasks/.current` if present (migration finalize).
   - Lock guard drops; OS releases.
5. Print summary: `created Quick task ... (1 active; this session focused)`.

Scenario B — `task new --slug b` from a second shell, same `.ark/`:

Identical to Scenario A, except step 4's reconcile sees `active = ["a"]` and the closure's `eprintln!` fires: `warn: 1 active task(s); see "ark agent task resume" to revisit`. Result: `active = ["a", "b"]`; two `sessions.<id>` entries with distinct PPIDs and distinct focuses.

Scenario C — `task execute` with no `--slug`:

1. CLI dispatch → `resolve_slug(root, None)`.
2. `resolve_slug` calls `resolve_slug_via_state(layout)`:
   - `let id = resolve_session_id(layout)?;`
   - `let state = load_state(layout)?;`
   - `state.sessions.get(&id.0).map(|s| s.focus.clone()).ok_or(Error::NoCurrentTask { path: layout.state_file() })`
3. Phase transition runs against the resolved slug.

Scenario D — `task archive --slug a` (this session focused on `a`):

1. Load TaskToml; check_transition.
2. Deep-tier SPEC promotion (unchanged).
3. `state_mutate`:
   - Drop `"a"` from `active`.
   - For each session: if `focus == "a"`, drop the session entry entirely.
   - Resolve own session id; if it was just dropped, `release_session_id(layout, &id)` (deletes cache file).
4. Mark TaskToml as Archived; save.
5. `task_dir.rename_to(archive_path)`.
6. `record_task(...)` (unchanged workspace bridge).

[**Failure Flow**]

- **Lock contention beyond backoff**: `Error::StateLockContended { path: layout.state_file() }`. CLI renders friendly message; user retries.
- **Corrupt state file**: `Error::StateTomlCorrupt { path, source }` with chained TOML error. Recovery: hand-delete `.state.toml`; next mutation re-synthesizes from legacy if present, else creates fresh.
- **Crash mid-write**: `.state.toml` either is the pre-write version (rename hadn't happened) or the post-write version (rename completed atomically). `.state.toml.tmp.<pid>` orphan reaped by next `state_mutate` (C-4). Lock released by OS on process death.
- **`task discard` on archived task**: `Error::TaskNotFound { slug }`. Tells user to look under `tasks/archive/` (no removal there yet — out of scope).
- **`task discard` without --force, PRD edited**: `Error::TaskStillActive { slug, file: "PRD.md" }`.
- **`task resume <bogus>`**: `Error::TaskNotFound { slug }`.
- **`task archive` rename fails after state mutate succeeds**: state file says inactive, dir still at `tasks/<slug>/`. Next `load_state` reconcile drops the stale active entry. User re-runs `task discard <slug>` (or `archive`) to finish cleanup.
- **GC drops own session by mistake**: cannot happen — `resolve_session_id` always (re)creates the cache file, so the session that called `state_mutate` ALWAYS has a live cache file when GC runs inside that mutation.
- **PPID recycling**: rare; if a new shell happens to get the recycled PPID and a stale cache file from a prior session of the same project still exists in temp dir, the new shell adopts the old UUID. Mitigation: cache file is deleted on `task archive`/`discard` (focus-release moments). Residual risk accepted (NG-5).

[**State Transitions**]

```
[no .state.toml, no legacy]      ──load_state──>      StateFile::default()
[no .state.toml, has legacy]     ──load_state──>      synthesize from legacy (in-memory only)
[has .state.toml]                ──load_state──>      parse TOML, reconcile, GC
[any of above]                   ──state_mutate──>    closure runs on reconciled state
[state_mutate save success]      ──finalize──>        delete legacy files if present

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

- `crates/ark-core/src/state_file/{mod,model,io,reconcile,migrate}.rs`
- `crates/ark-core/src/session/{mod,cache,gc}.rs`
- `crates/ark-core/src/error.rs` — add `StateTomlCorrupt`, `StateLockContended`, `TaskStillActive`.
- `crates/ark-core/src/layout.rs` — add `STATE_FILE`/`STATE_LOCK_FILE` constants and `state_file()`/`state_lock_file()` accessors.
- `crates/ark-core/src/lib.rs` — `pub mod state_file; pub mod session;` plus the new re-exports.
- `crates/ark-core/Cargo.toml` — add `uuid = { version = "1", features = ["v4"] }` dep. Pin `rust-version = "1.89"` in workspace if not already.

Unit tests in this phase (in-module `#[cfg(test)]`):

- `state_file::io::tests`
  - `load_state_returns_default_when_missing_and_no_legacy`
  - `load_state_synthesizes_from_legacy_developer_and_current`
  - `load_state_does_not_delete_legacy_on_pure_read` (C-11)
  - `state_mutate_writes_atomically_and_releases_lock` (no `.tmp.*` after, no `.lock` held)
  - `state_mutate_deletes_legacy_files_on_first_save_after_migration`
  - `state_mutate_unlinks_orphan_tmp_files_on_acquire` (C-4)
  - `state_mutate_returns_lock_contended_after_backoff`
  - `concurrent_state_mutate_serializes_and_both_succeed` (multi-thread)
- `state_file::reconcile::tests`
  - `drops_active_slug_with_missing_dir`
  - `drops_active_slug_with_phase_archived`
  - `drops_session_whose_focus_no_longer_active`
- `state_file::migrate::tests`
  - `synthesize_returns_none_when_state_toml_present`
  - `synthesize_reads_legacy_developer_and_current`
  - `synthesize_handles_missing_developer_or_missing_current_independently`
- `session::cache::tests`
  - `resolve_round_trips_within_same_ppid`
  - `cache_file_path_uses_project_hash_and_ppid`
  - `release_removes_cache_file_idempotently`
- `session::gc::tests`
  - `prune_drops_session_with_missing_cache`
  - `prune_drops_session_with_mismatched_uuid`
  - `prune_keeps_session_with_matching_cache`

[**Phase 2 — Rewire Callers (behavior preserved, internals switched)**]

- `commands/agent/workspace/identity.rs` — replace bodies with `state_file` delegations. Keep all existing tests; they should pass against the new backend (file-format details now an implementation detail).
- `commands/agent/workspace/init.rs:78` — `write_developer_file` call site unchanged.
- `commands/agent/task/new.rs:135-137, 262` — replace direct `tasks_current().write_bytes(...)` with `state_mutate` closure that appends to `tasks.active`, registers this session, warns on non-empty pre-existing active set.
- `commands/agent/task/archive.rs:103-141` — reverse the rename-first ordering. New ordering per Runtime/Main Flow: state_mutate → save TaskToml → rename. Update the comment block at line 103-105 to document the new invariant ("State file is mutated first; if rename fails, reconcile drops stale entry").
- `commands/context/gather.rs:314` — replace `tasks_current().read_text_optional()` with `resolve_session_id` + `load_state` + this-session-focus lookup.
- `commands/agent/task/worktree/discovery.rs:94, list.rs:87` — per-worktree `Layout::new(<wt_path>)` already in place. Replace `.current` reads with `state_file::load_state(&wt_layout)?.sessions...` per-session scan; for backward compat (older worktrees pre-migration), fall through to legacy `.current` (which `load_state` already handles via migrate).
- `crates/ark-cli/src/agent_cli.rs:384-394` — `resolve_slug` body becomes `resolve_slug_via_state(root, explicit)`; same signature.

Tests adjusted in this phase:

- `agent_lifecycle.rs:48`, `archive.rs:218` — assertion on `!.current.exists()` becomes assertion on state file's `!active.contains(&slug)`.
- `init.rs:580, 590` — assertion on `.developer` existence becomes assertion on state file's `identity.is_some()` and `identity.unwrap().name == "alice"`.
- `context/gather.rs:490, 524` — mock `.current` writes become `state_mutate` calls in test setup.

[**Phase 3 — New Ops + CLI Surface + Docs**]

- `commands/agent/task/resume.rs` and `discard.rs` — implement per signatures above.
- `commands/agent/task/mod.rs` — `pub mod resume; pub mod discard;` and re-exports.
- `crates/ark-cli/src/agent_cli.rs`:
  - Add `Resume(TaskSlugArgs)` and `Discard(TaskDiscardCliArgs)` enum variants.
  - Dispatch arms call `task_resume` / `task_discard` and `render(...)`.
- `crates/ark-core/src/lib.rs` — add the two new pub-uses.
- `templates/ark/.gitignore` — add `.state.toml` and `.state.toml.lock` lines (additive; the existing `.developer` line stays for the migration window).
- `templates/ark/workflow.md` — add a brief subsection under §6 Mechanics describing multi-session focus, `task resume`, `task discard`. Keep terse.
- `.claude/commands/ark/quick.md` and `design.md` — minor updates: mention the warn-on-active-nonempty behavior; advertise quick-tier `--worktree` opt-in (already supported per worktree SPEC G-2).

Integration tests in `crates/ark-cli/tests/`:

- New file `agent_session.rs`:
  - `multi_session_focus_isolation` — two simulated sessions, distinct PPIDs (forge cache files); each `task new` registers its own focus; `task execute` from each resolves to its own focus.
  - `gc_drops_dead_session_on_load` — hand-write state file with two sessions; create cache for one only; `load_state` returns state with the dead one pruned.
  - `lock_contention_succeeds_after_backoff` — two threads, each `state_mutate` with 50 ms inner sleep; both succeed.
  - `migration_synthesizes_from_legacy_and_deletes` — pre-seed `.developer` + `.current`; run `task new`; assert `.state.toml` correct and legacy gone.
  - `discard_force_removes_edited_task` — `task new`; mutate PRD; `task discard` returns `TaskStillActive`; `--force` succeeds.
  - `resume_invalid_slug_errors` — `task resume bogus` → `TaskNotFound`.
  - `worktree_isolation` — `task new --worktree --slug w1`; parent's `.state.toml` does not list `w1`.
- Extensions to `agent_lifecycle.rs`:
  - Update the post-archive assertion from `!.current.exists()` to active-set-not-containing-slug.

---

## Trade-offs

- **T-1: stdlib `File::try_lock` vs. `fs2`/`fd-lock` crate.** Stdlib is zero-dep, stable on all three target OSes since 1.89. `fs2` is more battle-tested but adds a dependency for a single primitive. **Choice: stdlib.** Risk: if Windows behavior under MSYS/Cygwin shells (where Claude Code might run) misbehaves, fall back to `fd-lock` in a follow-up.
- **T-2: One `.state.toml` vs. two files (`.identity.toml` + `.state.toml`).** Two files would isolate the rarely-changing identity from the churning task state, simplifying lock contention. But identity changes once and locks would never contend; one file is cleaner. **Choice: one file.** All state in one place; one lock; one atomic write.
- **T-3: Archive ordering — state_mutate-first vs. rename-first.** Rename-first (current code) means "if anything fails after the rename, the task is at the archive path — recoverable." State_mutate-first (chosen) means "if rename fails, the state file lies briefly until reconcile catches it." Per user direction, **state_mutate-first**: the cleanup invariant ("active set always reflects what's in `tasks/`") is stronger and reconcile self-heals. The current line-103 comment must be updated.
- **T-4: Discard guard — template-diff vs. always-require-`--force`.** Template-diff (chosen) is friendlier ergonomics: discarding a never-touched task within seconds of `task new` works without flag. Touching PRD then trying to discard requires `--force`. Cost: ~20 LOC for the diff scan. **Choice: template-diff.**
- **T-5: Project-hash length — 16 hex chars vs. full 64.** 16 chars (8 bytes of entropy) gives 2^64 namespace; collision risk in a single user's temp dir is astronomically low. Full 64-char hash makes filenames unwieldy. **Choice: 16.**
- **T-6: Deduplication of `tasks.active` — BTreeSet model vs. Vec with sort/dedup-on-save.** TOML can't serialize a Set type natively; either a Vec on the wire or a custom (de)serializer. Vec with sort+dedup on save (C-15) is simpler; reads tolerate duplicates by treating them as if deduped. **Choice: Vec with sort+dedup.**
- **T-7: Identity API preservation.** Could rename `read_developer_name` → `read_identity_name` to reflect the new backend, but the workspace SPEC's G-2/G-7 exposes those names to multiple call sites and doc references. **Choice: preserve names**, swap bodies. Future tidy-up out of scope.

---

## Validation

[**Unit Tests**]

- **V-UT-1:** `load_state_returns_default_when_missing_and_no_legacy` — fresh tempdir; `load_state` returns `StateFile::default()`.
- **V-UT-2:** `load_state_synthesizes_from_legacy_developer_and_current` — write legacy `.developer` + `.current`; `load_state` returns identity + `active = [<slug>]`.
- **V-UT-3:** `load_state_does_not_delete_legacy_on_pure_read` — verify legacy files still on disk after pure `load_state`.
- **V-UT-4:** `state_mutate_round_trips_atomically` — closure mutates; on disk: `.state.toml` present, no `.tmp.*`, lock file unlocked.
- **V-UT-5:** `state_mutate_deletes_legacy_files_on_first_save` — pre-seed legacy, run `state_mutate`, verify legacy files gone after.
- **V-UT-6:** `state_mutate_unlinks_orphan_tmp_files_on_acquire` — pre-seed `.state.toml.tmp.99999`; run `state_mutate`; orphan gone after.
- **V-UT-7:** `state_mutate_returns_lock_contended_after_backoff` — hold lock from a sibling thread for 500 ms; main thread's `state_mutate` returns `Error::StateLockContended` after ~320 ms.
- **V-UT-8:** `reconcile_drops_active_slug_with_missing_dir` — pre-seed state with active=`["ghost"]`; `load_state` returns `active=[]`.
- **V-UT-9:** `reconcile_drops_active_slug_with_phase_archived` — pre-seed task.toml with phase=Archived; `load_state` drops it.
- **V-UT-10:** `reconcile_drops_session_whose_focus_no_longer_active` — pre-seed orphan session; `load_state` drops.
- **V-UT-11:** `migrate_synthesize_handles_missing_developer_or_missing_current_independently`.
- **V-UT-12:** `session_resolve_round_trips_within_same_ppid`.
- **V-UT-13:** `session_cache_file_path_uses_project_hash_and_ppid`.
- **V-UT-14:** `session_release_removes_cache_file_idempotently`.
- **V-UT-15:** `gc_drops_session_with_missing_cache_file`.
- **V-UT-16:** `gc_drops_session_with_mismatched_uuid`.
- **V-UT-17:** `gc_keeps_session_with_matching_cache`.
- **V-UT-18:** `task_resume_invalid_slug_returns_task_not_found`.
- **V-UT-19:** `task_resume_sets_session_focus_idempotently`.
- **V-UT-20:** `task_discard_template_unchanged_succeeds_without_force` — fresh `task new`; no edits; `task_discard` works.
- **V-UT-21:** `task_discard_edited_prd_refuses_without_force` — fresh `task new`; mutate PRD; `task_discard` returns `Error::TaskStillActive`.
- **V-UT-22:** `task_discard_with_force_removes_edited_task`.
- **V-UT-23:** `task_discard_archived_task_returns_task_not_found`.

[**Integration Tests**]

- **V-IT-1:** `multi_session_focus_isolation_across_two_sessions` — two distinct simulated PPIDs each run `task new`; `state_mutate` records two sessions; each `--slug`-less command resolves to its own focus.
- **V-IT-2:** `task_new_warns_on_non_empty_active_set` — first `task new` silent; second `task new` writes to stderr a warning containing "1 active task".
- **V-IT-3:** `migration_e2e_via_task_new` — pre-seed `.developer` + `.current`; run `task new <other>`; verify `.state.toml` shape and legacy files gone.
- **V-IT-4:** `worktree_isolation_state_file` — `task new --worktree --slug w1` from parent; parent's `.state.toml` has `active=[]`; worktree's has `active=["w1"]`.
- **V-IT-5:** `archive_clears_focus_and_releases_cache` — `task new`; `task archive`; verify state file has no session entry for self and no active entry for slug; verify cache file deleted.

[**Failure / Robustness Validation**]

- **V-F-1:** `crash_mid_write_leaves_either_old_or_new_state_never_partial` — simulated by atomically corrupting tmp file mid-write (skipped if hard to test; rely on rename atomicity guarantee).
- **V-F-2:** `lock_contention_resolves_within_backoff_window` — two threads, each with 50 ms inner sleep; both succeed.
- **V-F-3:** `lock_contention_beyond_backoff_returns_state_lock_contended` — hold lock 500 ms; victim returns error.
- **V-F-4:** `archive_rename_failure_state_says_inactive_dir_remains_until_reconcile` — inject rename failure; next `load_state` drops stale active entry.

[**Edge Case Validation**]

- **V-E-1:** `state_mutate_with_concurrent_load_state_does_not_corrupt` — reader during write sees either old or new; never partial.
- **V-E-2:** `empty_active_set_serializes_to_omitted_field_per_skip_serializing_if`.
- **V-E-3:** `dedup_preserves_first_occurrence_order_after_sort` — `active=["b", "a", "b"]` → on save → `["a", "b"]`.
- **V-E-4:** `gc_handles_session_with_pid_that_belongs_to_unrelated_process` — pre-seed cache with mismatched UUID; gc drops.
- **V-E-5:** `discard_with_no_seeded_files_present_succeeds_without_force` — task dir exists but PRD/PLAN deleted; succeeds (no diff-scan target).

[**Acceptance Mapping**]

| Goal / Constraint | Validation                                  |
|-------------------|---------------------------------------------|
| G-1               | V-UT-1, V-UT-4, V-UT-5                      |
| G-2               | V-IT-1, V-IT-4, V-UT-12                     |
| G-3               | V-UT-18, V-UT-19, V-UT-20, V-UT-21, V-UT-22, V-UT-23 |
| G-4               | (existing identity tests still pass)        |
| G-5               | V-UT-2, V-UT-3, V-UT-5, V-IT-3              |
| G-6               | V-UT-4, V-UT-6, V-UT-7, V-F-1, V-F-2, V-F-3 |
| G-7               | V-UT-15, V-UT-16, V-UT-17, V-IT-1           |
| G-8               | V-IT-2                                      |
| G-9               | V-IT-4                                      |
| G-10              | V-UT-18, dispatch test in agent_lifecycle   |
| G-11              | V-UT-20, V-UT-21, V-UT-22, V-UT-23          |
| C-1, C-2          | V-UT-7, V-F-2, V-F-3                        |
| C-3               | V-UT-4, V-F-1                               |
| C-4               | V-UT-6                                      |
| C-5               | V-UT-13                                     |
| C-9               | V-UT-8, V-UT-9, V-UT-10                     |
| C-10              | V-UT-15, V-UT-16, V-UT-17                   |
| C-11              | V-UT-3                                      |
| C-13              | V-UT-20, V-UT-21, V-UT-22, V-E-5            |
| C-15              | V-E-3                                       |
| C-17              | V-IT-4                                      |
