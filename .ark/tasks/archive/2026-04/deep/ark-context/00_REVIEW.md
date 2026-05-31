# `ark-context` REVIEW `00`

> Status: Closed
> Feature: `ark-context`
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
- Blocking Issues: 3
- Non-Blocking Issues: 7



## Summary

The plan is broadly coherent and well-scoped. It correctly separates gather / projection / render, picks a sensible top-level stability story, and the JSON vs text story is cleanly modeled. However, three HIGH issues must be resolved before execute: (1) `.claude/settings.json` is not under `owned_dirs()` today, so the claimed `init → unload → load` round-trip (G-11) cannot work without a layout change that the plan does not specify; (2) the JSON-merge path (T-3) creates a real conflict with `ark-upgrade`'s hash model — a merged file on disk will never byte-equal the embedded template, causing perpetual `AmbiguousNoHash` / `UserModified` classifications; (3) projection filtering for `specs.features` is defined only informally ("filtered to `current_task.related_specs` ∪ project specs"), which conflates two spec layers and leaves the match predicate unspecified. Several MEDIUM issues concern artifact-kind disambiguation, iteration parsing, `--dir` cwd handling, the `TargetArgs::resolve` call graph, and text-mode `schema` encoding. Trade-offs T-1, T-2, T-4, T-5, T-6 are acceptable; T-3 needs restructuring.



## Findings

### R-001 Round-trip of `.claude/settings.json` is not backed by current `owned_dirs()`

- Severity: HIGH
- Section: `[**Architecture**]`, `[**Implementation**]` Phase 3 step 7, Goal G-11
- Problem:
  The plan asserts `ark unload → ark load` preserves `.claude/settings.json` (G-11, V-IT-9) and names an "end-to-end round-trip test in `commands/load.rs::tests`". But the actual `Layout::owned_dirs()` in `crates/ark-core/src/layout.rs:150` returns only `[ark_dir(), claude_commands_ark_dir()]` — `.claude/settings.json` is NOT under either. `unload` only snapshots files that `walk_files(owned_dir)` yields (see `commands/unload.rs:61`), so `settings.json` will not be captured, and `remove` will not clean it up either. The plan does not call out extending `owned_dirs`, adding a per-file capture list, or switching to a managed-block-style capture for `settings.json`.
- Why it matters:
  Without a capture path, the round-trip test V-IT-9 will fail by design: on `unload`, `settings.json` is not included in the snapshot; on `load` it is re-scaffolded from templates, meaning any user edits (or even the accumulated merged state) vanish. This contradicts ark-upgrade SPEC G-9 which deliberately limits upgrade to `manifest.files ∪ desired_templates`; `ark-context` is adding a new file into `manifest.files` that lives outside `owned_dirs()`, which breaks the symmetry between `init`/`upgrade` (file-level tracking) and `unload`/`load` (dir-level tracking).
- Recommendation:
  Pick one of the following and state it explicitly in the Architecture and Implementation sections:
  (a) Extend `Layout::owned_dirs()` (or introduce `owned_files()`) to include `.claude/settings.json`, and update `unload`/`load` to walk the file list in addition to owned dirs. Requires documenting that only *this file* under `.claude/` round-trips (the rest of `.claude/` stays user-owned).
  (b) Treat the SessionStart hook as a managed-block-like entity: capture the entry's JSON subtree on unload (by path + identity key), restore via `merge_json_managed` on load. This is a closer analog to the existing `managed_blocks` snapshot slot and avoids capturing user edits to unrelated JSON keys.
  Whichever is chosen, V-IT-9 must cover both (i) post-init-then-unload-then-load returns the Ark hook and (ii) a user-added hook entry survives the round-trip.



### R-002 JSON-merge path collides with `ark-upgrade`'s hash-tracking model

- Severity: HIGH
- Section: `[**API Surface**]` templates additions; `[**Trade-offs**]` T-3; Constraints C-11, C-12
- Problem:
  The plan proposes writing `.claude/settings.json` via `merge_json_managed` (Phase 3 step 2–3), then "hash-track the post-merge contents so `upgrade` handles changes correctly" (Phase 3 step 3). But the merged on-disk content is, by construction, **not** byte-equal to the embedded template (`templates/claude/settings.json`): the on-disk version contains additional or reordered keys (user hooks), pretty-print whitespace from `serde_json`, and merge-site artifacts. On `upgrade`, `collect_desired_templates` yields the raw embedded bytes (`upgrade.rs:205`), and `classify()` compares `hash_bytes(desired) vs hash_bytes(current)` (`upgrade.rs:301`). The on-disk hash will never match the desired template's hash, so every upgrade classifies `settings.json` as either `AutoUpdate` (if the recorded hash matches on-disk) or `UserModified` (if it doesn't), and `AutoUpdate` would then overwrite the file with the raw template — silently discarding user hooks. The `AmbiguousNoHash` branch (no recorded hash + on-disk differs) also maps to `UserModified` per upgrade SPEC C-11.
  This tension is the core design risk of T-3 and the plan does not resolve it. The note "running `ark init` twice produces byte-identical `settings.json`" (C-12) is about idempotence of `merge_json_managed`, not about parity with the desired template.
- Why it matters:
  Either upgrade silently clobbers user hooks (`AutoUpdate` case) or it prompts on every upgrade forever (`UserModified` case, where prompting is the Interactive default). Both are unacceptable. The ark-upgrade SPEC constraint C-11 explicitly expects `AmbiguousNoHash` with content mismatch to become `UserModified`; there is no escape hatch for "template-expected-to-be-merged-in-place".
  The correct analog already exists in the codebase: `reconcile_managed_blocks` in `upgrade.rs:237`, which splices on-disk managed-block bodies into the desired bytes before hashing. That mechanism was designed for exactly this problem — tracking a template whose on-disk form is expected to diverge from the embedded bytes. The plan's rejection of "managed-block markers in JSON" (T-3 last bullet) discards this mechanism without replacement.
- Recommendation:
  Restructure T-3 and Phase 3 along one of these lines:
  (a) **Preferred — reuse `reconcile_managed_blocks`.** Teach upgrade to recognize JSON-pointer-based managed regions (e.g. the `hooks.SessionStart` Ark-owned entry identified by `"command"` identity key). Before hashing, splice the on-disk Ark entry into the desired-template JSON. The desired-then-spliced bytes round-trip cleanly through hash classification, and user-added sibling keys never enter hash comparison because they're already normalized out of the desired side too. This requires a small extension to `scan_managed_markers` for JSON files, or a new `scan_json_regions` that returns `(pointer, identity_key)` pairs declared via a sidecar header.
  (b) **Alternative — exempt `settings.json` from hash tracking entirely.** Treat it like `CLAUDE.md`'s managed block (upgrade SPEC C-8): re-apply the Ark entry via `merge_json_managed` unconditionally on every upgrade, never record its hash, never compare, never prompt. This is simpler but means no "template content changed → automatic update" path, and `ConflictPolicy` doesn't apply. Trade-off needs to be explicit.
  Either way: add a constraint stating whether `.claude/settings.json` participates in hash classification or is exempted, and update the ark-upgrade spec CHANGELOG accordingly (since the SPEC currently has no such exemption).



### R-003 Projection spec filtering for `features` is underspecified

- Severity: HIGH
- Section: `[**Spec**]` G-7 (plan/review), `[**Validation**]` V-UT-9 / V-UT-10
- Problem:
  G-7 says the `plan` and `review` projections return `specs.features` "filtered to `current_task.related_specs` ∪ project specs". Two problems:
  1. `current_task.related_specs` is typed as `Vec<String>` (paths parsed from PRD). `specs.features` is typed as `Vec<SpecRow>` with a `name` and `path` field. The filter predicate is not specified — match by exact `path` string? By path suffix? By canonicalized path? What if PRD contains `.ark/specs/features/foo/SPEC.md` vs `specs/features/foo/SPEC.md` vs just `foo`?
  2. "∪ project specs" mixes two spec layers. `SpecsState` has separate `project: Vec<SpecRow>` and `features: Vec<SpecRow>` fields. The sentence reads as if the `features` list should include project-spec rows, which violates the model. Presumably the intent is "`specs.features` filtered to related_specs; `specs.project` included unchanged" — but that's not what it says.
- Why it matters:
  The executor has to guess the filter predicate and will almost certainly pick a different one than the reviewer expects; V-UT-9 / V-UT-10 as written ("features filtered to `current_task.related_specs` ∪ project specs") will either be written to tautologically match whatever the implementation does, or fail on a PRD edge case the test fixture doesn't exercise (e.g. the `reference/` PRD format variations, or a missing trailing slash). NG-11 ("no structured parsing … beyond a line-by-line extract") doesn't bound the problem because it only constrains how paths are *extracted*; the *match predicate* is still unspecified.
- Recommendation:
  Rewrite G-7 with explicit semantics:
  > `plan` / `review` projection returns `specs = Some(SpecsState { project: <full>, features: <filtered> })` where `features` is the subset of `ctx.specs.features` whose `path` ends with an entry in `current_task.related_specs`, after both sides are normalized to project-relative form (leading `./` stripped, `.ark/` prefix stripped if present). If `current_task.related_specs` is empty, `features` is `[]`.
  Also add a constraint C-N: "`related_specs` parser extracts tokens matching the regex `specs/features/[a-z0-9_-]+/SPEC\.md` (case-sensitive) from lines inside the `[**Related Specs**]` section; non-matching lines are ignored; the section ends at the next `[**...**]` heading or EOF." Then V-UT-19 has something concrete to assert against.



### R-004 `ArtifactKind::Plan { iteration } / Review { iteration }` disambiguation rule is missing

- Severity: MEDIUM
- Section: `[**Data Structure**]` `ArtifactKind`; `[**Runtime]` §State Transitions
- Problem:
  The struct declares `Plan { iteration: u32 }` and `Review { iteration: u32 }`, and §State Transitions says "latest-iteration PLAN picked by `max(NN)` parsed from filename". But `CurrentTask.artifacts: Vec<ArtifactSummary>` is described as "artifact files" — plural — and V-UT-4 asserts both `Plan { iteration: 0 }` *and* `Review { iteration: 0 }` appear. So the list is NOT filtered to latest; it's all plans / all reviews. But then the phase projections say "latest `NN_PLAN.md`" (`review` phase G-7) and "latest PLAN path" (`execute`, `verify`) — so the projection layer is doing the latest-pick, not gather. That's fine, but the rule is not stated in one place.
  Secondly, the filename parser is not specified: does it accept `00_PLAN.md`, `01_PLAN.md`, `99_PLAN.md`? Leading zeros always required? V-F-4 asserts `.bak` exclusion but the positive regex is only hinted at ("`^\d{2}_PLAN\.md$`").
- Why it matters:
  Two related callsites (gather artifact listing, projection latest-pick) need a shared rule. Without it, the executor may emit artifacts unsorted, causing non-determinism in `TextSummary` golden fixtures (V-UT-13) and confusing V-IT-1.
- Recommendation:
  Add to the Spec section:
  > **Artifact iteration rule.** `gather_context` emits all `NN_PLAN.md` / `NN_REVIEW.md` files matching `^(\d{2})_PLAN\.md$` / `^(\d{2})_REVIEW\.md$` in the task dir, sorted by parsed `NN` ascending, captured in `ArtifactSummary.kind`. Projections that need "latest" call `artifacts.iter().filter(|a| matches!(a.kind, ArtifactKind::Plan{..})).max_by_key(|a| a.kind.iteration())` — helper method on `ArtifactKind` returning `Option<u32>`.



### R-005 `TargetArgs::resolve` / cwd detection is not specified; V-E-5 flags it as a known bug

- Severity: MEDIUM
- Section: `[**API Surface**]` `ContextArgs`; Validation V-E-5
- Problem:
  `ContextArgs` uses `#[command(flatten)] target: TargetArgs`, reusing an existing type. V-E-5 itself admits "we need to ensure `--dir` flag or project-root detection walks up to find `.ark/`" and "Follow-up: reuse whatever detection existing commands use." But the plan neither specifies the detection rule nor cites the existing one. Looking at `init.rs` / `upgrade.rs`, they take `project_root: PathBuf` verbatim — no detection; callers pass `--dir` explicitly, default is cwd.
- Why it matters:
  Hooks invoke `ark context --scope session --format json` with cwd = wherever Claude Code's subprocess is launched. If that's a task subdirectory, today's TargetArgs contract yields `.ark/` not-found. The plan accepts this in V-E-5 ("without `--dir`, it errors with `NotLoaded`") but the `SessionStart` hook entry (C-11) passes no `--dir`, so whenever the hook runs from a non-root cwd, the session starts with a loud error. That either pollutes hook stderr or silently degrades the orientation feature.
- Recommendation:
  Pick one:
  (a) Declare V-E-5's behavior acceptable and add a Constraint / SessionStart-hook precondition: "hook must be invoked from project root; Claude Code sets cwd to the workspace root, so this holds in practice." Cite the Claude Code hook semantics.
  (b) Add a project-root walk: `Layout::discover_from(cwd)` climbs ancestors until `.ark/` is found, else `NotLoaded`. Apply consistently to all commands, not just `context`, to avoid inconsistent cwd semantics.
  Either way, V-E-5 should change from a "bug risk" note to a tested invariant.



### R-006 Text-mode `schema=1` header is specified but encoding is ambiguous

- Severity: MEDIUM
- Section: `[**Spec**]` G-4 / C-3; Validation V-UT-14
- Problem:
  C-3 requires "Text output's first line contains `schema=1` in a header comment", but `render.rs` is described as `impl Display for TextSummary<'a>` with Trellis-style `## GIT STATUS` etc. What does "header comment" mean in text mode — a `# schema=1` line? A `<!-- schema=1 -->`? Plain `schema=1` as the first line? V-UT-14 only tests JSON first-byte; C-10 says "text mode carries no schema version" which contradicts C-3.
- Why it matters:
  C-3 and C-10 are directly inconsistent as written. The executor needs to know which wins.
- Recommendation:
  Delete C-3's text-mode clause and keep C-10 ("text mode carries no schema version"). Text is for humans; adding a schema header is noise. Alternatively, keep the header and delete C-10 — but pick one.



### R-007 Git invocation is by `std::process::Command::new("git")` but routing through a helper is unstated

- Severity: MEDIUM
- Section: `[**Architecture**]` gather module; C-4 (PathExt only)
- Problem:
  C-4 mandates "All filesystem access in `commands/context/` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`." Phase 1 step 3 then says "git: reuse `std::process::Command::new("git")`; 3 calls". `std::process::Command` is not `std::fs` so C-4 is literally satisfied, but there is no helper module for git invocation, and the V-F-1 error shape ("`Error::Io` wrapping the spawn error") implies the executor will hand-roll error mapping. The upgrade/init/agent modules all avoid exactly this pattern.
- Why it matters:
  Three separate `git` invocations in `gather.rs` (branch, porcelain, log) will each duplicate error-wrapping boilerplate. First time we need a second git caller in-tree, we refactor. Also: `Error::Io` expects a `path`; there's no natural path for a spawn failure, which may push the executor toward a `PathBuf::new()` hack or a new `Error::GitSpawn` variant that isn't declared.
- Recommendation:
  Add a small helper module `commands/context/git.rs` with `run_git(args: &[&str], cwd: &Path) -> Result<String>` returning stdout on exit 0, soft-failing (returning `None` or a sentinel) on non-zero for the non-git-dir case. Declare the error shape explicitly: either add `Error::Git { reason: String }` or commit to reusing `Error::Io { path: cwd, source }`. Phase 1 step 3 should reference this helper rather than raw `Command::new`.



### R-008 `specs/project/INDEX.md` parser path is under-specified and currently non-managed

- Severity: MEDIUM
- Section: `[**Spec**]` G-6; `[**Implementation]` Phase 1 step 3 (specs)
- Problem:
  Phase 1 step 3 says "parse managed-block rows from `specs/project/INDEX.md` and `specs/features/INDEX.md`", then in parens "or whole body for project index which is user-authored". `specs/project/INDEX.md` is currently a user-authored markdown table **without any managed block** (confirmed by reading `.ark/specs/project/INDEX.md`). So the parser has two completely different grammars for the two files. Neither is concretely specified: column order, separator escaping, header-row skipping, etc.
- Why it matters:
  If the executor writes a naive "find the first markdown table after `## Index`" parser, it will break silently when users reformat the file (which they will — it's user-authored). That failure mode is silent because V-F-3 says "rows parse as empty, stderr warning in text mode only, JSON mode silent" — exactly the fail-open behavior that hides bugs.
- Recommendation:
  Specify both grammars precisely. For `features/INDEX.md`: "parse the table inside the `ARK:FEATURES` managed block body via `read_managed_block`; rows are `| name | scope | promoted |` with pipe-split and trim; the header / separator rows (lines that match `^\\s*\\|\\s*-+` or whose first cell equals `Feature`) are skipped." For `project/INDEX.md`: "locate the first GFM table after the `## Index` heading; same cell parse; skip header/separator; tolerate trailing / leading pipe differences." Add one unit test per grammar.



### R-009 `Error::ContextProjectionMismatch` is declared but never raised in production

- Severity: LOW
- Section: `[**Data Structure]` error additions
- Problem:
  The plan adds `Error::ContextProjectionMismatch { scope, reason }` and immediately documents that it is "raised only in tests / invariant guards; … in production that case yields an empty `current_task` field, not an error. Guard is there to catch programmer mistakes in future projections." This is a hypothetical-future error variant with no live call site, which is exactly the pattern AGENTS.md §"What Not to Do" calls out ("Don't add files just to host one function") in spirit.
- Why it matters:
  Unused error variants drift. If a future developer reads the match arms in `error.rs` expecting every variant to be reachable from `ark context` code, they will be confused.
- Recommendation:
  Either (a) cut the variant and use `debug_assert!` / `unreachable!` for invariant checks in `projection.rs`, or (b) define at least one production callsite — e.g. "projection is given a `Scope::Phase(_)` but `ctx.schema != SCHEMA_VERSION`" (defends against future schema-bump regressions). Option (a) is cleaner given the plan's own description.



### R-010 `--format json | jq` pretty-print is assumed but the exact shape is not nailed down

- Severity: LOW
- Section: `[**Runtime]` step 5; C-7
- Problem:
  Step 5 says "`serde_json::to_writer_pretty`". `to_writer_pretty` emits 2-space indent and trailing newline; `to_string` emits compact. Downstream hooks / `jq` pipelines don't care about formatting but `V-IT-7` ("matches fixture") will care, and C-7 ("exactly one stdout write") doesn't disambiguate whether the pretty-print call's internal writes count as one (they're writer-level, single logical write).
- Why it matters:
  Golden-fixture tests are brittle without the exact rendering commitment.
- Recommendation:
  Add a constraint: "JSON mode uses `serde_json::to_writer_pretty(stdout, &projected)` followed by `stdout.write_all(b"\n")`. Indent is 2 spaces. Field order follows `Serialize` derive order in `ProjectedContext`." Text mode: explicitly declare trailing-newline behavior.



## Trade-off Advice

### TR-1 Two flags (`--scope` + `--for`) vs single `--mode` enum

- Related Plan Item: `T-1`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (two flags)
- Advice:
  Keep the two-flag design.
- Rationale:
  The plan's rationale is sound: `--scope` already anticipates `task` / `feature` dimensions. A flat `--mode` enum would need renaming when those arrive. Clap's arg-relation enforcement covers the only ergonomic risk. The Trellis `--mode` precedent isn't compelling — Trellis doesn't have `phase × projection` orthogonality.
- Required Action:
  Adopt as specified. Ensure V-IT-5 / V-IT-6 cover both illegal combinations (`--for` without `--scope=phase`, `--scope=phase` without `--for`).



### TR-2 Schema-version from v1 vs wait-until-breaking-change

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (version from day one)
- Advice:
  Keep `"schema": 1` as the first field from day one.
- Rationale:
  Adding `schema` later is itself a breaking change (old payloads lack the field). Downstream `jq` consumers on a hook pipeline benefit from being able to gate now. The cost (one extra field to maintain) is trivial.
- Required Action:
  Adopt as specified. Resolve the C-3 / C-10 contradiction per R-006 so text mode does not also claim a schema.



### TR-3 JSON-pointer merge helper vs managed-block-reconcile vs full overwrite

- Related Plan Item: `T-3`
- Topic: Compatibility vs Clean Design (most important trade-off in this plan)
- Reviewer Position: Need More Justification
- Advice:
  Reject the current "merge helper + hash-track post-merge contents" formulation. Restructure along one of the two alternatives named in R-002: either reuse/extend `reconcile_managed_blocks` so JSON regions behave like existing managed blocks, or exempt `settings.json` from hash tracking entirely and always re-apply the Ark entry via `merge_json_managed` (analog to `CLAUDE.md` per upgrade SPEC C-8).
- Rationale:
  As designed, the merged file's on-disk bytes are never byte-equal to the embedded template. Under upgrade's current classifier, this means either `AutoUpdate` (which would overwrite the merged file with the raw template and destroy user hooks) or `UserModified` (which prompts on every upgrade). The plan's acknowledgement "a user who changes unrelated keys triggers the user-modified path on upgrade — acceptable" under-describes the problem: it happens on the very first upgrade, for every user, because the mere act of merging produces divergence.
  The `reconcile_managed_blocks` precedent (upgrade.rs:237) already solves exactly this for text-file managed blocks. Extending it to JSON-pointer regions is more work than "write a merge helper" but yields correct semantics. The no-hash-tracking alternative (C-8-style) is simpler and probably sufficient for Phase 0 scope since the template is tiny and user-editing the Ark-owned entry is rare.
- Required Action:
  In `01_PLAN`, replace T-3 with an explicit choice between (a) "extend upgrade reconcile to JSON regions" and (b) "exempt `settings.json` from hash tracking, always re-apply via `merge_json_managed`". Add constraints describing upgrade behavior either way. Update V-IT-8 to assert whichever behavior is chosen (specifically: what happens when a user adds a `PreToolUse` hook and template's `SessionStart` content is unchanged; and what happens when both change).



### TR-4 Emit `dirty_files` list (capped at 20) vs count only

- Related Plan Item: `T-4`
- Topic: Performance vs Utility
- Reviewer Position: Prefer Option A (list + count)
- Advice:
  Keep the list capped at 20 plus the total count.
- Rationale:
  The execute-phase hook/slash command genuinely needs the filenames. 20 strings ≈ 1KB is negligible next to the full JSON payload. Keeping only the count would force every caller to re-run `git status`, defeating the consolidation motive.
- Required Action:
  Adopt as specified. Consider documenting the cap value (`DIRTY_FILES_CAP = 20`) as a named constant so the test `C-8` asserts against it symbolically.



### TR-5 Trellis-section text layout vs compact one-liner

- Related Plan Item: `T-5`
- Topic: Usability
- Reviewer Position: Prefer Option A (multi-section)
- Advice:
  Adopt the section-headed layout. Users who want compact output pipe JSON through `jq`.
- Rationale:
  The text-mode audience is the ad-hoc human reader; compact output is worse for them. The JSON path covers terseness for scripts. Text mode is not a stable contract (per C-10 and R-006's resolution).
- Required Action:
  Adopt as specified. Commit to a specific set of section names (exactly the list in G-12) and lock them via golden fixtures in V-UT-13.



### TR-6 Parse `[**Related Specs**]` from PRD vs caller-supplied list

- Related Plan Item: `T-6`
- Topic: Coupling vs Locality
- Reviewer Position: Prefer Option A (parse from PRD)
- Advice:
  Parse it.
- Rationale:
  The alternative (caller-supplied) forces every slash command / hook invocation to mirror the PRD's related-specs list, creating two sources of truth. Coupling `ark context` to PRD template format is acceptable because the template is shipped by the same crate and versioned with the CLI. The risk the plan flags ("template change breaks the parser silently") is real but bounded — cover both "bullet with path" and "empty section" via V-UT-19, and add one test for malformed-section (no heading, heading without bullets).
- Required Action:
  Adopt as specified. Per R-003 above, tighten the match regex and declare the section terminator explicitly so the parser contract is testable.
