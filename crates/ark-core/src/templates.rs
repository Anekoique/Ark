//! Embedded template trees.
//!
//! Templates are compiled into the binary via `include_dir!`. Four trees ship:
//!
//! - [`crate::templates::ARK_TEMPLATES`] — extracted into the host project's `.ark/` directory
//! - [`crate::templates::CLAUDE_TEMPLATES`] — extracted into the host project's `.claude/` directory
//! - [`crate::templates::CODEX_TEMPLATES`] — extracted into the host project's `.codex/skills/`
//!   directory. Only the `skills/` subtree is hash-tracked; `.codex/hooks.json`
//!   is owned by [`crate::io::update_hook_file`] (driven by
//!   `CODEX_PLATFORM.hook_file`), and `.codex/config.toml` ships via the
//!   whole-file [`crate::templates::CODEX_CONFIG_TOML`] constant (re-applied unconditionally on
//!   every `init` / `upgrade`; not hash-tracked).
//! - [`crate::templates::OPENCODE_TEMPLATES`] — extracted into the host project's
//!   `.opencode/commands/` directory. The `.opencode/plugins/ark-context.ts`
//!   file ships via the whole-file [`crate::templates::OPENCODE_ARK_CONTEXT_TS`] constant
//!   (re-applied unconditionally on every `init` / `load` / `upgrade`; not
//!   hash-tracked, mirroring `CODEX_CONFIG_TOML`).

use include_dir::{Dir, include_dir};

/// Embedded `.ark/` template tree.
pub static ARK_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/ark");
/// Embedded `.claude/` template tree.
pub static CLAUDE_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude");
/// Embedded Codex skill template tree.
pub static CODEX_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/skills");
/// Embedded OpenCode command template tree.
pub static OPENCODE_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/commands");

/// Whole-file body for `.codex/config.toml`.
///
/// Re-applied unconditionally on every `init`/`upgrade`; not hash-tracked.
/// The matching `.codex/hooks.json` lifecycle is owned by
/// [`crate::io::update_hook_file`] (surgical `SessionStart` entry;
/// sibling user hooks preserved) — no whole-file rewrite is needed.
pub const CODEX_CONFIG_TOML: &str = include_str!("../../../templates/codex/config.toml");

/// Whole-file body for `.opencode/plugins/ark-context.ts`.
///
/// Re-applied unconditionally on every `init`/`load`/`upgrade`; not
/// hash-tracked. OpenCode has no native JSON `SessionStart` hook surface;
/// this Bun-loaded TypeScript plugin replaces that role by shelling out
/// to `ark context --scope session --format json` from `chat.message`
/// and prepending the unwrapped context to the first user message via
/// `experimental.chat.messages.transform`.
pub const OPENCODE_ARK_CONTEXT_TS: &str =
    include_str!("../../../templates/opencode/plugins/ark-context.ts");

/// A file to be extracted from a template tree, with its destination path.
pub struct Extracted<'a> {
    /// Path relative to the template tree root.
    pub relative_path: &'a std::path::Path,
    /// File contents embedded in the template tree.
    pub contents: &'a [u8],
}

/// Walks every file in `dir`, yielding each as an [`Extracted`] entry.
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

    /// Verifies that every Claude slash command has a Codex skill sibling.
    ///
    /// Existence-only: content parity is not asserted because Codex skills
    /// carry different frontmatter and rewrite slash-specific tokens.
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

    /// Verifies that each Codex skill opens with Codex frontmatter.
    ///
    /// A copy-pasted Claude header would fail this assertion; the `---\n`
    /// delimiter itself is required.
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

    /// Verifies that every Claude slash command has an OpenCode sibling.
    ///
    /// Existence-only: content parity is policed at code review.
    #[test]
    fn every_claude_command_has_an_opencode_command_sibling() {
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
            let opencode_path = format!("ark/{name}.md");
            assert!(
                OPENCODE_TEMPLATES.get_file(&opencode_path).is_some(),
                "missing OpenCode command sibling for claude command `{name}`: expected \
                 templates/opencode/commands/{opencode_path}",
            );
        }
    }

    /// Verifies OpenCode command frontmatter and heading shape.
    #[test]
    fn opencode_command_bodies_have_opencode_frontmatter_and_arguments_token() {
        let opencode_commands = OPENCODE_TEMPLATES
            .get_dir("ark")
            .expect("templates/opencode/commands/ark exists");
        let mut count = 0;
        for file in opencode_commands.files() {
            count += 1;
            let name = file
                .path()
                .file_stem()
                .expect("opencode command has a stem")
                .to_str()
                .expect("ascii name");
            let body = std::str::from_utf8(file.contents()).expect("utf8 command body");

            // (a) frontmatter starts with `description:`.
            assert!(
                body.starts_with("---\n"),
                "command `{name}` must open with `---` frontmatter delimiter"
            );
            let after_open = &body[4..];
            let next_line = after_open.lines().next().unwrap_or("");
            assert!(
                next_line.starts_with("description:"),
                "command `{name}` first frontmatter line must start with `description:`, got: \
                 {next_line:?}",
            );

            // (b) no `argument-hint:` line in the frontmatter block.
            let frontmatter_end = after_open.find("\n---\n").expect("closing ---");
            let frontmatter = &after_open[..frontmatter_end];
            assert!(
                !frontmatter
                    .lines()
                    .any(|l| l.trim_start().starts_with("argument-hint:")),
                "command `{name}` frontmatter must not contain Claude's `argument-hint:` line",
            );

            // (c) body contains the backtick-quoted heading verbatim.
            let needle = format!("# `/ark:{name} $ARGUMENTS`");
            assert!(
                body.contains(&needle),
                "command `{name}` body must contain the literal heading {needle:?}",
            );
        }
        assert!(count >= 3, "expected at least 3 OpenCode commands");
    }

    /// Verifies that [`OPENCODE_TEMPLATES`] excludes plugins.
    ///
    /// The plugin file ships separately via `extra_files` and
    /// [`OPENCODE_ARK_CONTEXT_TS`].
    #[test]
    fn opencode_templates_does_not_contain_plugins_subtree() {
        assert!(
            OPENCODE_TEMPLATES
                .get_file("plugins/ark-context.ts")
                .is_none(),
            "OPENCODE_TEMPLATES must not contain plugins/ — the plugin ships via extra_files"
        );
        assert!(
            OPENCODE_TEMPLATES.get_dir("plugins").is_none(),
            "OPENCODE_TEMPLATES must be rooted at templates/opencode/commands, not the parent"
        );
    }

    /// Verifies that no `package.json` is reachable via templates.
    ///
    /// Regression guard for future template-tree changes (vacuous against
    /// the current implementation by design; activates if a future commit
    /// adds a `package.json` to the templates tree).
    #[test]
    fn opencode_templates_ships_no_package_json() {
        assert!(OPENCODE_TEMPLATES.get_file("package.json").is_none());
        assert!(OPENCODE_TEMPLATES.get_file("ark/package.json").is_none());
    }

    /// Verifies that plugin helpers remain internal functions.
    ///
    /// OpenCode's plugin runtime treats every named export as a plugin
    /// factory and invokes it at load time with no arguments — exporting
    /// a parameterized helper crashes plugin loading (verified
    /// empirically: `error=undefined is not an object (evaluating
    /// 'processed.has')`). This string-level guard catches refactors that
    /// rename or remove either helper or accidentally re-add `export`.
    #[test]
    fn opencode_plugin_keeps_helpers_internal() {
        let body = OPENCODE_ARK_CONTEXT_TS;
        assert!(
            body.contains("function buildEnvelopePrefix("),
            "plugin must define `buildEnvelopePrefix`"
        );
        assert!(
            body.contains("function shouldInject("),
            "plugin must define `shouldInject`"
        );
        // Critical invariant: these helpers must NOT be re-exported.
        // OpenCode's plugin loader invokes every named export at load time.
        assert!(
            !body.contains("export function buildEnvelopePrefix("),
            "`buildEnvelopePrefix` must NOT be exported — opencode invokes every named export at \
             load time and the helper takes parameters"
        );
        assert!(
            !body.contains("export function shouldInject("),
            "`shouldInject` must NOT be exported — opencode invokes every named export at load \
             time and the helper takes parameters"
        );
        // The helpers must have live consumers (otherwise they are dead).
        assert!(
            body.contains("buildEnvelopePrefix(additionalContext)"),
            "plugin must invoke `buildEnvelopePrefix` (live consumer)"
        );
        assert!(
            body.contains("shouldInject(sessionID, processedSessions)"),
            "plugin must invoke `shouldInject` (live consumer)"
        );
    }
}
