[**Goals**]

- G-1: Provide a hidden `ark agent` subcommand group — not listed in `ark --help`, but discoverable via `ark agent --help` — that documents itself as non-semver-stable.
- G-2: Expose explicit per-phase transition subcommands (`task plan`, `task review`, `task execute`, `task verify`, `task archive`) that enforce legal transitions per tier and reject illegal ones with a named error.
- G-3: Package task-directory scaffolding (`task new`), tier change (`task promote`), and terminal archival (`task archive` — which on deep tier also extracts and registers the feature SPEC) as CLI commands; rare operations (iteration, task reopening) are done by hand-editing `task.toml`.
- G-4: Package feature-SPEC operations: `spec extract` (final PLAN's `## Spec` → `specs/features/<slug>/SPEC.md`, appending CHANGELOG on overwrite) and `spec register` (upsert row in `specs/features/INDEX.md`'s managed block).
- G-5: `task archive` internally invokes SPEC extract + register when `task.toml.tier == "deep"`; the agent calls one command, not three.
- G-6: Every subcommand writes to disk, prints a one-line `impl Display` summary, and does not pipe structured data to siblings.
- G-7: Archival is user-invoked via the `/ark:archive` slash command. `/ark:design` and `/ark:quick` stop at VERIFY (or EXECUTE for quick tier) and never archive automatically.
- G-8: Update the shipped slash commands and workflow doc to reference `ark agent` commands in place of raw `mkdir`/`cp`/`echo`/manual-TOML recipes, kept in lockstep across both embedded templates and the live repo's copies.

- NG-1: No workspace/journal/identity subcommands. Reserved for a follow-up task.
- NG-2: No git or GitHub operations. The agent uses `git`/`gh` directly.
- NG-3: No content-generating commands. Content (PRD prose, PLAN sections, REVIEW verdicts) is the agent's judgment; `ark agent` owns only structural mutation.
- NG-4: No generic `ark agent set <k>=<v>` TOML editor. Every mutation is a named command.
- NG-5: No consistency-check command. Reviewer judgment, not mechanical check.
- NG-6: No public-API stability promise. Callers are the shipped slash commands and workflow doc; end users prefer those.
- NG-7: No CLI wrappers for operations that are genuinely rare and safe to hand-edit: iteration bump, task reopening, template copying.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                      — adds `Agent(AgentArgs)` hidden subcommand
└── ark-core/src/
    ├── lib.rs                                — re-exports agent public API
    ├── error.rs                              — new variants (see Data Structure)
    ├── io/
    │   └── path_ext.rs                       — adds read_text, list_dir, rename_to
    ├── templates.rs                          — unchanged
    ├── layout.rs                             — adds tasks_dir, tasks_archive_dir,
    │                                           tasks_current, task_dir,
    │                                           specs_features_dir, specs_feature_dir,
    │                                           specs_features_index, ark_templates_dir
    └── commands/
        └── agent/                            — the namespace module
            ├── mod.rs                        — pub mod task/spec/template;
            │                                   pub use state::{Phase, Status, Tier, TaskToml}
            ├── state.rs                      — TaskToml load/save + legal-transition table
            ├── task/
            │   ├── mod.rs
            │   ├── new.rs                    — scaffold task dir + PRD + task.toml + .current
            │   ├── phase.rs                  — plan/review/execute/verify, each guarded;
            │   │                               seeds NN_PLAN / NN_REVIEW / VERIFY templates
            │   ├── promote.rs                — tier change with legality guard; no artifact rewrite
            │   └── archive.rs                — move dir; on deep tier, explicitly calls
            │                                   super::spec::{extract, register}
            ├── spec/
            │   ├── mod.rs
            │   ├── extract.rs                — parse final PLAN's `## Spec` → SPEC.md
            │   └── register.rs               — managed-block row upsert
            └── template.rs                   — (internal) copy_template helper
```

**Module coupling.** `task::archive` imports `super::spec::{extract, register}` explicitly. `commands/agent/mod.rs` does NOT `pub use` peer modules — only `state` is re-exported for its types. Dependency direction is one-way: `task → spec → state`; `template` is a leaf. This avoids surprising visibility graphs.

**CLI wrapper omitted for `template`.** `copy_template` is `pub(crate)` inside the agent module and used by `task_new` and `task_plan`/`task_review`/`task_verify` to seed artifacts. There is no `ark agent template copy` CLI — copying an embedded file is something the agent can do equally well with `cp`.

Internal call graph for `task archive` (deep tier):

```
task::archive::task_archive(opts)
  ├── TaskToml::load(task_dir)
  ├── check_transition(tier, phase, Archived)
  ├── if tier == Deep:                        (side effects before rename so that
  │     ├── spec::extract::spec_extract(…)     failure leaves the task dir intact)
  │     └── spec::register::spec_register(…)
  ├── toml.phase = Archived; toml.archived_at = now; toml.save(task_dir)
  ├── PathExt::rename_to → tasks/archive/YYYY-MM/<slug>/
  └── remove .ark/tasks/.current if it pointed at <slug>
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/state.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier { Quick, Standard, Deep }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase { Design, Plan, Review, Execute, Verify, Archived }

/// Derived from `Phase`; not persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status { InProgress, Completed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskToml {
    pub id: String,
    pub title: String,
    pub tier: Tier,
    pub phase: Phase,
    pub iteration: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_iterations: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archived_at: Option<DateTime<Utc>>,
}

impl TaskToml {
    pub fn status(&self) -> Status { /* Archived => Completed else InProgress */ }
    pub fn load(task_dir: &Path) -> Result<Self>;
    pub fn save(&self, task_dir: &Path) -> Result<()>;
}

pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool;
pub fn check_transition(tier: Tier, from: Phase, to: Phase) -> Result<()>;
```

Error variants added to `ark-core/src/error.rs`:

```rust
Error::IllegalPhaseTransition { tier: Tier, from: Phase, to: Phase },
Error::WrongTier              { expected: Tier, actual: Tier },
Error::TaskNotFound           { slug: String },
Error::TaskAlreadyExists      { slug: String },
Error::NoCurrentTask          { path: PathBuf },
Error::UnknownTemplate        { name: String },
Error::SpecSectionMissing     { plan_path: PathBuf },
Error::NoPlanFound            { task_dir: PathBuf },
Error::TaskTomlCorrupt        { path: PathBuf, source: toml::de::Error },
Error::InvalidSpecField       { field: String, reason: &'static str },
Error::ManagedBlockCorrupt    { path: PathBuf, marker: String },
```

`PathExt` additions (`ark-core/src/io/path_ext.rs`):

```rust
fn read_text(&self) -> Result<String>;                     // UTF-8 text; errors on missing
fn list_dir(&self) -> Result<fs::ReadDir>;                 // named to avoid `Path::read_dir` inherent-method shadowing
fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>; // rename; fails loud on cross-device
```

[**API Surface**]

Library re-exports from `ark-core/src/lib.rs`:

```rust
pub use commands::agent::{
    Phase, Status, TaskToml, Tier,
    spec::{
        SpecExtractOptions, SpecExtractSummary,
        SpecRegisterOptions, SpecRegisterSummary,
        spec_extract, spec_register,
    },
    task::{
        TaskArchiveOptions, TaskArchiveSummary,
        TaskNewOptions, TaskNewSummary,
        TaskPhaseOptions, TaskPhaseSummary,
        TaskPromoteOptions, TaskPromoteSummary,
        task_archive, task_execute, task_new, task_plan, task_promote, task_review, task_verify,
    },
};
```

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum Command {
    Init(...), Load(...), Unload(...), Remove(...),
    /// `hide = true` hides the variant from `ark --help`;
    /// `ark agent --help` still renders its children and about-text.
    #[command(hide = true)]
    Agent(AgentArgs),
}
```

Nine subcommands, grouped under `agent`:

| Command | Arguments |
|---|---|
| `ark agent task new` | `--slug <s> --title "<t>" --tier {quick\|standard\|deep}` |
| `ark agent task plan` | `[--slug <s>]` |
| `ark agent task review` | `[--slug <s>]` |
| `ark agent task execute` | `[--slug <s>]` |
| `ark agent task verify` | `[--slug <s>]` |
| `ark agent task archive` | `[--slug <s>]` |
| `ark agent task promote` | `[--slug <s>] --to <tier>` |
| `ark agent spec extract` | `[--slug <s>] [--plan <path>]` |
| `ark agent spec register` | `--feature <f> --scope "<s>" --from-task <t> [--date YYYY-MM-DD]` |

Every `--slug`-taking command defaults to `.ark/tasks/.current` when the flag is omitted. Missing `.current` → `Error::NoCurrentTask`.

[**Constraints**]

- C-1: `ark --help` MUST NOT list `agent`. Verified by the CLI snapshot test.
- C-2: `ark agent --help` MUST include the string "Not covered by semver".
- C-3: Every `ark agent` subcommand's output goes through `Display`-returning summary types; no ad-hoc `println!` in subcommand bodies.
- C-4: All filesystem access in `commands/agent/` routes through `io::PathExt` (no bare `std::fs::*`).
- C-5: All `.ark/`-relative path composition routes through `layout::Layout` helpers.
- C-6: `task.toml` parsing/writing uses the `toml` crate. Corrupt files produce `Error::TaskTomlCorrupt` with source error chained.
- C-7: `spec register` uses `io::update_managed_block` with the marker `ARK:FEATURES`.
- C-8: `task archive`'s directory move uses `PathExt::rename_to` (rename semantics); fails loud on cross-device moves — no copy+delete fallback.
- C-9: Illegal phase transitions produce `Error::IllegalPhaseTransition`. Deep-only operations invoked with the wrong tier produce `Error::WrongTier`. No silent success.
- C-10: `ark agent` subcommands depend on each other via direct function calls only. MUST NOT shell out to `ark` itself.
- C-11: **`## Spec` section-scan predicate.** Start line matches when `line.starts_with("## Spec")` AND (`line.len() == 7` OR `line.as_bytes()[7] == b' '`) — accepts `"## Spec"` and `"## Spec \`…\`"` but rejects `"## Speculation"`. End boundary is the first subsequent line where `line.starts_with("## ")` OR `line == "##"` — matches any H2 but NOT `### subheadings`. EOF also terminates.
- C-12: `spec register`'s `--feature`, `--scope`, and `--from-task` args are trimmed then rejected if empty, containing `|`, or containing `\n`/`\r`. Violation → `Error::InvalidSpecField`.
- C-13: `update_managed_block` refuses to write when an orphan START marker is present (START without matching END), returning `Error::ManagedBlockCorrupt`. Prevents silent corruption that would manifest on subsequent reads.
- C-14: Archival is always user-invoked via `/ark:archive`. Slash commands `/ark:design` and `/ark:quick` stop at VERIFY / EXECUTE and tell the user to run `/ark:archive`; they do not archive automatically.
