# `simplify-deep-tier` PRD

---

[**What**]

Drop the deep-tier PLAN ⇄ REVIEW iteration loop: deep tier becomes a linear PLAN → REVIEW → EXECUTE, with REVIEW findings folded back into the single PLAN in place.

[**Why**]

In practice the loop runs only 1–2 rounds and the lone REVIEW surfaces minor issues fixable at execution time. The iteration ceremony — `NN_` filenames, the PLAN `## Log` / Response Matrix, the `iteration` / `max_iterations` fields, the `Review → Plan` back-edge — buys no quality and adds friction. A single PLAN + single REVIEW + in-place polish captures the value without the loop.

[**Outcome**]

- Deep-tier lifecycle is `Design → Plan → Review → Execute → Verify → Committed → Archived`, with no `Review → Plan` transition.
- Deep tier seeds plain `PLAN.md` and `REVIEW.md` (no `NN_` prefix), parallel to `VERIFY.md`.
- `TaskToml` no longer carries `iteration` or `max_iterations`; existing task.toml files with those keys still load (fields ignored).
- PLAN and REVIEW templates have no iteration / Log / Response-Matrix / loop vocabulary; REVIEW keeps the single Verdict + Findings shape.
- Workflow doc and `/ark:design` describe: REVIEW writes findings → main session edits `PLAN.md` in place to address CRITICAL/HIGH → `task execute`. No new iteration file.
- SPEC extraction reads `PLAN.md`; the CHANGELOG line references `PLAN.md`, not `NN_PLAN.md`.
- `cargo build`, `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` all pass; the load/unload round-trip smoke test passes.

[**Related Specs**]

- `specs/features/ark-workflow-refactor/SPEC.md` — this is the feature that defined the tier lifecycle + `Review → Plan` iterate edge; this task supersedes its loop portion. (INDEX row exists; SPEC body is currently missing — pre-existing dangling row.)
- `specs/features/spec-actuators/SPEC.md` — SPEC extraction path (`PLAN.md` → feature SPEC) is exercised; verify actuator-tagged constraints still extract from the flattened filename.

[**SPEC Path**]

simplify-deep-tier
