//! `ark` — Phase 0 CLI.
//!
//! A thin adapter over [`ark_core`]: parse args, dispatch, print the summary.
//! All logic lives in the library.

use std::{
    fmt::Display,
    io::{BufRead, IsTerminal},
    path::{Path, PathBuf},
    process::ExitCode,
};

use ark_core::{
    ConflictChoice, ConflictPolicy, InitOptions, Layout, LoadOptions, PathExt, Prompter,
    RemoveOptions, SpecExtractOptions, SpecRegisterOptions, TaskArchiveOptions, TaskNewOptions,
    TaskPhaseOptions, TaskPromoteOptions, Tier, UnloadOptions, UpgradeOptions, WriteMode, init,
    load, remove, spec_extract, spec_register, task_archive, task_execute, task_new, task_plan,
    task_promote, task_review, task_verify, unload, upgrade,
};
use chrono::NaiveDate;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ark",
    version,
    about = "A simple CLI agent harness and development workflow for orchestrating AI-driven programming tasks",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold `.ark/` and Claude Code integration from the embedded templates.
    Init(InitArgs),
    /// Bring Ark into a project: restore from `.ark.db` if present, else scaffold.
    Load(LoadArgs),
    /// Freeze Ark state into `.ark.db` and remove the live files.
    Unload(TargetArgs),
    /// Remove Ark from the project, including any `.ark.db` snapshot.
    Remove(TargetArgs),
    /// Refresh embedded templates to the current CLI version.
    Upgrade(UpgradeArgs),
    /// Internal commands invoked by the Ark workflow and slash commands.
    /// Not covered by semver — prefer the slash commands over calling these directly.
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(clap::Args)]
struct InitArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Overwrite files that differ from the shipped templates.
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
struct LoadArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Wipe any existing `.ark/` before loading (otherwise errors if loaded).
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
#[group(id = "policy", multiple = false)]
struct UpgradeArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Overwrite user-modified files without prompting.
    #[arg(long, group = "policy")]
    force: bool,
    /// Preserve user-modified files without prompting.
    #[arg(long, group = "policy")]
    skip_modified: bool,
    /// Write updated template as `<path>.new` without prompting.
    #[arg(long, group = "policy")]
    create_new: bool,
    /// Allow proceeding when CLI version < project version.
    #[arg(long)]
    allow_downgrade: bool,
}

impl UpgradeArgs {
    fn policy(&self) -> ConflictPolicy {
        // Exclusivity is enforced by clap's `ArgGroup`, so at most one flag is set.
        if self.force {
            ConflictPolicy::Force
        } else if self.skip_modified {
            ConflictPolicy::Skip
        } else if self.create_new {
            ConflictPolicy::CreateNew
        } else {
            ConflictPolicy::Interactive
        }
    }
}

/// Shared `-C DIR` flag used by every subcommand.
#[derive(clap::Args)]
struct TargetArgs {
    /// Target directory (defaults to the current working directory).
    #[arg(short = 'C', long, value_name = "DIR", global = false)]
    dir: Option<PathBuf>,
}

impl TargetArgs {
    fn resolve(self) -> PathBuf {
        let raw = self
            .dir
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        absolutize(&raw)
    }
}

/// Resolve `path` to absolute, joining against the current working directory
/// when relative. Falls back to the path as-given if cwd lookup fails.
fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn main() -> ExitCode {
    match Cli::parse().command.dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            std::iter::successors(Some(&*err as &dyn std::error::Error), |e| e.source())
                .enumerate()
                .for_each(|(i, e)| match i {
                    0 => eprintln!("error: {e}"),
                    _ => eprintln!("  caused by: {e}"),
                });
            ExitCode::FAILURE
        }
    }
}

impl Command {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::Init(a) => {
                let root = a.target.resolve();
                let mode = if a.force {
                    WriteMode::Force
                } else {
                    WriteMode::Skip
                };
                announce("initializing ark in", &root);
                render(init(InitOptions::new(root).with_mode(mode))?);
            }
            Self::Load(a) => {
                let root = a.target.resolve();
                announce("loading ark into", &root);
                render(load(LoadOptions::new(root).with_force(a.force))?);
            }
            Self::Unload(a) => {
                let root = a.resolve();
                announce("unloading ark from", &root);
                render(unload(UnloadOptions::new(root))?);
            }
            Self::Remove(a) => {
                let root = a.resolve();
                announce("removing ark from", &root);
                render(remove(RemoveOptions::new(root))?);
            }
            Self::Upgrade(a) => {
                let policy = a.policy();
                let root = a.target.resolve();
                if matches!(policy, ConflictPolicy::Interactive) && !std::io::stdin().is_terminal()
                {
                    eprintln!(
                        "note: stdin is not a terminal; defaulting user-modified files to \
                         preserve. Use --force/--skip-modified/--create-new for non-interactive \
                         control."
                    );
                }
                let opts = UpgradeOptions::new(root.clone())
                    .with_policy(policy)
                    .with_allow_downgrade(a.allow_downgrade);
                let mut prompter = StdioPrompter;
                announce("upgrading ark in", &root);
                render(upgrade(opts, &mut prompter)?);
            }
            Self::Agent(a) => a.dispatch()?,
        }
        Ok(())
    }
}

/// Reads a single line from stdin per conflict. On non-TTY stdin, short-circuits
/// to Skip. The one-shot "not a terminal" note is emitted by the `Upgrade`
/// dispatch arm, not here — constructors should not have I/O side effects.
struct StdioPrompter;

impl Prompter for StdioPrompter {
    fn prompt(&mut self, relative_path: &Path) -> ark_core::Result<ConflictChoice> {
        if !std::io::stdin().is_terminal() {
            return Ok(ConflictChoice::Skip);
        }
        eprint!(
            "{}: [o]verwrite / [s]kip / [c]reate .new? ",
            relative_path.display()
        );
        let mut line = String::new();
        let stdin = std::io::stdin();
        stdin.lock().read_line(&mut line).ok();
        Ok(match line.trim() {
            "o" | "O" | "y" | "Y" => ConflictChoice::Overwrite,
            "c" | "C" => ConflictChoice::CreateNew,
            _ => ConflictChoice::Skip,
        })
    }
}

// ===== `ark agent` =====
//
// Hidden namespace packaging the structural workflow operations as Rust
// subcommands. Not covered by semver.

#[derive(clap::Args)]
struct AgentArgs {
    #[command(subcommand)]
    command: AgentCommand,
}

#[derive(Subcommand)]
enum AgentCommand {
    /// Task-lifecycle operations.
    Task(TaskArgs),
    /// Feature-SPEC operations.
    Spec(SpecArgs),
}

#[derive(clap::Args)]
struct TaskArgs {
    #[command(subcommand)]
    command: TaskCommand,
}

#[derive(Subcommand)]
enum TaskCommand {
    /// Scaffold a new task directory with PRD + task.toml.
    New(TaskNewCliArgs),
    /// Transition: -> Plan.
    Plan(TaskSlugArgs),
    /// Transition: -> Review (deep tier).
    Review(TaskSlugArgs),
    /// Transition: -> Execute.
    Execute(TaskSlugArgs),
    /// Transition: -> Verify.
    Verify(TaskSlugArgs),
    /// Transition: -> Archived; deep tier extracts + registers SPEC.
    Archive(TaskSlugArgs),
    /// Change tier mid-flight. Does not rewrite artifacts.
    Promote(TaskPromoteCliArgs),
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
    /// quick | standard | deep
    #[arg(long, value_parser = parse_tier)]
    tier: Tier,
}

#[derive(clap::Args)]
struct TaskSlugArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Task slug. Defaults to the value in `.ark/tasks/.current`.
    #[arg(long)]
    slug: Option<String>,
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
    /// Extract the final PLAN's `## Spec` section into specs/features/<slug>/SPEC.md.
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
    fn dispatch(self) -> anyhow::Result<()> {
        match self.command {
            AgentCommand::Task(a) => a.command.dispatch(),
            AgentCommand::Spec(a) => a.command.dispatch(),
        }
    }
}

impl TaskCommand {
    fn dispatch(self) -> anyhow::Result<()> {
        match self {
            Self::New(a) => {
                let root = a.target.resolve();
                render(task_new(TaskNewOptions {
                    project_root: root,
                    slug: a.slug,
                    title: a.title,
                    tier: a.tier,
                })?);
            }
            Self::Plan(a) => run_phase(a, task_plan)?,
            Self::Review(a) => run_phase(a, task_review)?,
            Self::Execute(a) => run_phase(a, task_execute)?,
            Self::Verify(a) => run_phase(a, task_verify)?,
            Self::Archive(a) => {
                let root = a.target.resolve();
                let slug = resolve_slug(&root, a.slug)?;
                render(task_archive(TaskArchiveOptions {
                    project_root: root,
                    slug,
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

/// Resolve `--slug` with fallback to `.ark/tasks/.current`.
fn resolve_slug(root: &Path, explicit: Option<String>) -> anyhow::Result<String> {
    if let Some(slug) = explicit {
        return Ok(slug);
    }
    let current = Layout::new(root).tasks_current();
    match current.read_text_optional()? {
        Some(text) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        _ => Err(ark_core::Error::NoCurrentTask { path: current }.into()),
    }
}

fn announce(verb: &str, root: &Path) {
    println!("{verb} {}", root.display());
}

fn render<S: Display>(summary: S) {
    println!("{summary}");
}
