[**Goals**]

- G-1: Hidden `ark agent` subcommand group, not in `ark --help`, not semver-stable.
- G-2: Per-phase task verbs (`plan` / `review` / `execute` / `verify` / `archive`) enforce legal transitions per tier.
- G-3: Structural ops (`task new` / `task promote` / `task archive`) packaged as CLI commands; rare ops are hand-edits.
- G-4: Feature-SPEC ops (`spec extract` / `spec register`) promote the final PLAN's `## Spec` verbatim on deep-tier archive.
- G-5: Every subcommand prints a one-line `Display` summary; no structured-data piping between siblings.

[**Non-goals**]

- NG-1: No content-generating commands; structural mutation only.
- NG-2: No public-API stability promise on the `agent` namespace.
- NG-3: No CLI wrappers for genuinely rare hand-edits (iteration bump, reopen).

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                  (adds Agent(AgentArgs) hidden subcommand)
└── ark-core/src/
    ├── error.rs                         (new error variants — see Data Structure)
    ├── layout.rs                        (tasks_dir, tasks_archive_dir, tasks_current,
    │                                      task_dir, specs_features_dir, specs_feature_dir,
    │                                      specs_features_index, ark_templates_dir)
    ├── io/path_ext.rs                   (read_text, list_dir, rename_to)
    └── commands/agent/
        ├── mod.rs                       (pub mod task/spec/template;
        │                                  pub use state::{Phase, Status, Tier, TaskToml})
        ├── state.rs                     (TaskToml load/save + legal-transition table)
        ├── task/
        │   ├── mod.rs
        │   ├── new.rs                   (scaffold task dir + PRD + task.toml + .current)
        │   ├── phase.rs                 (plan/review/execute/verify; each guarded;
        │   │                              seeds NN_PLAN / NN_REVIEW / VERIFY templates)
        │   ├── promote.rs               (tier change with legality guard; no artifact rewrite)
        │   └── archive.rs               (move dir; on deep tier, calls super::spec::*)
        ├── spec/
        │   ├── mod.rs
        │   ├── extract.rs               (parse final PLAN's `## Spec` → SPEC.md;
        │   │                              append CHANGELOG on overwrite)
        │   └── register.rs              (managed-block row upsert in features/INDEX.md)
        └── template.rs                  (internal copy_template helper; no CLI wrapper)
```

Module coupling: `task::archive` imports `super::spec::{extract, register}` directly. `commands/agent/mod.rs` does NOT `pub use` peer modules — only `state` is re-exported for its types. Dependency direction: `task → spec → state`; `template` is a leaf. `task::new`, `task::phase`, and `task::verify` use `template::copy_template` (`pub(crate)`).

Call graph for `task archive` (deep tier — SPEC promotion before rename so that failure leaves the task dir intact):

```
task::archive::task_archive(opts)
  ├── TaskToml::load(task_dir)
  ├── check_transition(tier, phase, Archived)
  ├── if tier == Deep:
  │     ├── spec::extract::spec_extract(...)        → writes specs/features/<slug>/SPEC.md
  │     └── spec::register::spec_register(...)      → upserts row in features/INDEX.md
  ├── toml.phase = Archived
  ├── toml.archived_at = now
  ├── toml.save(task_dir)
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
    pub fn status(&self) -> Status;       // Archived → Completed; else InProgress
    pub fn load(task_dir: &Path) -> Result<Self>;
    pub fn save(&self, task_dir: &Path) -> Result<()>;
}

pub fn can_transition(tier: Tier, from: Phase, to: Phase) -> bool;
pub fn check_transition(tier: Tier, from: Phase, to: Phase) -> Result<()>;

// ark-core/src/error.rs (additions)
Error::IllegalPhaseTransition { tier: Tier, from: Phase, to: Phase },
Error::WrongTier              { expected: Tier, actual: Tier },
Error::TaskNotFound           { slug: String },
Error::TaskAlreadyExists      { slug: String },
Error::NoFocus                { project_root: PathBuf, candidates: Vec<String> },
Error::UnknownTemplate        { name: String },
Error::SpecSectionMissing     { plan_path: PathBuf },
Error::NoPlanFound            { task_dir: PathBuf },
Error::TaskTomlCorrupt        { path: PathBuf, source: toml::de::Error },
Error::InvalidSpecField       { field: String, reason: &'static str },
Error::ManagedBlockCorrupt    { path: PathBuf, marker: String },

// ark-core/src/io/path_ext.rs (additions)
trait PathExt {
    fn read_text(&self) -> Result<String>;                             // UTF-8; errors on missing
    fn list_dir(&self) -> Result<fs::ReadDir>;                         // avoids inherent-method shadow
    fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>;         // fails loud on cross-device
}
```

[**API Surface**]

```rust
// Library re-exports from ark-core/src/lib.rs
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
        task_archive, task_execute, task_new, task_plan,
        task_promote, task_review, task_verify,
    },
};

// CLI shape (ark-cli/src/main.rs)
#[derive(Subcommand)]
enum Command {
    Init(...), Load(...), Unload(...), Remove(...),
    /// `hide = true` hides the variant from `ark --help`;
    /// `ark agent --help` still renders its children.
    #[command(hide = true)]
    Agent(AgentArgs),
}
```

Subcommands under `ark agent`:

```
ark agent task new       --slug <s> --title "<t>" --tier <quick|standard|deep>
ark agent task plan
ark agent task review
ark agent task execute
ark agent task verify
ark agent task archive
ark agent task promote   --to <tier>
ark agent task resume    --slug <s>
ark agent task discard   --slug <s> [--force]
ark agent spec extract   [--plan <path>]
ark agent spec register  --feature <f> --scope "<s>" --from-task <t> [--date YYYY-MM-DD]
```

[**Constraints**]

- C-1: @test-binding: top_level_help_does_not_mention_agent
`ark --help` does not list `agent`.
- C-2: @test-binding: agent_help_includes_stability_banner
`ark agent --help` includes the string "Not covered by semver".
- C-3: @judgment
Every subcommand's output goes through a `Display`-returning summary; no ad-hoc `println!`.
- C-4: @judgment
All filesystem access in `commands/agent/` routes through `io::PathExt`.
- C-5: @judgment
All `.ark/`-relative path composition routes through `layout::Layout`.
- C-6: @test-binding: load_errors_on_corrupt_toml
`task.toml` parsing/writing uses the `toml` crate; corrupt files produce `Error::TaskTomlCorrupt`.
- C-7: @test-binding: registers_fresh_row
`spec register` uses `io::update_managed_block` with marker `ARK:FEATURES`.
- C-8: @test-binding: rename_to_moves_file
`task archive` directory move uses `PathExt::rename_to`; fails loud on cross-device.
- C-9: @test-binding: illegal_phase_under_target_tier_errors
Illegal phase transitions return `Error::IllegalPhaseTransition`; deep-only ops on wrong tier return `Error::WrongTier`.
- C-10: @judgment
`ark agent` subcommands depend on each other via direct calls only; never shell out to `ark`.
- C-11: @test-binding: extracts_with_inline_code_suffix
`## Spec` section-scan: start matches `line == "## Spec"` or `line.starts_with("## Spec ")`; end matches the next `^## ` H2 or EOF; rejects `## Speculation`.
- C-12: @test-binding: rejects_pipe_in_feature
`spec register` arg validation rejects empty strings or strings containing `|`, `\n`, `\r` → `Error::InvalidSpecField`.
- C-13: @test-binding: errors_on_corrupt_managed_block
`update_managed_block` refuses to write on orphan START marker → `Error::ManagedBlockCorrupt`.
- C-14: @judgment
`--slug` is required only on `task new`, `task resume`, `task discard`. Other verbs read `.state.toml`'s `[focus]` field; absent focus → `Error::NoFocus { project_root, candidates }`.
- C-15: @judgment
Archival is user-invoked via `/ark:archive`; `/ark:design` and `/ark:quick` never archive automatically.

[**CHANGELOG**]

- 2026-05-06 `drop-task-slug`: `--slug` confined to `task new` / `resume` / `discard`; other verbs resolve via topology cascade. Added `Error::NoActiveTask` and `Error::AmbiguousActiveTask`.
- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
- 2026-05-08 `extract-spec-cmd`: added `ark agent spec import --feature <s> --scope "<s>" --from-file <p> --from-commit <sha>` for brownfield SPEC authoring. Shares `upsert_index_row` with `spec register`; INDEX rows use `from-task = "extracted"` sentinel. Added `Error::SpecAlreadyExists`.
- 2026-05-08 `session-focus-bind`: replaced topology cascade with per-checkout `[focus]` field in `.state.toml`. `--slug` still required on `new`/`resume`/`discard`; other verbs read `state.focus`. Removed `Error::NoActiveTask` and `Error::AmbiguousActiveTask`; added `Error::NoFocus { project_root, candidates }`.
