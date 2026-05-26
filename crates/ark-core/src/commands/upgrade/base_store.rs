//! Sidecar store of last-Ark-written bytes for `[upgrade] merged` paths.
//!
//! The manifest records only a content hash, never the bytes, so a future
//! `ark upgrade` has no diff3 base. This store keeps the exact bytes Ark last
//! wrote for each `merged`-eligible path, mirrored under
//! [`Layout::upgrade_base_dir`]. Gitignored and skipped by `unload` — purely
//! local state, like `.state.toml`. Recorded only for `merged` paths so its
//! size is bounded by what the user opted into.

use std::path::{Path, PathBuf};

use crate::{error::Result, io::PathExt, layout::Layout};

/// Read/write access to the merge-base sidecar (`.ark/.upgrade-base/`).
pub struct UpgradeBaseStore<'a> {
    layout: &'a Layout,
}

impl<'a> UpgradeBaseStore<'a> {
    /// Creates a store rooted at `layout`'s upgrade-base dir.
    pub fn new(layout: &'a Layout) -> Self {
        Self { layout }
    }

    /// Returns the sidecar path that mirrors `relative`.
    ///
    /// `relative` is a project-relative path already validated safe by
    /// [`super::strategy::UpgradeConfig::validate`]; mirroring it under the
    /// base dir cannot escape the root.
    fn sidecar_path(&self, relative: &Path) -> PathBuf {
        self.layout.upgrade_base_dir().join(relative)
    }

    /// Records `bytes` as the base for `relative`, replacing any prior base.
    pub fn record(&self, relative: &Path, bytes: &[u8]) -> Result<()> {
        self.sidecar_path(relative).write_bytes(bytes)
    }

    /// Returns the recorded base bytes for `relative`, or `None` if absent.
    pub fn base_for(&self, relative: &Path) -> Result<Option<Vec<u8>>> {
        self.sidecar_path(relative).read_optional()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V-UT-7: record / read round-trip; `base_for` is `None` when absent.
    #[test]
    fn record_read_round_trip_and_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let store = UpgradeBaseStore::new(&layout);
        let rel = Path::new(".ark/workflow.md");

        assert!(store.base_for(rel).unwrap().is_none());
        store.record(rel, b"original bytes").unwrap();
        assert_eq!(
            store.base_for(rel).unwrap().as_deref(),
            Some(&b"original bytes"[..])
        );

        // Recording again replaces the prior base.
        store.record(rel, b"newer bytes").unwrap();
        assert_eq!(
            store.base_for(rel).unwrap().as_deref(),
            Some(&b"newer bytes"[..])
        );
    }

    /// Bases for nested paths land under the mirrored sidecar tree.
    #[test]
    fn nested_path_mirrors_under_base_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let store = UpgradeBaseStore::new(&layout);
        let rel = Path::new(".claude/commands/ark/quick.md");
        store.record(rel, b"x").unwrap();
        assert!(
            layout
                .upgrade_base_dir()
                .join(".claude/commands/ark/quick.md")
                .exists()
        );
    }
}
