# `ark-agent-namespace` REVIEW `01`

> Status: Closed
> Feature: `ark-agent-namespace`
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

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 3



## Summary

Verified the Response Matrix against the plan body: all 11 prior findings (R-001 CRITICAL, R-002 HIGH, R-003..R-011) are RESOLVED, and the seven trade-off verdicts (TR-1..TR-7, including the previously open T-6) are applied. Spot-checks confirm the claims: `PathExt::rename_to` is absent today and Phase 1 step 4 adds it with V-UT-17; C-13 spells out both the start predicate (`line.len() == 7 || line.as_bytes()[7] == b' '`) and end predicate (`starts_with("## ") || line == "##"`); `toml` is absent from `Cargo.lock` and Phase 1 steps 1-2 pin it at workspace level; G-8 lists all four template paths plus the repo copies, and `.claude/commands/ark/{quick,design}.md` are confirmed to contain the raw `mkdir`/`cp`/`echo` recipes that V-IT-4 now guards against; `status` field is dropped and derived via `TaskToml::status()`; `Error::NoCurrentTask` and `Error::WrongTier` are present. Three new non-blocking issues (R-012..R-014) concern under-specification around V-F-5's managed-block edge case, C-14's character set, and the `spec extract` append-vs-rewrite contract; none block execution. Execute may start.



## Findings

### R-012 `V-F-5 and C-7 under-specify update_managed_block's behavior when END marker is missing`

- Severity: LOW
- Section: `[**Runtime**] spec register step 3`, `[**Validation**] V-F-5`, `[**Constraints**] C-7`
- Problem:
  V-F-5 says "test documents and asserts `update_managed_block`'s existing behavior in this case." Reading `crates/ark-core/src/io/fs.rs` reveals: `Marker::locate` returns `None` when `END` is absent (line 199: `text[start..].find(&self.end())?`), so `replace_in` returns `None`, so `update_managed_block` falls through to `append_block`, which appends a NEW complete `<!-- ARK:START -->...<!-- ARK:END -->` block to the file. Net result: the file now contains TWO `ARK:FEATURES:START` markers and one `:END`, which subsequent `read_managed_block` calls will lock onto the FIRST `START` and find the newly-appended `END`, yielding a garbled body that includes the orphaned original content. This is latent corruption, not "defined behavior" worth locking in with a test.
- Why it matters:
  The plan promises to "document what the existing code does" but the existing code does something wrong in this case. If V-F-5 simply asserts current behavior, it locks in a bug.
- Recommendation:
  Pin the expected behavior in C-7 or a new C-15: on malformed managed block (START without matching END), `spec register` should error with a dedicated variant (e.g., `Error::ManagedBlockCorrupt { path, marker }`) rather than silently append a second block. Either extend `io::fs::update_managed_block` to detect this case and return an error, OR have `spec_register` call `read_managed_block` first and validate before calling `update_managed_block`. V-F-5 then asserts the error, not the corruption.

### R-013 `C-14 character set for --feature / --scope is minimal; silent acceptance of whitespace, angle brackets, backticks`

- Severity: LOW
- Section: `[**Constraints**] C-14`, `[**Validation**] V-UT-18`
- Problem:
  C-14 rejects only `|` and newline. But `--feature` / `--scope` values are rendered into a markdown table cell and then written to `specs/features/INDEX.md`, which is also parsed by future `read_managed_block` calls. Leading/trailing whitespace leaks through (cosmetic but also breaks upsert-by-feature-name matching if one row has trailing space and another doesn't), empty string produces malformed rows, and backticks inside the cell will render funny. Angle brackets (`<`, `>`) are mostly fine in markdown but may collide with HTML-comment-like syntax.
- Why it matters:
  Low-impact, but the PRD's "single correctness contract" framing makes minimum-viable validation feel inconsistent with the rest of the plan's strictness.
- Recommendation:
  Either (a) broaden C-14 to reject the full set `{|, \n, \r, leading/trailing whitespace, empty string}` and specify a canonical form (trim then validate non-empty), OR (b) explicitly narrow the allowed set to `[A-Za-z0-9_-]` and a documented subset of spaces, and note in C-14 why the narrow set is appropriate for an identifier-like field. Either way, document the decision explicitly rather than leaving it minimum-viable.

### R-014 `Runtime step 5 of spec extract is ambiguous: full rewrite + CHANGELOG, or pure CHANGELOG append`

- Severity: LOW
- Section: `[**Runtime**] spec extract step 5`, `[**Log**] Changed`
- Problem:
  Runtime step 5 reads: "If `specs/features/<slug>/SPEC.md` exists: append a `[**CHANGELOG**]` block with today's UTC date + extracted body. Else write fresh from the `SPEC.md` template with the body spliced in." The first branch is ambiguous:
  - Interpretation A: The existing SPEC body stays, and a CHANGELOG entry is appended at the end noting "on <date>, <new body>". The old body remains the "canonical" SPEC and divergence accumulates in CHANGELOG.
  - Interpretation B: The existing SPEC body is replaced with the new extracted body, and a CHANGELOG entry is appended noting "on <date>, these are the prior body's differences."
  - Interpretation C: Neither — only the CHANGELOG is appended, the extracted body is discarded on overwrite.
  V-UT-11 says "existing SPEC → CHANGELOG appended" which is consistent with all three.
- Why it matters:
  This is the deep-tier promotion path. Getting which interpretation is authoritative wrong means subsequent iterations of the same feature will silently either (A) grow stale, (B) lose historical context, or (C) overwrite without keeping the new spec. All three are defensible; the plan must pick one.
- Recommendation:
  Replace Runtime step 5 with an explicit 3-step sequence. Recommended (interpretation B — new body wins, old body summarized in CHANGELOG):
  1. If SPEC exists, read its current body.
  2. Write new SPEC = extracted body from PLAN, with an appended `[**CHANGELOG**]` section that lists `- YYYY-MM-DD: replaced from <iteration>_PLAN.md (prior body preserved in git history)`.
  3. If SPEC did not exist, write from template with body spliced in, no CHANGELOG.
  Add a V-UT assertion that after two `spec extract` calls on the same slug, the SPEC body equals the second extraction's body (not the first).



## Trade-off Advice

_No new trade-offs introduced in 01. Prior TR-1..TR-7 are closed._
