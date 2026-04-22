//! `ark` — Phase 0 CLI.
//!
//! A thin adapter over [`ark_core`]: parse args, dispatch, print the summary.
//! All logic lives in the library.

use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::ExitCode,
};

use ark_core::{
    InitOptions, LoadOptions, RemoveOptions, UnloadOptions, WriteMode, init, load, remove, unload,
};
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
        }
        Ok(())
    }
}

fn announce(verb: &str, root: &Path) {
    println!("{verb} {}", root.display());
}

fn render<S: Display>(summary: S) {
    println!("{summary}");
}
