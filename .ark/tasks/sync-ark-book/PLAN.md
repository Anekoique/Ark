# sync-ark-book PLAN `00`

> Status: Draft
> Feature: `sync-ark-book`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none

---

## Summary

Rewrite and extend `docs/book/` so it describes Ark as it exists on `main` today. The book froze on 2026-05-02 and missed the research tier, subagents, workspace/journals, recursive feature-SPEC paths, the `ark cleanup` command, and five new slash commands; several existing pages also carry factual drift. This PLAN enumerates the exact per-file edits, two new pages, and the `SUMMARY.md` wiring, validated by a clean `mdbook build` with no broken intra-book links. Source of truth throughout is `.ark/workflow.md`, the shipped templates, and live `ark … --help` output — not prior assumptions.

## Log `None in 00_PLAN`

---

## Spec

> Documentation-only task; standard tier promotes no SPEC. The Spec section records the durable shape of the change for the VERIFY gate.

[**Goals**]

- G-1: Workflow chapter documents four tiers including research, matching workflow.md.
- G-2: Book documents the researcher/reviewer/verifier subagents and where each is offered.
- G-3: Book documents workspace journals + developer identity as a first-class concept.
- G-4: Specs chapter describes recursive feature-SPEC paths, not the old flat model.
- G-5: Reference + platform pages list every shipped command and slash command accurately.
- G-6: `mdbook build docs/book` succeeds with zero broken intra-book links.

[**Non-goals**]

- NG-1: Document the unmerged `feat/ark-run` / `feat/ark-benchmark` commands — book reflects `main` only.
- NG-2: Document the unshipped `revise` / `finalize` workspace ops — they are not on `main`.
- NG-3: Restructure the five-part book layout or change `book.toml` theming.

[**Architecture**]

```
docs/book/src/
├── SUMMARY.md                         # EDIT: wire two new pages
├── introduction.md                    # EDIT: "four tiers"; subagents in the why-list; book-parts blurb
├── getting-started/
│   ├── quick-start.md                 # EDIT: scaffold tree (config.toml sections, full command list); slash-command list
│   └── first-task.md                  # EDIT: "standard/deep/research" pointer; NN_REVIEW wording stays
├── workflow/
│   ├── tiers.md                       # EDIT: add research row + prose; promotion-excludes-research note
│   ├── lifecycle.md                   # EDIT: research path; subagent hand-off at REVIEW/VERIFY
│   ├── specs.md                       # EDIT: recursive paths, [**SPEC Path**] block, extract-spec brownfield
│   ├── subagents.md                   # NEW: researcher/reviewer/verifier; embedded vs research-tier
│   └── worktrees.md                   # EDIT (verify only): confirm current; touch if drifted
├── reference/
│   ├── cli-overview.md                # EDIT: add `ark cleanup`; visible-command table accurate
│   ├── ark-agent.md                   # EDIT: task tree (resume/discard); spec tree (import); recursive paths
│   ├── config-toml.md                 # EDIT: add [workspace] section; fix post_create default
│   └── workspace.md                   # NEW: .ark/workspace/, .ark/.developer, ark init --developer, /ark:record
│                                      #      (reference-layer companion to workflow/… if a workflow page is cleaner)
├── platforms/
│   ├── claude-code.md                 # EDIT: full slash-command list + command-dir tree
│   ├── codex.md                       # EDIT (verify): skill list parity with shipped templates
│   └── opencode.md                    # EDIT (verify): command list parity
└── contributing/
    └── workspace-layout.md            # EDIT: ark-core module map (cleanup.rs, workspace/*, recursive spec/, task/commit)
```

> Placement decision (T-1): the new Workspace page lives in the **Workflow** chapter (`workflow/workspace.md`), since journals/identity are a workflow concept the user interacts with, not a CLI command. The `[workspace]` config table stays in `reference/config-toml.md`.

[**Data Structure**]

N/A — no code, no types.

[**API Surface**]

N/A — no code, no functions.

[**Constraints**]

- C-1: Every command and flag named in the book must match live `ark … --help` on `main` at authoring time.
- C-2: All cross-page links use repo-relative `./…`/`../…` paths that resolve under `mdbook build`.
- C-3: Every new page added to `src/` has a corresponding `SUMMARY.md` entry (mdbook drops unlisted pages).
- C-4: Tier counts and slash-command lists are stated once per page consistently — no page says "three tiers".
- C-5: No book page references `feat/ark-run`, `ark run`, `ark benchmark`, `revise`, or `finalize`.
- C-6: Edits to existing pages preserve their voice and section structure; change facts, not tone.

---

## Runtime

[**Main Flow**]

1. Re-read each target page immediately before editing it (state may have shifted during the task).
2. Apply the per-file edits in the Implementation phases below.
3. Add the two new pages and their `SUMMARY.md` entries.
4. Run `mdbook build docs/book`; fix any error or broken-link warning.
5. Grep the built/src tree for forbidden tokens (C-5) and stale phrases ("three tiers").

[**Failure Flow**]

1. `mdbook build` errors on a missing linked file → add the file or fix the link, rebuild.
2. A `--help` snippet in the book diverges from the binary → trust the binary, correct the book.

[**State Transitions**]

- N/A — no runtime state machine; this is content.

---

## Implementation

[**Phase 1 — Workflow chapter (the conceptual core)**]

- `workflow/tiers.md`: add a Research row to the tier table (`Trigger /ark:research`, `Artifacts PRD.md + research/`, `Path research → committed → archived`); add a short "Research" prose paragraph; in "Tier promotion mid-flight" note research does **not** participate in promotion (cross-over is a fresh task citing the research slug). Keep the existing three-tier prose, reframed as four.
- `workflow/lifecycle.md`: add a research note to the ASCII/­prose (research skips PLAN/REVIEW/EXECUTE/VERIFY, goes DESIGN→COMMIT→ARCHIVE); in REVIEW add the reviewer hand-off (`ark-reviewer` / different model / self); in VERIFY add the verifier hand-off (`ark-verifier` / different model / self).
- `workflow/specs.md`: replace the flat `specs/features/<name>/` model with the recursive path model — the PRD's `[**SPEC Path**]` block, slash-separated paths, leaf-to-root INDEX upsert, recursive INDEX tree; add a short note on `/ark:extract-spec` for brownfield SPEC authoring (and `ark agent spec import`).
- `workflow/subagents.md` (NEW): document the three reserved stems (`ark-researcher`, `ark-reviewer`, `ark-verifier`), what each does, that stems are reserved + overwritten on init/upgrade/load, and the embedded-researcher-vs-research-tier distinction (lifted from workflow.md §Research). Cross-platform note (claude/codex/opencode all ship them).
- `workflow/workspace.md` (NEW): document `.ark/workspace/` per-developer journals (rotation at `journal_max_lines`), developer identity in `.ark/.developer` (gitignored, set via `ark init --developer <name>` or hand-write), and `/ark:record` for manual journal entries. Point at `reference/config-toml.md#workspace` for the knobs.

[**Phase 2 — Reference chapter**]

- `reference/cli-overview.md`: add `ark cleanup` to the visible-command table with its one-line summary; confirm `ark archive` row; keep the hidden-`ark agent` split.
- `reference/ark-agent.md`: update the `task` subtree to include `resume` and `discard`; update the `spec` subtree to include `import`; correct `spec extract`/`register` descriptions to recursive `specs/features/<path>/`; verify worktree-cleanup vs. top-level `ark cleanup` cross-reference; keep error list, add nothing unshipped.
- `reference/config-toml.md`: add the `[workspace]` section (the `journal_max_lines` field, default 2000, must be >0) with a synopsis + field table + lifecycle note ("preserved by upgrade"); fix the `[worktree] post_create` default from `[]` to `["git submodule update --init --recursive"]` in both the synopsis block and the field table; note `.ark/.developer` is synced automatically and must not be added to `copy`.

[**Phase 3 — Getting-started, platforms, contributing, then build**]

- `getting-started/quick-start.md`: update the scaffold tree (config.toml is sectioned `[worktree]`/`[workspace]`, `.ark/workspace/` exists, `.ark/.developer`); expand the slash-command examples to the full shipped set; keep the prose mental-model framing.
- `getting-started/first-task.md`: change the "standard and deep" closing section to mention research as the fourth tier with a one-line pointer; leave the quick-tier walkthrough intact.
- `introduction.md`: "Three tiers" → "Four tiers"; add subagents/journals to the why-list as appropriate; update the "what's in this book" blurb if new pages change a chapter's contents.
- `platforms/claude-code.md`: expand the `commands/ark/` tree and slash-command table to all eight shipped command files (`quick`, `design`, `commit`, `research`, `resume`, `discard`, `extract-spec`, `record`).
- `platforms/codex.md`, `platforms/opencode.md`: verify the command/skill lists against shipped templates; correct any count or naming drift (no new pages).
- `contributing/workspace-layout.md`: refresh the `ark-core` module map to the current tree — add `commands/cleanup.rs`, the `agent/workspace/` submodules (`config.rs`, `developer.rs`, `stamp.rs`, `transaction.rs`, `mod.rs`), recursive `agent/spec/`, and `task/commit`; mirror the structure in `CLAUDE.md`.
- Build + sweep: `mdbook build docs/book`; grep for "three tiers", `ark run`, `ark benchmark`, `revise`, `finalize`; fix anything that surfaces.

---

## Trade-offs

- T-1: New Workspace page in **Workflow** vs **Reference**. Chose Workflow — journals/identity are a user-facing workflow concept; the config knobs alone live in Reference. Keeps the "Reference = CLI/schema" contract clean.
- T-2: Two new pages vs folding subagents/workspace into existing pages. Chose new pages — each is a distinct concept with enough surface to warrant a TOC entry; folding would bloat `tiers.md`/`lifecycle.md` past their single-topic focus.
- T-3: Rewrite `specs.md` in place vs append a recursive-paths section. Chose in-place rewrite of the flat-model parts — leaving the old flat description alongside the new recursive one would actively mislead (the flat model no longer exists).

---

## Validation

[**Unit Tests**]

- V-UT-1: N/A — documentation task, no code units. Content checks live under Integration/Edge below.

[**Integration Tests**]

- V-IT-1 (G-6): `mdbook build docs/book` exits 0 with no broken-link warnings in output.
- V-IT-2 (G-1): `tiers.md` and `lifecycle.md` both contain a research-tier entry; `grep -ri "three tiers" docs/book/src` returns nothing.
- V-IT-3 (G-2): `workflow/subagents.md` exists, is listed in `SUMMARY.md`, and names all three stems.
- V-IT-4 (G-3): `workflow/workspace.md` exists, is listed in `SUMMARY.md`, and documents `.ark/.developer` + `/ark:record`.
- V-IT-5 (G-4): `specs.md` references `[**SPEC Path**]` and recursive leaf-to-root INDEX behavior; no longer claims a flat `specs/features/<name>/`-only layout.
- V-IT-6 (G-5): `cli-overview.md` lists `ark cleanup`; `ark-agent.md` lists `task resume`, `task discard`, `spec import`; `config-toml.md` has a `[workspace]` section and the corrected `post_create` default.

[**Failure / Robustness**]

- V-F-1: Every `--help` excerpt quoted in the book is diffed against live binary output; mismatches corrected toward the binary (C-1).

[**Edge Cases**]

- V-E-1 (C-5): `grep -rE "ark run|ark benchmark|feat/ark-run|\brevise\b|\bfinalize\b" docs/book/src` returns nothing.
- V-E-2 (C-3): Every file under `docs/book/src/**/*.md` (except `SUMMARY.md` itself) appears in `SUMMARY.md`.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-2 |
| G-2 | V-IT-3 |
| G-3 | V-IT-4 |
| G-4 | V-IT-5 |
| G-5 | V-IT-6 |
| G-6 | V-IT-1 |
| C-1 | V-F-1 |
| C-2 | V-IT-1 |
| C-3 | V-E-2 |
| C-4 | V-IT-2 |
| C-5 | V-E-1 |
