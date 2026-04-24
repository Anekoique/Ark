# `ark-agent-namespace` PLAN `01`

> Status: Revised
> Feature: `ark-agent-namespace`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: none

---

## Summary

Iteration 01 addresses all 11 findings and all 7 trade-off verdicts from `00_REVIEW.md`. Load-bearing corrections: adds `PathExt::rename_to` as a Phase 1 deliverable (R-001); formalizes the `## Spec` section-scan predicate (R-002); acknowledges `toml` as a net-new workspace-pinned dep (R-003); extends the workflow-doc rewrite to the shipped slash-command templates (R-004); adds a dedicated `Error::WrongTier` variant (TR-6) plus `Error::InvalidSpecField` and `Error::NoCurrentTask`; drops the free-form `status: String` and derives it from `phase` (R-006); and specifies the deep-tier integration test sequence explicitly (R-011). The overall design is unchanged; the spec and validation sections are tightened.

## Log

[**Added**]
- New `PathExt::rename_to` helper as a Phase 1 prerequisite for `task archive`'s directory move (R-001).
- Two error variants: `Error::WrongTier { expected, actual }` (TR-6) and `Error::InvalidSpecField { field, reason }` (R-005) and `Error::NoCurrentTask { path }` (R-007).
- Explicit `## Spec` section-scan predicate in Constraints (C-13) with accompanying V-UT-14 and V-UT-15 for the two edge cases the reviewer named.
- Scope extension in G-8 covering `templates/claude/commands/ark/{quick,design}.md` (R-004), plus a new validation V-IT-4 asserting those slash commands no longer contain raw `mkdir`/`cp`/`echo` recipes for ark-managed paths.
- New failure tests: V-F-5 (`spec_register` on malformed managed block) and a tightened V-UT-9 (assert PRD/PLAN bytes unchanged after `task_promote`).
- One-line clarifications in Architecture / API Surface / State Transitions for R-008, R-009, R-010.
- Explicit V-IT-2 sequence per R-011.

[**Changed**]
- `TaskToml::status` field **dropped**; status is derived from `phase` via `TaskToml::status()` method that returns `InProgress` (any phase ≠ `Archived`) or `Completed` (`phase == Archived`). Removes a stringly-typed footgun (R-006).
- `spec extract` tier check now returns `Error::WrongTier`, not `Error::IllegalPhaseTransition` (TR-6). T-6 closed.
- `toml` crate added as `[workspace.dependencies]` entry pinned at `0.8`, consumed by `ark-core` via `toml.workspace = true`. T-3 note extended to acknowledge net-new dep (R-003).
- V-E-5 (`spec register` with `|` or newline in `feature` / `scope`) resolved: error with `Error::InvalidSpecField`. No sanitization.

[**Removed**]
- `TaskToml::status` field (see Changed above).
- T-6 from Trade-offs — now a closed decision, moved to Log.

[**Unresolved**]
- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Phase 1 item added: extend `io::PathExt` with `rename_to`; unit test in `io/path_ext.rs`. C-8 + Runtime step 9 cite the new method. |
| Review | R-002 | Accepted | Added C-13 specifying the exact predicate. V-UT-14 covers inline-code suffix form; V-UT-15 covers `### subheading` inside `## Spec` section. Runtime and Implementation updated. |
| Review | R-003 | Accepted | T-3 note extended; `toml = "0.8"` pinned at `[workspace.dependencies]`; ark-core uses `toml.workspace = true`. |
| Review | R-004 | Accepted | G-8 path list extended to `templates/claude/commands/ark/{quick,design}.md`. Implementation Phase 3 step 16 expanded. New V-IT-4 asserts absence of raw recipes in shipped slash commands. |
| Review | R-005 | Accepted | V-F-5 added (malformed managed block in `specs/features/INDEX.md`). V-UT-9 tightened (PRD/PLAN byte-compare after promote). V-E-5 resolved → `Error::InvalidSpecField`. |
| Review | R-006 | Accepted — drop option | `status` field removed from `TaskToml`. Derived via `TaskToml::status()` → `Status::{InProgress, Completed}` enum. |
| Review | R-007 | Accepted | New `Error::NoCurrentTask { path }`. Cited in Runtime's slug-resolution step and in V-E-1. |
| Review | R-008 | Accepted | API Surface gains a one-liner: `hide = true` on `Agent(AgentArgs)` hides `agent` from `ark --help`; `ark agent --help` still renders children. V-IT-3 asserts both. |
| Review | R-009 | Accepted | Architecture gains a one-liner on the coupling pattern: `commands/agent/mod.rs` keeps `state` as a private module re-exported publicly; `task::archive` imports `super::spec::{extract, register}` explicitly; no `pub use` of peer modules. |
| Review | R-010 | Accepted | State Transitions clarified: `iterate` is the `Review → Plan` transition AND bumps NN; illegal from any other phase. `task iterate` rejects from non-Review phases with `IllegalPhaseTransition`. |
| Review | R-011 | Accepted | V-IT-2 sequence specified: `new(deep) → plan → review → iterate → review → execute → verify → archive`. Asserts `00_*` and `01_*` PLAN/REVIEW artifacts present, SPEC promoted, INDEX row present. |
| Review | TR-1 | Applied | Keep explicit per-phase subcommands (as planned). |
| Review | TR-2 | Applied | Keep `task archive` as single command dispatching deep-tier side effects. `--help` text for `task archive` will name deep-tier extras. |
| Review | TR-3 | Applied | Keep `toml` crate. R-003 acknowledgment added. |
| Review | TR-4 | Applied | Keep string scan with tightened predicate per R-002. No Markdown parser dep. |
| Review | TR-5 | Applied | Keep hidden subcommand. R-008 addressed. |
| Review | TR-6 | Applied | Added `Error::WrongTier`. Updated C-11 to reference both `IllegalPhaseTransition` and `WrongTier`. |
| Review | TR-7 | Applied | Keep `.current`. R-007 addressed. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here. ✓ (R-001 CRITICAL, R-002 HIGH)
> - Every Master directive must appear here. ✓ (none)
> - Rejections must include explicit reasoning. ✓ (no rejections)

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
- G-8: Update the following to reference `ark agent` commands in place of raw `mkdir`/`cp`/`echo`/manual-TOML recipes (all in lockstep):
  - `.ark/workflow.md` (this repo, live)
  - `templates/ark/workflow.md` (embedded; future `ark init`s)
  - `templates/claude/commands/ark/quick.md` (embedded slash command)
  - `templates/claude/commands/ark/design.md` (embedded slash command)
  - `.claude/commands/ark/quick.md` + `design.md` (this repo's copies, regenerated from templates)

- NG-1: No workspace/journal/identity subcommands. Reserved for a follow-up task.
- NG-2: No git or GitHub operations. Agent uses `git`/`gh` directly.
- NG-3: No content-generating commands. Content is the agent's judgment; `ark agent` owns only structural mutation.
- NG-4: No generic `ark agent set <k>=<v>` TOML editor.
- NG-5: No consistency-check command.
- NG-6: No public-API stability promise. Callers are the shipped slash commands and workflow doc.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                      — adds `Agent(AgentArgs)` hidden subcommand
└── ark-core/src/
    ├── lib.rs                                — re-exports new public API
    ├── error.rs                              — adds IllegalPhaseTransition, WrongTier,
    │                                           TaskNotFound, TaskAlreadyExists,
    │                                           UnknownTemplate, SpecSectionMissing,
    │                                           TaskTomlCorrupt, InvalidSpecField,
    │                                           NoCurrentTask variants
    ├── io/
    │   └── path_ext.rs                       — adds fn rename_to(&self, dest) -> Result<()>
    ├── templates.rs                          — unchanged
    ├── layout.rs                             — adds tasks_dir(), tasks_archive_dir(),
    │                                           tasks_current(), specs_features_index(),
    │                                           task_dir(slug), specs_features_dir(slug)
    └── commands/
        └── agent/                            — new module
            ├── mod.rs                        — pub mod task; pub mod spec; pub mod template;
            │                                   mod state; pub use state::{Phase, Tier,
            │                                   Status, TaskToml};
            ├── state.rs                      — (private module) TaskToml load/save +
            │                                   legal-transition table
            ├── task/
            │   ├── mod.rs
            │   ├── new.rs                    — scaffold task dir + PRD + task.toml + .current
            │   ├── phase.rs                  — one fn per transition, each guarded
            │   ├── iterate.rs                — Review → Plan + NN bump
            │   ├── promote.rs                — tier change; no artifact rewrite
            │   ├── reopen.rs                 — archive → active; slug collision guard
            │   └── archive.rs                — move dir; on deep → super::spec::{extract,
            │                                   register} explicitly (no pub use peer shortcuts)
            ├── spec/
            │   ├── mod.rs
            │   ├── extract.rs                — parse final PLAN's ## Spec → SPEC.md
            │   └── register.rs               — managed-block row upsert
            └── template.rs                   — resolve name → embedded bytes → disk write
```

**Module coupling note (R-009).** `task::archive` imports `super::spec::{extract, register}` explicitly and calls their public functions. `commands/agent/mod.rs` does NOT re-export peer modules with `pub use` — only `state` is re-exported (for `Phase`, `Tier`, `Status`, `TaskToml`). This keeps the dependency direction one-way (task → spec → state; template is a leaf) and avoids surprising visibility graphs.

Internal call graph for `task archive` (deep tier) — unchanged from 00_PLAN:

```
task::archive::archive(opts)
  ├── state::load(task.toml) → TaskToml
  ├── state::check_can_archive(toml) → ()
  ├── if toml.tier == Deep:
  │     ├── spec::extract::extract(...)    ← same function exposed via CLI
  │     └── spec::register::register(...)  ← same function exposed via CLI
  ├── state::set_phase(Archived)
  ├── rename tasks/<slug> → tasks/archive/YYYY-MM/<slug> via PathExt::rename_to
  └── remove .ark/tasks/.current if it points at <slug>
```

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

/// Derived from `Phase`. Not persisted; computed on demand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<DateTime<Utc>>,
}

impl TaskToml {
    /// Derived status; not persisted.
    pub fn status(&self) -> Status {
        if self.phase == Phase::Archived {
            Status::Completed
        } else {
            Status::InProgress
        }
    }
}

/// Legal transitions per tier (encoded as a static table).
pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool { /* see State Transitions */ }

pub fn check_transition(tier: Tier, from: Phase, to: Phase) -> Result<()> {
    if can_transition(tier, from, to) {
        Ok(())
    } else {
        Err(Error::IllegalPhaseTransition { tier, from, to })
    }
}
```

Error enum additions (`ark-core/src/error.rs`):

```rust
Error::IllegalPhaseTransition { tier: Tier, from: Phase, to: Phase },
Error::WrongTier { expected: Tier, actual: Tier },
Error::TaskNotFound { slug: String },
Error::TaskAlreadyExists { slug: String },
Error::NoCurrentTask { path: PathBuf },
Error::UnknownTemplate { name: String },
Error::SpecSectionMissing { plan_path: PathBuf },
Error::TaskTomlCorrupt { path: PathBuf, source: toml::de::Error },
Error::InvalidSpecField { field: String, reason: &'static str },
```

New `PathExt` method:

```rust
// ark-core/src/io/path_ext.rs — addition

fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>;
// Wraps std::fs::rename, mapping errors via Error::io(src_path, source).
// Fails loud on cross-device moves (consistent with C-8: no copy+delete fallback).
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
    state::{Phase, Tier, Status, TaskToml},
};
```

CLI shape (unchanged shape, annotation semantics clarified per R-008):

```rust
#[derive(Subcommand)]
enum Command {
    Init(...),
    Load(...),
    Unload(...),
    Remove(...),
    /// `hide = true` hides `agent` from `ark --help`;
    /// `ark agent --help` still renders its own children and about-text.
    /// C-1 / C-2 verified by V-IT-3.
    #[command(hide = true, about = "Internal commands invoked by the Ark workflow and slash commands. Not covered by semver.")]
    Agent(AgentArgs),
}
```

Argument schema (unchanged from 00_PLAN). Slug defaulting: every `--slug`-taking subcommand reads `.ark/tasks/.current` when the flag is omitted. Missing `.current` → `Error::NoCurrentTask { path }`.

[**Constraints**]

- C-1: `ark --help` output MUST NOT list `agent`. Verified by CLI snapshot test.
- C-2: `ark agent --help` output MUST include a stability banner containing the string "Not covered by semver".
- C-3: Every `ark agent` subcommand's output goes through `Display`-returning summary types; no ad-hoc `println!` in subcommand bodies.
- C-4: All filesystem mutation routes through `io::PathExt`. No direct `std::fs::*` in `commands/agent/`.
- C-5: All `.ark/`-relative path composition routes through `layout::Layout` helpers.
- C-6: `task.toml` parsing/writing uses the `toml` crate. Corrupt files produce `Error::TaskTomlCorrupt` with source error chained.
- C-7: `spec register` uses `io::fs::update_managed_block` with marker `ARK:FEATURES`.
- C-8: `task archive`'s dir move uses `PathExt::rename_to` (rename semantics); fails loud on cross-device moves rather than falling back to copy+delete.
- C-9: `task promote` does not rewrite existing PLAN/PRD artifacts — it only updates `task.toml.tier` and prints a reminder.
- C-10: `template copy` only resolves names that exist in the embedded `ARK_TEMPLATES` or `CLAUDE_TEMPLATES` trees. Unknown names → `Error::UnknownTemplate`.
- C-11: Illegal phase transitions produce `Error::IllegalPhaseTransition`. Deep-tier-only operations invoked with wrong tier produce `Error::WrongTier`. No silent success.
- C-12: `ark agent` subcommands depend on each other via direct function calls only. MUST NOT shell out to `ark` itself.
- C-13: **`## Spec` section-scan predicate.** Start line matches exactly when `line.starts_with("## Spec")` AND (`line.len() == 7` OR `line.as_bytes()[7] == b' '`) — this accepts both `"## Spec"` and `"## Spec \`...\`"` but rejects `"## Speculation"`. End boundary is the first subsequent line where `line.starts_with("## ")` OR `line == "##"` — this matches any H2 but NOT `### subheadings`. EOF also terminates.
- C-14: `spec register`'s `--feature` and `--scope` args MUST NOT contain `|` (pipe, conflicts with markdown table cells) or newline characters. Violation → `Error::InvalidSpecField`. No sanitization.

---

## Runtime `runtime logic`

[**Main Flow — `ark agent task archive --slug <s>`** (updated per R-001)]

1. Parse CLI args; resolve `-C` to absolute project root.
2. Build `Layout::new(root)`.
3. Resolve slug: if `--slug` absent, read `.ark/tasks/.current`; if missing → `Error::NoCurrentTask { path }`.
4. Load `task.toml` from `layout.task_dir(slug)` into `TaskToml`.
5. Check `can_transition(toml.tier, toml.phase, Phase::Archived)`; if false → `Error::IllegalPhaseTransition`.
6. If `toml.tier == Tier::Deep`:
   a. `spec::extract::extract(SpecExtractOptions { slug, ... })` — writes `specs/features/<slug>/SPEC.md`, CHANGELOG appended when it existed.
   b. `spec::register::register(SpecRegisterOptions { feature: slug, scope: ..., from_task: slug, date: today_utc() })`.
7. Mutate `toml.phase = Archived`, `toml.archived_at = Some(now())`, `toml.updated_at = now()`.
8. Write `toml` back to `layout.task_dir(slug).join("task.toml")` (still at active path).
9. `mkdir -p` the archive parent `layout.tasks_archive_dir().join(YYYY_MM)`.
10. `layout.task_dir(slug).rename_to(target)` via `PathExt::rename_to`.
11. Read `.ark/tasks/.current`; if it equals `slug`, remove the file.
12. Return `TaskArchiveSummary { slug, tier, deep_spec_promoted: bool, archive_path }`.

[**Main Flow — `ark agent spec extract --slug <s>`** (updated per R-002, TR-6)]

1. Load `task.toml`; if `tier != Deep` → `Error::WrongTier { expected: Deep, actual }`.
2. Resolve final PLAN: scan `task_dir(slug)` for files matching `^[0-9]{2}_PLAN\.md$`; pick highest NN.
3. Read that PLAN; locate the `## Spec` section using C-13's predicate:
   - Start = first line where `line.starts_with("## Spec") && (line.len() == 7 || line.as_bytes()[7] == b' ')`.
   - End = first subsequent line where `line.starts_with("## ") || line == "##"`, or EOF.
4. If no start line found → `Error::SpecSectionMissing { plan_path }`.
5. If `specs/features/<slug>/SPEC.md` exists: append a `[**CHANGELOG**]` block with today's UTC date + extracted body. Else write fresh from the `SPEC.md` template with the body spliced in.
6. Return `SpecExtractSummary { slug, target_path, was_update: bool }`.

[**Main Flow — `ark agent spec register`** (updated per R-005)]

1. Validate `feature` and `scope`: reject if either contains `|` or newline → `Error::InvalidSpecField`.
2. Read `specs/features/INDEX.md` via `io::fs::read_managed_block` (marker `ARK:FEATURES`).
3. If file or block is missing, `update_managed_block` will create it (existing primitive behavior). If the file exists with the `ARK:FEATURES:START` marker but no matching `ARK:FEATURES:END` → the existing `update_managed_block` behavior applies (see V-F-5 for defined semantics).
4. Compute upsert row: `| <feature> | <scope> | <YYYY-MM-DD> from task \`<from-task>\` |`.
5. Inside the managed block body, upsert by feature name: regex-free line scan; replace if present, append otherwise.
6. Write back via `io::fs::update_managed_block`.
7. Return `SpecRegisterSummary { feature, was_update: bool }`.

Other command flows (`task new`, `task plan`/`review`/`execute`/`verify`, `task iterate`, `task promote`, `task reopen`) are unchanged from 00_PLAN, except:
- All slug-resolution steps now error with `Error::NoCurrentTask` on missing `.current`.
- `task iterate` is explicitly restricted to `phase == Review` (see State Transitions).

[**Failure Flow**]

Unchanged from 00_PLAN, plus:
7. `spec extract` on a non-deep task → `Error::WrongTier { expected: Deep, actual }`.
8. `spec register` with `|` or newline in `--feature` / `--scope` → `Error::InvalidSpecField`.
9. Any `--slug`-taking command with neither `--slug` nor `.current` → `Error::NoCurrentTask { path }`.

[**State Transitions**]

```
Quick:     Design -> Execute
           Execute -> Archived

Standard:  Design -> Plan
           Plan -> Execute
           Execute -> Verify
           Verify -> Archived

Deep:      Design -> Plan
           Plan -> Review
           Review -> Plan        (via `task iterate`: bumps NN, only legal from Review)
           Review -> Execute
           Execute -> Verify
           Verify -> Archived
```

`task iterate` is specifically the `Review → Plan` transition AND bumps the iteration counter (and creates `NN+1_PLAN.md` + `NN+1_REVIEW.md` via template copies). Illegal from any other phase → `Error::IllegalPhaseTransition { tier: Deep, from: <actual>, to: Plan }`.

`task promote` is NOT a phase transition — it swaps `tier` in `task.toml`. The current `phase` must still be legal under the new tier, else rejected with `Error::IllegalPhaseTransition`.

`task reopen` moves dir archive → active and sets `phase = Design`, `archived_at = None`, `updated_at = now()`. Refused on slug collision.

---

## Implementation `split task into phases`

[**Phase 1 — foundations (state, layout, error, PathExt, CLI skeleton)**]

1. Root `Cargo.toml`: add `toml = "0.8"` to `[workspace.dependencies]`.
2. `ark-core/Cargo.toml`: add `toml.workspace = true` (per R-003, pinned at workspace).
3. `ark-core/src/error.rs`: add the 9 new `Error` variants from Data Structure.
4. `ark-core/src/io/path_ext.rs`: add `fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>` wrapping `std::fs::rename`, mapping errors via `Error::io(self.as_ref(), source)`. Add a unit test: rename a tempdir-scoped file, assert the source is gone and the destination exists with the same bytes.
5. `ark-core/src/layout.rs`: add helpers — `tasks_dir()`, `tasks_archive_dir()`, `tasks_current()`, `task_dir(slug)`, `specs_features_dir(slug)`, `specs_features_index()`, `ark_templates_dir()`.
6. `ark-core/src/commands/agent/mod.rs`: declare `pub mod task; pub mod spec; pub mod template; mod state; pub use state::{Phase, Tier, Status, TaskToml};`.
7. `ark-core/src/commands/agent/state.rs`: `Phase`, `Tier`, `Status` enums; `TaskToml` + `status()` method + load/save + `can_transition(tier, from, to)` static table + `check_transition(tier, from, to) -> Result<()>`.
8. `ark-cli/src/main.rs`: add hidden `Agent(AgentArgs)` variant; nested `AgentCommand` enum; `dispatch` arms that currently `todo!()` each leaf so the CLI compiles and help-snapshot tests pass.

Unit tests (Phase 1):
- `state::can_transition` — exhaustive (3 tiers × 6 phases × 6 phases = 108 cases, positive and negative).
- `state::load/save` round-trip; corrupt TOML → `TaskTomlCorrupt`.
- `TaskToml::status()` — returns `Completed` iff `phase == Archived`.
- `PathExt::rename_to` — success + nonexistent-source failure.
- CLI help snapshot: `ark --help` does not contain `agent`; `ark agent --help` contains `Not covered by semver`.

[**Phase 2 — task subcommands**]

9. `commands/agent/task/new.rs`: `task_new`. Deps: `layout`, `template::template_copy`, `state::TaskToml`.
10. `commands/agent/task/phase.rs`: five functions — `task_plan`, `task_review`, `task_execute`, `task_verify`, `task_archive`. `task_archive` dispatches to `spec::extract::extract` + `spec::register::register` when tier == Deep.
11. `commands/agent/task/iterate.rs`: `task_iterate` — guarded to `phase == Review`; finds highest NN of `NN_PLAN.md`, increments, copies PLAN + REVIEW templates to `NN+1_PLAN.md` + `NN+1_REVIEW.md`, sets `phase = Plan`, bumps `iteration`.
12. `commands/agent/task/promote.rs`: `task_promote` — tier swap with legality check against current phase. No artifact rewrite.
13. `commands/agent/task/reopen.rs`: `task_reopen` — find archived dir, check no active collision, `rename_to` back, reset phase.
14. CLI dispatch: wire each leaf to its `task_*` function.

Unit tests (Phase 2):
- Each `task_*` function gets a `tempfile::tempdir()`-backed test verifying disk state + returned summary.
- Failure cases: `TaskNotFound`, `TaskAlreadyExists`, `IllegalPhaseTransition` — one test each.
- `task_iterate` from `phase != Review` → `IllegalPhaseTransition`.
- `task_promote` asserts PRD/PLAN bytes unchanged (R-005 tightened V-UT-9).

[**Phase 3 — spec + template subcommands + workflow doc + integration tests**]

15. `commands/agent/template.rs`: `template_copy` — look up name in `ARK_TEMPLATES`, fallback to `CLAUDE_TEMPLATES`; write via `io::fs::write_file` in `Force` mode.
16. `commands/agent/spec/extract.rs`: parse `NN_PLAN.md` via C-13's predicate.
17. `commands/agent/spec/register.rs`: `--feature`/`--scope` validation (C-14) + managed-block row upsert.
18. **Workflow doc rewrite (R-004 expanded):**
    - `templates/ark/workflow.md` (embedded)
    - `.ark/workflow.md` (this repo, live — same content)
    - `templates/claude/commands/ark/quick.md` (embedded slash command body)
    - `templates/claude/commands/ark/design.md` (embedded slash command body)
    - `.claude/commands/ark/quick.md` + `design.md` (this repo's copies)

    Each replaces raw `mkdir .ark/tasks/$SLUG`, `cp .ark/templates/PRD.md ...`, `echo` and hand-written TOML edits with the corresponding `ark agent task new/plan/review/...` invocations.
19. Integration test in `commands/agent/mod.rs::tests`:
    - **V-IT-1 (standard):** `init → agent task new --tier standard → agent task plan → agent task execute → agent task verify → agent task archive`. Asserts archive path exists, `.current` absent, no SPEC side effects.
    - **V-IT-2 (deep, R-011 sequence):** `init → agent task new --tier deep → agent task plan → agent task review → agent task iterate → agent task review → agent task execute → agent task verify → agent task archive`. Asserts `00_PLAN.md`, `00_REVIEW.md`, `01_PLAN.md`, `01_REVIEW.md` all present in the archive dir; `phase = archived`; `specs/features/<slug>/SPEC.md` exists; `specs/features/INDEX.md` has the row.
    - **V-IT-3 (CLI help, C-1/C-2):** `ark --help` does not contain `agent`; `ark agent --help` contains `Not covered by semver`.
    - **V-IT-4 (slash-command recipe absence, R-004):** Load embedded `templates/claude/commands/ark/{quick,design}.md`, grep for `mkdir -p ".ark/tasks` / `cp .ark/templates/` / `echo ` + `> .ark/tasks`; asserts none present.
20. `AGENTS.md`: add a section explaining the `ark agent` namespace and its non-stability policy.

Unit tests (Phase 3):
- `template_copy`: known name writes exact bytes; unknown name errors.
- `spec_extract`: (a) PLAN with `## Spec` section — extracts; (b) PLAN without — `SpecSectionMissing`; (c) existing SPEC — CHANGELOG appended; (d) **V-UT-14** PLAN with `## Spec \`{Core specification}\`` header (inline-code suffix) — extracts correctly; (e) **V-UT-15** PLAN with `### Subheading` inside `## Spec` section — does NOT terminate early.
- `spec_register`: empty INDEX block → appended; existing row → replaced; different feature → appended.
- `spec_register` with `|` or `\n` in `--feature` or `--scope` → `InvalidSpecField`.
- **V-F-5:** `spec_register` against a file with `ARK:FEATURES:START` but no matching `:END` marker — assert defined behavior (follows `update_managed_block`'s existing semantics; test documents what that is).

---

## Trade-offs `ask reviewer for advice`

- T-1: **Explicit per-transition subcommands vs. single `task phase --to <p>`.** Kept explicit per TR-1.
- T-2: **`task archive` deep-tier side effects — one command vs. three.** Kept single command per TR-2.
- T-3: **`toml` crate vs. hand-rolled parser.** Kept `toml` per TR-3. **Net-new dep acknowledged** (R-003); pinned at workspace level to enable future sharing with `ark-cli`.
- T-4: **`## Spec` extraction — string scan vs. Markdown parser.** Kept string scan per TR-4 with tightened predicate (C-13).
- T-5: **Hidden subcommand (`hide = true`) vs. separate binary (`ark-agent`).** Kept hidden subcommand per TR-5.
- ~~T-6~~: **`spec extract` tier check error variant — IllegalPhaseTransition vs. dedicated WrongTier.** **CLOSED per TR-6**: added dedicated `Error::WrongTier { expected, actual }`. See Log.
- T-7: **`.current` file vs. deriving from `task.toml` scan.** Kept `.current` per TR-7.
- T-8: **`TaskToml::status` field — typed enum, dropped, or kept as String.** Per R-006, **dropped the persisted field**; status is derived via `TaskToml::status() -> Status`. Alternatives (persisted enum) rejected because Phase already carries the information and a second field invites drift.

---

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `state::can_transition` — exhaustive table-driven (108 cases).
- V-UT-2: `state::load` / `state::save` round-trip.
- V-UT-3: `state::load` on corrupt TOML → `Error::TaskTomlCorrupt` with source preserved.
- V-UT-4: `task_new` writes dir + PRD + task.toml + `.current`; second call errors `TaskAlreadyExists`.
- V-UT-5: `task_plan`/`_review`/`_execute`/`_verify` — legal succeeds, illegal errors, `updated_at` bumps.
- V-UT-6: `task_archive` standard tier — moves dir, updates `.current`, no SPEC side effects.
- V-UT-7: `task_archive` deep tier — moves dir, calls extract + register, writes expected files.
- V-UT-8: `task_iterate` — from Review bumps NN, writes `NN+1_PLAN.md` + `NN+1_REVIEW.md`, sets phase back to Plan; from any other phase → `IllegalPhaseTransition`.
- V-UT-9: `task_promote` — legal tier swap; illegal when current phase doesn't exist under target tier; **PRD/PLAN bytes unchanged** after successful promote (byte-compare assertion, per R-005).
- V-UT-10: `task_reopen` — archived → active, error on slug collision.
- V-UT-11: `spec_extract` — PLAN with `## Spec` → writes SPEC.md; PLAN without → `SpecSectionMissing`; existing SPEC → CHANGELOG appended; non-deep tier → `WrongTier`.
- V-UT-12: `spec_register` — empty INDEX managed block → row appended; existing row → replaced; different feature → appended.
- V-UT-13: `template_copy` — known name writes exact bytes; unknown name → `UnknownTemplate`.
- **V-UT-14 (R-002):** `spec_extract` on PLAN whose header is `## Spec \`{Core specification}\`` (inline-code suffix) — extracts correctly.
- **V-UT-15 (R-002):** `spec_extract` on PLAN containing `### Subheading` inside the `## Spec` section — body includes the subheading; scanner does NOT terminate at `###`.
- **V-UT-16 (R-006):** `TaskToml::status()` — `Archived → Completed`, anything else → `InProgress`.
- **V-UT-17 (new, PathExt):** `PathExt::rename_to` — rename within tempdir succeeds; rename from nonexistent source → `Error::Io`.
- **V-UT-18 (R-005):** `spec_register` with `|` in `--feature` → `InvalidSpecField`; with `\n` in `--scope` → `InvalidSpecField`.

[**Integration Tests**]

- **V-IT-1:** Standard-tier round-trip (sequence above).
- **V-IT-2 (R-011):** Deep-tier round-trip with iteration — explicit sequence: `new(deep) → plan → review → iterate → review → execute → verify → archive`. Asserts all four `00_*` + `01_*` PLAN/REVIEW artifacts present in archive; `phase = archived`; SPEC promoted; INDEX row present.
- **V-IT-3:** CLI help snapshot.
- **V-IT-4 (R-004):** Embedded `templates/claude/commands/ark/{quick,design}.md` grepped for raw recipes; asserts absence of `mkdir -p ".ark/tasks`, `cp .ark/templates/`, and `echo ` redirecting into `.ark/tasks/`.

[**Failure / Robustness Validation**]

- V-F-1: `task_new` with pre-existing dir → `TaskAlreadyExists`, no partial write.
- V-F-2: `task_archive` with corrupt `task.toml` → `TaskTomlCorrupt`, dir not moved.
- V-F-3: `spec_extract` without `## Spec` section → `SpecSectionMissing`, nothing written.
- V-F-4: Illegal transition leaves `task.toml` unchanged (byte-compare).
- **V-F-5 (R-005):** `spec_register` against `specs/features/INDEX.md` with `ARK:FEATURES:START` but no matching `:END` — test documents and asserts `update_managed_block`'s existing behavior in this case.

[**Edge Case Validation**]

- V-E-1: `--slug` omitted with missing `.current` → `Error::NoCurrentTask { path }` (per R-007, no more angle-bracketed fake-slug trick).
- V-E-2: `task_archive` when `.current` points at a different slug → moves the target task; does not clear `.current`.
- V-E-3: `task_iterate` when only `00_PLAN.md` exists (fresh task at Review) → creates `01_PLAN.md` + `01_REVIEW.md`.
- V-E-4: `task_archive` twice (already archived) → `IllegalPhaseTransition { from: Archived, to: Archived }`.
- V-E-5 (**resolved per R-005**): `spec_register` with `|` or `\n` in `--feature` / `--scope` → `Error::InvalidSpecField`. (See V-UT-18.)
- V-E-6: Concurrent `ark agent` invocations on the same task dir — no locking; documented as "agent should not parallelize against the same slug."

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-3 |
| G-2 | V-UT-1, V-UT-5, V-UT-8 (iterate guard) |
| G-3 | V-UT-4, V-UT-6, V-UT-7, V-UT-8, V-UT-9, V-UT-10 |
| G-4 | V-UT-11, V-UT-12, V-UT-14, V-UT-15, V-UT-18, V-F-5 |
| G-5 | V-UT-13 |
| G-6 | V-UT-7, V-IT-2 |
| G-7 | V-IT-1, V-IT-2 (both assert one-line summary output) |
| G-8 | V-IT-4 + manual inspection during VERIFY |
| C-1 | V-IT-3 |
| C-2 | V-IT-3 |
| C-3 | Code review during VERIFY (`grep 'println!' crates/ark-core/src/commands/agent/` must be empty) |
| C-4 | Code review during VERIFY (`grep 'std::fs::' crates/ark-core/src/commands/agent/` must be empty) |
| C-5 | Code review during VERIFY (`grep '\.join(\"\\.ark' crates/ark-core/src/commands/agent/` must be empty) |
| C-6 | V-UT-2, V-UT-3 |
| C-7 | V-UT-12, V-F-5 |
| C-8 | V-UT-17 (rename_to); V-F-2 (fails loud on corrupt) |
| C-9 | V-UT-9 (PRD/PLAN bytes unchanged) |
| C-10 | V-UT-13 |
| C-11 | V-UT-1, V-UT-5, V-UT-9, V-UT-11 (WrongTier), V-E-4 |
| C-12 | Code review during VERIFY — no `Command::new("ark")` in agent module |
| C-13 | V-UT-11, V-UT-14, V-UT-15 |
| C-14 | V-UT-18 |
