//! User-facing command implementations.

/// Hidden `ark agent` workflow command implementations.
pub mod agent;
/// `ark context` state snapshot command implementation.
pub mod context;
/// `ark init` command implementation.
pub mod init;
/// `ark load` command implementation.
pub mod load;
/// `ark remove` command implementation.
pub mod remove;
/// `ark unload` command implementation.
pub mod unload;
/// `ark upgrade` command implementation.
pub mod upgrade;

#[cfg(test)]
pub(crate) mod tests_common;

pub use context::{
    ArchiveState, ArchivedTask, ArtifactKind, ArtifactSummary, Context, ContextOptions,
    ContextSummary, CurrentTask, Format, GitCommit, GitState, PhaseFilter, ProjectedContext,
    SCHEMA_VERSION, Scope, ScopeTag, SpecRow, SpecsState, TaskSummary, TasksState, context,
};
pub use init::{InitOptions, InitSummary, init};
pub use load::{LoadOptions, LoadSummary, load};
pub use remove::{RemoveOptions, RemoveSummary, remove};
pub use unload::{UnloadOptions, UnloadSummary, unload};
pub use upgrade::{
    ConflictChoice, ConflictPolicy, Prompter, UpgradeOptions, UpgradeSummary, upgrade,
};
