# `session-insight-retention` PRD

---

[**What**]

Research how Ark could automatically extract durable insights and reusable
procedural skills from agent sessions and completed task implementation, retain
them with provenance and lifecycle controls, and surface them to later sessions
without turning transcripts into an unbounded prompt.

[**Why**]

Ark currently persists project constraints in project/feature SPECs and task
intent/evidence in PRDs, plans, reviews, verification reports, research corpora,
and workspace journals. Those artifacts say what the project requires and what
happened, but they do not deliberately promote implementation experience such
as successful debugging recipes, failed approaches, environment-specific
workarounds, or repeatable procedures into reusable semantic or procedural
memory.

This leaves valuable knowledge trapped in session transcripts or verbose task
artifacts. The user specifically identified Stello and Hermes Agent. Both
provide useful but different reference models: Stello separates externally
consumed session memory from one-shot insight injection, while Hermes
advertises agent-curated memory, cross-session search, autonomous skill
creation, and skill evolution.

The design space is risky as well as useful. Automatic retention can preserve
incorrect conclusions, secrets, prompt injection, obsolete commands, or
overfitted procedures; automatic loading can waste context and silently change
agent behavior. Ark needs evidence-backed boundaries before choosing an
implementation task.

[**Outcome**]

A curated, source-backed corpus under `research/` that:

1. Reconstructs the actual extraction, consolidation, storage, recall, and
   update loops used by Hermes and Stello, distinguishing shipped behavior from
   marketing or design intent.
2. Compares additional relevant systems and research on episodic-to-semantic
   memory, trajectory-to-skill extraction, skill evolution, and experience
   retrieval.
3. Defines an Ark-specific memory taxonomy and answers which evidence belongs
   in transcripts, task-local research, journals, insights, reusable skills,
   or SPECs.
4. Evaluates trigger points, scopes, provenance, deduplication, validation,
   security, staleness, rollback, context loading, and cross-harness
   portability.
5. Ends with an adopt/adapt/reject synthesis and a staged recommendation for a
   later Ark design task, including measurable evaluation criteria and explicit
   non-goals. No production code is changed by this research task.

[**Related Specs**]

- `specs/features/ark-research/SPEC.md` — defines this task's corpus-only
  lifecycle and closeout boundary.
- `specs/features/subagent-support/SPEC.md` — defines the researcher write
  contract and persistent per-topic corpus shape.

[**SPEC Path**]

Ignored for research tier.
