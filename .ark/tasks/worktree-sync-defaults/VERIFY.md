# `worktree-sync-defaults` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `worktree-sync-defaults`
> Target Task: `worktree-sync-defaults`
> Tier: standard

---

## Project Spec Compliance

- [x] LAYOUT.md: PASS — no new layout deviations; identity-sync code lives under existing `commands/agent/task/` tree.
- [x] rust/COMMENTS.md: PASS — `sync_identity` carries one doc comment on the `WHY` (prompt branch + non-TTY contract); helper updates carry no extraneous narration.
- [x] rust/STYLE.md: PASS — uses `?` and explicit `match` for error mapping; no `unwrap` in the new code; no `mut` where unnecessary.
- [x] rust/ERRORS.md: PASS — reuses existing `Error::MissingIdentity` and `Error::DeveloperWriteFailed`; no new variants introduced.

## Related Feature Spec Compliance

- [x] .ark/specs/features/worktree/SPEC.md: PASS — G-1 amended (`post_create` default now `["git submodule update --init --recursive"]`, file path corrected from stale `.ark/worktree.toml` to `.ark/config.toml [worktree]`), G-13 added (identity-sync goal), NG-7 refined to permit documented-default submodule hooks while keeping code submodule-agnostic.
- [x] .ark/specs/features/workspace/SPEC.md: PASS — workspace identity module is reused unchanged. No edits to the workspace feature.

## PRD Constraints

- [x] `task new --worktree` resolves identity from parent and mirrors into worktree (V-IT-1).
- [x] Non-TTY + missing parent identity returns `Error::MissingIdentity` (V-IT-2).
- [x] `WorktreeConfig::default().post_create` includes submodule init (V-UT-1, V-IT-3).
- [x] `cfg.copy` semantics preserved (V-E-2 `worktree_copy_missing_source_hard_fails_and_rolls_back` still passes — see test output below).
- [x] `templates/ark/config.toml` updated to active `post_create = ["git submodule update --init --recursive"]` (V-UT-2).
- [x] All existing worktree tests pass (`cargo test -p ark-core`: 402 passed; 0 failed).

## Plan Fidelity

- [x] **G-1:** default `post_create` includes submodule init: PASS — `default_post_create()` returns the command; `WorktreeConfig::default()` uses it. Verified by `worktree_config_default_post_create_has_submodule_init`.
- [x] **G-2:** identity-sync step inserted between `register_focus` and the copy loop, inside the rollback boundary: PASS — see `new.rs:289–294`. Failure path `MissingIdentity` triggers existing `inspect_err → rollback_worktree`. Verified by `worktree_creation_mirrors_parent_identity` + `worktree_creation_fails_on_missing_identity_when_non_tty`.
- [x] **G-3:** TTY detection via `std::io::IsTerminal::is_terminal(&stdin())`: PASS — see `sync_identity` in `new.rs`. Verified by V-IT-2 (non-TTY branch in cargo-test process).
- [x] **G-4:** template `[worktree]` updated with active `post_create = ["..."]`: PASS — `templates/ark/config.toml` re-read, comment text reflects automatic identity sync. Verified by `worktree_template_default_matches_code`.
- [x] **G-5:** Worktree SPEC additions (G-13 + G-1/NG-7 amendments): PASS — applied directly to `.ark/specs/features/worktree/SPEC.md`.
- [x] **G-1 amended (template default):** PASS — same as G-4.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — `.ark/specs/features/worktree/SPEC.md` was amended in place; existing convention in this repo's feature SPECs is amend-without-changelog (no SPEC carries a CHANGELOG section).

## Findings

### V-001 SPEC amendment for worktree feature

- **Severity:** MEDIUM
- **Location:** `.ark/specs/features/worktree/SPEC.md`
- **Problem:** G-1 (`post_create` default) and NG-7 (submodule policy) drifted from implementation; G-13 (identity sync) was missing.
- **Why it matters:** Feature SPECs are the source of truth.
- **Recommendation:** Amend G-1, NG-7; insert G-13.
- **Resolution:** FIXED in this task — see `.ark/specs/features/worktree/SPEC.md` G-1, G-13, NG-7. Also corrected stale `.ark/worktree.toml` reference in G-1 to `.ark/config.toml [worktree]` (the workspace consolidation already happened upstream).

### V-002 Test process TTY behavior is platform-dependent

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/new_tests.rs::worktree_creation_fails_on_missing_identity_when_non_tty`
- **Problem:** Test relies on `cargo test`'s stdin not being a TTY. On most CI and local invocations this is true. If a developer runs `cargo test -- --nocapture` from an interactive shell with stdin attached to a terminal, the test would block on the prompt instead of failing fast.
- **Why it matters:** Flaky in unusual local invocations.
- **Recommendation:** Future work could inject an `IsTerminal` trait or use a process spawn to guarantee non-TTY, but the simpler fix is documenting the constraint.
- **Resolution:** ACCEPTED — documented in this finding; matches how other identity prompt tests (in `identity.rs`) handle the same concern.

## Follow-ups

*None.*

## Notes

- Test output: `cargo test -p ark-core` → 402 passed, 0 failed. `cargo clippy --all-targets` clean.
- Identity sync intentionally writes the prompt's stderr output: keeps stdout clean for the `TaskNewSummary` `Display`, which agents parse.
- `identity_prompt` is reused via the `commands::agent::workspace::identity` path (not re-exported in `mod.rs`'s `pub use` block; not worth adding a re-export for one internal call site).
