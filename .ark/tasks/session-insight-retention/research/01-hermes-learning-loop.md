# Research: Hermes Agent learning loop

- Query: Reconstruct Hermes Agent's automatic extraction and retention loop across session traces, bounded memory, skills, review nudges, approval, security, and lifecycle maintenance.
- Scope: external
- Date: 2026-07-29

## Findings

### Source status

This note uses Hermes Agent's current public documentation and issue tracker as
of 2026-07-29. The documentation is a living `main` snapshot rather than a
version-pinned release manual. Statements labeled **documented behavior** come
from the official documentation. Statements labeled **issue evidence** are
reports in the project's tracker and may describe an older or subsequently
changed implementation.

### The closed loop is three stores, not one memory

Hermes separates raw evidence, compact always-on facts, and reusable procedures:

| Layer | Persistence | Prompt behavior | Mutation / retrieval |
| --- | --- | --- | --- |
| Session trace | `~/.hermes/state.db`, including `sessions`, `messages`, and FTS5 `messages_fts` | Not injected wholesale | Every conversation is stored automatically. `session_search` retrieves actual messages on demand with keyword search and local context windows. |
| Bounded memory | `~/.hermes/memories/MEMORY.md` (2,200 chars) and `USER.md` (1,375 chars) | Both are injected as a frozen system-prompt snapshot at session start | The model calls `memory` with `add`, `replace`, or `remove`. Writes persist immediately but do not alter the frozen prompt until the next session. |
| Procedural skills | `~/.hermes/skills/<name>/SKILL.md` plus optional support files | A metadata index is available first; full skill content loads only when relevant | The model calls `skill_view` to read and `skill_manage` to create, patch, edit, delete, or manage support files. `/learn` asks the foreground model to author a skill through the same tool. |

This is a meaningful distinction:

- Session history is **evidence**. Search returns source messages rather than a
  generated summary.
- `MEMORY.md` and `USER.md` are **small semantic selections** that pay a token
  cost in every session.
- Skills are **procedural packages** whose bodies are loaded lazily.

The official memory guide explicitly contrasts persistent memory with session
search: memory is for critical facts that should always be present; search is
for recovering specifics from prior conversations. The skills guide makes the
parallel distinction that memory holds compact durable facts while skills hold
longer procedures.

### End-to-end learning loop

#### 1. Capture is automatic; promotion is model-mediated

**Documented behavior:** all CLI and messaging conversations are recorded in
SQLite. This capture does not itself convert a trace into memory or a skill.
FTS5 discovery returns session bookends plus a window around a match; scrolling
can then retrieve adjacent messages. The search path performs no LLM
summarization.

Promotion from trace/context into a durable semantic artifact requires a model
tool call:

- The foreground agent may proactively save a preference, environment fact,
  correction, convention, or completed-work fact with the `memory` tool.
- The foreground agent may create or patch a skill with `skill_manage` after a
  complex successful task, a recovered dead end, a user correction, or discovery
  of a non-trivial workflow.
- `/learn` is an explicit user path for material already in the conversation,
  local files, a URL, or pasted notes. It is implemented as a normal agent turn:
  Hermes constructs an authoring prompt, the live agent gathers sources, and
  the agent writes through `skill_manage`. There is no separate deterministic
  ingestion engine.
- If an opt-in session reset is about to happen, Hermes gives the agent a turn
  to save important memories or skills first.

The “5+ tool calls” criterion in the skills guide is presented under “When the
Agent Creates Skills.” It is authoring guidance, not a schema-enforced proof
that a successful workflow is reusable.

#### 2. After-turn review supplies a nudge, not a compiler

**Documented behavior:** after a turn, a background self-improvement review may
replay the conversation and call only memory/skill tools. It can save a compact
fact, create a skill, or patch an existing skill, with a short user-visible
notification by default. The review can use the main model or a separately
configured `auxiliary.background_review` model.

The deterministic part is scheduling and tool restriction. The following
remain model decisions:

- whether anything is worth retaining;
- whether it is a user preference, environment/project fact, or procedure;
- whether to create a new artifact or patch an existing one;
- the exact abstraction, trigger wording, procedure, pitfalls, and verification
  content;
- whether two observations are meaningfully duplicates.

The public pages are not fully consistent about the exact nudge threshold.
The curator guide describes the background skill review as periodic, roughly
every ten agent turns. Issue #20273 reports a review after a conversation turn
with at least ten tool iterations, while the skills guide separately lists a
successful task with five or more tool calls as a skill-creation signal. These
should not be collapsed into one stable trigger without checking the source at
a pinned Hermes revision.

#### 3. Writes may be direct or staged

Hermes exposes independent approval gates:

| Store | Default | With approval enabled |
| --- | --- | --- |
| Memory | `memory.write_approval: false`; foreground and background writes land directly | Foreground CLI entries prompt inline. Messaging/script/background writes are staged and reviewed with `/memory pending`, `/memory approve <id>`, and `/memory reject <id>`. |
| Skills | `skills.write_approval: false`; every `skill_manage` mutation can land directly | Every create/edit/patch/delete/support-file mutation is staged under `~/.hermes/pending/skills/`, survives restarts, and is reviewed with `/skills pending`, `/skills diff <id>`, `/skills approve <id>`, and `/skills reject <id>`. |

The gate applies to `/learn` because `/learn` ultimately uses `skill_manage`.
For skills, the diff is the review unit. For memory, the individual compact
entry is the review unit.

Approval is distinct from content scanning:

- Memory entries are always described as scanned for injection, exfiltration,
  SSH-backdoor patterns, and invisible Unicode before acceptance.
- `skills.guard_agent_created` is a heuristic scanner for agent-authored skill
  writes. Current configuration documentation says it is off by default because
  legitimate workflows involving SSH paths or API-key names generated too many
  false positives.
- Hub-installed skills have a separate install-time scanner and trust policy.

These scanners classify suspicious content. They do not establish that a
captured procedure is correct, general, current, or useful.

#### 4. Retrieval is progressive

The skills guide documents three disclosure levels:

1. a name/description/category index;
2. full `SKILL.md` through `skill_view(name)`;
3. a specific referenced support file through `skill_view(name, path)`.

Full procedure bodies are therefore not permanent prompt content. MEMORY/USER
use the opposite tradeoff: they are tiny enough to stay always-on, but are
frozen for one session to preserve prefix caching.

Session retrieval is also progressive. Discovery returns a hit window and
beginning/end bookends, then the model can scroll around a selected message.
This preserves source evidence while bounding prompt cost.

### Automatic mechanics versus model judgment

| Step | Deterministic / system-owned | Model- or user-decided |
| --- | --- | --- |
| Trace capture | Store every message in SQLite; maintain FTS5 index | Whether and when to search it |
| Review activation | Periodic/threshold nudge and a background fork | Whether there is signal and which tool to call |
| Memory limits | Enforce 2,200/1,375 character caps; reject overflow and exact duplicates | Which entries to consolidate, replace, remove, or add |
| Skill creation | Validate tool action, path/package mechanics, optional approval staging | Applicability, abstraction level, procedure, pitfalls, verification |
| `/learn` | Convert the slash command into a normal guided turn; route writes through the standard gate | User chooses source/request; model researches and authors |
| Approval | Persist pending entries/diffs; apply or discard selected IDs | Human accepts or rejects |
| Security | Pattern scanner and trust-source policy | Human interpretation of warnings; semantic correctness remains unverified |
| Curation | Track usage; deterministic idle-state transitions; snapshot before mutation | Optional LLM consolidation decides keep/patch/merge/archive |

The main architectural fact is therefore not “Hermes automatically compiles a
session into a verified skill.” It automatically preserves the trace and
periodically asks a model to curate semantic memory or procedural artifacts.

### Curator lifecycle

Hermes adds a second, slower loop around skills:

1. An inactivity check runs at CLI session start and on a recurring gateway
   tick. Defaults require a seven-day interval and two idle hours.
2. A deterministic pass moves eligible skills from `active` to `stale` after
   30 unused days and to recoverable `archived` storage after 90 unused days.
   It does not automatically delete them.
3. An optional LLM consolidation pass can inspect agent-created skills and
   propose patches, umbrella skills, consolidation, or archival. This pass is
   off by default (`curator.consolidate: false`).
4. Each real run snapshots the skill tree. The CLI supports dry run, status,
   backup, rollback, pause/resume, pin/unpin, adopt, archive, restore, and
   listing unmanaged/archived skills.

The curator uses a usage sidecar (`~/.hermes/skills/.usage.json`) with view,
use, patch, and timestamp counters. Its scope is provenance-sensitive:

- Current docs say skills created by the background review carry an
  agent-created marker and are curator-managed.
- Foreground skills created at the user's request are treated as user-directed
  and unmanaged until the user explicitly adopts them.
- Hub-installed skills are outside the curator's mutation scope.
- Pinned skills skip automatic transitions and LLM consolidation; archives are
  restorable.

The documentation makes an unusually useful distinction: the persisted
`created_by` field is consumed as a policy flag (“may autonomous curation touch
this?”), not reliable authorship provenance. Manual adoption changes authority,
not history.

### Concrete failure evidence

These are project issue reports, not proof that every report still reproduces
on the current head.

#### Store misclassification and “save something” pressure

[Issue #30220](https://github.com/NousResearch/hermes-agent/issues/30220)
reports that the background prompts pressured the reviewer to be active and
discouraged “Nothing to save.” The reporter observed:

- false-positive saves when no durable signal existed;
- duplication of one preference into both memory and skills;
- confusion between `USER.md`, `MEMORY.md`, and procedural skills;
- a bias toward the cheaper `memory(add)` action instead of authoring a full
  skill when both routes were available.

This is direct evidence for a classification failure mode: a typed store does
not guarantee correct routing when one model prompt owns every target and is
rewarded for emitting something.

#### Automation presented with user-role authority

[Issue #25839](https://github.com/NousResearch/hermes-agent/issues/25839)
reports that a background review instruction was inserted as a `role: "user"`
message. A parallel agent interpreted it as an owner command and patched a
carefully maintained skill without direct consent. The issue also reports that
curator pinning protected against curator mutation but did not, in that
version, protect against the separate review-agent path.

The relevant failure is provenance at the instruction boundary: a correct
patch can still be unauthorized if an automation request is indistinguishable
from a human request.

#### Mutation scope and prompt-only boundaries

[Issue #20273](https://github.com/NousResearch/hermes-agent/issues/20273)
reported that background review and curator agents could modify bundled or
hub-installed skills through `skill_manage`, with protection depending on
prompt instructions rather than a tool-level boundary. The current curator
documentation now describes hub skills as off-limits and a provenance-based
managed set, so the report should be read as historical failure evidence, not a
current capability statement. It still demonstrates why mutation authority
must be enforced below the reviewing model.

### Limits of the Hermes model

- The trace-to-artifact classifier is an LLM prompt and tool choice, not a
  verifier. Successful execution, reuse across contexts, and procedure
  generality are not established merely by saving.
- Write approval is available but off by default for both memory and skills.
  With defaults, an after-turn model can change future prompt state without a
  human diff review.
- Memory exact-duplicate rejection is not semantic deduplication. Broader
  overlap control is deferred to model judgment and the slower curator.
- FTS5 session search is lexical, not semantic. This makes evidence retrieval
  cheap and inspectable, but wording changes can reduce recall.
- MEMORY/USER are global to a Hermes profile and frozen per session. Current
  docs do not expose per-entry source pointers, validation commits, expiry, or
  confidence fields.
- Skill usage telemetry measures viewing, loading, and patching. It does not
  by itself measure task success or correctness.
- Time-based staleness is deterministic but only approximates value. Pinning,
  backups, dry runs, and restore reduce the cost of a wrong archival decision.
- The agent-created skill scanner is heuristic and off by default; approval and
  filesystem/process permissions remain separate policy layers.
- Official documentation is living and several issue reports refer to
  implementation details that have changed. A production comparison needs a
  pinned commit or release.

### Transferable questions for Ark

The Hermes design surfaces options Ark can evaluate without assuming the same
answers:

| Hermes lesson | Ark design question |
| --- | --- |
| Raw session history remains source evidence; compact memory and skills are derived stores. | Should Ark retain only provenance pointers/digests to harness traces while keeping derived artifacts in the repository? |
| Memory facts are always-on and bounded; procedures are indexed then lazily loaded. | Which Ark insight types deserve prompt residency, and which should be discoverable bodies? |
| Capture is automatic, but semantic promotion is a model judgment. | Should Ark automate candidate extraction while making activation a separate transition? |
| Foreground `/learn` and background review share the same write gate. | Can explicit task closeout and opportunistic session review produce one common candidate/diff format? |
| “Nothing to save” pressure caused reported false positives and cross-store duplication. | Can no-op be first-class and can one candidate have exactly one owning store? |
| Curator authority depends on an explicit policy marker, not inferred authorship. | Should Ark track both historical provenance and an independent “may agents mutate this” policy? |
| Curator lifecycle uses usage, reversible archival, backups, pins, and optional model consolidation. | What combination of validation evidence, use feedback, age, supersession, and explicit pinning should govern an Ark skill lifecycle? |
| Pattern scanning and approval are independent. | Which checks are security gates, which are human authority gates, and which establish procedure correctness? |
| Background review failures involved prompt authority and tool-level mutation scope. | Can background analysis be restricted to producing candidates while repository-wide activation remains an explicit, auditable workflow event? |

## External references

- [Hermes Agent — Skills System](https://hermes-agent.nousresearch.com/docs/user-guide/features/skills) — on-demand skill storage, `/learn`, progressive disclosure, creation signals, `skill_manage`, and skill write approval.
- [Hermes Agent — Persistent Memory](https://hermes-agent.nousresearch.com/docs/user-guide/features/memory/) — MEMORY/USER limits, frozen injection, memory tool actions, security scanning, background review, and approval.
- [Hermes Agent — Sessions](https://hermes-agent.nousresearch.com/docs/user-guide/sessions) — SQLite/FTS5 trace storage, source-message retrieval, reset-time save turn, and retention behavior.
- [Hermes Agent — Curator](https://hermes-agent.nousresearch.com/docs/user-guide/features/curator) — usage telemetry, provenance-based mutation scope, deterministic staleness/archive transitions, optional LLM consolidation, backups, pins, and restore.
- [Hermes Agent — Configuration](https://hermes-agent.nousresearch.com/docs/user-guide/configuration) — current defaults and the distinction between approval gates and content scanning.
- [Hermes Agent issue #30220 — store misclassification](https://github.com/NousResearch/hermes-agent/issues/30220) — reported false positives, duplication, and weak “nothing to save” handling.
- [Hermes Agent issue #25839 — automation/user-role confusion](https://github.com/NousResearch/hermes-agent/issues/25839) — reported unauthorized skill mutation caused by background instruction provenance.
- [Hermes Agent issue #20273 — autonomous mutation scope](https://github.com/NousResearch/hermes-agent/issues/20273) — historical report about prompt-only boundaries around bundled/hub skill mutation.

## Caveats / Not found

- No versioned Hermes release documentation was found for this current feature
  set; exact defaults and trigger thresholds may drift with `main`.
- The public pages do not give one unambiguous current formula for background
  review activation. The “5+ tool calls,” “about every ten turns,” and issue
  report of “at least ten tool iterations” refer to different layers or
  revisions.
- No documented per-memory-entry provenance schema, confidence score, expiry,
  validation result, or source-session pointer was found in the built-in
  MEMORY/USER format.
- No deterministic trajectory-to-skill generalization or correctness verifier
  was found. The documented extraction path is model review plus tool calls,
  optionally followed by human approval and later curation.
- Issue evidence was not independently reproduced against a pinned checkout.
