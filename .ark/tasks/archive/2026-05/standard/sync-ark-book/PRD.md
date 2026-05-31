# sync-ark-book PRD

---

[**What**]

Bring `docs/book/` (the mdBook user guide) back in sync with `main`, which has drifted since the book's last edit on 2026-05-02.

[**Why**]

The book was last touched on 2026-05-02 (the commit that *removed* workspace support). Since then ten-plus features landed on `main` and the book never followed: the research tier, the researcher/reviewer/verifier subagents, workspace (journals + developer identity, since re-added), recursive feature-SPEC paths, project specs, the detachable-feature-SPEC mechanics, and five new slash commands (`research`, `resume`, `discard`, `extract-spec`, `record`). Existing pages also carry factual drift — `reference/config-toml.md` omits the `[workspace]` section and lists the wrong `post_create` default. A reader following the book today is mislead about the current product. The canonical source of truth is `.ark/workflow.md` plus the shipped templates and current CLI surface.

[**Outcome**]

The published book describes Ark as it exists on `main` today. Concretely:

- **Tiers** documents four tiers, not three — research (`Research → Committed → Archived`, no PLAN/REVIEW/EXECUTE/VERIFY) added alongside quick/standard/deep. The promotion note states research does not participate in promotion.
- **Lifecycle** covers the research path and names the subagent hand-off at REVIEW and VERIFY (pick `ark-reviewer` / `ark-verifier` / a different model / self). The REVIEW iteration text matches workflow.md.
- **Specs** describes recursive feature-SPEC paths (the `[**SPEC Path**]` PRD block, slash-separated paths, leaf-to-root INDEX upsert) instead of the old flat `<name>` model, and mentions the detachable feature SPEC + `/ark:extract-spec` brownfield path.
- A new **Subagents** workflow page (or section) documents `ark-researcher` / `ark-reviewer` / `ark-verifier`: what each does, the reserved stems, and the embedded-researcher vs. research-tier distinction from workflow.md §Research.
- A new **Workspace & Journals** page documents `.ark/workspace/`, developer identity (`.ark/.developer`, `ark init --developer`), `/ark:record`, and the `[workspace]` config section.
- **CLI Overview** lists `ark archive` accurately and the visible/hidden split is correct against the current `ark --help`.
- **`.ark/config.toml`** reference gains the `[workspace]` section and corrects the `[worktree] post_create` default to `["git submodule update --init --recursive"]`.
- The **slash-command surface** (in getting-started and/or platforms pages) lists all eight shipped commands: `quick`, `design`, `commit`, `research`, `resume`, `discard`, `extract-spec`, `record`.
- **Contributing → Workspace Layout**'s `ark-core` module map matches the current tree (adds `cleanup.rs`, `workspace/` submodules, recursive `spec/`, `task/commit`).
- `SUMMARY.md` is updated for any new pages; `mdbook build docs/book` succeeds with no broken intra-book links.
- Nothing documents the unmerged `feat/ark-run` or `feat/ark-benchmark` work — book reflects `main` only.

[**Related Specs**]

This is a documentation-only task; it ships no code and promotes no SPEC. The book *describes* the following shipped feature SPECs, so each is a source of truth to read while writing, but the task does not modify any of them:

- `specs/features/ark-research/SPEC.md` — research tier + `/ark:research`; source for the Tiers/Lifecycle/Subagents updates.
- `specs/features/subagent-support/SPEC.md` — researcher/reviewer/verifier stems; source for the new Subagents page.
- `specs/features/workspace/SPEC.md` — journals + developer identity; source for the new Workspace page and `[workspace]` config docs.
- `specs/features/detachable-feature-spec/SPEC.md` — recursive SPEC paths + `/ark:extract-spec`; source for the Specs page rewrite.
- `specs/features/project-spec/SPEC.md` — project-spec layer; cross-check against the Specs page.
- `specs/features/ark-context/SPEC.md` — current `ark context` surface; cross-check the reference page.

[**SPEC Path**]

N/A — standard tier, no SPEC promotion.
