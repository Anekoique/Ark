//! Builds a nested [`SpecNode`] tree from a flat list of [`SpecRow`]s.
//!
//! Pure function over its input. Groups rows by [`SpecRow::feature_path`]
//! segments, producing a tree whose leaves carry per-row metadata and
//! whose branches carry only the segment name.

use crate::commands::context::model::{FEATURES_TREE_MAX_DEPTH, SpecNode, SpecRow};

/// Returns the nested [`SpecNode`] tree for `rows`, or [`None`] when
/// `rows` is empty.
///
/// Rows with an empty [`SpecRow::feature_path`] are skipped (project SPEC
/// rows). The build is deterministic: sibling order is by `segment`
/// ascending, regardless of input order. Recursion is bounded by
/// [`FEATURES_TREE_MAX_DEPTH`]; rows deeper than the bound are dropped.
pub fn build_features_tree(rows: &[SpecRow]) -> Option<SpecNode> {
    let mut roots: Vec<SpecNode> = Vec::new();
    for row in rows {
        if row.feature_path.is_empty() || row.feature_path.len() > FEATURES_TREE_MAX_DEPTH {
            continue;
        }
        insert_row(&mut roots, &row.feature_path, row, 0);
    }
    sort_recursive(&mut roots);
    if roots.is_empty() {
        return None;
    }
    if roots.len() == 1 {
        return Some(roots.pop().expect("len checked"));
    }
    Some(SpecNode::Branch {
        segment: String::new(),
        children: roots,
    })
}

/// Inserts a single `row` into the partially-built tree under `nodes`.
///
/// `path[depth..]` is the remaining path to walk. When `depth + 1 ==
/// path.len()`, a [`SpecNode::Leaf`] is appended at this level; otherwise
/// a [`SpecNode::Branch`] (creating one if absent) is recursed into.
fn insert_row(nodes: &mut Vec<SpecNode>, path: &[String], row: &SpecRow, depth: usize) {
    let segment = path[depth].clone();
    let is_leaf = depth + 1 == path.len();
    if is_leaf {
        nodes.push(SpecNode::Leaf {
            segment,
            name: row.name.clone(),
            path: row.path.clone(),
            scope: row.scope.clone(),
            promoted: row.promoted.clone(),
        });
        return;
    }
    if let Some(existing) = nodes.iter_mut().find_map(|n| match n {
        SpecNode::Branch {
            segment: s,
            children,
        } if s == &segment => Some(children),
        _ => None,
    }) {
        insert_row(existing, path, row, depth + 1);
        return;
    }
    let mut children: Vec<SpecNode> = Vec::new();
    insert_row(&mut children, path, row, depth + 1);
    nodes.push(SpecNode::Branch { segment, children });
}

/// Recursively sorts every level by `segment` ascending.
fn sort_recursive(nodes: &mut [SpecNode]) {
    nodes.sort_by(|a, b| segment_of(a).cmp(segment_of(b)));
    for node in nodes {
        if let SpecNode::Branch { children, .. } = node {
            sort_recursive(children);
        }
    }
}

/// Returns the `segment` field for either variant.
fn segment_of(node: &SpecNode) -> &str {
    match node {
        SpecNode::Branch { segment, .. } | SpecNode::Leaf { segment, .. } => segment,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// Builds a `SpecRow` with the documented shape for tree-building.
    fn row(feature_path: &[&str], name: &str) -> SpecRow {
        SpecRow {
            name: name.to_string(),
            path: PathBuf::from(format!(
                ".ark/specs/features/{}/SPEC.md",
                feature_path.join("/")
            )),
            feature_path: feature_path.iter().map(|s| s.to_string()).collect(),
            scope: format!("scope of {name}"),
            promoted: None,
        }
    }

    /// Builds a flat list of single-segment leaves.
    #[test]
    fn flat_input_produces_branch_with_two_leaves() {
        let rows = vec![
            row(&["ark-context"], "ark-context"),
            row(&["worktree"], "worktree"),
        ];
        let tree = build_features_tree(&rows).expect("non-empty input");
        let children = match tree {
            SpecNode::Branch {
                segment, children, ..
            } => {
                assert_eq!(segment, "");
                children
            }
            other => panic!("expected branch wrapper, got {other:?}"),
        };
        assert_eq!(children.len(), 2);
        assert!(matches!(
            &children[0],
            SpecNode::Leaf { segment, .. } if segment == "ark-context"
        ));
        assert!(matches!(
            &children[1],
            SpecNode::Leaf { segment, .. } if segment == "worktree"
        ));
    }

    /// Mixed-depth input yields a single root branch only when the union
    /// of top-level segments has more than one entry; otherwise the root
    /// is collapsed.
    #[test]
    fn mixed_input_groups_by_subtree() {
        let rows = vec![
            row(&["klib"], "klib"),
            row(&["xemu", "csr"], "csr"),
            row(&["xemu", "io", "mmio"], "mmio"),
        ];
        let tree = build_features_tree(&rows).expect("non-empty input");
        let children = match tree {
            SpecNode::Branch { children, .. } => children,
            other => panic!("expected branch root, got {other:?}"),
        };
        assert_eq!(children.len(), 2);

        match &children[0] {
            SpecNode::Leaf { segment, .. } => assert_eq!(segment, "klib"),
            other => panic!("expected klib leaf, got {other:?}"),
        }

        let xemu_children = match &children[1] {
            SpecNode::Branch {
                segment, children, ..
            } => {
                assert_eq!(segment, "xemu");
                children
            }
            other => panic!("expected xemu branch, got {other:?}"),
        };
        assert_eq!(xemu_children.len(), 2);
        match &xemu_children[0] {
            SpecNode::Leaf { segment, .. } => assert_eq!(segment, "csr"),
            other => panic!("expected csr leaf, got {other:?}"),
        }
        let io_children = match &xemu_children[1] {
            SpecNode::Branch {
                segment, children, ..
            } => {
                assert_eq!(segment, "io");
                children
            }
            other => panic!("expected io branch, got {other:?}"),
        };
        assert_eq!(io_children.len(), 1);
        match &io_children[0] {
            SpecNode::Leaf { segment, name, .. } => {
                assert_eq!(segment, "mmio");
                assert_eq!(name, "mmio");
            }
            other => panic!("expected mmio leaf, got {other:?}"),
        }
    }

    /// Empty input returns `None`.
    #[test]
    fn empty_input_returns_none() {
        assert!(build_features_tree(&[]).is_none());
    }

    /// Tree shape is independent of input order (determinism).
    #[test]
    fn input_order_does_not_change_tree_shape() {
        let a = vec![
            row(&["xemu", "csr"], "csr"),
            row(&["klib"], "klib"),
            row(&["xemu", "io", "mmio"], "mmio"),
        ];
        let b = vec![
            row(&["klib"], "klib"),
            row(&["xemu", "io", "mmio"], "mmio"),
            row(&["xemu", "csr"], "csr"),
        ];
        let tree_a = build_features_tree(&a);
        let tree_b = build_features_tree(&b);
        let json_a = serde_json::to_string(&tree_a).unwrap();
        let json_b = serde_json::to_string(&tree_b).unwrap();
        assert_eq!(json_a, json_b);
    }

    /// A single root leaf collapses to the leaf itself (no synthetic
    /// branch wrapper).
    #[test]
    fn single_leaf_is_not_wrapped() {
        let rows = vec![row(&["only"], "only")];
        let tree = build_features_tree(&rows).expect("non-empty input");
        assert!(matches!(tree, SpecNode::Leaf { ref segment, .. } if segment == "only"));
    }

    /// A row deeper than the depth bound is dropped silently.
    #[test]
    fn rows_beyond_max_depth_are_dropped() {
        let deep: Vec<&str> = (0..(FEATURES_TREE_MAX_DEPTH + 1)).map(|_| "seg").collect();
        let rows = vec![row(&deep, "too-deep"), row(&["fine"], "fine")];
        let tree = build_features_tree(&rows).expect("non-empty input");
        // Only the `fine` leaf survives.
        assert!(matches!(tree, SpecNode::Leaf { ref segment, .. } if segment == "fine"));
    }
}
