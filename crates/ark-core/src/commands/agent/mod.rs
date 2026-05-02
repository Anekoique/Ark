//! Internal `ark agent` commands.
//!
//! Invoked by the Ark workflow and slash commands. Not covered by semver;
//! callers are the shipped slash commands and workflow doc, not end users.
//!
//! The namespace packages every mechanical mutation the workflow asks the
//! agent to perform (file creation, TOML edits, template copies, managed-block
//! updates, directory moves) as deterministic subcommands. Agents call these
//! via `ark agent <verb>`; human invocation is possible but discouraged.

/// Feature SPEC extraction and registration commands.
pub mod spec;
/// Task state models and validation helpers.
pub mod state;
/// Task lifecycle command implementations.
pub mod task;
/// Embedded workflow artifact template helper.
pub mod template;
/// Per-developer workspace journals and identity machinery.
pub mod workspace;

pub use state::{Phase, Status, TaskToml, Tier};
