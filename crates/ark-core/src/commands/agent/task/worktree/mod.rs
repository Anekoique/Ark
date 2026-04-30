//! `ark agent task worktree {cleanup, list}` manages worktree-backed tasks.
//!
//! Worktree *creation* lives in [`crate::commands::agent::task::new`] under the `--worktree`
//! flag; that is the sole creation entry point. This module owns the
//! `cleanup` and `list` subcommands plus shared helpers.

/// Cleans up task-bound git worktrees.
pub mod cleanup;
/// Loads worktree configuration from Ark config.
pub mod config;
/// Discovers git worktrees and their task bindings.
pub mod discovery;
/// Lists task-bound worktrees.
pub mod list;

pub use cleanup::{WorktreeCleanupOptions, WorktreeCleanupSummary, worktree_cleanup};
pub use config::WorktreeConfig;
pub use list::{WorktreeListOptions, WorktreeListSummary, WorktreeRow, worktree_list};

use crate::error::{Error, Result};

/// Branch types accepted by `--branch-type`.
pub const BRANCH_TYPES: &[&str] = &["feat", "fix", "refactor", "chore", "ci", "docs"];

/// Rejects `value` if it is not in [`BRANCH_TYPES`].
pub(crate) fn validate_branch_type(value: &str) -> Result<()> {
    if BRANCH_TYPES.contains(&value) {
        Ok(())
    } else {
        Err(Error::InvalidBranchType {
            value: value.to_string(),
        })
    }
}

/// Resolves a branch name from worktree options, config, and slug.
///
/// Priority order: `--branch` > `<--branch-type>/<slug>` > `<cfg.branch_prefix>/<slug>`.
pub(crate) fn resolve_branch(
    branch_override: Option<&str>,
    branch_type: Option<&str>,
    cfg_prefix: &str,
    slug: &str,
) -> Result<String> {
    if let Some(b) = branch_override {
        return Ok(b.to_string());
    }
    let prefix = match branch_type {
        Some(t) => {
            validate_branch_type(t)?;
            t
        }
        None => cfg_prefix,
    };
    Ok(format!("{prefix}/{slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies branch precedence.
    #[test]
    fn resolve_branch_precedence() {
        // --branch wins over both
        assert_eq!(
            resolve_branch(Some("custom/x"), Some("fix"), "feat", "foo").unwrap(),
            "custom/x"
        );
        // --branch-type wins over cfg prefix
        assert_eq!(
            resolve_branch(None, Some("fix"), "feat", "foo").unwrap(),
            "fix/foo"
        );
        // cfg prefix is the floor
        assert_eq!(
            resolve_branch(None, None, "feat", "foo").unwrap(),
            "feat/foo"
        );
        assert_eq!(
            resolve_branch(None, None, "chore", "bar").unwrap(),
            "chore/bar"
        );
    }

    /// Verifies that `--branch-type` rejects unknown values.
    #[test]
    fn resolve_branch_rejects_unknown_branch_type() {
        let err = resolve_branch(None, Some("oops"), "feat", "foo").unwrap_err();
        assert!(matches!(err, crate::error::Error::InvalidBranchType { .. }));
    }

    #[test]
    fn validate_branch_type_accepts_each_known() {
        for t in BRANCH_TYPES {
            assert!(validate_branch_type(t).is_ok(), "{t}");
        }
    }
}
