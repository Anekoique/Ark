# `worktree-support` REVIEW `01`

> Status: Closed (revisions accepted inline into 01_PLAN; no new iteration)
> Feature: `worktree-support`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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
- Blocking Issues: 0
- Non-Blocking Issues: 8

## Summary

Iteration 01 substantively addresses every CRITICAL and HIGH from `00_REVIEW.md`. The three CRITICALs are properly closed: Spec is now self-contained (R-001 fix is genuine — `[**Failure Modes**]` absorbs the load-bearing contracts that 00_PLAN had buried in Runtime); the worktree-first protocol (R-002 Option A) eliminates the `.current` divergence story by scaffolding the task dir inside the worktree from the start; and `walk_files_excluding(root, skip_under)` solves the unload recursion. Trade-offs locked in (TR-1 user-editable `worktree.toml`, TR-2 verbatim branch, TR-3 silent zero-row list) match the predecessor's positions. CLI mutual exclusion via clap `conflicts_with`/`requires`, the rename to `TaskAlreadyHasWorktree` / `WorktreeDirExists`, the nested-worktree guard, the parent-task collision error, and the hard-fail on missing copy source are all in.

What remains is MEDIUM/LOW residue and one factual error worth fixing before execute (the `upgrade.rs` claim — see R-101). None of the residue is blocking; most are short Spec sentences. Recommended path: executor accepts the revisions inline, no new iteration needed, proceed to EXECUTE.

## Findings

### R-101 `upgrade.rs does not call walk_files; the C-7 / G-10 claim is factually wrong`

- Severity: MEDIUM
- Section: `## Spec` G-10, C-7; Phase 1 step 5; V-IT-14
- Problem:
  Plan asserts (G-10, C-7, Phase 1.5, V-IT-14): `upgrade::extract` walks the filesystem with `walk_files` and therefore needs `walk_files_excluding(_, &[layout.worktrees_dir()])`. I grep'd `crates/ark-core/src/commands/upgrade.rs`: there is **no `walk_files` callsite there**. `upgrade::extract` enumerates the embedded `ARK_TEMPLATES` tree (`include_dir!`) via the `walk(tree)` helper at line 219, then resolves each template entry's destination via `layout.resolve(...)`. It never recursively reads the user's filesystem. The recursion-into-worktree risk exists in `unload.rs` (two callsites at lines 67 and 146 — `owned_dirs` loop *and* the Stage B `capture_orphan_hook_entries` loop) but not in `upgrade`.
- Why it matters:
  V-IT-14 (`upgrade_extract_excludes_worktree_contents`) is testing a property that's already trivially true for a different reason than the plan asserts — extract doesn't walk the filesystem. Either the test passes vacuously (and the constraint is misnamed) or the executor wires `walk_files_excluding` into a path that doesn't exist and the test fails to construct. Phase 1 step 5 instructs "Update `commands/upgrade.rs` to use `walk_files_excluding(_, &[layout.worktrees_dir()])`" — there's no such call to update.
- Recommendation:
  1. Drop `upgrade.rs` from G-10 / C-7 / Phase 1.5. Keep the constraint focused on `unload.rs`, where it has teeth.
  2. Drop V-IT-14 (or convert it into a property test that worktree files don't end up in the manifest's `files`/`hashes`, if there's a real risk surface — I don't see one).
  3. **Don't forget the second `walk_files` callsite at `unload.rs:146`** (`capture_orphan_hook_entries`). The plan only mentions the line-67 callsite implicitly via "`unload`'s snapshot capture". Both must pass the skip list, otherwise Stage B will scan every JSON file under every worktree's source tree for Ark hook identities. State this explicitly in C-7 ("both `walk_files` callsites in `unload.rs`").

### R-102 `walk_files_excluding skip-prefix semantics are unspecified`

- Severity: MEDIUM
- Section: `## Spec` G-10, C-7; `## Data Structure` (signature)
- Problem:
  Signature is `walk_files_excluding(root, skip_under: &[impl AsRef<Path>]) -> Result<Vec<PathBuf>>`. The plan says "prunes any subtree whose path starts with one of the skip prefixes" but does not pin:
  - Are skip prefixes resolved as absolute paths before the prefix-match, or compared lexically?
  - Looking at `io/fs.rs::walk_files`: it pushes `root.to_path_buf()` onto a stack and descends via `read_dir`. The `path` it yields is whatever `read_dir` produced — i.e. it has the form `<root-as-given>/...`. If `root` is relative (e.g. `"./foo"`) and `skip_under` is the absolute `/abs/foo/.ark/worktrees`, lexical `starts_with` will not match.
  - The unload callsite passes `layout.owned_dirs()` (absolute, since `Layout` carries the project root), and would pass `layout.worktrees_dir()` (also absolute). So in practice they'll both be absolute. But the contract doesn't *say* that.
  - A second wrinkle: `walk_files`'s yielded paths are not canonicalized (no symlink resolution). If the user's `cfg.worktree_dir` points at a path that resolves through a symlink, lexical match fails. Probably fine for the .ark/worktrees default; worth a constraint regardless.
  - The plan also leaves "`walk_files` becomes a thin wrapper *or* the zero-skip case" as an either/or. Pin one — it's a Spec issue (call-site stability), not implementation freedom.
- Why it matters:
  C-7 is the load-bearing safety guarantee of the feature. The contract needs to be unambiguous so the executor and reviewers can verify it. "Lexical prefix match on absolute paths, no symlink canonicalization" is a fine answer; just say so.
- Recommendation:
  1. Add a sentence to C-7: "`walk_files_excluding` performs lexical `Path::starts_with` against the caller-supplied prefixes; both `root` and `skip_under` entries must be absolute paths. No symlink resolution is performed."
  2. Pin the wrapper question: e.g. "the existing `walk_files(root)` becomes `walk_files_excluding(root, &[] as &[PathBuf])`". (Either choice works; just commit.)
  3. Add V-UT: `walk_files_excluding_skip_is_lexical` (asserts that a skip prefix that doesn't lexically match — e.g. relative-vs-absolute — does NOT prune).

### R-103 `task.toml.worktree_path absolute-vs-relative is unspecified`

- Severity: MEDIUM
- Section: `## Spec` G-3 step 10, G-6, Data Structure (TaskToml)
- Problem:
  G-6 declares `worktree_path: Option<PathBuf>` with no statement on what's stored in the path. G-3 step 10 says "write `task.toml` with `... worktree_path = Some(<worktree_path>)`" where `<worktree_path>` was computed as `<root>/<cfg.worktree_dir>/<branch>/` — i.e. absolute.
  - **If absolute**: the user moves the repo (rename containing dir, clone elsewhere) and `task.toml.worktree_path` now points to a non-existent absolute path. Worse, an `unload` snapshots that absolute path; `load` after a relocation restores the absolute path and any consumer reading it post-restore points to the wrong place.
  - **If project-relative**: cleanup discovery (C-20) doesn't actually use `task.toml.worktree_path` — it walks `git worktree list --porcelain`. So the cost is minor: just resolve relative-to-`layout.root()` when reading.
  - `worktree list` output (G-5) prints `worktree_path` as a path string; absolute-vs-relative changes the user-visible output.
  - Same question silently applies to `task.toml.base_branch` (just a branch name string, not load-bearing here) and `branch` (string).
- Why it matters:
  An unload/load round-trip is a critical-path command pair. Snapshotting absolute paths into a portable `.ark.db` violates the snapshot's portability invariant. Worth pinning in Spec.
- Recommendation:
  1. Add a Spec sentence (Data Structure or new C-21): "`task.toml.worktree_path` stores the worktree path as **project-relative** (e.g. `.ark/worktrees/feat/foo`). Consumers that need an absolute path resolve it against `layout.root()`."
  2. Update `WorktreeRow.worktree_path` accordingly, or document that `worktree list` prints it absolute by joining at format time.
  3. Add V-UT or V-IT: assert `task.toml.worktree_path` is project-relative after `task new --worktree`.

### R-104 `Worktree's .ark/ depends on base_branch's HEAD; not surfaced as a failure mode`

- Severity: MEDIUM
- Section: `## Spec` G-3 step 9-10, G-9; `[**Failure Modes**]` (missing)
- Problem:
  `git worktree add -b <branch> <wt> <base_branch>` checks out `base_branch`'s HEAD commit into the worktree. The worktree's `.ark/` is whatever exists in *that commit*. G-3 step 10 then layers `<wt>/.ark/tasks/<slug>/` and `<wt>/.ark/tasks/.current` on top — but only the `tasks/` subtree. Everything else under the worktree's `.ark/` (`workflow.md`, `templates/`, `specs/`, `worktree.toml`) is the base_branch's view.
  - If `base_branch` doesn't have Ark loaded (e.g. user ran `task new --worktree` with `base_branch = main` and main is pre-`ark init`), the worktree won't have `.ark/workflow.md`, `.ark/templates/`, etc. Inside the worktree, `Layout::discover_from` will still find `<wt>/.ark/` (because step 10 created `.ark/tasks/<slug>/`, so `.ark/` exists), but `ark agent task plan` invoked there will fail when it tries to copy a template that doesn't exist.
  - The plan's G-9 claim "`Layout::discover_from(cwd)` correctly resolves the worktree as the project root — because the worktree has its own `.ark/`" is true but mildly misleading. It has a *partial* `.ark/`.
  - F-16 documents the unborn-HEAD case, but doesn't document the more common "base_branch lacks Ark" case.
- Why it matters:
  This is a real failure mode users will hit. It's not a CRITICAL because Ark is typically loaded on the user's working branch, but it's a "easily reachable, confusing error" footgun.
- Recommendation:
  1. Add F-17 to `[**Failure Modes**]`: "If `base_branch` does not have Ark loaded (no committed `.ark/workflow.md`), the worktree starts without templates/specs. `task new --worktree` succeeds (creates the task dir under `.ark/tasks/`), but downstream `ark agent task plan` etc. fail when they look for `.ark/templates/PLAN.md`. Mitigation: ensure `.ark/` is committed on `base_branch` before invoking `task new --worktree`."
  2. Optionally guard at the top of G-3 by checking that `<base_branch>` resolves to a commit containing `.ark/workflow.md` (treeish probe via `git cat-file -e <base>:.ark/workflow.md`). Reject early with `Error::BaseBranchLacksArk` if absent. Cheap and turns a confusing late failure into a clear early one.

### R-105 `Detached HEAD on the parent is not addressed; F-16 only covers unborn HEAD`

- Severity: MEDIUM
- Section: `## Spec` G-3 step 5; F-16
- Problem:
  G-3 step 5: `run_git(["symbolic-ref", "--short", "HEAD"], root)`. `git symbolic-ref` only succeeds if HEAD is a symbolic ref; on detached HEAD (e.g. user checked out a tag or a SHA), it errors with "fatal: ref HEAD is not a symbolic ref". F-16 mentions "unborn HEAD" (no commits yet) but not detached HEAD. Different error class, different user remedy.
  - Detached HEAD is common — e.g. user runs `git checkout v0.1.2` to inspect a release, then tries `task new --worktree`. They'll get an opaque `run_git` error.
  - Either accept the symbolic-ref failure as the documented behavior (with a clearer error variant), or fall back to `git rev-parse HEAD` and store the SHA as `base_branch` (verbatim — TR-2 spirit applies).
- Why it matters:
  Predictable failure mode that an executor might silently inherit as an opaque `run_git` Io error. Better to name it and decide.
- Recommendation:
  1. Extend F-16: "Detached HEAD: `git symbolic-ref --short HEAD` fails. Surface as `Error::DetachedHead` with a hint to checkout a branch first, or document that `--branch <full>` plus an explicit `--base-branch` (future flag) would be needed. For now, reject."
  2. Or: fall back to `git rev-parse HEAD` and store the 40-char SHA in `task.toml.base_branch` verbatim. Simpler, no new error variant, matches the verbatim-string spirit of C-12.

### R-106 `F-7 (TaskAlreadyHasWorktree retained "for defensive use") violates dead-code principle`

- Severity: LOW
- Section: `## Spec` Data Structure (errors); F-7
- Problem:
  F-7: "Mooted under R-002 — the standalone path is dropped. Retained as `Error::TaskAlreadyHasWorktree` for defensive use if future code paths need it; no current call site."
  - Speculative additions to a public error enum (the variant is `pub` via the library re-export) are forward-incompatible: removing it later is a breaking change. Either there is a path that needs it (specify the path), or it shouldn't exist yet.
  - The user CLAUDE.md / coding-style.md "Code Quality Checklist" calls out: "Code is readable and well-named" — a `pub` variant with no call site is the opposite.
- Why it matters:
  Tech debt up front. The plan is otherwise rigorous about pruning unused surface.
- Recommendation:
  Drop `Error::TaskAlreadyHasWorktree` from this iteration. If a future task adds the standalone path, the variant comes back with the call site at the same time. F-7 then collapses into "no call site; not present."

### R-107 `task.toml first commit on the worktree's branch — no constraint about whether it's expected`

- Severity: LOW
- Section: `## Spec` G-3 step 10; State Transitions
- Problem:
  After step 10-13, the worktree's `.ark/tasks/<slug>/`, PRD.md, task.toml, and `.current` are uncommitted edits on the new branch. `cfg.copy` files are uncommitted too. The plan doesn't say whether the *first commit* on the new branch is expected to include the scaffolding (e.g. `chore: scaffold ark task <slug>`) or whether the user merges those into their first feature commit. Either choice is fine, but it affects PR review aesthetics — a 200-line scaffolding diff in the same commit as the first feature change is noisy.
  - This is a style/convention question, not a correctness one. The plan says nothing, which is probably correct (it's the user's call), but it's worth a one-line State Transition note: "The scaffolding lands as uncommitted edits on the new branch. The user decides whether to include it in their first feature commit or land it as a separate `chore:` commit."
- Why it matters:
  Documentation completeness; UX clarity. Not load-bearing.
- Recommendation:
  Add one sentence to State Transitions: "After `task new --worktree`, the scaffolding (`.ark/tasks/<slug>/`, `cfg.copy` files) is uncommitted on the new branch. Convention is left to the user."

### R-108 `Cleanup discovery's silent skip on Ark-unaware worktree may surprise users`

- Severity: LOW
- Section: `## Spec` G-4 step 1; C-20; F-9
- Problem:
  C-20: "If no worktree's `.current` matches the slug, `Error::WorktreeNotFound { slug }`." `worktree list` (G-5) silently skips worktrees whose `.current` or `task.toml` is unreadable (`.ok()?` in the call graph). That's correct for *third-party worktrees* (created by `git worktree add` outside Ark). It's also correct for Ark worktrees that the user has corrupted (deleted `.current` manually, etc.) — but in that case silent skip leaves the user wondering why their worktree isn't listed.
  - This is a UX trade-off, not a correctness one. The plan picks "silent skip" implicitly via `.ok()?`. An alternative: emit a single-line `[skipped: <wt>: malformed]` diagnostic. Or just leave it silent and document it.
- Why it matters:
  Debug-friendliness; minor.
- Recommendation:
  Add a Spec sentence to G-5: "Worktrees whose `.ark/tasks/.current` or `task.toml` is missing/unreadable are silently skipped. This is the documented behavior for non-Ark third-party worktrees living under `worktrees_dir()`."

## Trade-off Advice

(No new trade-offs in 01_PLAN. The three carried over from 00 — TR-1, TR-2, TR-3 — are all locked per the predecessor's recommendations. T-7 and T-8 (new in 01) document decisions already made and don't request advice.)

### TR-1 `Detached-HEAD handling`

- Related Plan Item: F-16, R-105
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option B (fall back to SHA)
- Advice:
  When `git symbolic-ref --short HEAD` fails on a detached HEAD, fall back to `git rev-parse HEAD` and record the 40-char SHA verbatim as `task.toml.base_branch`. Don't add a new error variant.
- Rationale:
  - Storing the SHA verbatim matches C-12's "branch verbatim" spirit (the field is named `base_branch` but is type `Option<String>` — semantically a "git ref I should track from").
  - Detached-HEAD is common during release inspection workflows; giving the user a clean path through is friendlier than rejecting.
  - PR-targeting (the original motivation for `base_branch`, per PRD) gets degraded but not broken: the user still has the SHA they were on; tooling that wants to target a branch can prompt later. We're not optimizing for that yet.
- Required Action:
  Executor: change G-3 step 5 to "`git symbolic-ref --short HEAD` if it succeeds; else `git rev-parse HEAD` and store the SHA". Update F-16 accordingly. No new error variant.

## Final Note

Iteration 01 is a substantial recovery from a tough first-round review. The three CRITICALs are properly closed; the protocol is now coherent (Option A worktree-first); Spec is genuinely self-contained — copying the `## Spec` block verbatim into `specs/features/worktree-support/SPEC.md` would yield a readable standalone document; Failure Modes catalog F-1..F-16 is a notable improvement over 00_PLAN's Runtime block.

The MEDIUM/LOW residue is mostly polish: drop the wrong upgrade.rs claim (R-101), pin walk_files_excluding semantics (R-102), pin path representation (R-103), document base_branch's Ark-loaded assumption (R-104), handle detached HEAD (R-105), drop dead error variant (R-106). Most are one-sentence Spec edits; none warrant a third iteration.

Verdict: **Approved with Revisions**. Executor merges the above inline (no new iteration), proceeds to EXECUTE.

For workflow.md compliance: every prior CRITICAL/HIGH (R-001..R-008) appears in 01_PLAN's Response Matrix; MEDIUM/LOW (R-009..R-012) and TR-1..TR-3 also appear. No prior finding was missed.
