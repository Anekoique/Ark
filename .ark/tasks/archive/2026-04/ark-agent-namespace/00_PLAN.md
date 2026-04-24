# `ark-agent-namespace` PLAN `00`

> Status: Draft
> Feature: `ark-agent-namespace`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Introduce `ark agent` as a hidden top-level subcommand group that owns every mechanical mutation the Ark workflow currently asks the agent to perform by hand. The namespace packages three families — `task` (lifecycle transitions + dir management), `spec` (feature SPEC extraction + index registration), and `template` (embedded-template extraction). Every subcommand is a narrow, named operation with deterministic output, guarded state transitions, and unit-test coverage. No workspace/journal logic is introduced in this task (reserved for a follow-up).

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: Provide a hidden `ark agent` subcommand group (not listed in `ark --help`, but discoverable via `ark agent --help`) that documents itself as non-semver-stable.
- G-2: Expose explicit per-phase transition subcommands (`task plan`, `task review`, `task execute`, `task verify`, `task archive`) that enforce legal transitions per tier and reject illegal ones with a named error.
- G-3: Package task-directory management as subcommands: `task new` (scaffold), `task iterate` (deep-tier NN bump), `task promote` (tier change), `task reopen` (archive → active), `task archive` (active → archive + deep-tier SPEC side-effects).
- G-4: Package feature-SPEC operations: `spec extract` (final PLAN's `## Spec` → `specs/features/<slug>/SPEC.md`, CHANGELOG on overwrite) and `spec register` (append/update row in `specs/features/INDEX.md` managed block).
- G-5: Expose `template copy --name <t> --to <path>` to extract an embedded template to disk, so every other subcommand can share one template-extraction code path.
- G-6: `task archive` internally invokes SPEC extract + register when `task.toml.tier == "deep"`; the agent calls one command, not three.
- G-7: Every subcommand writes to disk, prints a one-line `impl Display` summary, and does not pipe structured data to siblings.
- G-8: Update `.ark/workflow.md` and `templates/ark/workflow.md` (embedded) to reference `ark agent` commands in place of raw `mkdir`/`cp`/`echo`/manual-TOML recipes.

- NG-1: No workspace/journal/identity subcommands. Reserved for a follow-up task.
- NG-2: No git or GitHub operations (no `ark agent pr`, `ark agent commit`). Agent uses `git`/`gh` directly.
- NG-3: No content-generating commands (no `ark agent verify`, no `ark agent review`). Content is the agent's judgment; `ark agent` owns only structural mutation.
- NG-4: No generic `ark agent set <k>=<v>` TOML editor. Every state mutation is a named command.
- NG-5: No consistency-check command (`ark agent doctor`, `ark agent validate`) — reviewer judgment, not mechanical check.
- NG-6: No public-API stability promise. Callers are the shipped slash commands and workflow doc, not end users.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                      — adds `Agent(AgentArgs)` hidden subcommand
└── ark-core/src/
    ├── lib.rs                                — re-exports new public API
    ├── error.rs                              — adds IllegalPhaseTransition, TaskNotFound,
    │                                           UnknownTemplate, SpecExtractionFailed variants
    ├── templates.rs                          — unchanged; `walk()` reused
    ├── layout.rs                             — adds tasks_dir(), tasks_archive_dir(),
    │                                           tasks_current(), specs_features_index(),
    │                                           task_dir(slug), specs_features_dir(slug)
    └── commands/
        └── agent/                            — new module
            ├── mod.rs                        — re-exports; shared types (Tier, Phase)
            ├── state.rs                      — TaskToml load/save + legal-transition table
            ├── task/
            │   ├── mod.rs
            │   ├── new.rs                    — scaffold task dir + PRD + task.toml + .current
            │   ├── phase.rs                  — one fn per transition, each guarded
            │   ├── iterate.rs                — copy NN_PLAN + NN_REVIEW at next N
            │   ├── promote.rs                — tier change; no artifact rewrite
            │   ├── reopen.rs                 — archive → active; slug collision guard
            │   └── archive.rs                — move dir; on deep → extract + register
            ├── spec/
            │   ├── mod.rs
            │   ├── extract.rs                — parse final PLAN's ## Spec → SPEC.md
            │   └── register.rs               — managed-block row upsert
            └── template.rs                   — resolve name → embedded bytes → disk write
```

Internal call graph for `task archive` (deep tier):

```
task::archive::archive(opts)
  ├── state::load(task.toml) → TaskToml
  ├── state::check_can_archive(toml) → ()
  ├── if toml.tier == Deep:
  │     ├── spec::extract::extract(...)    ← same function exposed via CLI
  │     └── spec::register::register(...)  ← same function exposed via CLI
  ├── state::set_phase(Archived)
  ├── filesystem move: tasks/<slug> → tasks/archive/YYYY-MM/<slug>
  └── remove .ark/tasks/.current if it points at <slug>
```

Sharing (not shelling out to) the `spec` functions keeps a single correctness path for both `task archive` and ad-hoc `ark agent spec extract` invocations.

[**Data Structure**]

```rust
// ark-core/src/commands/agent/state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Quick,
    Standard,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub status: String,            // "in_progress" | "completed" | ...
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,
}

// Legal transitions per tier (encoded as a static table).
pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool { ... }

// Error::IllegalPhaseTransition carries tier + from + to for a useful message.
```

Error enum additions (`ark-core/src/error.rs`):

```rust
Error::IllegalPhaseTransition { tier: Tier, from: Phase, to: Phase },
Error::TaskNotFound { slug: String },
Error::TaskAlreadyExists { slug: String },
Error::UnknownTemplate { name: String },
Error::SpecSectionMissing { plan_path: PathBuf },
Error::TaskTomlCorrupt { path: PathBuf, source: toml::de::Error },
```

[**API Surface**]

```rust
// crates/ark-core/src/lib.rs — additive re-exports
pub use commands::agent::{
    task::{
        new::{task_new, TaskNewOptions, TaskNewSummary},
        phase::{task_plan, task_review, task_execute, task_verify, TaskPhaseSummary},
        iterate::{task_iterate, TaskIterateOptions, TaskIterateSummary},
        promote::{task_promote, TaskPromoteOptions, TaskPromoteSummary},
        reopen::{task_reopen, TaskReopenOptions, TaskReopenSummary},
        archive::{task_archive, TaskArchiveOptions, TaskArchiveSummary},
    },
    spec::{
        extract::{spec_extract, SpecExtractOptions, SpecExtractSummary},
        register::{spec_register, SpecRegisterOptions, SpecRegisterSummary},
    },
    template::{template_copy, TemplateCopyOptions, TemplateCopySummary},
    state::{Phase, Tier, TaskToml},
};
```

CLI shape (derive-based `clap`, in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Command {
    Init(...),
    Load(...),
    Unload(...),
    Remove(...),
    #[command(hide = true, about = "Internal commands invoked by the Ark workflow and slash commands. Not covered by semver.")]
    Agent(AgentArgs),
}

#[derive(clap::Args)]
struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Subcommand)]
enum AgentCommand {
    Task(TaskArgs),
    Spec(SpecArgs),
    Template(TemplateArgs),
}

// TaskArgs nests: New, Plan, Review, Execute, Verify, Archive, Iterate, Promote, Reopen
// SpecArgs nests: Extract, Register
// TemplateArgs nests: Copy
```

Argument schema (abbreviated — full form in Implementation):

| Subcommand | Required args | Optional args |
|---|---|---|
| `agent task new` | `--slug`, `--title`, `--tier` | `-C/--dir` |
| `agent task plan` | `--slug` | `-C/--dir` |
| `agent task review` | `--slug` | `-C/--dir` |
| `agent task execute` | `--slug` | `-C/--dir` |
| `agent task verify` | `--slug` | `-C/--dir` |
| `agent task archive` | `--slug` | `-C/--dir` |
| `agent task iterate` | `--slug` | `-C/--dir` |
| `agent task promote` | `--slug`, `--to <tier>` | `-C/--dir` |
| `agent task reopen` | `--slug` | `-C/--dir` |
| `agent spec extract` | `--slug` | `-C/--dir`, `--plan <path>` (override auto-pick) |
| `agent spec register` | `--feature`, `--scope`, `--from-task <slug>` | `-C/--dir`, `--date` (defaults to today UTC) |
| `agent template copy` | `--name`, `--to` | `-C/--dir` |

Every subcommand accepts `-C/--dir` via a shared `TargetArgs` (reusing the existing one from `main.rs`). Slug resolution: all subcommands that take `--slug` default to reading `.ark/tasks/.current` if the flag is omitted.

[**Constraints**]

- C-1: `ark --help` output MUST NOT list `agent`. Verified by a CLI snapshot test.
- C-2: `ark agent --help` output MUST include a stability banner containing the string "Not covered by semver".
- C-3: Every `ark agent` subcommand's output goes through `Display`-returning summary types; no ad-hoc `println!` in subcommand bodies.
- C-4: All filesystem mutation routes through `io::PathExt`. No direct `std::fs::*` in `commands/agent/`.
- C-5: All `.ark/`-relative path composition routes through `layout::Layout` helpers (no `root.join(".ark/tasks/...")` ad-hoc).
- C-6: `task.toml` parsing/writing uses the `toml` crate (new dependency). Corrupt files produce `Error::TaskTomlCorrupt` with source error chained.
- C-7: `spec register` uses `io::fs::update_managed_block` with a new marker `ARK:FEATURES` (matches existing `templates/ark/specs/features/INDEX.md`).
- C-8: `task archive`'s dir move uses rename semantics; fails loud on cross-device moves rather than falling back to copy+delete.
- C-9: `task promote` does not rewrite existing PLAN/PRD artifacts — it only updates `task.toml.tier` and prints a reminder. Artifact reshaping is agent judgment.
- C-10: `template copy` only resolves names that exist in the embedded `ARK_TEMPLATES` or `CLAUDE_TEMPLATES` trees. Unknown names → `Error::UnknownTemplate`.
- C-11: Illegal phase transitions produce `Error::IllegalPhaseTransition` with tier + from + to. No silent success.
- C-12: `ark agent` subcommands are permitted to depend on each other via direct function calls (`task::archive` calling `spec::extract`), but MUST NOT shell out to `ark` itself.

---

## Runtime `runtime logic`

[**Main Flow — representative: `ark agent task archive --slug <s>`**]

1. Parse CLI args; resolve `-C` to absolute project root.
2. Build `Layout::new(root)`.
3. Load `task.toml` from `layout.task_dir(slug)` into `TaskToml`.
4. Check `can_transition(toml.tier, toml.phase, Phase::Archived)`; if false → `Error::IllegalPhaseTransition`.
5. If `toml.tier == Deep`:
   a. Call `spec::extract::extract(SpecExtractOptions { slug, ... })` — writes `specs/features/<slug>/SPEC.md`, appends CHANGELOG entry when it existed.
   b. Call `spec::register::register(SpecRegisterOptions { feature: slug, scope: ..., from_task: slug, date: today() })` — updates managed block.
6. Mutate `toml.phase = Archived`, `toml.archived_at = Some(now())`, `toml.updated_at = now()`.
7. Write `toml` back to `layout.task_dir(slug).join("task.toml")` (still at active path — about to be moved).
8. Compute archive target `layout.tasks_archive_dir().join(YYYY_MM).join(slug)`; `mkdir -p` parent.
9. `fs::rename(layout.task_dir(slug), target)` via `PathExt::rename_to`.
10. Read `.ark/tasks/.current`; if it equals `slug`, remove the file.
11. Return `TaskArchiveSummary { slug, tier, deep_spec_promoted: bool, archive_path }` — its `Display` prints a one-line summary.

[**Main Flow — `ark agent task new --slug <s> --title <t> --tier <T>`**]

1. Refuse if `layout.task_dir(slug)` exists → `Error::TaskAlreadyExists`.
2. `mkdir -p layout.task_dir(slug)`.
3. `template_copy(name="PRD", to=layout.task_dir(slug).join("PRD.md"))` — one of the few places where `template_copy` is called internally.
4. Build `TaskToml { id: slug, title, tier, phase: Phase::Design, iteration: 0, ... }`.
5. Write `task.toml` to the task dir.
6. Write `slug\n` to `layout.tasks_current()`.
7. Return `TaskNewSummary { slug, tier, task_dir }`.

[**Main Flow — `ark agent task plan --slug <s>`**]

1. Load `task.toml`.
2. Check `can_transition(tier, Design, Plan)`.
3. Mutate `phase = Plan`, bump `updated_at`, write back.
4. If no `00_PLAN.md` exists, call `template_copy(name="PLAN", to=task_dir.join("00_PLAN.md"))` as a convenience.
5. Return summary.

Analogous flows for `task review`, `task execute`, `task verify` — each is a thin wrapper around `state::transition(Phase)` plus a template-copy convenience when the destination phase has one (REVIEW, VERIFY).

[**Main Flow — `ark agent spec extract --slug <s>`**]

1. Load `task.toml`; require `tier == Deep` else `Error::IllegalPhaseTransition` (reuse — misnamed but fits: "you can't extract from a non-deep task"). *(Open for review: may warrant its own variant `Error::WrongTier`.)*
2. Resolve final PLAN: scan `task_dir(slug)` for `NN_PLAN.md`; pick highest NN.
3. Read that PLAN; find the `## Spec` section via a line-range parser that bounds on the next `##` or EOF.
4. If `specs/features/<slug>/SPEC.md` exists: append a CHANGELOG block with today's date; else write fresh from the `SPEC.md` template with the extracted `## Spec` body spliced in.
5. Return `SpecExtractSummary { slug, target_path, was_update: bool }`.

[**Main Flow — `ark agent spec register --feature <f> --scope <s> --from-task <t>`**]

1. Read `specs/features/INDEX.md`.
2. Compute the upsert row: `| <feature> | <scope> | <YYYY-MM-DD> from task `<t>` |`.
3. Inside the `ARK:FEATURES` managed block (existing marker), upsert by feature name — replace the row if present, append otherwise.
4. Write back via `io::fs::update_managed_block`.
5. Return `SpecRegisterSummary { feature, was_update: bool }`.

[**Failure Flow**]

1. Missing task dir → `Error::TaskNotFound { slug }` — every `--slug`-taking command errors at step 1 before mutating anything.
2. Corrupt `task.toml` → `Error::TaskTomlCorrupt { path, source }` — surfaces the `toml` parser's error chain.
3. Illegal transition → `Error::IllegalPhaseTransition { tier, from, to }` — the message names all three.
4. `task new` into existing dir → `Error::TaskAlreadyExists { slug }` — no partial write, no `--force` in this task.
5. `task archive` rename fails (cross-device, permission) → propagated as `Error::Io` with path context; no copy+delete fallback (C-8).
6. `spec extract` with no `## Spec` section in final PLAN → `Error::SpecSectionMissing { plan_path }`.
7. `template copy` with unknown name → `Error::UnknownTemplate { name }`.
8. CLI-level: `ExitCode::FAILURE` with the existing error-chain printer in `main.rs` — no new error-rendering code.

[**State Transitions**]

Legal transitions, encoded as a static match in `state::can_transition`:

```
Quick:     Design -> Execute
           Execute -> Archived

Standard:  Design -> Plan
           Plan -> Execute
           Execute -> Verify
           Verify -> Archived

Deep:      Design -> Plan
           Plan -> Review
           Review -> Plan        (iteration — agent calls `task iterate`)
           Review -> Execute
           Execute -> Verify
           Verify -> Archived
```

`task promote --to <tier>` is NOT a phase transition — it replaces `tier` in `task.toml` and prints a reminder. The current `phase` must still be legal under the new tier, otherwise promote is rejected. (Example: deep→quick while in Review is illegal because Quick has no Review phase.)

`task reopen` moves the dir back to active and sets `phase = Design`, `archived_at = None`, `updated_at = now()`. Refused if an active task with the same slug exists.

---

## Implementation `split task into phases`

[**Phase 1 — foundations (state, layout, error, CLI skeleton)**]

1. `ark-core/Cargo.toml`: add `toml = "0.8"` as a workspace dep.
2. `ark-core/src/error.rs`: add the six new `Error` variants from Data Structure. Update `Error::io` factory remains unchanged.
3. `ark-core/src/layout.rs`: add helpers — `tasks_dir()`, `tasks_archive_dir()`, `tasks_current()`, `task_dir(slug)`, `specs_features_dir(slug)`, `specs_features_index()`, `ark_templates_dir()`. No change to `owned_dirs()`.
4. `ark-core/src/commands/agent/mod.rs`: declare the submodule tree; export `Phase`, `Tier`, `TaskToml`.
5. `ark-core/src/commands/agent/state.rs`: `TaskToml` + load/save + `can_transition(tier, from, to)` static table + `check_transition(tier, from, to) -> Result<()>`.
6. `ark-cli/src/main.rs`: add hidden `Agent(AgentArgs)` subcommand; nested `AgentCommand` enum; `dispatch` match arms that currently `todo!()` each leaf — CLI compiles, tests assert help output only.

Unit tests (Phase 1):
- `state::can_transition` — exhaustive table-driven test: for every (tier, from, to), assert expected legality.
- `state::load/save` round-trip a sample `task.toml` through tempdir.
- CLI test: assert `ark --help` does not contain `agent`; `ark agent --help` contains `Not covered by semver`.

[**Phase 2 — task subcommands**]

7. `commands/agent/task/new.rs`: implement `task_new`. Deps: `layout`, `template::template_copy`, `state::TaskToml`.
8. `commands/agent/task/phase.rs`: five functions — `task_plan`, `task_review`, `task_execute`, `task_verify`, `task_archive`. `task_archive` dispatches to `spec::extract::extract` + `spec::register::register` when tier == Deep.
9. `commands/agent/task/iterate.rs`: `task_iterate` — read task dir, find highest NN of `NN_PLAN.md`, increment, copy PLAN + REVIEW templates.
10. `commands/agent/task/promote.rs`: `task_promote` — tier swap with legality check against current phase.
11. `commands/agent/task/reopen.rs`: `task_reopen` — find archived dir, check no active collision, rename back, reset phase.
12. CLI dispatch: wire each leaf to its `task_*` function; build and display summary.

Unit tests (Phase 2):
- Each `task_*` function gets a `tempfile::tempdir()`-backed test verifying both the disk state and the returned summary.
- Failure cases: `TaskNotFound`, `TaskAlreadyExists`, `IllegalPhaseTransition` — one test each.
- `task_iterate` with no prior PLANs (shouldn't happen in practice) → errors with `IllegalPhaseTransition` (can't iterate from Design).

[**Phase 3 — spec + template subcommands + workflow doc + integration test**]

13. `commands/agent/template.rs`: `template_copy` — look up name in `ARK_TEMPLATES` (with fallback to `CLAUDE_TEMPLATES`), write bytes via `io::fs::write_file` in `Force` mode.
14. `commands/agent/spec/extract.rs`: parse `NN_PLAN.md`, extract `## Spec` section (regex-free line scanner — header line "## Spec" through next "##" or EOF). Write to `specs/features/<slug>/SPEC.md` (fresh or CHANGELOG append).
15. `commands/agent/spec/register.rs`: upsert managed-block row.
16. `templates/ark/workflow.md`: rewrite §1.4–§6.3 to use `ark agent` commands. Embedded template is baked into the binary, so this affects future `ark init`s.
17. `.ark/workflow.md` in this repo: updated in lockstep (same content) — so this repo dogfoods the new workflow immediately.
18. Integration test in `commands/agent/mod.rs::tests`: `tempdir → init → agent task new --tier standard → agent task plan → agent task execute → agent task verify → agent task archive`, asserting `.ark/tasks/archive/YYYY-MM/<slug>/` contains the expected files.
19. `AGENTS.md`: add a section explaining the `ark agent` namespace and its non-stability policy.

Unit tests (Phase 3):
- `template_copy`: known name writes file with expected bytes; unknown name errors.
- `spec_extract`: a synthetic PLAN with a `## Spec` section and a synthetic PLAN without; assert success + `SpecSectionMissing` respectively.
- `spec_register`: empty INDEX block → appended; existing row → replaced; scoped by feature name.
- Round-trip integration test as above, plus deep-tier variant asserting SPEC promoted + INDEX updated.

---

## Trade-offs `ask reviewer for advice`

- T-1: **Explicit per-transition subcommands vs. single `task phase --to <p>`.** Chosen: explicit (`task plan`, `task review`, etc.), per user decision. Adv: typo-resistant, each command's `--help` documents its transition, the CLI self-documents the state machine. Disadv: five commands instead of one, slight code duplication in phase.rs (mitigated by delegating to one shared `state::transition` helper).

- T-2: **`task archive` deep-tier side effects — one command vs. three.** Chosen: one command (per user decision). Adv: single contract for the agent — "archive closes out the task, period." Disadv: CLI-visible behavior differs by tier, which could surprise someone reading the command list. Mitigation: `--help` output names the deep-tier extras explicitly.

- T-3: **`toml` crate vs. hand-rolled parser.** Chosen: `toml` (well-maintained, ~100 LOC dep). Alternative: regex-based line scanner. Rejected because a hand-rolled parser can silently accept malformed input, which violates the "named errors or fail loud" principle (C-6/C-11).

- T-4: **`## Spec` extraction — string scan vs. Markdown parser.** Chosen: string scan (match `## Spec` header, read to next `## ` prefix or EOF). Alternative: `pulldown-cmark`. Rejected because it adds a significant dep for one use case; the scan is ~20 LOC and covers the PLAN.md format we control.

- T-5: **Hidden subcommand (`hide = true`) vs. separate binary (`ark-agent`).** Chosen: hidden subcommand. Adv: one binary to ship, one release artifact, no cargo-dist reconfig. Disadv: surface remains in the `ark` binary (discoverable via `ark agent --help`). Mitigation: banner makes non-stability explicit; follows precedent from `cargo` (`cargo run` hidden flags etc.).

- T-6: **`spec extract` tier check using `IllegalPhaseTransition` vs. a dedicated `WrongTier` variant.** Open. `IllegalPhaseTransition` reuses existing infrastructure but its name is misleading here. A fresh `Error::WrongTier { expected, actual }` is clearer but adds a variant for one call site. Reviewer to decide.

- T-7: **`.current` file vs. deriving from `task.toml` scan.** Chosen: keep `.current` (current workflow already uses it; agent needs a cheap lookup). Alternative: scan every `task.toml.status` for "in_progress". Rejected — O(tasks) per lookup, and multiple in-progress tasks are legal.

---

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `state::can_transition` — exhaustive (3 tiers × 6 phases × 6 phases = 108 cases; table-driven; positive and negative each asserted).
- V-UT-2: `state::load` / `state::save` round-trip — identical bytes after write→read.
- V-UT-3: `state::load` on corrupt TOML → `Error::TaskTomlCorrupt` with source preserved.
- V-UT-4: `task_new` writes dir + PRD + task.toml + `.current`; second call errors `TaskAlreadyExists`.
- V-UT-5: Each of `task_plan`/`_review`/`_execute`/`_verify` — legal transition succeeds, illegal transition errors, `updated_at` bumps.
- V-UT-6: `task_archive` standard tier — moves dir, updates `.current`, no SPEC side effects.
- V-UT-7: `task_archive` deep tier — moves dir, calls extract + register, writes expected files.
- V-UT-8: `task_iterate` — finds highest NN, writes NN+1 PLAN + REVIEW copies.
- V-UT-9: `task_promote` — legal tier swap, illegal when current phase doesn't exist under target tier.
- V-UT-10: `task_reopen` — archived → active, error on slug collision.
- V-UT-11: `spec_extract` — PLAN with `## Spec` → writes SPEC.md; PLAN without → `SpecSectionMissing`; existing SPEC → CHANGELOG appended.
- V-UT-12: `spec_register` — empty INDEX managed block → row appended; existing row for same feature → replaced; different feature → appended.
- V-UT-13: `template_copy` — known name writes exact bytes; unknown name errors.

[**Integration Tests**]

- V-IT-1: Standard-tier round-trip in a tempdir: `init` → `agent task new --tier standard` → `agent task plan` → `agent task execute` → `agent task verify` → `agent task archive`. Asserts the final filesystem state (archive path, `.current` absent).
- V-IT-2: Deep-tier round-trip: `init` → `agent task new --tier deep` → `agent task plan` → `agent task review` → `agent task iterate` (loop once) → `agent task execute` → `agent task verify` → `agent task archive`. Asserts `specs/features/<slug>/SPEC.md` exists and `specs/features/INDEX.md` has the row.
- V-IT-3: CLI help snapshot: `ark --help` contains `init/load/unload/remove`, does NOT contain `agent`. `ark agent --help` contains the stability banner.

[**Failure / Robustness Validation**]

- V-F-1: Partial write on `task_new` (simulated by pre-creating a conflicting file) → `TaskAlreadyExists`, no partial dir left.
- V-F-2: `task_archive` with corrupt `task.toml` → `TaskTomlCorrupt`, dir not moved.
- V-F-3: `spec_extract` with malformed `NN_PLAN.md` (no `## Spec`) → `SpecSectionMissing`, nothing written to `specs/features/`.
- V-F-4: Illegal transition attempt leaves `task.toml` unchanged (verified by mtime or byte-compare).

[**Edge Case Validation**]

- V-E-1: `--slug` omitted → reads `.ark/tasks/.current`; missing `.current` → `Error::TaskNotFound { slug: "<.current missing>" }` (or similar dedicated variant — open for review).
- V-E-2: `task_archive` when `.current` points at a different slug → moves the task anyway; does not clear `.current`.
- V-E-3: `task_iterate` in a fresh task with only `00_PLAN.md` → creates `01_PLAN.md` + `01_REVIEW.md`.
- V-E-4: `task_archive` twice (already archived) → `IllegalPhaseTransition { from: Archived, to: Archived }`.
- V-E-5: `spec_register` called with `--feature` containing special characters (spaces, pipes) → sanitize or error? (Open for review — probably error.)
- V-E-6: Concurrent `ark agent` invocations on the same task dir — no file locking in v1; documented as "agent should not parallelize against the same slug."

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-3 |
| G-2 | V-UT-1, V-UT-5 |
| G-3 | V-UT-4, V-UT-6, V-UT-7, V-UT-8, V-UT-9, V-UT-10 |
| G-4 | V-UT-11, V-UT-12 |
| G-5 | V-UT-13 |
| G-6 | V-UT-7, V-IT-2 |
| G-7 | V-IT-1, V-IT-2 (both assert one-line summary output) |
| G-8 | Manual inspection during VERIFY; V-IT-1 & V-IT-2 exercise the new `workflow.md` commands end-to-end |
| C-1 | V-IT-3 |
| C-2 | V-IT-3 |
| C-3 | Code review during VERIFY (`grep 'println!' crates/ark-core/src/commands/agent/` must be empty) |
| C-4 | Code review during VERIFY (`grep 'std::fs::' crates/ark-core/src/commands/agent/` must be empty) |
| C-5 | Code review during VERIFY (`grep '\.join(\"\\.ark' crates/ark-core/src/commands/agent/` must be empty) |
| C-6 | V-UT-2, V-UT-3 |
| C-7 | V-UT-12 |
| C-8 | V-F-2 implicitly; additional explicit test only if CI hits cross-device scenario |
| C-9 | V-UT-9 (promote doesn't mutate PRD/PLAN artifacts — assert file contents unchanged) |
| C-10 | V-UT-13 |
| C-11 | V-UT-1, V-UT-5, V-UT-9, V-E-4 |
| C-12 | Code review during VERIFY — no `Command::new("ark")` in agent module |
