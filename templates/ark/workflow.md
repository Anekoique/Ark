# Ark Workflow

How work flows from intent to archive. Read before starting any task.

---

## 1. Principles

1. **Right ceremony for the right task.** Three tiers. Pick the smallest that fits.
2. **Intent before edits.** Write down what the change is before touching code.
3. **Review is a gate, not a ritual.** Verdicts block progress; do not fabricate compliance.
4. **Archive is memory.** Every completed task leaves a traceable record.

---

## 2. Layout

```
.ark/
├── workflow.md
├── templates/             # read-only source templates
│   ├── PRD.md
│   ├── PLAN.md
│   ├── REVIEW.md
│   ├── VERIFY.md
│   └── SPEC.md
├── tasks/
│   ├── <slug>/            # active task
│   │   ├── task.toml      #   phase, tier, dates
│   │   ├── PRD.md         #   all tiers — design-phase artifact
│   │   ├── NN_PLAN.md     #   standard (NN=00) / deep (iterated)
│   │   ├── NN_REVIEW.md   #   deep only — pairs with NN_PLAN
│   │   └── VERIFY.md      #   standard + deep
│   └── archive/YYYY-MM/<slug>/
└── specs/
    ├── project/<name>/SPEC.md     # user-authored conventions
    └── features/<name>/SPEC.md    # promoted on archive (deep)
```

---

## 3. Tiers

| Tier     | Command              | Artifacts                                                               | Path through states                                  |
| -------- | -------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------- |
| Quick    | `/ark:quick`         | `PRD.md`                                                                | design → execute → archived                          |
| Standard | `/ark:design`        | `PRD.md`, `PLAN.md`, `VERIFY.md`                                        | design → plan → execute → verify → archived          |
| Deep     | `/ark:design --deep` | `PRD.md`, `NN_PLAN.md`, `NN_REVIEW.md`, `VERIFY.md`, promoted `SPEC.md` | design → plan ⇄ review → execute → verify → archived |

PRD captures *what we're building and why*. PLAN elaborates *how*. VERIFY checks the shipped code against PRD's Outcome and PLAN's Validation.

```
quick:    reversible + no new abstractions
deep:     breaking / cross-cutting / new subsystem
standard: everything else
```

Promote mid-flight with `ark agent task promote --to <tier>`; prior artifacts are preserved.

---

## 4. Lifecycle

```
       ┌────────────┐
       │  /ark:*    │  slash command starts a task
       └─────┬──────┘
             ▼
       ┌────────────┐
       │  DESIGN    │  write PRD.md — What / Why / Outcome
       └─────┬──────┘
             │  (quick skips plan/review/verify)
             ▼
       ┌────────────┐
       │    PLAN    │  write NN_PLAN.md — elaborate how
       └─────┬──────┘
             │         (deep only — plan review loop)
             │         ┌──────────────┐
             ├────────►│    REVIEW    │  NN_REVIEW.md
             │         └──────┬───────┘
             │ ◄─── rejected ─┘
             ▼
       ┌────────────┐
       │  EXECUTE   │  implement; update PLAN's Spec section if gaps emerge
       └─────┬──────┘
             ▼
       ┌────────────┐
       │   VERIFY   │  single-pass gate
       └─────┬──────┘  rejected → halt for user decision
             ▼
       ┌────────────┐
       │  ARCHIVE   │  move to tasks/archive/YYYY-MM/;
       └────────────┘  deep: extract SPEC → specs/features/<name>/
```

Each stage below names its **purpose**, the **calls** to make, and the **gate** to advance.

### DESIGN — capture what & why

- **Purpose:** write `PRD.md` (What / Why / Outcome / Related Specs). Brainstorm: quick = none, standard = ≤3 clarifying questions, deep = thorough.
- **Calls:**
  - `ark context --scope phase --for design` — orient on git, project specs, feature specs index, recent archive.
  - `ark agent task new --slug <s> --title "<t>" --tier {quick|standard|deep}` — scaffolds the task dir + PRD + `task.toml`.
- **Gate:** PRD drafted, Outcome stated. Quick → EXECUTE; standard/deep → PLAN.

### PLAN — elaborate how

- **Purpose:** fill `NN_PLAN.md` from the embedded template (Spec, Runtime, Implementation, Trade-offs, Validation). Every Goal mapped to ≥1 Validation.
- **Calls:**
  - `ark context --scope phase --for plan` — pulls current PRD + related feature specs (filtered to the PRD's `[**Related Specs**]`) + project specs.
  - `ark agent task plan` — transitions DESIGN → PLAN and seeds `00_PLAN.md`.
- **Gate:** PLAN complete; Acceptance Mapping fills every Goal. Standard → EXECUTE; deep → REVIEW.

### REVIEW — pre-execute gate (deep only, iterative)

- **Purpose:** evaluate the latest `NN_PLAN.md` against PRD and project specs; write `NN_REVIEW.md` with verdict + findings. Loop until verdict = *Approved* with zero open CRITICAL.
- **Calls:**
  - `ark context --scope phase --for review` — pulls current task, latest PLAN, related feature specs, project specs.
  - `ark agent task review` — transitions PLAN → REVIEW and seeds `NN_REVIEW.md`.
- **Iteration:** copy `NN_PLAN.md`/`NN_REVIEW.md` to the next number, bump `task.toml.iteration`, reset `phase = "plan"` (hand-edited; the state machine is small).
- **Gate:** verdict *Approved*, zero open CRITICAL. → EXECUTE.

### EXECUTE — implement

- **Purpose:** work through the latest PLAN's Implementation phases. If implementation reveals design gaps, **update the latest PLAN's `## Spec` section** to reflect reality.
- **Calls:**
  - `ark context --scope phase --for execute` — git dirty files + current task + latest PLAN + project specs.
  - `ark agent task execute` — transitions to EXECUTE.
- **Gate:** implementation complete; project's checks pass; code committed.

### VERIFY — post-execute gate (single-pass)

- **Purpose:** verify the shipped code against PRD's Outcome and PLAN's Validation. Apply the higher quality bar: plan fidelity, correctness, code quality, organization, abstraction, SPEC drift. Fill `VERIFY.md`.
- **Calls:**
  - `ark context --scope phase --for verify` — current task with PRD + latest PLAN + VERIFY.md (if exists) + git state.
  - `ark agent task verify` — transitions to VERIFY and seeds `VERIFY.md`.
- **Gate:** verdict *Approved* or *Approved with Follow-ups* → tell the user to run `/ark:archive`. *Rejected* → halt for user decision.

### ARCHIVE — preserve as memory (user-invoked)

- **Purpose:** move the task to `tasks/archive/YYYY-MM/<slug>/`. Deep tier: extract the final PLAN's `## Spec` section to `specs/features/<name>/SPEC.md` and register it in the features INDEX.
- **Calls:**
  - `ark agent task archive` — moves the dir; on deep tier, internally invokes `ark agent spec extract` and `ark agent spec register`.
- **Trigger:** `/ark:archive`. `/ark:design` and `/ark:quick` deliberately stop at VERIFY (or EXECUTE for quick); the user decides when to close out.
- **Reopen:** move the archived dir back to `.ark/tasks/<slug>/` and reset `phase = "design"` + clear `archived_at` in `task.toml`. Refuse if a same-slug active task exists.

---

## 5. Specs

Two layers: `specs/project/<name>/SPEC.md` (user-authored conventions) and `specs/features/<name>/SPEC.md` (extracted from deep-tier PLANs on archive).

**Read pattern.**
- **Project specs** — read every SPEC listed in `specs/project/INDEX.md` before any task. These are conventions that apply always.
- **Feature specs** — scan `specs/features/INDEX.md`, then read only the SPECs the task touches. Record them in PRD's `[**Related Specs**]` so VERIFY can check adherence. The DESIGN/PLAN/REVIEW context calls above expose both indices in their JSON output.

**Archive promotion (deep tier).** `ark agent task archive` extracts the final PLAN's Spec section to `specs/features/<name>/SPEC.md` and appends a row to the features INDEX. If the task modifies an existing feature SPEC, the agent appends a `[**CHANGELOG**]` entry to that SPEC.

**Divergence.** If a PLAN contradicts an existing feature SPEC, REVIEW flags it. Either the PLAN conforms or explicitly updates the SPEC.

---

## 6. Mechanics

Two CLI surfaces drive the workflow; both are referenced inline above.

- **`ark context`** — top-level, semver-stable, **read-only**. Reports git + active tasks + specs + recent archive + current task. Auto-invoked at session start via the `SessionStart` hook in `.claude/settings.json`. Use `--scope session` (default) for orientation; `--scope phase --for <phase>` for phase-targeted slices. `--format json` for machine consumers; default text for humans. `ark context --help` for the full surface.
- **`ark agent`** — hidden, **not semver-stable**, structural mutation only. Each subcommand prints a one-line summary; illegal transitions error out (e.g. `IllegalPhaseTransition`, `WrongTier`) — never bypass them with hand-edits. Every `--slug`-taking command defaults to `.ark/tasks/.current` when omitted. `ark agent --help` lists the children.

**Operations without a CLI.** Deep-tier iteration (copy `NN_PLAN.md`/`NN_REVIEW.md` to the next number, bump `iteration`, reset `phase = "plan"`) and task reopening are handled by direct file edits — the state machine is small enough that hand-edits stay manageable, and `ark agent task plan/review/...` rejects illegal transitions if the agent gets the phase wrong.
