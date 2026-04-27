//! Embedded template trees.
//!
//! Templates are compiled into the binary via `include_dir!`. Three trees ship:
//!
//! - [`ARK_TEMPLATES`] — extracted into the host project's `.ark/` directory
//! - [`CLAUDE_TEMPLATES`] — extracted into the host project's `.claude/` directory
//! - [`CODEX_TEMPLATES`] — extracted into the host project's `.codex/skills/`
//!   directory. Only the `skills/` subtree is hash-tracked; `.codex/hooks.json`
//!   is owned by [`crate::io::update_hook_file`] (driven by
//!   `CODEX_PLATFORM.hook_file`), and `.codex/config.toml` ships via the
//!   whole-file [`CODEX_CONFIG_TOML`] constant (re-applied unconditionally on
//!   every `init` / `upgrade` per codex-support C-11; not hash-tracked).

use include_dir::{Dir, include_dir};

pub static ARK_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/ark");
pub static CLAUDE_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude");
pub static CODEX_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/skills");

/// Whole-file body for `.codex/config.toml`. Re-applied unconditionally on
/// every `init`/`upgrade` per codex-support C-11. Not hash-tracked. The
/// matching `.codex/hooks.json` lifecycle is owned by `update_hook_file`
/// (surgical SessionStart entry; sibling user hooks preserved) — no
/// whole-file rewrite needed.
pub const CODEX_CONFIG_TOML: &str = include_str!("../../../templates/codex/config.toml");

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

#[cfg(test)]
mod tests {
    use super::*;

    /// V-IT-9 (codex-support G-12, C-14): every Claude slash command under
    /// `templates/claude/commands/ark/<name>.md` has a matching Codex skill
    /// at `templates/codex/skills/ark-<name>/SKILL.md`. Existence-only —
    /// content parity is not asserted because Codex skills carry different
    /// frontmatter and rewrite slash-specific tokens.
    #[test]
    fn every_claude_command_has_a_codex_skill_sibling() {
        let claude_commands = CLAUDE_TEMPLATES
            .get_dir("commands/ark")
            .expect("templates/claude/commands/ark exists");
        for file in claude_commands.files() {
            let name = file
                .path()
                .file_stem()
                .expect("claude command has a stem")
                .to_str()
                .expect("ascii name");
            let skill_path = format!("ark-{name}/SKILL.md");
            assert!(
                CODEX_TEMPLATES.get_file(&skill_path).is_some(),
                "missing Codex skill sibling for claude command `{name}`: expected \
                 templates/codex/skills/{skill_path}",
            );
        }
    }

    /// V-E-2 (codex-support C-7): Codex skill bodies open with their *own*
    /// YAML frontmatter (`name`, `description`) rather than Claude's
    /// (`description`, `argument-hint`). A copy-pasted Claude header would
    /// fail this assertion; the `---\n` delimiter itself is required.
    #[test]
    fn codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter() {
        let skills_root = CODEX_TEMPLATES.dirs();
        let mut count = 0;
        for skill_dir in skills_root {
            let Some(file) = skill_dir.get_file(format!(
                "{}/SKILL.md",
                skill_dir.path().file_name().unwrap().to_str().unwrap()
            )) else {
                continue;
            };
            count += 1;
            let body = std::str::from_utf8(file.contents()).expect("utf8 skill body");
            assert!(
                body.starts_with("---\nname: ark-"),
                "skill `{}` must start with Codex `name:` frontmatter (not Claude's \
                 `description:`/`argument-hint:`)",
                skill_dir.path().display(),
            );
        }
        assert!(count >= 3, "expected at least 3 Codex skills");
    }
}
