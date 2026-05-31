# `doc-tighten` PRD

---

[**What**]

Tighten the agent-facing surface of Ark — command files (Claude / OpenCode), Codex skills, `workflow.md`, and the artifact templates (PRD / PLAN / REVIEW / VERIFY / SPEC) — so an agent can scan a doc once, find the contract, and execute. Take structural cues from the Trellis reference (`reference/Trellis/.cursor/commands/*.md`): `[AI]` / `[USER]` actor markers, tabular phase summaries, `Step N:` headers around bash blocks, checklist gates, "Failure modes" / "Common oversights" tables.

[**Why**]

Current docs are narrative — every command repeats the tier comparison, the `ark context` rationale, and what each `ark agent task <verb>` does. Three platforms × six commands × ~150 lines of mostly-identical prose. Agents have to re-read paragraphs that the workflow doc already covers, and template hint blocks (`{Clear definition of the "What" and "Why"}`) invite the agent to write more pep-talk prose rather than constrain it. Tightening reduces noise, removes triplicated rationale, and makes the templates *constrain* output rather than seed it with verbose hints.

[**Outcome**]

- Each command/skill file is ~30% shorter, in tabular `Step N:` format with `[AI]` / `[USER]` markers and an explicit gate per step.
- Inline rationale paragraphs (e.g. "This bundles git state, current task, project specs…") are removed; each command points to `workflow.md` once. Single sources of truth.
- `workflow.md` is restructured to absorb the cross-cut rationale (one §"Phase contracts" table that every command references).
- PRD/PLAN/REVIEW/VERIFY/SPEC templates use a tight schema-marker style (`<one-line description>` instead of `{One-line description of the change or feature.}`) — agents are *constrained* to terse, structured output rather than prompted to write essays.
- Output stays content-equivalent: every CLI command, every gate, every failure mode, every spec rule that exists today still exists post-rewrite. No semantic loss.
- Three platform mirrors (Claude / Codex / OpenCode) stay in sync — same trims applied to all three; only frontmatter differs.

[**Related Specs**]

- `.ark/specs/features/ark-context/SPEC.md` — commands invoke `ark context --scope phase --for <phase>`; phrasing in commands must match the SPEC's projection contract.
- `.ark/specs/features/ark-agent-namespace/SPEC.md` — commands name `ark agent task <verb>` calls; the verb list and `--slug` rules must match.
- `.ark/specs/features/ark-workflow-refactor/SPEC.md` — `workflow.md` shape is governed here; restructuring must respect its constraints.
- `.ark/specs/project/LAYOUT.md` — Layout A applies to convention SPECs only (not to commands/templates); reference for style consistency.
