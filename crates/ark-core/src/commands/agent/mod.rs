//! `ark agent` — internal commands invoked by the Ark workflow and slash
//! commands. Not covered by semver; callers are the shipped slash commands
//! and workflow doc, not end users.
//!
//! The namespace packages every mechanical mutation the workflow asks the
//! agent to perform (file creation, TOML edits, template copies, managed-block
//! updates, directory moves) as deterministic subcommands. Agents call these
//! via `ark agent <verb>`; human invocation is possible but discouraged.

pub mod spec;
pub mod state;
pub mod task;
pub mod template;

pub use state::{Phase, Status, TaskToml, Tier};
