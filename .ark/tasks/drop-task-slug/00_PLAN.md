# `drop-task-slug` PLAN `00`

> Status: Draft
> Feature: `drop-task-slug`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Drop `--slug` from the seven non-targeted task verbs (`plan|review|execute|verify|commit|promote|archive`), replace `resolve_slug`'s session-id-driven body with a topology-first cascade (worktree path → single active → session focus), and split the misleading `Error::NoCurrentTask` into `Error::NoActiveTask` + `Error::AmbiguousActiveTask`. `resume`/`discard` keep `--slug` as the only required-target verbs. Two related-feature SPECs are revised to match.

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: `ark agent task {plan, review, execute, verify, commit, promote, archive}` and `ark agent spec extract` and `ark agent task worktree cleanup` no longer accept `--slug`. Field removed from each `*CliArgs` struct in `crates/ark-cli/src/agent_cli.rs`. `ark agent task <verb> --slug X` exits with clap's "unexpected argument" error.
- G-2: `ark agent task resume <slug>` and `ark agent task discard <slug>` keep `--slug` exactly as today (`resume` requires it; `discard` accepts it as `Option<String>` with the same fallback chain as the other verbs — see G-4).
- G-3: A new `resolve_slug(root) -> Result<String>` function (no `explicit: Option<String>` parameter; it never had any other override) replaces the existing one. Resolution order:
  1. **Worktree topology.** If `Layout::new(root).slug_from_worktree_root()` returns `Some(slug)` AND a task dir for that slug exists at `<root>/.ark/tasks/<slug>/`, return it.
  2. **Single active task.** Read `state.tasks.active` via `load_state(&layout, &RealPpid::new())`; if `len() == 1`, return that slug.
  3. **Session focus.** If `state.sessions.<this-session>.focus` resolves to a non-empty slug present in `state.tasks.active`, return it.
  4. Error per G-5.
- G-4: `discard` accepts the same fallback chain (it already takes `Option<String>` today). Architecturally `discard` could keep mandatory-slug semantics (it deletes data), but its current shape is `Option<String>` with `--force` for the destructive guard, so we keep the cascade for consistency. The `--force` guard remains the actual deletion safeguard.
- G-5: `Error::NoCurrentTask` is removed. Two replacement variants land in `crates/ark-core/src/error.rs`:
  - `Error::NoActiveTask { project_root: PathBuf }` — message: `` "no active task in `{project_root}`; run `ark agent task new` first" ``.
  - `Error::AmbiguousActiveTask { candidates: Vec<String> }` — message: `` "multiple active tasks: {candidates_joined}; `cd` into a worktree or run `ark agent task resume --slug <s>` to focus this session" ``. `candidates_joined` is `candidates.join(", ")`.
- G-6: The bug from the PRD reproduces as a regression test. Fixture: a `.state.toml` with two active slugs (`a`, `b`) and one `[sessions.<UUID>]` entry whose `<UUID>` does NOT match the session id `RealPpid` would resolve to. Caller's `cwd` is `<root>/.ark/worktrees/feat/b/`. Expected: `resolve_slug` returns `Ok("b")`. Without the fix, today's code returns `Err(NoCurrentTask)`.
- G-7: Both feature SPECs (`ark-agent-namespace`, `task-concurrency-control`) are updated in lockstep. Edits:
  - `ark-agent-namespace` SPEC: drop `[--slug <s>]` from the table rows for `plan|review|execute|verify|archive|promote` and `spec extract`; replace the "Every `--slug`-taking command defaults to `.ark/tasks/.current` … Missing `.current` → `Error::NoCurrentTask`" sentence with the topology-first cascade described in G-3 and the two new error variants.
  - `task-concurrency-control` SPEC: revise G-9 to: *"`--slug`-less commands resolve via topology cascade (worktree path → single active → session focus). Empty active set → `Error::NoActiveTask`. Multiple actives with no resolving topology → `Error::AmbiguousActiveTask`."*. Remove the `NoCurrentTask` reference; add `NoActiveTask` and `AmbiguousActiveTask` to the error-variants list.
- G-8: All slash-command template files under `templates/{claude,codex,opencode}/` already invoke the verbs without `--slug`; no template edits are required. (Verified via grep — no `--slug` argument is passed to any of the 7 affected verbs in any template.)
- G-9: Existing tests pass. `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` all green.

[**Non-goals**]

- NG-1: Public-CLI surface (`ark init|load|unload|remove|upgrade|context|archive`) is unchanged.
- NG-2: Write-side concurrency (`state_mutate`, session-id cache file lifecycle, `task new` warn-on-other-active) is unchanged. The session-id machinery survives intact for write paths.
- NG-3: The focus map's role inside `ark context` (showing "this session is focused on X") is unchanged. `ark context`'s own gather code already looks up focus by session id (`commands/context/gather.rs:323`); we do not touch it.
- NG-4: No new `Layout::slug_from_worktree_root` cousin for non-worktree task dirs. Topology detection only fires when `cwd` resolves to a worktree.
- NG-5: No deprecation period for the old flag. The hidden CLI is explicitly not semver-covered (per `ark-agent-namespace` SPEC NG-6); the templates that drive it are versioned with the binary.
- NG-6: No test changes for `task new` / `task resume` / `task discard` write paths beyond signature mechanics — the resolver is purely a read-side helper.

[**Architecture**]

```
crates/
├── ark-cli/src/
│   └── agent_cli.rs                       MOD: drop `slug: Option<String>` from
│                                                TaskSlugArgs, TaskCommitCliArgs,
│                                                TaskArchiveCliArgs, TaskPromoteCliArgs,
│                                                WorktreeCleanupCliArgs, SpecExtractCliArgs;
│                                                rewrite resolve_slug; drop the
│                                                `Option<String>` param at every callsite.
│                                                Discard keeps Option<String> + cascade.
└── ark-core/src/
    ├── error.rs                            MOD: remove NoCurrentTask;
    │                                              add NoActiveTask, AmbiguousActiveTask
    ├── lib.rs                              MOD: re-export new error variants if needed
    └── layout.rs                           MOD: add slug_from_worktree_root() helper.
.ark/specs/features/
├── ark-agent-namespace/SPEC.md             MOD: table rows + the fallback paragraph
└── task-concurrency-control/SPEC.md        MOD: G-9 prose + error-variant list
```

**Module coupling.** `agent_cli::resolve_slug` already imports `Layout`, `RealPpid`, `load_state`, `resolve_session_id`. After this task, it imports `Layout` only (for `slug_from_worktree_root` + `state_file`), `RealPpid`, `load_state`. `resolve_session_id` is no longer called from the read path; it remains used by `task new`/`task resume`/`task archive` write paths and by `commands/context/gather`.

**Call graph for `resolve_slug(root)`:**

```
resolve_slug(root: &Path) -> Result<String>
  ├── layout = Layout::new(root)
  ├── if let Some(slug) = layout.slug_from_worktree_root():
  │     └── if layout.task_dir(&slug).is_dir(): return Ok(slug)
  │        # else fall through — slug-from-path didn't match a real task
  ├── state = load_state(&layout, &RealPpid::new())?
  ├── match state.tasks.active.len():
  │     ├── 0 => Err(Error::NoActiveTask { project_root: root.to_owned() })
  │     ├── 1 => Ok(state.tasks.active[0].clone())
  │     └── _ =>
  │         ├── id = lookup_session_id(&layout, &RealPpid::new())?  // optional, never creates
  │         ├── if let Some(id) = id:
  │         │     if let Some(s) = state.sessions.get(id.as_str())
  │         │        .map(|s| &s.focus)
  │         │        .filter(|f| !f.is_empty() && state.tasks.active.contains(f)):
  │         │       return Ok(s.clone())
  │         └── Err(Error::AmbiguousActiveTask { candidates: state.tasks.active.clone() })
```

Note: step 3 uses `lookup_session_id` (read-only, returns `Option<SessionId>`) rather than `resolve_session_id` (which creates a cache entry on miss). This matches the `commands/context/gather` precedent and prevents the side effect from a read-only resolution.

[**Data Structure**]

```rust
// ark-core/src/error.rs — REMOVED:
//   Error::NoCurrentTask { path: PathBuf }
//
// ark-core/src/error.rs — ADDED:

#[error("no active task in `{project_root}`; run `ark agent task new` first")]
NoActiveTask { project_root: PathBuf },

#[error("multiple active tasks: {}; `cd` into a worktree or run `ark agent task resume --slug <s>` to focus this session", candidates.join(", "))]
AmbiguousActiveTask { candidates: Vec<String> },
```

```rust
// ark-core/src/layout.rs — ADDED:

impl Layout {
    /// If `self.root()` is a worktree-style path
    /// (`<parent>/.ark/worktrees/<branch-type>/<slug>`), returns `Some(slug)`.
    /// The check is purely lexical: it does NOT verify the dir exists or that
    /// the slug corresponds to a real task. Callers gate further by checking
    /// `self.task_dir(&slug).is_dir()`.
    ///
    /// Returns `None` when the root has fewer than three trailing components
    /// matching `.ark/worktrees/*` or when component validation fails.
    pub fn slug_from_worktree_root(&self) -> Option<String>;
}
```

```rust
// ark-cli/src/agent_cli.rs — REPLACED resolve_slug body:

fn resolve_slug(root: &Path) -> anyhow::Result<String> {
    let layout = Layout::new(root);
    let ppid = RealPpid::new();

    if let Some(slug) = layout.slug_from_worktree_root() {
        if layout.task_dir(&slug).is_dir() {
            return Ok(slug);
        }
    }

    let state = load_state(&layout, &ppid)?;
    match state.tasks.active.as_slice() {
        []     => Err(ark_core::Error::NoActiveTask { project_root: root.to_path_buf() }.into()),
        [one]  => Ok(one.clone()),
        many   => {
            let id = lookup_session_id(&layout, &ppid)?;
            if let Some(id) = id
                && let Some(focus) = state.sessions.get(id.as_str()).map(|s| &s.focus)
                && !focus.is_empty()
                && many.iter().any(|s| s == focus)
            {
                return Ok(focus.clone());
            }
            Err(ark_core::Error::AmbiguousActiveTask { candidates: many.to_vec() }.into())
        }
    }
}
```

[**API Surface**]

CLI arg structs lose the `slug: Option<String>` field on:
- `TaskSlugArgs` — backs Plan / Review / Execute / Verify
- `TaskCommitCliArgs`
- `TaskArchiveCliArgs`
- `TaskPromoteCliArgs`
- `WorktreeCleanupCliArgs`
- `SpecExtractCliArgs`

Field is preserved on:
- `TaskResumeCliArgs` — `slug: String` (required, unchanged)
- `TaskDiscardCliArgs` — `slug: Option<String>` (optional, fallback cascade unchanged)

Library re-exports in `crates/ark-core/src/lib.rs` change only by `Error` variants (added/removed). No public function signatures change.

Internal callsite signature change in `agent_cli.rs`:

```rust
// before: fn resolve_slug(root: &Path, explicit: Option<String>) -> anyhow::Result<String>
// after:  fn resolve_slug(root: &Path) -> anyhow::Result<String>
```

Every callsite (8 of them in `TaskCommand::dispatch` + `SpecCommand::dispatch` + `WorktreeSubcommand::Cleanup` + `discard`) drops the second arg.

[**Constraints**]

- C-1: `slug_from_worktree_root` is purely lexical. It MUST NOT touch the filesystem. It splits `self.root()` by component and matches the trailing pattern `[..., ".ark", "worktrees", <branch-type>, <slug>]` (slug is the last component). Returns `None` if the root has fewer than 4 components or any of the fixed components don't match. The `<branch-type>` slot is not validated against the `BRANCH_TYPES` allowlist — the type might have been added since the worktree was created, and we lean on the `task_dir(&slug).is_dir()` guard outside the helper. **The slug MUST NOT be empty**: if the trailing component is `""` (e.g. the path ends in a separator that produced an empty `Component::Normal`), return `None`.
- C-2: `slug_from_worktree_root` returns `Option<String>`, never an error. Path-component traversal failures (non-UTF-8 components) yield `None`.
- C-3: The new `resolve_slug` MUST call `lookup_session_id` (not `resolve_session_id`) for the session-focus branch. Verified by source-scan test: `agent_cli::resolve_slug` body must not contain the literal `resolve_session_id`.
- C-4: `Error::NoCurrentTask` is fully removed. `grep -rn 'NoCurrentTask' crates/` returns zero results post-task. CI will fail otherwise via the existing build.
- C-5: All filesystem access in `slug_from_worktree_root` and the new `resolve_slug` routes through `io::PathExt` (`is_dir` is on `PathExt`). Path composition routes through `Layout`.
- C-6: `Error::AmbiguousActiveTask`'s `candidates` is the verbatim `state.tasks.active` snapshot at the moment of failure, in its existing sort order (`state` is sorted+deduped by `state_mutate` per C-15 of `task-concurrency-control`).
- C-7: The two SPEC files are edited inline; managed-block markers untouched. SPEC body changes are committed in the same commit as the code change (deep-tier convention; this task is standard-tier so no SPEC promotion happens, but G-7 SPEC edits are still part of the same commit per `ark agent task commit` semantics — they ride the work tree of the task).
- C-8: The CLI snapshot tests in `crates/ark-cli/tests/cli_help.rs` and `crates/ark-cli/tests/agent_lifecycle.rs` are updated for the `--help` text changes if needed. Run `cargo test --workspace -- --nocapture` and update the snapshots if `insta` is in use; otherwise update assertions manually.

## Runtime `runtime logic`

[**Main Flow**]

1. User runs `ark agent task plan` (or any of the seven dropped-slug verbs) inside a worktree.
2. `agent_cli::TaskCommand::dispatch` invokes `run_phase(args, task_plan)`.
3. `run_phase` calls `resolve_slug(&root)`.
4. `Layout::slug_from_worktree_root` returns `Some("vdso-support")` from path `.../.ark/worktrees/feat/vdso-support`.
5. `layout.task_dir("vdso-support").is_dir()` returns `true` (the worktree's task dir exists).
6. Resolver returns `Ok("vdso-support")`. Phase transition proceeds.

[**Failure Flow**]

1. User runs `ark agent task plan` in the parent checkout with two active tasks and a stale session-id (e.g. previous `claude` exited).
2. Resolver: worktree path doesn't match (we're in parent root). `state.tasks.active.len() == 2`. `lookup_session_id` returns `Some(id)` but `state.sessions.get(id)` is `None` (stale UUID). Falls through to `Err(AmbiguousActiveTask { candidates: ["a", "b"] })`.
3. Error message printed: "multiple active tasks: a, b; `cd` into a worktree or run `ark agent task resume --slug <s>` to focus this session".
4. User remediation: either `ark agent task resume --slug a` (rewrites focus map for current session id) or `cd .ark/worktrees/feat/a/` and re-run.

[**State Transitions**]

- `Error::NoCurrentTask` → REMOVED (state transition: error variant retired)
- `Error::NoActiveTask` + `Error::AmbiguousActiveTask` → ADDED

## Implementation `split task into phases`

[**Phase 1: Core types + helper**]

1. Edit `crates/ark-core/src/error.rs`:
   - Remove `NoCurrentTask` variant.
   - Add `NoActiveTask { project_root: PathBuf }` and `AmbiguousActiveTask { candidates: Vec<String> }` with `thiserror` `#[error]` macros per Data Structure.
2. Edit `crates/ark-core/src/layout.rs`:
   - Add `slug_from_worktree_root(&self) -> Option<String>` per Data Structure.
   - Add unit tests: `slug_from_worktree_root_returns_some_for_canonical_path`, `_returns_none_for_parent_root`, `_returns_none_for_short_path`, `_returns_none_for_empty_trailing_component`.
3. `cargo build -p ark-core` to confirm compile. `cargo test -p ark-core layout` to confirm new helper tests pass.

[**Phase 2: Resolver rewrite + clap surface**]

1. Edit `crates/ark-cli/src/agent_cli.rs`:
   - Change `TaskSlugArgs` — remove `slug: Option<String>`. The struct now contains only `target: TargetArgs` (consider whether the struct can be replaced with just `TargetArgs` directly; keep the type for `--help` doc-comment continuity).
   - Same removal on `TaskCommitCliArgs`, `TaskArchiveCliArgs`, `TaskPromoteCliArgs`, `WorktreeCleanupCliArgs`, `SpecExtractCliArgs`.
   - Rewrite `fn resolve_slug(root: &Path) -> anyhow::Result<String>` per Data Structure.
   - Update every callsite in `TaskCommand::dispatch`, `SpecCommand::dispatch`, `WorktreeSubcommand` to drop the `Option<String>` arg.
   - Update imports: `lookup_session_id` is needed; `resolve_session_id` is no longer used in this file.
2. `cargo build --workspace` — should compile clean.
3. `cargo clippy --workspace --all-targets -- -D warnings`.

[**Phase 3: Tests**]

1. Add unit tests in `agent_cli.rs` (mod `tests`) for `resolve_slug`. Use `tempfile::tempdir` and synthesize `.ark/.state.toml` directly (the worktree topology fixture doesn't need a real git worktree — `slug_from_worktree_root` is lexical):
   - `resolve_slug_finds_slug_from_worktree_path` — root = `<tmp>/.ark/worktrees/feat/foo`, task dir at `<tmp>/.ark/worktrees/feat/foo/.ark/tasks/foo/` → returns `Ok("foo")`.
   - `resolve_slug_falls_through_when_worktree_slug_has_no_task_dir` — root = `<tmp>/.ark/worktrees/feat/ghost`, no task dir; state file empty → `Err(NoActiveTask)`.
   - `resolve_slug_returns_only_active_when_one` — non-worktree root, state has `active = ["only"]` → `Ok("only")`.
   - `resolve_slug_errors_no_active_when_state_empty` — non-worktree root, state empty → `Err(NoActiveTask)`.
   - `resolve_slug_errors_ambiguous_with_no_session_focus` — non-worktree root, state has 2 actives, no matching session → `Err(AmbiguousActiveTask)`.
   - `resolve_slug_uses_session_focus_when_ambiguous` — state has 2 actives + a session entry whose UUID matches `lookup_session_id`'s output → returns the focused slug. (Needs the cache file in `temp_dir` keyed by `(project_hash, RealPpid::new().parent_id())` — write it pre-call.)
   - `resolve_slug_regression_pr_repro` — explicit replay of the PRD bug: root = `<wt>/.ark/worktrees/feat/b`, state with stale session entry → `Ok("b")` via worktree path.
2. Add layout test in `crates/ark-core/src/layout.rs::tests`: `slug_from_worktree_root_*` per Phase 1.2.
3. `cargo test --workspace`.

[**Phase 4: SPEC + workflow doc updates**]

1. Edit `.ark/specs/features/ark-agent-namespace/SPEC.md`:
   - Table at lines 178–186: drop `[--slug <s>]` from the rows for `plan|review|execute|verify|archive|promote` and `spec extract`. Resume's row is unchanged. Discard isn't in this table (it landed via the concurrency-control SPEC).
   - Sentence at line 188 ("Every `--slug`-taking command defaults to … Missing `.current` → `Error::NoCurrentTask`."): replace with the topology cascade prose: *"`--slug` is required only on `task resume` (target by definition). Other verbs resolve via topology cascade — worktree path → single active → this session's focus map. Failures yield `Error::NoActiveTask` (empty active set) or `Error::AmbiguousActiveTask` (multiple actives, no resolver hit)."*
   - Error variants list at lines 117–129: replace `Error::NoCurrentTask { path: PathBuf }` with the two new variants.
2. Edit `.ark/specs/features/task-concurrency-control/SPEC.md`:
   - G-9 at line 12 ("`--slug`-less commands resolve to *this session's* focused slug. With no focus, return `Error::NoCurrentTask`."): replace with the topology-cascade text per G-7 of this PLAN.
   - Error-variants list (line 240): replace `TaskStillActive` block adjacency — drop `NoCurrentTask` if present elsewhere, add the two new variants.
3. Inspect `.ark/workflow.md` for any prose mentioning `--slug` defaults; update if found. (`grep --slug .ark/workflow.md` — the multi-session-focus paragraph at line 197 mentions `--slug` for `resume`/`discard`, which stays correct; no edits expected.)

[**Phase 5: Verification + commit**]

1. Run the four-gate check: `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`.
2. Run the E2E smoke from `AGENTS.md`: `cargo build --release; TMP=$(mktemp -d); ./target/release/ark load --dir "$TMP" && ./target/release/ark unload --dir "$TMP" && ./target/release/ark load --dir "$TMP" && ./target/release/ark remove --dir "$TMP"`.
3. Manual repro of the PRD bug: stage a state.toml with two active slugs and stale session, `cd` into a faked worktree path, run `ark agent task plan` — should now succeed without `--slug`.
4. Stage `crates/`, `.ark/specs/features/ark-agent-namespace/SPEC.md`, `.ark/specs/features/task-concurrency-control/SPEC.md`, `.ark/tasks/drop-task-slug/`. Run `/ark:commit -m "feat(agent): drop --slug from non-targeted task verbs"`.

## Trade-offs `ask reviewer for advice`

- T-1: **Should `discard` keep the cascade or require explicit slug?** Today: `Option<String>` + cascade + `--force` for content guard. *Adv. of cascade*: consistent with all other `Option<String>`-slug verbs; agent UX is symmetric. *Disadv.*: a destructive op picking up an implicit target via single-active or worktree-path is surprising — discarding the wrong task is unrecoverable (file-deletion is the explicit user intent, but the *target* was implicit). *Recommendation*: keep the cascade. The destructive guard is `--force`, not `--slug`. The cascade can never pick a slug the user isn't already operating on (single active = obvious; worktree-path = literally the cwd; focus = this session's chosen task). User typing `ark agent task discard` in a worktree means "discard *this* task" — which is what they'd want.

- T-2: **Lookup by `lookup_session_id` vs `resolve_session_id` in the focus branch.** *Adv. of `lookup`*: pure read, no side-effect cache file creation when no cache exists. *Disadv.*: if the user has just invoked a write verb that did create a cache, then a subsequent read invocation expects the cache to exist — `lookup` will find it. There is no scenario where `lookup` returns `None` but `resolve` would have returned the right id; `resolve` mints a fresh UUID on miss, which doesn't match any `state.sessions` entry anyway. So `lookup` is strictly better for read-side resolution. *Recommendation*: use `lookup`.

- T-3: **`Layout::slug_from_worktree_root` lexical vs filesystem-aware.** *Adv. of lexical*: deterministic, side-effect-free, fast, testable without git fixtures. *Disadv.*: a user who manually `mv`s a worktree dir or symlinks into it can spoof a slug. *Recommendation*: lexical with the `task_dir(&slug).is_dir()` external guard. The combination accepts canonical layouts and rejects spoofs (the task dir won't exist).

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Layout::slug_from_worktree_root` — canonical path returns `Some(slug)`. (Phase 1.2)
- V-UT-2: `Layout::slug_from_worktree_root` — non-worktree path returns `None`. (Phase 1.2)
- V-UT-3: `Layout::slug_from_worktree_root` — too-short path returns `None`. (Phase 1.2)
- V-UT-4: `Layout::slug_from_worktree_root` — trailing-separator / empty-component returns `None`. (Phase 1.2; covers C-1's empty-slug guard)
- V-UT-5: `resolve_slug` — worktree path with existing task dir returns the slug. (Phase 3.1)
- V-UT-6: `resolve_slug` — worktree path with no matching task dir falls through to state-based resolution. (Phase 3.1)
- V-UT-7: `resolve_slug` — non-worktree, single active, returns the only slug. (Phase 3.1)
- V-UT-8: `resolve_slug` — non-worktree, empty active, returns `NoActiveTask`. (Phase 3.1)
- V-UT-9: `resolve_slug` — non-worktree, ambiguous, no session focus, returns `AmbiguousActiveTask` with the active list as candidates. (Phase 3.1)
- V-UT-10: `resolve_slug` — non-worktree, ambiguous, valid session focus, returns the focused slug. (Phase 3.1)

[**Integration Tests**]

- V-IT-1: PRD-bug regression — synthesize the exact failure scenario (worktree path + stale session id + multi-active state) and assert `resolve_slug` returns `Ok("b")`. Lives in `agent_cli.rs::tests` since `resolve_slug` is a `fn`-private helper. (Phase 3.1)
- V-IT-2: `ark agent task plan` (no `--slug`) succeeds when invoked from inside a real worktree fixture. Add to `crates/ark-cli/tests/agent_lifecycle.rs` (or wherever the existing `assert_cmd`-style integration tests live).
- V-IT-3: `ark agent task plan --slug X` exits with clap's "unexpected argument" error post-removal. Add to `agent_lifecycle.rs`.

[**Failure / Robustness Validation**]

- V-F-1: Corrupt `.state.toml` propagates as `Error::StateTomlCorrupt` (existing variant) without crashing the resolver. Existing tests in `state_file::io::tests` already cover this; verify no new path bypasses them.
- V-F-2: Worktree path detection survives non-UTF-8 path components by returning `None` (per C-2). Add `slug_from_worktree_root_handles_non_utf8` if `OsStr::to_str().is_none()` codepath isn't already exercised.

[**Edge Case Validation**]

- V-E-1: Empty `state.tasks.active` + worktree path with no task dir → `NoActiveTask` (V-UT-8 + V-UT-6 in combination).
- V-E-2: Two active tasks, both matching by name in `state.sessions.<id>.focus`, but `<id>` doesn't match `lookup_session_id` → `AmbiguousActiveTask` (V-UT-9 covers this).
- V-E-3: Worktree path where the slug component is `.ark` itself (path ends in `.ark/worktrees/feat/.ark`) → returns `Some(".ark")`, then `task_dir(".ark").is_dir()` likely fails → falls through. Edge case but harmless.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-3 (clap rejects `--slug`), Phase 2.1 source-scan |
| G-2 | Manual: `ark agent task resume --slug X` and `ark agent task discard` continue to work; existing tests in `task::resume::tests` and `task::discard::tests` unchanged |
| G-3 | V-UT-5 through V-UT-10 cover each step |
| G-4 | Existing `task_discard` tests pass; resolver call signature change is the only edit |
| G-5 | V-UT-8, V-UT-9, C-4 source-scan |
| G-6 | V-IT-1 (PRD-bug regression) |
| G-7 | Manual SPEC review during /ark:commit; SPEC files diffed against the new prose |
| G-8 | Phase 4.3 grep audit; no template edits expected |
| G-9 | Phase 5.1 four-gate check |
| C-1 | V-UT-1 through V-UT-4 |
| C-2 | V-F-2 |
| C-3 | Phase 2 source-scan in clippy or grep step |
| C-4 | Phase 5.1 build pass (compile error if any code still references `NoCurrentTask`) |
| C-5 | Code review |
| C-6 | V-UT-9 assertion checks the candidates Vec equals `state.tasks.active` |
| C-7 | Phase 4 + Phase 5.4 commit groups them |
| C-8 | Phase 5.1 — if snapshot tests exist, update them |
