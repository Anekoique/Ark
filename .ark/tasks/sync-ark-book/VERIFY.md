# `sync-ark-book` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `sync-ark-book`
> Target Task: `sync-ark-book`
> Tier: `standard`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Standard tier: `/ark:commit` warns on any `PENDING` but proceeds.

---

## Project Spec Compliance

> The project SPECs under `specs/project/` (`LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`) are Rust source-code conventions. This task ships no Rust — it edits markdown under `docs/book/src/` only. The convention SPECs are therefore non-applicable to the changed files. Index integrity is still checked because it is a structural invariant independent of language.

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `specs/project/INDEX.md` rows `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; the on-disk children match. This task did not add or remove any project SPEC.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: N/A — this task changed no project SPEC; the four leaves are unmodified (`git status` shows no edits under `specs/project/`). Their conformance is out of this task's scope.
  - `LAYOUT.md` — N/A (unchanged)
  - `rust/COMMENTS.md` — N/A (unchanged; governs Rust source, none shipped)
  - `rust/STYLE.md` — N/A (unchanged; governs Rust source, none shipped)
  - `rust/ERRORS.md` — N/A (unchanged; governs Rust source, none shipped)

## Related Feature Spec Compliance

> The PRD lists these as *sources of truth to describe*, not SPECs to modify. Each item below = does the book describe the feature faithfully against the SPEC / binary / workflow.md?

- [x] specs/features/ark-research/SPEC.md: PASS — research tier documented as the 4th tier with the `research → committed → archived` lifecycle in `tiers.md:10,29`, `lifecycle.md:43`, `introduction.md:13`; binary confirms `task new --tier` accepts `research` and `task promote` is rejected for research.
- [x] specs/features/subagent-support/SPEC.md: PASS — `workflow/subagents.md` documents the three reserved stems, per-platform paths (`.claude/agents/*.md`, `.codex/agents/*.toml`, `.opencode/agents/*.md` — all verified to ship), the reserved-overwrite-on-init/upgrade/load rule, and the embedded-vs-research-tier distinction.
- [x] specs/features/workspace/SPEC.md: PASS — `workflow/workspace.md` documents `.ark/.developer`, `ark init --developer`, `--no-developer` → `MissingIdentity`, `[workspace] developer` fallback, `/ark:record`, and rotation at `journal_max_lines`; every claim verified against `ark init --help` / `ark agent workspace --help` and `templates/ark/config.toml`.
- [x] specs/features/detachable-feature-spec/SPEC.md: PASS — `workflow/specs.md:49-65` documents the `[**SPEC Path**]` block, slash-separated recursive paths, leaf-to-root INDEX upsert, single-segment-reproduces-flat, and `/ark:extract-spec` → `ark agent spec import` brownfield path; `ark agent spec import` confirmed in the binary. (One stale attribution to *archive* instead of *commit* tracked in V-001.)
- [x] specs/features/project-spec/SPEC.md: PASS — `workflow/specs.md:5,10-39` describes the user-authored `specs/project/<name>/SPEC.md` layer, read-every-task semantics, and the add-a-SPEC procedure; consistent with the actual `specs/project/` tree.
- [x] specs/features/ark-context/SPEC.md: PASS — `reference/cli-overview.md` and the workflow pages describe `ark context` scopes (`session`/`phase`/`record`), phases, and `--format json`; consistent with `ark --help` and workflow.md §CLI surfaces.

## PRD Constraints

> One item per PRD `[**Outcome**]` bullet.

- [x] Tiers documents four tiers; research path + promotion-excludes-research note: PASS — `tiers.md:3,10` (four-tier table incl. Research) and `tiers.md:41` (research excluded from promotion).
- [x] Lifecycle covers research path + names subagent hand-off at REVIEW/VERIFY: PASS — `lifecycle.md:43` (research exception), `:75` (reviewer pick), `:103` (verifier pick), all matching workflow.md §REVIEW/§VERIFY.
- [x] Specs describes recursive feature-SPEC paths + detachable SPEC + `/ark:extract-spec`: PASS — `specs.md:6,49-65`. (SPEC-promotion *timing* drift on the same page tracked in V-001.)
- [x] New Subagents page documents researcher/reviewer/verifier + reserved stems + embedded-vs-research distinction: PASS — `workflow/subagents.md`, wired in `SUMMARY.md`.
- [x] New Workspace page documents `.ark/workspace/`, `.ark/.developer`, `ark init --developer`, `/ark:record`, `[workspace]`: PASS — `workflow/workspace.md`, wired in `SUMMARY.md`.
- [x] CLI Overview lists `ark archive` accurately + correct visible/hidden split: PASS — `cli-overview.md:7-20` matches `ark --help` exactly (8 visible commands incl. `ark cleanup`; `ark agent` the sole hidden command).
- [x] `.ark/config.toml` gains `[workspace]` + corrects `post_create` default: PASS — `config-toml.md:14-15,31,43` show `[workspace] journal_max_lines = 2000` and `post_create = ["git submodule update --init --recursive"]`, both matching `templates/ark/config.toml`.
- [x] Slash-command surface lists all eight commands: PASS — `quick-start.md:64` and `platforms/claude-code.md`/`codex.md`/`opencode.md` all list `quick`, `design`, `commit`, `research`, `resume`, `discard`, `extract-spec`, `record`; all eight verified present under `templates/{claude/commands/ark,codex/skills,opencode/commands/ark}`.
- [x] Contributing → Workspace Layout `ark-core` module map matches the tree: PASS — `workspace-layout.md:28-67` adds `cleanup.rs`, `agent/workspace/` submodules, recursive `agent/spec/`, and `task/commit`; an illustrative high-level map (abstracts `io/fs/*` and `state/checkout/*`), consistent with CLAUDE.md's own terse style. See Notes.
- [x] `SUMMARY.md` updated; `mdbook build` succeeds with no broken intra-book links: PASS — `SUMMARY.md` wires both new pages into the Workflow chapter; `mdbook build` exits 0; all 73 intra-book `.md` links resolve (verified manually — see V-IT-1/C-2).
- [x] Nothing documents `feat/ark-run` / `feat/ark-benchmark`: PASS — see V-E-1; grep returns nothing.

## Plan Fidelity

> One item per `00_PLAN.md` Goal (`G-N`).

- [x] G-1: Workflow chapter documents four tiers including research, matching workflow.md: PASS — V-IT-2 met. `tiers.md` and `lifecycle.md` both carry research entries; `grep -ri "three tiers"` returns nothing.
- [x] G-2: Book documents the researcher/reviewer/verifier subagents and where each is offered: PASS — V-IT-3 met. `workflow/subagents.md` exists, is in `SUMMARY.md`, names all three stems; REVIEW/VERIFY hand-off documented in `lifecycle.md`.
- [x] G-3: Book documents workspace journals + developer identity as a first-class concept: PASS — V-IT-4 met. `workflow/workspace.md` exists, is in `SUMMARY.md`, documents `.ark/.developer` + `/ark:record`.
- [x] G-4: Specs chapter describes recursive feature-SPEC paths, not the old flat model: PASS — V-IT-5 met. `specs.md` references `[**SPEC Path**]` + recursive leaf-to-root INDEX behavior; the flat-only model is gone. (Promotion-timing drift on the same page tracked in V-001 — does not negate the recursive-path delivery.)
- [x] G-5: Reference + platform pages list every shipped command and slash command accurately: PASS — V-IT-6 met. `cli-overview.md` lists `ark cleanup`; `ark-agent.md` lists `task resume`/`task discard`/`spec import`; `config-toml.md` has `[workspace]` + corrected `post_create`. All cross-checked against the binary.
- [x] G-6: `mdbook build docs/book` succeeds with zero broken intra-book links: PASS — V-IT-1 met. Build exits 0; 73/73 intra-book `.md` links resolve; every `src/**/*.md` page is listed in `SUMMARY.md`.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — this task modified no feature SPEC body. The only `specs/features/` change is whitespace/column re-alignment of the managed-block table in `specs/features/INDEX.md` (no rows added/removed, no content changed). See Notes.

## Findings

## Severity Summary: 0 CRITICAL · 1 HIGH · 2 MEDIUM · 1 LOW — all 4 findings FIXED (re-validated: build clean, links resolve, no archive-promotion / `.current` / `phase="design"` / `<name>` drift remains)
## Verification: build PASS · tests N/A (docs-only; no test suite for the book) · lint N/A (no mdbook linter configured) · format N/A (no markdown formatter configured)

> Verification dimensions: `mdbook build docs/book` exits 0 (build). No test/lint/format toolchain is configured for the book (no `mdbook-linkcheck`, no markdown linter in `book.toml` or CI for `docs/book`); link integrity was instead checked manually (73/73 resolve) and recorded under V-IT-1.

### V-001 SPEC promotion / CHANGELOG wrongly attributed to *archive* instead of *commit*

- **Severity:** HIGH
- **Location:** `docs/book/src/workflow/tiers.md:27`; `docs/book/src/workflow/specs.md:87`; `docs/book/src/workflow/specs.md:95`; `docs/book/src/workflow/lifecycle.md:65`
- **Problem:** Four passages say feature-SPEC promotion (and the `[**CHANGELOG**]` append + `Promoted`-column rewrite) happen at *archive*. The canonical source of truth says otherwise: `.ark/workflow.md:254` states archive is "Side-effect-free — no SPEC promotion (already happened on commit)" and `.ark/workflow.md:235,320` place extraction/CHANGELOG at **commit**. The binary agrees (`ark agent task commit --help`: "deep-tier SPEC extract … covering work + … (deep) SPEC + features INDEX"; `ark agent task archive --help`: "Side-effect-free move"). The same task already corrected this on the same pages — `specs.md:43` now says "When a deep-tier task **commits**", `lifecycle.md:33,111,114` and `first-task.md:73` say "on commit" — so each page is now internally self-contradictory. Exact strings: `tiers.md:27` "On archive, the final PLAN's `## Spec` section is promoted"; `specs.md:87` "`ark agent task archive` appends a `[**CHANGELOG**]` entry … rewrites the `Promoted` column"; `specs.md:95` "Deep-tier task archive promotes it."; `lifecycle.md:65` "It's copied verbatim to `specs/features/<name>/SPEC.md` on archive (deep tier)."
- **Why it matters:** This is precisely the reader-misleading drift the PRD's `[**Why**]` set out to eliminate. A reader is told both that archive is side-effect-free and that archive promotes the SPEC — they cannot tell which is true, and the wrong half describes a behavior the binary does not have.
- **Recommendation:** Change all four to attribute promotion/CHANGELOG/`Promoted`-rewrite to **commit** (`ark agent task commit`), matching the corrected `specs.md:43` and `lifecycle.md:111`. Suggested edits: `tiers.md:27` "On commit, the final PLAN's `## Spec` …"; `specs.md:87` "`ark agent task commit` appends a `[**CHANGELOG**]` entry …"; `specs.md:95` "Deep-tier task commit promotes it."; `lifecycle.md:65` "… copied verbatim to `specs/features/<path>/SPEC.md` on commit (deep tier)."
- **Resolution:** FIXED — all four passages now attribute SPEC promotion / CHANGELOG / `Promoted`-rewrite to commit: `tiers.md:27` "On commit, …"; `specs.md:87` "`ark agent task commit` appends …"; `specs.md:95` "Deep-tier task commit promotes it."; `lifecycle.md:65` "… on commit (deep tier)." plus the ASCII COMMIT box and `:111` prose. Grep for archive-promotion phrasing returns nothing; rebuild clean.

### V-002 Book describes the retired `.ark/tasks/.current` slug pointer instead of the `.ark/.state.toml` focus model

- **Severity:** MEDIUM
- **Location:** `docs/book/src/reference/ark-agent.md:118`; `docs/book/src/workflow/tiers.md:45`; `docs/book/src/workflow/tiers.md:51`
- **Problem:** `ark-agent.md:118` states "Every subcommand that takes `--slug` defaults to `.ark/tasks/.current` when omitted." `tiers.md:45,51` say two tasks "would collide on `.ark/tasks/.current`" and "The parent checkout's `.current` is untouched." On `main`, slug resolution is the focus model in `.ark/.state.toml` (`.ark/workflow.md:348-384`: slug resolves from `[focus]`, errors `NoFocus`, set via `task resume`). `.ark/tasks/.current` is now a **legacy** file kept only for self-healing migration — `crates/ark-core/src/state/checkout/migrate.rs:1` ("Self-healing migration from the legacy `tasks/.current` pointer") and `layout.rs:40` retain it solely for that path. This drift is pre-existing (not introduced by this task) and not enumerated in the PRD `[**Outcome**]`, but the PRD's stated purpose is "the book describes Ark as it exists on `main` today," and these statements describe a mechanism `main` no longer uses.
- **Why it matters:** A reader who reaches for `ark agent` (the page's whole audience) is told the wrong resolution mechanism and would not learn about `NoFocus` / `task resume`. The two `tiers.md` mentions misdescribe how parallel tasks avoid collision.
- **Recommendation:** Replace `.ark/tasks/.current` references with the focus model: `--slug`-omitting verbs resolve the focused slug from `.ark/.state.toml`'s `[focus]` (set by `task new` / `task resume`), erroring `NoFocus` when none is bound; reframe the `tiers.md` collision prose around one-focus-per-checkout. Cross-check against workflow.md §"Focus model".
- **Resolution:** FIXED — `ark-agent.md` Defaults section rewritten to the focus model (`.ark/.state.toml` `[focus]`, `NoFocus`, `task resume`); `NoFocus` added to the error list; `tiers.md` parallel-tasks prose reframed around one-focus-per-checkout (worktree owns its own `.state.toml`). No `.current` references remain in `docs/book/src`.

### V-003 Reopen instruction sets `phase = "design"` where the source of truth says `phase = "verify"`

- **Severity:** MEDIUM
- **Location:** `docs/book/src/workflow/lifecycle.md:126`
- **Problem:** "Reopen. Move the archived dir back to `.ark/tasks/<slug>/` and reset `phase = "design"` + clear `archived_at`." Both `.ark/workflow.md:256` and `:370` say a reopened task resets to `phase = "verify"` (so it lands back at the commit-able gate), not `design`. Pre-existing drift, outside the edited region.
- **Why it matters:** A reader following this hand-edit would push a finished task all the way back to DESIGN and then be unable to commit without re-walking PLAN/EXECUTE/VERIFY; the intended escape hatch is to land at VERIFY.
- **Recommendation:** Change `phase = "design"` to `phase = "verify"` to match workflow.md.
- **Resolution:** FIXED — `lifecycle.md:126` reopen now resets `phase = "verify"` (lands at the commit-able gate), matching workflow.md:256,370. No `phase = "design"` remains in the book.

### V-004 `<name>` vs `<path>` placeholder inconsistency in the SPEC destination

- **Severity:** LOW
- **Location:** `docs/book/src/workflow/lifecycle.md:33`; `docs/book/src/workflow/lifecycle.md:65`; `docs/book/src/workflow/lifecycle.md:111`
- **Problem:** `lifecycle.md` writes the SPEC destination as `specs/features/<name>/SPEC.md` while the now-recursive model uses `<path>` (e.g. `specs.md:6,43,59`, `tiers.md:27`, `ark-agent.md:40,110`, `first-task.md:73` all use `<path>`). Technically `<name>` is the valid single-segment case, so this is cosmetic, not wrong — but it reads as a stale placeholder against the rewritten Specs chapter and is inconsistent within the same chapter the task touched.
- **Why it matters:** Minor reader confusion; the recursive-path model is the point of G-4 and the placeholder should be consistent across the chapter.
- **Recommendation:** Normalize `<name>` → `<path>` in `lifecycle.md:33,65,111` for consistency with the rest of the Workflow chapter. (Bundle with V-001's `lifecycle.md:65` fix.)
- **Resolution:** FIXED — `lifecycle.md` SPEC destination normalized to `specs/features/<path>/SPEC.md` at all three sites (ASCII COMMIT box, PLAN rule, COMMIT purpose). No `specs/features/<name>` placeholder remains.

## Notes

- **Build / link integrity.** `mdbook build docs/book` exits 0 with no warnings. `mdbook-linkcheck` is not installed and `book.toml` configures no link-check backend, so the default build does not flag broken links. Link integrity was verified manually: 73/73 intra-book `.md` links (with or without anchors) resolve to existing files, and every `src/**/*.md` page (26 total, incl. the two new pages) is listed in `SUMMARY.md` (C-2, C-3, V-E-2 all satisfied).
- **Forbidden-content sweep (C-5 / V-E-1).** `grep -rnE "ark run|ark benchmark|feat/ark-run|\brevise\b|\bfinalize\b"` and `grep -rn "ark-benchmark|feat/ark-benchmark|ark-run"` over `docs/book/src` both return nothing. `grep -ri "three tiers"` also returns nothing (C-4 / V-IT-2). The unmerged `feat/ark-run` / `feat/ark-benchmark` work and the unshipped `revise`/`finalize` ops are absent.
- **Binary cross-check (C-1 / V-F-1).** Built `./target/debug/ark` and diffed every command/flag/default named in the book against live `--help` and `templates/ark/config.toml`. All match: 4 tiers, `task new --tier` accepts `research`, `ark cleanup` visible, `ark agent task` includes `resume`/`discard`, `ark agent spec` includes `import`, `ark agent workspace` exists, `[workspace] journal_max_lines = 2000`, `[worktree] post_create = ["git submodule update --init --recursive"]`. The `.codex/hooks.json` shown in scaffold trees is a generated artifact (`layout.rs:128`, `init.rs:292`), not a shipped template — correctly documented.
- **Module map abstraction.** `contributing/workspace-layout.md`'s `ark-core` map is an illustrative high-level tree, not a 1:1 mirror: it collapses `io/fs/{mod,hook,managed_block,walk}.rs` into a single `io/fs.rs` line, omits `state/checkout/`, and shows `upgrade.rs` for the `upgrade/` dir. This matches CLAUDE.md's own terse style and the PRD-required additions (`cleanup.rs`, `workspace/`, recursive `spec/`, `task/commit`) are all present. Not flagged as a finding — intentional terseness, consistent with the canonical map.
- **`specs/features/INDEX.md` change.** The working tree carries a whitespace/column-realignment of the `<!-- ARK:FEATURES -->` managed block in `.ark/specs/features/INDEX.md` (no rows or content changed). It is outside the book and outside this task's declared scope; it is cosmetic and not a SPEC modification. Worth confirming the user intends to stage it with this docs commit (or revert it), since managed blocks are normally owned by `ark agent spec register`.
- **Voice/structure (C-6).** Spot-checked the diffs for `lifecycle.md`, `tiers.md`, `specs.md`, `introduction.md`, `claude-code.md`: edits are additive and surgical (research note, reviewer/verifier hand-off, recursive-paths section, four-tier table row) — existing tone and section structure preserved, facts changed not rewritten wholesale.
