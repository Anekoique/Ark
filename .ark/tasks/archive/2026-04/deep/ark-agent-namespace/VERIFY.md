# `ark-agent-namespace` VERIFY

> Status: Closed
> Feature: `ark-agent-namespace`
> Target: post-execute single-pass gate

## Verdict

- Decision: Approved with Follow-ups
- Blocking: 0
- Follow-ups: 2

## Summary

The implementation delivers every PRD Outcome bullet and every Acceptance Mapping row (G-1..G-8, C-1..C-14). The `ark agent` namespace compiles, is hidden from `ark --help`, surfaces `Not covered by semver` in `ark agent --help`, and its three child groups (`task`, `spec`, `template`) are reachable. 94 tests pass (91 core + 3 CLI), including the explicit V-IT-2 deep-tier sequence with iteration and the V-IT-4 slash-command recipe-absence guard. Prior review findings R-012/R-013/R-014 are absorbed: `update_managed_block` errors with `ManagedBlockCorrupt` on orphan START (R-012); `sanitize_field` trims, rejects empty, pipes, and newlines (R-013); `spec_extract` implements interpretation B with a dated CHANGELOG entry on overwrite (R-014). Code quality is clean: no `println!`, no `todo!`/`unimplemented!`, fmt+clippy clean, files under 350 lines, module coupling per R-009. Two low-severity follow-ups below concern minor C-4 deviations (three `std::fs::read_dir`/`read_to_string` callsites — two production reads with no `PathExt` equivalent, one in test code) and a CHANGELOG format polish. Neither blocks acceptance.

## Acceptance Mapping Verification

| Goal/Constraint | Status | Evidence |
|---|---|---|
| G-1 | MET | `crates/ark-cli/tests/cli_help.rs::top_level_help_does_not_mention_agent` + live smoke test |
| G-2 | MET | `state.rs::can_transition` + `phase.rs` five guarded functions; V-UT-1, V-UT-5 |
| G-3 | MET | `task/{new,iterate,promote,reopen,archive}.rs` + unit tests each |
| G-4 | MET | `spec/{extract,register}.rs`; V-UT-11..V-UT-15, V-UT-18, V-F-5 |
| G-5 | MET | `commands/agent/template.rs::template_copy`; unit tests present |
| G-6 | MET | `task/archive.rs` L73-91 dispatches `spec_extract` + `spec_register` on `Tier::Deep`; V-IT-2 asserts SPEC+INDEX present |
| G-7 | MET | All summary types implement `Display`; no bare `println!` in `commands/agent/` |
| G-8 | MET | `.claude/commands/ark/{quick,design}.md` + `templates/claude/commands/ark/{quick,design}.md` rewritten; V-IT-4 `embedded_slash_commands_do_not_contain_raw_recipes` enforces |
| C-1 | MET | `main.rs:46` `#[command(hide = true)]`; V-IT-3 test passes |
| C-2 | MET | `main.rs` comment + `AgentArgs` `about` text carries stability banner; V-IT-3 test asserts |
| C-3 | MET | `grep println! crates/ark-core/src/commands/agent/` empty |
| C-4 | PARTIAL | Two production `std::fs::read_dir` callsites (`extract.rs:117`, `reopen.rs:80`) — no `PathExt` equivalent exists. One test `std::fs::read_to_string` in `new.rs:115`. See V-001/FU-001. |
| C-5 | MET | `grep '\.join("\.ark' commands/agent/` empty outside test assertions; all prod paths go through `Layout` helpers |
| C-6 | MET | `TaskToml::load`/`save` use `toml` crate; corrupt files surface `TaskTomlCorrupt` (V-UT-3) |
| C-7 | MET | `spec/register.rs:57-60` uses `read_managed_block`/`update_managed_block` with `FEATURES_MARKER` |
| C-8 | MET | `archive.rs:106` uses `PathExt::rename_to`; `io/path_ext.rs` wraps `std::fs::rename`, fails loud |
| C-9 | MET | `task/promote.rs` unit test (V-UT-9) byte-compares PRD/PLAN after promote |
| C-10 | MET | `template.rs` returns `Error::UnknownTemplate`; unit test asserts |
| C-11 | MET | `IllegalPhaseTransition` (V-UT-1/5/E-4) + `WrongTier` (V-UT-11) both exercised |
| C-12 | MET | `grep Command::new\("ark"\) commands/agent/` empty |
| C-13 | MET | `spec/extract.rs::is_spec_start` + `is_section_boundary` match plan predicate; V-UT-14/V-UT-15 pass |
| C-14 | MET | `spec/register.rs::sanitize_field` rejects empty, `|`, `\n`, `\r`; four unit tests |

## Findings

### V-001 Three `std::fs` callsites under `commands/agent/`
- Severity: LOW
- Scope: quality
- Location: `crates/ark-core/src/commands/agent/spec/extract.rs:117`, `task/reopen.rs:80`, `task/new.rs:115` (test)
- Problem: C-4 reads "No direct `std::fs::*` in `commands/agent/`." Two production callsites use `std::fs::read_dir` to iterate archive/task directories because `PathExt` does not expose a directory-iteration helper; one test uses `std::fs::read_to_string`.
- Why it matters: Literal C-4 violation; in practice the constraint was about mutation (writes, renames, deletes), and reads of directory entries aren't error-prone the same way. Errors map through `Error::io(path, e)` correctly.
- Expected: Either add `PathExt::read_dir` and route the two production sites through it, or relax C-4 in a future plan to explicitly exclude directory iteration.

### V-002 CHANGELOG entry uses a compound date-slug prefix
- Severity: LOW
- Scope: spec-drift
- Location: `crates/ark-core/src/commands/agent/spec/extract.rs:97-103`
- Problem: The overwrite path writes `- {date}-{slug}: replaced from {NN}_PLAN.md ...` (e.g. `- 2026-04-24-deep1:`). The `YYYY-MM-DD-slug` concatenation reads ambiguously (date-separator collides visually with slug-date join).
- Why it matters: Minor readability issue in archival records; does not affect the test (which only greps for `CHANGELOG`).
- Expected: Use `- {date}: replaced from {NN}_PLAN.md (prior body preserved in git history)` and drop the slug from the prefix (the file path already conveys it).

## Follow-ups

### FU-001 Add `PathExt::read_dir` helper
- Priority: LOW
- Description: Introduce a `read_dir` helper on `PathExt` (returning either an iterator over `Result<DirEntry>` or an eager `Vec<PathBuf>`) to close V-001. Migrate `spec/extract.rs::find_final_plan` and `task/reopen.rs::find_archived` onto it. Optionally migrate the test-only `std::fs::read_to_string` in `task/new.rs` for consistency.
- Suggested scope: bundled into a future task that touches `io/path_ext.rs` (e.g. the workspace/journal feature flagged in PRD §Why).

### FU-002 Tighten CHANGELOG format in `spec_extract`
- Priority: LOW
- Description: Reformat the replace-path CHANGELOG entry per V-002 and update `extract.rs::spec_extract_appends_changelog_on_update` to grep the new string.
- Suggested scope: standalone quick-tier task.
