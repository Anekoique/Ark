# `ark-sandbox` PRD

---

[**What**]

Add `ark sandbox` — an opt-in Docker container wrapping an existing task worktree, so the ark-workflow runs confined: `create` starts a container with the worktree mounted, `enter` drops into it (shell, or the agent CLI with `--agent`), `rm` tears it down, `list` enumerates running boxes.

[**Why**]

Today the only boundary around an Ark agent is the platform's native permission config (Claude `settings.json`, Codex `sandbox_mode`) — a *seatbelt* that `--dangerously-skip-permissions`/`--yolo` defeats entirely. There is no way to run an unsupervised yolo agent that *cannot* touch anything outside the task. A container turns the seatbelt into a *cage*: kernel-enforced confinement to the worktree, opt-in per task. The worktree feature already produces the exact unit to confine, which makes this tractable without re-implementing agent-infra's ~5,000-LOC sandbox.

[**Outcome**]

- `ark sandbox create [--slug X]` resolves the worktree for the focused/named task and starts a detached container: worktree bind-mounted rw at `/workspace`, parent repo `.git` bind-mounted ro with the worktree gitdir rewritten so in-box git resolves history, a persistent named volume holding the agent config dir, and `ANTHROPIC_API_KEY` passed through when set on the host.
- `ark sandbox enter [--slug X] [--agent]` runs `docker exec -it` into the box: bash at `/workspace` by default; with `--agent`, launches the configured platform's CLI with its yolo flag.
- `ark sandbox rm [--slug X] [--keep-volume]` stops + removes the container (named volume preserved unless explicitly dropped); worktree teardown stays the separate `ark cleanup` / `task worktree cleanup` step.
- `ark sandbox list` prints one row per running Ark sandbox (slug, branch, container id, status); empty stdout when none.
- Subscription login works cross-platform: the user runs the CLI's login flow once inside the box and the OAuth token persists in the named volume across recreate — no host keychain access, no credential reconciliation.
- Requires `docker` on PATH; a clear `Error::DockerUnavailable` when absent. All container ops route through a new `io/docker.rs` (sibling to `io/git.rs`); no `Command::new` leaks into `commands/`.
- Existing flows unchanged: a task without `ark sandbox create` behaves exactly as today; `ark unload` / `load` / `upgrade` ignore sandbox state.

[**Related Specs**]

- `specs/features/worktree/SPEC.md` — sandbox **reuses** the worktree, never creates one. It resolves the box's mount target via the same `find_worktree_for_slug` discovery + `WorktreeConfig::resolve_worktree_dir` the worktree feature owns; sandbox lifecycle is strictly downstream of `task new --worktree`. Mirrors worktree's NG-3 (no PR integration) and its rollback-boundary discipline.
- `specs/features/task-concurrency-control/SPEC.md` — sandbox is per-worktree, so it inherits the per-checkout `.state.toml` focus model: `--slug` is optional and defaults to `state.focus` (→ `Error::NoFocus` when unset), exactly like the other downstream verbs. Sandbox adds no new state-file fields.
- `specs/features/ark-agent-namespace/SPEC.md` — decision point in PLAN: whether `sandbox` is a top-level `ark sandbox` (semver-covered, like `ark cleanup`) or lives under the hidden `ark agent` namespace. Leaning top-level, since it is a user-facing command, not a workflow-structural mutation.
- `specs/features/codex-support/SPEC.md`, `specs/features/opencode-support/SPEC.md` — the `--agent` enter path needs each platform's CLI binary + yolo flag (`claude --dangerously-skip-permissions`, `codex --yolo`, …). This is the only place sandbox couples to the platform registry; the default shell path stays platform-agnostic.

[**SPEC Path**]

ark-sandbox
