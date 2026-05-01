//! `ark agent workspace init --name <x>` — bootstrap developer identity.

use std::{fmt, path::PathBuf};

use chrono::Utc;

use super::{
    identity::{read_developer_name, validate_developer_name, write_developer_file},
    index::seed_index,
    journal::seed_journal,
};
use crate::{
    error::{Error, Result},
    io::PathExt,
    layout::Layout,
};

/// Options for initializing workspace identity.
#[derive(Debug, Clone)]
pub struct WorkspaceInitOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Developer identity name.
    pub name: String,
}

/// Summary of workspace identity initialization.
#[derive(Debug, Clone)]
pub struct WorkspaceInitSummary {
    /// Developer identity name.
    pub name: String,
    /// Developer workspace directory.
    pub dev_dir: PathBuf,
    /// `false` when re-init with the same name was a no-op.
    pub created: bool,
}

impl fmt::Display for WorkspaceInitSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.created {
            write!(
                f,
                "developer `{}` initialized at {}",
                self.name,
                self.dev_dir.display()
            )
        } else {
            write!(
                f,
                "developer `{}` already initialized at {}",
                self.name,
                self.dev_dir.display()
            )
        }
    }
}

/// Initializes per-developer workspace state.
pub fn workspace_init(opts: WorkspaceInitOptions) -> Result<WorkspaceInitSummary> {
    validate_developer_name(&opts.name)?;

    let layout = Layout::new(&opts.project_root);

    // Idempotent re-init only when the existing identity matches; otherwise
    // refuse so we do not accidentally rename or overwrite.
    let existing = read_developer_name(&layout)?;
    let already = match &existing {
        Some(name) if name == &opts.name => true,
        Some(name) => {
            return Err(Error::DeveloperAlreadyInitialized { name: name.clone() });
        }
        None => false,
    };

    let now = Utc::now();
    let mut created = !already;
    if !already {
        write_developer_file(&layout, &opts.name, now)?;
    }

    let dev_dir = layout.workspace_developer_dir(&opts.name);
    if !dev_dir.exists() {
        created = true;
    }
    dev_dir.ensure_dir()?;

    let index_path = layout.workspace_index(&opts.name);
    if !index_path.exists() {
        seed_index(&index_path, &opts.name)?;
        created = true;
    }

    let journal_path = layout.workspace_journal(&opts.name, 1);
    if !journal_path.exists() {
        seed_journal(&journal_path, &opts.name, 1, now.date_naive())?;
        created = true;
    }

    Ok(WorkspaceInitSummary {
        name: opts.name,
        dev_dir,
        created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that init creates the workspace identity files.
    #[test]
    fn workspace_init_creates_files() {
        let tmp = tempfile::tempdir().unwrap();
        let s = workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        assert!(s.created);
        assert_eq!(s.name, "alice");

        let layout = Layout::new(tmp.path());
        // Identity now lives in `.state.toml`'s `[identity]` section.
        let state =
            crate::state::load_state(&layout, &crate::session::ppid::RealPpid::new()).unwrap();
        assert_eq!(
            state.identity.as_ref().map(|i| i.name.as_str()),
            Some("alice")
        );
        assert!(layout.workspace_index("alice").exists());
        assert!(layout.workspace_journal("alice", 1).exists());
        let idx = layout.workspace_index("alice").read_text().unwrap();
        assert!(idx.contains("# Workspace Index — alice"));
        assert!(idx.contains("ARK:WORKSPACE_STATUS:START"));
        assert!(idx.contains("ARK:WORKSPACE_SESSIONS:START"));
        let journal = layout.workspace_journal("alice", 1).read_text().unwrap();
        assert!(journal.contains("# Journal — alice (Part 1)"));
    }

    /// Verifies that re-initializing with the same name is idempotent.
    #[test]
    fn workspace_init_idempotent_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let _ = workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        let s2 = workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        assert!(!s2.created, "second invocation must be no-op");
    }

    /// Verifies that initializing with a different name yields `DeveloperAlreadyInitialized`.
    #[test]
    fn workspace_init_rejects_different_name() {
        let tmp = tempfile::tempdir().unwrap();
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "alice".into(),
        })
        .unwrap();
        let err = workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "bob".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::DeveloperAlreadyInitialized { .. }));
    }

    #[test]
    fn workspace_init_validates_name() {
        let tmp = tempfile::tempdir().unwrap();
        let err = workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: "1bad".into(),
        })
        .unwrap_err();
        assert!(matches!(err, Error::InvalidDeveloperName { .. }));
    }
}
