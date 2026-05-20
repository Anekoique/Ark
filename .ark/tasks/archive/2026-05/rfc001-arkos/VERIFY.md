# `rfc001-arkos` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `rfc001-arkos`
> Target Task: `rfc001-arkos`
> Tier: `standard` (promoted down from deep mid-flight; REVIEW skipped by user direction)
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation).

---

## Project Spec Compliance

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: N/A — task makes no `specs/project/` changes; index unchanged.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: N/A — task touches no project SPEC; existing conformance unaffected.
  - `LAYOUT.md`
  - `rust/COMMENTS.md`
  - `rust/STYLE.md`
  - `rust/ERRORS.md`

## Related Feature Spec Compliance

- [x] specs/features/ark-context/SPEC.md: PASS — RFC references `SCHEMA_VERSION` as the existing stability mechanism; no SPEC change required.
- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — RFC frames `ark agent` stability for stage-1 ArkOS as Open Question Q3, not a commitment; no SPEC CHANGELOG entry required per the PRD scope.
- [x] specs/features/ark-workflow-refactor/SPEC.md: PASS — RFC reinforces human-gated workflow philosophy without altering SPEC.
- [x] specs/features/subagent-support/SPEC.md: PASS — RFC names subagents as a stage-2 absorption direction; no immediate SPEC change.

## PRD Constraints

- [x] Single file at `docs/rfcs/001-arkos.md`, ~250–600 lines: PASS — `wc -l` reports 370.
- [x] Three-digit prefix `001-`, kebab-case slug: PASS — filename is `001-arkos.md`.
- [x] Workflow-native vocabulary only; OS-technical jargon excluded from body: PASS — `grep -nE` for `syscall|kernel-equivalent|memory manager|process abstraction|scheduler` hits only line 33, which is the explicit "terms to avoid" callout.
- [x] Self-improvement section names grounding-signal dichotomy with citations: PASS — DGM reward-hacking, AutoGPT loops, Augment 2026 measurement, Panickssery 2024 self-preference all cited.
- [x] Open Questions enumerates Q1–Q6 without resolving: PASS — six questions named, each with framing and unresolved-status.
- [x] References section links three research files: PASS — `grep -c "research/" docs/rfcs/001-arkos.md` reports 4 (preamble + 3 named files).
- [x] No `README.md` / `AGENTS.md` / source-code / template / SPEC changes: PASS — `git status` shows only `docs/rfcs/001-arkos.md` and `.ark/tasks/rfc001-arkos/` artifacts.

## Plan Fidelity

- [x] G-1: RFC frames ArkOS as substrate, not orchestrator: PASS — Summary, *Layered model*, *ArkOS — what it is* (explicit "ArkOS is **not** an autonomous orchestrator").
- [x] G-2: RFC frames Ark and ArkOS as peers at substrate layer: PASS — Layered diagram shows peers; *Relationship to Ark* contains "**peer, not stack**" and the "what will not happen" commitment.
- [x] G-3: RFC commits ArkOS's self-improvement to workload-grounded evolution; forbids self-grading and self-harness-editing: PASS — *Self-improvement model* §"ArkOS's discipline" enumerates the four rules including "the substrate cannot edit its own evaluation harness."
- [x] G-4: RFC names six first-class open questions: PASS — Q1 Goodhart, Q2 discoverability, Q3 runtime-dependency, Q4 intermediate-artifact grounding, Q5 recursive context, Q6 sibling interference; none resolved.
- [x] G-5: Workflow-native vocabulary; no OS-technical jargon in body: PASS — see PRD-constraints check above.
- [x] G-6: References section links three research files: PASS — see PRD-constraints check above.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no feature SPECs were modified by this task.

## Findings

(No findings. Single-file documentation delivery; the PLAN/RFC structure and the checks above cover the surface area.)

## Notes

- The RFC was authored after the design conversation surfaced and corrected the "ArkOS = autonomous orchestrator" misread; the substrate framing is the load-bearing reframe. Future revisions should preserve this distinction or explicitly argue why it should change.
- Three persisted research files (under `research/`) carry the citation evidence behind the *Self-improvement*, *Open questions*, and *Prior art* sections. They are not summarized in the RFC body beyond a brief preamble in *References*; readers wanting the full prior-art landscape go to the files.
- The deep → standard tier promotion mid-flight was a deliberate call: the deep-tier ceremony was justified by the research depth (three parallel ark-researcher dispatches), but the SPEC promotion deep-tier produces would have been misleading because "RFC about ArkOS" is not a feature SPEC of Ark. The research persists with the task regardless of tier.
- REVIEW was skipped by user direction. The substrate framing has not been pressure-tested by a fresh reviewer. If the RFC is revised later, that's a natural time to run REVIEW against the next iteration.
