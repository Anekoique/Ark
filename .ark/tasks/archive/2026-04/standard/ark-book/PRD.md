# `ark-book` PRD

---

[**What**]
Two-part deliverable, shipped together as prep for the next Ark version bump:

1. **Comment cleanup.** Strip task-mark tags (`V-IT-15`, `V-UT-9`, `V-E-2`, `C-18`, `G-12`, `R-010`, etc.) from doc-comments and inline comments across `crates/`. These tags reference internal PLAN/REVIEW labels from archived deep-tier tasks; they leak ephemeral process artifacts into shipped source. Comments stay; only the tag tokens go (and lines that become empty after the strip).
2. **`docs/book/` mdBook.** Stand up an mdBook at `docs/book/` covering install, workflow, CLI reference, platform integrations, and contributor guide. Wire a GitHub Actions workflow that builds and deploys to GitHub Pages on tag.

[**Why**]
- The next release should be the first one a non-internal user can land on without reading source. Today the only entry points are `README.md` (5-minute overview) and `AGENTS.md` (written for AI agents in *this* repo, not for end users).
- Task-mark tags violate the project's own commenting norms ("don't reference the current task, fix, or callers — those rot as the codebase evolves"). They survived because they were authored mid-task as plan-traceability aids; nothing prunes them at archive. 234 instances across 38 files is enough that a sweep is warranted before the version bump rather than letting them keep accumulating.
- Cutting both at once means one publish event ships clean source + public docs as one milestone.

[**Outcome**]
- `grep -rEn '\b(V-IT-[0-9]+|V-UT-[0-9]+|V-E-[0-9]+|C-[0-9]+|G-[0-9]+|R-[0-9]+|IT-[0-9]+)\b' crates/` returns zero hits inside doc-comments and `//` comments. (Identifier-like matches in code — variable names, string literals — stay; the sweep targets *comments* only.)
- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all pass post-cleanup.
- `docs/book/` contains a working `book.toml` and `src/SUMMARY.md`; `mdbook build docs/book` produces `docs/book/book/` with no broken links.
- Book covers: Getting Started (install, `ark init`, first task), Workflow (tiers, lifecycle, specs), CLI Reference (every top-level subcommand + `.ark/config.toml` schema), Platform Integrations (Claude / Codex / OpenCode), Contributing (workspace map, adding a slash command, adding a platform, release).
- `.github/workflows/book.yml` builds and deploys the book to GitHub Pages on push of a release tag (`v*`). Manual `workflow_dispatch` trigger included for ad-hoc rebuilds.
- The version bump itself is **out of scope** — the user will bump `[workspace.package].version` and tag.

[**Related Specs**]

- `.ark/specs/features/ark-context/SPEC.md` — book's CLI Reference chapter documents `ark context`. Reference must match the SPEC's flag surface and JSON schema.
- `.ark/specs/features/ark-upgrade/SPEC.md` — Reference covers `ark upgrade` semantics (hash-tracked vs. non-tracked files, interactive flags).
- `.ark/specs/features/codex-support/SPEC.md` — Platform Integrations chapter sources its Codex section from this SPEC.
- `.ark/specs/features/opencode-support/SPEC.md` — same, for OpenCode.
- `.ark/specs/features/worktree/SPEC.md` — Workflow chapter cross-links to worktree usage; CLI Reference covers `ark agent task worktree *` (visible despite `ark agent` being hidden, because the subcommand is part of the documented workflow).
- `.ark/specs/features/workspace/SPEC.md` — Workflow chapter documents the per-developer journal; `[workspace]` table appears in the `.ark/config.toml` reference.
- `.ark/specs/features/ark-agent-namespace/SPEC.md` — Reference establishes the `ark agent` stability policy (hidden, not semver) and explains why most users never call it directly.
