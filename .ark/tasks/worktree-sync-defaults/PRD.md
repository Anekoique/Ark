# `worktree-sync-defaults` PRD

---

[**What**]
Make `task new --worktree` synchronize developer identity (`.ark/.developer`) and git submodules into the new worktree by default — promoting from documentation-only to first-class behavior.

[**Why**]
Two recurring sync gaps surface when users run `task new --worktree`:

1. `.ark/.developer` (gitignored per-checkout identity) does not propagate. The new worktree starts without identity, so `ark agent workspace record` and any other identity-gated command fails with `MissingIdentity` until the user re-runs `ark init --developer <name>` inside every worktree.
2. Git submodules are not initialized inside the new worktree. Submodule paths sit empty until the user manually runs `git submodule update --init --recursive`.

The existing `[worktree].copy` and `[worktree].post_create` levers were considered but neither fits cleanly:
- Defaulting `copy = [".ark/.developer"]` would hard-fail every new worktree in projects that haven't bootstrapped identity yet (current copy semantics: source-missing → `WorktreeCopySourceMissing`).
- Defaulting `post_create = ["git submodule update --init --recursive"]` is safe (no-op without `.gitmodules`) but solves only half the problem.

A first-class identity-sync step (reusing the existing `identity_resolve` / `identity_prompt` / `identity_write` machinery from `commands/agent/workspace/identity.rs`) is the clean fix for `.developer`. Defaulting the submodule hook in `WorktreeConfig::default().post_create` covers submodules.

[**Outcome**]
- `task new --worktree` resolves developer identity from the parent's `.ark/.developer`. If absent, prompts the user (TTY) and writes the answer back to the parent. Then mirrors identity into the new worktree's `.ark/.developer`.
- Non-TTY (no stdin or `ARK_NO_PROMPT=1`-equivalent) with parent missing `.developer` returns `Error::MissingIdentity` so the agent surfaces the existing message *"no developer identity set; run `ark init --developer <name>` ..."* to the user.
- `WorktreeConfig::default().post_create` includes `git submodule update --init --recursive`. No-op when `.gitmodules` is absent (git exits 0).
- Existing `cfg.copy` semantics (hard-fail on missing source) remain unchanged. Identity sync is a separate, dedicated step in the create flow — not a `copy` entry.
- Templates: `templates/ark/config.toml` `[worktree]` section's `post_create` example shows the new default; comments updated to reflect identity-sync is automatic.
- All existing worktree tests still pass. New tests cover the four identity branches (parent has, parent missing+TTY-prompt, parent missing+non-TTY, write-failure rollback) and the submodule-init default.

[**Related Specs**]

- `.ark/specs/features/worktree/SPEC.md` — adds a Goal for identity sync between `git worktree add` and `cfg.copy`; amends G-1 (defaults) to include the submodule-init default in `post_create`. NG-7 ("No monorepo / submodule init in worktrees") is partly relaxed: a documented default `post_create` entry is added, but the worktree code itself remains submodule-agnostic (it just runs the configured shell command).
- `.ark/specs/features/workspace/SPEC.md` — `identity_resolve`, `identity_prompt`, `identity_write` are reused verbatim from the workspace feature. No change to that feature.
