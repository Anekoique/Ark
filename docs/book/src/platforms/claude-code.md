# Claude Code

[Claude Code](https://claude.com/claude-code) integration ships first-class. It's the original target platform for Ark and the most thoroughly exercised.

## What gets installed

```
.claude/
├── commands/ark/
│   ├── quick.md          # /ark:quick <title>
│   ├── design.md         # /ark:design [--deep] <title>
│   ├── commit.md         # /ark:commit [-m <msg>] [<slug>]
│   ├── research.md       # /ark:research <topic>
│   ├── resume.md         # /ark:resume <slug>
│   ├── discard.md        # /ark:discard [<slug>] [--force]
│   ├── extract-spec.md   # /ark:extract-spec <feature-name> [hint]
│   └── record.md         # /ark:record [<title>]
├── agents/
│   ├── ark-researcher.md # dispatched during DESIGN/PLAN
│   ├── ark-reviewer.md   # dispatched at REVIEW (deep)
│   └── ark-verifier.md   # dispatched at VERIFY
└── settings.json         # contains Ark's SessionStart hook entry

CLAUDE.md                 # managed block pointing at .ark/
```

The `.claude/commands/ark/` files are slash-command bodies. Each one carries Claude-specific YAML frontmatter (`description:`, `argument-hint:`) and a body that walks the agent through the corresponding workflow. The `.claude/agents/` files are the three Ark [subagents](../workflow/subagents.md) the command bodies dispatch.

## Slash commands

| Command            | Trigger                              | What it does                                                                          |
| ------------------ | ------------------------------------ | ------------------------------------------------------------------------------------- |
| `/ark:quick`       | `/ark:quick <title>`                 | Quick-tier flow: PRD → execute → user invokes commit.                                 |
| `/ark:design`      | `/ark:design [--deep] <title>`       | Standard-tier flow (or deep with `--deep`): PRD → PLAN → review → execute → verify.   |
| `/ark:research`    | `/ark:research <topic>`              | Research-tier flow: gather a reference corpus; follow-up implementation optional.     |
| `/ark:commit`      | `/ark:commit [-m <msg>] [<slug>]`    | Atomically close the current (or named) task in one git commit. Deep tier promotes SPEC. |
| `/ark:resume`      | `/ark:resume <slug>`                 | Refocus this session on an existing active slug. Idempotent.                          |
| `/ark:discard`     | `/ark:discard [<slug>] [--force]`    | Remove an unarchived task. Refuses without `--force` if seeded files have user content. |
| `/ark:extract-spec`| `/ark:extract-spec <feature> [hint]` | Author a feature SPEC from existing code (brownfield), no deep-tier task required.    |
| `/ark:record`      | `/ark:record [<title>]`              | Append a manual entry to the developer's workspace journal.                           |

Each tier command body starts by calling `ark context --scope phase --for <phase> --format json` to orient itself, then guides the agent through the rest of the phase.

## SessionStart hook

`ark init` installs a hook entry into `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "ark context --scope session --format json",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

`timeout` is in **milliseconds** (Claude's hook schema). At session start, Claude runs the command and feeds its stdout (the JSON `additionalContext` envelope) into the model's context window — so the agent sees current git state, active task, specs, and recent archive without any user message.

The `command` field is the **identity key** Ark uses to detect its own entry across runs. A user-added sibling hook in the same `SessionStart` array (e.g. a `PreToolUse` hook for some other purpose) is preserved across `unload` / `load` round-trips and across `ark upgrade`. Ark's entry is surgically removed and re-added; nothing else in `settings.json` is touched.

## CLAUDE.md managed block

`ark init` writes (or merges into existing) `CLAUDE.md`:

```markdown
<!-- ARK:START -->
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md
<!-- ARK:END -->
```

Edit anywhere outside the markers; `ark upgrade` re-renders inside them. The `@.ark/specs/INDEX.md` line is a Claude include directive — it inlines the SPEC index into the CLAUDE.md context every session.

If you don't want the block at all (e.g. you maintain your own `CLAUDE.md` from a template generator), [`ark remove`](../reference/ark-remove.md) deletes the block (and its delimiters) without touching surrounding content.

## What `unload` / `remove` do

- **`ark unload`** captures every file under `.claude/commands/ark/` (full bytes), the `<!-- ARK -->` block in `CLAUDE.md` (block content + position), and the `SessionStart` Ark entry from `settings.json`. Then deletes the live copies. Sibling content in `settings.json` (other top-level keys, other `SessionStart` array entries) stays on disk.
- **`ark remove`** does the destructive twin: same removal, no capture. Empty parents are pruned (e.g. an empty `.claude/commands/` after the `ark/` subtree is gone).

After `unload`, `.claude/settings.json` may shrink to `{}` or even be removed if it carried only the Ark entry — but if it had user content, the user content is preserved verbatim.

## Round-trip preservation

`unload` → `load` is byte-identical for everything except snapshot timestamps. Specifically:

- User-added files under `.claude/commands/` (e.g. `.claude/commands/my-team/foo.md`) survive captured-and-restored.
- User-modified `CLAUDE.md` content outside the managed block survives.
- User-added `SessionStart` siblings (other entries in the array) are preserved on disk through `unload`; `load` does not need to re-add them because `unload` doesn't remove them.
