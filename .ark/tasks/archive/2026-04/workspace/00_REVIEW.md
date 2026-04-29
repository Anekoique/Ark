# `workspace` REVIEW `00`

> Status: Open
> Feature: `workspace`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 4
- Non-Blocking Issues: 6

## Summary

The plan is coherent, well-aligned with the PRD, and respects the conventions inherited from `ark-agent-namespace`, `ark-context`, and `worktree-support`. The architecture (one-way `task → workspace` coupling, parent-write invariant, opt-in identity) is sound, the call graphs match real ark-core APIs (verified `PathExt`, `Layout`, `update_managed_block`, `walk_files_excluding`, `task_archive` flow), and the `## Spec` section is largely self-contained.

Blocking issues cluster around: (1) `task_archive` integration is gated only on deep tier in the existing code but the PLAN suggests inserting `record_task` after spec promotion universally — placement is ambiguous and contradicts a stated outcome; (2) auto-record on archive contradicts itself between PRD ("unconditional"), plan G-2 ("opt-in via missing identity"), and config G-8 (`auto_record_on_archive: bool`) — the toggle's semantics are inconsistent; (3) C-7's "file-level path equality skip" needs a concrete API change to `walk_files_excluding` since it currently uses `Path::starts_with` (works, but PLAN should call out that the file path itself is a valid prefix); (4) Acceptance Mapping has cross-reference errors (G-1's listed validations don't actually validate G-1's claims). Non-blocking issues span a few inconsistencies, missing call graph step for index re-render in init, and a parser ambiguity in `parse_entries`.

Trade-off calls below favor the executor's defaults except T-3 (need stricter justification) and T-5 (re-affirm with a concrete bound).

---

## Findings

### R-001 Auto-record toggle semantics are internally inconsistent

- Severity: HIGH
- Section: `[**Goals**] G-2` / `G-8` and `PRD [**Outcome**]`
- Problem:
  The PRD says auto-record is "unconditional when identity is set (no opt-out flag — covered by `workspace.toml` config field for future tuning)". G-2 says missing `.developer` makes it a no-op. G-8 introduces `auto_record_on_archive: bool` (default `true`) as a real opt-out *now*, with `record_task` returning `SkippedDisabled` when `false`. C-11 reaffirms `auto_record_on_archive` as a runtime-checked toggle. T-4 also takes it as given.

  This is three different positions: (a) PRD says "no opt-out flag" and config is for future use; (b) plan implements a working opt-out today; (c) workflow doc and slash command text don't mention the toggle.
- Why it matters:
  Reviewers and future implementers will read these as contradictory. If the toggle ships, the PRD's language ("no opt-out flag") is wrong. If it doesn't ship, G-8/C-11/T-4 must be revised. Either way, the resulting feature SPEC (extracted on archive) carries the contradiction into project memory.
- Recommendation:
  Decide one path and harmonize:
  - Recommended: keep `auto_record_on_archive` as a real toggle (it's cheap, the plumbing is already specified, and the user explicitly asked for "configure for user" in the PRD brainstorm). Revise PRD's "no opt-out flag" line; clarify in G-2 that "unconditional" means "no per-archive flag, but the global config may disable". Add NG entry: "no per-archive `--no-record` flag — only the global config".
  - Alternative: drop `auto_record_on_archive` from G-8/C-11/T-4 and `WorkspaceConfig`; remove `SkippedDisabled` enum variant.

### R-002 `record_task` placement vs deep-tier gate

- Severity: HIGH
- Section: `[**Goals**] G-7` / `[**Architecture**]` call graph for `task archive`
- Problem:
  G-7 says auto-record runs "After `spec_extract`+`spec_register` on deep tier". The implied placement (lines 96–111 of `task/archive.rs`) is *inside* the `if tier == Tier::Deep` block. But the PRD says auto-record fires on every `task archive`, and the call graph in `[**Architecture**]` shows `record_task` running unconditionally after the existing flow, with no tier gate.

  Looking at actual `archive.rs`:
  - Line 95–111: `if tier == Tier::Deep { spec_extract; spec_register; }`
  - Line 113–118: `.current` clearing (always runs)
  - The natural insertion point for a tier-agnostic `record_task` is *after* line 118 (or before, between line 111 and 113), NOT inside the deep block.

  The PLAN is ambiguous about where to insert and silently implies tier-agnostic behavior in the call graph. Standard- and quick-tier archives need session entries too (the PRD doesn't restrict to deep).
- Why it matters:
  - Wrong placement means quick/standard-tier archives never get a journal entry — half the feature's value is lost.
  - The placement also affects the `archive_path` vs. `worktree_path` decision in commit collection (G-6): if `record_task` runs after rename, `worktree_path` from `task.toml` may already be a stale path under `.ark/worktrees/...` that may or may not still exist on disk depending on whether the user cleaned up the worktree.
- Recommendation:
  - State explicitly in G-7: `record_task` runs **regardless of tier**, after the `.current` clearing block (or before — but document the choice). Reference line numbers tentatively, not strictly.
  - Add to G-6: when running from inside an archived task, prefer `worktree_path` if the dir still exists (the worktree wasn't cleaned up), fall back to `archive_path`. State the precedence explicitly.
  - Add a new test: `quick_tier_archive_records_session` and `standard_tier_archive_records_session`.

### R-003 Acceptance Mapping is mis-cited for G-1

- Severity: HIGH
- Section: `[**Validation]] [**Acceptance Mapping**]`
- Problem:
  The mapping for `G-1` (which says "New `commands/agent/workspace/` module with public API: `workspace_init`, `workspace_record`...") cites `V-UT-1, V-UT-2, V-UT-3, V-UT-4`. But:
  - V-UT-1 validates `validate_developer_name` (covers C-3, not G-1's "module exists with public API")
  - V-UT-2 validates `read_developer_name`
  - V-UT-3 validates `WorkspaceConfig`
  - V-UT-4 validates `render_entry`

  None of these *actually* exercise G-1's module-public-API claim. The right validation for G-1 is the integration test "calling `workspace_init` and `workspace_record` from a binary import compiles and works" — which is implicit in V-IT-1, V-IT-3.

  Also: G-2 cites only V-F-1 and V-F-4 — but G-2 includes the stronger claim "`ark init` does NOT prompt for identity and does NOT create `.developer` or `.ark/workspace/`". Neither V-F-1 nor V-F-4 verify this. Need a test asserting `ark init` post-condition contains no `.developer` and no `.ark/workspace/`.

  G-10 cites V-IT-1 / V-IT-3 — but G-10 specifies `TargetArgs` plumbing and "`record` resolves layout via `TargetArgs::resolve_with_discovery`". Need a CLI-shape test.
- Why it matters:
  The Plan-gate rule says every Goal must map to ≥1 Validation. The mapping is currently formally complete but semantically wrong in several rows. Future reviewers / verifiers will read the mapping as authoritative; if it lies about coverage, drift will go unnoticed.
- Recommendation:
  - Re-thread G-1 → V-IT-1 (calls `workspace_init` end-to-end) + V-IT-3 (calls `workspace_record`).
  - Add a unit test V-UT-9: `ark_init_does_not_create_workspace_or_developer` covering G-2's negative claim. Add to mapping.
  - Add a CLI-shape test V-UT-10 verifying `ark agent workspace init --name ...` and `ark agent workspace record [--title ...]` parse correctly under the existing `TargetArgs` flatten. Add to G-10 mapping.
  - Audit other G→V mappings for similar semantic mis-cites (G-13, G-14 cite V-IT-1 which only checks creation, not template content).

### R-004 `## Spec` references `[**Architecture**]` lines 96–111 — not self-contained

- Severity: HIGH
- Section: `[**Goals**] G-7`
- Problem:
  G-7 says: "After `spec_extract`+`spec_register` on deep tier (existing pattern, lines 96–111 of `task/archive.rs`), insert: ...". This embeds an external-file line-number reference inside `## Spec`. Per workflow.md §4 and worktree-support's `spec-extract-self-contained` archived task: `## Spec` is copied verbatim to `specs/features/<name>/SPEC.md`. The line-number reference in the extracted SPEC will rot the moment `task/archive.rs` is touched.

  The promoted SPEC will say "lines 96–111 of `task/archive.rs`" — a citation that means nothing in 6 months.
- Why it matters:
  Workflow's HIGH-rejection criterion: "if the latest PLAN's `## Spec` references prior iterations instead of restating in full". This is a sibling case: external file-line references inside `## Spec` are also unstable.

  Additionally, `## Spec` references `[**Related Specs**]` of PRD via "G-1 (worktree-support's NG-1)" — that's fine because it cites the SPEC by name, not by line. But `lines 96–111` is brittle.
- Recommendation:
  - Replace "lines 96–111 of `task/archive.rs`" in G-7 with a structural description: "after the deep-tier SPEC promotion block and the `.current` cleanup, but before the `Ok(...)` return."
  - Audit the rest of `## Spec` for similar brittle citations. (None found on a scan, but worth a final pass.)

### R-005 `parse_entries` parser definition gap

- Severity: MEDIUM
- Section: `[**Goals**] G-5` / `[**Data Structure**] journal.rs`
- Problem:
  G-5 specifies the entry markdown shape and an anchor pattern `^## Session (\d+):`. `parse_entries(text: &str) -> Vec<ParsedEntry>` is declared as the inverse, used by `index::rerender` to rebuild the sessions table. But:
  - `ParsedEntry` is referenced but never defined in `[**Data Structure**]`. Field set is unspecified.
  - The end boundary of an entry is undefined. Is it the next `^## Session N:`? The next `^## ` (any H2)? A `### Next Steps` section? EOF?
  - The summary subsection contains arbitrary markdown — could include code blocks with `## ` lines that the parser would misread as next-entry start.
  - Whether `parse_entries` reads from a single journal file or all journals is implied (per the `index::rerender` pattern) but never stated.
- Why it matters:
  Without a precise parser predicate, `index::rerender` is not implementable from the SPEC. The corruption mode is silent — a misparsed entry produces a wrong sessions table. Mirrors `ark-context` C-19/C-20 (artifact iteration rule and related-specs parser): both are stated as precise predicates.
- Recommendation:
  - Add `ParsedEntry` struct to `[**Data Structure**]` with explicit fields (`session_number`, `title`, `date`, `kind`, `slug`, `branch`, `commits_count`).
  - Add a constraint C-19: "Entry-boundary predicate. An entry begins at `^## Session (\d+):\s+(.*)$` and ends at the next line matching the same pattern OR EOF. Lines inside fenced code blocks (between matched ` ``` `) are ignored. Any hand-edits inside the entry body are tolerated but must not introduce a `## Session NN:` heading."
  - Reference test: V-UT-5 (already exists) should round-trip across multiple entries with code blocks and edited summaries.

### R-006 `walk_files_excluding` file-level skip

- Severity: MEDIUM
- Section: `[**Constraints**] C-7`
- Problem:
  C-7 says: "Implementation: file-level path equality skip, additive to the existing `walk_files_excluding(skip = [worktrees_dir])` directory skip."

  Verified `walk_files_excluding` source: it uses `path.starts_with(p.as_ref())`. A file path *does* match `starts_with(file_path)` (every path is a prefix of itself), so passing `layout.developer_file()` in `skip_under` *will* exclude the file. So the implementation works as-is.

  However, the PLAN's wording "file-level path equality skip, additive to the existing directory skip" suggests two separate code paths or a special-case branch. No new API needed: just append `developer_file()` to the existing `skip` array in `unload.rs`.

  Also: `capture_orphan_hook_entries` (`unload.rs:156`) has its own walk loop with its own skip array. Both must include the developer file, or capture leaks via Stage B.
- Why it matters:
  The PLAN's wording undersells the simplicity (no API change) and oversells the work. More importantly, it omits the second walk site. Without specifying both, an implementer could miss `capture_orphan_hook_entries`.
- Recommendation:
  - Reword C-7: "Add `layout.developer_file()` to *both* `walk_files_excluding` skip arrays in `unload.rs` (the Stage A capture loop at `unload` and the Stage B `capture_orphan_hook_entries` loop). Existing helper takes `&[P: AsRef<Path>]` and matches via `Path::starts_with`; a single file path is a valid skip prefix."
  - Add explicit test: `unload_skips_developer_file_in_orphan_walk` (covers Stage B) in addition to the existing V-IT-7.

### R-007 `record_task` worktree-side commit collection edge case

- Severity: MEDIUM
- Section: `[**Goals**] G-6` / `[**Architecture**]` call graph for `task archive`
- Problem:
  Call graph: `commits = run_git(["log", "{base}..HEAD", "--oneline"], task_cwd)` where `task_cwd = worktree_path.unwrap_or(archive_path)`.

  Failure mode: the `archive_path` is *inside the worktree's `.ark/`*. After `task_dir.rename_to(&archive_path)` (line 83 of `archive.rs`), the task lives under `<worktree>/.ark/tasks/archive/YYYY-MM/<slug>/`. The current `git log` HEAD inside the worktree still points at the worktree's branch (e.g. `feat/workspace`). So `<base>..HEAD` will include all branch commits — fine.

  But if `task_cwd = archive_path` (because `worktree_path` was None and we're at the parent-checkout flow without a worktree), `archive_path = parent/.ark/tasks/archive/YYYY-MM/<slug>/`. `git log` from there picks up `parent`'s `.git` via discovery — still works.

  The corner: `worktree_path.unwrap_or(archive_path)` — if `worktree_path` is `Some(p)` but `p` no longer exists on disk (user pre-cleaned the worktree), the `git log` call fails with a working-dir-not-found error. Needs a fallback chain: try `worktree_path` if exists, else `archive_path`, else the project root.

  Also: when `task_cwd` is the *archive_path*, `--oneline -n 20` from a non-worktree dir (just a regular subdir of the parent repo) *is fine*, but the commits you collect are the *parent branch's* commits as of now, not the archived task's branch's commits. For a task that was never on a worktree (`base_branch = None`, no `--worktree`), this is the right behavior. For a task that *was* on a worktree but the worktree dir is gone, it's wrong.
- Why it matters:
  Auto-record produces a misleading commits table when the worktree was cleaned up before archive. Not catastrophic, but the journal claims commits the task didn't actually contribute.
- Recommendation:
  - Add to G-6 a fallback chain: `worktree_path` (if exists on disk and `is_dir()`) → `archive_path` (if exists) → parent project root. Document each fallback's commits-correctness expectation.
  - Add edge test V-E-7: archive after worktree cleanup → commits from `archive_path` (parent's HEAD), entry annotated or not — your call.

### R-008 `workspace_init` call graph misses index re-render

- Severity: LOW
- Section: `[**Architecture**]` call graph for `workspace init`
- Problem:
  The call graph for `workspace_init` seeds `index.md` and `journal-1.md`, but doesn't call `index::rerender`. The seeded index has empty managed blocks. That's fine *initially* (no sessions = no rows), but if the user runs `workspace init`, then deletes the developer dir manually, then re-inits with the same name — they'll have a half-scaffolded state where journals from a prior life exist (impossible by C-5, but worth being explicit). More practically, if `seed_index` writes the literal template (with markers but empty body), and then a future record runs `update_managed_block`, the markers must already be in place — which the template provides. So: *technically OK*, but the call graph could be clearer.
- Why it matters:
  Minor doc/clarity issue. Doesn't block.
- Recommendation:
  Add a comment in the call graph: `// index has empty managed blocks; first record populates them via update_managed_block`.

### R-009 `parent.rs` Layout::clone dependency

- Severity: LOW
- Section: `[**API Surface**]` and `[**Architecture**]` parent.rs
- Problem:
  `resolve_parent_layout(layout: &Layout) -> Result<Layout>` returns a fresh `Layout` (or a clone). Verified `Layout::new(impl Into<PathBuf>)` — so construction is fine. But `Layout` doesn't currently `derive(Clone)`. Quick check via `grep -n "impl.*Clone.*Layout\|derive.*Clone\|#\[derive" crates/ark-core/src/layout.rs` — needs verification by the executor before relying on `layout.clone()` in the implementation.
- Why it matters:
  Implementation detail; would be caught at compile time.
- Recommendation:
  Add to Phase 1 implementation notes: "verify or add `#[derive(Clone)]` to `Layout`."

### R-010 `validate_developer_name` accepts pure-numeric names like `123`

- Severity: LOW
- Section: `[**Constraints**] C-3` / `[**Validation]] V-UT-1`
- Problem:
  C-3 regex `^[A-Za-z0-9_-]{1,40}$` accepts `123` as a valid developer name. While syntactically fine, pure-numeric names are filesystem-unfriendly on some platforms (less an issue today, more of a fingerprint-of-laziness flag). Trellis appears to enforce no such rule — purely informational.
- Why it matters:
  Not a real bug. Future support requests if a user picks `0` and confuses tooling.
- Recommendation:
  Optional: require name to start with a letter (`^[A-Za-z][A-Za-z0-9_-]{0,39}$`). Or leave as-is and note in C-3: "all-numeric names are accepted but discouraged."

---

## Trade-off Advice

### TR-1 Slash command shape

- Related Plan Item: `T-1`
- Topic: Friction vs Safety
- Reviewer Position: Prefer Option A (one-shot)
- Advice:
  Confirm executor's lean. One-shot is correct.
- Rationale:
  Recording is append-only, the entry is human-readable markdown, and the user's slash-command invocation is itself the consent. A confirmation step adds friction that scales linearly with usage frequency (the feature is meant to be invoked daily). The recovery cost (hand-edit the journal) is low.
- Required Action:
  Keep T-1 Option A. No revision.

### TR-2 `workspace.toml` placement

- Related Plan Item: `T-2`
- Topic: Symmetry vs Encapsulation
- Reviewer Position: Prefer Option A (sibling of `worktree.toml`)
- Advice:
  Keep `.ark/workspace.toml`.
- Rationale:
  Symmetry with `worktree.toml` matters for discoverability (`ls .ark/*.toml` shows the user what configs exist). Burying config inside `.ark/workspace/.config.toml` hides it. Also `workspace_dir` itself might one day be a configurable setting (`workspace_dir = ".ark/workspace"` future-proofing) — putting the config inside the dir it configures is a chicken-and-egg.
- Required Action:
  Keep T-2 Option A. No revision.

### TR-3 Parent-root resolution method

- Related Plan Item: `T-3`
- Topic: Robustness vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  Option A (`git rev-parse --git-common-dir`) is correct for the worktree case, but the PLAN doesn't address a real failure mode: what if the parent repo was cloned without `--git-dir` magic and `--git-common-dir` returns a relative path on some platforms? Or what if the user runs `ark agent workspace record` outside *any* git repo? G-9 step 3 says "no `.git` at all → return `layout.clone()`" — fine, but the detection ("has no .git") is implementation-defined.
- Rationale:
  Robustness matters more than simplicity here, but the spec needs to nail the detection algorithm. The current text leaves room for "well, `.git` is a file because of submodules" misinterpretation.
- Required Action:
  Expand G-9 with explicit detection:
  - Step 1: `let dot_git = layout.root().join(".git")`
  - Step 2a: if `dot_git.is_dir()` → regular checkout, return `layout.clone()`.
  - Step 2b: if `dot_git.is_file()` → worktree pointer, run `git rev-parse --git-common-dir` from `layout.root()`. Output is a path relative to `layout.root()` or absolute. Canonicalize via `PathBuf::canonicalize` (trailing `/.git` strip is OS-dependent).
  - Step 2c: if `dot_git` does not exist → return `layout.clone()` (non-git project; record locally).
  - Step 2d: if `dot_git` exists but is neither file nor dir (symlink, pipe) → `Error::ParentRootResolution { reason: "unrecognized .git type" }`.

### TR-4 Auto-record toggle plumbing location

- Related Plan Item: `T-4`
- Topic: Cohesion
- Reviewer Position: Prefer Option A (in `workspace.toml`) — agree with executor
- Advice:
  Keep `auto_record_on_archive` in `workspace.toml`. (Subject to R-001's decision on whether the toggle exists at all.)
- Rationale:
  Workspace concerns belong in workspace config. `worktree.toml` is for worktree behavior.
- Required Action:
  Keep, contingent on R-001 resolution. If R-001 removes the toggle, T-4 becomes moot.

### TR-5 Index re-render strategy

- Related Plan Item: `T-5`
- Topic: Performance vs Robustness
- Reviewer Position: Prefer Option A (full re-scan), with a stated bound
- Advice:
  Re-scan all journals. Add a soft bound.
- Rationale:
  Option A is unambiguously correct in the face of hand-edits, deletions, and rotations. The "50K lines per developer" bound is an executor's gut number — needs measurement or a defensive cap.
- Required Action:
  Add to C-8 or a new constraint: "Index re-render reads at most 100 journal files (i.e., `journal-1` … `journal-100`), and at most 100 sessions per file (the `journal_max_lines = 2000` cap implies ~100 entries/file at typical entry size). Beyond that, sessions are silently truncated from the table; the journal files themselves remain canonical." Set a hard ceiling. This is preferable to discovering a 5-minute index render in 2 years.

### TR-6 `record_task` location

- Related Plan Item: `T-6`
- Topic: Coupling
- Reviewer Position: Prefer Option B (in `workspace::record`) — agree with executor
- Advice:
  Keep `record_task` in `workspace::record`.
- Rationale:
  One-way coupling (`task → workspace`) is the right architectural call. `workspace` having no knowledge of `task` keeps the module self-contained, easier to test, and doesn't accumulate cross-domain imports. The slight friction of "a third option struct" is a fair price.
- Required Action:
  Keep T-6 Option B. No revision.
