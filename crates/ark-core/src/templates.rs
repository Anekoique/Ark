//! Embedded template trees.
//!
//! Templates are compiled into the binary via `include_dir!`. Four trees ship:
//!
//! - [`ARK_TEMPLATES`] — extracted into the host project's `.ark/` directory
//! - [`CLAUDE_TEMPLATES`] — extracted into the host project's `.claude/` directory
//! - [`CODEX_TEMPLATES`] — extracted into the host project's `.codex/skills/`
//!   directory. Only the `skills/` subtree is hash-tracked; `.codex/hooks.json`
//!   is owned by [`crate::io::update_hook_file`] (driven by
//!   `CODEX_PLATFORM.hook_file`), and `.codex/config.toml` ships via the
//!   whole-file [`CODEX_CONFIG_TOML`] constant (re-applied unconditionally on
//!   every `init` / `upgrade`; not hash-tracked).
//! - [`OPENCODE_TEMPLATES`] — extracted into the host project's
//!   `.opencode/commands/` directory. The `.opencode/plugins/ark-context.ts`
//!   file ships via the whole-file [`OPENCODE_ARK_CONTEXT_TS`] constant
//!   (re-applied unconditionally on every `init` / `load` / `upgrade`; not
//!   hash-tracked, mirroring `CODEX_CONFIG_TOML`).

use include_dir::{Dir, include_dir};

pub static ARK_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/ark");
pub static CLAUDE_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude");
pub static CODEX_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/skills");
pub static OPENCODE_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/commands");

/// Whole-file body for `.codex/config.toml`. Re-applied unconditionally on
/// every `init`/`upgrade`. Not hash-tracked. The matching `.codex/hooks.json`
/// lifecycle is owned by `update_hook_file` (surgical SessionStart entry;
/// sibling user hooks preserved) — no whole-file rewrite needed.
pub const CODEX_CONFIG_TOML: &str = include_str!("../../../templates/codex/config.toml");

/// Whole-file body for `.opencode/plugins/ark-context.ts`. Re-applied
/// unconditionally on every `init`/`load`/`upgrade`. Not hash-tracked.
/// OpenCode has no native JSON `SessionStart` hook surface; this Bun-loaded
/// TypeScript plugin replaces that role by shelling out to
/// `ark context --scope session --format json` from `chat.message` and
/// prepending the unwrapped context to the first user message via
/// `experimental.chat.messages.transform`.
pub const OPENCODE_ARK_CONTEXT_TS: &str =
    include_str!("../../../templates/opencode/plugins/ark-context.ts");

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

    /// opencode-support V-IT-1 / G-12 (a): every Claude slash command under
    /// `templates/claude/commands/ark/<name>.md` has a matching OpenCode
    /// slash command at `templates/opencode/commands/ark/<name>.md`.
    /// Existence-only — content parity is policed at code review.
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

    /// opencode-support V-IT-2 / G-12 (b): each OpenCode command body opens
    /// with a `---\ndescription:` frontmatter block (no `argument-hint:`
    /// line — that's Claude-specific) and contains the verbatim
    /// backtick-quoted heading `` # `/ark:<name> $ARGUMENTS` `` matching
    /// the Claude templates exactly.
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

    /// opencode-support V-UT-13 (per R-010): `OPENCODE_TEMPLATES` does NOT
    /// contain a `plugins/` subtree — the plugin file ships separately via
    /// `extra_files` + `OPENCODE_ARK_CONTEXT_TS`. Locks down C-2.
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

    /// opencode-support V-UT-14 (per R-010, role per R-103): no
    /// `package.json` is reachable via `OPENCODE_TEMPLATES`. Regression
    /// guard for future template-tree changes (vacuous against the current
    /// implementation by design; activates if a future commit adds a
    /// `package.json` to the templates tree).
    #[test]
    fn opencode_templates_ships_no_package_json() {
        assert!(OPENCODE_TEMPLATES.get_file("package.json").is_none());
        assert!(OPENCODE_TEMPLATES.get_file("ark/package.json").is_none());
    }

    /// opencode-support: the plugin's pure helpers (`buildEnvelopePrefix`,
    /// `shouldInject`) MUST be defined as plain `function` declarations,
    /// NOT `export function`. OpenCode's plugin runtime treats every named
    /// export as a plugin factory and invokes it at load time with no
    /// arguments — exporting a parameterized helper crashes plugin loading
    /// (verified empirically: `error=undefined is not an object (evaluating
    /// 'processed.has')`). The helpers are still testable via this
    /// string-level guard, which catches refactors that rename or remove
    /// either helper or accidentally re-add `export`.
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
        // The helpers must have live consumers (otherwise they're dead).
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
