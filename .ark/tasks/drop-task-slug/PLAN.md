# `drop-task-slug` PLAN `02`

> Status: Revised
> Feature: `drop-task-slug`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Drop `--slug` from `plan|review|execute|verify|commit|promote|archive`, `spec extract`, `worktree cleanup`. Keep `--slug` REQUIRED on both `resume` AND `discard` (R-003 reversal). Replace `resolve_slug` with a `&dyn Ppid`-parameterized topology cascade: worktree path *constrained by active-set membership* → single active → session focus. Split `Error::NoCurrentTask` into `NoActiveTask` + `AmbiguousActiveTask`. Update three feature SPECs (`ark-agent-namespace`, `task-concurrency-control`, `worktree`) and both copies of `workflow.md` in lockstep, gated by an automated grep step.

## Log

[**Added**]
- Phase 4.6: edit three discard template files — drop bare form (N-1).
- API Surface: `run_phase` signature spelled out as `fn run_phase<F>(a, ppid, f)` (N-2).
- Phase 3.1 V-UT-11 fixture: `unique_test_ppid()` pattern + cleanup guard (N-3).
- Phase 4.5 enumerates lines 141, 165, 195 in both workflow.md copies; tightens line 191 prose (N-4).
- T-4: load-state-first rationale (N-5).
- All PLAN 01 additions carried (Phase 4.4 worktree SPEC, Phase 5.1.1 grep gate, C-9 through C-12, NG-7).

[**Changed**]
- Phase 4.5 line list: 141, 165, 195 (was 165 + 195).
- API Surface: `run_phase` signature explicit (was implied via "construct RealPpid once at dispatch top").
- Phase 3.1 V-UT-11 description: replaces literal `StubPpid(42)` with `unique_test_ppid()` from `cache.rs:248`.

[**Removed**]
- None (all 01_PLAN content carried).

[**Unresolved**]
- N-6 deferred to follow-up task `worktree-spec-current-cleanup`. No code change in this PLAN.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review (00) | R-001 – R-010 | Carried | Resolved in 01_PLAN; verified ✅ in 01_REVIEW. |
| Review (01) | N-1 | Accepted | Phase 4.6 added: edits the three discard template files. |
| Review (01) | N-2 | Accepted | API Surface block specifies `fn run_phase<F>(a: TaskSlugArgs, ppid: &dyn Ppid, f: F) -> anyhow::Result<()>`. Phase 2.1 lists `run_phase` in the signature-change set. |
| Review (01) | N-3 | Accepted | Phase 3.1 V-UT-11 description updated: `unique_test_ppid()` pattern, explicit cleanup. |
| Review (01) | N-4 | Accepted | Phase 4.5 enumerates lines 141, 165, 195; line 191 prose tightened. |
| Review (01) | N-5 | Accepted | T-4 added. |
| Review (01) | N-6 | Deferred | Follow-up task `worktree-spec-current-cleanup`. |

---

## Spec `Core specification`

[**Goals**]

- G-1: `ark agent task {plan, review, execute, verify, commit, promote, archive}`, `ark agent spec extract`, and `ark agent task worktree cleanup` no longer accept `--slug`. The field is removed from each `*CliArgs` struct in `crates/ark-cli/src/agent_cli.rs`. `ark agent task <verb> --slug X` exits with clap's "unexpected argument" error.
- G-2: `ark agent task resume --slug <s>` and `ark agent task discard --slug <s>` REQUIRE `--slug`. Both verbs target a specific task by name (`resume` claims focus, `discard` deletes). Missing flag → clap's "the following required arguments were not provided" error. `discard`'s `--force` continues to guard content divergence; the slug requirement guards target selection.
- G-3: A new `resolve_slug(root: &Path, ppid: &dyn Ppid) -> Result<String>` (note the second parameter, per C-11) replaces the existing function. Resolution order:
  1. **Worktree topology with active-set guard.** Compute `state = load_state(layout, ppid)?` first. If `Layout::new(root).slug_from_worktree_root()` returns `Some(slug)`, AND `layout.task_dir(&slug).is_dir()`, AND `state.tasks.active.contains(&slug)`, return it.
  2. **Single active task.** If `state.tasks.active.len() == 1`, return that slug.
  3. **Session focus.** `lookup_session_id(layout, ppid).ok().flatten()` (any IO error is treated as "no session"). If `Some(id)` and `state.sessions.<id>.focus` resolves to a non-empty slug present in `state.tasks.active`, return it.
  4. Error per G-5.
- G-4: REMOVED. (Was: discard cascade. Reverted to G-2.)
- G-5: `Error::NoCurrentTask` is removed from `crates/ark-core/src/error.rs`. Two replacement variants land:
  - `Error::NoActiveTask { project_root: PathBuf }` — message: `` "no active task in `{project_root}`; run `ark agent task new` first" ``.
  - `Error::AmbiguousActiveTask { candidates: Vec<String> }` — message: `` "multiple active tasks: {joined}; run `ark agent task resume --slug <one-of>` to focus this session" ``, where `joined = candidates.join(", ")`. (Per C-12, no `cd worktree` advice — it lies when no candidate has a worktree.)
- G-6: PRD-bug regression test: synthesize `.state.toml` with two active slugs `a, b`, one `[sessions.<UUID>]` whose UUID does NOT match `lookup_session_id`'s output, caller's `cwd` resolves to `<root>/.ark/worktrees/feat/b/`. Expected: `resolve_slug(root, &StubPpid(p))` returns `Ok("b")`. Without the fix, today's resolver returns `Err(NoCurrentTask)`.
- G-7: Three feature SPECs revised:
  - `ark-agent-namespace/SPEC.md`: drop `[--slug <s>]` from the table rows for `plan|review|execute|verify|archive|promote` and `spec extract`. Replace the "Every `--slug`-taking command defaults to `.ark/tasks/.current` … Missing `.current` → `Error::NoCurrentTask`" sentence with the topology-cascade prose. Remove `Error::NoCurrentTask` from the error-variants list; add `NoActiveTask` and `AmbiguousActiveTask`.
  - `task-concurrency-control/SPEC.md`: revise G-9 to: *"`--slug`-less commands resolve via topology cascade (worktree path + active-set membership → single active → session focus). Empty active set → `Error::NoActiveTask`. Multiple actives with no resolver hit → `Error::AmbiguousActiveTask`. Both `resume` and `discard` REQUIRE `--slug`."*. Drop `NoCurrentTask`; add `NoActiveTask` and `AmbiguousActiveTask` to the error-variants list (lines 232–244 region).
  - `worktree/SPEC.md`: in G-2 and G-4 step 1, drop `[--slug <s>]` from the cited `task worktree cleanup` signature. Update prose to say "discovery via `git worktree list --porcelain` + per-checkout `state.tasks.active` traversal." (G-4's existing reference to `.ark/tasks/.current` for discovery is stale legacy text orthogonal to this task; do NOT touch — that's a separate concern that would require its own SPEC review.)
- G-8: Both copies of `workflow.md` (`.ark/workflow.md` and `templates/ark/workflow.md`) are edited at lines 165 and 195: drop `--slug <s>` from the example invocations. Verify other lines via grep (Phase 4.5).
- G-9: All checks pass: `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`. The Phase 5.1.1 grep gate also passes.

[**Non-goals**]

- NG-1: Public CLI surface (`ark init|load|unload|remove|upgrade|context|archive`) unchanged.
- NG-2: Write-side concurrency (`state_mutate`, session-id cache lifecycle, `task new` warn-on-other-active) unchanged.
- NG-3: `ark context`'s focus display logic in `commands/context/gather.rs` is unchanged.
- NG-4: No new helper for non-worktree task dirs.
- NG-5: No deprecation period.
- NG-6: No test changes for `task new` / `task resume` / `task archive` write paths beyond the resolver signature change.
- NG-7: Symlinked worktree roots and case-insensitive filesystems (e.g. macOS APFS default) are best-effort. Lexical match against `Layout::discover_from`'s output is the contract; users with non-canonical layouts can pass `--slug` to `resume`/`discard` or rename the worktree dir.

[**Architecture**]

```
crates/
├── ark-cli/src/
│   └── agent_cli.rs                       MOD: drop slug field from TaskSlugArgs,
│                                                TaskCommitCliArgs, TaskArchiveCliArgs,
│                                                TaskPromoteCliArgs, WorktreeCleanupCliArgs,
│                                                SpecExtractCliArgs;
│                                                TaskDiscardCliArgs.slug: String (required);
│                                                rewrite resolve_slug(root, ppid);
│                                                construct RealPpid once at dispatch top.
└── ark-core/src/
    ├── error.rs                            MOD: remove NoCurrentTask;
    │                                              add NoActiveTask, AmbiguousActiveTask
    ├── lib.rs                              MOD: re-export new error variants
    └── layout.rs                           MOD: add slug_from_worktree_root().

.ark/specs/features/
├── ark-agent-namespace/SPEC.md             MOD: table rows + fallback paragraph + error list
├── task-concurrency-control/SPEC.md        MOD: G-9 + error list
└── worktree/SPEC.md                        MOD: G-2, G-4 step 1 — drop [--slug <s>]

.ark/workflow.md                            MOD: lines 165, 195 — drop --slug from examples
templates/ark/workflow.md                   MOD: same edits, mirrored
```

**Module coupling.** Unchanged from PLAN 00. Read-side `resolve_slug` consumes `Layout`, `state_file::load_state`, `session::cache::lookup_session_id`. Write-side (`task new`/`resume`/`archive`) continues to use `resolve_session_id` for cache materialization.

**Call graph for `resolve_slug(root, ppid)`:**

```
resolve_slug(root, ppid) -> Result<String>
  ├── layout = Layout::new(root)
  ├── state = load_state(layout, ppid)?            // load FIRST, used by all branches
  ├── if let Some(slug) = layout.slug_from_worktree_root() {
  │     if layout.task_dir(&slug).is_dir() && state.tasks.active.contains(&slug) {
  │         return Ok(slug);
  │     }
  │   }                                              // else fall through
  ├── match state.tasks.active.as_slice() {
  │     []     => Err(NoActiveTask { project_root: root.to_owned() }),
  │     [one]  => Ok(one.clone()),
  │     many   => {
  │         let id = lookup_session_id(layout, ppid).ok().flatten();
  │         if let Some(id) = id
  │             && let Some(focus) = state.sessions.get(id.as_str()).map(|s| &s.focus)
  │             && !focus.is_empty()
  │             && many.iter().any(|s| s == focus)
  │         { return Ok(focus.clone()); }
  │         Err(AmbiguousActiveTask { candidates: many.to_vec() })
  │     }
  │ }
```

[**Data Structure**]

```rust
// ark-core/src/error.rs — REMOVED:  Error::NoCurrentTask { path: PathBuf }
//                       — ADDED:

#[error("no active task in `{}`; run `ark agent task new` first", project_root.display())]
NoActiveTask { project_root: PathBuf },

#[error("multiple active tasks: {}; run `ark agent task resume --slug <one-of>` to focus this session", candidates.join(", "))]
AmbiguousActiveTask { candidates: Vec<String> },
```

```rust
// ark-core/src/layout.rs — ADDED:

impl Layout {
    /// If `self.root()` ends with `.ark/worktrees/<branch-type>/<slug>`,
    /// returns `Some(slug)`. Pure lexical match against `self.root().components()`;
    /// no filesystem access, no canonicalization. Symlinks and case-insensitive
    /// volumes that produce a non-canonical root are out of scope (NG-7).
    /// Returns `None` if the slug component fails UTF-8 conversion or the
    /// trailing four components do not match the expected pattern.
    pub fn slug_from_worktree_root(&self) -> Option<String>;
}
```

```rust
// ark-cli/src/agent_cli.rs — REPLACED resolve_slug:

fn resolve_slug(root: &Path, ppid: &dyn Ppid) -> anyhow::Result<String> {
    let layout = Layout::new(root);
    let state = load_state(&layout, ppid)?;

    if let Some(slug) = layout.slug_from_worktree_root()
        && layout.task_dir(&slug).is_dir()
        && state.tasks.active.contains(&slug)
    {
        return Ok(slug);
    }

    match state.tasks.active.as_slice() {
        [] => Err(ark_core::Error::NoActiveTask { project_root: root.to_path_buf() }.into()),
        [one] => Ok(one.clone()),
        many => {
            if let Some(id) = lookup_session_id(&layout, ppid).ok().flatten()
                && let Some(focus) = state.sessions.get(id.as_str()).map(|s| s.focus.clone())
                && !focus.is_empty()
                && many.iter().any(|s| s == &focus)
            {
                return Ok(focus);
            }
            Err(ark_core::Error::AmbiguousActiveTask { candidates: many.to_vec() }.into())
        }
    }
}
```

```rust
// ark-cli/src/agent_cli.rs — TaskDiscardCliArgs:

#[derive(clap::Args)]
struct TaskDiscardCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. REQUIRED — discard targets a specific task by name.
    #[arg(long)]
    slug: String,
    /// Force discard even when seeded files have user content.
    #[arg(long)]
    force: bool,
}
```

[**API Surface**]

CLI structs lose `slug: Option<String>` on:
- `TaskSlugArgs` (Plan/Review/Execute/Verify)
- `TaskCommitCliArgs`
- `TaskArchiveCliArgs`
- `TaskPromoteCliArgs`
- `WorktreeCleanupCliArgs`
- `SpecExtractCliArgs`

`TaskDiscardCliArgs` keeps `slug` but changes type from `Option<String>` to `String` (required).

`TaskResumeCliArgs` unchanged: `slug: String` (required).

Internal helper signature changes:
```rust
// before: fn resolve_slug(root: &Path, explicit: Option<String>) -> anyhow::Result<String>
// after:  fn resolve_slug(root: &Path, ppid: &dyn Ppid) -> anyhow::Result<String>

// before: fn run_phase(a: TaskSlugArgs, f: impl FnOnce(...) -> ...) -> anyhow::Result<()>
// after:  fn run_phase<F>(a: TaskSlugArgs, ppid: &dyn Ppid, f: F) -> anyhow::Result<()>
//         where F: FnOnce(TaskPhaseOptions) -> ark_core::Result<TaskPhaseSummary>
```

CLI dispatch constructs `RealPpid::new()` once at the top of `TaskCommand::dispatch` / `SpecCommand::dispatch` / `WorktreeSubcommand` and passes `&ppid` through to every `resolve_slug` call AND through to `run_phase` (which forwards it to `resolve_slug` internally).

[**Constraints**]

- C-1: `slug_from_worktree_root` is purely lexical. Splits `self.root()` via `Path::components()`, walks from the back, and matches the pattern `[..., Component::Normal(".ark"), Component::Normal("worktrees"), Component::Normal(<branch-type>), Component::Normal(<slug>)]`. Slug component is converted via `OsStr::to_str()`; if conversion fails (non-UTF-8) or any of the fixed-name components don't match, return `None`. `<branch-type>` slot is not validated against any allowlist (the type might post-date the worktree).
- C-2: `slug_from_worktree_root` returns `Option<String>`, never `Result`. No filesystem access.
- C-3: `resolve_slug`'s focus-branch MUST use `lookup_session_id(...)` (not `resolve_session_id`). Source-scan check: the body of `resolve_slug` must not contain the literal `resolve_session_id`.
- C-4: `Error::NoCurrentTask` is fully removed. `grep -rn 'NoCurrentTask' crates/` returns zero results post-task.
- C-5: All filesystem access in `slug_from_worktree_root` and the new `resolve_slug` routes through `io::PathExt`. Path composition routes through `Layout`.
- C-6: `Error::AmbiguousActiveTask`'s `candidates` is the verbatim `state.tasks.active` snapshot, in its existing sort order (sorted+deduped per `task-concurrency-control` C-15).
- C-7: SPEC and workflow.md edits are committed in the same commit as the code change.
- C-8: CLI snapshot tests (`cli_help.rs`, `agent_lifecycle.rs`) are updated for `--help` text changes.
- C-9 (NEW per R-003): `discard --slug` is REQUIRED at the clap level. Field type is `String`, not `Option<String>`. Missing flag → clap's standard required-argument error.
- C-10 (NEW per R-002): Worktree-path resolution returns the slug only when ALL of: `slug_from_worktree_root().is_some()`, `task_dir(&slug).is_dir()`, AND `state.tasks.active.contains(&slug)`. The active-set check prevents resolution to archived-but-half-recreated dirs.
- C-11 (NEW per R-004): `resolve_slug` accepts `&dyn Ppid`. CLI dispatch constructs `RealPpid::new()` exactly once at the top of each dispatch function and passes a reference. Tests pass `StubPpid(u32)`. This matches `load_state` and `lookup_session_id` upstream signatures.
- C-12 (NEW per R-005): `AmbiguousActiveTask` message text drops the "cd into a worktree" advice. Final string: `"multiple active tasks: {joined}; run `ark agent task resume --slug <one-of>` to focus this session"`.

## Runtime `runtime logic`

[**Main Flow**]

1. `claude` invokes `ark agent task plan`.
2. `agent_cli::TaskCommand::dispatch` constructs `let ppid = RealPpid::new();` once.
3. `run_phase` calls `resolve_slug(&root, &ppid)`.
4. `load_state(layout, &ppid)?` reads `.ark/.state.toml`, including reconcile.
5. `Layout::slug_from_worktree_root()` returns `Some("vdso-support")`.
6. `layout.task_dir("vdso-support").is_dir() && state.tasks.active.contains("vdso-support")` → `true`.
7. Resolver returns `Ok("vdso-support")`. Phase transition proceeds.

[**Failure Flow**]

1. User runs `ark agent task plan` in parent checkout, two active tasks, stale session id.
2. Worktree path doesn't match. `state.tasks.active.len() == 2`. `lookup_session_id` returns `None` or a `Some(id)` not in `state.sessions`. Falls through to `Err(AmbiguousActiveTask { candidates: ["a", "b"] })`.
3. Error printed: `"multiple active tasks: a, b; run `ark agent task resume --slug <one-of>` to focus this session"`.
4. User runs `ark agent task resume --slug a` → focus map updated → next `ark agent task plan` resolves cleanly.

Alternative failure: user runs `ark agent task discard` without `--slug` — clap exits before dispatch with `error: the following required arguments were not provided: --slug <SLUG>`.

[**State Transitions**]

- `Error::NoCurrentTask` → REMOVED
- `Error::NoActiveTask`, `Error::AmbiguousActiveTask` → ADDED

## Implementation `split task into phases`

[**Phase 1: Core types + helper**]

1. `crates/ark-core/src/error.rs`:
   - Remove `NoCurrentTask` variant.
   - Add `NoActiveTask { project_root: PathBuf }` and `AmbiguousActiveTask { candidates: Vec<String> }` per Data Structure.
2. `crates/ark-core/src/layout.rs`:
   - Add `slug_from_worktree_root(&self) -> Option<String>`.
   - Add unit tests: `slug_from_worktree_root_returns_some_for_canonical_path`, `_returns_none_for_parent_root`, `_returns_none_for_short_path`, `_returns_none_for_non_utf8_slug` (cfg(unix) using `OsStr::from_bytes` with invalid UTF-8 — gate with `#[cfg(unix)]`).
3. `cargo build -p ark-core && cargo test -p ark-core layout` — green.

[**Phase 2: Resolver rewrite + clap surface**]

1. `crates/ark-cli/src/agent_cli.rs`:
   - Drop `slug: Option<String>` from `TaskSlugArgs`, `TaskCommitCliArgs`, `TaskArchiveCliArgs`, `TaskPromoteCliArgs`, `WorktreeCleanupCliArgs`, `SpecExtractCliArgs`.
   - Change `TaskDiscardCliArgs.slug` from `Option<String>` to `String` (required).
   - Rewrite `resolve_slug(root, ppid)` per Data Structure.
   - Change `run_phase` signature to `fn run_phase<F>(a: TaskSlugArgs, ppid: &dyn Ppid, f: F)` and forward `ppid` to its internal `resolve_slug` call.
   - Update every callsite in `TaskCommand::dispatch`, `SpecCommand::dispatch`, `WorktreeSubcommand` to drop the `Option<String>` arg and pass `&ppid`. Construct `let ppid = RealPpid::new();` once at the top of each `dispatch` function and reuse the binding across all calls.
   - Imports: add `lookup_session_id`, drop `resolve_session_id` from this file (verify it's no longer used here).
2. `cargo build --workspace` clean.
3. `cargo clippy --workspace --all-targets -- -D warnings` clean.

[**Phase 3: Tests**]

1. Add unit tests in `agent_cli.rs::tests` for `resolve_slug` using `StubPpid`:
   - `resolve_slug_finds_slug_from_worktree_path` — root = `<tmp>/.ark/worktrees/feat/foo`, task dir present, slug in `state.tasks.active = ["foo"]` → `Ok("foo")`.
   - `resolve_slug_falls_through_when_worktree_slug_not_in_active` (V-UT-11) — root = `<tmp>/.ark/worktrees/feat/foo`, task dir present, but `state.tasks.active = ["bar"]` → falls through; with single active "bar", returns `Ok("bar")`.
   - `resolve_slug_falls_through_when_worktree_slug_has_no_task_dir` — root = worktree path, no task dir → falls through to state.
   - `resolve_slug_returns_only_active_when_one` — non-worktree root, `active = ["only"]` → `Ok("only")`.
   - `resolve_slug_errors_no_active_when_state_empty` — non-worktree, empty active → `Err(NoActiveTask)`.
   - `resolve_slug_errors_ambiguous_with_no_session_focus` — non-worktree, two actives, no matching session → `Err(AmbiguousActiveTask)` with candidates equal to active list.
   - `resolve_slug_uses_session_focus_when_ambiguous` — two actives, `StubPpid(unique_test_ppid())` (counter-derived, mirrors `cache.rs::tests::unique_test_ppid` at `crates/ark-core/src/session/cache.rs:248` to avoid collisions across parallel test workers), pre-write the cache file at `cache_file_path(&layout, ppid)` with a UUID, plant `[sessions.<UUID>] focus = "a"` in state.toml → returns `Ok("a")`. Test guard explicitly calls `release_session_id` (or `path.remove_if_exists()`) on teardown to prevent leaked cache files in `std::env::temp_dir()`.
   - `resolve_slug_regression_pr_repro` (G-6) — explicit replay.
2. Layout test added in Phase 1 (V-UT-1, V-UT-2, V-UT-3 + non-UTF-8 case).
3. Integration tests in `crates/ark-cli/tests/agent_lifecycle.rs`:
   - V-IT-2: `ark agent task plan` succeeds inside a real worktree fixture (no `--slug`).
   - V-IT-3: `ark agent task plan --slug X` exits with clap's "unexpected argument" code 2.
   - V-UT-12 (better named V-IT-4): `ark agent task discard` (no `--slug`) exits with clap's "required argument" code 2.
4. `cargo test --workspace`.

[**Phase 4: SPEC + workflow.md updates**]

1. `.ark/specs/features/ark-agent-namespace/SPEC.md`:
   - Table at lines 178–186: drop `[--slug <s>]` from `plan`, `review`, `execute`, `verify`, `archive`, `promote`, `spec extract`. `resume` row unchanged. `discard` not in this table.
   - Sentence at line 188: replace with cascade prose per G-7.
   - Error variants list lines 117–129: replace `Error::NoCurrentTask { path }` with the two new variants.
2. `.ark/specs/features/task-concurrency-control/SPEC.md`:
   - G-9 (line 12): replace per G-7.
   - Error-variants block (~line 240): drop `NoCurrentTask` if listed; add `NoActiveTask` and `AmbiguousActiveTask`.
3. `.ark/specs/features/worktree/SPEC.md` (NEW per R-001):
   - G-2: drop `[--slug <s>]` from the cited `task worktree cleanup` signature.
   - G-4 line 1: drop `[--slug <s>]` from the heading signature (`task worktree cleanup [--delete-branch] [--force]`).
4. `.ark/workflow.md` (per R-007 + N-4):
   - Line 141 (`ark agent task worktree cleanup --slug <s> [--delete-branch]`) → `ark agent task worktree cleanup [--delete-branch]`.
   - Line 165 (`ark agent task archive --slug <s> [--month YYYY-MM]`) → `ark agent task archive [--month YYYY-MM]`.
   - Line 191 ("Every `--slug`-taking command defaults to *this session's focused task*..."): tighten prose. Replace generic "Every `--slug`-taking command" with explicit list: "`task new`, `task resume`, and `task discard` accept `--slug` (required on all three). Other verbs use the topology cascade — see `task-concurrency-control` SPEC G-9."
   - Line 195 (`ark agent task worktree cleanup --slug <s>`) → `ark agent task worktree cleanup`.
   - Re-grep `--slug` to confirm only `task new` / `resume` / `discard` mentions remain.
5. `templates/ark/workflow.md`:
   - Same four edits, mirrored. Templates are embedded at build time → `cargo build` after editing.
6. (NEW per N-1) Discard template tri-edit:
   - `templates/claude/commands/ark/discard.md:35`: drop the `ark agent task discard            # uses this session's focus from .ark/.state.toml` line and its trailing `# or` continuation. Leave the explicit `--slug <slug>` and `--slug <slug> --force` examples.
   - `templates/codex/skills/ark-discard/SKILL.md:35`: same edit.
   - `templates/opencode/commands/ark/discard.md:34`: same edit.
   - Re-grep templates for the bare form: `grep -rnE '^ark agent task discard *$' templates/` must return zero hits.

[**Phase 5: Verification + commit**]

1. Four-gate check: `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`.
   - 1.1 (per R-009 + N-1 extension): SPEC + template drift gate. Run:
     ```
     ! grep -rn 'NoCurrentTask\|\[--slug <s>\]' \
         .ark/specs/features/ark-agent-namespace/SPEC.md \
         .ark/specs/features/task-concurrency-control/SPEC.md \
         .ark/specs/features/worktree/SPEC.md
     ! grep -n 'task archive --slug\|worktree cleanup --slug' \
         .ark/workflow.md templates/ark/workflow.md
     ! grep -rnE '^ark agent task discard([[:space:]]|$|#)' templates/
     ```
     All three `grep` invocations must exit non-zero (no matches). Negated by `!`, the pipeline succeeds only when all three greps return empty.
2. E2E smoke: `cargo build --release; TMP=$(mktemp -d); ./target/release/ark load --dir "$TMP" && ./target/release/ark unload --dir "$TMP" && ./target/release/ark load --dir "$TMP" && ./target/release/ark remove --dir "$TMP"`.
3. Manual repro of PRD bug: stage two-active-task `.state.toml` with stale session, faked worktree cwd, run `ark agent task plan` → succeeds.
4. Stage `crates/`, the three SPECs, both `workflow.md` files, `.ark/tasks/drop-task-slug/`. Run `/ark:commit -m "feat(agent): drop --slug from non-targeted task verbs; topology-driven resolver"`.

## Trade-offs `ask reviewer for advice`

- T-1: **Discard targeting.** *Resolved per R-003 acceptance:* `--slug` is REQUIRED on discard. Aligns discard with resume (the other targeted verb). Removes the implicit-target footgun.
- T-2: **`lookup_session_id` Result handling.** *Resolved per R-010 acceptance:* downgrade to `.ok().flatten()`. Cache-tier IO failures fall through to `AmbiguousActiveTask` rather than crashing the verb.
- T-3: **Worktree-path detection: lexical vs. canonical.** Kept lexical per the original recommendation. R-006's symlink/case concern accepted as documented limitation (NG-7) rather than wired through `fs::canonicalize`. Reasoning: `canonicalize` would silently mask user mistakes (typo'd worktree path that happens to symlink-resolve to a real worktree) — the lexical-match contract is more predictable. Users with non-canonical layouts pass `--slug` to `resume`/`discard`, the only verbs where `--slug` survives.

- T-4 (NEW per N-5): **Cascade ordering: load state first vs. try worktree path first.** The call graph in Architecture loads state unconditionally before the worktree branch. Alternative would be: try `slug_from_worktree_root` first as a fast lexical path, only consult state on fallthrough. We chose load-first because C-10's active-set membership guard requires state anyway — a fast-path that returns before the guard runs would silently skip C-10 and reintroduce the very edge case R-002 fixed. `load_state` is a single TOML parse on a small file; the perf cost is irrelevant. Future maintainers MUST NOT reorder these without re-evaluating C-10.

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `Layout::slug_from_worktree_root` — canonical path → `Some(slug)`.
- V-UT-2: `_` — non-worktree path → `None`.
- V-UT-3: `_` — too-short path → `None`.
- V-UT-4: REMOVED per R-008.
- V-UT-5: `resolve_slug` — worktree path with task dir + slug in active → `Ok(slug)`.
- V-UT-6: `_` — worktree path, task dir present, slug NOT in active → falls through. (R-002, **V-UT-11 in 00_PLAN**)
- V-UT-7: `_` — worktree path, no task dir → falls through.
- V-UT-8: `_` — non-worktree, single active → `Ok(slug)`.
- V-UT-9: `_` — non-worktree, empty active → `Err(NoActiveTask)`.
- V-UT-10: `_` — non-worktree, ambiguous, no session focus → `Err(AmbiguousActiveTask)` with candidates equal to active list.
- V-UT-11: `_` — non-worktree, ambiguous, valid session focus → returns focus.
- V-UT-12: `_` — non-worktree, ambiguous, session focus pointing at slug NOT in active → falls through to error.
- V-UT-13: regression — full PRD-bug fixture → `Ok("b")`.

[**Integration Tests**]

- V-IT-1: `ark agent task plan` (no `--slug`) inside real worktree fixture → exit 0.
- V-IT-2: `ark agent task plan --slug X` → clap "unexpected argument" exit code 2.
- V-IT-3: `ark agent task discard` (no `--slug`) → clap "required argument" exit code 2. (R-003)

[**Failure / Robustness Validation**]

- V-F-1: Corrupt `.state.toml` → `Error::StateTomlCorrupt` (existing); resolver propagates without crash.
- V-F-2: `slug_from_worktree_root` with non-UTF-8 slug component → `None`. (cfg(unix) only.)
- V-F-3 (NEW per R-010): cache-file `EACCES` during `lookup_session_id` → resolver falls through to `AmbiguousActiveTask` rather than propagating the IO error. Test via `chmod 000` on the cache file in a tempdir; cleanup with `chmod 600` in test guard.

[**Edge Case Validation**]

- V-E-1: Empty `state.tasks.active` + worktree path with no task dir → `NoActiveTask`.
- V-E-2: Two actives, focus pointing at non-existent session id → `AmbiguousActiveTask`.
- V-E-3: Worktree path where slug component is `.ark` itself → `task_dir(".ark").is_dir()` likely false OR slug not in active → falls through.
- V-E-4 (NEW): worktree path, task dir present, slug archived (in `tasks/archive/` not `tasks/`) → `task_dir.is_dir()` returns false → falls through. Covers R-002's deeper concern.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-2 (clap rejects `--slug`); Phase 2 source-scan for absence of removed fields |
| G-2 | V-IT-3 (discard rejects missing `--slug`); existing `task_resume` tests unchanged |
| G-3 | V-UT-5–V-UT-13 cover each cascade step |
| G-5 | V-UT-9, V-UT-10; C-4 source-scan |
| G-6 | V-UT-13 (PRD-bug regression) |
| G-7 | Phase 5.1.1 grep gate (R-009) |
| G-8 | Phase 5.1.1 grep gate covers workflow.md too |
| G-9 | Phase 5.1 four-gate |
| C-1 | V-UT-1, V-UT-2, V-UT-3, V-F-2 |
| C-2 | V-UT-1–V-UT-3 (return type matches) |
| C-3 | clippy + Phase 2 source-scan (no `resolve_session_id` in `resolve_slug`) |
| C-4 | Build pass — compile error if any code references `NoCurrentTask` |
| C-5 | Code review |
| C-6 | V-UT-10 asserts `candidates == state.tasks.active` |
| C-7 | Single-commit gate at /ark:commit |
| C-8 | Phase 5.1 — snapshot test updates if `insta` is in use |
| C-9 | V-IT-3 |
| C-10 | V-UT-6 (slug NOT in active falls through) |
| C-11 | All `resolve_slug` call sites pass `&ppid`; tests use `StubPpid` |
| C-12 | Assertion in V-UT-10 checks the message string contains the new prose |
