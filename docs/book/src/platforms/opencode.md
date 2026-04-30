# OpenCode

[OpenCode](https://opencode.ai) integration ships out of the box. Like Claude Code, OpenCode supports user-typed slash commands; unlike Claude, OpenCode injects context via a TypeScript plugin rather than a JSON hook.

## What gets installed

```
.opencode/
├── commands/ark/
│   ├── quick.md          # /ark:quick $ARGUMENTS
│   ├── design.md         # /ark:design [--deep] $ARGUMENTS
│   ├── archive.md        # /ark:archive
│   └── record.md         # /ark:record [<title>]
└── plugins/
    └── ark-context.ts    # injects `ark context` output as additionalContext

AGENTS.md                 # managed block (shared with Codex if both are installed)
```

## Slash commands

OpenCode commands look very similar to Claude's, but with OpenCode-specific frontmatter:

```yaml
---
description: Run a quick-tier Ark task — reversible, no new abstractions.
---

# `/ark:quick $ARGUMENTS`

... body identical to Claude's, with the same flow ...
```

Two structural invariants are tested:

- `every_claude_command_has_an_opencode_command_sibling` — adding a Claude command without an OpenCode twin fails the build.
- `opencode_command_bodies_have_opencode_frontmatter_and_arguments_token` — each OpenCode command body must open with `---\ndescription:` (no `argument-hint:` line — that's Claude-specific) and contain the verbatim heading `` # `/ark:<name> $ARGUMENTS` `` matching the Claude templates exactly.

## Plugin-based context injection

OpenCode doesn't have a JSON hook schema like Claude or Codex. Instead, Ark ships a TypeScript plugin at `.opencode/plugins/ark-context.ts` that runs on session start, calls `ark context --scope session --format json`, and injects the output as `additionalContext` for the conversation.

The plugin is shipped via a separate mechanism (`extra_files` in the platform registry) rather than the embedded template tree, because:

1. The plugin is a real TypeScript file with imports, not a markdown command body.
2. The path is `.opencode/plugins/`, parallel to but distinct from `.opencode/commands/`.

The plugin file is **never** hash-tracked — it's re-applied unconditionally on every `ark init` / `ark load` / `ark upgrade`. User edits to the plugin will be overwritten on the next upgrade.

### Plugin internals

The plugin defines two pure helpers — `buildEnvelopePrefix` and `shouldInject` — as plain `function` declarations, **not** `export function`. OpenCode's plugin runtime treats every named export as a plugin factory and invokes it at load time with no arguments; exporting a parameterized helper crashes plugin loading. This invariant is locked down in `templates.rs` tests:

```rust
assert!(
    !body.contains("export function buildEnvelopePrefix("),
    "`buildEnvelopePrefix` must NOT be exported — opencode invokes every named export at \
     load time and the helper takes parameters"
);
```

If you're modifying the plugin, keep helpers internal and only export the actual plugin factory.

## AGENTS.md managed block

OpenCode reads `AGENTS.md` (same file Codex uses). When both platforms are installed, the managed block is recorded once in the manifest (`(file, marker)` dedupe), and the on-disk file contains exactly one `<!-- ARK:START -->` marker.

## What `unload` / `remove` do

- **`ark unload`** captures `.opencode/commands/`, `.opencode/plugins/`, and the `AGENTS.md` managed block. There's no hook file to capture (OpenCode's plugin isn't hook-shaped). The whole `.opencode/` tree is deleted from disk.
- **`ark remove`** removes the entire `.opencode/` directory (including any sibling subdirs the user added — there's no surgical hook removal because there's no hook).

OpenCode's `removal_root` is `.opencode/` itself, not just `.opencode/commands/`. This is intentional: the `.opencode/plugins/` subtree is also Ark-managed (the plugin file is the only thing in it), so removing the whole `.opencode/` keeps the cleanup honest.

If you want to keep `.opencode/` for some other purpose (e.g. you have non-Ark plugins or commands there), use `ark unload` to snapshot, then manually edit before `load`. Or don't install OpenCode integration in the first place: `ark init --no-opencode`.
