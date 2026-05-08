//! Seed-time substitution for the `VERIFY.md` template.
//!
//! `task_verify` copies the embedded VERIFY template into the task dir, then
//! invokes [`render_seeded_verify`] to overlay four dynamically-derived
//! checklist sections:
//!
//! - **Project Spec Compliance** — recursive walk of
//!   `.ark/specs/project/INDEX.md`. Renders two subsections: one PENDING per
//!   discovered `INDEX.md` (integrity), and one rolled-up PENDING for leaf
//!   SPEC layout conformance plus a traceability sublist of leaf paths.
//! - **Related Feature Spec Compliance** — one bullet per spec listed in the
//!   PRD's `[**Related Specs**]` block.
//! - **PRD Constraints** — one bullet for the Outcome plus one per Constraint
//!   (the PRD's outcome usually carries multi-bullet observable success
//!   criteria; this section preserves that shape).
//! - **Plan Fidelity** — one bullet per Goal (`G-N`) in the latest PLAN's
//!   `## Spec` section.
//!
//! Each section's marker (`{{PROJECT_SPEC_COMPLIANCE}}`, etc.) is replaced
//! by a list of `- [ ] <item>: PENDING` lines. A missing marker errors with
//! [`Error::TemplateMarkerMissing`].

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    io::PathExt,
};

const MARKER_PROJECT: &str = "{{PROJECT_SPEC_COMPLIANCE}}";
const MARKER_RELATED: &str = "{{RELATED_FEATURE_COMPLIANCE}}";
const MARKER_OUTCOME: &str = "{{PRD_CONSTRAINTS}}";
const MARKER_PLAN: &str = "{{PLAN_FIDELITY}}";

/// Project-level SPEC topology discovered by recursively walking
/// `specs/project/INDEX.md`.
///
/// Paths are stored relative to `specs/project/` (e.g. `INDEX.md`,
/// `coding/INDEX.md`, `rust/STYLE.md`). Order is preserved as a depth-first
/// pre-order traversal of the tree so the rendered VERIFY checklist reads
/// top-down.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectSpecTree {
    /// Every `INDEX.md` reached by the walk, including the root. Each one
    /// becomes an integrity-check bullet in the seeded VERIFY.
    pub indexes: Vec<String>,
    /// Every leaf SPEC reached by the walk (rows whose first cell points at a
    /// non-`INDEX.md` markdown file). Rendered as a single rolled-up bullet
    /// plus a traceability sublist.
    pub leaves: Vec<String>,
}

impl ProjectSpecTree {
    /// `true` when neither index nor leaf rows were discovered (e.g. no INDEX
    /// file at all). The renderer falls back to the `(none registered)`
    /// placeholder.
    fn is_empty(&self) -> bool {
        self.indexes.is_empty() && self.leaves.is_empty()
    }
}

/// Inputs for the VERIFY seed substitution.
#[derive(Debug, Clone, Default)]
pub struct SeedInputs {
    /// Recursive project-SPEC tree rooted at `specs/project/INDEX.md`.
    pub project_specs: ProjectSpecTree,
    /// One label per related feature SPEC named in the PRD.
    pub related_specs: Vec<String>,
    /// Each entry seeds one bullet under PRD Constraints. Typical content:
    /// the Outcome's prose plus any explicit Constraints. Empty list emits
    /// a placeholder bullet so the section is never structurally empty.
    pub prd_constraints: Vec<String>,
    /// One label per Goal in the latest PLAN's `## Spec` section.
    pub plan_goals: Vec<String>,
}

/// Returns the rendered VERIFY body for the given template + seed inputs.
///
/// `template` is the embedded VERIFY template (post-`copy_template`). The
/// caller writes the returned string back to disk.
///
/// # Errors
///
/// Returns [`Error::TemplateMarkerMissing`] when any of the four required
/// markers is absent.
pub fn render_seeded_verify(template: &str, inputs: &SeedInputs, path: &Path) -> Result<String> {
    let mut out = template.to_string();
    substitute_project(&mut out, &inputs.project_specs, path)?;
    substitute_or_error(&mut out, MARKER_RELATED, &inputs.related_specs, path)?;
    substitute_or_error(&mut out, MARKER_OUTCOME, &inputs.prd_constraints, path)?;
    substitute_or_error(&mut out, MARKER_PLAN, &inputs.plan_goals, path)?;
    Ok(out)
}

/// Replaces a marker with a `- [ ] <item>: PENDING` bullet list.
///
/// An empty input list produces a single placeholder bullet so the section
/// retains valid markdown shape.
fn substitute_or_error(
    body: &mut String,
    marker: &'static str,
    items: &[String],
    path: &Path,
) -> Result<()> {
    if !body.contains(marker) {
        return Err(Error::TemplateMarkerMissing {
            marker,
            path: path.to_path_buf(),
        });
    }
    *body = body.replace(marker, &render_bullet_list(items));
    Ok(())
}

/// Replaces `{{PROJECT_SPEC_COMPLIANCE}}` with the two-subsection layout.
fn substitute_project(body: &mut String, tree: &ProjectSpecTree, path: &Path) -> Result<()> {
    if !body.contains(MARKER_PROJECT) {
        return Err(Error::TemplateMarkerMissing {
            marker: MARKER_PROJECT,
            path: path.to_path_buf(),
        });
    }
    *body = body.replace(MARKER_PROJECT, &render_project_block(tree));
    Ok(())
}

/// Renders the bullet list for a marker. Empty input yields a placeholder so
/// the section is never structurally empty (one `(none)` line is friendlier
/// to readers than a blank section).
fn render_bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        return "- (none registered): N/A".into();
    }
    let mut out = String::with_capacity(items.len() * 32);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&format!("- [ ] {item}: PENDING"));
    }
    out
}

/// Renders the `Project Spec Compliance` block: an `### Index integrity`
/// subsection (one PENDING per discovered INDEX) and a `### Leaf SPECs`
/// subsection (one rolled-up PENDING plus a traceability sublist).
fn render_project_block(tree: &ProjectSpecTree) -> String {
    if tree.is_empty() {
        return "- (none registered): N/A".into();
    }
    let mut out = String::new();
    out.push_str("### Index integrity\n\n");
    if tree.indexes.is_empty() {
        out.push_str("- (none discovered): N/A\n");
    } else {
        for index in &tree.indexes {
            let dir = index_parent_display(index);
            out.push_str(&format!(
                "- [ ] `{index}` enumerates all children of `{dir}`: PENDING\n"
            ));
        }
    }
    out.push_str("\n### Leaf SPECs\n\n");
    if tree.leaves.is_empty() {
        out.push_str("- (none discovered): N/A");
    } else {
        out.push_str(
            "- [ ] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PENDING\n",
        );
        for (i, leaf) in tree.leaves.iter().enumerate() {
            out.push_str(&format!("  - `{leaf}`"));
            if i + 1 < tree.leaves.len() {
                out.push('\n');
            }
        }
    }
    out
}

/// Display string for an INDEX's parent directory, anchored at
/// `specs/project/`. The root INDEX shows `specs/project/`; nested ones show
/// `specs/project/<rel>/`.
fn index_parent_display(index_rel: &str) -> String {
    match index_rel.rsplit_once('/') {
        Some((dir, _)) => format!("specs/project/{dir}/"),
        None => "specs/project/".into(),
    }
}

/// Recursively walks `specs/project/INDEX.md` and returns the discovered
/// [`ProjectSpecTree`].
///
/// A row whose first backtick-wrapped cell ends in `INDEX.md` is treated as a
/// nested-index reference and recursed into; any other markdown path is a
/// leaf. Cycles and self-references are guarded with a visited set keyed on
/// canonicalised absolute paths. Rows pointing at files that don't exist on
/// disk are skipped silently — the parent INDEX's integrity bullet is the
/// natural place to flag such mismatches at review time.
///
/// Files that are missing or have no Index table yield an empty tree — the
/// caller treats that as "no project SPECs to check," which renders the
/// `(none registered)` placeholder.
pub fn read_project_spec_tree(project_index: &Path) -> Result<ProjectSpecTree> {
    if !project_index.is_file() {
        return Ok(ProjectSpecTree::default());
    }
    let root_dir = project_index
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let mut tree = ProjectSpecTree::default();
    let mut visited = HashSet::new();
    walk_index(
        &root_dir,
        project_index,
        "INDEX.md",
        &mut tree,
        &mut visited,
    )?;
    Ok(tree)
}

/// Recursive helper: parses one INDEX, records it, and descends into nested
/// INDEX rows. `index_rel` is the path of this INDEX relative to `root_dir`.
fn walk_index(
    root_dir: &Path,
    index_path: &Path,
    index_rel: &str,
    tree: &mut ProjectSpecTree,
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    let canonical = index_path
        .canonicalize()
        .unwrap_or_else(|_| index_path.to_path_buf());
    if !visited.insert(canonical) {
        return Ok(());
    }
    let Some(text) = index_path.read_text_optional()? else {
        return Ok(());
    };
    tree.indexes.push(index_rel.to_string());
    let dir = index_path.parent().unwrap_or(root_dir);
    for entry in parse_index_rows(&text) {
        // `entry` is the row's first-cell path, relative to `dir`.
        let child_abs = dir.join(&entry);
        if !child_abs.exists() {
            continue;
        }
        let child_rel = pathbuf_to_rel_string(root_dir, &child_abs);
        if entry.ends_with("INDEX.md") {
            walk_index(root_dir, &child_abs, &child_rel, tree, visited)?;
        } else {
            tree.leaves.push(child_rel);
        }
    }
    Ok(())
}

/// Returns the row entries (first-cell backtick-wrapped paths) found in the
/// `## Index` table of an INDEX.md body. Header, separator, and
/// `{...}`-wrapped placeholder rows are skipped.
fn parse_index_rows(text: &str) -> Vec<String> {
    let mut in_index = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Index") {
            in_index = true;
            continue;
        }
        if in_index && trimmed.starts_with("## ") {
            break;
        }
        if !in_index {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("| ")
            && let Some(start) = rest.find('`')
            && let Some(end) = rest[start + 1..].find('`')
        {
            let name = &rest[start + 1..start + 1 + end];
            if !name.is_empty() {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Returns `abs_path` rendered relative to `root_dir`, falling back to a
/// stringified absolute path when the strip fails (defensive — every call
/// site builds `abs_path` by joining onto `root_dir`'s ancestry).
fn pathbuf_to_rel_string(root_dir: &Path, abs_path: &Path) -> String {
    abs_path
        .strip_prefix(root_dir)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| abs_path.to_string_lossy().to_string())
}

/// Parses a PRD's `[**Related Specs**]` bullet list.
///
/// Each bullet has the form ``- `path/to/SPEC.md` — description``; the parser
/// extracts the backtick-wrapped path. Bullets without a backtick segment
/// are skipped.
pub fn read_related_specs(prd_path: &Path) -> Result<Vec<String>> {
    let Some(text) = prd_path.read_text_optional()? else {
        return Ok(Vec::new());
    };
    let mut in_section = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[**Related Specs**]") {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Hit the next bracket-section header.
            break;
        }
        if !in_section {
            continue;
        }
        if let Some(bullet) = trimmed.strip_prefix("- ")
            && let Some(start) = bullet.find('`')
            && let Some(end) = bullet[start + 1..].find('`')
        {
            let label = &bullet[start + 1..start + 1 + end];
            if !label.is_empty() {
                out.push(label.to_string());
            }
        }
    }
    Ok(out)
}

/// Parses a PRD's `[**Outcome**]` block + any `[**Constraints**]` bullets
/// into checklist items.
///
/// Outcome usually carries multi-paragraph or multi-bullet content; we lift
/// the first non-empty paragraph and any subsequent numbered list items as
/// individual checklist entries. Constraints (when present) follow the same
/// bullet-extraction rule as related specs.
pub fn read_prd_constraints(prd_path: &Path) -> Result<Vec<String>> {
    let Some(text) = prd_path.read_text_optional()? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut section: Option<&str> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[**Outcome**]") {
            section = Some("outcome");
            continue;
        }
        if trimmed.starts_with("[**Constraints**]") {
            section = Some("constraints");
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = None;
            continue;
        }
        match section {
            Some("outcome") => {
                if let Some(rest) = numbered_bullet(trimmed) {
                    out.push(short_label(rest));
                }
            }
            Some("constraints") => {
                if let Some(rest) = trimmed.strip_prefix("- ") {
                    out.push(short_label(rest));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Strips a leading `<digit>. ` (markdown numbered-list bullet).
fn numbered_bullet(s: &str) -> Option<&str> {
    let mut chars = s.char_indices();
    let mut digits = 0;
    while let Some((_, c)) = chars.clone().next() {
        if c.is_ascii_digit() {
            chars.next();
            digits += 1;
        } else {
            break;
        }
    }
    if digits == 0 {
        return None;
    }
    let rest = &s[digits..];
    rest.strip_prefix(". ")
}

/// Truncates a checklist label to a readable summary.
///
/// VERIFY items are scanned by humans; the first sentence (or first 80 chars)
/// is enough context, and longer prose belongs in the PRD/PLAN it cites.
/// Trailing `.` on a single-sentence label is stripped so adjacent labels
/// render consistently in the bullet list.
fn short_label(s: &str) -> String {
    let s = s.trim();
    if let Some(idx) = s.find(". ") {
        return s[..idx].to_string();
    }
    let truncated: String = if s.chars().count() <= 80 {
        s.to_string()
    } else {
        s.chars().take(80).collect::<String>() + "…"
    };
    truncated.trim_end_matches('.').to_string()
}

/// Parses the latest `NN_PLAN.md`'s `## Spec` Goals section into bullet labels.
///
/// Goals are bullets of the form `- G-N: <text>` under the `[**Goals**]`
/// header inside the `## Spec` section. The parser stops at the next
/// bracket-section header.
pub fn read_plan_goals(plan_path: &Path) -> Result<Vec<String>> {
    let Some(text) = plan_path.read_text_optional()? else {
        return Ok(Vec::new());
    };
    let mut in_spec = false;
    let mut in_goals = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## Spec") {
            in_spec = true;
            continue;
        }
        if in_spec && trimmed.starts_with("## ") {
            in_spec = false;
            in_goals = false;
            continue;
        }
        if !in_spec {
            continue;
        }
        if trimmed.starts_with("[**Goals**]") {
            in_goals = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_goals = false;
            continue;
        }
        if in_goals && let Some(rest) = trimmed.strip_prefix("- ") {
            // Accept both bare `G-N: …` and bolded `**G-N: …**` bullets so the
            // parser tolerates the two PLAN styles in active use across the
            // codebase. Reject NG- bullets (non-goals).
            let unbolded = rest.strip_prefix("**").unwrap_or(rest);
            if unbolded.starts_with("G-") {
                out.push(short_label(rest));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker_template() -> &'static str {
        "## A\n{{PROJECT_SPEC_COMPLIANCE}}\n## B\n{{RELATED_FEATURE_COMPLIANCE}}\n## \
         C\n{{PRD_CONSTRAINTS}}\n## D\n{{PLAN_FIDELITY}}\n"
    }

    #[test]
    fn render_substitutes_all_four_markers() {
        let inputs = SeedInputs {
            project_specs: ProjectSpecTree {
                indexes: vec!["INDEX.md".into()],
                leaves: vec!["LAYOUT.md".into()],
            },
            related_specs: vec!["features/foo/SPEC.md".into()],
            prd_constraints: vec!["Outcome: ship".into()],
            plan_goals: vec!["G-1: x".into()],
        };
        let out = render_seeded_verify(marker_template(), &inputs, Path::new("VERIFY.md")).unwrap();
        assert!(out.contains("### Index integrity"));
        assert!(
            out.contains("- [ ] `INDEX.md` enumerates all children of `specs/project/`: PENDING")
        );
        assert!(out.contains("### Leaf SPECs"));
        assert!(out.contains(
            "- [ ] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PENDING"
        ));
        assert!(out.contains("  - `LAYOUT.md`"));
        assert!(out.contains("- [ ] features/foo/SPEC.md: PENDING"));
        assert!(out.contains("- [ ] Outcome: ship: PENDING"));
        assert!(out.contains("- [ ] G-1: x: PENDING"));
        assert!(!out.contains("{{"));
    }

    #[test]
    fn render_emits_placeholder_for_empty_input() {
        let out = render_seeded_verify(
            marker_template(),
            &SeedInputs::default(),
            Path::new("VERIFY.md"),
        )
        .unwrap();
        assert!(out.contains("(none registered)"));
    }

    #[test]
    fn render_errors_on_missing_marker() {
        let template = "## A\n## B\n";
        let err = render_seeded_verify(template, &SeedInputs::default(), Path::new("VERIFY.md"))
            .unwrap_err();
        assert!(matches!(err, Error::TemplateMarkerMissing { .. }));
    }

    #[test]
    fn render_project_block_renders_nested_indexes() {
        let tree = ProjectSpecTree {
            indexes: vec!["INDEX.md".into(), "coding/INDEX.md".into()],
            leaves: vec![
                "LAYOUT.md".into(),
                "rust/STYLE.md".into(),
                "coding/testing/SPEC.md".into(),
            ],
        };
        let block = render_project_block(&tree);
        assert!(
            block.contains("- [ ] `INDEX.md` enumerates all children of `specs/project/`: PENDING")
        );
        assert!(block.contains(
            "- [ ] `coding/INDEX.md` enumerates all children of `specs/project/coding/`: PENDING"
        ));
        assert!(block.contains("  - `LAYOUT.md`"));
        assert!(block.contains("  - `rust/STYLE.md`"));
        assert!(block.contains("  - `coding/testing/SPEC.md`"));
    }

    #[test]
    fn read_project_spec_tree_walks_flat_index() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(root.join("rust")).unwrap();
        let index = root.join("INDEX.md");
        index
            .write_bytes(
                b"# Project Specs\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `LAYOUT.md` | foo |\n| `rust/STYLE.md` | bar |\n\n## How to Use\n\n",
            )
            .unwrap();
        // Touch leaves so they exist on disk.
        root.join("LAYOUT.md").write_bytes(b"# layout\n").unwrap();
        root.join("rust/STYLE.md")
            .write_bytes(b"# style\n")
            .unwrap();

        let tree = read_project_spec_tree(&index).unwrap();
        assert_eq!(tree.indexes, vec!["INDEX.md"]);
        assert_eq!(tree.leaves, vec!["LAYOUT.md", "rust/STYLE.md"]);
    }

    #[test]
    fn read_project_spec_tree_recurses_into_nested_index() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(root.join("coding/testing")).unwrap();
        root.join("INDEX.md")
            .write_bytes(
                b"# Project Specs\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `LAYOUT.md` | foo |\n| `coding/INDEX.md` | nested |\n\n## How to Use\n\n",
            )
            .unwrap();
        root.join("LAYOUT.md").write_bytes(b"# l\n").unwrap();
        root.join("coding/INDEX.md")
            .write_bytes(
                b"# Coding Specs\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `testing/SPEC.md` | tests |\n\n",
            )
            .unwrap();
        root.join("coding/testing/SPEC.md")
            .write_bytes(b"# t\n")
            .unwrap();

        let tree = read_project_spec_tree(&root.join("INDEX.md")).unwrap();
        assert_eq!(tree.indexes, vec!["INDEX.md", "coding/INDEX.md"]);
        assert_eq!(tree.leaves, vec!["LAYOUT.md", "coding/testing/SPEC.md"]);
    }

    #[test]
    fn read_project_spec_tree_skips_missing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(&root).unwrap();
        root.join("INDEX.md")
            .write_bytes(
                b"# Project Specs\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `LAYOUT.md` | exists |\n| `rust/STYLE.md` | missing |\n\
                  | `coding/INDEX.md` | also missing |\n\n",
            )
            .unwrap();
        root.join("LAYOUT.md").write_bytes(b"# l\n").unwrap();

        let tree = read_project_spec_tree(&root.join("INDEX.md")).unwrap();
        assert_eq!(tree.indexes, vec!["INDEX.md"]);
        assert_eq!(tree.leaves, vec!["LAYOUT.md"]);
    }

    #[test]
    fn read_project_spec_tree_handles_cycle() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        std::fs::create_dir_all(root.join("a")).unwrap();
        // Root references a/INDEX.md; a/INDEX.md references back to ../INDEX.md.
        root.join("INDEX.md")
            .write_bytes(
                b"# Project Specs\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `a/INDEX.md` | child |\n\n",
            )
            .unwrap();
        root.join("a/INDEX.md")
            .write_bytes(
                b"# A\n\n## Index\n\n| Spec | Scope |\n|------|-------|\n\
                  | `../INDEX.md` | back-edge |\n\n",
            )
            .unwrap();

        let tree = read_project_spec_tree(&root.join("INDEX.md")).unwrap();
        // Both indexes recorded once; the back-edge does not loop.
        assert_eq!(tree.indexes, vec!["INDEX.md", "a/INDEX.md"]);
        assert!(tree.leaves.is_empty());
    }

    #[test]
    fn read_project_spec_tree_returns_empty_when_index_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("INDEX.md"); // never created
        let tree = read_project_spec_tree(&p).unwrap();
        assert!(tree.indexes.is_empty());
        assert!(tree.leaves.is_empty());
    }

    #[test]
    fn read_related_specs_extracts_backtick_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("PRD.md");
        p.write_bytes(
            b"[**What**]\nfoo\n\n[**Related Specs**]\n- `specs/features/a/SPEC.md` \
              \xE2\x80\x94 description\n- `specs/features/b/SPEC.md` \xE2\x80\x94 description\n\n\
              [**Other**]\nirrelevant\n",
        )
        .unwrap();
        let got = read_related_specs(&p).unwrap();
        assert_eq!(
            got,
            vec!["specs/features/a/SPEC.md", "specs/features/b/SPEC.md"]
        );
    }

    #[test]
    fn read_prd_constraints_lifts_outcome_numbered_items() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("PRD.md");
        p.write_bytes(
            b"[**Outcome**]\n\n1. First criterion. Then more text.\n2. Second criterion.\n\n\
              [**Constraints**]\n- Must be fast\n- Must be safe\n\n[**Related Specs**]\nirrelevant\n",
        )
        .unwrap();
        let got = read_prd_constraints(&p).unwrap();
        assert_eq!(
            got,
            vec![
                "First criterion",
                "Second criterion",
                "Must be fast",
                "Must be safe",
            ]
        );
    }

    #[test]
    fn read_plan_goals_extracts_g_bullets() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("00_PLAN.md");
        p.write_bytes(
            b"## Summary\nfoo\n\n## Spec\n\n[**Goals**]\n- G-1: First. Detail.\n- G-2: Second\n- \
              NG-1: not goal\n\n[**Constraints**]\nbar\n\n## Runtime\nirrelevant\n",
        )
        .unwrap();
        let got = read_plan_goals(&p).unwrap();
        assert_eq!(got, vec!["G-1: First", "G-2: Second"]);
    }

    /// Verifies the parser tolerates bolded bullets (`- **G-N: ...**`) and
    /// rejects bolded NG- bullets.
    #[test]
    fn read_plan_goals_accepts_bolded_bullets() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("00_PLAN.md");
        p.write_bytes(
            b"## Spec\n\n[**Goals**]\n- **G-1: First.** Detail.\n- **G-2: Second**\n- **NG-1: \
              skip**\n\n[**Constraints**]\nbar\n",
        )
        .unwrap();
        let got = read_plan_goals(&p).unwrap();
        assert_eq!(got.len(), 2);
        assert!(got[0].starts_with("**G-1"));
        assert!(got[1].starts_with("**G-2"));
        assert!(!got.iter().any(|g| g.contains("NG-")));
    }
}
