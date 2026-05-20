# `ark-research` PRD

---

[**What**]

Add a fourth workflow tier — `research` — and a dedicated `/ark:research <topic>` slash command whose deliverable is a curated reference corpus under `.ark/tasks/<slug>/research/`, not a code change.

[**Why**]

The current tier set (`quick` / `standard` / `deep`) assumes the user can already write a PRD — they know what they want to build. Many real engineering problems start upstream of that: "should we even do X?", "how do other agent harnesses handle Y?", "what's the prior art for Z?". Today, the only outlet is an embedded `ark-researcher` dispatch during a deep-tier DESIGN/PLAN — which presupposes a deep task already exists. That conflates *exploration* with *implementation*, forcing users to either (a) skip first-class research tracking entirely, or (b) fabricate an implementation task whose PRD is half-imagined. Add a research tier whose deliverable IS the corpus, whose follow-up is genuinely optional, and which reuses the existing `ark-researcher` subagent contract verbatim.

[**Outcome**]

- `/ark:research "<topic>"` creates a research task at `.ark/tasks/<slug>/` with `tier = "research"`, `phase = "research"`, a seeded PRD scoped for research semantics, and binds focus to the slug.
- Iterative `ark-researcher` dispatches populate `.ark/tasks/<slug>/research/<topic>.md` files exactly as they do today on deep tier.
- `/ark:commit -m "<msg>"` closes a research task atomically: stages `task.toml` + `PRD.md` + `research/**`, commits, marks `phase = Committed`. No VERIFY gate, no SPEC extraction, no `[**SPEC Path**]` requirement.
- `ark agent task plan` / `review` / `execute` / `verify` on a research task error out with `IllegalPhaseTransition` — the only legal transition from `Research` is `Committed`.
- `--worktree` is accepted on research tier (user choice), not required.
- `ark archive` moves committed research tasks into the YYYY-MM bucket like any other tier.
- All three platforms (Claude / Codex / OpenCode) ship the `/ark:research` slash command: Claude and OpenCode bodies are byte-identical modulo frontmatter; Codex applies the documented substitution map (`/ark:<cmd> → ark-<cmd>` for the slash-command set, `$ARGUMENTS → <topic>`, H1 reshape).
- workflow.md explains: when to use research vs. embedded researcher dispatch inside a tiered task; PRD-on-research semantic remap (Outcome optional; SPEC Path ignored; Related Specs optional).
- Existing tasks of tier `quick` / `standard` / `deep` continue to round-trip through `task.toml` load/save with no migration.

[**Related Specs**]

- `specs/features/ark-agent-namespace/SPEC.md` — extended: `Tier::Research` variant + `Phase::Research` variant + new transition row `(Research, Research, Committed)`; CHANGELOG entry on the touched SPEC.
- `specs/features/subagent-support/SPEC.md` — reused as-is: the `ark-researcher` write-allowed path `.ark/tasks/<slug>/research/*.md` already covers research-tier tasks. No SPEC change; only the workflow doc references new dispatch context.
- `specs/features/task-concurrency-control/SPEC.md` — unchanged: focus model, `state.tasks.active`, reconcile semantics all work for research-tier slugs without modification.
- `specs/features/detachable-feature-spec/SPEC.md` — `task_commit` for research tier explicitly skips `parse_spec_path` / `spec_extract` / `spec_register`. No SPEC change.
- `specs/features/worktree/SPEC.md` — `--worktree` flag remains opt-in across all tiers including research. No SPEC change.
- `specs/features/ark-context/SPEC.md` — design-phase projection already serves freshly-seeded PRD tasks; research tier reuses without schema change.
- `specs/features/workspace/SPEC.md` — journal entry on commit fires for research tier identically.

[**SPEC Path**]

`ark-research`
