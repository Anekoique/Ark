# `worktree-support` REVIEW `00`

> Status: Open
> Feature: `worktree-support`
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

- Decision: Rejected
- Blocking Issues: 7
- Non-Blocking Issues: 5

## Summary

The plan is coherent in shape — module layout mirrors `agent`'s sibling pattern, the CLI surface is named-command and `Display`-only as `ark-agent-namespace` requires, and the call graphs are precise. But several blocking gaps remain before execute. The Spec section is **not self-contained**: it leans on Runtime/Failure-Flow text and references Validation IDs that live outside Spec (workflow.md PLAN rule break — directly contradicts the recent `spec extract self contained` fix in commit 71f09b1). The G-9 / C-15 claim that `Layout::discover_from` already handles worktrees is misleading — the existing implementation walks ancestors, so from inside `.ark/worktrees/<branch>/` it finds whatever `.ark/` the **branch's HEAD commit** carries, which is fundamentally not the freshly-created task dir. The interaction with `ark unload`'s `walk_files(ark_dir())` will recursively descend into every nested worktree's source tree, capturing potentially gigabytes into `.ark.db` and clobbering on `load`. The `--branch-type` / `--branch` precedence is documented but not enforced at clap level. Finally, error variant naming (`WorktreeAlreadyExists` vs `WorktreeExists`) is inverted relative to natural English and will cause executor confusion.

## Findings

### R-001 `Spec section depends on out-of-Spec context`

- Severity: CRITICAL
- Section: `## Spec` (whole section vs workflow.md §4 PLAN rule)
- Problem:
  workflow.md REVIEW step says: *"Reject (HIGH) if the latest PLAN's `## Spec` references prior iterations instead of restating in full"*, and PLAN says: *"`## Spec` must be self-contained every iteration … copied verbatim to `specs/features/<name>/SPEC.md` on archive."* Commit 71f09b1 (`fix(workflow): spec extract self contained`) made this enforcement teeth.

  The current `## Spec` block contains multiple references that resolve only by reading other sections of the PLAN or PRD:
  - C-8's rollback policy refers to "the underlying error" but the actual ordering of rollback steps lives in the call-graph's pseudocode comments. After extraction, the call-graph is in Architecture (inside Spec, fine) but the pseudocode legend isn't.
  - **Failure Flow #5** (`cfg.copy` source missing → log warning, skip) lives in **Runtime**, not Spec. Yet G-3, C-8 and V-E-3 all rely on this contract. After extraction, that semantics is *gone* from `specs/features/worktree-support/SPEC.md`.
  - **Failure Flow #1**'s "branch in use detected via `git worktree list --porcelain`" is the only place that pins down *how* `BranchInUse` is detected — Architecture's call graph just says `git_branch_in_use(branch, repo_root)` opaquely.
  - Spec text says "Validation gate per C-8" / refers to behaviors qualified in Failure Flow which are not part of Spec. Future readers see dangling references.

- Why it matters:
  When `ark agent task archive` runs on this feature, the Spec block is copied verbatim to `specs/features/worktree-support/SPEC.md`. Future tasks that read it (via `ark context --scope phase --for plan` filtering by Related Specs) will see dangling references and missing failure-mode contracts. This is exactly the regression the recent self-contained fix was meant to prevent.
- Recommendation:
  1. Add a `[**Failure Modes**]` (or absorb into Constraints) under Spec covering: missing copy-source policy; branch-in-use detection mechanism (`git worktree list --porcelain`); rollback ordering when `post_create` fails after `git worktree add` succeeds; archived-task gating semantics.
  2. Verify Spec reads cleanly when copy-pasted into a fresh file with no surrounding context — every contract that V-* or Runtime relies on must appear inside Spec.
  3. Remove any reference to V-* IDs from inside Spec; they live in Validation.

### R-002 `discover_from claim ignores branch-state divergence; G-9 / C-15 / V-IT-6 inadequate`

- Severity: CRITICAL
- Section: `## Spec` G-9, C-15; Validation V-IT-6
- Problem:
  G-9 / C-15 assert: *"`Layout::discover_from(cwd)` already walks ancestors looking for `.ark/`. From inside a worktree, the worktree's `.ark/` is the project root — found at the worktree dir itself."* I read `layout.rs` lines 220-230: `discover_from` walks `cwd.ancestors()` and stops at the first dir whose `<dir>/.ark/` exists.

  This claim is only true if the worktree's branch contains a `.ark/` directory at the worktree root. The plan does not address what `.ark/` the worktree actually has:

  - `.ark/worktrees/feat/foo/` is a checkout of `feat/foo` based on whatever `<base_branch>`'s HEAD commit was at `git worktree add` time. The contents of `.ark/` inside the worktree are *whatever that commit had*.
  - `task new --slug foo` creates `.ark/tasks/foo/` and `.ark/tasks/.current` on the parent's working tree as **uncommitted changes**. They do NOT exist in any commit; they are workdir edits.
  - When `task new --worktree` then runs `git worktree add`, the worktree is built from the base commit, NOT the workdir. The freshly-created `.ark/tasks/foo/` is **NOT** in the worktree.
  - Result: `cd <worktree> && ark context` reads the worktree's `.ark/tasks/.current` — which is `<base_branch>`'s last-committed `.current`, NOT `foo`. The user's task is invisible from the worktree.

  Conversely, if the user *commits* `.ark/tasks/foo/` on the parent's branch before `worktree create`, the worktree gets the task — but the parent and worktree both carry the same `.current`, defeating the parallelism rationale.

  V-IT-6 as drafted only tests "scaffold an Ark project, `git worktree add` a branch, set `.current` inside the worktree, assert `gather_context` returns the worktree-side slug." This is the trivial path — manually pre-populating `.current` inside the worktree's working tree. It does not exercise *how the task dir gets there in the first place* under `task new --worktree`.

- Why it matters:
  This is the central correctness claim of the feature. If the freshly-created task is not visible inside the worktree, the parallelism story collapses. The PRD's verification gate (*"creates two worktree-backed tasks, advances both through PLAN simultaneously, and asserts no `.current` collision"*) cannot pass with the current design.
- Recommendation:
  1. Pin down the protocol explicitly. Two viable options:
     - **Option A (worktree-first)**: `task new --worktree` runs `git worktree add` *first* (against an empty starter branch or a clone of base_branch), then scaffolds `.ark/tasks/<slug>/` *inside the worktree*. Parent's working tree is untouched.
     - **Option B (commit-and-checkout)**: scaffold `.ark/tasks/<slug>/` on parent, `git add` + `git commit` it, then `git worktree add` so the commit lands in the worktree. Pollutes parent with a "scaffolding" commit.
     Option A is the cleaner story and matches Trellis. Pick one and document.
  2. Document explicitly: when `task worktree create` runs against an *existing* parent task dir (not via `task new --worktree`), the parent's task dir must be migrated into the worktree (via `git mv` + commit, or rename + `task new` inside the worktree) — and `.current` on parent updated. The plan currently leaves this undefined.
  3. Add V-IT-6b: `task_new_worktree_task_dir_lives_only_in_worktree` — assert that after `task new --slug foo --worktree`, `<repo>/.ark/tasks/foo/` does NOT exist on the parent's working tree, but the worktree at the worktree path has its own.
  4. Add V-IT-6c: `gather_context_from_worktree_does_not_see_parent_current` — set parent's `.current` to a different slug, run `gather_context` from inside the worktree, assert it does NOT resolve the parent's slug.

### R-003 `unload / walk_files recurses into worktrees and captures every byte`

- Severity: CRITICAL
- Section: `## Spec` G-10, C-16; Failure Modes (missing)
- Problem:
  `unload.rs` line 66 iterates `layout.owned_dirs()` (`[ark_dir(), claude_commands_ark_dir(), codex_dir()]`) and calls `walk_files(&owned)` on each. With worktrees stored at `.ark/worktrees/<branch>/`, every file in every active worktree gets walked, hashed, and captured into `.ark.db`:
  - The branch's committed source tree (Cargo build dirs if present, `target/`, `node_modules/`, etc.).
  - The worktree's uncommitted edits (the user's WIP).
  - The worktree's internal `.git` pointer file (or directory in older git setups) — capturing internal git state into the snapshot.

  Then `ark load` *restores* those files, overwriting whatever the worktree's branch HEAD currently is. **Silent data loss** for any commit made between `unload` and `load`.

  Adding `.ark/worktrees/` to `.gitignore` (G-10) does **nothing** to fix this — `walk_files` walks the filesystem, not the gitignore. It is structural to placing worktrees inside a directory `unload` already owns.

  The `upgrade.rs` `extract` walk (line 215) also iterates `ARK_TEMPLATES → ark_dir()`. Need to confirm extract doesn't blow away `.ark/worktrees/` — likely safe because templates only ship to specific paths, but worth a constraint.

- Why it matters:
  Unload/load is a critical-path command pair. Silently corrupting it on any project with active worktrees turns this feature into a footgun. This is exactly the kind of cross-cutting interaction the deep-tier review bar exists to catch.
- Recommendation:
  1. Add a hard constraint in Spec: `walk_files` (or specifically the unload/snapshot capture path) must skip `.ark/worktrees/`. Implementation options: (a) add a skip-list parameter to `walk_files`, (b) carve `worktrees_dir()` out at the snapshot-capture layer, (c) special-case it in `unload`. Choose one.
  2. Add constraint that `manifest.files` and `manifest.hashes` exclude any path under `worktrees_dir()`.
  3. Add V-IT: `unload_excludes_worktree_contents` — create a worktree with a sentinel file in it, run `ark unload`, assert `.ark.db` size is bounded and the sentinel is absent.
  4. Add V-IT: `load_round_trip_does_not_touch_worktrees` — `unload` then `load`, assert worktree dirs and contents are unchanged.
  5. Audit `upgrade.rs::extract` for the same recursion risk and document if safe.

### R-004 `task new --worktree atomic rollback is incompletely specified given R-002 protocol gap`

- Severity: HIGH
- Section: `## Spec` G-7, C-14; Implementation Phase 3
- Problem:
  C-14: *"`task new --worktree` is atomic: if `worktree_create` fails, the task dir created by `task_new` is removed before returning the error."*

  This contract assumes `task_new` runs first and creates the task dir on the parent's working tree. Combined with R-002's protocol question, the rollback story changes:
  - Under Option A (worktree-first), there's no parent task dir to roll back; rollback removes the *worktree dir*. Different semantics.
  - Under Option B (commit-and-checkout), rollback must `git reset --hard` to drop the scaffolding commit. Possibly destructive — what if user had unrelated work?

  The "atomic" wording also doesn't address: what if `task_new` succeeds, `git worktree add` succeeds, but `cfg.copy` or `post_create` fails? C-8 says "best-effort rollback: `git worktree remove --force` and surface error." But the **task dir still exists** (it was created in step 1). Is that rolled back too? The plan is silent.

- Why it matters:
  The atomic-failure path is the most likely user-observable bug surface for `--worktree`. Half-rolled-back state means a user running `task new --slug foo --worktree` and getting an error has to manually inspect what survived.
- Recommendation:
  1. After resolving R-002, restate C-14 with the chosen protocol's rollback semantics.
  2. Explicitly enumerate rollback for each failure point: (a) `task_new` fails — nothing to roll back; (b) `git worktree add` fails — `task_new` artifacts removed; (c) `cfg.copy` / `post_create` fails — both worktree AND `task_new` artifacts removed.
  3. Add V-IT: `task_new_worktree_post_create_failure_rolls_back_both` — simulate post_create failure, assert neither task dir nor worktree dir survive.

### R-005 `Error::WorktreeExists vs WorktreeAlreadyExists are inverted relative to natural reading`

- Severity: HIGH
- Section: `## Spec` Data Structure (error variants); Failure Flow #3
- Problem:
  Spec lists two error variants:
  - `Error::WorktreeAlreadyExists { slug, path }` — used when `task.toml.worktree_path` is already set (Architecture call graph line: `if toml.worktree_path.is_some() → Error::WorktreeAlreadyExists`).
  - `Error::WorktreeExists { path }` — used when the dir on disk already exists (`if worktree_path.exists() → Error::WorktreeExists`).

  Failure Flow says: *"Worktree dir already exists on disk → Error::WorktreeExists. (Distinct from WorktreeAlreadyExists, which is the task.toml level guard.)"*

  This reads backwards. "AlreadyExists" naturally implies "the directory is already there"; the plan reserves it for the metadata case. The existing `Error::TaskAlreadyExists` (in `agent-namespace`) refers to the on-disk task dir, reinforcing the natural reading. An executor implementing this will flip them, and reviewers reading match arms will mis-read the contract.

- Why it matters:
  Subtle naming bugs in error variants survive into APIs and break callers' match arms. An executor coding the natural way silently inverts the contract.
- Recommendation:
  Rename for clarity. Suggested:
  - `Error::TaskAlreadyHasWorktree { slug, path }` — for the task.toml-level guard (clear semantic).
  - `Error::WorktreeDirExists { path }` — for the on-disk case.

  Or collapse to one variant `Error::WorktreeExists { slug, path, source: WorktreeExistsSource::Toml | OnDisk }`.

### R-006 `--branch-type / --branch should be mutually exclusive at clap level`

- Severity: HIGH
- Section: `## Spec` API Surface (`WorktreeCreateCliArgs`); C-5
- Problem:
  C-5 documents precedence (`--branch` > `--branch-type/<slug>` > `<cfg.branch_prefix>/<slug>`). The CLI struct lets users pass *both* `--branch-type fix --branch refactor/foo` and silently drops one. Per `ark-agent-namespace` G-2 (named commands, no generic setters) and the broader spirit of "fail fast at the CLI boundary", clap should reject the combination outright with `conflicts_with`.

  Also: `--branch <full>` lets the user write a branch unrelated to the task slug (`--slug foo --branch refactor/bar`). The plan does not say whether `task.toml.branch = "refactor/bar"` is permitted or rejected. Likely permitted, but be explicit.

- Why it matters:
  Silent precedence is a footgun. Users who mix flags will assume both took effect. Clap-level rejection makes the contract obvious.
- Recommendation:
  1. Add `#[arg(long = "branch-type", conflicts_with = "branch")]` on the CLI struct.
  2. Add Spec sentence: when `--branch <full>` is given, the slug-vs-branch relationship is unconstrained (the user owns naming).
  3. Add V-UT: `branch_type_and_branch_are_mutually_exclusive_at_cli`.

### R-007 `Recursive worktree-from-worktree nesting unaddressed`

- Severity: HIGH
- Section: `## Spec` G-3; Failure Modes (missing)
- Problem:
  Nothing in Spec prevents the user from running `ark agent task worktree create` *from inside an existing worktree* (`cwd = <repo>/.ark/worktrees/feat/foo/`). With `Layout::discover_from`, that resolves to the worktree's own root. Then `worktree_create` resolves `worktree_path` as `<wt>/.ark/worktrees/feat/bar/` — recursive nesting.
  - The new nested worktree tracks the *worktree's branch*, not the parent repo's.
  - Cleanup of the outer worktree leaves orphan inner worktrees with broken `.git` pointers.
  - `git worktree list --porcelain` (used by `BranchInUse` detection per Failure Flow #1) is run from inside the outer worktree, finding only outer-tracked worktrees — branch-in-use detection misses the parent's worktrees and false-negatives.
  - `unload` on the outer worktree (which user might do mistakenly) recurses again per R-003.

  Trellis (the reference codebase the PRD cites) explicitly forbids this. The decision to store worktrees inside `.ark/` (T-1) makes it more likely a user lands here, because `.ark/worktrees/...` is a place users naturally `cd` into.

- Why it matters:
  Footgun directly enabled by the inside-the-repo storage choice. The fix is small but must be specified.
- Recommendation:
  1. Add Spec constraint: `worktree create` detects when the resolved `layout.root()` lies under any path matching `*/.ark/worktrees/*/` and refuses with `Error::NestedWorktreeForbidden { current_root }`.
  2. Add V-F: `worktree_create_inside_worktree_rejected`.
  3. Consider whether `Layout::discover_from` itself should refuse to resolve into a worktree dir — probably no (it's a generic helper), but the worktree command-layer must guard.

### R-008 `Cross-worktree task-dir migration semantics underspecified`

- Severity: HIGH
- Section: `## Spec` G-7; State Transitions
- Problem:
  Scenario unaddressed: user runs `task new --slug foo` (no `--worktree`) on `main`. `.ark/tasks/foo/` and `.ark/tasks/.current = foo` land on `main`'s working tree (uncommitted). User then runs `task worktree create --slug foo`.

  The plan handwaves this in G-3 by saying `worktree_path` is recorded in `task.toml`, but doesn't say what happens to *the task dir itself* across the worktree boundary:
  - Per R-002, the worktree at `.ark/worktrees/feat/foo/` is built from main's HEAD — does not contain the uncommitted task dir.
  - The user's task dir lives only on the parent. They can't work on it from the worktree.
  - The parent's `.ark/tasks/.current` still says `foo`; the worktree's `.current` is whatever main HEAD has.

  Two `.current` files now disagree across worktrees. `ark context` from the parent will see the parent's stale `.current`, list a task that doesn't exist on `main`, and report inconsistent state.

- Why it matters:
  The "task new without --worktree, then worktree create later" path is explicitly supported by PRD outcome #2 ("`task new` followed by `task worktree create`"). It must work, and the current plan does not specify how.
- Recommendation:
  1. Spec must specify: after `task worktree create` on an *existing* parent task dir, the parent's task dir is **migrated** into the worktree. Two reasonable mechanisms — pick one:
     - `git mv .ark/tasks/<slug> .ark/tasks/<slug>` is a no-op; instead, rename + commit on the worktree's branch (or on `<base_branch>` and rebase). Messy.
     - Move the task dir at the OS level into the worktree, then `git add` + commit on the worktree's branch. Parent's working tree no longer has it.
  2. Spec must specify: after migration, the parent's `.ark/tasks/.current` is cleared if it pointed at this slug.
  3. Add V-IT: `worktree_create_on_existing_parent_task_migrates_dir`.
  4. Document: `task list` / `worktree list` / `gather_context` invoked from the **parent** will not enumerate worktree-exclusive tasks (since they're not in the parent's tasks dir). State this as an invariant so reviewers don't expect cross-tree visibility.

### R-009 `Branch slash in branch name leaves orphan parent dirs after cleanup`

- Severity: MEDIUM
- Section: `## Spec` Data Structure (`Layout::worktree_dir`); G-4
- Problem:
  V-UT-7 says `Layout::worktree_dir("feat/foo")` returns `<root>/.ark/worktrees/feat/foo/` — a two-level dir. After `worktree cleanup` removes `feat/foo`, the empty `.ark/worktrees/feat/` parent dir remains. The plan doesn't say to prune.

  Also: with `--branch <full>` accepting any ref name, a user could pass `--branch refs/heads/feat/foo` (rare but legal) yielding `.ark/worktrees/refs/heads/feat/foo/` — three levels deep.

- Why it matters:
  Footprint hygiene; minor CI snapshot stability if tests assert directory tree shape.
- Recommendation:
  1. Add Spec constraint: cleanup prunes empty parent directories under `worktrees_dir()` after `git worktree remove`, mirroring `Layout::prunable_empty_parents` style.
  2. Add Spec constraint: `--branch <full>` must additionally not contain `..` segments (already implied by `git check-ref-format`, but explicit is safer).

### R-010 `worktree.toml management contract is contradictory; G-1 / T-6 / Q-1 disagree`

- Severity: MEDIUM
- Section: `## Spec` G-1; Trade-offs T-6; Open Q-1
- Problem:
  G-1 says the file is "snapshot-tracked like other `.ark/` content" and "documented defaults" — but T-6 / Q-1 explicitly leave open whether it's managed (refresh-on-upgrade) or user-editable. These conflate two things:
  - `unload`/`load` does walk `.ark/`, so it captures `worktree.toml` either way (this is fine and what the user wants).
  - The unresolved bit is whether `ark upgrade` overwrites it.

  Per Q-1's lean: user-editable. But Spec G-1 currently reads as if both managed *and* user-editable — contradictory.

  Also: G-1 lists "three keys: `worktree_dir`, `copy`, `post_create`" but Data Structure shows **four** keys including `branch_prefix`. Inconsistency; pick one.

- Why it matters:
  Q-1 is the kind of ambiguity that gets resolved differently in two places by an executor in flow. Lock it in Spec before execute.
- Recommendation:
  1. Resolve Q-1 in Spec. Recommended: user-editable, NOT refreshed on upgrade. Ship via `ark init` only, mirroring `workflow.md`'s treatment.
  2. Fix G-1's key count — it's four keys, not three.
  3. Add Spec constraint: `worktree.toml` is created by `ark init` from `templates/ark/worktree.toml`. `ark upgrade` does NOT overwrite it.

### R-011 `cfg.copy missing-source policy is too lenient and lives outside Spec`

- Severity: MEDIUM
- Section: Failure Flow #5; V-E-3 (also R-001)
- Problem:
  Failure Flow #5: "cfg.copy source missing → log warning, skip (do not abort). Source-not-found is a config error, not a runtime crash."

  Two issues:
  1. This contract is in **Runtime/Failure Flow**, not Spec — won't be in extracted SPEC.md (R-001).
  2. "Log warning" — to where? `ark agent` commands are `Display`-summary-only per `ark-agent-namespace` C-3. There's no logger; stderr writes are not standard. The plan needs to commit: stderr `eprintln!`, included in summary, or silently skipped.

  Counter-argument: a missing `.env` is *exactly* the case where the user needs to know — they'll wonder why their tooling doesn't work in the worktree. A warning that disappears is worse than a hard fail. The PRD names `.env` explicitly as the headline use case.

- Why it matters:
  Silent skip on a typo'd `copy = ["env"]` (missing dot) leaves the worktree silently un-configured.
- Recommendation:
  1. Move the copy-missing policy into Spec (Constraints).
  2. Reconsider the policy: lean toward hard fail (`Error::WorktreeCopySourceMissing { path }`). Users keep `copy` lists short; failing fast is cheap and matches the deep-tier intent.
  3. If "warn and skip" wins, document the warn channel explicitly and surface it in the Summary's Display impl (e.g. ` [skipped: .env not found]`).
  4. V-E-3 must assert the chosen behavior; add a paired V-F for the typo case.

### R-012 `Acceptance Mapping has gaps for G-7 and G-8`

- Severity: LOW
- Section: `## Validation` Acceptance Mapping
- Problem:
  - G-7 maps to V-IT-3, V-IT-4, V-E-4 — these cover atomic happy-path and rollback, but not the negative half ("`task new` without `--worktree` leaves the existing flow unchanged"). Add a V-UT/V-IT confirming `task new --slug x --tier deep` (no flag) writes nothing under `.ark/worktrees/`.
  - G-8 maps to "V-IT-1 followed by `task archive` test (asserts worktree intact)" — that test is described prose-only with no V-* ID. Promote it to V-IT-9.
  - V-IT-7's mapping is biased toward G-1 but the test description ("create a worktree with `.env` listed in `copy`") exercises G-3 (create) more than G-1 (config). Re-map.
- Why it matters:
  Acceptance mapping is the "every Goal has ≥1 Validation" gate (workflow.md PLAN). Loose mappings let goals slip uncovered.
- Recommendation:
  1. Add `V-IT-9: archive_does_not_remove_worktree` and map G-8.
  2. Add `V-UT-X: task_new_without_worktree_flag_makes_no_worktree_changes` and map G-7.
  3. Tighten V-IT-7's mapping (it should appear under G-3 as well as G-1).

## Trade-off Advice

### TR-1 `worktree.toml as managed file vs user-editable`

- Related Plan Item: T-6 / Q-1
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B (user-editable, not refreshed on upgrade)
- Advice:
  Treat `worktree.toml` as user-owned. `ark init` writes it from `templates/ark/worktree.toml`. `ark upgrade` does NOT touch it. Mirrors `workflow.md`'s treatment per current convention.
- Rationale:
  - The file's contents (`copy` lists, `post_create` hooks, `branch_prefix`) are *user-specific*, not Ark-specific. Refreshing on upgrade clobbers the very thing users edit.
  - There are no expected "shipped defaults that drift" — the defaults are essentially empty (`copy = []`, `post_create = []`). Nothing to refresh.
  - The `.claude/settings.json` precedent in `ark-context` C-17 is the inverse direction (Ark *re-applies* its hook entry unconditionally) — but there the entry is the Ark-owned identity, not user content. `worktree.toml` has no Ark-owned identity inside it; it is pure config.
- Required Action:
  Executor adopts: write Spec C-X stating `worktree.toml` is created by `ark init` from the template, never touched by `ark upgrade`, and is included in `ark unload`'s snapshot capture (it lives under `.ark/`, so this is automatic — confirm in V-IT).

### TR-2 `Recording branch verbatim vs parsed`

- Related Plan Item: Q-2
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (record verbatim)
- Advice:
  Adopt Q-2's lean. `task.toml.branch` is the literal `--branch <full>` value (or the computed `<type>/<slug>`); no parsing into a separate `branch_type` field on `task.toml`. PR-targeting needs `base_branch` only; the "type" is presentational.
- Rationale:
  - Parsing `--branch refactor/foo-bar` into `{type: "refactor", slug: "foo-bar"}` is a fragile string split that fails on `--branch user/initials/foo`.
  - Nothing downstream consumes `branch_type` independently; CLI-time validation is the only place the type matters (and that's at create time, not persisted).
  - Verbatim storage round-trips cleanly through `worktree list`.
- Required Action:
  Lock in Q-2's lean: branch stored verbatim. Drop any "parse into type+slug" logic from `create.rs`. Add a Spec constraint stating this so future readers don't try to add it.

### TR-3 `worktree list zero-row output`

- Related Plan Item: Q-3
- Topic: Flexibility vs Safety (UX vs convention)
- Reviewer Position: Prefer Option A (silent, exit 0)
- Advice:
  `worktree list` prints nothing when there are zero rows; exits 0. The "broken pipe vs no rows" distinction the plan invokes is a red herring — pipes carry exit codes already, and "no rows = empty output" is the standard Unix convention (`ls`, `git branch --list <pattern>` with no matches).
- Rationale:
  - Plan's Q-3 lean (`"no worktree-backed tasks"` to stderr) violates `ark-agent-namespace` G-6 / C-3: every command writes a one-line `Display`. A zero-row list is still one Display call — but writing user-facing prose to stderr is a new pattern, inconsistent with the rest of `agent`.
  - Other `agent` commands (`task new`, `task archive`) print a single line per call. `list` is the odd one out — fine for it to print 0..N lines.
  - Scriptability: `if [ -z "$(ark agent task worktree list)" ]; then ...` is the natural test. Adding stderr noise breaks that.
- Required Action:
  Spec states: `worktree list` prints `<row>\n` for each row; zero rows → empty stdout; exit 0. Drop the "no worktree-backed tasks" stderr write.

## Final Note

Three CRITICALs (R-001 self-contained Spec, R-002 worktree `.current` protocol, R-003 unload recursion) and four HIGHs (R-004 atomic rollback, R-005 error naming, R-006 clap exclusion, R-007 nesting guard, R-008 task-dir migration). The shape of the feature is right and the module decomposition is solid; what's missing is rigor around the worktree-vs-parent boundary.

Specifically, the plan needs:
1. Spec self-contained per the recent self-contained fix (R-001).
2. A concrete protocol for how the task dir crosses the worktree boundary, replacing the hand-wavy "discover_from already does it" claim (R-002, R-008).
3. Explicit specification of the unload interaction — placing worktrees inside `.ark/` is not free (R-003).
4. API hygiene fixes (R-004 atomic rollback, R-005 error naming, R-006 clap exclusion, R-007 nesting guard).

Re-iterate the PLAN with these issues addressed (copy `00_PLAN.md`/`00_REVIEW.md` to `01_PLAN.md`/`01_REVIEW.md`, bump iteration, reset `phase = "plan"`) and re-submit for review.
