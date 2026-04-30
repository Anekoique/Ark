# Adding a Slash Command

Adding a new slash command (e.g. `/ark:status`) means writing parallel files under three template trees and letting the test suite enforce parity.

## The three trees

```
templates/
├── claude/commands/ark/
│   └── status.md           # Claude slash command
├── codex/skills/
│   └── ark-status/
│       └── SKILL.md        # Codex skill (description-routed)
└── opencode/commands/ark/
    └── status.md           # OpenCode slash command
```

Each platform reads its own subtree. The structural tests in `crates/ark-core/src/templates.rs` enforce that the three trees stay in sync:

- **`every_claude_command_has_a_codex_skill_sibling`** — every `templates/claude/commands/ark/<name>.md` must have a matching `templates/codex/skills/ark-<name>/SKILL.md`.
- **`every_claude_command_has_an_opencode_command_sibling`** — same, but `templates/opencode/commands/ark/<name>.md`.

So: add a Claude command, the test fails until you add the Codex skill and OpenCode command. Adding a Codex skill or OpenCode command without a Claude origin is allowed but unusual.

## Frontmatter contracts

Each platform's frontmatter has a different shape, and the tests check the right one is in the right tree:

**Claude** (`templates/claude/commands/ark/status.md`):

```yaml
---
description: Print the current task status.
argument-hint: <optional argument hint>
---

# `/ark:status $ARGUMENTS`

... slash command body ...
```

**Codex** (`templates/codex/skills/ark-status/SKILL.md`):

```yaml
---
name: ark-status
description: Use this skill when the user wants to know the current task status...
---

# ark-status

... description-routed skill body ...
```

**OpenCode** (`templates/opencode/commands/ark/status.md`):

```yaml
---
description: Print the current task status.
---

# `/ark:status $ARGUMENTS`

... slash command body ...
```

The test `codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter` catches copy-paste regressions where you accidentally drop a Claude header into the Codex tree. The test `opencode_command_bodies_have_opencode_frontmatter_and_arguments_token` checks OpenCode commands open with `description:` (no `argument-hint:`) and contain the verbatim heading `` # `/ark:<name> $ARGUMENTS` `` (with the actual command name substituted in).

## Body translation

The bodies aren't byte-equal across platforms — they can't be:

- Claude bodies use slash-invocation idioms (`# /ark:foo $ARGUMENTS`).
- Codex skills use description routing — the body explains *what* the skill does without referencing slash syntax.
- OpenCode bodies are closest to Claude's, but with OpenCode-specific frontmatter.

Body content parity is **not** mechanically asserted. It's policed at code review when you edit a template. The structural tests catch the failure modes that matter (missing twin, wrong frontmatter); deeper drift is a review concern.

A reasonable workflow:

1. Write the Claude command body first, get the workflow shape right.
2. Mechanically translate to OpenCode (drop `argument-hint:`, keep everything else).
3. For Codex, rewrite the body to drop slash references and lead with description-routing language.

## Backing CLI surface

Most slash commands wrap a chain of `ark agent` calls. If your new command needs new subcommands (e.g. `ark agent task status`), add them in `crates/ark-core/src/commands/agent/` first, then have the slash command bodies invoke them.

If your command only reads state (like `/ark:status` would), prefer extending `ark context` over adding a new `ark agent` subcommand. `ark context` is the semver-stable read-only surface; `ark agent` is the not-semver-stable mutation surface.

## Testing

After your three template files exist:

```bash
cargo test --package ark-core templates  # parity tests
cargo build --workspace                  # confirms include_dir!() picks up the new files
```

If you also added an `ark agent` subcommand, write integration tests under `crates/ark-cli/tests/agent_lifecycle.rs` exercising the new command end-to-end.

## Don't forget

- **`ark upgrade`** — your new template files are included in the embedded tree on the next CLI release. Existing projects pick them up via `ark upgrade`.
- **The `templates.rs` tests run in CI.** A missing twin will fail the build, not just a test.
- **Update the book.** Add a row to [Claude Code](../platforms/claude-code.md), [Codex](../platforms/codex.md), and [OpenCode](../platforms/opencode.md), and document the new command's purpose in [Quick Start](../getting-started/quick-start.md) if it's user-facing.
