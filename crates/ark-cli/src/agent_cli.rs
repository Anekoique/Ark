//! `ark agent` — hidden namespace packaging the structural workflow operations
//! as Rust subcommands. Not covered by semver — prefer the slash commands over
//! calling these directly.

use std::path::{Path, PathBuf};

use ark_core::{
    Layout, RealPpid, SpecExtractOptions, SpecRegisterOptions, TaskArchiveMoveOptions,
    TaskCommitOptions, TaskDiscardOptions, TaskNewOptions, TaskNewWorktree, TaskPhaseOptions,
    TaskPromoteOptions, TaskResumeOptions, TaskToml, Tier, WorktreeCleanupOptions,
    WorktreeListOptions, load_state, resolve_session_id, spec_extract, spec_register,
    task_archive_move, task_commit, task_discard, task_execute, task_new, task_plan, task_promote,
    task_resume, task_review, task_verify, worktree_cleanup, worktree_list,
};
use chrono::NaiveDate;
use clap::Subcommand;

use crate::{TargetArgs, render};

#[derive(clap::Args)]
pub(crate) struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Task-lifecycle operations.
    Task(TaskArgs),
    /// Feature-SPEC operations.
    Spec(SpecArgs),
    /// Workspace operations (developer journals, manual records).
    Workspace(WorkspaceArgs),
}

#[derive(clap::Args)]
struct WorkspaceArgs {
    #[command(subcommand)]
    command: WorkspaceCommand,
}

#[derive(Subcommand)]
enum WorkspaceCommand {
    /// Append a journal entry (manual or task-driven).
    Record(WorkspaceRecordArgs),
    /// Developer registrar (top-level Active Developers index).
    Developer(WorkspaceDeveloperArgs),
}

#[derive(clap::Args)]
struct WorkspaceRecordArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug for task-driven entries. Mutually exclusive with `--manual`.
    #[arg(long, conflicts_with = "manual")]
    task: Option<String>,
    /// Manual mode (no slug, no Closing Commit sentinel, no Git Commits table).
    #[arg(long, default_value_t = false)]
    manual: bool,
}

#[derive(clap::Args)]
struct WorkspaceDeveloperArgs {
    #[command(subcommand)]
    command: WorkspaceDeveloperCommand,
}

#[derive(Subcommand)]
enum WorkspaceDeveloperCommand {
    /// Register a developer in the top-level Active Developers index.
    Register(WorkspaceDeveloperRegisterArgs),
    /// Refresh an existing developer row (Last Active / Sessions / Active
    /// Journal). Falls through to `register` if no row exists yet.
    Touch(WorkspaceDeveloperRegisterArgs),
}

#[derive(clap::Args)]
struct WorkspaceDeveloperRegisterArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Developer name.
    #[arg(long)]
    name: String,
    /// Active journal filename (defaults to `journal-1.md`).
    #[arg(long = "active-journal", default_value = "journal-1.md")]
    active_journal: String,
    /// Session count (defaults to 1).
    #[arg(long = "session-count", default_value_t = 1)]
    session_count: u32,
}

#[derive(clap::Args)]
struct TaskArgs {
    #[command(subcommand)]
    command: TaskCommand,
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Scaffold a new task directory with PRD and task.toml.
    ///
    /// Pass `--worktree` to bind the task to a git worktree under
    /// `.ark/worktrees/<branch>/`.
    New(TaskNewCliArgs),
    /// Transition: -> Plan.
    Plan(TaskSlugArgs),
    /// Transition: -> Review (deep tier).
    Review(TaskSlugArgs),
    /// Transition: -> Execute.
    Execute(TaskSlugArgs),
    /// Transition: -> Verify.
    Verify(TaskSlugArgs),
    /// Transition: -> Committed. Atomic: VERIFY gate, deep-tier SPEC extract,
    /// stage Ark-managed files, single `git commit` covering work + journal +
    /// task.toml + (deep) SPEC + features INDEX.
    Commit(TaskCommitCliArgs),
    /// Transition: -> Archived. Side-effect-free move of a committed task to
    /// `tasks/archive/<month>/<slug>/`. Defaults `--month` to the task's own
    /// `committed_at`. Bulk archive lives at the top-level `ark archive` CLI.
    Archive(TaskArchiveCliArgs),
    /// Claim an active task as this session's focus.
    Resume(TaskResumeCliArgs),
    /// Discard an unarchived task; refuses without `--force` when seeded files have user content.
    Discard(TaskDiscardCliArgs),
    /// Change tier mid-flight. Does not rewrite artifacts.
    Promote(TaskPromoteCliArgs),
    /// Worktree lifecycle (cleanup / list). Creation lives in `task new --worktree`.
    Worktree(WorktreeCliArgs),
}

#[derive(clap::Args)]
struct TaskDiscardCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to this session's focused task in `.ark/.state.toml`.
    #[arg(long)]
    slug: Option<String>,
    /// Force discard even when seeded files have user content.
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
struct WorktreeCliArgs {
    #[command(subcommand)]
    command: WorktreeSubcommand,
}

#[derive(Subcommand)]
enum WorktreeSubcommand {
    /// Remove the worktree dir; optionally delete the branch.
    Cleanup(WorktreeCleanupCliArgs),
    /// List active worktree-backed tasks.
    List(WorktreeListCliArgs),
}

#[derive(clap::Args)]
struct WorktreeCleanupCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to the value in `.ark/tasks/.current`.
    #[arg(long)]
    slug: Option<String>,
    /// After removing the worktree dir, delete the git branch too.
    #[arg(long = "delete-branch")]
    delete_branch: bool,
    /// Force removal of dirty worktree and force-delete unmerged branch.
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
struct WorktreeListCliArgs {
    #[command(flatten)]
    target: TargetArgs,
}

#[derive(clap::Args)]
struct TaskNewCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug (filesystem-safe identifier).
    #[arg(long)]
    slug: String,
    /// One-line title.
    #[arg(long)]
    title: String,
    /// Task tier (`quick`, `standard`, or `deep`).
    #[arg(long, value_parser = parse_tier)]
    tier: Tier,
    /// Creates a git worktree under `.ark/worktrees/<branch>/`.
    ///
    /// The task directory is scaffolded inside that worktree.
    #[arg(long)]
    worktree: bool,
    /// Branch type prefix used when `--worktree` is set.
    ///
    /// One of `feat`, `fix`, `refactor`, `chore`, `ci`, or `docs`. Defaults
    /// to `config.toml`'s `[worktree].branch_prefix` (`feat`).
    /// Mutually exclusive with --branch.
    #[arg(long = "branch-type", conflicts_with = "branch", requires = "worktree")]
    branch_type: Option<String>,
    /// Full branch name override; bypasses `--branch-type/<slug>` resolution.
    /// Validated via `git check-ref-format --branch`.
    #[arg(long, conflicts_with = "branch_type", requires = "worktree")]
    branch: Option<String>,
}

#[derive(clap::Args)]
struct TaskSlugArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to this session's focused task in `.ark/.state.toml`.
    #[arg(long)]
    slug: Option<String>,
}

#[derive(clap::Args)]
struct TaskCommitCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to this session's focused task in `.ark/.state.toml`.
    #[arg(long)]
    slug: Option<String>,
    /// Commit message. Required unless `--no-commit` is set.
    #[arg(short = 'm', long = "message")]
    message: Option<String>,
    /// Skip git commit + journal write; deep tier still extracts SPEC and the
    /// phase still flips to Committed.
    #[arg(long = "no-commit", default_value_t = false)]
    no_commit: bool,
}

#[derive(clap::Args)]
struct TaskArchiveCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to this session's focused task in `.ark/.state.toml`.
    #[arg(long)]
    slug: Option<String>,
    /// `YYYY-MM` archive bucket. Defaults to the task's own `committed_at`
    /// month — set explicitly only to override historical placement.
    #[arg(long)]
    month: Option<String>,
}

#[derive(clap::Args)]
struct TaskResumeCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug to resume. Required — `task resume` has no implicit default.
    #[arg(long)]
    slug: String,
}

#[derive(clap::Args)]
struct TaskPromoteCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long)]
    slug: Option<String>,
    /// Target tier.
    #[arg(long = "to", value_parser = parse_tier)]
    to: Tier,
}

#[derive(clap::Args)]
struct SpecArgs {
    #[command(subcommand)]
    command: SpecCommand,
}

#[derive(Subcommand)]
enum SpecCommand {
    /// Extract the final PLAN's `## Spec` section into `specs/features/<slug>/SPEC.md`.
    Extract(SpecExtractCliArgs),
    /// Upsert a row in specs/features/INDEX.md.
    Register(SpecRegisterCliArgs),
}

#[derive(clap::Args)]
struct SpecExtractCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long)]
    slug: Option<String>,
    /// Optional explicit PLAN path. Default: highest-NN `NN_PLAN.md`.
    #[arg(long)]
    plan: Option<PathBuf>,
}

#[derive(clap::Args)]
struct SpecRegisterCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    #[arg(long)]
    feature: String,
    #[arg(long)]
    scope: String,
    #[arg(long = "from-task")]
    from_task: String,
    /// Override the registration date (YYYY-MM-DD). Default: today UTC.
    #[arg(long)]
    date: Option<String>,
}

fn parse_tier(s: &str) -> Result<Tier, String> {
    match s {
        "quick" => Ok(Tier::Quick),
        "standard" => Ok(Tier::Standard),
        "deep" => Ok(Tier::Deep),
        other => Err(format!(
            "unknown tier `{other}`; expected quick | standard | deep"
        )),
    }
}

impl AgentArgs {
    pub(crate) fn dispatch(self) -> anyhow::Result<()> {
        match self.command {
            AgentCommand::Task(a) => a.command.dispatch(),
            AgentCommand::Spec(a) => a.command.dispatch(),
            AgentCommand::Workspace(a) => a.command.dispatch(),
        }
    }
}

impl WorkspaceCommand {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::Record(a) => {
                let root = a.target.resolve();
                let summary = if a.manual {
                    ark_core::workspace_record(ark_core::RecordOptions::new(
                        &root,
                        ark_core::RecordMode::Manual,
                    ))?
                } else {
                    let slug = a
                        .task
                        .ok_or_else(|| anyhow::anyhow!("--task <slug> or --manual is required"))?;
                    let toml = TaskToml::load(&Layout::new(&root).task_dir(&slug))?;
                    let branch = toml.branch.clone().unwrap_or_else(|| "HEAD".into());
                    let base_branch = toml.base_branch.clone().unwrap_or_else(|| "main".into());
                    let start_head_short = toml
                        .start_head
                        .as_deref()
                        .map(|s| s.chars().take(7).collect::<String>())
                        .unwrap_or_else(|| "<unknown>".into());
                    let mode = ark_core::RecordMode::Task {
                        slug: &slug,
                        branch: &branch,
                        base_branch: &base_branch,
                        start_head_short: &start_head_short,
                        commits_in_range: &[],
                    };
                    ark_core::workspace_record(ark_core::RecordOptions::new(&root, mode))?
                };
                println!(
                    "recorded session {} in {}",
                    summary.session_number(),
                    summary.journal_path_relative(),
                );
                Ok(())
            }
            Self::Developer(a) => a.command.dispatch(),
        }
    }
}

impl WorkspaceDeveloperCommand {
    fn dispatch(self) -> anyhow::Result<()> {
        // `Register` and `Touch` share an implementation today; the verb
        // distinction lives in the slash-command surface for SPEC clarity.
        let args = match self {
            Self::Register(a) | Self::Touch(a) => a,
        };
        let summary = ark_core::developer_register(ark_core::DeveloperRegisterOptions {
            project_root: args.target.resolve(),
            name: args.name,
            active_journal: args.active_journal,
            date: chrono::Local::now().date_naive(),
            session_count: args.session_count,
        })?;
        render(summary);
        Ok(())
    }
}

impl TaskCommand {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::New(a) => {
                let root = a.target.resolve();
                let worktree = if a.worktree {
                    Some(TaskNewWorktree {
                        branch_type: a.branch_type,
                        branch_override: a.branch,
                    })
                } else {
                    None
                };
                render(task_new(TaskNewOptions {
                    project_root: root,
                    slug: a.slug,
                    title: a.title,
                    tier: a.tier,
                    worktree,
                })?);
            }
            Self::Plan(a) => run_phase(a, task_plan)?,
            Self::Review(a) => run_phase(a, task_review)?,
            Self::Execute(a) => run_phase(a, task_execute)?,
            Self::Verify(a) => run_phase(a, task_verify)?,
            Self::Commit(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                render(task_commit(TaskCommitOptions {
                    project_root: root,
                    slug,
                    message: a.message,
                    no_commit: a.no_commit,
                })?);
            }
            Self::Archive(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                let archive_month = match a.month {
                    Some(m) => m,
                    None => derive_archive_month(&root, &slug)?,
                };
                render(task_archive_move(TaskArchiveMoveOptions {
                    project_root: root,
                    slug,
                    archive_month,
                })?);
            }
            Self::Resume(a) => {
                let root = a.target.resolve();
                render(task_resume(TaskResumeOptions {
                    project_root: root,
                    slug: a.slug,
                })?);
            }
            Self::Discard(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                render(task_discard(TaskDiscardOptions {
                    project_root: root,
                    slug,
                    force: a.force,
                })?);
            }
            Self::Promote(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                render(task_promote(TaskPromoteOptions {
                    project_root: root,
                    slug,
                    to: a.to,
                })?);
            }
            Self::Worktree(a) => match a.command {
                WorktreeSubcommand::Cleanup(c) => {
                    let root = c.target.resolve();
                    let slug = resolve_slug(&root, c.slug)?;
                    render(worktree_cleanup(WorktreeCleanupOptions {
                        project_root: root,
                        slug,
                        delete_branch: c.delete_branch,
                        force: c.force,
                    })?);
                }
                WorktreeSubcommand::List(l) => {
                    let root = l.target.resolve();
                    render(worktree_list(WorktreeListOptions { project_root: root })?);
                }
            },
        }
        Ok(())
    }
}

fn run_phase(
    a: TaskSlugArgs,
    f: impl FnOnce(TaskPhaseOptions) -> ark_core::Result<ark_core::TaskPhaseSummary>,
) -> anyhow::Result<()> {
    let root = a.target.resolve();
    let slug = resolve_slug(&root, a.slug)?;
    render(f(TaskPhaseOptions {
        project_root: root,
        slug,
    })?);
    Ok(())
}

impl SpecCommand {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::Extract(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                render(spec_extract(SpecExtractOptions {
                    project_root: root,
                    slug,
                    plan_override: a.plan,
                    task_dir_override: None,
                })?);
            }
            Self::Register(a) => {
                let root = a.target.resolve();
                let date = match a.date {
                    Some(s) => NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                        .map_err(|e| anyhow::anyhow!("invalid --date `{s}`: {e}"))?,
                    None => chrono::Utc::now().date_naive(),
                };
                render(spec_register(SpecRegisterOptions {
                    project_root: root,
                    feature: a.feature,
                    scope: a.scope,
                    from_task: a.from_task,
                    date,
                })?);
            }
        }
        Ok(())
    }
}

/// Resolve `--slug` with fallback to this session's focused slug in `.state.toml`.
fn resolve_slug(root: &Path, explicit: Option<String>) -> anyhow::Result<String> {
    if let Some(slug) = explicit {
        return Ok(slug);
    }
    let layout = Layout::new(root);
    let ppid = RealPpid::new();
    let id = resolve_session_id(&layout, &ppid)?;
    let state = load_state(&layout, &ppid)?;
    match state.sessions.get(id.as_str()).map(|s| s.focus.clone()) {
        Some(slug) if !slug.is_empty() => Ok(slug),
        _ => Err(ark_core::Error::NoCurrentTask {
            path: layout.state_file(),
        }
        .into()),
    }
}

/// Reads the slug's `task.toml` and returns its `committed_at` month as `YYYY-MM`.
///
/// Used by `ark agent task archive` when `--month` is omitted so the archive
/// bucket always reflects the task's *commit* time, not the archive run's
/// wall clock. Errors with `CommittedAtMissing` if the task is in
/// `Committed` phase but has no `committed_at` field.
fn derive_archive_month(root: &Path, slug: &str) -> anyhow::Result<String> {
    let layout = Layout::new(root);
    let task_dir = layout.task_dir(slug);
    let toml = TaskToml::load(&task_dir)?;
    let when = toml
        .committed_at
        .ok_or_else(|| ark_core::Error::CommittedAtMissing {
            slug: slug.to_string(),
        })?;
    Ok(when.format("%Y-%m").to_string())
}
