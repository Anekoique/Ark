//! Embedded template trees.
//!
//! Templates are compiled into the binary via `include_dir!`. Two trees ship:
//!
//! - [`ARK_TEMPLATES`] — extracted into the host project's `.ark/` directory
//! - [`CLAUDE_TEMPLATES`] — extracted into the host project's `.claude/` directory

use include_dir::{Dir, include_dir};

pub static ARK_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/ark");
pub static CLAUDE_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude");

/// A file to be extracted from a template tree, with its destination path.
pub struct Extracted<'a> {
    pub relative_path: &'a std::path::Path,
    pub contents: &'a [u8],
}

/// Walk every file in `dir`, yielding each as an [`Extracted`] entry.
pub fn walk<'a>(dir: &'a Dir<'a>) -> impl Iterator<Item = Extracted<'a>> + 'a {
    let mut stack = vec![dir];
    let mut files = Vec::new();
    while let Some(current) = stack.pop() {
        files.extend(current.files());
        stack.extend(current.dirs());
    }
    files.into_iter().map(|f| Extracted {
        relative_path: f.path(),
        contents: f.contents(),
    })
}
