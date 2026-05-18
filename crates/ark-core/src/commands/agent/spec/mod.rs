//! `ark agent spec` — feature-SPEC extraction, registration, and import.

/// Extracts feature SPEC content from deep-tier task plans.
pub mod extract;
/// Imports a feature SPEC authored from an existing codebase.
pub mod import;
/// Registers feature SPEC rows in the feature index.
pub mod register;

pub use extract::{SpecExtractOptions, SpecExtractSummary, spec_extract};
pub use import::{EXTRACTED_SENTINEL, SpecImportOptions, SpecImportSummary, spec_import};
pub use register::{SpecRegisterOptions, SpecRegisterSummary, spec_register};

#[cfg(test)]
mod tests {
    /// Source-scan guard against regressions of the `Layout::specs_feature_dir`
    /// signature refactor.
    ///
    /// `specs_feature_dir` used to take `&str`; the recursive-features refactor
    /// changed it to `(&[&str], impl Into<PathBuf>) -> Result<PathBuf>`. If a
    /// future contributor reintroduces the single-string form (e.g. as a
    /// "convenience" shim), the validation gate that lives in
    /// `Layout::specs_feature_dir(&[&str], …)` is bypassed.
    ///
    /// This test reads every Rust source under `crates/ark-core/src/commands/`
    /// and asserts that no `specs_feature_dir("..."` or `specs_feature_dir(&"...`
    /// invocation appears outside comments / docstrings. Mirrors the
    /// Mirrors the precedent set by the `commands_no_bare_command_new`
    /// source-scan test (which forbids `Command::new` outside `io/git.rs`).
    #[test]
    fn specs_feature_dir_no_single_str_invocations() {
        use std::{
            fs,
            path::{Path, PathBuf},
        };

        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let p = entry.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|e| e == "rs") {
                    out.push(p);
                }
            }
        }

        // CARGO_MANIFEST_DIR is `crates/ark-core/`; walk its `src/commands/`.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
        let mut files = Vec::new();
        walk(&root, &mut files);
        assert!(
            !files.is_empty(),
            "source-scan found no .rs files under {root:?}"
        );

        // Forbidden patterns: `specs_feature_dir("<literal>"` and
        // `specs_feature_dir(&"<literal>"` — these invoke the old single-`&str`
        // signature, which the refactor removed. Allow `&["foo"]` / `&[...]`
        // (the new `&[&str]` form).
        let forbidden_patterns: [&str; 2] = ["specs_feature_dir(\"", "specs_feature_dir(&\""];

        for file in &files {
            let body = fs::read_to_string(file).unwrap();
            for line in body.lines() {
                let trimmed = line.trim_start();
                // Skip comments / docstrings.
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }
                for pat in &forbidden_patterns {
                    assert!(
                        !line.contains(pat),
                        "forbidden pattern `{pat}` found in {file:?} at line:\n  {line}\nUse \
                         `Layout::specs_feature_dir(&[\"seg\"], source_path)?` instead.",
                    );
                }
            }
        }
    }
}
