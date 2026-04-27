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
    ConflictChoice, ConflictPolicy, ContextFormat, ContextOptions, ContextScope, InitOptions,
    Layout, LoadOptions, PLATFORMS, PathExt, PhaseFilter, Platform, Prompter, RemoveOptions,
    SpecExtractOptions, SpecRegisterOptions, TaskArchiveOptions, TaskNewOptions, TaskPhaseOptions,
    TaskPromoteOptions, Tier, UnloadOptions, UpgradeOptions, WriteMode, context, init, load,
    remove, spec_extract, spec_register, task_archive, task_execute, task_new, task_plan,
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
    /// Print a structured snapshot of git + .ark/ workflow state.
    Context(ContextArgs),
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

    /// Install Claude Code integration (default: prompt on TTY).
    #[arg(long)]
    claude: bool,
    /// Install Codex CLI integration (default: prompt on TTY).
    #[arg(long)]
    codex: bool,
    /// Skip Claude Code integration.
    #[arg(long = "no-claude")]
    no_claude: bool,
    /// Skip Codex CLI integration.
    #[arg(long = "no-codex")]
    no_codex: bool,
}

/// Per-flag state from `InitArgs`: positive (`--<flag>`) vs negative
/// (`--no-<flag>`) for one platform. Pure data, easy to construct in tests.
#[derive(Debug, Default, Clone, Copy)]
struct PlatformFlag {
    on: bool,
    off: bool,
}

impl InitArgs {
    /// Map each platform's `cli_flag` to the parsed `PlatformFlag` state.
    fn flags(&self) -> Vec<(&'static Platform, PlatformFlag)> {
        PLATFORMS
            .iter()
            .copied()
            .map(|p| {
                let flag = match p.cli_flag {
                    "claude" => PlatformFlag {
                        on: self.claude,
                        off: self.no_claude,
                    },
                    "codex" => PlatformFlag {
                        on: self.codex,
                        off: self.no_codex,
                    },
                    _ => PlatformFlag::default(),
                };
                (p, flag)
            })
            .collect()
    }

    /// Resolve `Vec<&'static Platform>` from CLI flags + TTY state. Per
    /// codex-support G-3.
    fn resolve_platforms(&self) -> anyhow::Result<Vec<&'static Platform>> {
        let resolved =
            resolve_platforms_pure(&self.flags(), std::io::stdin().is_terminal(), || {
                interactive_select_platforms()
            })?;
        if resolved.is_empty() {
            anyhow::bail!("init requires at least one platform");
        }
        Ok(resolved)
    }
}

/// Pure resolution logic, factored for testability. The caller supplies
/// `is_tty` and a closure that runs the interactive prompt; the function
/// itself does no I/O.
///
/// - Positive flag (`--<flag>`) narrows to that subset.
/// - Negative flag (`--no-<flag>`) excludes.
/// - Both unset, TTY: run the interactive prompt.
/// - Both unset, non-TTY: error — no silent default.
fn resolve_platforms_pure(
    flags: &[(&'static Platform, PlatformFlag)],
    is_tty: bool,
    interactive: impl FnOnce() -> anyhow::Result<Vec<&'static Platform>>,
) -> anyhow::Result<Vec<&'static Platform>> {
    let any_positive = flags.iter().any(|(_, f)| f.on);
    let any_negative = flags.iter().any(|(_, f)| f.off);

    if any_positive {
        return Ok(flags
            .iter()
            .filter(|(_, f)| f.on && !f.off)
            .map(|(p, _)| *p)
            .collect());
    }
    if any_negative {
        return Ok(flags
            .iter()
            .filter(|(_, f)| !f.off)
            .map(|(p, _)| *p)
            .collect());
    }
    if is_tty {
        return interactive();
    }
    anyhow::bail!(
        "init requires --claude, --codex, or both when stdin is not a TTY (use --no-claude / \
         --no-codex to opt out)"
    );
}

/// Tiny stdin-driven multi-select. Each platform is offered with a default
/// of "yes". User types `y`/`n` (or just enter for default).
fn interactive_select_platforms() -> anyhow::Result<Vec<&'static Platform>> {
    eprintln!("Select integrations to install:");
    let mut chosen = Vec::with_capacity(PLATFORMS.len());
    for platform in PLATFORMS {
        eprint!("  install {} integration? [Y/n] ", platform.id);
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line).ok();
        if !matches!(line.trim().to_ascii_lowercase().as_str(), "n" | "no") {
            chosen.push(*platform);
        }
    }
    Ok(chosen)
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

#[derive(clap::Args)]
struct ContextArgs {
    #[command(flatten)]
    target: TargetArgs,

    /// Which projection to run.
    #[arg(long, value_enum, default_value = "session")]
    scope: ScopeArg,

    /// Phase to filter by. Required when --scope=phase; rejected otherwise.
    #[arg(long = "for", value_enum)]
    r#for: Option<PhaseArg>,

    /// Output format.
    #[arg(long, value_enum, default_value = "text")]
    format: FormatArg,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum ScopeArg {
    Session,
    Phase,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum PhaseArg {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum FormatArg {
    Json,
    Text,
}

impl From<PhaseArg> for PhaseFilter {
    fn from(p: PhaseArg) -> Self {
        match p {
            PhaseArg::Design => PhaseFilter::Design,
            PhaseArg::Plan => PhaseFilter::Plan,
            PhaseArg::Review => PhaseFilter::Review,
            PhaseArg::Execute => PhaseFilter::Execute,
            PhaseArg::Verify => PhaseFilter::Verify,
        }
    }
}

impl From<FormatArg> for ContextFormat {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Json => ContextFormat::Json,
            FormatArg::Text => ContextFormat::Text,
        }
    }
}

impl ContextArgs {
    fn resolve_scope(&self) -> Result<ContextScope, String> {
        match (self.scope, self.r#for) {
            (ScopeArg::Session, None) => Ok(ContextScope::Session),
            (ScopeArg::Session, Some(_)) => {
                Err("`--for <PHASE>` is only valid with `--scope=phase`".to_string())
            }
            (ScopeArg::Phase, None) => {
                Err("`--for <PHASE>` is required when `--scope=phase`".to_string())
            }
            (ScopeArg::Phase, Some(p)) => Ok(ContextScope::Phase(p.into())),
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
    /// Resolve to the explicit target (cwd, or `--dir`). No walk-up. Used by
    /// commands whose job is to scaffold or operate on a specific target
    /// directory (`init`, `load --force`).
    fn resolve(self) -> PathBuf {
        let raw = self
            .dir
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        absolutize(&raw)
    }

    /// Resolve and then walk ancestors looking for an Ark project root, per
    /// ark-context C-21. If `--dir` was given, it wins (no walk-up). Used by
    /// commands that require an existing `.ark/`.
    fn resolve_with_discovery(self) -> anyhow::Result<PathBuf> {
        // Explicit --dir always wins.
        if let Some(dir) = self.dir.as_ref() {
            return Ok(absolutize(dir));
        }
        let cwd = std::env::current_dir().unwrap_or_default();
        let cwd_abs = absolutize(&cwd);
        let layout = Layout::discover_from(&cwd_abs)?;
        Ok(layout.root().to_path_buf())
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
                // ark-context C-21 carve-out: init creates `.ark/`; no walk-up.
                let platforms = a.resolve_platforms()?;
                let root = a.target.resolve();
                let mode = if a.force {
                    WriteMode::Force
                } else {
                    WriteMode::Skip
                };
                announce("initializing ark in", &root);
                render(init(
                    InitOptions::new(root)
                        .with_mode(mode)
                        .with_platforms(platforms),
                )?);
            }
            Self::Load(a) => {
                // `ark load` works on the explicit target: it either restores
                // from a local `.ark.db` snapshot or scaffolds fresh. Both
                // branches operate on the cwd (or `--dir`); discovery would
                // wrongly refuse when only `.ark.db` is present.
                let root = a.target.resolve();
                announce("loading ark into", &root);
                render(load(LoadOptions::new(root).with_force(a.force))?);
            }
            Self::Unload(a) => {
                let root = a.resolve_with_discovery()?;
                announce("unloading ark from", &root);
                render(unload(UnloadOptions::new(root))?);
            }
            Self::Remove(a) => {
                // `ark remove` is unconditional cleanup — runs even when
                // `.ark/` is already gone (e.g. after `ark unload` left only
                // `.ark.db`). Resolve to the explicit target without
                // requiring an existing project.
                let root = a.resolve();
                announce("removing ark from", &root);
                render(remove(RemoveOptions::new(root))?);
            }
            Self::Upgrade(a) => {
                let policy = a.policy();
                let root = a.target.resolve_with_discovery()?;
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
            Self::Context(a) => {
                let scope = a.resolve_scope().map_err(|msg| anyhow::anyhow!("{msg}"))?;
                let format: ContextFormat = a.format.into();
                let root = a.target.resolve_with_discovery()?;
                let opts = ContextOptions::new(root)
                    .with_scope(scope)
                    .with_format(format);
                render(context(opts)?);
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn parse_init(argv: &[&str]) -> InitArgs {
        #[derive(Parser)]
        #[command(no_binary_name = true)]
        struct Wrapper {
            #[command(subcommand)]
            cmd: Wrapped,
        }
        #[derive(Subcommand)]
        enum Wrapped {
            Init(InitArgs),
        }
        let Wrapped::Init(a) = Wrapper::parse_from(argv).cmd;
        a
    }

    /// Resolution helper that drives `resolve_platforms_pure` with an
    /// explicit `is_tty` and panics if the interactive branch is reached.
    fn resolve(argv: &[&str], is_tty: bool) -> anyhow::Result<Vec<&'static Platform>> {
        let args = parse_init(argv);
        resolve_platforms_pure(&args.flags(), is_tty, || {
            unreachable!("test should not reach the interactive branch")
        })
    }

    fn ids(ps: &[&'static Platform]) -> Vec<&'static str> {
        ps.iter().map(|p| p.id).collect()
    }

    /// V-IT-12: `--no-claude` narrows to Codex only; `--no-codex` to Claude
    /// only; both → empty.
    #[test]
    fn cli_resolve_platforms_no_x_excludes() {
        assert_eq!(
            ids(&resolve(&["init", "--no-claude"], true).unwrap()),
            ["codex"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--no-codex"], true).unwrap()),
            ["claude-code"]
        );
        let neither = resolve(&["init", "--no-claude", "--no-codex"], true).unwrap();
        assert!(neither.is_empty(), "{neither:?}");
    }

    /// V-IT-12 (positive flags): `--codex` (no `--no-X`) narrows to Codex only.
    #[test]
    fn cli_resolve_platforms_positive_flags_narrow() {
        assert_eq!(
            ids(&resolve(&["init", "--codex"], true).unwrap()),
            ["codex"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--claude", "--codex"], true).unwrap()),
            ["claude-code", "codex"]
        );
    }

    /// V-IT-11 (codex-support G-3 / R-007): non-TTY without flags errors.
    /// Resolution must not silently install both platforms.
    #[test]
    fn cli_resolve_platforms_no_flags_non_tty_errors() {
        let err = resolve(&["init"], false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--claude"), "{msg}");
        assert!(msg.contains("--codex"), "{msg}");
    }
}
