# Ark

An agent harness and development workflow designed to orchestrate AI-driven programming tasks.

Ark is a small CLI (`ark`) plus a directory layout (`.ark/`) that gives an AI coding agent a stable place to think before it edits. Tasks are markdown files. Specs are markdown files. The agent fills the templates; you review the diffs.

> **Status.** Ark is experimental. The CLI surface marked semver-stable (`ark init`, `ark load`, `ark unload`, `ark remove`, `ark upgrade`, `ark context`) won't break across patch releases. The internal `ark agent` namespace is **not** semver-covered — its contract is with Ark's own shipped templates, not external callers.

## Why Ark?

AI coding agents work better with a harness.

- **Right ceremony, right task.** Three tiers — quick fix, feature, deep refactor — each with the minimum process that fits.
- **Reviewed, not rubber-stamped.** PLAN ↔ REVIEW iteration on deep work; a VERIFY gate before every archive.
- **Plain markdown, no hidden magic.** Tasks and specs live in `.ark/`, diffable and git-tracked. Ark writes only what it tracks; round-tripping `unload` → `load` preserves user-edited and user-added files losslessly.
- **Multi-platform.** Ships first-class integrations for Claude Code, Codex, and OpenCode out of the box.

## What's in this book

This book has five parts:

1. **Getting Started** walks you through install, the first `ark init`, and a complete `/ark:quick` task.
2. **Workflow** is the conceptual model: tiers, lifecycle, the spec system, and how worktrees fit in.
3. **Reference** documents every top-level CLI command and the `.ark/config.toml` schema.
4. **Platform Integrations** covers what files Ark drops where, per platform.
5. **Contributing** is for developers extending Ark itself: workspace layout, adding a slash command, adding a platform, and the release process.

If you're new, read parts 1–2 in order. If you're looking up syntax for a flag, jump to part 3.

## Inspiration

Ark builds on several prior projects:

- [Trellis](https://github.com/mindfold-ai/trellis) — the workflow shape (tiers, PRD/PLAN/VERIFY) is heavily borrowed.
- [Superpowers](https://github.com/obra/superpowers) — slash-command-driven agent UX.
- [OpenSpec](https://github.com/Fission-AI/OpenSpec) and [spec-kit](https://github.com/github/spec-kit) — spec-first development conventions.
- [humanize](https://github.com/humania-org/humanize) — multi-agent review loops.
