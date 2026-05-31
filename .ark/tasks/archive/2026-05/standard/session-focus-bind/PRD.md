# `session-focus-bind` PRD

---

[**What**]

Replace the per-session focus map and topology cascade with a single per-checkout `[focus]` field in `.state.toml`. Delete the session-id cache, the `Ppid` trait, and the `session/` module.

[**Why**]

The topology cascade (worktree path → single active → session focus) was meant to let non-targeted verbs like `task plan` / `task execute` find the right slug without `--slug`. It fails any time `tasks.active` carries multiple slugs — the steady state of the workflow, since every `/ark:commit` adds a slug and `/ark:archive` runs on the user's schedule. The session-focus tiebreaker doesn't help because (a) it's only consulted as the third resolver, and (b) the per-session keying assumes a stable interactive shell. Under any AI harness (Claude Code, Codex, OpenCode), each tool call is a new subprocess, so the PPID changes every invocation, every `ark` call materializes a fresh session id, and the focus written by one call is keyed to a session id the next call will never see.

The session-pruning bug compounds the same root cause: `prune_dead_sessions` uses cache-file existence as a liveness signal, but `$TMPDIR` files outlive the shell on macOS. The state file accumulates dead `[sessions.*]` entries forever (currently 17 against PIDs from long-gone Claude shells), and `$TMPDIR` accumulates orphaned cache files (currently 26 for this project alone).

Both bugs share a root: per-session identity is the wrong granularity. The user works in one checkout at a time and binds it to one task at a time. The git worktree feature already gives a separate checkout (and so a separate `.state.toml`) when a deep task wants its own focus. Per-checkout focus is what the existing `.state.toml` *should* have carried from the start; the session-keyed map was machinery added in `task-concurrency-control` for a multi-shell concurrency case that the workflow never actually requires.

Path B (per-checkout focus) is a net deletion: the `Ppid` trait, the `session/` module, the `$TMPDIR` cache file, `cache_matches`, `release_session_id`, `prune_dead_sessions`, and the `lookup_session_id` plumbing through every call site all go away.

[**Outcome**]

State-file shape:

- `.state.toml` gains a top-level optional `[focus] slug = "..."` field. Absent or empty when no task is bound.
- The `[sessions.*]` table and `Session` struct are removed. `BTreeMap` import follows.
- `tasks.active` semantics unchanged — sorted+deduped slug list of non-archived task dirs.

Verb behavior:

- `task new --slug X --tier T` and `task resume --slug X` write `state.focus = Some("X")` in the same `state_mutate` that updates `tasks.active`.
- `task archive` and `task discard` clear `state.focus` if and only if the cleared slug equals the focus.
- `task new` no longer warns about other active tasks (the workflow is purely user-driven; multiple actives are normal).
- All other verbs (`task plan`, `task review`, `task execute`, `task verify`, `task commit`, `task promote`) resolve the slug via `state.focus`. No cascade. No worktree-path inference. No "single active wins" inference.
- Verbs run with no focus set error with a new `Error::NoFocus { project_root, candidates: Vec<String> }`. Display lists `tasks.active` and tells the user to run `task new` or `task resume`.

Module deletions:

- `crates/ark-core/src/session/` (the entire module — `cache.rs`, `ppid.rs`, `mod.rs`).
- `Ppid` trait, `RealPpid`, `StubPpid`, `SessionId`, `cache_file_path`, `cache_matches`, `resolve_session_id`, `lookup_session_id`, `release_session_id`.
- `state::checkout::reconcile::prune_dead_sessions`.
- `Session` struct and the `BTreeMap<String, Session>` field on `StateFile`.
- `Error::NoActiveTask`, `Error::AmbiguousActiveTask` removed (replaced by `Error::NoFocus`).
- `lookup_session_id` callers (`commands/context/gather.rs` and `agent_cli.rs::resolve_slug`) drop the `&dyn Ppid` parameter.

State-file migration:

- Loading a `.state.toml` with `[sessions.*]` blocks: ignore them. The next `state_mutate` writes the file without the section.
- Loading a `.state.toml` with no `[focus]`: that's the absent state. Verbs that need it error cleanly.
- No on-disk migration step; reconcile handles it implicitly.

Cleanup:

- The 26 stale `$TMPDIR/ark-session-f213d6cbc5842122-*.id` files are unlinked when the new code runs (a one-shot best-effort cleanup in the `state_mutate` migration path: scan `$TMPDIR` for `ark-session-<this-project-hash>-*.id` and `remove_if_exists`). Best-effort — IO errors do not abort.
- Verified by an integration test that plants stale cache files and asserts they're gone after a `state_mutate`.

API surface:

- `lib.rs` re-exports updated: drop `session::*`, drop `prune_dead_sessions`, drop `Session`. Add nothing — `StateFile.focus` and `Error::NoFocus` are reachable via the existing `state` and `error` paths.
- `state_mutate` and `load_state` signatures lose the `&dyn Ppid` parameter. Every call site updates.
- `agent_cli.rs::resolve_slug(&Path)` returns `Result<String>` from `state.focus`, no `Ppid`.

Tests:

- `crates/ark-core/src/session/` tests deleted with the module.
- `state::checkout::reconcile::tests::prune_dead_sessions_runs_after_orphan_drop` deleted.
- `agent_cli::tests` cascade tests (`resolve_slug_finds_slug_from_worktree_path`, `resolve_slug_returns_only_active_when_one`, `resolve_slug_uses_session_focus_when_ambiguous`, `resolve_slug_falls_through_when_*`, `resolve_slug_errors_no_active_when_state_empty`, `resolve_slug_errors_ambiguous_with_no_session_focus`) replaced with two tests: `resolve_slug_returns_focus`, `resolve_slug_errors_no_focus_when_focus_unset`.
- New tests for the `task_new` / `task_resume` / `task_archive` / `task_discard` focus mutations (one each, against in-memory state).
- New test for the `$TMPDIR` cache-file cleanup pass.

Workflow doc:

- `.ark/workflow.md` §"Session model" rewritten to "Focus model": `.state.toml` carries one focus per checkout; deep-tier worktrees have their own. Failure-modes table swaps `NoActiveTask` / `AmbiguousActiveTask` for `NoFocus`. The "topology cascade" paragraph is deleted.

[**Related Specs**]

- `specs/features/ark-agent-namespace/SPEC.md` — C-14 (topology cascade) is replaced by C-14′: "`--slug` required on `task new` / `task resume` / `task discard`. Other verbs resolve via `.state.toml`'s `[focus]` field; absent → `Error::NoFocus`." Data-Structure block updated to drop `NoActiveTask` / `AmbiguousActiveTask`, add `NoFocus`. CHANGELOG entry dated 2026-05-08.
- `specs/features/task-concurrency-control/SPEC.md` — substantial rewrite. C-1, C-5, C-9, C-10, C-16, C-18, C-22, C-23 all touch session-id machinery and need updates: session-id cache and `Ppid` trait deleted; reconcile drops only `tasks.active` entries (no session pruning); `[focus]` is the new state-file field. The `worktree list` call graph stays the same (each worktree has its own `.state.toml`). Architecture block deletes `session/` and shrinks `state_file/reconcile.rs` to a one-pass active-set reconcile. CHANGELOG entry dated 2026-05-08.
- `specs/features/workspace/SPEC.md` — if it references `[sessions.*]` or `Ppid`, update accordingly. Likely just a CHANGELOG note. (To verify in PLAN phase.)
