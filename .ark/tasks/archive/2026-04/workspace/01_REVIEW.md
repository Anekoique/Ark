# `workspace` REVIEW `01`

> Status: Closed
> Feature: `workspace`
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
> - Verification of prior findings

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 1
- Non-Blocking Issues: 4

## Summary

The revisions resolve all four HIGH findings and all three MEDIUM findings from `00_REVIEW.md` cleanly. The G-2 / G-8 / NG-12 / PRD harmonization is correct (R-001), the tier-agnostic placement with C-18 is correct (R-002), Acceptance Mapping is now semantically honest (R-003), the line citation is gone (R-004), C-19 specifies a precise entry-boundary predicate (R-005), C-7 names both walk sites (R-006), the G-6 fallback chain is explicit (R-007). The new C-20 (parent-resolution algorithm) and C-21 (re-render hard ceiling) are well-formed.

One HIGH issue: G-4 step 5 and the manual record call graph (line 350) reference `target_journal.append_text(&entry)?` and `PathExt::append_text` — but `append_text` does not exist in `PathExt` (verified — `PathExt` has `read_optional`, `read_text_optional`, `read_bytes`, `read_text`, `write_bytes`, `ensure_dir`, `list_dir`, `remove_*`, `rename_to`, `hash_sha256`; no append). The `## Spec` will be extracted to `specs/features/workspace/SPEC.md` claiming an API that doesn't exist. Either add `append_text` to `PathExt` (and call it out in Phase 1 / Data Structure / API Surface) or change the journal write to read-modify-rewrite via `read_text_optional` + `write_bytes`. The latter is simpler and consistent with C-2 (no bare `std::fs::*`), but slower for large journals — though the rotation cap bounds it.

The remaining four findings are MEDIUM/LOW. No new HIGH or CRITICAL issues introduced by the revisions. The `## Spec` section is now self-contained — one final audit pass confirms no external file-line references remain, and Spec doesn't depend on `## Log` knowledge.

---

## Findings

### R-001 `PathExt::append_text` does not exist

- Severity: HIGH
- Section: `[**Goals**] G-4` step 5; `[**Architecture**]` call graphs for `workspace_record` and `task archive`; `[**Data Structure**] journal.rs` (implicit dependency on append semantics)
- Problem:
  G-4 step 5 says "Render the session entry per G-5 and append (open with append mode via `PathExt::append_text`)." The manual record call graph echoes: `target_journal.append_text(&entry)?`. The `PathExt` trait (verified at `crates/ark-core/src/io/path_ext.rs`) declares no `append_text` method. Available write API is only `write_bytes(&self, contents: &[u8]) -> Result<()>` — full overwrite semantics.

  The call graph's append idiom is therefore unimplementable as-stated. An implementer reading the SPEC after archive will either (a) fabricate an `append_text` method without checking; (b) use `std::fs::OpenOptions::new().append(true).open(...).write_all(...)` directly, violating C-2 ("All filesystem access in `commands/agent/workspace/` routes through `io::PathExt`"); or (c) read-modify-rewrite via `read_text_optional` + `write_bytes`. Each path has different correctness/perf tradeoffs and the SPEC currently doesn't pick one.

  The `## Spec` ambiguity is the rejection trigger here. Once promoted to `specs/features/workspace/SPEC.md`, the API claim becomes load-bearing for future readers.
- Why it matters:
  - The PLAN claims an API (`append_text`) that doesn't exist. The promoted SPEC will mislead.
  - C-2 binds workspace code to `PathExt`. Without an append primitive, the journal write either violates C-2 (raw `OpenOptions`) or pays an O(file_size) read-rewrite cost per record.
  - For the rotation cap of 2000 lines × ~30-byte avg entry = ~60KB per journal — a read-rewrite is fine in practice. But that decision needs to be in the SPEC.
- Recommendation:
  Choose one and lock it in:
  - **Option A (recommended):** Add `fn append_text(&self, contents: &str) -> Result<()>` to `PathExt` in Phase 2 (alongside the journal module). Documented as: opens `OpenOptions::new().create(true).append(true)`. Add a Data Structure entry under `PathExt additions`. Update C-2 footnote to mention the new API. Add a unit test.
  - **Option B:** Remove the `append_text` reference; describe journal append as read-modify-rewrite using `read_text_optional` + `write_bytes`. Update G-4, the call graph, and add a perf note acknowledging the O(file_size) cost is bounded by `journal_max_lines` × ~50 bytes ≈ 100KB worst case.

  Either way, the PLAN's `[**Data Structure**]` and `[**API Surface**]` sections must reflect reality.

### R-002 `parse_oneline` helper is referenced but undefined

- Severity: MEDIUM
- Section: `[**Architecture**]` call graphs for `workspace_record` and `task archive` auto-record
- Problem:
  Both call graphs use `parse_oneline(o.stdout)` to convert `git log --oneline` output to `Vec<JournalCommit>`. The function isn't declared in `[**Data Structure**]` (`journal.rs`'s public API lists `render_entry`, `parse_entries`, `find_active`, `line_count`, `scan_session_count`, `seed_journal` — no `parse_oneline`). Whether it's a private helper in `journal.rs` or `record.rs` is not specified.

  Minor on its own, but `parse_oneline` has subtle correctness questions: does it strip leading/trailing whitespace? How does it handle empty stdout? Multi-line commit messages (rare with `--oneline` but possible with quoted shell args)? The split between `<short_hash> <message>` is the first space — but commit hashes can contain only `[0-9a-f]` so this is safe — yet not documented.
- Why it matters:
  Implementer guesswork. Future reviewer verifying the parser may flag inconsistencies between the test output format and the parser implementation.
- Recommendation:
  Add to `[**Data Structure**] journal.rs`:
  ```rust
  /// Parse `git log --oneline` output into commit rows. Each line splits at
  /// the first ASCII space: prefix is `short`, suffix (trimmed) is `message`.
  /// Empty lines are skipped. Returns up to 20 entries.
  pub(crate) fn parse_oneline(stdout: &str) -> Vec<JournalCommit>;
  ```
  Or note in C-13: "`parse_oneline` is a private helper splitting on the first ASCII space; documented in journal.rs."

### R-003 C-19 boundary predicate has gaps in fenced-code-block handling

- Severity: MEDIUM
- Section: `[**Constraints**] C-19`
- Problem:
  C-19 says: "Fenced code blocks: tracked by toggling on each line that starts with three backticks (optionally followed by a language tag). Lines inside a fenced block are ignored for boundary detection."

  Edge cases not addressed:
  1. **Asymmetric fences.** GitHub-flavored Markdown allows ` ```` ` (4 backticks) to *contain* ` ``` ` lines. If a user pastes 4-backtick fenced output into a session summary, the toggle logic misreads the inner 3-backticks as fence boundaries.
  2. **Tilde fences.** GFM also accepts `~~~` as a fence marker. C-19 only mentions backticks.
  3. **Indented code blocks.** A 4-space-indented `## Session 999: trick` line is, per CommonMark, a code block — not interpretable as a heading. C-19 doesn't address indented code.
  4. **Inline code with `## ...`.** A line containing `` `## Session 5: ...` `` (heading inside backticks) is fine because backticks need to be at line start to fence. Probably out of scope for boundary detection.

  V-UT-5 only tests "embedded fenced code blocks containing `## Session 999: trick`" — covers the basic case but not 4-backtick fences or tildes.
- Why it matters:
  A user who pastes their own session output into a journal summary may inadvertently break the parser. Silent misparse → wrong sessions table.
- Recommendation:
  Either tighten C-19:
  ```
  Fenced code blocks: tracked by matching opening fence regex
  `^(`{3,}|~{3,})(\w+)?\s*$`. The closing fence must use the same character
  type (backtick or tilde) AND be at least as long as the opening fence.
  Indented code blocks (4+ leading spaces or a tab) are NOT treated as
  fenced; the parser ignores indentation when matching `## Session`.
  ```
  …or accept the limitation and document in NG: "NG-13: parser does not handle 4-backtick fences, tilde fences, or indented code blocks. Authors who paste markdown-with-headings into summaries should verify the index re-render."

### R-004 C-20 step 3 canonicalization may fail on read-only filesystems

- Severity: LOW
- Section: `[**Constraints**] C-20` step 3
- Problem:
  C-20 step 3 says: "Resolve to absolute via `layout.root().join(stdout.trim())`, then `canonicalize`." `Path::canonicalize` requires the target path to exist AND requires read access to every component. On a read-only mount or with `.git/` ownership issues (UID mismatch, common in containerized worktrees), this can return `PermissionDenied` instead of a clean parent path.
- Why it matters:
  Auto-record fails noisily on Docker / shared-volume worktree setups. Edge case but worth documenting.
- Recommendation:
  Add to C-20: "If `canonicalize` fails (read-only FS, permission denied), return `Error::ParentRootResolution { reason: format!(\"canonicalize failed: {e}\") }` rather than panicking." Or use lighter-weight normalization: lexical strip of `..` components only, no symlink resolution. The git output for `--git-common-dir` is already a real path on disk by git's own resolution; `canonicalize` is defensive but not strictly required.

### R-005 `archive_path` lifetime in C-18 is implicit

- Severity: LOW
- Section: `[**Constraints**] C-18`, call graph for `task archive` auto-record
- Problem:
  C-18 says `record_task` runs after `.current` cleanup. `task_archive` (verified at `crates/ark-core/src/commands/agent/task/archive.rs:51-126`) computes `archive_path` early, then renames the task dir into it. After rename, `archive_path` is a valid disk location. The call graph correctly hands `archive_path` to `record_task`. ✅

  Implicit detail: when `record_task` runs, it may invoke `git log` from `archive_path` (per G-6 fallback chain step 2). The archive_path is now `parent/.ark/tasks/archive/YYYY-MM/<slug>/`. `git log` from there should work — it's inside the parent's working tree — but only if the parent is itself a git repo. In a non-git project, step 3 (parent root) is the fallback. The call graph and G-6 cover this.

  Nit: the call graph's `archive_path` reference is slightly ambiguous for readers — is it the variable from `task_archive`'s frame, or a re-derived value inside `record_task`? It's the former (passed via `RecordTaskOptions.archive_path`), but the SPEC reader has to infer.
- Why it matters:
  Pure clarity. Doesn't affect correctness.
- Recommendation:
  Add to C-18 or the call graph annotation: "`record_task` receives the same `archive_path` value `task_archive` computed (passed via `RecordTaskOptions`); the value is post-rename and points at the now-archived task dir on disk."

---

## Verification of Prior Findings

| ID | Severity | Plan Claim | Verified |
|----|----------|------------|----------|
| R-001 (toggle inconsistency) | HIGH | Accepted — kept toggle, harmonized PRD/G-2/NG-12 | ✅ G-2 lines 85, NG-12 line 248, PRD line 13 all consistent. PRD's "no opt-out flag" parenthetical removed; new wording explicitly mentions `auto_record_on_archive`. |
| R-002 (placement / tier gate) | HIGH | Accepted — tier-agnostic placement; C-18 added; V-IT-8 added | ✅ G-7 line 146 says "regardless of tier"; C-18 line 673 specifies "after `.current` cleanup, before `Ok(summary)` return"; V-IT-8 covers quick + standard. |
| R-003 (Acceptance Mapping mis-cites) | HIGH | Accepted — rewrote mapping; added V-UT-9 (`ark init` negative), V-UT-10 (CLI shape) | ✅ G-1 now maps to V-IT-1+V-IT-3+V-UT-10 (semantic — the integration tests do call workspace_init/record). G-2 maps to V-UT-9 (negative test) + V-F-1/F-4/F-6 (skip behavior). G-10 maps to V-UT-10. G-13/G-14 still map to V-IT-1 — V-IT-1 description now explicitly checks "managed-block markers and `{{name}}` substituted", which actually exercises G-13/G-14. ✅ |
| R-004 (line citation in `## Spec`) | HIGH | Accepted — citation removed; structural placement | ✅ G-7 has no line numbers; replaced with "after the `.current` cleanup block AND before the `Ok(summary)` return". Audited rest of `## Spec` — no other external file-line references. |
| R-005 (parser definition gap) | MEDIUM | Accepted — C-19 added; ParsedEntry struct in Data Structure | ✅ C-19 (line 674) specifies the predicate; `ParsedEntry` struct (line 510) has all fields: session_number, title, date, kind_label, slug, branch, commits_count. Boundary rule and code-block handling stated. (R-003 in this review tightens it further.) |
| R-006 (walk sites) | MEDIUM | Accepted — C-7 reworded; V-IT-9 for Stage B | ✅ C-7 line 662 explicitly names "Stage A `unload` capture loop AND Stage B `capture_orphan_hook_entries`". V-IT-9 (line 820) tests Stage B isolation. Verified at `commands/unload.rs:77` (Stage A) and `:162` (Stage B) — both are `walk_files_excluding(&owned, &skip)` sites. |
| R-007 (worktree-cleanup edge) | MEDIUM | Accepted — G-6 fallback chain; V-E-7 | ✅ G-6 lines 133-137 describe three-step fallback. V-E-7 (line 837) tests the fallback when `worktree_path` is gone. |
| R-008 (call graph clarity) | LOW | Accepted — annotation added | ✅ Call graph for `workspace_init` (lines 318-320) has the annotation: "seeded with markers + empty body; first record populates managed-block bodies via update_managed_block." |
| R-009 (Layout Clone) | LOW | Accepted — Phase 1 note + Data Structure shows `#[derive(Clone)]` | ✅ Phase 1 (line 749) says "Verify `Layout: Clone` is derived... add if missing". Data Structure (line 573) shows `#[derive(Debug, Clone)]`. NOTE: Verified independently — `Layout` already derives `Clone` at `layout.rs:104`. The plan note is harmless but redundant. |
| R-010 (numeric-only names) | LOW | Accepted — regex now requires leading letter | ✅ C-3 line 658 regex `^[A-Za-z][A-Za-z0-9_-]{0,39}$`. V-UT-1 (line 800) explicitly rejects `1leading`, `0`. |
| TR-1..TR-6 | — | Applied per reviewer | ✅ All trade-off resolutions applied; T-3 expanded into C-20 four-branch detection; T-5 expanded into C-21 hard ceiling. |

---

## Trade-off Advice

(All prior trade-offs T-1..T-6 were resolved in iter 00; no new trade-offs introduced in iter 01. The Trade-offs section in `01_PLAN.md` correctly summarizes them as resolved.)

### TR-1 (none — no new trade-offs)

- Related Plan Item: N/A
- Topic: N/A
- Reviewer Position: N/A
- Advice:
  No outstanding trade-offs to advise on. The `## Trade-offs` section in `01_PLAN.md` is a traceability log, not an open question set.
- Rationale:
  Iter 00's trade-offs were converted into constraints (C-20, C-21) or acknowledged design choices. No new ambiguity introduced by the revisions.
- Required Action:
  None.
