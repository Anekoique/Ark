# Workspace & Journals

Beyond per-task artifacts, Ark keeps a **workspace** — a per-developer journal of what happened in the project over time. Where a task's PRD/PLAN capture one unit of work, the journal is the running narrative across tasks: what shipped, who shipped it, and the standalone investigations that never became tasks.

## Layout

```
.ark/
├── .developer                  # your identity (gitignored)
└── workspace/
    ├── index.md                # auto-maintained roster of developers
    └── <dev>/
        ├── index.md            # auto-maintained session index for one developer
        ├── journal-1.md        # session entries; rotates at journal_max_lines
        └── journal-2.md        # journal-1 → journal-2 → … when the cap is hit
```

The two `index.md` files carry auto-maintained tables inside managed blocks (`<!-- ARK:DEVELOPERS -->` at the top level, `<!-- ARK:SESSIONS -->` per developer) — Ark rewrites those blocks; everything outside them is yours.

## Developer identity

Workspace writes are attributed to a developer name read from `.ark/.developer` (gitignored, so each checkout owns its own identity). Bootstrap it at init time:

```bash
ark init --developer "Ada Lovelace"
```

`--no-developer` skips the prompt; workspace operations then fail with `MissingIdentity` until you either rerun with `--developer`, write `.ark/.developer` by hand, or set `[workspace] developer` in `.ark/config.toml`.

## Two ways entries get written

- **Automatically, at commit.** `/ark:commit` appends a session entry to your active journal as part of the *same atomic commit* as the work. The closing-commit SHA is patched in deterministically at archive time, so each entry links to the commit that shipped it.
- **Manually, with `/ark:record`.** For notes not tied to any task — an exploration, a debugging session, an observation. `/ark:record [<title>]` appends a free-standing session block. Don't use it for task-driven work; `/ark:commit` already handles that.

`/ark:record` reads its context from `ark context --scope record --format json`, which resolves your identity, the active journal path, the session count, and `journal_max_lines`.

## Rotation

A journal file rotates to the next number once it exceeds `journal_max_lines` (default 2000, must be > 0). Configure it under `[workspace]` in `.ark/config.toml` — see [`.ark/config.toml`](../reference/config-toml.md).
