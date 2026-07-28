# Ark design options: from task evidence to retained insight

- Question: how should Ark extract and retain session-derived knowledge without
  turning model output into unreviewed project truth?
- Scope: design synthesis over the Hermes, Stello, prior-art, and research
  tracks in this corpus
- Date: 2026-07-29
- Evidence labels:
  - **Fact** describes current Ark behavior or a cited external source.
  - **Inference** connects evidence to Ark but is not directly claimed by a
    source.
  - **Recommendation** is a proposed boundary for a later design task.

## Decision

**Recommendation:** automate the extraction of *candidates*, not their
publication as active memory, reusable skills, or project constraints.

The most reliable portable extraction boundary is the agent-driven Ark task
closeout, immediately before `ark agent task commit`. At that point the agent
has the task artifacts, implementation diff, review/verification evidence,
user corrections, and failed approaches. The Rust CLI should remain a
deterministic data actuator: it may validate, move, index, or atomically commit
artifacts, but it should not call an LLM or decide what was learned.

This produces the following loop:

```text
session/tool trace ─┐
task artifacts ─────┼─> agent extracts typed candidates
diff + VERIFY ──────┤            │
user corrections ──┘            v
                          task-local review queue
                                   │
                           human/agent validation
                                   │
                   ┌───────────────┼───────────────┐
                   v               v               v
                insight          skill         SPEC proposal
             (advisory fact)  (procedure)   (normative rule)
                   │               │               │
                   └──────── index + lazy retrieval ┘
```

“Nothing durable was learned” is a successful and expected result. The
extractor must not be pressured to populate every store.

## 1. Why the task boundary is better than every session boundary

### Available triggers

| Trigger | Evidence available | Portability | Failure mode | Call |
| --- | --- | --- | --- | --- |
| Every tool result | Very local | Harness-specific | Noise, secrets, premature conclusions | Reject |
| End of each turn | Recent dialogue | Harness-specific | False positives; live-store churn | Reject for publication |
| Before compaction | Context about to be lost | Harness-specific | Timing is implementation-dependent | Optional observation capture |
| Session end | Whole transcript | Weak across harnesses | Session can end mid-task or never close cleanly | Optional observation capture |
| Ark phase transition | Phase artifacts | Strong | Some discoveries occur outside formal tasks | Useful secondary trigger |
| Ark task closeout | Artifacts, diff, tests, review, outcome | Strong | Misses unfinished-task knowledge | Adopt for MVP |
| Periodic archive reflection | Multiple completed tasks | Strong | Delayed feedback and synthesis cost | Adopt later for consolidation |

**Fact:** Hermes can run a background self-improvement review after a turn and
can create or update skills; its documentation also exposes approval gates for
memory and skill writes. Oh My Pi instead performs background extraction over
older, idle rollout files and then consolidates the extracted material. These
are useful implementation patterns, but both depend on harness-owned session
state.

Sources:

- [Hermes skills and agent-managed skill writes][hermes-skills]
- [Hermes persistent memory and write approval][hermes-memory]
- [Oh My Pi memory implementation][omp-memory]

**Inference:** Ark cannot assume that Claude Code, Codex, or a future harness
will expose identical turn, compaction, and transcript hooks. Ark *does* own
task lifecycle semantics across integrations. A closeout step is therefore the
first trigger that is both evidence-rich and portable.

**Recommendation:** add candidate extraction to the `ark-commit` agent
workflow, before the existing atomic CLI closeout. Do not hide an LLM call
inside `ark agent task commit`. Optional harness adapters may later write
task-local observations at compaction or session end, but those observations
must remain evidence until the closeout extractor evaluates them.

## 2. One candidate, one semantic owner

The hardest problem is routing, not summarization. A discovery copied into a
journal, insight store, skill, and SPEC will diverge and then compete during
retrieval.

| Kind | Question it answers | Lifetime and authority | Ark destination |
| --- | --- | --- | --- |
| Raw trace | What exactly happened? | Ephemeral evidence; harness-owned | Session store, not git |
| Observation | What might matter later? | Task-local, unvalidated | Candidate queue |
| Episode | What happened in this task? | Historical, outward-facing | Journal/archive summary |
| Insight | What stable fact or pitfall helps later work? | Advisory; current repo outranks it | Project insight entry |
| Skill | How should a repeatable procedure be performed? | Procedural; invoked when applicable | Project skill entry |
| SPEC rule | What must future work preserve or satisfy? | Normative project contract | Proposed SPEC change |
| User preference | How does this person prefer agents to work? | Cross-project/personal | Harness/user memory, not project Ark |
| Ephemera or secret | What is transient or unsafe to retain? | No durable value | Discard |

### Routing tests

1. If it is primarily a narrative of one task, write only the journal/archive
   summary.
2. If it remains useful after the task but is not a procedure, propose an
   insight.
3. If it has a recognizable trigger, ordered actions, and a verifiable
   postcondition, propose a skill.
4. If violating it should fail review regardless of task or agent, propose a
   SPEC amendment rather than an insight.
5. If it names a user preference that is not a repository property, leave it
   to the user's cross-project memory system.
6. If its value depends on a temporary path, token, credential, uncommitted
   state, or one machine without a stable scope, discard it or record only a
   redacted task-local pointer.
7. If two destinations seem equally correct, keep one candidate in review and
   require the reviewer to choose a single owner.

**Recommendation:** prohibit automatic SPEC edits. A candidate may explain why
a SPEC amendment is warranted and point to evidence, but the normal Ark design
or spec workflow must own the normative change.

## 3. Candidate contract and provenance

An insight or skill without provenance becomes folklore. Provenance should be
small enough to review, yet sufficient to recover the evidence.

### Common candidate fields

```yaml
id: package-clean-after-rename
kind: skill
status: proposed
scope: project
summary: Clear stale package-scoped incremental state after a crate rename.
applicability:
  repository: this-project
  platforms: [macos, linux]
  toolchains: [cargo]
evidence:
  tasks: [rename-qemuctl]
  commits: []
  artifacts:
    - .ark/tasks/archive/.../VERIFY.md
  session_refs: []
confidence: observed-once
created_at: 2026-07-29
last_validated_at:
last_validated_commit:
supersedes: []
```

The body of a procedural candidate should contain:

1. **When to use** — positive triggers.
2. **When not to use** — contraindications and scope limits.
3. **Preconditions** — repository state, tools, platform, permissions.
4. **Procedure** — the smallest successful sequence, including decision
   points rather than a copied transcript.
5. **Pitfalls and recovery** — relevant failed paths and how to recognize them.
6. **Verification** — observable postconditions or commands.
7. **Evidence** — task/artifact/commit pointers and whether the result was
   observed, inferred, or merely suggested.

### Confidence and status are separate

Suggested evidence strength:

```text
suggested
  -> observed-once
  -> repeated
  -> deterministically-verified
```

Suggested lifecycle:

```text
proposed -> approved -> active -> deprecated
                    \-> rejected
active -> proposed-update -> active
```

Evidence strength describes support for the claim. Lifecycle describes whether
Ark may surface it. A deterministically verified candidate can still be
rejected as too narrow, dangerous, or redundant.

**Recommendation:** require at least one of these before a skill becomes
active:

- a deterministic verifier passed against the candidate procedure; or
- the same procedure succeeded in two materially distinct task instances.

Human approval remains necessary in the initial design. Scripts or executable
assets should be excluded from the first version; their security and
portability surface is much larger than a text-only procedure.

## 4. Extraction and consolidation

### Stage A — bounded task extraction

The extractor reads only evidence associated with the closing task:

- PRD, PLAN, REVIEW, VERIFY, and research files where present;
- the staged or intended implementation diff;
- test, lint, or runtime evidence already captured by the task;
- explicit user corrections and retained task-local observations;
- a bounded portion of the session/tool trace when the harness exposes it.

It emits strict structured candidates or an empty list. Each claim must point
to a task artifact or a narrowly scoped trace reference. Text matching common
secret forms and invisible/prompt-injection content is rejected before review.

### Stage B — library reconciliation

A later consolidator compares candidates with the existing index and chooses
one of:

- `add`: novel, adequately scoped knowledge;
- `merge`: same procedure/fact with stronger or broader evidence;
- `patch`: active entry is directionally correct but stale or incomplete;
- `supersede`: materially different replacement;
- `drop`: duplicate, ephemeral, unsafe, unsupported, or already normative in a
  SPEC.

**Fact:** Oh My Pi demonstrates a practical two-stage pipeline: extract
rollout-local raw memory and summaries, then consolidate them into memory and
skill files. Its current implementation also regenerates the consolidated
library as a whole.

**Inference:** two-stage processing is worth adopting, but wholesale
regeneration is not. A model omission must never delete an unrelated,
previously approved Ark entry.

**Recommendation:** apply candidate-scoped patches against fixed,
git-trackable files. Every update should be reviewable as a normal diff and
recoverable through git. Concurrency should be resolved per entry rather than
through a global library rewrite.

## 5. Storage and retrieval

### Storage

Use a fixed project layout with a compact index and one body per entry. The
exact path is a follow-up design decision; the relevant invariant is:

```text
small metadata index -> selective body load -> source evidence on demand
```

Do not copy full transcripts into `.ark/`. A harness may retain them in its own
session database; Ark should keep only stable references or redacted evidence
digests. Do not introduce a vector database for the first version. The corpus
is initially small, git diffs matter, and keyword/metadata selection is easier
to audit.

### Retrieval precedence

```text
current code/config/test evidence
  > active project and feature SPECs
  > current task artifacts
  > approved project skills and insights
  > historical episodes and raw traces
```

An insight is advisory. A skill is a proposed method. Neither may override
current repository state or an active SPEC.

### Loading

Inject only index metadata such as name, summary, applicability, status, and
last validation. Load a full body when:

- the task explicitly invokes it;
- its applicability matches the current phase/repository/tooling; or
- the agent searches for help after a relevant failure.

**Fact:** Hermes documents progressive skill disclosure, while Stello's design
separates compact outward-facing memory from one-shot insight injection.

**Recommendation:** preserve Ark's existing index-plus-lazy-body discipline.
Do not put all retained procedures into every prompt. A one-shot task inbox may
be useful for an orchestrator handing a discovery to a child agent, but it is
not a substitute for a reviewed durable store.

## 6. Safety and failure containment

Experience is executable influence even when stored as Markdown. The threat
model therefore includes:

- credentials or personal data copied from terminal/tool output;
- prompt injection retained from web pages, issues, logs, or dependencies;
- a successful but unsafe workaround promoted because tests passed;
- commands overfitted to one host, branch, path, or tool version;
- stale procedures that silently conflict with current code;
- poisoning through repeated low-quality traces;
- duplicate entries that amplify one claim during retrieval;
- generated scripts whose effects exceed the reviewed prose.

Recent controlled research reports that experience-driven adaptation can
degrade agent safety even when the retained experience is benign, with
execution-oriented experience a particularly important driver. This makes
“it worked once” insufficient as a publication rule.

Source: [On Safety Risks in Experience-Driven Self-Evolving Agents][safety]

Minimum controls for an MVP:

1. Redact or reject secrets before model extraction and again before write.
2. Treat external content as evidence, never as instructions to the extractor.
3. Require explicit status transitions; only `active` entries are retrievable
   by default.
4. Display a unified diff and provenance during approval.
5. Preserve a first-class reject/no-candidate path.
6. Exclude generated executables and scripts.
7. Bind applicability to repository, platform, and toolchain when known.
8. Store last validation time and commit; flag rather than silently use stale
   entries.
9. Record retrieval and outcome so a failing skill can be patched or
   deprecated.
10. Keep deletion and replacement recoverable through git.

## 7. Evidence from current research

### What supports extraction

- ExpeL demonstrates extracting natural-language insights from prior agent
  experience and recalling both insights and trajectories for new tasks.
- Agent Workflow Memory induces reusable workflows from demonstrations and
  supports online workflow induction.
- CODESKILL and related 2026 work explore add/merge/drop/evolve policies over a
  compact procedural library.

Sources:

- [ExpeL][expel]
- [Agent Workflow Memory paper][awm]
- [Agent Workflow Memory implementation][awm-code]
- [CODESKILL][codeskill]

### What argues against ungated publication

SkillsBench v4 (2026-06-14) evaluates 87 tasks across eight domains. Curated
skills raise the mean pass rate by 16.6 percentage points, but 13 of 87 tasks
have a negative skill delta. Focused bundles of at most three skills outperform
larger sets. In a separate three-configuration condition, self-generated skill
packs fall 8.1–11.5 points below the no-skill baseline; the audit identifies
non-discovery, displaced task work, and confidently wrong generated guidance.
This is direct evidence that “a model can write a plausible skill” is not the
same as “the skill helps.”

Source: [SkillsBench][skillsbench]

The Devin documentation is another useful boundary: sessions can be analyzed
to create or refine playbooks, and skills can be suggested after learning, but
the documented product workflow retains explicit user authorship or approval.
It should not be cited as evidence that automatic playbook publication is
already solved.

Sources:

- [Create a playbook from a session][devin-session]
- [Devin skills][devin-skills]

**Recommendation:** the scientific and product evidence supports candidate
generation, targeted retrieval, and iterative evolution. It does not justify
autonomously installing every generated procedure.

## 8. Staged rollout

### Phase 0 — shadow extraction

- Run only at explicit Ark research/design/quick closeout.
- Emit task-local candidates and an empty-result marker.
- Do not inject or publish them.
- Measure candidate volume, duplicate rate, secret/injection flags, reviewer
  acceptance, and reviewer time.

Exit criterion: accepted candidates are consistently source-grounded and the
review burden is bounded.

### Phase 1 — reviewed project insights

- Add a fixed, git-tracked insight index and per-entry bodies.
- Support approve/reject/merge through an agent workflow and deterministic
  file operations.
- Surface index metadata through `ark context`; load bodies on demand.
- Keep insights advisory and project-scoped.

Exit criterion: prospective tasks retrieve relevant entries without measurable
context bloat or stale-entry regressions.

### Phase 2 — text-only procedural skills

- Add the richer skill contract, applicability matching, verification, and
  deprecation.
- Require explicit invocation or high-confidence applicability.
- Graduate only repeated or deterministically verified procedures.
- Continue to exclude executable support files.

Exit criterion: controlled replay and prospective use show an improvement over
the same model without the skill, with no safety or task-success regression.

### Phase 3 — feedback and optional automation

- Record which entry was loaded and whether verification passed.
- Propose patches or deprecation after failures.
- Consider periodic cross-task consolidation and harness-specific
  pre-compaction observation adapters.
- Consider automatic retrieval only after precision and safety gates are met.

## 9. Evaluation plan

### Offline replay

Build a benchmark from archived Ark tasks whose repository state can be
reconstructed. Run the same model and task under:

1. no retained insight/skill;
2. an approved relevant entry;
3. a plausible but irrelevant entry;
4. a stale or adversarial entry.

Measure:

- VERIFY and test outcomes;
- task completion rate and correctness;
- tool calls, tokens, elapsed time, and repeated dead ends;
- retrieval precision/recall;
- safety-policy violations and destructive-action attempts;
- stale-entry and irrelevant-entry regressions.

### Shadow-mode health

Track:

- candidates per task, including zero;
- accept, edit, merge, reject, and duplicate rates;
- time to review;
- provenance completeness;
- secret/injection detection rate;
- percentage later retrieved;
- post-retrieval verification success;
- entries patched, superseded, or deprecated.

### Graduation gate

A later implementation should not enable automatic project-wide publication
unless it demonstrates all of:

- no secret or prompt-injection leakage in the evaluation corpus;
- bounded false-positive and duplicate rates;
- an acceptable reviewer workload;
- positive task utility relative to the same model without retained knowledge;
- no statistically or operationally meaningful safety regression;
- correct invalidation or warning when code, platform, or toolchain scope
  changes.

## 10. Explicit non-goals for the first implementation

- A global personal-memory replacement.
- Committing raw session transcripts to the repository.
- A vector database or semantic retrieval service.
- Autonomous mutation of SPECs.
- Automatic installation of generated scripts or executable assets.
- Background rewriting or deletion of the live knowledge library.
- A universal session topology or orchestration engine.
- Cross-project sharing before project-local provenance and lifecycle work.
- Treating a successful task as proof that every extracted procedure is safe.

## Adopt / adapt / reject

| Source idea | Decision for Ark | Reason |
| --- | --- | --- |
| Hermes memory/skill distinction | Adopt | Separates short always-relevant facts from long on-demand procedures |
| Hermes foreground/background autonomous writes | Adapt | Use candidate staging; do not default to live publication |
| Hermes progressive skill disclosure | Adopt | Keeps prompt cost bounded |
| Hermes write approval and diff queue | Adopt, stricter by default | Git-tracked project knowledge deserves review |
| Stello one-shot insight | Adapt | Useful for parent/child handoff, not durable truth |
| Stello binary/data vs application/reflection split | Adopt | Matches Ark's current architecture |
| Oh My Pi two-stage extraction/consolidation | Adopt | Separates local evidence reading from library reconciliation |
| Oh My Pi whole-library regeneration | Reject | Omission must not delete approved knowledge |
| Devin session-to-playbook flow | Adapt | Explicit, source-driven creation is safer than invisible promotion |
| Fully automatic live skill evolution | Reject for MVP | Current evaluation and safety evidence do not support it |

## Sources

[hermes-skills]: https://hermes-agent.nousresearch.com/docs/user-guide/features/skills
[hermes-memory]: https://hermes-agent.nousresearch.com/docs/user-guide/features/memory
[omp-memory]: https://github.com/can1357/oh-my-pi/tree/main/packages/coding-agent/src/memories
[expel]: https://arxiv.org/abs/2308.10144
[awm]: https://arxiv.org/abs/2409.07429
[awm-code]: https://github.com/zorazrw/agent-workflow-memory
[skillsbench]: https://arxiv.org/html/2602.12670v4
[codeskill]: https://arxiv.org/abs/2605.25430
[safety]: https://aclanthology.org/2026.findings-acl.2091/
[devin-session]: https://docs.devin.ai/use-cases/gallery/create-playbook-from-session
[devin-skills]: https://docs.devin.ai/product-guides/skills
