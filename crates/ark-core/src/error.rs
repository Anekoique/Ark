use std::{io, path::PathBuf};

use thiserror::Error;

use crate::commands::agent::state::{Phase, Tier};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("manifest corrupt at {path}: {source}")]
    ManifestCorrupt {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("snapshot corrupt: {reason}")]
    SnapshotCorrupt { reason: String },

    #[error("ark is already loaded at {path}; pass --force to replace it")]
    AlreadyLoaded { path: PathBuf },

    #[error("no ark installation found at {path}")]
    NotLoaded { path: PathBuf },

    #[error("refusing unsafe snapshot path {path:?}: {reason}")]
    UnsafeSnapshotPath { path: PathBuf, reason: &'static str },

    #[error("illegal phase transition under tier {tier:?}: {from:?} -> {to:?}")]
    IllegalPhaseTransition { tier: Tier, from: Phase, to: Phase },

    #[error("wrong tier: expected {expected:?}, got {actual:?}")]
    WrongTier { expected: Tier, actual: Tier },

    #[error("task not found: {slug}")]
    TaskNotFound { slug: String },

    #[error("task already exists: {slug}")]
    TaskAlreadyExists { slug: String },

    #[error(
        "no active task set: {path} is missing; pass --slug <s> or run `ark agent task new` first"
    )]
    NoCurrentTask { path: PathBuf },

    #[error("unknown template: {name}")]
    UnknownTemplate { name: String },

    #[error("PLAN at {plan_path} has no `## Spec` section")]
    SpecSectionMissing { plan_path: PathBuf },

    #[error("no `NN_PLAN.md` found in {task_dir}")]
    NoPlanFound { task_dir: PathBuf },

    #[error("task.toml corrupt at {path}: {source}")]
    TaskTomlCorrupt {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid spec field `{field}`: {reason}")]
    InvalidSpecField { field: String, reason: &'static str },

    #[error("invalid task field `{field}`: {reason}")]
    InvalidTaskField { field: String, reason: &'static str },

    #[error("managed block corrupt in {path}: marker `{marker}` has START without END")]
    ManagedBlockCorrupt { path: PathBuf, marker: String },

    #[error(
        "refusing to downgrade: project is at {project_version}, CLI is {cli_version}; pass \
         --allow-downgrade to proceed"
    )]
    DowngradeRefused {
        project_version: String,
        cli_version: String,
    },

    #[error("unsafe path in installation manifest {path:?}: {reason}")]
    UnsafeManifestPath { path: PathBuf, reason: &'static str },

    #[error("failed to spawn git: {source}")]
    GitSpawn {
        #[source]
        source: io::Error,
    },

    // worktree-support feature errors --------------------------------------
    #[error("worktree directory already exists at {path:?}")]
    WorktreeDirExists { path: PathBuf },

    #[error("no worktree found for slug `{slug}`")]
    WorktreeNotFound { slug: String },

    #[error("worktree at {path:?} has uncommitted changes; pass --force to override")]
    WorktreeDirty { path: PathBuf },

    #[error("branch `{branch}` is already checked out at {where_at:?}")]
    BranchInUse { branch: String, where_at: PathBuf },

    #[error("invalid branch name `{branch}`: {reason}")]
    InvalidBranchName { branch: String, reason: String },

    #[error("invalid branch type `{value}`; expected one of feat, fix, refactor, chore, ci, docs")]
    InvalidBranchType { value: String },

    #[error("config.toml corrupt at {path:?}: {source}")]
    WorktreeConfigCorrupt {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("post_create hook `{command}` failed with exit code {exit_code}")]
    PostCreateHookFailed { command: String, exit_code: i32 },

    #[error("config.toml [worktree] `copy` source missing: {path:?}")]
    WorktreeCopySourceMissing { path: PathBuf },

    #[error(
        "task `{slug}` already exists on the parent at {path:?}; archive it or run without \
         --worktree"
    )]
    TaskExistsOnParent { slug: String, path: PathBuf },

    #[error(
        "`task new --worktree` cannot be invoked from inside an existing worktree (root is \
         {current_root:?})"
    )]
    NestedWorktreeForbidden { current_root: PathBuf },

    #[error("invalid config.toml field `{field}`: {reason}")]
    InvalidConfigField {
        field: &'static str,
        reason: &'static str,
    },

    // workspace feature errors --------------------------------------------
    #[error(
        "developer not initialized: {path:?} is missing; run `ark agent workspace init --name \
         <x>` to create it"
    )]
    DeveloperNotInitialized { path: PathBuf },

    #[error("developer already initialized as `{name}`; remove .ark/.developer to re-init")]
    DeveloperAlreadyInitialized { name: String },

    #[error("config.toml corrupt at {path:?}: {source}")]
    WorkspaceConfigCorrupt {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("journal rotation limit hit for `{dev}`: max {max} journals")]
    JournalRotationLimit { dev: String, max: u32 },

    #[error("invalid developer name `{name}`: {reason}")]
    InvalidDeveloperName { name: String, reason: &'static str },
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
