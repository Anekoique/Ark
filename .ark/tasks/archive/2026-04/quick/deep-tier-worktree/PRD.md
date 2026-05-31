# `deep-tier-worktree` PRD

---

[**What**]
Make worktrees mandatory for deep-tier tasks. Update the four DESIGN documents (`workflow.md` + the three platform `design.md` / `SKILL.md` templates) so the rule is stated where the agent will see it at scaffold time.

[**Why**]
Worktree-support shipped (`c318367 feat(workflow): add worktree management`) so deep-tier work can run on its own branch and not collide with the parent's `.current` or dirty tree. The current docs frame `--worktree` as an optional collision-avoidance flag, so an agent reading the workflow scaffolds deep tasks straight into the parent — defeating the feature. Pin it as a deep-tier requirement at every place the agent decides how to scaffold.

[**Outcome**]

- `.ark/workflow.md` §3 — replaces the soft *"To run multiple tasks in parallel without `.current` collisions, pass `--worktree`"* sentence with a sharper rule: *"Deep tier MUST use `--worktree`. Standard tier: opt in when work would collide with `.current` or with in-flight changes on the active branch. Quick tier: never."* Same paragraph slot, same length budget.
- `.ark/workflow.md` §4 DESIGN — the bulleted `Calls:` example for `task new` shows `--worktree` as required for deep tier (e.g. `--tier deep --worktree`), with one trailing line stating the deep-tier requirement and the post-scaffold `cd .ark/worktrees/<branch>/`. EXECUTE's existing "Worktree note" stays as-is.
- Three platform DESIGN templates updated identically at §1.4 "Create the task":
  - `templates/claude/commands/ark/design.md`
  - `templates/codex/skills/ark-design/SKILL.md`
  - `templates/opencode/commands/ark/design.md`
  Each gets: deep-tier example shows `--tier deep --worktree`; one new line stating *"Deep tier MUST use `--worktree`. After scaffolding, `cd .ark/worktrees/<branch>/` and run all subsequent phase commands (plan/review/execute/verify/archive) from the worktree."*
- No code changes (CLI does NOT enforce `--tier deep` ⇒ `--worktree`; this task is doc-only — mechanical enforcement is a separate follow-up if desired).
- No new templates, no `Cargo.toml` changes, no SPEC promotion (quick tier).
- Verification: `grep -n "worktree" .ark/workflow.md templates/claude/commands/ark/design.md templates/codex/skills/ark-design/SKILL.md templates/opencode/commands/ark/design.md` shows the new rule in all four files at the §1.4 / §3 / §4-DESIGN sites; `cargo build -p ark-core` still succeeds (templates are compiled in via `include_dir!` / `include_str!`).

**Verified 2026-04-29.** `grep` shows the `**Deep tier MUST use --worktree.**` line in all three platform templates (claude `design.md:73`, codex `SKILL.md:73`, opencode `design.md:72`); `workflow.md:60` carries the §3 "Worktree rule" replacement and `workflow.md:105` carries the §4-DESIGN trailing line. `cargo build -p ark-core` succeeds silently.

[**Related Specs**]

- `.ark/specs/features/worktree-support/SPEC.md` — defines `--worktree` at scaffold time, `.ark/worktrees/<branch>/` layout, and the lifecycle commands (`task worktree list`/`cleanup`). This task tightens the *guidance* around when to use the feature; the SPEC's surface is unchanged.
