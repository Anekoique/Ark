# Start here: automatic session insight and skill retention for Ark

- Question: should Ark turn session/task implementation experience into
  durable insights and procedural skills, and where should automation stop?
- Date: 2026-07-29
- Status: research recommendation, not an implementation specification

## Answer

Yes—but Ark should initially automate **evidence collection and candidate
extraction**, not the silent publication of model-authored guidance.

The recommended first loop is:

1. At explicit task closeout, assemble a bounded evidence packet from task
   artifacts, the implementation diff, review/VERIFY results, corrections,
   failed paths, and any harness-provided trace references.
2. Let an agent emit zero or more typed candidates: episode, advisory insight,
   procedural skill, SPEC-change proposal, user preference, or discard.
3. Retain candidates task-locally with provenance. “No durable lesson” is a
   valid result.
4. Reconcile candidates against the existing project library and stage a
   source-linked diff.
5. Activate only reviewed entries. Require stronger outcome evidence before a
   procedural skill graduates than before an advisory insight does.
6. Surface a compact index; load full bodies selectively. Current repository
   evidence and active SPECs always outrank retained guidance.

The Rust CLI should continue to own deterministic data mutation and state
transitions. The agent workflow should own semantic extraction and
consolidation. This preserves Ark's existing “binary owns data; agent owns
judgment” boundary.

## Why this boundary

The strongest prior art agrees on the value of extracting reusable signal, but
does not establish that one model-generated procedure should immediately steer
future work:

- [Hermes Agent][hermes] is the closest product precedent: it separates
  bounded always-on memory, on-demand skills, and searchable session history;
  it can create and evolve skills and offers optional staged approval. The
  absence of a mandatory transfer test before every autonomous write is a
  reason for Ark to adopt a stricter default.
- [Stello][stello] carefully separates raw records, outward-facing per-session
  memory, one-shot insight, shared memory, and authored skills. Its automatic
  behavior consolidates raw records into a digest; cross-session reflection is
  application-owned, and current source does not extract skills from sessions.
- [Oh My Pi][omp] demonstrates a robust two-pass shape: extract per-rollout
  durable signal, then consolidate it into memory and skill packages, with
  project isolation, retries, watermarks, and a first-class no-signal result.
  Its whole-library model rewrite and omission-based pruning are too risky for
  an approved Ark library.
- [Devin][devin] automatically diagnoses sessions but documents reusable
  Playbook creation and testing as a separate, deliberate workflow.
- Research systems show that experience and induced workflows can improve
  agents, especially when grounded in environment feedback or multiple
  trajectories. They also show retrieval bias, overfitting, and lifecycle gaps.
- [SkillsBench v4][skillsbench] reports a +16.6-point mean gain from curated
  skills, yet 13 of 87 tasks regress; self-generated packs fall below the
  no-skill baseline in all three tested model-harness configurations.
- A 2026 [ACL Findings safety study][safety] reports that even benign
  accumulated experience can degrade safety in high-risk tasks because
  procedural experience pushes agents toward action. Experience is therefore
  executable influence, not harmless documentation.

This evidence supports an asymmetric rule:

```text
low-risk automation                     high-risk automation

capture -> extract -> classify -> stage -> validate -> activate -> auto-load
   yes        yes        yes       yes      gated       later       later
```

## Ark's missing artifact, precisely

Ark already preserves:

- normative project/feature constraints in SPECs;
- task intent and implementation evidence in PRD/PLAN/REVIEW/VERIFY;
- source-backed investigation in research corpora;
- episodic closure in the journal and archive index;
- workflow procedures in shipped Ark skills.

It does **not** preserve a typed, project-local layer for:

- advisory implementation insights such as a stable pitfall or workaround;
- reusable procedures with applicability, contraindications, and
  verification;
- source-linked candidates waiting for validation and promotion;
- retrieval/use feedback, staleness, supersession, or deprecation.

A raw transcript is not that layer. It is evidence from which a candidate can
be derived. A journal entry is not that layer either: it says what happened,
not when and how a procedure should be reused. A SPEC is more authoritative
than that layer: it says what must hold and must never be silently rewritten by
experience extraction.

## Artifact routing

Every extracted item should have exactly one semantic owner:

| Content | Owner | Default authority |
| --- | --- | --- |
| Raw messages and tool results | Harness session store | Evidence only |
| Task outcome and narrative | Journal/archive | Historical |
| Stable project fact or pitfall | Project insight | Advisory |
| Repeatable method with a verifier | Project skill | Procedural |
| Required invariant | SPEC-change proposal, then normal SPEC workflow | Normative only after approval |
| Personal preference | User/harness memory | Cross-project, outside project Ark |
| Secret, transient state, or unsupported claim | Discard | None |

Do not duplicate the same claim across stores. If routing is ambiguous, retain
one pending candidate and force the reviewer to choose.

## Recommended trigger model

### MVP trigger: explicit task closeout

Place extraction in the agent-side `ark-commit` workflow immediately before
the current atomic CLI commit operation.

This boundary is better than a generic session-end hook because it is:

- portable across Claude Code, Codex, and future harnesses;
- explicit—the user already activated an Ark workflow;
- outcome-aware—VERIFY, review, tests, and the final diff exist;
- provenance-rich—the source task and eventual commit are identifiable;
- retryable and reviewable rather than a hidden background mutation.

### Optional later triggers

- Pre-compaction or session-end adapters may capture task-local observations.
  They must not activate durable entries.
- A periodic archive reflector may find repeated evidence across completed
  tasks and propose merges or promotion.
- Explicit “learn this workflow” remains useful when the user wants to
  generalize current work, even if automatic extraction exists.

## Candidate and skill contract

Every candidate needs:

- kind, scope, status, summary, and applicability;
- source task/artifact/commit or narrowly scoped session reference;
- evidence strength separate from lifecycle status;
- project, platform, and toolchain limits where applicable;
- creation and last-validation metadata;
- merge, supersede, rejection, and deprecation history.

A procedural body additionally needs:

- when to use and when not to use;
- preconditions and permissions;
- ordered steps and decision points;
- known failed paths, pitfalls, and recovery;
- observable verification/postconditions.

Model confidence is not validation. For the first implementation, a skill
should become active only after human approval and either a deterministic
verifier or success across two materially distinct task instances. Exclude
generated scripts and executable support files until their security model is
designed separately.

## Storage and recall

Keep a fixed, git-tracked project layout with:

```text
compact index
  -> selected insight/skill body
    -> provenance and source evidence on demand
```

Do not commit transcripts or start with a vector database. The first corpus
should be small enough for metadata/keyword filtering, and ordinary git diffs
provide review, rollback, and conflict visibility.

Retrieval precedence should be:

```text
current code/config/runtime
  > active SPECs
  > current task artifacts
  > approved insights and skills
  > archived episodes and raw traces
```

Only active entry metadata belongs in the small default index. Load a body on
explicit invocation, a high-quality applicability match, or a relevant
failure/search. Stello's shipped eager global body injection is specifically
not the model to copy.

## Staged implementation recommendation

### Phase 0: shadow candidates

- Extract at task closeout.
- Store task-local candidates or an explicit empty result.
- Publish and inject nothing.
- Measure volume, duplication, provenance, secret/injection findings,
  acceptance, edits, rejections, and reviewer time.

### Phase 1: reviewed project insights

- Add the fixed project index and per-entry bodies.
- Stage source-linked diffs with approve/reject/merge.
- Surface metadata through read-only context and load bodies on demand.
- Keep entries advisory.

### Phase 2: text-only skills

- Add applicability, contraindications, verification, use feedback, and
  deprecation.
- Require outcome evidence and approval before activation.
- Evaluate each skill against the same task/model/harness with and without it.

### Phase 3: evolution

- Propose patches or retirement after observed failures or drift.
- Consolidate repeated evidence across archived tasks.
- Consider automatic retrieval only after precision, safety, and utility gates
  pass.

## Evaluation gate

Use archived Ark tasks with reconstructable repository states for matched
replay:

1. no retained entry;
2. approved relevant entry;
3. irrelevant entry;
4. stale or adversarial entry.

Measure VERIFY/test success, task correctness, tool calls, tokens/time,
repeated dead ends, retrieval precision, unsafe action attempts, context cost,
review burden, and stale-entry regressions. Structural validity, procedural
validity, and transfer validity are separate gates.

Do not enable automatic project-wide publication until the evaluation shows
positive utility, bounded false positives and reviewer work, no secret or
prompt-injection leakage, and no meaningful safety regression.

## Adopt / adapt / reject

### Adopt

- Hermes's memory/skill/session separation, progressive disclosure, diffable
  pending writes, and recoverable lifecycle.
- Stello's separate evidence/digest/one-shot/shared/skill planes and
  application-owned reflection.
- Oh My Pi's two-stage extraction/consolidation, no-signal output, project
  isolation, and deterministic job controls.
- Outcome grounding, negative evidence, and cross-trajectory comparison from
  research systems.

### Adapt

- Run extraction at Ark's semantic task boundary, not every harness turn.
- Turn background product writes into project-local candidates with stricter
  approval.
- Reconcile with entry-scoped patches, not whole-library regeneration.
- Treat one-shot insight as a targeted handoff mechanism, not durable memory.
- Compile a harness-neutral approved procedure into harness-specific skill
  adapters only after the source artifact exists.

### Reject for the first implementation

- Direct transcript-to-active-skill publication.
- Automatic SPEC mutation.
- Eager injection of all retained bodies.
- Provenance-free `{name, body}` entries.
- Silent overwrite, omission-based deletion, or fire-and-forget extraction.
- Generated scripts, cross-project sharing, vector storage, and autonomous
  background rewrites.

## Corpus guide

- [01-hermes-learning-loop.md](01-hermes-learning-loop.md) — exact Hermes
  triggers, stores, write gates, retrieval, lifecycle, and observed failure
  modes.
- [02-stello-and-ark-memory-model.md](02-stello-and-ark-memory-model.md) —
  source-checked Stello planes and the current Ark artifact gap.
- [03-prior-art-and-research.md](03-prior-art-and-research.md) — Oh My Pi,
  claude-mem, Devin, and research-system comparison.
- [04-ark-design-options-and-evaluation.md](04-ark-design-options-and-evaluation.md)
  — concrete candidate schema, routing, security controls, rollout, and
  evaluation plan.

## Follow-up design question

The next `/ark:design` task should be intentionally narrow:

> Build Phase 0 shadow extraction at agent-driven Ark task closeout, with a
> typed, task-local, provenance-bearing candidate artifact and explicit
> no-signal result. Do not add active project memory/skill retrieval yet.

That task will need to decide the candidate file schema and path, how each
harness provides bounded trace references, whether extraction is mandatory or
best-effort at closeout, and how the eventual commit hash is attached without
violating the current atomic commit contract.

[hermes]: https://hermes-agent.nousresearch.com/docs/user-guide/features/skills
[stello]: https://github.com/stello-agent/stello
[omp]: https://github.com/can1357/oh-my-pi/tree/main/packages/coding-agent/src/memories
[devin]: https://docs.devin.ai/use-cases/gallery/create-playbook-from-session
[skillsbench]: https://arxiv.org/html/2602.12670v4
[safety]: https://aclanthology.org/2026.findings-acl.2091/
