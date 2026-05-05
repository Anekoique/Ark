//! `ark` — CLI entry.
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
    ArchiveOptions, ConflictChoice, ConflictPolicy, ContextFormat, ContextOptions, ContextScope,
    Identity, InitOptions, Layout, LoadOptions, Manifest, PLATFORMS, PhaseFilter, Platform,
    Prompter, RemoveOptions, UnloadOptions, UpgradeOptions, WriteMode, ark_archive, context,
    identity_write, init, load, remove, unload, upgrade,
};
use clap::{Parser, Subcommand};

mod agent_cli;
use agent_cli::AgentArgs;

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
    /// Bulk-archive every committed task into its `committed_at` month bucket.
    ///
    /// Manager-only operation. Run after a release cut or whenever you want
    /// to consolidate completed work into the YYYY-MM archive directories.
    Archive(ArchiveCliArgs),
    /// Internal commands invoked by the Ark workflow and slash commands.
    /// Not covered by semver — prefer the slash commands over calling these directly.
    #[command(hide = true)]
    Agent(AgentArgs),
}

#[derive(clap::Args)]
struct ArchiveCliArgs {
    #[command(flatten)]
    target: TargetArgs,
    /// Restrict to tasks whose `committed_at` falls in the given `YYYY-MM`.
    #[arg(long)]
    month: Option<String>,
    /// List candidates without performing any move.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
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
    /// Install OpenCode integration (default: prompt on TTY).
    #[arg(long)]
    opencode: bool,
    /// Skip Claude Code integration.
    #[arg(long = "no-claude")]
    no_claude: bool,
    /// Skip Codex CLI integration.
    #[arg(long = "no-codex")]
    no_codex: bool,
    /// Skip OpenCode integration.
    #[arg(long = "no-opencode")]
    no_opencode: bool,

    /// Set the developer name for workspace journals.
    ///
    /// Writes `.ark/.developer` (gitignored). Mutually exclusive with
    /// `--no-developer`.
    #[arg(long, conflicts_with = "no_developer")]
    developer: Option<String>,

    /// Skip developer identity setup; bypasses the interactive prompt.
    ///
    /// Workspace operations will fail with `MissingIdentity` until the user
    /// runs `ark init --developer <name>` later or sets `[workspace]
    /// developer` in `.ark/config.toml`.
    #[arg(long = "no-developer")]
    no_developer: bool,
}

/// Stores positive and negative CLI flag state for one platform.
///
/// Pure data, easy to construct in tests.
#[derive(Debug, Default, Clone, Copy)]
struct PlatformFlag {
    on: bool,
    off: bool,
}

impl InitArgs {
    /// Maps each platform's `cli_flag` to the parsed `PlatformFlag` state.
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
                    "opencode" => PlatformFlag {
                        on: self.opencode,
                        off: self.no_opencode,
                    },
                    _ => PlatformFlag::default(),
                };
                (p, flag)
            })
            .collect()
    }

    /// Resolves `Vec<&'static Platform>` from CLI flags + manifest + TTY state.
    fn resolve_platforms(&self, project_root: &Path) -> anyhow::Result<Vec<&'static Platform>> {
        let installed = installed_platforms(project_root);
        if let Some(set) = installed.as_deref()
            && !self.flags().iter().any(|(_, f)| f.on || f.off)
            && !set.is_empty()
        {
            let ids: Vec<&str> = set.iter().map(|p| p.id).collect();
            eprintln!("note: detected installed platforms ({}); use --<platform> / --no-<platform> to override",
                ids.join(", "));
        }
        let resolved = resolve_platforms_pure(
            &self.flags(),
            installed.as_deref(),
            std::io::stdin().is_terminal(),
            || interactive_select_platforms(),
        )?;
        if resolved.is_empty() {
            anyhow::bail!("init requires at least one platform");
        }
        Ok(resolved)
    }
}

/// Returns the set of platforms whose `dest_dir` appears in the on-disk
/// `.ark/.installed.json`, or `None` when the manifest is missing.
fn installed_platforms(project_root: &Path) -> Option<Vec<&'static Platform>> {
    let manifest = Manifest::read(project_root).ok().flatten()?;
    let detected: Vec<&'static Platform> = PLATFORMS
        .iter()
        .copied()
        .filter(|p| {
            let dest = Path::new(p.dest_dir);
            manifest.files.iter().any(|f| f.starts_with(dest))
        })
        .collect();
    Some(detected)
}

/// Resolves platform flags without performing I/O.
///
/// The caller supplies `is_tty` and a closure that runs the interactive
/// prompt. `installed` is the set derived from `.ark/.installed.json`
/// (`None` on fresh installs).
///
/// - Positive flag (`--<flag>`) narrows to that subset.
/// - Negative flag (`--no-<flag>`) excludes.
/// - Both unset, manifest exists with non-empty platforms: keep the
///   installed set (re-init preserves the current install).
/// - Both unset, no manifest, TTY: run the interactive prompt.
/// - Both unset, no manifest, non-TTY: error — no silent default.
fn resolve_platforms_pure(
    flags: &[(&'static Platform, PlatformFlag)],
    installed: Option<&[&'static Platform]>,
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
    if let Some(set) = installed
        && !set.is_empty()
    {
        return Ok(set.to_vec());
    }
    if is_tty {
        return interactive();
    }
    anyhow::bail!(
        "init requires at least one of --claude, --codex, or --opencode when stdin is not a TTY \
         (use --no-claude / --no-codex / --no-opencode to opt out per platform)"
    );
}

/// Prompts for platform selection on stdin.
///
/// Each platform is offered with a default of "yes". User types `y`/`n`
/// or just enter for default.
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
    Record,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum PhaseArg {
    Design,
    Plan,
    Review,
    Execute,
    Verify,
    Commit,
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
            PhaseArg::Commit => PhaseFilter::Commit,
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
            (ScopeArg::Record, None) => Ok(ContextScope::Record),
            (ScopeArg::Record, Some(_)) => {
                Err("`--for <PHASE>` is only valid with `--scope=phase`".to_string())
            }
        }
    }
}

/// Shared `-C DIR` flag used by every subcommand.
#[derive(clap::Args, Clone)]
pub(crate) struct TargetArgs {
    /// Target directory (defaults to the current working directory).
    #[arg(short = 'C', long, value_name = "DIR", global = false)]
    dir: Option<PathBuf>,
}

impl TargetArgs {
    /// Resolves to the explicit target without ancestor discovery.
    ///
    /// Used by commands whose job is to scaffold or operate on a specific
    /// target directory (`init`, `load --force`).
    pub(crate) fn resolve(self) -> PathBuf {
        let raw = self
            .dir
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        absolutize(&raw)
    }

    /// Resolves and walks ancestors looking for an Ark project root.
    /// If `--dir` was given, it wins (no walk-up). Used by commands that
    /// require an existing `.ark/`.
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

/// Resolves `path` to an absolute path.
///
/// Relative paths are joined against the current working directory. Falls
/// back to the path as-given if cwd lookup fails.
/// Resolves the developer identity and writes `.ark/.developer` if set.
///
/// Precedence:
/// 1. `--developer <name>` flag → write the file.
/// 2. `--no-developer` → skip entirely (no prompt, no file write).
/// 3. Existing `.ark/.developer` (e.g. re-running `ark init`) → leave as-is.
/// 4. Interactive prompt on a TTY → write what the user enters.
/// 5. Non-TTY without flags → skip silently (workspace ops fail later with
///    a clear `MissingIdentity` until the user sets one).
fn resolve_and_persist_identity(
    project_root: &Path,
    explicit: Option<&str>,
    no_developer: bool,
) -> anyhow::Result<()> {
    if no_developer {
        return Ok(());
    }
    if let Some(name) = explicit {
        let identity = Identity::new(name)?;
        identity_write(project_root, &identity)?;
        return Ok(());
    }
    // Existing identity wins; re-running `ark init` should not re-prompt.
    if Layout::new(project_root).developer_file().exists() {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        return Ok(());
    }
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut writer = std::io::stderr();
    match ark_core::identity_prompt(&mut reader, &mut writer, 3) {
        Ok(identity) => {
            identity_write(project_root, &identity)?;
            Ok(())
        }
        Err(_) => {
            // Don't fail init for an aborted prompt; user can run again.
            Ok(())
        }
    }
}

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
                // `init` creates `.ark/`; no walk-up.
                let root = a.target.clone().resolve();
                let platforms = a.resolve_platforms(&root)?;
                let mode = if a.force {
                    WriteMode::Force
                } else {
                    WriteMode::Skip
                };
                announce("initializing ark in", &root);
                let opts = InitOptions::new(root.clone())
                    .with_mode(mode)
                    .with_platforms(platforms);
                render(init(opts)?);

                // Developer identity bootstrap (Phase 1 of workspace).
                resolve_and_persist_identity(&root, a.developer.as_deref(), a.no_developer)?;
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
            Self::Archive(a) => {
                let summary = ark_archive(ArchiveOptions {
                    project_root: a.target.resolve(),
                    month: a.month,
                    dry_run: a.dry_run,
                })?;
                let any_fail = !summary.failures.is_empty();
                render(summary);
                if any_fail {
                    std::process::exit(1);
                }
            }
            Self::Agent(a) => a.dispatch()?,
        }
        Ok(())
    }
}

/// Reads one stdin line per upgrade conflict.
///
/// On non-TTY stdin, short-circuits to Skip. The one-shot "not a terminal"
/// note is emitted by the `Upgrade` dispatch arm, not here.
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

fn announce(verb: &str, root: &Path) {
    println!("{verb} {}", root.display());
}

pub(crate) fn render<S: Display>(summary: S) {
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

    /// Drives `resolve_platforms_pure` with an explicit `is_tty` value.
    ///
    /// Panics if the interactive branch is reached. `installed = None`
    /// matches a fresh install (no manifest yet).
    fn resolve(argv: &[&str], is_tty: bool) -> anyhow::Result<Vec<&'static Platform>> {
        let args = parse_init(argv);
        resolve_platforms_pure(&args.flags(), None, is_tty, || {
            unreachable!("test should not reach the interactive branch")
        })
    }

    /// Drives `resolve_platforms_pure` with an explicit `installed` set.
    ///
    /// Panics if the interactive branch is reached.
    fn resolve_with_installed(
        argv: &[&str],
        installed: &[&'static Platform],
        is_tty: bool,
    ) -> anyhow::Result<Vec<&'static Platform>> {
        let args = parse_init(argv);
        resolve_platforms_pure(&args.flags(), Some(installed), is_tty, || {
            unreachable!("test should not reach the interactive branch")
        })
    }

    fn ids(ps: &[&'static Platform]) -> Vec<&'static str> {
        ps.iter().map(|p| p.id).collect()
    }

    /// Verifies exclusion-only platform flag resolution.
    ///
    /// `--no-X` narrows by exclusion; combinations exclude correspondingly.
    /// Excluding all yields empty.
    #[test]
    fn cli_resolve_platforms_no_x_excludes() {
        assert_eq!(
            ids(&resolve(&["init", "--no-claude"], true).unwrap()),
            ["codex", "opencode"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--no-codex"], true).unwrap()),
            ["claude-code", "opencode"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--no-opencode"], true).unwrap()),
            ["claude-code", "codex"]
        );
        let neither = resolve(
            &["init", "--no-claude", "--no-codex", "--no-opencode"],
            true,
        )
        .unwrap();
        assert!(neither.is_empty(), "{neither:?}");
    }

    /// Positive flags narrow to the named subset.
    #[test]
    fn cli_resolve_platforms_positive_flags_narrow() {
        assert_eq!(
            ids(&resolve(&["init", "--codex"], true).unwrap()),
            ["codex"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--opencode"], true).unwrap()),
            ["opencode"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--claude", "--codex"], true).unwrap()),
            ["claude-code", "codex"]
        );
        assert_eq!(
            ids(&resolve(&["init", "--claude", "--opencode"], true).unwrap()),
            ["claude-code", "opencode"]
        );
    }

    /// Verifies that non-TTY without flags names all three flag pairs.
    #[test]
    fn cli_resolve_platforms_no_flags_non_tty_errors() {
        let err = resolve(&["init"], false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("--claude"), "{msg}");
        assert!(msg.contains("--codex"), "{msg}");
        assert!(msg.contains("--opencode"), "{msg}");
    }

    /// Verifies that `--<flag> --no-<flag>` excludes that platform.
    ///
    /// Negative wins on conflict via the `f.on && !f.off` rule. The
    /// opencode flag pair matches the existing claude / codex semantics.
    #[test]
    fn cli_resolve_platforms_opencode_with_no_opencode_excludes_opencode() {
        // With at least one positive flag, the filter narrows to platforms
        // that are positive AND not negative. `--opencode --no-opencode`
        // sets both, so opencode is excluded; no other positive flag is set,
        // so the result is empty.
        let resolved = resolve(&["init", "--opencode", "--no-opencode"], true).unwrap();
        assert!(
            resolved.is_empty(),
            "opencode excluded by --no-opencode; no other positive flag → empty: {resolved:?}"
        );
        // With another positive flag, that platform is the only survivor.
        assert_eq!(
            ids(&resolve(&["init", "--claude", "--opencode", "--no-opencode"], true).unwrap()),
            ["claude-code"]
        );
        // Symmetry check with the existing claude / codex pairs.
        let claude_self = resolve(&["init", "--claude", "--no-claude"], true).unwrap();
        assert!(claude_self.is_empty(), "{claude_self:?}");
        let codex_self = resolve(&["init", "--codex", "--no-codex"], true).unwrap();
        assert!(codex_self.is_empty(), "{codex_self:?}");
    }

    /// Verifies that `resolve_platforms_pure` invokes the interactive closure.
    ///
    /// The closure is called exactly once when no flags are set, no manifest
    /// is supplied, and `is_tty` is true. Its return value is propagated
    /// unchanged.
    #[test]
    fn cli_resolve_platforms_pure_invokes_interactive_when_tty_and_no_flags() {
        let args = parse_init(&["init"]);
        let mut calls = 0;
        let resolved = resolve_platforms_pure(&args.flags(), None, true, || {
            calls += 1;
            Ok(PLATFORMS.to_vec())
        })
        .unwrap();
        assert_eq!(calls, 1, "interactive closure must be called exactly once");
        assert_eq!(ids(&resolved), ["claude-code", "codex", "opencode"]);
    }

    /// Manifest-derived defaults skip the prompt when no flags are passed.
    #[test]
    fn cli_resolve_platforms_uses_installed_set_when_no_flags() {
        use ark_core::{CLAUDE_PLATFORM, CODEX_PLATFORM};
        let installed = [&CLAUDE_PLATFORM, &CODEX_PLATFORM];
        let resolved = resolve_with_installed(&["init"], &installed, true).unwrap();
        assert_eq!(ids(&resolved), ["claude-code", "codex"]);
    }

    /// Explicit positive flag wins over the manifest.
    #[test]
    fn cli_resolve_platforms_positive_flag_overrides_installed() {
        use ark_core::{CLAUDE_PLATFORM, CODEX_PLATFORM};
        let installed = [&CLAUDE_PLATFORM, &CODEX_PLATFORM];
        let resolved = resolve_with_installed(&["init", "--opencode"], &installed, true).unwrap();
        assert_eq!(ids(&resolved), ["opencode"]);
    }

    /// Negative flag against the installed set still narrows by exclusion.
    #[test]
    fn cli_resolve_platforms_negative_flag_overrides_installed() {
        use ark_core::{CLAUDE_PLATFORM, CODEX_PLATFORM, OPENCODE_PLATFORM};
        let installed = [&CLAUDE_PLATFORM, &CODEX_PLATFORM, &OPENCODE_PLATFORM];
        let resolved =
            resolve_with_installed(&["init", "--no-opencode"], &installed, true).unwrap();
        assert_eq!(ids(&resolved), ["claude-code", "codex"]);
    }

    /// Empty installed set falls through to the prompt branch on TTY.
    #[test]
    fn cli_resolve_platforms_empty_installed_falls_back_to_interactive() {
        let args = parse_init(&["init"]);
        let mut calls = 0;
        let resolved = resolve_platforms_pure(&args.flags(), Some(&[]), true, || {
            calls += 1;
            Ok(vec![&ark_core::CLAUDE_PLATFORM])
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(ids(&resolved), ["claude-code"]);
    }
}
