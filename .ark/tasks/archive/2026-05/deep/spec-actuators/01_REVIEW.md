# `spec-actuators` REVIEW `01`

> Status: Closed
> Feature: `spec-actuators`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `4` (1 HIGH, 2 MEDIUM, 1 LOW)

## Summary

The redraft lands the language-agnostic, hybrid-by-kind pivot cleanly and the prior CRITICAL is fully resolved: `DeadScan`/fatal-on-zero is gone end-to-end and zero matches is consistently PASS across Data Structure, Constraints (C-5), Runtime, Failure Flow, and Validation. R-003 is honored — no compile-time embedding remains; the runtime-glob approach matches the `commands_no_bare_command_new` precedent (`CARGO_MANIFEST_DIR` + comment-line skip at `commands/context/mod.rs:508`). The schema is genuinely Rust-free, with Rust appearing only as Ark's own dogfooding instance. The one HIGH is a soundness gap: the `tool` "absent command ⇒ FAIL" rule lives only in Failure-Flow prose, not in the C-8 contract that is promoted verbatim — an undiscovered tool could be read as a silent pass at the SPEC level. The remainder are MEDIUM/LOW polish. No CRITICAL, so this advances with one bounded revision.

---

## Findings

### R-001 `tool absent-command soundness gap is not in the promoted contract`

- **Severity:** HIGH
- **Section:** `## Spec` C-8; vs `## Runtime` Failure Flow item 3
- **Problem:** C-8 states only that `tool` "names a command key resolved to a concrete command by the verifier's existing auto-discovery; the schema names no toolchain." It does **not** state what happens when discovery resolves *nothing*. The only place "absent ⇒ FAIL" appears is Failure-Flow prose ("A discovered `tool`/`test-binding` command absent or non-zero → verifier records a FAIL Finding"). The `## Spec` is the verbatim-promoted contract; Runtime/Failure-Flow are not promoted. As written, the durable SPEC permits the exact failure this whole task exists to kill: a `tool` rule that *looks* enforced but, because discovery found no command, is silently never run and reads as PASS. This is the C-8 hazard called out in the brief, and it is the actuator analogue of the original `DeadScan` motivation — an enforcer that claims coverage it does not deliver.
- **Why it matters:** Auto-discovery is heuristic (T-3 concedes "discovery's heuristic guesswork"). A misspelled `command_key`, a renamed CI step, or a project without the expected manifest yields no command. If undiscovered silently passes, `tool` becomes the new weakest-actuator default — undermining the PRD thesis that "enforcement status is never unknown." Worse, it is invisible: the health histogram counts the rule as a `tool` (enforced) while nothing runs.
- **Recommendation:** Promote the rule into a Constraint, e.g. "C-8a: a `tool` whose `command_key` resolves to no discoverable command is a FAIL (or a fatal health finding), never a silent pass." Add a matching Validation row (a fixture `tool` rule with an unresolvable key asserts FAIL). Decide and state whether the failure surfaces as a `HealthFinding` (engine-side, like `UnparseableActuator`) or only at VERIFY (verifier-side) — the two have different blast radii and the SPEC should pin one.

### R-002 `C-6 unknown-extension whole-line scan invites false positives`

- **Severity:** MEDIUM
- **Section:** `## Spec` C-6; `## Validation` V-UT-6, V-UT-7
- **Problem:** The extension→marker table is a sound, genuinely portable approach for the common languages (V-UT-6 covers `.rs`→`//`, `.py`→`#`), and "comments-only" scope is the correct mechanism for self-non-flagging (R-005, V-UT-7). The weak edge is the fallback: "unknown extension scans the whole line." For any file type the table does not know, the scanner abandons comment-awareness and matches code, string literals, and data alike — which is precisely the false-positive class C-6/V-UT-7 are designed to prevent. A `source-scan` over a mixed-language tree could flag an id-shaped token inside a string literal in an unrecognized file type and report a spurious violation, exactly when a user cannot easily see why.
- **Why it matters:** False positives in a build-failing check erode trust and push users to weaken or remove the rule — the drift this task fights. The risk is bounded (only unknown extensions, only id-shaped patterns) so it is acceptable for a first cut, but the contract should make the trade-off explicit rather than leave whole-line scanning as an unflagged default.
- **Recommendation:** State the fallback policy deliberately. Either (a) keep whole-line scan but document it as a known false-positive surface and add a Validation row asserting the behavior on an unknown extension, or (b) make unknown extensions *skip* (scan nothing, count as not-applicable) so absence of a marker never produces a false FAIL. Option (b) is safer for a language-agnostic tool and is one sentence in C-6.

### R-003 `Acceptance Mapping omits several Constraints`

- **Severity:** MEDIUM
- **Section:** `## Validation` Acceptance Mapping
- **Problem:** Rubric item 6 requires every Goal and Constraint to carry ≥1 Validation. The mapping table lists G-1..G-5 and C-1, C-5, C-6, C-9, C-12, C-13, C-14 — but omits C-2, C-3, C-4, C-7, C-8, C-10, C-11. Some are covered in the test bodies (C-2/C-4 by V-UT-1/V-UT-3; C-3 by V-UT-2/V-IT-1; C-7 by V-IT-2/V-E-1; C-10 by V-UT-8; C-11 by V-IT-1) but the table does not show it, and C-8 has no validation at all (see R-001).
- **Why it matters:** The Acceptance Mapping is the gate the verifier reads. Gaps in the table read as gaps in coverage even when a test exists, and a genuinely unvalidated Constraint (C-8) hides in the omission.
- **Recommendation:** Add the missing rows so every C-N maps to ≥1 V-*. Closing R-001 supplies the C-8 row.

### R-004 `parse_tag arg-bound spec is prose-only for the backtick edge`

- **Severity:** LOW
- **Section:** `## Spec` C-1; `## Log` R-002 resolution
- **Problem:** The Response Matrix pins the grammar well ("final element of the first line, fenced as `⟨@kind: arg⟩`, arg bounded by closing `⟩`") and V-UT-5 tests both a `.`-terminated line and a backtick-terminated line. C-1 itself says "before terminal punctuation" but does not restate the closing-`⟩` bound that disambiguates an arg containing backticked path tokens — the precise hazard 00-review R-002 raised. The detail lives in the Log (not promoted) and the test, not in the promoted Constraint.
- **Why it matters:** Minor — the test pins behavior — but the promoted SPEC should be self-contained enough that a future reader parses the grammar from C-1 alone without reconstructing it from the Log.
- **Recommendation:** Fold the "arg is bounded by the closing `⟩`" clause into C-1 so the delimiter contract is in the verbatim text.

---

## Response Matrix Audit (prior CRITICAL/HIGH/MEDIUM/LOW)

- **R-001 (CRITICAL) — RESOLVED.** `DeadScan` is removed from `HealthFinding` (only `Untagged` + `UnparseableActuator` remain). C-5 = "zero lines is a PASS." Runtime step 3, Failure Flow item 2, V-UT-4, V-IT-2, V-E-1 all agree. No residual contradiction anywhere.
- **R-002 (HIGH) — RESOLVED.** Grammar pinned to a fenced trailing token with an explicit closing-delimiter bound; V-UT-5 exercises the multi-paragraph + backtick edge. (Minor self-containment nit logged as R-004 above.)
- **R-003 (HIGH) — RESOLVED.** No `include_dir!`/`include_str!` of `.ark/specs` or `crates/` survives; C-7 mandates runtime glob reads, consistent with the `CARGO_MANIFEST_DIR` precedent. `embedded.rs` deleted from the Architecture.
- **R-004 (HIGH) — RESOLVED.** C-14 is genuinely additive: the verifier "applies every project-SPEC rule" and only *defers* mechanical kinds to their deterministic result. No `subagent-support` Constraint is contradicted (C-10/C-11 there are untouched; the verifier still applies every rule per its shipped mandate). C-22 tri-platform byte-identity is acknowledged in the Architecture and Phase 6.
- **R-005 (MEDIUM) — RESOLVED.** Self-non-flagging is now a consequence of comments-only scope (C-6), not a magic rule; V-UT-7 covers it.
- **R-006 (MEDIUM) — RESOLVED.** `test-binding` names a concrete project test id, never a `V-*` label, in **both** C-9 and C-12 ("downgraded to `judgment` when no test maps"). V-UT-9 asserts rejection of a `V-*` arg.
- **R-007 (MEDIUM) — RESOLVED.** V-IT-4 (broken enforcer ⇒ check fails) covers G-3; V-E-2 covers C-13.
- **R-008 (LOW) — RESOLVED.** Migration counts pinned per file; `[**Rules**]` tagged, `[**Exceptions**]` excluded as carve-outs.
- **TR-1 — addressed (Deferred, user chose one task; per-phase commit boundary stated).**
- **TR-2 — addressed (Untagged stays reported-not-fatal; V-IT-1 asserts post-migration zero).**

All prior CRITICAL/HIGH findings are accounted for and their fixes are present in the redrafted `## Spec`. No Response-Matrix entry claims a fix absent from the Spec.

---

## Trade-off Advice

### TR-1 `tool soundness — FAIL-on-undiscovered vs silent-pass`

- **Related Plan Item:** C-8 / T-3 (auto-discovery vs config section)
- **Topic:** Flexibility vs Safety
- **Reviewer Position:** Prefer B (fail-closed)
- **Advice:** Reusing the verifier's discovery instead of a new config section is the right call (T-3 is endorsed — it dodges the upgrade-merge burden flagged in project memory). But discovery's heuristic nature must be paired with fail-closed semantics: an undiscovered `tool` command is a FAIL, never a pass. This keeps the "enforcement is never unknown" invariant intact while preserving the no-config win.
- **Rationale:** A heuristic resolver that fails open converts every resolution miss into invisible non-enforcement — the precise drift the PRD targets. Fail-closed makes a discovery miss loud and fixable.
- **Required Action:** Adopt — see R-001; pin the fail-closed rule in a Constraint with a Validation row.

### TR-2 `unknown-extension scan — whole-line vs skip`

- **Related Plan Item:** C-6
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer skip (not-applicable) over whole-line
- **Advice:** For a language-agnostic tool, defaulting an unknown extension to a comment-unaware whole-line scan trades safety for coverage on exactly the files the engine understands least. Skipping unknown extensions (or treating them as not-applicable) avoids spurious build failures; users can extend the marker table when they want coverage.
- **Rationale:** A false FAIL on an unrecognized file type is worse than a missed scan there — the former pushes users to delete the rule.
- **Required Action:** Justify rejection or adopt; either way state the fallback policy explicitly in C-6 (R-002).

---

## Acceptance

- [x] R-001 (prior CRITICAL) resolved — DeadScan inversion gone; zero matches = PASS everywhere.
- [x] R-002/R-003/R-004 (prior HIGH) resolved in the redrafted Spec with Log entries.
- [x] Every Goal maps to ≥1 validation; Constraint coverage gaps flagged (R-003 this iteration).
- [ ] R-001 (this iteration, HIGH): pin `tool` fail-closed semantics in a Constraint + Validation.
- [ ] R-002/R-003/R-004 (this iteration, MEDIUM/LOW): C-6 fallback policy, Acceptance Mapping completeness, C-1 delimiter clause.
