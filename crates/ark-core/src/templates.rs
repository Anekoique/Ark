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
//! - [`crate::templates::CLAUDE_AGENT_TEMPLATES`] — extracted under
//!   `.claude/agents/` via `Platform.agents_templates`. `CLAUDE_TEMPLATES`
//!   physically contains the same files (since it is rooted at `templates/claude/`),
//!   but the main extract path filters them out so the agents are written
//!   unconditionally via `apply_managed_state` (preserves C-26's reserved-stem
//!   invariant on default `WriteMode::Skip`).
//! - [`crate::templates::CODEX_AGENT_TEMPLATES`] — extracted under `.codex/agents/`
//!   via `Platform.agents_templates`. Codex's main tree is rooted at `skills/`,
//!   so agents need a separate static.
//! - [`crate::templates::OPENCODE_AGENT_TEMPLATES`] — extracted under
//!   `.opencode/agents/` via `Platform.agents_templates`. OpenCode's main tree
//!   is rooted at `commands/`, so agents need a separate static.

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

/// Embedded Claude Code agent template tree (extracts under `.claude/agents/`).
///
/// `CLAUDE_TEMPLATES` is rooted at `templates/claude/`, so it physically also
/// contains the `agents/` subtree. The main extract path skips files whose
/// destination falls under [`crate::platforms::Platform::agents_dest_dir`] to
/// avoid double-extraction; this dedicated static is what
/// [`Platform::apply_managed_state`] uses to write Claude agents
/// unconditionally (so the reserved-stem invariant in `subagent-support`'s
/// SPEC C-26 holds even on default `WriteMode::Skip`).
///
/// [`Platform::apply_managed_state`]: crate::platforms::Platform::apply_managed_state
pub static CLAUDE_AGENT_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude/agents");

/// Embedded Codex agent template tree (extracts under `.codex/agents/`).
///
/// Codex's main template tree [`CODEX_TEMPLATES`] is rooted at
/// `templates/codex/skills/`, so the agents subtree must be a separate static.
/// Mirrors the same `Platform.agents_templates` indirection used by OpenCode.
pub static CODEX_AGENT_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/codex/agents");

/// Embedded OpenCode agent template tree (extracts under `.opencode/agents/`).
///
/// OpenCode's main template tree [`OPENCODE_TEMPLATES`] is rooted at
/// `templates/opencode/commands/`, so the agents subtree must be a separate
/// static.
pub static OPENCODE_AGENT_TEMPLATES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/../../templates/opencode/agents");

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

/// Whole-file body for auto-created subtree feature INDEX files.
///
/// Seeded by `spec_register`'s leaf-to-root upsert when a subtree `INDEX.md`
/// does not yet exist. Carries `<!-- ARK:FEATURES:START -->` /
/// `<!-- ARK:FEATURES:END -->` markers byte-identical to the root
/// `features/INDEX.md`'s managed-block delimiters; the existing
/// `read_managed_block` / `update_managed_block` helpers apply unchanged.
///
/// Embedded directly rather than living under `.ark/templates/` — that tree
/// is reserved for workflow artifact templates (PRD, PLAN, REVIEW, SPEC,
/// VERIFY), not infrastructure helpers.
pub const FEATURE_SUBTREE_INDEX_MD: &str = "\
# Feature Specs

Subtree index. Rows are managed by `ark agent task commit`'s leaf-to-root
walk. Edit prose outside the markers freely; the rows between the markers
are auto-maintained.

## Index

<!-- ARK:FEATURES:START -->
| Feature | Scope | Promoted |
|---------|-------|----------|
<!-- ARK:FEATURES:END -->
";

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

    /// Names of the three Ark subagents shipped per platform.
    const ARK_AGENTS: &[&str] = &["ark-researcher", "ark-reviewer", "ark-verifier"];

    /// Verifies that each platform ships exactly the three Ark subagents.
    ///
    /// Claude's agents live under [`CLAUDE_TEMPLATES`]'s `agents/` subtree;
    /// Codex's and OpenCode's live under their dedicated agent template trees.
    #[test]
    fn each_platform_ships_three_agents() {
        let claude_agents = CLAUDE_TEMPLATES
            .get_dir("agents")
            .expect("templates/claude/agents exists");
        let claude_stems: std::collections::BTreeSet<_> = claude_agents
            .files()
            .filter_map(|f| f.path().file_stem()?.to_str())
            .collect();
        for name in ARK_AGENTS {
            assert!(
                claude_stems.contains(name),
                "Claude must ship `{name}.md`, got {claude_stems:?}",
            );
        }

        let codex_stems: std::collections::BTreeSet<_> = CODEX_AGENT_TEMPLATES
            .files()
            .filter_map(|f| f.path().file_stem()?.to_str())
            .collect();
        for name in ARK_AGENTS {
            assert!(
                codex_stems.contains(name),
                "Codex must ship `{name}.toml`, got {codex_stems:?}",
            );
        }

        let opencode_stems: std::collections::BTreeSet<_> = OPENCODE_AGENT_TEMPLATES
            .files()
            .filter_map(|f| f.path().file_stem()?.to_str())
            .collect();
        for name in ARK_AGENTS {
            assert!(
                opencode_stems.contains(name),
                "OpenCode must ship `{name}.md`, got {opencode_stems:?}",
            );
        }
    }

    /// Returns the bytes of an Ark agent body for the given platform and stem.
    fn agent_body(platform: &str, stem: &str) -> Vec<u8> {
        match platform {
            "claude" => CLAUDE_TEMPLATES
                .get_file(format!("agents/{stem}.md"))
                .expect("claude agent file")
                .contents()
                .to_vec(),
            "codex" => CODEX_AGENT_TEMPLATES
                .get_file(format!("{stem}.toml"))
                .expect("codex agent file")
                .contents()
                .to_vec(),
            "opencode" => OPENCODE_AGENT_TEMPLATES
                .get_file(format!("{stem}.md"))
                .expect("opencode agent file")
                .contents()
                .to_vec(),
            _ => unreachable!("unknown platform `{platform}`"),
        }
    }

    /// Verifies that every agent prompt body carries a Recursion Guard section.
    #[test]
    fn agent_prompts_carry_recursion_guard() {
        for stem in ARK_AGENTS {
            for platform in &["claude", "codex", "opencode"] {
                let body = agent_body(platform, stem);
                let body_str = std::str::from_utf8(&body).expect("utf8 agent body");
                assert!(
                    body_str.contains("## Recursion guard"),
                    "{platform}/{stem} body must contain `## Recursion guard`"
                );
            }
        }
    }

    /// Verifies that every agent prompt declares an explicit write scope wall.
    #[test]
    fn agent_prompts_carry_write_scope_walls() {
        for stem in ARK_AGENTS {
            for platform in &["claude", "codex", "opencode"] {
                let body = agent_body(platform, stem);
                let body_str = std::str::from_utf8(&body).expect("utf8 agent body");
                assert!(
                    body_str.contains("## Write scope"),
                    "{platform}/{stem} body must contain `## Write scope`"
                );
                assert!(
                    body_str.contains("**Allowed:**"),
                    "{platform}/{stem} body must contain `**Allowed:**`"
                );
                assert!(
                    body_str.contains("**Forbidden:**"),
                    "{platform}/{stem} body must contain `**Forbidden:**`"
                );
            }
        }
    }

    /// Verifies the Claude agents' YAML frontmatter shape.
    #[test]
    fn claude_agent_frontmatter_shape() {
        for stem in ARK_AGENTS {
            let body = agent_body("claude", stem);
            let body_str = std::str::from_utf8(&body).expect("utf8");
            let prefix = format!("---\nname: {stem}\n");
            assert!(
                body_str.starts_with(&prefix),
                "Claude agent `{stem}` must open `{prefix}`"
            );
            let frontmatter_end = body_str
                .find("\n---\n")
                .expect("Claude agent has closing frontmatter delimiter");
            let frontmatter = &body_str[..frontmatter_end];
            assert!(
                frontmatter.contains("description:"),
                "Claude agent `{stem}` frontmatter must contain `description:`"
            );
            assert!(
                frontmatter.contains("tools:"),
                "Claude agent `{stem}` frontmatter must contain `tools:`"
            );
        }
    }

    /// Verifies the Codex agents' TOML schema (per `codex-support` SPEC C-25 / Trellis prior art).
    #[test]
    fn codex_agent_frontmatter_shape() {
        for stem in ARK_AGENTS {
            let body = agent_body("codex", stem);
            let body_str = std::str::from_utf8(&body).expect("utf8");
            let parsed: toml::Value =
                toml::from_str(body_str).expect("Codex agent file is parseable TOML");
            let table = parsed.as_table().expect("TOML root is a table");
            for key in &[
                "name",
                "description",
                "sandbox_mode",
                "developer_instructions",
            ] {
                assert!(
                    table.contains_key(*key),
                    "Codex agent `{stem}` must define `{key}` at top level"
                );
            }
            assert_eq!(
                table.get("name").and_then(|v| v.as_str()),
                Some(*stem),
                "Codex agent `{stem}` `name` must equal stem"
            );
            assert_eq!(
                table.get("sandbox_mode").and_then(|v| v.as_str()),
                Some("workspace-write"),
                "Codex agent `{stem}` must use `sandbox_mode = \"workspace-write\"`"
            );
            let features = table
                .get("features")
                .and_then(|v| v.as_table())
                .expect("Codex agent has [features] table");
            assert_eq!(
                features.get("multi_agent").and_then(|v| v.as_bool()),
                Some(false),
                "Codex agent `{stem}` must set `[features].multi_agent = false`"
            );
            let v2 = features
                .get("multi_agent_v2")
                .and_then(|v| v.as_table())
                .expect("[features.multi_agent_v2] table present");
            assert_eq!(
                v2.get("enabled").and_then(|v| v.as_bool()),
                Some(false),
                "Codex agent `{stem}` must set `[features.multi_agent_v2].enabled = false`"
            );
        }
    }

    /// Verifies the OpenCode agents' frontmatter shape.
    #[test]
    fn opencode_agent_frontmatter_shape() {
        for stem in ARK_AGENTS {
            let body = agent_body("opencode", stem);
            let body_str = std::str::from_utf8(&body).expect("utf8");
            assert!(
                body_str.starts_with("---\ndescription:"),
                "OpenCode agent `{stem}` must open `---\\ndescription:`"
            );
            let frontmatter_end = body_str
                .find("\n---\n")
                .expect("OpenCode agent has closing frontmatter delimiter");
            let frontmatter = &body_str[..frontmatter_end];
            assert!(
                frontmatter.contains("mode: subagent"),
                "OpenCode agent `{stem}` must declare `mode: subagent`"
            );
            assert!(
                frontmatter.contains("permission:"),
                "OpenCode agent `{stem}` must define a `permission:` block"
            );
        }
    }

    /// Strips per-platform frontmatter from an agent body and normalizes the
    /// commit-command token so the residual is the platform-neutral prompt.
    ///
    /// Codex skills use bare `ark-commit`; Claude / OpenCode slash commands
    /// use `/ark:commit`. Both forms collapse to `<COMMIT>` for the equality
    /// check.
    fn strip_frontmatter_and_normalize(platform: &str, stem: &str) -> String {
        let body = agent_body(platform, stem);
        let body_str = std::str::from_utf8(&body).expect("utf8");
        let stripped = match platform {
            "codex" => {
                let parsed: toml::Value = toml::from_str(body_str).unwrap();
                parsed
                    .get("developer_instructions")
                    .and_then(|v| v.as_str())
                    .expect("codex developer_instructions present")
                    .to_string()
            }
            _ => {
                // Skip the first --- ... --- frontmatter block.
                let after_open = body_str.strip_prefix("---\n").expect("frontmatter open");
                let end = after_open
                    .find("\n---\n")
                    .expect("frontmatter close marker");
                after_open[end + "\n---\n".len()..].to_string()
            }
        };
        stripped
            .replace("/ark:commit", "<COMMIT>")
            .replace("ark-commit", "<COMMIT>")
            .trim_start_matches('\n')
            .to_string()
    }

    /// Verifies that prompt bodies are byte-identical across platforms after
    /// stripping per-platform frontmatter and normalizing the platform-native
    /// commit-command form (Codex's bare `ark-commit` vs Claude/OpenCode's
    /// `/ark:commit`).
    #[test]
    fn agent_bodies_are_byte_identical_modulo_platform_idioms() {
        for stem in ARK_AGENTS {
            let claude = strip_frontmatter_and_normalize("claude", stem);
            let codex = strip_frontmatter_and_normalize("codex", stem);
            let opencode = strip_frontmatter_and_normalize("opencode", stem);
            assert_eq!(
                claude, codex,
                "agent `{stem}`: Claude and Codex bodies must be byte-identical after frontmatter \
                 + Dispatch-line strip"
            );
            assert_eq!(
                claude, opencode,
                "agent `{stem}`: Claude and OpenCode bodies must be byte-identical after \
                 frontmatter + Dispatch-line strip"
            );
        }
    }

    /// Verifies that the researcher prompt names the literal contract phrase.
    #[test]
    fn researcher_prompt_carries_paths_summaries_contract() {
        for platform in &["claude", "codex", "opencode"] {
            let body = agent_body(platform, "ark-researcher");
            let body_str = std::str::from_utf8(&body).expect("utf8");
            assert!(
                body_str.contains("paths plus one-line summaries"),
                "{platform}/ark-researcher body must contain the literal contract phrase `paths \
                 plus one-line summaries`"
            );
        }
    }

    /// Verifies that the reviewer prompt names the self-containment rule with HIGH severity.
    #[test]
    fn reviewer_prompt_carries_self_containment_rule() {
        for platform in &["claude", "codex", "opencode"] {
            let body = agent_body(platform, "ark-reviewer");
            let body_str = std::str::from_utf8(&body).expect("utf8");
            assert!(
                body_str.contains("not self-contained"),
                "{platform}/ark-reviewer must carry the self-containment rule"
            );
            assert!(
                body_str.contains("HIGH"),
                "{platform}/ark-reviewer must tag the rule as HIGH severity"
            );
        }
    }

    /// Verifies that the reviewer prompt names the SPEC-contradiction rule with CRITICAL severity.
    #[test]
    fn reviewer_prompt_carries_spec_contradiction_rule() {
        for platform in &["claude", "codex", "opencode"] {
            let body = agent_body(platform, "ark-reviewer");
            let body_str = std::str::from_utf8(&body).expect("utf8");
            assert!(
                body_str.contains("contradicts an existing feature SPEC"),
                "{platform}/ark-reviewer must carry the SPEC-contradiction rule"
            );
            assert!(
                body_str.contains("CRITICAL"),
                "{platform}/ark-reviewer must tag the rule as CRITICAL severity"
            );
        }
    }

    /// Verifies that the verifier prompt explicitly forbids self-fix.
    #[test]
    fn verifier_prompt_carries_no_self_fix_rule() {
        for platform in &["claude", "codex", "opencode"] {
            let body = agent_body(platform, "ark-verifier");
            let body_str = std::str::from_utf8(&body).expect("utf8");
            let phrases = ["does not self-fix", "no self-fix", "do NOT self-fix"];
            assert!(
                phrases.iter().any(|p| body_str.contains(*p)),
                "{platform}/ark-verifier must contain a no-self-fix phrase ({phrases:?})"
            );
        }
    }

    /// Verifies that `templates/claude/commands/ark/design.md` names all three Ark subagents.
    #[test]
    fn design_md_names_three_agents() {
        let body = std::str::from_utf8(
            CLAUDE_TEMPLATES
                .get_file("commands/ark/design.md")
                .expect("design.md exists")
                .contents(),
        )
        .expect("utf8");
        for name in ARK_AGENTS {
            assert!(
                body.contains(name),
                "design.md must reference `{name}` at the dispatch site"
            );
        }
    }

    /// Verifies that `codex-support` SPEC carries a CHANGELOG entry naming this task.
    ///
    /// The SPEC file lives in this repo's own dotfile checkout (`.ark/specs/...`)
    /// rather than the embedded `ARK_TEMPLATES` tree — the templates ship the
    /// scaffolding, not the project's own promoted feature SPECs. This repo is
    /// self-hosting so the path is stable.
    #[test]
    fn codex_support_spec_changelog_present() {
        let body = include_str!("../../../.ark/specs/features/codex-support/SPEC.md");
        assert!(
            body.contains("`subagent-support`"),
            "codex-support SPEC must reference `subagent-support` in its CHANGELOG"
        );
    }

    /// Verifies that `opencode-support` SPEC carries a CHANGELOG entry naming this task.
    ///
    /// See `codex_support_spec_changelog_present` for the path rationale.
    #[test]
    fn opencode_support_spec_changelog_present() {
        let body = include_str!("../../../.ark/specs/features/opencode-support/SPEC.md");
        assert!(
            body.contains("`subagent-support`"),
            "opencode-support SPEC must reference `subagent-support` in its CHANGELOG"
        );
    }

    /// The `research.md` bodies under
    /// `templates/claude/commands/ark/` and `templates/opencode/commands/ark/`
    /// are byte-identical after stripping their respective frontmatter blocks.
    #[test]
    fn research_slash_command_claude_and_opencode_bodies_match() {
        let claude = CLAUDE_TEMPLATES
            .get_file("commands/ark/research.md")
            .expect("claude research.md exists")
            .contents_utf8()
            .expect("utf8");
        let opencode = OPENCODE_TEMPLATES
            .get_file("ark/research.md")
            .expect("opencode research.md exists")
            .contents_utf8()
            .expect("utf8");

        let claude_body = strip_md_frontmatter(claude);
        let opencode_body = strip_md_frontmatter(opencode);
        assert_eq!(
            claude_body, opencode_body,
            "Claude and OpenCode research.md bodies must be byte-identical after frontmatter strip",
        );
    }

    /// The Codex `ark-research/SKILL.md` body matches the
    /// Claude `research.md` body after applying the inverse of C-11's
    /// substitution map.
    ///
    /// Per C-11, the substitution only rewrites slash-command names — i.e.
    /// `/ark:research`, `/ark:quick`, `/ark:design`, `/ark:commit` — to their
    /// `ark-<name>` Codex equivalents, plus `$ARGUMENTS → <topic>` and the
    /// H1 line reshape. The agent subagent name `ark-researcher` is NOT a
    /// slash command (it never had a `/ark:` form) and is left alone on both
    /// sides.
    #[test]
    fn research_codex_skill_matches_claude_under_inverse_substitution() {
        let claude = CLAUDE_TEMPLATES
            .get_file("commands/ark/research.md")
            .expect("claude research.md exists")
            .contents_utf8()
            .expect("utf8");
        let codex = CODEX_TEMPLATES
            .get_file("ark-research/SKILL.md")
            .expect("codex SKILL.md exists")
            .contents_utf8()
            .expect("utf8");

        let claude_body = strip_md_frontmatter(claude);
        let codex_body = strip_md_frontmatter(codex);

        // Apply the inverse substitution to the Codex body. Order matters:
        // rewrite the H1 line's `ark-research <topic>` to
        // `/ark:research $ARGUMENTS` before the generic `ark-research` pass,
        // so we don't double-rewrite. The closed slash-command set is
        // exactly {research, quick, design, commit} — `ark-researcher`
        // (the subagent) is deliberately NOT in this set.
        let codex_normalized = codex_body
            .replace("ark-research <topic>", "/ark:research $ARGUMENTS")
            .replace("ark-research`", "/ark:research`")
            .replace("ark-quick", "/ark:quick")
            .replace("ark-design", "/ark:design")
            .replace("ark-commit", "/ark:commit");

        assert_eq!(
            claude_body, codex_normalized,
            "Codex SKILL.md body must match Claude research.md after inverse substitution \
             `/ark:<cmd> → ark-<cmd>` and `$ARGUMENTS → <topic>`",
        );
    }

    /// Strips the leading `---\n...\n---\n` markdown frontmatter block.
    /// Returns the input unchanged if no frontmatter is present.
    fn strip_md_frontmatter(body: &str) -> String {
        if let Some(after_open) = body.strip_prefix("---\n")
            && let Some(end) = after_open.find("\n---\n")
        {
            return after_open[end + "\n---\n".len()..].to_string();
        }
        body.to_string()
    }
}
