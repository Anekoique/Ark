# `session-focus-bind` PLAN

> Status: Draft
> Feature: `session-focus-bind`
> Iteration: 0
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Replace the per-session focus map (`[sessions.*]`) and the topology cascade in `resolve_slug` with a single per-checkout `[focus] slug` field. Delete the entire `session/` module (`Ppid` trait, cache file, UUID plumbing). `task new` / `task resume` write `state.focus`; `task archive` / `task discard` clear it when the cleared slug equals the focus. All other verbs read `state.focus`; absent → `Error::NoFocus { project_root, candidates }`. One-shot best-effort cleanup unlinks orphan `$TMPDIR/ark-session-<this-project-hash>-*.id` files on first new-code `state_mutate`. Net deletion of ~600 LOC across 8 files, plus all `session/` module tests.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: `.state.toml` carries one optional `[focus] slug` per checkout.
- G-2: `task new`/`task resume` set focus; `task archive`/`task discard`/`task commit` clear it iff slug matches.
- G-3: Non-targeted verbs resolve via `state.focus`; absent → `Error::NoFocus`.
- G-4: `session/` module, `Ppid` trait, and `[sessions.*]` map are removed from `ark-core`.
- G-5: First new-code `state_mutate` unlinks orphan `$TMPDIR/ark-session-<hash>-*.id` files for this checkout.
- G-6: `task new` and `task resume` warn when they overwrite an existing focus, suggesting `--worktree` for parallel work.

[**Non-goals**]

- NG-1: No multi-shell concurrency feature; one focus per checkout, period.
- NG-2: No SessionStart hook changes.

[**Architecture**]

```
crates/ark-core/src/
├── state/checkout/
│   ├── model.rs                (StateFile.focus: Option<String>; Session struct DELETED;
│   │                             BTreeMap import DELETED)
│   ├── io.rs                   (load_state/state_mutate lose &dyn Ppid; clear_focus_for_slug
│   │                             becomes pure mutation; orphan-cache-cleanup pass added)
│   ├── reconcile.rs            (drop prune_dead_sessions; reconcile loses session-orphan
│   │                             pass; only active-set add/drop remains)
│   └── migrate.rs              (unchanged; only handles legacy `.current` file)
├── session/                    (DELETED — entire module)
├── commands/agent/task/
│   ├── new.rs                  (write state.focus; drop ppid threading; drop had_other_active)
│   ├── resume.rs               (write state.focus; drop ppid threading)
│   ├── archive.rs              (clear state.focus iff matches; drop ppid + release_session_id)
│   └── discard.rs              (clear state.focus iff matches; drop ppid threading)
├── commands/context/gather.rs  (focused slug = state.focus.clone(); drop lookup_session_id +
│                                  RealPpid construction)
├── error.rs                    (add NoFocus; remove NoActiveTask, AmbiguousActiveTask)
└── lib.rs                      (drop session::* re-exports; drop Session, prune_dead_sessions)

crates/ark-cli/src/
└── agent_cli.rs                (resolve_slug(&Path) -> Result<String> reads state.focus;
                                  drop ppid threading from dispatch)

crates/ark-cli/tests/agent_lifecycle.rs   (drop ppid plumbing; new tests for focus binding)
```

Module coupling after this change:

```
state::checkout → io::PathExt, layout, error                  (no longer pulls session)
commands/agent/task/{new,resume,archive,discard} → state, layout, error (no session)
commands/context/gather → state                               (no session)
commands/agent/task/worktree/{discovery,list} → state         (already; unchanged)
```

Call graph for `task plan` (representative non-targeted verb):

```
agent_cli::dispatch
  ├── layout = Layout::new(&root)
  ├── slug = resolve_slug(&root)?                  (reads state.focus; NoFocus if None)
  └── task_phase(TaskPhaseOptions { slug, ... })

resolve_slug(root)
  ├── state = load_state(&Layout::new(root))?
  └── state.focus.clone().ok_or(Error::NoFocus {
        project_root: root.into(),
        candidates: state.tasks.active.clone(),
      })
```

Call graph for `task new --slug X --tier T`:

```
task_new(opts)
  ├── validate_slug(&opts.slug)
  ├── if task_dir.exists() → Error::TaskAlreadyExists
  ├── task_dir.ensure_dir()
  ├── copy_template("PRD", ...)
  ├── build_task_toml(&opts).save(&task_dir)
  └── state_mutate(&layout, |state| {
        if !state.tasks.active.contains(&opts.slug) {
            state.tasks.active.push(opts.slug.clone());
        }
        state.focus = Some(opts.slug.clone());
        Ok(())
      })?
```

Call graph for `task archive` (rename-first, focus cleanup):

```
task_archive(opts)
  ├── load TaskToml from task_dir
  ├── check_transition(tier, phase, Archived)
  ├── task_dir.rename_to(&archive_path)
  ├── toml.phase = Archived; toml.archived_at = now; toml.save(&archive_path)
  ├── if tier == Deep: spec_extract + spec_register
  └── state_mutate(&layout, |state| {
        state.tasks.active.retain(|s| s != &opts.slug);
        if state.focus.as_deref() == Some(opts.slug.as_str()) {
            state.focus = None;
        }
        Ok(())
      })?
```

[**Data Structure**]

```rust
// state/checkout/model.rs
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

// DELETED: pub struct Session { pub focus: String, pub pid: u32 }
// DELETED: pub sessions: BTreeMap<String, Session>

// error.rs (additions)
#[error(
    "no focus set in `{}`; run `ark agent task new` or `task resume --slug <s>`",
    project_root.display()
)]
NoFocus {
    project_root: PathBuf,
    candidates: Vec<String>,        // populated for diagnostics; not in Display
}

// DELETED: NoActiveTask { project_root: PathBuf }
// DELETED: AmbiguousActiveTask { candidates: Vec<String> }
```

State-file on-disk shape:

```toml
[tasks]
active = ["foo", "bar"]

[focus]
slug = "foo"
```

When `state.focus = None`, the `[focus]` table is omitted entirely (via `skip_serializing_if`). Old `[sessions.*]` blocks in legacy files: `serde(default)` ignores unknown fields by default; we add `#[serde(rename = "sessions")] _sessions: Option<toml::Value>` only if needed to deserialize forward-compat — *judgment in implementation: try without, see if `serde_ignored` is default, fall back to a phantom field if needed.*

[**API Surface**]

```rust
// state/checkout/io.rs
pub fn load_state(layout: &Layout) -> Result<StateFile>;
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
    where F: FnOnce(&mut StateFile) -> Result<()>;

// signature change: drops &dyn Ppid from both.
// clear_focus_for_slug(layout, slug) → Result<()>: pure focus-mutation helper,
//     useful in archive/discard. (Old version went through session id; new is direct.)

// state/checkout/reconcile.rs
pub fn reconcile_against_disk(layout: &Layout, state: &mut StateFile) -> Result<()>;
//   Steps: (1) enumerate task dirs; (2) add missing actives; (3) drop missing/archived
//          actives; (4) sort+dedup; (5) clear state.focus if it points outside active.
// DELETED: prune_dead_sessions

// agent_cli.rs
fn resolve_slug(root: &Path) -> Result<String>;
//   Reads state.focus; returns Error::NoFocus when None.

// DELETED API:
//   pub mod session;
//   pub use session::{Ppid, RealPpid, StubPpid, SessionId, cache_file_path,
//                     cache_matches, lookup_session_id, resolve_session_id,
//                     release_session_id};
//   pub use state::Session;
//   pub use state::prune_dead_sessions;
```

[**Constraints**]

- C-1: `StateFile.focus: Option<String>`; `[focus]` table omitted on save when `None`.
- C-2: `task_new` and `task_resume` set `state.focus = Some(slug)` in the same `state_mutate` that updates `tasks.active`.
- C-3: `task_archive`, `task_discard`, and `task_commit` set `state.focus = None` iff `state.focus.as_deref() == Some(slug)`.
- C-3b: `task_new` and `task_resume` capture pre-mutate focus and surface a stderr-rendered warning in their `Display` summary when the new slug overwrites an existing different focus. The warning text suggests `--worktree` for parallel work.
- C-4: `resolve_slug` reads `state.focus`; `None` returns `Error::NoFocus { project_root, candidates: state.tasks.active.clone() }`.
- C-5: Reconcile clears `state.focus` when its target is no longer in `tasks.active`.
- C-6: `load_state` and `state_mutate` take no `Ppid`. Every call site updates.
- C-7: `crates/ark-core/src/session/` module is deleted along with all its tests.
- C-8: `Error::NoActiveTask` and `Error::AmbiguousActiveTask` are removed; no remaining matches in production code.
- C-9: First `state_mutate` after the upgrade scans `std::env::temp_dir()` for `ark-session-<this-project-hash>-*.id` and `remove_if_exists` each. IO errors logged via `tracing::debug!` and ignored.
- C-10: `task_new` no longer writes the "warn: N other active task(s)" message.
- C-11: All filesystem access in changed files routes through `io::PathExt`; no inline `std::fs::*`.

---

## Runtime

[**Main Flow**]

1. User runs `ark agent task new --slug foo --tier standard`. `state_mutate` adds `"foo"` to `tasks.active`, sets `focus = Some("foo")`. Cache cleanup pass (one-time, gated by absence of `[sessions.*]` legacy keys having been observed earlier — judgment: just always run the cleanup pass; cost is one `read_dir` of `$TMPDIR`).
2. User runs `ark agent task plan` (or any non-targeted verb). `resolve_slug(&root)` calls `load_state`; `state.focus = Some("foo")`; returns `"foo"`; verb proceeds.
3. User runs `ark agent task commit -m "..."`. Phase advances to `Committed`. Focus stays on `"foo"` because commit doesn't archive.
4. User runs `ark archive`. Each archived task: `state_mutate` removes from `active`, clears `focus` iff matches. After archive, `state.focus = None`.
5. User runs `ark agent task plan` with no focus → `Error::NoFocus`. User runs `ark agent task resume --slug bar` to bind, or `ark agent task new`.

[**Failure Flow**]

1. Verb run with no focus → `Error::NoFocus { project_root, candidates }`. Display tells user to run `task new` or `task resume`. CLI exit code 1.
2. Reconcile observes `state.focus = Some("ghost")` but `"ghost"` not on disk → clears focus to `None`. Next verb errors with `NoFocus` (recovered cleanly).
3. `state_mutate` lock contention → `Error::StateLockContended` (existing behavior, unchanged).
4. Cache cleanup pass IO error → logged at debug; `state_mutate` continues.

[**State Transitions**]

- `state.focus = None` → `Some(slug)` on `task new --slug slug` or `task resume --slug slug`.
- `state.focus = Some(slug)` → `None` on `task archive` or `task discard` of `slug`.
- `state.focus = Some(slug)` → `Some(slug')` on `task resume --slug slug'`.
- `state.focus = Some(ghost)` (pointing to slug not in `tasks.active`) → `None` on next `load_state` (reconcile).

---

## Implementation

[**Phase 1 — model + state machinery**]

1. `state/checkout/model.rs`: replace `sessions` field with `focus: Option<String>`. Delete `Session` struct. Delete `BTreeMap` import.
2. `state/checkout/io.rs`:
   - drop `&dyn Ppid` from `load_state`, `state_mutate`, `write_atomic`, `clear_focus_for_slug`.
   - rewrite `clear_focus_for_slug(layout, slug) -> Result<()>`: pure `state_mutate` clearing `state.focus` iff matches.
   - drop `lookup_session_id` and `release_session_id` imports/use.
   - add `cleanup_orphan_session_caches(layout)` helper called after `write_atomic`. Best-effort; debug-log errors.
3. `state/checkout/reconcile.rs`:
   - delete `prune_dead_sessions` and the `cache_matches` import.
   - rewrite step (4)/(5) of `reconcile_against_disk`: replace session-orphan + prune passes with a single `if state.focus.as_ref().map_or(false, |f| !state.tasks.active.contains(f)) { state.focus = None; }`.
   - delete `prune_dead_sessions_runs_after_orphan_drop` test; rewrite `drops_session_whose_focus_no_longer_active` to the new shape.
4. `state/checkout/mod.rs` and `state/mod.rs`: drop re-exports of `Session`, `prune_dead_sessions`. Drop `synthesize_from_legacy` / migrate exports if untouched (verify they don't reference sessions).
5. `error.rs`: add `NoFocus { project_root, candidates }`. Delete `NoActiveTask` and `AmbiguousActiveTask` variants.

[**Phase 2 — delete `session/` module**]

1. `rm -r crates/ark-core/src/session/`.
2. `lib.rs`: delete `pub mod session;` and the `pub use session::*` re-exports. Delete `pub use state::Session` and `pub use state::prune_dead_sessions`.
3. Update doctests in `lib.rs` if any reference the deleted symbols.

[**Phase 3 — verb call sites**]

1. `commands/agent/task/new.rs`: drop `&dyn Ppid` parameter from `task_new` and any `task_new_with_ppid` test seam. Drop `had_other_active` / `eprintln!` warning. Change session-mutate block to set `state.focus = Some(opts.slug.clone())`.
2. `commands/agent/task/resume.rs`: drop `task_resume_with_ppid`. Change inner mutate to `state.focus = Some(opts.slug.clone())`.
3. `commands/agent/task/archive.rs`: drop ppid threading. Change cleanup mutate to `if state.focus.as_deref() == Some(opts.slug.as_str()) { state.focus = None; }`.
4. `commands/agent/task/discard.rs`: same change as archive.
5. `commands/agent/task/worktree/{discovery,list}.rs`: `load_state` calls drop the `&ppid` argument.
6. `commands/context/gather.rs`: rewrite focused-slug derivation to `state.focus.clone()` (drop `lookup_session_id`, `RealPpid::new()`, the early-return-on-no-id branch).
7. `agent_cli.rs`: `resolve_slug` becomes `fn resolve_slug(root: &Path) -> Result<String>` reading `state.focus`. Dispatch drops `RealPpid` construction. CLI no longer matches `Error::AmbiguousActiveTask` / `Error::NoActiveTask`; matches `Error::NoFocus` to print the candidate list.

[**Phase 4 — tests**]

1. `state/checkout/io.rs::tests`: drop `StubPpid` arguments from every call. Add `cleanup_orphan_session_caches_unlinks_stale` integration test (planting a fake `ark-session-<hash>-12345.id` and asserting it disappears after `state_mutate`).
2. `state/checkout/reconcile.rs::tests`: drop `prune_dead_sessions_runs_after_orphan_drop`. Rewrite `drops_session_whose_focus_no_longer_active` as `clears_focus_when_focus_no_longer_active`.
3. `commands/agent/task/{new,resume,archive,discard}.rs::tests`: drop ppid plumbing; assert `state.focus` after each verb.
4. `commands/agent/task/concurrency_tests.rs`, `new_tests.rs`: update to no-ppid signatures.
5. `commands/context/gather.rs::tests`: rewrite the focused-slug test to populate `state.focus` directly.
6. `agent_cli.rs::tests`: replace all 8 cascade-related tests with `resolve_slug_returns_focus` and `resolve_slug_errors_no_focus_when_unset`.
7. `crates/ark-cli/tests/agent_lifecycle.rs`: drop ppid; cover `new → plan` (focus auto-set), `archive → plan` (NoFocus error), `resume → plan` (focus restored).

[**Phase 5 — workflow doc + SPEC updates**]

1. `.ark/workflow.md`: rewrite §"Session model" → "Focus model"; failure-modes table swaps `NoActiveTask`/`AmbiguousActiveTask` for `NoFocus`; delete cascade paragraph.
2. `specs/features/ark-agent-namespace/SPEC.md`: rewrite C-14, drop `NoActiveTask`/`AmbiguousActiveTask` from Data Structure, add `NoFocus`. CHANGELOG entry.
3. `specs/features/task-concurrency-control/SPEC.md`: substantial rewrite per PRD's Related Specs section. CHANGELOG entry.
4. `specs/features/workspace/SPEC.md`: scan for `[sessions.*]` / `Ppid` references; CHANGELOG entry only if the SPEC body needs no body change.

[**Phase 6 — manual cleanup of dirty state**]

The current `.state.toml` carries 17 stale `[sessions.*]` entries from the old code. After the new binary builds, run `ark context --scope session` once — first `state_mutate` rewrites the file without sessions and runs the orphan-cache cleanup. No manual edit needed; verify by `cat .ark/.state.toml`.

---

## Trade-offs

- T-1: **Per-checkout focus vs per-session focus.** Per-session was meant to support multiple shells working on different tasks in one checkout. The workflow never actually requires this — when a deep task wants its own focus, it materializes a worktree (separate `.state.toml`). Per-checkout deletes ~600 LOC of plumbing and works correctly under AI harnesses where PPID is unstable. Cost: a hypothetical multi-shell user can't have two focuses in one checkout. Judgment: that user should use a worktree.
- T-2: **Delete `Ppid` trait vs keep it for telemetry.** The trait could record session-touched-at timestamps even without focus. Deleting is cleaner; if telemetry is later wanted, add a separate `last_touched_at: DateTime<Utc>` field on `StateFile`.
- T-3: **One-shot `$TMPDIR` cleanup vs leave files orphaned.** The 26 stale cache files in `$TMPDIR` would otherwise survive until OS reaping. Cleanup is cheap (one `read_dir`, `O(N)` `unlink`) and runs once on the first new-code `state_mutate`. Cost: very brief IO on first call after upgrade. Worth it; it leaves no traces of the old scheme.
- T-4: **`Error::NoFocus` carries `candidates` vs minimal.** Carrying the active-set lets the CLI print "candidates: [foo, bar]" in the error message body, making `task resume` ergonomic. Cost: tiny serde footprint; not exposed in `thiserror`'s Display (kept on the struct for the CLI to read).

---

## Validation

[**Unit Tests**]

- V-UT-1: `StateFile` serde round-trip with `focus = Some("foo")` and `focus = None`.
- V-UT-2: `state_mutate` setting `focus` persists across `load_state`.
- V-UT-3: `reconcile_against_disk` clears `focus` pointing to a missing slug.
- V-UT-4: `task_new` sets `focus` to the new slug.
- V-UT-5: `task_resume` overwrites `focus`.
- V-UT-6: `task_archive` clears `focus` iff slug matches; leaves alone otherwise.
- V-UT-7: `task_discard` clears `focus` iff slug matches.
- V-UT-8: `resolve_slug` returns `Ok(slug)` when focus set; returns `Err(NoFocus)` when `None`.
- V-UT-9: `cleanup_orphan_session_caches` unlinks `ark-session-<hash>-*.id` for this project; ignores other projects' files.

[**Integration Tests**]

- V-IT-1: `agent_lifecycle::new_then_plan_uses_focus` — `task new --slug foo` → `task plan` advances foo.
- V-IT-2: `agent_lifecycle::archive_clears_focus_then_plan_errors` — `task new` → `task commit` → `task archive` → `task plan` errors `NoFocus`.
- V-IT-3: `agent_lifecycle::resume_rebinds_focus` — multiple actives; `task resume --slug bar` → `task plan` advances bar.
- V-IT-4: `agent_lifecycle::legacy_state_file_with_sessions_is_accepted` — plant a `.state.toml` with `[sessions.*]` blocks; first `state_mutate` strips them.

[**Failure / Robustness**]

- V-F-1: Cache cleanup pass with read-only `$TMPDIR` does not fail the verb.
- V-F-2: Reconcile during a partial archive (active still listed; dir already renamed) clears focus correctly.
- V-F-3: Loading a `.state.toml` with `[focus]` but no `slug` key returns `focus = None` (default).

[**Edge Cases**]

- V-E-1: `task new --slug foo` when `state.focus = Some("bar")` overwrites focus to `"foo"` (per PRD).
- V-E-2: `task discard --slug foo` when `state.focus = Some("bar")` leaves focus on `"bar"`.
- V-E-3: Empty `tasks.active` and `focus = None` → `resolve_slug` errors with `candidates = []`.
- V-E-4: `state.focus = Some("")` (empty string) — reject at `state_mutate` write side via `Option::filter(|s| !s.is_empty())`. Belt-and-braces.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-UT-2, V-F-3 |
| G-2 | V-UT-4, V-UT-5, V-UT-6, V-UT-7, V-IT-1, V-IT-3 |
| G-3 | V-UT-8, V-IT-2 |
| G-4 | (compile-time: tests in `session/` are deleted; cargo build green) |
| G-5 | V-UT-9, V-IT-4, V-F-1 |
| C-1 | V-UT-1 |
| C-2 | V-UT-4, V-UT-5 |
| C-3 | V-UT-6, V-UT-7, V-E-2 |
| C-4 | V-UT-8, V-E-3 |
| C-5 | V-UT-3, V-F-2 |
| C-6 | (compile-time: signatures in API Surface) |
| C-7 | (compile-time: `rm -r src/session/`) |
| C-8 | (compile-time: grep for variant names is empty) |
| C-9 | V-UT-9, V-IT-4, V-F-1 |
| C-10 | V-IT-1 (asserts no warning on stderr) |
| C-11 | (review: `grep -n "std::fs::" src/**` shows no inline use) |
