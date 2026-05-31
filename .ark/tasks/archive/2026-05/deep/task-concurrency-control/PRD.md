# `task-concurrency-control` PRD

---

[**What**]
Replace `.ark/tasks/.current` (single-slug pointer) and `.ark/.developer` (one-line identity file) with a single per-checkout `.ark/.state.toml` that holds developer identity, the **set** of active task slugs, and per-session focus pointers. Add two new agent ops — `task resume <slug>` and `task discard <slug>` — and the locking, atomic-write, GC, and migration machinery needed for cross-platform concurrent access.

[**Why**]

The current model is wrong on three axes:

1. **Single-pointer `.current` collapses parallel-task storage.** Each `.ark/tasks/<slug>/` already carries its own `task.toml` (tier, phase, iteration); storage has supported parallel tasks since day one. But `.current` is a single-slug file, and `task new` (`crates/ark-core/src/commands/agent/task/new.rs:135-137`) clobbers it without consent. Starting a second task silently orphans the first — the bug that opened this thread.

2. **Multi-session workflows are unrepresentable.** Users want to run `/ark:quick fix-A` in one terminal and `/ark:quick fix-B` in another, both against the same checkout. Today they race on `.current`; whichever shell ran `task new` last "owns" focus for both. There is no per-session attention model.

3. **The downstream workflow refactor (ROADMAP item #2) needs this substrate.** Once archive is postponed to version-cut time, the active set will routinely have N entries between cuts. A single-pointer `.current` becomes actively misleading in that world. This task ships the data-model upgrade first, so the workflow refactor can build on it.

The fix is structurally small: one new file format (`.state.toml`), one new locking pattern (`File::try_lock` + atomic rename), one session-id derivation (PPID + project-hash + UUID cache in temp dir), two new lifecycle ops (`resume`, `discard`). The blast radius is bounded — 4 read sites, 3 write sites, 1 remove site, 4 identity-API call sites, 5 test modules — but the contract change (state-file shape, session-aware focus) makes this deep tier rather than standard.

[**Outcome**]

Observable success criteria:

- **Multi-session focus isolation.** Two shells in the same checkout can each run `task new --slug a` and `task new --slug b`; both appear in `[tasks].active`; each shell sees its own focus when running `--slug`-less commands; archiving one task in shell A does not disturb shell B's focus.
- **Stale `.current` clobber is gone.** `task new` warns (does not refuse) when `[tasks].active` is non-empty and never silently overrides another session's focus.
- **`task resume <slug>`** claims an existing active task as the current session's focus; `task discard <slug>` removes a task from active + dir, refusing without `--force` when seeded files have user content; `task discard` of an archived task is refused.
- **Cross-platform.** Tests pass on Linux, macOS, Windows. No `kill -0`, no `/proc`, no UNIX-only path tricks. `File::try_lock` (stdlib stable Rust 1.89+) for locking; `std::fs::rename` for atomic write; `std::env::temp_dir()` + `std::process::parent_id()` for session-id cache.
- **GC works.** A session whose cache file is missing or whose UUID mismatches the recorded one is dropped from `[sessions.*]` on the next state read. Tested by hand-writing a state file with two sessions and verifying the dead one is pruned.
- **Lock contention is bounded.** Two threads each calling `state_mutate` with a 50 ms sleep inside the closure both succeed (≤200 ms total backoff). Under heavier contention, `Error::StateLockContended` is raised after exhausting retries.
- **Migration is self-healing.** A pre-existing install with `.ark/.developer` and `.ark/tasks/.current` keeps working; on the first state mutation, the new `.state.toml` is created and the two legacy files are deleted. No `ark upgrade` step required.
- **Worktree isolation.** Each worktree has its own `.ark/.state.toml`; `task new --worktree --slug w1` from the parent does not touch the parent's state file's `active` list.
- **Identity API is unchanged at the call-site level.** `read_developer_name`, `require_developer_name`, `write_developer_file` keep the same signatures; bodies delegate to the new state-file backend. The 4 existing call sites do not need to change.
- **Tests pass.** `cargo test -p ark-core -p ark-cli` is green. `cargo clippy --all-targets -- -D warnings` is green. New tests cover: multi-session isolation, GC, lock contention, migration, `discard --force` vs. refusal, `resume <invalid>` error, worktree isolation.
- **End-to-end smoke** (per the manual sequence in the PLAN's Validation section): two-shell sandbox produces the expected state-file contents at every step.

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — extends G-3 (task verb set) with `resume` and `discard`. Adds new error variants compatible with the existing `Error::*` family. Subcommand pattern matches `Archive`'s shape.
- `.ark/specs/features/workspace/SPEC.md` — identity flow (G-2/G-7/G-11/G-18) keeps its API surface but the `.developer` file is replaced. The legacy reader stays in `migrate.rs` for the migration window. Auto-record on archive (G-7) continues to work because identity reads now resolve via state file. The `templates/ark/.gitignore` adds `.state.toml` and `.state.toml.lock` lines (additive; the existing `.developer` line stays for the migration window).
- `.ark/specs/features/worktree/SPEC.md` — G-9 (each worktree has its own `.ark/`) is the basis for "each worktree gets its own `.state.toml`." No worktree-SPEC change needed; just consume the existing per-worktree `Layout` resolution. Quick-tier `--worktree` opt-in (G-2) is already in place.
