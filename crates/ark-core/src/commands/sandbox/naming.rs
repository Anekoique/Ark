//! Container, volume, and label names derived from a task's branch.
//!
//! Names are `ark-sandbox-<sanitized-branch>-<hash8>` where `hash8` is the
//! first eight hex of SHA-256 of the *exact* branch string. The sanitized
//! segment keeps names docker-legal and human-readable; the hash makes the
//! derivation collision-free, so two branches that sanitize to the same
//! string (`feat/x` and `feat-x` both → `feat-x`) still get distinct names.

use sha2::{Digest, Sha256};

/// Label key carrying the task slug on every Ark sandbox container.
pub const LABEL_SLUG: &str = "ark.sandbox.slug";
/// Label key carrying the branch on every Ark sandbox container.
pub const LABEL_BRANCH: &str = "ark.sandbox.branch";

const PREFIX: &str = "ark-sandbox";

/// Resolved names for one sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxNames {
    /// Container name: `ark-sandbox-<sanitized-branch>-<hash8>`.
    pub container: String,
    /// Config volume name: `<container>-cfg`.
    pub volume: String,
    /// Task slug (carried for labels and summaries).
    pub slug: String,
    /// Branch verbatim (carried for labels and summaries).
    pub branch: String,
}

/// Derives the container/volume names for `slug` on `branch`.
pub fn derive(slug: &str, branch: &str) -> SandboxNames {
    let container = format!("{PREFIX}-{}-{}", sanitize(branch), hash8(branch));
    let volume = format!("{container}-cfg");
    SandboxNames {
        container,
        volume,
        slug: slug.to_string(),
        branch: branch.to_string(),
    }
}

/// Maps any character outside `[A-Za-z0-9_.-]` to `-`.
fn sanitize(branch: &str) -> String {
    branch
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// First eight hex chars of SHA-256 of the exact branch string.
fn hash8(branch: &str) -> String {
    let digest = Sha256::digest(branch.as_bytes());
    let mut out = String::with_capacity(8);
    for byte in digest.iter().take(4) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names follow the documented shape and the volume suffixes the container.
    #[test]
    fn derive_shape() {
        let n = derive("my-task", "feat/x");
        assert!(n.container.starts_with("ark-sandbox-feat-x-"));
        assert_eq!(n.volume, format!("{}-cfg", n.container));
        assert_eq!(n.slug, "my-task");
        assert_eq!(n.branch, "feat/x");
    }

    /// Derivation is stable across calls.
    #[test]
    fn derive_is_stable() {
        assert_eq!(derive("t", "feat/x"), derive("t", "feat/x"));
    }

    /// Gives branches that sanitize identically distinct names.
    #[test]
    fn collision_resistant() {
        let a = derive("t", "feat/x");
        let b = derive("t", "feat-x");
        assert_ne!(
            a.container, b.container,
            "feat/x and feat-x must not collide"
        );
    }
}
