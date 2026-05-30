//! Concrete [`crate::commands::sandbox::engine::SandboxEngine`] backends.
//!
//! v1 ships only [`docker::DockerEngine`]. A future OS-native (Seatbelt/bwrap)
//! backend would add a sibling module here and a branch in `select_engine`.

/// Docker / OCI backend.
pub mod docker;
