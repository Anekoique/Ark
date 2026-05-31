# Specs

Specs are the long-lived source of truth Ark reads before any task. They live under `.ark/specs/` in two layers:

- **Project specs** (`specs/project/<name>/SPEC.md`) — user-authored conventions: language style, testing requirements, architectural decisions. Ark never edits these.
- **Feature specs** (`specs/features/<path>/SPEC.md`) — promoted from deep-tier task PLANs at commit time. Ark writes these. `<path>` is slash-separated and may nest (e.g. `core/runtime/scheduler`).

Each layer has its own `INDEX.md`. Start there.

## Project specs

Project specs encode rules that apply *always*, across every task. They're the reason an AI agent reading your repo behaves like a senior on the team rather than a contractor seeing it for the first time.

Examples of what belongs here:

- Language conventions: "We use `anyhow` for application errors and `thiserror` for library errors."
- Testing requirements: "Every public API has a doc-test. Integration tests in `tests/`, not under `src/`."
- Architecture: "All filesystem I/O routes through `io::PathExt`. No bare `std::fs::*` outside `io/`."
- Process: "Commit messages follow Conventional Commits. PR titles must reference an issue."

The agent reads every file listed in `specs/project/INDEX.md` before starting any task. Keep them tight; long, vague guidance gets ignored.

To add one:

```
.ark/specs/project/
├── INDEX.md
└── <name>/
    └── SPEC.md
```

Then add a row to `INDEX.md`'s `## Index` table:

```markdown
| Spec       | Scope                                              |
| ---------- | -------------------------------------------------- |
| `style`    | Rust style: prefer `?`, no `.unwrap()` in prod.   |
| `testing`  | Doc-tests + integration tests, 80%+ coverage goal. |
```

## Feature specs

Feature specs are the *output* of deep-tier work. When a deep-tier task **commits**, the final PLAN's `## Spec` section is extracted verbatim to `specs/features/<path>/SPEC.md` and registered in the features INDEX *inside the same commit*.

You don't write feature specs by hand. The system writes them when a deep task completes. Subsequent tasks that touch the feature read the SPEC and either conform to it or — if they need to change it — append to its `[**CHANGELOG**]` section.

This is how Ark accumulates institutional memory. The feature SPEC for `ark-context` exists because someone ran a deep-tier task to design it; the next person extending the context system reads that SPEC before drafting their PRD.

### Recursive SPEC paths

The destination is set by the PRD's `[**SPEC Path**]` block — a single `/`-separated path relative to `specs/features/`, ending in the task slug:

```
[**SPEC Path**]

core/runtime/scheduler
```

`ark agent task commit` writes the SPEC to `specs/features/core/runtime/scheduler/SPEC.md` and **upserts a row in every `INDEX.md` from the leaf up to the root** — seeding any missing intermediate INDEX from the shipped subtree template. Each `INDEX.md` rows its immediate children: leaves as `<segment>/SPEC.md`, subtrees as `<segment>/INDEX.md`. A single-segment path (e.g. `klib`) reproduces the old flat layout bit-for-bit, so most features still land at `specs/features/<slug>/SPEC.md`.

The block is **required on deep tier**; a missing or malformed block fails the commit with `FeaturePathMissing` / `InvalidFeaturePath`. Quick, standard, and research tiers ignore it.

### Brownfield extraction

For a project adopting Ark mid-life — where the feature already exists in code but has no SPEC — use `/ark:spec-extract <feature-name>` (which drives `ark agent spec import`). It produces a `specs/features/<slug>/SPEC.md` from the existing codebase without faking a deep-tier task.

To trace a feature back to the originating task: every entry in the features INDEX has a `Promoted` column with the date and source slug, and the archived task dir under `.ark/tasks/archive/YYYY-MM/<slug>/` carries the full PRD/PLAN/REVIEW history.

## Read pattern

Different phases read different slices of the spec system:

| Phase   | Reads project specs | Reads feature specs                                         |
| ------- | ------------------- | ----------------------------------------------------------- |
| design  | yes (full)          | yes, full unfiltered + recent archive                       |
| plan    | yes (full)          | filtered to PRD's `[**Related Specs**]`                     |
| review  | yes (full)          | filtered to PRD's `[**Related Specs**]`                     |
| execute | yes (full)          | none — code is the source of truth at this point            |
| verify  | yes (full)          | none — VERIFY checks the code against PRD/PLAN, not specs   |

The filtering happens automatically via `ark context --scope phase --for <phase>`. The slash commands run that command at each phase entry point and feed the structured output to the agent.

## Divergence

If a PLAN contradicts an existing feature SPEC, the REVIEW phase flags it. Either the PLAN conforms, or the PLAN explicitly updates the SPEC. There's no third option — silent drift means the next task reading the SPEC gets stale information.

When a deep-tier task modifies an existing feature SPEC (rather than promoting a new one), `ark agent task commit` appends a `[**CHANGELOG**]` entry to that SPEC and rewrites the `Promoted` column in the features INDEX with the latest touch date.

## Project SPEC ⇆ feature SPEC tradeoffs

The two layers serve different purposes:

| Question                                  | Project SPEC                                    | Feature SPEC                                     |
| ----------------------------------------- | ----------------------------------------------- | ------------------------------------------------ |
| Who writes it?                            | You.                                            | Deep-tier task commit promotes it.               |
| When is it read?                          | Every task, every phase.                        | Phase-conditional, filtered by the PRD.          |
| What's it for?                            | Norms that apply broadly.                       | Specific subsystem behavior + invariants.        |
| What's the lifetime?                      | Long-lived; you maintain it.                    | Long-lived; updated by future deep tasks.        |
| Does it appear in the slash command flow? | Read at start of every command's design phase.  | Read when the PRD references it via Related Specs. |

If you find yourself wanting to write something between these two — a project-wide convention that touches a specific feature — just put it in the feature SPEC. The project layer is for things truly orthogonal to any single feature.
