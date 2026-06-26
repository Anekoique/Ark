# Synthesis: Stello's design, mapped onto Ark

- Query: what does Stello's session/task/memory organization teach an AI-native memory system for Ark?
- Scope: synthesis over corpus files 01–05 (read those for evidence + citations)
- Date: 2026-06-25

> This is the capstone. Files 01–05 hold the evidence with file:line citations.
> This file does the cross-walk: Stello concept → Ark analogue → **adopt / adapt / reject**.
> Read 01–05 first if you want the proof; read this if you want the conclusions.

---

## 1. The one-paragraph thesis

Stello separates three axes that Ark currently conflates or leaves implicit:
**(a) topology** — how a body of work branches (Stello: a session forest; Ark: a flat
task directory); **(b) memory scope** — *who* a piece of remembered knowledge belongs to
(Stello: agent-shared vs. per-session-persistent vs. per-session-one-shot; Ark: global
`MEMORY.md` vs. per-task `research/` vs. nothing for one-shot); and **(c) reflection
ownership** — *who* synthesizes across units (Stello: emphatically the application layer,
never the framework; Ark: undecided). Stello's most transferable lesson is not any one
data structure — it's the **discipline of keeping these three axes orthogonal**, and the
hard stance that the framework owns *pure-data IO* while *synthesis is an agent-driven loop
on top*. Ark already accidentally implements Stello's abandoned north-star for one of these
axes (the index + lazy-load memory file layout). The opportunity is to make the other two
axes deliberate.

---

## 2. Concept cross-walk

| Stello concept | What it is (see file) | Closest Ark analogue today | Call |
| --- | --- | --- | --- |
| Single-Session model (root==child, differ only by `parentId`) | 01 | Task dirs are flat; no parent/child | **Adapt** — see §3.1 |
| Session forest / topology decoupled from identity | 01, 05 | No topology; `tasks/<slug>/` siblings | **Adapt (lightweight)** |
| Fork synthesis chain (defaults→parent→profile→opts) | 01 | Tier templates seed a task; no inheritance | **Reject (for now)** |
| 3-slot per-session context (systemPrompt/insight/memory) | 02 | PRD/PLAN are persistent; no insight/one-shot slot | **Adopt the *distinction*** — §3.2 |
| Invariant: `memory` ≠ self-context (outward-facing) | 02, 03 | A task's own artifacts *are* re-read by the agent | **Adopt as a lens**, not a rule |
| Agent-shared memory `{slug, body}`, zero-domain-modeling | 03 | `MEMORY.md` + `memory/*.md` (slug + summary + body) | **Already have it** — §3.3 |
| Index-injected + body-lazy-load | 03 | `MEMORY.md` index, bodies in files, read on demand | **Ark already does Stello's *intended* model** |
| Orchestrator-facing data SDK (`listSessionDigests`…) | 03, 04 | `ark context --format json` | **Adopt the principle** — §3.4 |
| "Framework owns data IO, app owns reflection" | 03, 04 | `ark` binary is pure-data; agent does judgment | **Adopt — it's already Ark's spine** |
| Agent-as-orchestrator-client over a CLI (kitkit-cli) | 04 | `ark agent` namespace + slash commands | **Strong validation of Ark's shape** |
| Pluggable storage interfaces (SessionStorage/SessionTree) | 05 | Fixed `.ark/` filesystem layout | **Reject** — §3.5 |
| writeLock serialize-writes / dirty-reads | 03, 05 | File-per-topic sidesteps concurrency | **Hold** until parallel writers appear |

---

## 3. The decisions that matter

### 3.1 Topology — adopt *lightweight* parent/child links, reject the forest engine

Stello's payoff from topology is **scoped context isolation**: a branch carries only what it
inherited, and cross-branch information flows through one narrow channel (insight), not by
everyone reading everyone. Ark's tasks are already isolated (separate dirs, separate
worktrees on deep tier), but the *relationship* between tasks is only encoded in prose ("this
PRD references the research slug `…`") — file 04 and the research workflow both note Ark's
cross-over from research→implementation is a **prose pointer**, not a structured edge.

- **Adopt:** a structured, optional `parent`/`derived-from` field in `task.toml` (or a thin
  `tasks/INDEX.md` edge list). This is Stello's `sourceSessionId` idea at task granularity —
  cheap, makes "what did this task descend from" queryable, and lets `ark context` show
  lineage. This is the single highest-value, lowest-cost borrow.
- **Reject (for now):** the full fork-synthesis chain (config inheritance, ForkProfiles,
  systemPrompt merge modes). Ark tasks don't inherit *configuration*; they inherit *knowledge*,
  and knowledge already flows by the agent reading the parent's artifacts. Building a merge
  engine would be abstraction Stello itself only needs because it runs live LLM sessions.

### 3.2 Memory scope — adopt the three-way *distinction*, name the missing slot

Stello's sharpest contribution (file 02) is the strict separation of three lifetimes:

| Stello slot | Lifetime | Ark today | Gap |
| --- | --- | --- | --- |
| agent-shared memory | persistent, global | `MEMORY.md` + `memory/*.md` | ✅ present |
| per-session `memory` (L2) | persistent, unit-scoped, **outward-facing** | task `research/`, artifacts | ✅ present (but always self-read) |
| per-session `insight` | **one-shot inbox**, consumed-then-cleared | — | ❌ **no analogue** |

The missing piece is the **one-shot inbox**. Ark has no structured way to say "inject this
*once* into the next agent turn, then forget it." Today that need is met ad hoc by stuffing
instructions into a subagent dispatch prompt (file 04 calls this out). Whether Ark *needs* a
formal insight slot is open — but the corpus gives the vocabulary to decide deliberately
rather than by omission.

- **Adopt as a lens:** when designing Ark's memory layer, classify every piece of remembered
  state by (scope: user/project/task) × (lifetime: persistent/one-shot). Stello proves the
  grid is the right mental model.
- **Note, don't necessarily adopt:** the invariant that a unit's `memory` does **not** re-enter
  its own context (file 02). For Stello this prevents a session reflecting on its own
  outward-description; for Ark, a task *should* re-read its own PRD/PLAN. The invariant is
  Stello-specific. But the underlying idea — *a description written for an external reader is
  a different artifact than working context* — is worth keeping: Ark's journal entries and
  archive INDEX rows are exactly "outward-facing descriptions," distinct from working PLANs.

### 3.3 Shared memory — Ark already shipped Stello's north star; don't regress to full-body

The single most striking finding (file 03): Stello's **spec** designed an index + lazy-load
shared memory (`slug + summary` injected, `body` fetched via tool), but the **shipped code**
backed off to injecting every entry's **full body** every send. **Ark's `MEMORY.md` is
exactly Stello's *intended* model** — a one-line index per entry, bodies in separate files,
read on demand. Ark is the more disciplined implementation of Stello's own abandoned design.

- **Keep:** the index/lazy-load split. Don't "improve" Ark by inlining memory bodies into a
  single always-loaded file — that's the path Stello took *under scale pressure* and it
  trades discipline for fewer round-trips. Ark's per-topic files + `MEMORY.md` index is right.
- **Adopt the constraint "zero application-domain modeling" — partially.** Stello's
  `{slug, body}` refuses type/tags/timestamps; categorization is a slug/summary prefix.
  Ark's auto-memory already adds light structure (`metadata.type: user|feedback|project|reference`).
  That's a deliberate, small schema and it earns its keep (recall relevance). Stello's stance is
  the explicit *argument against* growing it further — hold the line at that four-type enum.

### 3.4 Reflection ownership — `ark` stays pure-data; cross-task synthesis is an agent loop

Stello's governing stance (files 03, 04): the framework exposes **only data IO with zero
implicit LLM** (`listSessionDigests` → app reflects with any LLM → `putInsight` back); it
deleted its old in-framework "Main Session" synthesis entirely. **This is already Ark's
spine** — `ark` the binary never calls an LLM; `ark context` emits structured state and the
*agent* does all judgment.

- **Adopt as a guardrail:** when tempted to put "summarize across tasks" or "detect stale
  memories" logic into the Rust binary, don't. Make it a slash command / agent loop over
  `ark context` primitives. File 03's reflection-loop sketch is the template: a read-primitive
  (`ark context --format json`) → agent reflection → a write-primitive (`ark agent …` or a
  `MEMORY.md` edit). Keep the binary on the data side of that line.
- **Adapt:** Ark's equivalent of `listSessionDigests` is `ark context`. If a cross-task
  reflection workflow ever ships, it wants a *batch digest* view — e.g. `ark context` gaining
  a mode that emits one-line summaries of recent archived tasks (their journal `### Summary`
  lines), the raw material an agent would reflect over.

### 3.5 Storage — reject pluggability; Ark's fixed layout is a feature

Stello ships *interfaces* (SessionStorage, SessionTree, SharedMemoryStore) with only
in-memory implementations, pushing persistence to the app (file 05) — because Stello is a
*library* embedded in many products (KitKit on PG, others on files). Ark is the opposite: a
*single-purpose CLI* whose entire value is an opinionated, fixed `.ark/` filesystem layout.

- **Reject:** storage abstraction. Ark should never grow a `MemoryStore` interface. The fixed
  layout is what makes `ark` files diffable, git-trackable, and human-auditable.
- **Borrow one detail:** Stello's *suggested* on-disk layout for shared memory —
  `shared-memory/INDEX.md + entries/<slug>.md` (file 05, §8.3) — is essentially Ark's
  `MEMORY.md + memory/*.md` already. Convergent design; confirms Ark's layout is sound.

---

## 4. A concrete sketch of "Ark memory, Stello-informed"

If a follow-up `/ark:design` task builds Ark's memory layer, the corpus argues for:

1. **Three named scopes, two lifetimes** (§3.2 grid), made explicit in docs even if the
   storage stays flat files:
   - *User* (cross-project, `~/.claude/.../memory/`) — already exists.
   - *Project* (`.ark/`-local, persistent) — could formalize "project memory" distinct from SPECs.
   - *Task* (`tasks/<slug>/`, persistent while active, archived after) — already exists.
   - *One-shot* (Stello's `insight`) — decide explicitly whether Ark needs it.
2. **Index + lazy-load everywhere** (§3.3) — never inline bodies into an always-loaded file.
3. **Structured lineage edges** (§3.1) — `task.toml` gains an optional `derived_from = "<slug>"`,
   so research→implementation cross-over is a queryable edge, not just prose.
4. **Reflection as a slash command, not a binary feature** (§3.4) — e.g. a future
   `/ark:reflect` that reads archived-task digests via `ark context` and proposes `MEMORY.md`
   updates, with the agent (not Rust) doing the synthesis.
5. **No storage interface, no domain schema growth** (§3.3, §3.5) — hold the four-type memory
   enum; keep the layout fixed and diffable.

---

## 5. Three sharp takeaways (the TL;DR)

1. **Ark already won the memory-layout debate Stello lost to scale.** Index + lazy-load
   file-per-topic is Stello's *intended* design; Stello shipped full-body-injection instead.
   Don't regress toward the latter.
2. **The orthogonal-axes discipline is the real lesson.** Topology, memory-scope, and
   reflection-ownership are independent. Ark conflates topology (flat dirs) and leaves
   reflection-ownership implicit. Making them deliberate — even minimally (a `derived_from`
   edge, a named "one-shot" decision) — is where the leverage is.
3. **"Binary owns data, agent owns synthesis" is already Ark's spine — defend it.** Stello
   amputated its in-framework synthesis to reach this stance; Ark started here. Every future
   "make `ark` smarter" temptation should resolve to a slash-command/agent loop over pure-data
   primitives, never LLM logic in the binary.
