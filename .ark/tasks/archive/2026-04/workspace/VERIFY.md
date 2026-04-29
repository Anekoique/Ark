# `workspace` VERIFY

> Status: Closed
> Feature: `workspace`
> Owner: Verifier (self-review with quality-pass cleanups)
> Target Task: `workspace`
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
- Non-Blocking Issues: 9 (8 fixed in this VERIFY pass; 1 deferred — see FU-002)

## Summary

Implementation delivers everything the final PLAN (`02_PLAN.md`) promised: identity opt-in via `ark init` + manual `workspace init`, journal append + index re-render, tier-agnostic auto-record on `task archive`, parent-write invariant from inside a worktree, `PathExt::append_text`, and the merged `.ark/config.toml`. All G-1..G-18 are implemented. All 324 ark-core lib tests pass; clippy clean with `-D warnings`; end-to-end smoke tests on `/tmp` pass.

The single SPEC-drift point — config file rename from `worktree.toml`/`workspace.toml` → `config.toml` — was an explicit user-directed change after EXECUTE began; the `02_PLAN.md ## Spec` and PRD were updated in lockstep, and the worktree-support feature SPEC will need a CHANGELOG entry on archive (handled by `spec_extract`).

Code-quality findings below were detected during this VERIFY pass and **fixed in-place** as part of the verification (per the user's directive to clean up redundant code during VERIFY). All fixes preserve behavior; no test failures introduced.

## Findings

### V-001 `open_fence` carries dead-code commentary

- Severity: LOW
- Scope: Quality
- Location: `crates/ark-core/src/commands/agent/workspace/journal.rs:189–199`
- Problem:
  The block `if rest.chars().any(|c| c == '`' || c == '~') { /* … 5-line comment, no body */ }` has no effect — the conditional executes nothing. The comment hand-waves about the heuristic being "conservative". The branch was a placeholder during iteration that never grew teeth.
- Why it matters:
  Reads as if there's logic the reader is missing. Future maintainers will spend time figuring out whether they broke an invariant by removing it. Pure noise.
- Expected:
  Delete the `if`. The function's actual semantics — "any line of `n` backticks or `n` tildes (n≥3) opens a fence" — are correct without it. Update the comment to state the chosen behavior crisply.
- Resolution: Fixed in this VERIFY pass.

### V-002 `is_close_fence` redundant length check

- Severity: LOW
- Scope: Quality
- Location: `journal.rs:208–221`
- Problem:
  The function does `if bytes.len() < state.len { return false }`, then iterates every byte requiring it equal the fence char, then returns `bytes.len() >= state.len` — which is now necessarily true since we passed the early guard. The trailing comparison is dead.
- Why it matters:
  Misleads the reader into thinking there's an additional length constraint after the loop. Three early-return-style checks of the same condition.
- Expected:
  Replace the loop body with `bytes.iter().all(|&b| b == state.char as u8)` and return that directly. Fewer moving parts.
- Resolution: Fixed.

### V-003 `find_active` `if let && match` chain reads awkwardly

- Severity: LOW
- Scope: Quality
- Location: `journal.rs:319–327`
- Problem:
  ```rust
  if let Some(n) = parse_journal_index(&name_str)
      && match highest {
          Some((h, _)) => n > h,
          None => true,
      }
  {
      highest = Some((n, entry.path()));
  }
  ```
  The `match` inside an `if-let-&&` is a workaround for a single-line max comparison. `Option::map_or` reads cleaner.
- Why it matters:
  Readability — the intent ("update if larger or first") gets buried in the syntax.
- Expected:
  ```rust
  if let Some(n) = parse_journal_index(&name_str) {
      let current = highest.as_ref().map_or(0, |(h, _)| *h);
      if n > current {
          highest = Some((n, entry.path()));
      }
  }
  ```
- Resolution: Fixed.

### V-004 `crate::error::Error::io` qualified paths in `journal.rs`

- Severity: LOW
- Scope: Quality
- Location: `journal.rs:316, 359` (two call sites)
- Problem:
  The file imports `use crate::{error::Result, ...}` but `Error` is referenced via fully-qualified `crate::error::Error::io`. Inconsistent with the rest of the workspace module which imports `Error` directly.
- Why it matters:
  Cosmetic but slightly noisier. Two sites becomes four if the file grows.
- Expected:
  Add `Error` to the existing `use crate::{...}` and use bare `Error::io`.
- Resolution: Fixed.

### V-005 `scan_session_count` re-parses every journal that `rerender` re-parses again

- Severity: MEDIUM
- Scope: Performance
- Location: `journal.rs:351–377` + `index.rs:57–109`
- Problem:
  `workspace_record` calls `scan_session_count` to assign the next session number, then later calls `index::rerender` which re-reads and re-parses every journal file. Two passes per record over the same content.

  Worst case bounded by C-21: 100 journals × 2000 lines = 200K lines parsed twice. Not catastrophic at expected workspace sizes (sub-millisecond), but pure redundancy.
- Why it matters:
  Both producers compute the same metadata (`total`, `last_date`). One pass would suffice.
- Expected:
  Refactor either by (a) making `rerender` return a `RerenderSummary { total, last_active }` and having `record_task`/`workspace_record` consume that, or (b) extracting a `parse_all_journals(layout, dev)` cache shared between the two producers.

  Either path requires reordering: the new entry's session number is embedded in its rendered text, so the count must be known *before* the journal write. Restructuring to write the entry first and re-render after needs the renderer to inject "live" entries on top of disk state — non-trivial and changes the on-disk-is-source-of-truth invariant.

  Cost/benefit at current expected scale: low. Documented as a follow-up for the next workspace task.
- Resolution: **Deferred.** Documented in FU-002 below. No code change in this VERIFY pass.

### V-006 `record_task` `summary_record` variable shadowing

- Severity: LOW
- Scope: Quality
- Location: `record.rs:164`
- Problem:
  Local variable named `summary_record` (a `WorkspaceRecordSummary`). The name reads like a verb; readers parse it as "record the summary" rather than "the summary record returned from `write_journal_and_index`". Subtle friction.
- Why it matters:
  Tiny, but every name budget matters. The variable is used twice (read `journal_path` and `session_number` from it) — destructure instead.
- Expected:
  ```rust
  let WorkspaceRecordSummary { journal_path, session_number, .. } =
      write_journal_and_index(...)?;
  Ok(WorkspaceRecorded::Recorded { journal_path, session_number })
  ```
- Resolution: Fixed.

### V-007 `parse_next_steps` strips `*` and ` ` greedily

- Severity: LOW
- Scope: Correctness
- Location: `record.rs:133–140`
- Problem:
  `trim_start_matches(['-', '*', ' '])` strips any leading run of dashes, asterisks, and spaces. Input `"-- bad"` becomes `"bad"` (acceptable); but `"* * nested"` becomes `"nested"` (not what the user wrote — they meant a literal `*` followed by a bullet?). For valid bullet lists (`- item`, `* item`) the behavior is correct. For pathological inputs it's lossy.
- Why it matters:
  Edge case; users typing markdown bullet lists won't hit it. Worth narrowing to "strip at most one leading bullet character + run of spaces" for predictable behavior.
- Expected:
  ```rust
  fn strip_bullet(s: &str) -> &str {
      let s = s.strip_prefix('-').or_else(|| s.strip_prefix('*')).unwrap_or(s);
      s.trim_start()
  }
  ```
  Idempotent for normal lists; loss-bounded for weird input.
- Resolution: Fixed.

### V-008 `lib.rs` re-exports `WorkspaceConfig as WorkspaceConfigToml`

- Severity: LOW
- Scope: Organization
- Location: `crates/ark-core/src/lib.rs:34`
- Problem:
  The `as WorkspaceConfigToml` rename was added defensively to avoid potential collision but there is no actual collision (`WorktreeConfig` lives separately in the same `task::` namespace; both names are distinct). The `Toml` suffix wrongly suggests this is a serde-only DTO when it's the in-memory config users may construct.
- Why it matters:
  Misleading name in the public surface.
- Expected:
  Drop the rename; export as `WorkspaceConfig`.
- Resolution: Fixed.

### V-009 `record_task` task-summary uses `{:?}` for `Tier`

- Severity: LOW
- Scope: Quality
- Location: `record.rs:174–179`
- Problem:
  ```rust
  format!("Archived `{}` ({:?}). See {} for the task artifacts.", opts.slug, opts.tier, ...)
  ```
  `{:?}` produces `Quick` / `Standard` / `Deep` — capitalized via Debug derive. The rest of the codebase uses lowercase serde representations (see `Tier`'s `#[serde(rename_all = "lowercase")]`). The journal entry text drifts from that convention.
- Why it matters:
  Cosmetic but readers comparing log lines to other ark output will notice the capitalization swing.
- Expected:
  ```rust
  let tier_str = match opts.tier {
      Tier::Quick => "quick",
      Tier::Standard => "standard",
      Tier::Deep => "deep",
  };
  ```
  Or implement `Display` on `Tier` (broader change; defer).
- Resolution: Fixed (inline match in `record.rs`; no `Display` impl on `Tier` so the change stays scoped).

## Follow-ups

Eight findings (V-001, V-002, V-003, V-004, V-006, V-007, V-008, V-009) were fixed in this VERIFY pass per the user's directive to clean up redundant code as part of verification. All cleanups preserve behavior; the 324-test suite still passes; clippy is clean under `-D warnings`; the end-to-end smoke (`ark init --developer alice` → `workspace record`) round-trips correctly.

One finding deferred:

- **FU-001** (optional doc fix): the worktree-support feature SPEC at `.ark/specs/features/worktree-support/SPEC.md` still references `worktree.toml` (the pre-merge filename). The deep-tier `spec_extract` will write a fresh `workspace/SPEC.md` from this task's `02_PLAN.md ## Spec`, but it does NOT touch sibling SPECs. A trivial quick-tier task can update worktree-support's SPEC text to reference `.ark/config.toml [worktree]`; or absorb it lazily next time worktree-support gets a real change.

- **FU-002** (V-005 deferred): refactor `scan_session_count` + `index::rerender` to share a single parse pass over the journal directory. Worth doing before workspaces grow large; not blocking at expected scale.
