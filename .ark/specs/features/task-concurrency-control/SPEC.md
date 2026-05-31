[**Goals**]

- G-1: Per-checkout `.ark/.state.toml` carries the active-task set and a single `focus` slug.
- G-2: `task new --slug X` and `task resume --slug X` write `state.focus = Some(X)`; `task archive`/`task discard` clear it iff slug matches.
- G-3: `ark agent task resume <slug>` and `ark agent task discard <slug>` extend the agent task verb set.
- G-4: Legacy `.ark/tasks/.current` is auto-migrated on first state-file mutation and then deleted.
- G-5: State-file writes are atomic across crash and concurrent writers (file lock + temp+rename).
- G-6: Each git worktree owns its own `.state.toml`; worktree enumeration goes through state.

[**Non-goals**]

- NG-1: No SessionStart hook integration in this task.
- NG-2: No cross-host coordination (NFS-shared `.ark/`).
- NG-3: No `.state.toml` capture into `.ark.db` snapshots.
- NG-4: No multi-shell concurrency feature; one focus per checkout. Multi-shell users use a worktree.

[**Architecture**]

```
crates/ark-core/src/
├── state/checkout/
│   ├── mod.rs                        (pub use; invariants doc)
│   ├── model.rs                      (StateFile { tasks, focus: Option<String> })
│   ├── io.rs                         (load_state, state_mutate; lock acquire/release;
│   │                                   atomic temp+rename write; orphan cache cleanup)
│   ├── reconcile.rs                  (add_missing + drop_stale + clear_stale_focus)
│   └── migrate.rs                    (synthesize_from_legacy + delete_legacy_files)
├── commands/agent/task/
│   ├── resume.rs                     (task_resume; sets state.focus)
│   ├── discard.rs                    (task_discard, --force, template-diff guard;
│   │                                   clears focus via clear_focus_for_slug)
│   ├── new.rs                        (writes state.focus on register_focus)
│   └── archive.rs                    (clear_focus_for_slug before rename — C-20)
├── commands/context/gather.rs        (focused slug via state.focus)
├── commands/agent/task/worktree/
│   ├── discovery.rs                  (enumerate via state.tasks.active per worktree)
│   └── list.rs                       (same)
├── commands/unload.rs                (skip set adds state_file/state_lock_file/tmp glob)
├── layout.rs                         (state_file(), state_lock_file();
│                                       STATE_FILE, STATE_LOCK_FILE consts)
├── error.rs                          (StateTomlCorrupt, StateLockContended,
│                                       TaskStillActive, NoFocus)
└── lib.rs                            (re-exports state, resume, discard)

crates/ark-cli/src/agent_cli.rs       (Resume/Discard subcommands; resolve_slug
                                        reads state.focus; no Ppid threading)
```

Module coupling (one-way):

```
state/checkout → io::PathExt, io::hash_bytes, layout, error
commands/agent/task/{new, archive, resume, discard} → state
commands/context/gather                              → state
commands/agent/task/worktree/{discovery, list}       → state
commands/unload                                       → state (path constants only)
```

Call graph for `task new --slug a --tier quick`:

```
task_new(opts)
  ├── validate_slug(&opts.slug)
  ├── if task_dir.exists() → Error::TaskAlreadyExists
  ├── task_dir.ensure_dir()
  ├── copy_template("PRD", &task_dir.join("PRD.md"))
  ├── build_task_toml(&opts).save(&task_dir)
  └── state_mutate(&layout, |state| {
        if !state.tasks.active.contains(&opts.slug) {
            state.tasks.active.push(opts.slug.clone());
        }
        state.focus = Some(opts.slug.clone());
        Ok(())
      })?
```

Call graph for `task archive --slug a` (rename-first):

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── clear_focus_for_slug(&layout, &opts.slug)?      (0) drop active + focus
  ├── task_dir.rename_to(&archive_path)               (1) rename
  ├── toml.phase = Archived; toml.archived_at = now;
  │     toml.save(&archive_path)                      (2) save Archived metadata
  └── if tier == Deep:                                (3) SPEC promotion
        ├── spec_extract(SpecExtractOptions { task_dir_override: Some(&archive_path), ... })
        └── spec_register(SpecRegisterOptions { ... })
```

Recovery: if rename or later steps fail, the next `load_state` runs reconcile — add-pass re-adds the slug from the surviving `tasks/<slug>/` directory; drop-pass and clear-focus are idempotent. SPEC integrity is durable; state integrity is recoverable. No corruption.

Call graph for `task worktree list`:

```
worktree_list()
  ├── git_worktree_list_porcelain()
  └── for each <wt> under <root>/<cfg.worktree_dir>/:
        ├── wt_layout = Layout::new(&wt)
        ├── state = state::load_state(&wt_layout)?        (includes reconcile)
        └── for slug in &state.tasks.active:
              ├── task_dir = wt_layout.task_dir(slug)
              ├── toml = TaskToml::load(&task_dir)?
              └── emit row
```

[**Data Structure**]

```rust
// crates/ark-core/src/state/checkout/model.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateFile {
    #[serde(default)]
    pub tasks: Tasks,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tasks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<String>,
}

// crates/ark-core/src/error.rs (additions)
#[error("state file `{path}` is corrupt: {source}")]
StateTomlCorrupt { path: PathBuf, #[source] source: toml::de::Error },

#[error("state file `{path}` is locked by another process; gave up after backoff")]
StateLockContended { path: PathBuf },

#[error("task `{slug}` has user content in {file}; pass --force to discard anyway")]
TaskStillActive { slug: String, file: String },

#[error(
    "no focus set in `{}`; run `ark agent task new` or `task resume --slug <one-of>` to bind \
     this checkout (active: {})",
    project_root.display(),
    if candidates.is_empty() { "<none>" } else { candidates.join(", ") },
)]
NoFocus { project_root: PathBuf, candidates: Vec<String> },
```

On-disk shape:

```toml
focus = "foo"          # omitted entirely when None

[tasks]
active = ["foo", "bar"]
```

[**API Surface**]

```rust
// state/checkout/io.rs
pub fn load_state(layout: &Layout) -> Result<StateFile>;
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
    where F: FnOnce(&mut StateFile) -> Result<()>;

/// Drop `slug` from active and clear `state.focus` iff it pointed at `slug`.
pub fn clear_focus_for_slug(layout: &Layout, slug: &str) -> Result<()>;

// state/checkout/reconcile.rs
/// One-way: enumerate task dirs, add missing actives, drop stale, sort+dedup,
/// then clear `state.focus` if its target is no longer in `tasks.active`.
pub fn reconcile_against_disk(layout: &Layout, state: &mut StateFile) -> Result<()>;

// commands/agent/task/resume.rs
pub fn task_resume(opts: TaskResumeOptions) -> Result<TaskResumeSummary>;

// commands/agent/task/discard.rs
pub fn task_discard(opts: TaskDiscardOptions) -> Result<TaskDiscardSummary>;

// Library re-exports from crates/ark-core/src/lib.rs
pub use state::{
    StateFile, Tasks,
    clear_focus_for_slug, load_state, reconcile_against_disk, state_mutate,
};
pub use commands::agent::task::{
    TaskDiscardOptions, TaskDiscardSummary,
    TaskResumeOptions, TaskResumeSummary,
    task_discard, task_resume,
};
```

CLI subcommand additions:

```
ark agent task resume  --slug <s>
ark agent task discard --slug <s> [--force]
```

[**Constraints**]

- C-1: @test-binding: state_mutate_round_trips_atomically
Locking primitive is stdlib `File::try_lock` (Rust 1.89+).
- C-2: @test-binding: state_mutate_returns_lock_contended_after_backoff
Lock file is `.ark/.state.toml.lock`; backoff: 5 attempts at 10/20/40/80/160 ms (≤ 320 ms cumulative).
- C-3: @test-binding: state_mutate_round_trips_atomically
State write is atomic: `.state.toml.tmp.<pid>` then `rename_to(.state.toml)`.
- C-4: @test-binding: state_mutate_unlinks_orphan_tmp_files
`state_mutate` unlinks `.state.toml.tmp.*` orphans on lock acquire.
- C-5: @test-binding: cleanup_orphan_session_caches_removes_only_this_projects_files
Every successful `state_mutate` runs a best-effort sweep that unlinks `<temp>/ark-session-<this-project-hash>-*.id` orphans (legacy session-cache scheme; IO errors swallowed).
- C-6: @judgment
No `.unwrap()` in production; one allowed `.expect("StateFile serializes")` mirrors `TaskToml::save`.
- C-7: @judgment
All filesystem access in `state/checkout/` routes through `io::PathExt`.
- C-8: @judgment
All `.ark/`-relative paths route through `Layout` helpers. New consts `STATE_FILE`, `STATE_LOCK_FILE`. Legacy `tasks_current()` accessor kept for migration.
- C-9: @test-binding: drops_active_slug_with_missing_dir
Reconcile drops a `tasks.active` entry when its `task.toml` is missing or `phase == Archived`.
- C-10: @test-binding: cleanup_orphan_session_caches_removes_only_this_projects_files
Reconcile clears `state.focus` when its target is no longer in `tasks.active`.
- C-11: @test-binding: load_state_does_not_delete_legacy_on_pure_read
`load_state` does NOT delete legacy files; migration deletion happens on the next successful `state_mutate`.
- C-12: @judgment
`state_mutate` is the sole path that mutates the state file.
- C-13: @test-binding: first_diverging_artifact
`task_discard` reads each seeded file and compares to its template; first divergence → `Error::TaskStillActive`. `--force` skips the scan.
- C-14: @judgment
Legacy accessors carry a doc comment "legacy migration accessor; remove after migration window."
- C-15: @test-binding: add_then_drop_order_is_idempotent
`[tasks].active` is sorted+deduped on every save.
- C-16: @judgment
No SessionStart hook integration.
- C-17: @judgment
Each worktree's state file is independent.
- C-18: @judgment
`task_new` and `task_resume` set `state.focus = Some(slug)` in the same `state_mutate` that updates `tasks.active`. Their summaries carry `overwrote_focus: Option<String>` populated when the prior focus differed from the new slug; `Display` renders a stderr-rendered warning suggesting `--worktree` for parallel work.
- C-19: @test-binding: archive_rename_failure_recovery_via_reconcile
`task_archive`, `task_discard`, and `task_commit` set `state.focus = None` iff `state.focus.as_deref() == Some(slug)`. `task_commit` does NOT remove the slug from `tasks.active` (the slug stays active until `ark archive`).
- C-20: @judgment
`task_archive` ordering is rename-first: (0) `clear_focus_for_slug`; (1) rename to archive path; (2) save Archived metadata; (3) deep-tier `spec_extract + spec_register`. State integrity recoverable via reconcile if any step after (0) fails.
- C-21: @test-binding: task_new_first_task_does_not_warn
`unload.rs` skip set in BOTH walk sites: `[cfg.resolve_worktrees_dir(&layout), layout.state_file(), layout.state_lock_file()]`. `walk_files_excluding` extended to skip `.state.toml.tmp.*` orphans.
- C-22: @test-binding: task_archive_of_focused_clears_focus
`reconcile_against_disk` runs in order: (1) enumerate `.ark/tasks/<slug>/task.toml` excluding `archive/`, push to `state.tasks.active` if absent; (2) drop `tasks.active` entries whose `task.toml` is missing or `phase == Archived`; (3) sort+dedup per C-15; (4) clear `state.focus` if its target is no longer active. Order matters: add precedes drop so a transient inconsistency never collapses an active slug.
- C-23: @test-binding: resolve_focus_slug_precedence
`--slug` is required only on `task new`, `task resume`, `task discard`. Other verbs read `state.focus`; absent → `Error::NoFocus { project_root, candidates: state.tasks.active.clone() }`.
- C-24: @judgment
Loading a `.state.toml` carrying legacy `[sessions.*]` blocks does not error; serde drops unknown fields by default and the next save writes the file without them.

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
- 2026-05-08 `session-focus-bind`: replaced per-session focus map with a single per-checkout `[focus]` field. Deleted the `session/` module (`Ppid` trait, `RealPpid`, `StubPpid`, `SessionId`, `cache_file_path`, `cache_matches`, `resolve_session_id`, `release_session_id`, `lookup_session_id`) and `prune_dead_sessions`. `state_mutate` and `load_state` lose their `&dyn Ppid` parameter; the `Session` struct and `BTreeMap<String, Session>` field are gone. Added `Error::NoFocus { project_root, candidates }`; removed `Error::NoActiveTask` and `Error::AmbiguousActiveTask`. Reconcile now clears `state.focus` when its target leaves the active set instead of pruning sessions. Added one-shot best-effort `$TMPDIR` cleanup of legacy `ark-session-*.id` files. `task_commit` now releases focus on success (slug stays in `tasks.active` until `ark archive`). `task_new` and `task_resume` carry `overwrote_focus: Option<String>` and warn in `Display` when rebinding, suggesting `--worktree` for parallel work. C-1 through C-24 rewritten to match.
