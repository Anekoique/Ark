# `rfc001-arkos` PLAN

> Status: Approved for Implementation
> Feature: `rfc001-arkos`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none (standard tier; promoted from deep mid-flight, REVIEW skipped by user direction)

---

## Summary

Add `docs/rfcs/001-arkos.md` positioning ArkOS as a workflow substrate for agents (peer to Ark at the same architectural layer), grounded by external workload outcomes rather than self-evaluation. Establishes `docs/rfcs/` as the home for future numbered RFCs. The substantive design work happened in conversation and three persisted research files (`research/self-improving-agents.md`, `research/recursive-decomposition.md`, `research/self-generating-specs.md`); the PLAN below documents the RFC's structural claims so a future reader can audit what was committed without re-reading the discussion transcript.

## Log

None (00_PLAN, no prior iteration).

---

## Spec

[**Goals**]

- G-1: RFC frames ArkOS as a **substrate** for agents, not an autonomous orchestrator.
- G-2: RFC frames Ark and ArkOS as **peers** at the substrate layer, not stacked.
- G-3: RFC commits ArkOS's self-improvement to **workload-grounded** evolution, with the substrate forbidden from grading itself or editing its own evaluation harness.
- G-4: RFC names **six first-class open questions** (Goodhart drift, agent discoverability, runtime-dependency stability, intermediate-artifact grounding, recursive context, sibling interference) without resolving them.
- G-5: RFC uses **workflow-native vocabulary** ("substrate," "service," "primitive," "lifecycle," "grounding") and avoids OS-technical jargon ("syscall," "kernel," "scheduler," "memory manager," "process abstraction").
- G-6: RFC links the three persisted research files so future readers can reach the evidence behind the *Self-improvement*, *Open questions*, and *Prior art* sections.

[**Non-goals**]

- Designing ArkOS's substrate-API shape (deferred to ArkOS's own RFC in `Anekoique/arkos`).
- Committing Ark to specific stability tiers on `ark agent` (named as Q3 / open).
- Editing `README.md` or `AGENTS.md` (descoped from PRD per user direction; identity-pinning can be a follow-up task).

[**Architecture / Data Structure / API Surface**]

N/A — documentation-only delivery; no executable artifacts, no schema changes, no API.

[**Constraints**]

- C-1: Single file delivered at `docs/rfcs/001-arkos.md`. No other source / template / SPEC file is created or modified.
- C-2: Filename uses three-digit prefix (`001-`), not four-digit, per user direction during DESIGN.
- C-3: RFC length ~250–600 lines; current 370 fits.
- C-4: "OS" appears in the title and summary as a positioning metaphor; the body uses workflow-native vocabulary exclusively.

---

## Runtime

Single phase: write the RFC. No state transitions, no failure paths, no integration.

---

## Implementation

1. `mkdir -p docs/rfcs` — already done.
2. Write `docs/rfcs/001-arkos.md` per the structure agreed during DESIGN (Status / Summary / Motivation / Layered model / Ark's identity / ArkOS — what it is / ArkOS — what it provides / Self-improvement model / Two-stage evolution / Relationship to Ark / Out of scope / Open questions / Prior art / Phased delivery / References). — already done.

---

## Trade-offs

- Deep tier was the initial choice, motivated by the research depth (three parallel ark-researcher dispatches landed ~800 lines of cited prior-art surveys). The deep-tier ceremony's payoff — REVIEW iteration and feature-SPEC promotion — does not apply here: the RFC is itself the design artifact (no separate code), and ArkOS is not a feature of Ark (the promoted SPEC would have been misleading). Promoting back to standard mid-flight matches the actual shape of the deliverable.
- Skipping the REVIEW loop is a user-direction call. The risk is that no fresh-perspective reviewer pressure-tested the substrate framing; the mitigation is the three persisted research files, which contain the evidence base a future reviewer (or revision of this RFC) would consult.

---

## Validation

[**Unit / Integration / Failure / Edge**]

N/A — documentation-only.

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1 | V-1: RFC's Summary, Layered model, and "ArkOS — what it is" sections explicitly state substrate, not orchestrator. |
| G-2 | V-2: Layered model diagram and Relationship-to-Ark section state peer relationship; "what will not happen" subsection commits Ark to not absorbing ArkOS. |
| G-3 | V-3: Self-improvement model section names the grounding-signal dichotomy, commits ArkOS to workload-outcome grounding, forbids self-grading and self-harness-editing. |
| G-4 | V-4: Open questions section enumerates Q1–Q6 with framing; none is marked resolved. |
| G-5 | V-5: Body contains no occurrence of "syscall," "kernel-equivalent," "scheduler," "memory manager," "process abstraction." "OS" appears only in title, summary, and positioning context. |
| G-6 | V-6: References section links all three files under `.ark/tasks/rfc001-arkos/research/`. |
| C-1 | V-7: Single new file under `docs/rfcs/`; no other tracked changes in `git status`. |
| C-2 | V-8: Filename is `001-arkos.md` exactly. |
| C-3 | V-9: `wc -l docs/rfcs/001-arkos.md` between 250 and 600. |
| C-4 | V-10: Same as V-5; vocabulary check. |
