# `improve-ark-context` PRD

---

[**What**]

Grow `ark context`'s projection surface additively — preserving `schema: 1` — so phase projections carry the workflow state that features shipped after the original `ark-context` promotion (worktree, focus, recursive feature SPECs, workspace journals, installed subagents) now demand. No restructure, no schema bump.

[**Why**]

The original `ark-context` SPEC was promoted on 2026-04-27, before the following features landed: `worktree` (2026-04-28), `project-spec` (2026-04-30), `task-concurrency-control` (2026-05-01), `workspace` (2026-05-02), `subagent-support` (2026-05-10), `detachable-feature-spec` (2026-05-18), `ark-research` (2026-05-20). Each one introduced workflow state that an agent should be able to read from a single `ark context --scope phase --for <phase>` call. Today the projection forces the agent to either guess (`do I need --worktree?`, `is ark-reviewer installed on this platform?`, `where does the journal live?`) or re-read disk to recover what the projection already could have surfaced.

Concretely:

- `detachable-feature-spec` added `SpecRow.feature_path: Vec<String>`, but the projection ships a flat sorted list — there's no tree view to navigate subtrees (e.g., `xemu/*` features).
- `worktree` and `task-concurrency-control` exposed the per-checkout focus model, but the projection never tells the agent whether it's running in the main checkout vs a worktree, nor whether the current `[focus]` slug is bound here.
- `workspace` introduced `RecordProjection` for `--scope record`, but the COMMIT phase projection (`--for commit`) — the one a slash command reads right before writing a journal entry — does not surface it.
- `subagent-support` shipped three Ark subagents across Claude / Codex / OpenCode and documents that the main session dispatches them, but slash commands have no way to know which agents are actually installed in this checkout; users can add their own agents too, and the slash command's "ask the user which reviewer" prompt becomes inaccurate when the canonical agent has been removed.

The downstream cost is paid in every slash command body that wants to behave correctly: each one either makes do with less information than the projection could provide, or re-reads files the projection already touched.

[**Outcome**]

- `ark context --scope session --format json` and `--scope phase --for <phase> --format json` carry, where appropriate, the following additive fields without bumping `SCHEMA_VERSION`:
  - `checkout: { root_kind: "main" | "worktree", branch: String, focus_slug: Option<String> }` — every projection.
  - `specs.features_tree: Option<SpecNode>` — session + design scopes only.
  - `subagents: [{ platform: String, stems: [String] }]` — session + design/plan/review/verify scopes (the scopes whose slash commands dispatch them).
  - `record: Some(RecordProjection)` on commit scope (same shape as `--scope record`).
- Slash commands and other consumers are NOT updated as part of this task; the new fields are additive and any downstream that wants them can read the projection. The reviewer/verifier-pick workflow in `/ark:design` continues to talk about the three reserved Ark canonical stems (`ark-researcher` / `ark-reviewer` / `ark-verifier`) per `subagent-support` SPEC and does not branch on arbitrary user-installed agents.
- `ark context` is still a single stdout write per invocation. Text output stays human-readable; new sections render under the existing locked headings or under new locked subheadings.
- The recursive-tree warnings (`GatherWarning::MissingChild`, `OrphanLeaf`, `OrphanSubtree`) already surface in JSON; text mode also renders them where they appear.
- `--scope phase --for research` is **not** added — `ark-research` SPEC NG-4 stands; the existing design projection serves research tasks.
- Existing consumers (slash commands that don't yet read the new fields, `.installed.json` parsers, downstream tools) continue to work — additive serde fields, no field renames, no behavior change for `--scope session` envelope wrapping.
- A `[**CHANGELOG**]` entry on `specs/features/ark-context/SPEC.md` records the additive growth.

[**Related Specs**]

- `specs/features/ark-context/SPEC.md` — primary SPEC under modification; gains `checkout`, `features_tree`, `subagents`, commit-scope `record` fields. CHANGELOG entry on commit.
- `specs/features/detachable-feature-spec/SPEC.md` — `feature_path: Vec<String>` is the input the new `features_tree` is derived from. No SPEC change; the tree is a projection-side reshape.
- `specs/features/workspace/SPEC.md` — `RecordProjection` is reused on commit-scope. No SPEC change to workspace itself.
- `specs/features/worktree/SPEC.md` — `[worktree]` config + `.ark/worktrees/<branch>/` topology are the inputs the new `checkout` field describes. No SPEC change.
- `specs/features/task-concurrency-control/SPEC.md` — per-checkout `.state.toml` `[focus]` slug is the source of `checkout.focus_slug`. No SPEC change.
- `specs/features/subagent-support/SPEC.md` — `agents_dest_dir` per platform is where the `subagents` scan reads from. No SPEC change to subagent-support itself; the scan does **not** restrict to Ark's three canonical agents — it lists every installed agent stem.
- `specs/features/ark-research/SPEC.md` — NG-4 is honored; no `PhaseFilter::Research` added; `/ark:research` continues to use the existing design projection.

[**SPEC Path**]

`ark-context`
