# `worktree-support` VERIFY

> Status: Closed (revisions accepted; V-002 resolved post-verify)
> Feature: `worktree-support`
> Owner: Verifier (self-verify pass — executor-as-reviewer per workflow §5)
> Target Task: `worktree-support`
> Verify Scope:
>
> - Plan Fidelity        — does the code deliver what the final PLAN promised?
> - Functional Correctness — does it work under the Validation matrix?
> - Code Quality         — readability, naming, error handling, test depth
> - Organization         — module boundaries, file placement, cohesion
> - Abstraction          — appropriate abstractions; no premature, no leaky
> - SPEC Drift           — does PLAN's Spec section still match the shipped code?

---

## Verdict

- Decision: Approved with Follow-ups
- Blocking Issues: 0
- Non-Blocking Issues: 3 (was 4; V-002 resolved post-verify per user feedback)

## Summary

The implementation delivers what 01_PLAN promised across all G-1..G-12 and C-1..C-21 gate items. Phase 1 (layout/config/walk_files/state), Phase 2 (`task new --worktree` end-to-end with rollback, including the worktree-first protocol of C-2/G-3), and Phase 3 (cleanup/list discovery via `git worktree list --porcelain` and parent-prune per C-15) all landed. **27 new tests** were added against a baseline of 226 (now 253 in ark-core, plus 20 in ark-cli, all green); coverage hits every V-IT-* in the acceptance matrix except V-IT-14 (which 01_PLAN explicitly removed) and V-IT-3 / V-IT-9 / V-IT-11 / V-IT-12 / V-IT-13 / V-IT-15, which all have direct test names mapping into the implementation.

The four non-blocking findings below cover scope-cuts the executor made deliberately during EXECUTE (none reverse a plan decision; all are surfaced for the user to acknowledge or convert into follow-up tasks):

1. **V-001** — F-18's optional probe (`Error::BaseBranchLacksArk`) was added to the error enum but **no call site was wired**. The executor judged the failure mode acceptable to surface as a downstream `task plan` error rather than an early-reject. Per 01_PLAN's F-18 explicit "decision deferred to executor", this is consistent.
2. **V-002** — Per 01_PLAN G-11, `.ark/worktrees/` should be added to a managed `.gitignore` block on `init`/`upgrade`. **The executor scope-cut this**: no `.gitignore` integration today; instead the shipped `worktree.toml` template documents that users must add the line manually. Reasoning: the project has zero existing `.gitignore` management surface, and adding one cleanly (managed-block, init+upgrade idempotent, snapshot-aware) is meaningful work that deserves its own task.
3. **V-003** — `process-spawn locality (C-10)` was satisfied by adding `run_shell` to `io/git.rs` (the existing sanctioned spawn module), expanding the comment to read "sanctioned subprocess spawns" and listing both git and shell as clients. This is a constraint-edit at execute time; documenting it here for SPEC drift transparency.
4. **V-004** — `Error::TaskAlreadyHasWorktree` was dropped per R-106 (no call site). 01_PLAN's Data Structure block lists the renamed variant; the live `error.rs` does not. SPEC-vs-code consistent only if a reader notices the F-7 "removed" footnote in 01_PLAN.

Recommend approving with follow-ups: convert V-002 into a future `gitignore-managed-block` task, leave V-001/V-003/V-004 as documented decisions.

## Findings

### V-001 `BaseBranchLacksArk error variant has no call site`

- Severity: LOW
- Scope: SPEC Drift / Plan Fidelity (intentional deferral)
- Location: `crates/ark-core/src/error.rs:140`, `commands/agent/task/new.rs` (no probe call site)
- Problem:
  01_PLAN's F-18 says: *"Probe via `git cat-file -e <base>:.ark/workflow.md`; non-zero → `Error::BaseBranchLacksArk`. Decision deferred to executor."* The variant was added to `error.rs`; the probe was not wired into `task_new_with_worktree`. A user creating a worktree off a branch that lacks Ark loaded will succeed at create-time but get an opaque `template not found` error from `task plan` later.
- Why it matters:
  Confusing late failure. F-18 explicitly flagged "skipping the probe is the lighter choice" as acceptable; the executor took the lighter path. No correctness loss, only UX.
- Expected:
  Either accept as-is and trim the variant, OR add the probe + V-IT for the rejected case. Recommend a follow-up task: `worktree-base-branch-probe`.

### V-002 `.gitignore managed-block integration` ✅ RESOLVED post-verify

- Severity: ~~MEDIUM~~ — RESOLVED
- Scope: Plan Fidelity
- Resolution:
  Per user feedback during verify, the executor implemented `.gitignore` managed-block management instead of deferring. Added `update_gitignore_block` and `remove_gitignore_block` to `io/fs.rs` (using `# ARK:START` / `# ARK:END` line-comment markers since `<!-- … -->` is invalid in `.gitignore`). Wired into `init` (write block + record manifest), `upgrade` (re-apply unconditionally per the `CLAUDE.md` precedent), `unload` (dispatch by filename — gitignore uses the line-comment removal helper), `load` (re-apply canonical body even for legacy snapshots), `remove` (dispatch by filename). Added 8 unit tests (`update_gitignore_block_*`, `remove_gitignore_block_*`) and 2 integration tests (`init_writes_gitignore_managed_block`, `init_preserves_existing_gitignore_content`). Existing tests for `unload_captures_and_removes` and `load_restores_from_snapshot` updated to assert the new behavior. `templates/ark/worktree.toml` updated to drop the manual-gitignore note.

### V-003 `run_shell added to io/git.rs broadens module scope`

- Severity: LOW
- Scope: Organization / Abstraction
- Location: `crates/ark-core/src/io/git.rs:1-9`, `:60-78`
- Problem:
  C-10 says "All git invocations route through `io::git::run_git`. `Command::new` may NOT appear under `commands/agent/task/worktree/`." To execute `worktree.toml`'s `post_create` shell hooks, the executor added `run_shell` to `io/git.rs` and updated the module's docstring to read "sanctioned subprocess spawns" with two clients (git, shell). The source-scan test (`commands_no_bare_command_new`) still passes because `Command::new` only appears in `io/git.rs`. But the module name no longer matches its scope.
- Why it matters:
  Drift between module name and contents is a maintainability tax. Future readers will look for shell hooks under a hypothetical `io/shell.rs`. Renaming `io/git.rs` → `io/spawn.rs` (or splitting into `io/{git,shell}.rs` with a single module-level allowlist for the scan test) would tighten organization.
- Expected:
  Either rename `io/git.rs` → `io/spawn.rs` in a follow-up, or split the file. Low-priority cleanup; no behavioral change.

### V-004 `TaskAlreadyHasWorktree variant referenced in 01_PLAN data structure but absent from error.rs`

- Severity: LOW
- Scope: SPEC Drift
- Location: 01_PLAN `## Spec` Data Structure block; `crates/ark-core/src/error.rs`
- Problem:
  01_PLAN's Data Structure block (after the inline R-106 revision) shows the rename `WorktreeAlreadyExists → TaskAlreadyHasWorktree`. R-106's resolution row in `## Log` says "Dropped `Error::TaskAlreadyHasWorktree` from the error enum entirely. F-7 marked removed." Live `error.rs` does NOT include the variant — correct per R-106 — but a reader of just the Data Structure block (after archive's `spec extract` runs) might expect it. F-7's "Removed" footnote disambiguates.
- Why it matters:
  Spec-vs-code consistency. After `spec extract`, the future `specs/features/worktree-support/SPEC.md` will copy the Data Structure verbatim. R-106 footnote is in `[**Failure Modes**]` (also in Spec), so a careful reader connects the dots. Imperfect but spec-extract-survivable.
- Expected:
  Edit 01_PLAN's Data Structure block to drop `Error::TaskAlreadyHasWorktree` from the listed variants before archive, since R-106 removed it. Or accept the F-7 cross-reference as adequate. (Lean: leave for archive cleanup.)

## Follow-ups

- ~~FU-001: `gitignore-managed-block`~~ — RESOLVED inline post-verify (see V-002).
- FU-002: `worktree-base-branch-probe` — add the F-18 probe (`git cat-file -e <base>:.ark/workflow.md`) and an early `Error::BaseBranchLacksArk` reject. Removes V-001. (Optional — current behavior is acceptable.)

## Post-verify amendments (per user feedback)

- **Workflow doc integration.** The `### Worktree (optional)` appendix under §6 was replaced with inline references at the natural touchpoints: §3 Tiers gains a paragraph on `--worktree`, §4 DESIGN's `task new` recipe gains the flag, §4 EXECUTE gains a "Worktree note" subsection on `cd`-ing into the worktree and post-merge cleanup, §6 Mechanics gains a tighter "Worktree commands" line referencing §3 and §4. The standalone subsection is gone. Mirrored in both `templates/ark/workflow.md` and `.ark/workflow.md`.
