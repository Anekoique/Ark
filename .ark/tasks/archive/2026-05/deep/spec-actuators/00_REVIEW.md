# `spec-actuators` REVIEW `00`

> Verdict: Rejected
> Reviewer: ark-reviewer (agent); reconstructed to disk by main session from the agent's returned findings (the agent's own write produced an empty file)
> Plan under review: `00_PLAN.md`

---

## Summary

The PLAN is well-framed and the two prior leak removals (C-23 pattern hardcoding, comment-hygiene restatement) are confirmed gone — no Constraint leaks instance detail or restates a project-convention rule. However the central enforcement mechanism contains a logic inversion (`DeadScan`-fatal), and three load-bearing aspects (the inline-token grammar, compile-time embedding of `.ark/specs`, and the verifier behavior change vs the shipped subagent-support SPEC) are under-specified. These need a redraft, not a one-line touch-up. Rejected.

## Findings

### `R-001` — `DeadScan`-fatal is a logic inversion `CRITICAL`

- **Where:** `00_PLAN.md` `## Spec` C-5; contradicts `## Validation` V-IT-2.
- **Problem:** On a clean codebase a correctly-written `source-scan` guard matches **zero** lines — zero matches is the *passing* state (the `commands_no_bare_command_new` precedent at `commands/context/mod.rs:514-650` asserts `!live.contains("Command::new")`, i.e. success = no match, with no liveness assertion). C-5 makes zero matches *fatal*, so it would fail the build for every correctly-passing source-scan rule. V-IT-2 ("`crates/**/*.rs` is clean under the C-23 scan") directly asserts the state C-5 calls fatal. The PLAN deferred this to REVIEW; it is unresolved and blocks.
- **Direction:** "Dead guard" must be detected by a signal independent of the live tree. Options: (a) require the `source-scan` to match its own positive fixture (the rule carries an example that *must* match), and treat zero matches against the live tree as PASS; or (b) drop dead-detection from the engine and cover it with a separate "every pattern has a positive fixture test" requirement. Resolve before EXECUTE.

### `R-002` — inline trailing-token grammar under-specified for multi-paragraph bodies `HIGH`

- **Where:** `00_PLAN.md` `## Spec` C-1; vs `LAYOUT.md` L-3/L-4.
- **Problem:** C-1 says the actuator tag is the "trailing inline token" of a rule bullet, but COMMENTS/STYLE/ERRORS rule bodies are multi-sentence, multi-paragraph, and end in `.` or a backticked token. "Trailing token" is not well-defined for a multi-paragraph body, and a `⟨@kind: arg⟩` suffix is ambiguous against bodies that already end in backticked path tokens. Conflicts with L-3's bullet shape.
- **Direction:** Pin the grammar exactly — e.g. the actuator token occupies its own trailing line of the bullet, or a dedicated delimiter that cannot occur in rule prose. Amend L-3 explicitly and show the parse rule. Add a parser test against a real multi-paragraph rule (C-1 in COMMENTS.md is the hardest case).

### `R-003` — C-9 compile-time embedding of `.ark/specs` has no precedent and conflicts with the one it cites `HIGH`

- **Where:** `00_PLAN.md` `## Spec` C-9, `## Architecture` `embedded.rs`.
- **Problem:** `templates.rs` embeds only `templates/` trees (`include_dir!("$CARGO_MANIFEST_DIR/../../templates/...")`). The cited `commands_no_bare_command_new` precedent `include_str!`s sibling `crates/` source, not `.ark/specs/`, which lives *outside* the crate. Embedding `.ark/specs/project/**` from `ark-core` crosses the crate boundary with a fragile relative path and no existing pattern.
- **Direction:** Decide the source of truth deliberately: either (a) the engine reads the *shipped template copies* under `templates/ark/specs/` (consistent with `templates.rs`), accepting that Ark's live `.ark/specs/` is the dogfooding copy; or (b) the convention SPECs are relocated/symlinked so a stable relative path exists; or (c) the test reads them at runtime via `CARGO_MANIFEST_DIR` rather than `include_*`. Note: project SPECs are currently NOT shipped under `templates/ark/specs/project/` — only INDEX.md is — so option (a) implies also shipping them, a scope decision.

### `R-004` — verifier behavior change not reconciled with shipped subagent-support SPEC `HIGH`

- **Where:** `00_PLAN.md` `## Runtime` + Implementation Phase 6; vs `subagent-support/SPEC.md`.
- **Problem:** The PLAN changes `ark-verifier` to consume engine results for mechanical rules and judge only `judgment` rules, but the shipped verifier template mandates it "read every project SPEC rule and apply it." The `## Log` is empty (correct for 00), so there is no record superseding the shipped Constraint. Also subagent-support C-22 requires agent prompt bodies be byte-identical across claude/codex/opencode after frontmatter strip — editing one verifier template requires editing all three identically.
- **Direction:** Either narrow the change so it does not contradict the shipped Constraint (the verifier still applies every rule, but *defers mechanical rules to `cargo test`* rather than re-judging them), or record the supersession explicitly in a later iteration's `## Log`. Call out the C-22 tri-platform byte-identity requirement as an Implementation constraint.

### `R-005` — C-13 self-non-flagging is undesigned / potentially circular `MEDIUM`

- **Where:** `00_PLAN.md` `## Spec` C-13.
- **Problem:** "A source-scan never flags its own pattern literals" is asserted but no mechanism is given. The scanner scans `crates/**/*.rs`, which includes `scan.rs` itself; pattern strings live there. Without a defined exclusion (scan comments only, never string literals; or exclude `scan.rs`) this is circular.
- **Direction:** Specify the exclusion precisely — the `comments-only` scope already excludes string literals, so state that C-13 is *satisfied by* `ScanScope::CommentsOnly` plus "pattern args are never written as comments." Make it a consequence, not a separate magic rule.

### `R-006` — C-11 default persists `V-*` IDs into the SPEC, against the task's own thesis `MEDIUM`

- **Where:** `00_PLAN.md` `## Spec` C-11 (extract default `test-binding` names the mapped `V-*` test).
- **Problem:** The whole task exists to stop workflow IDs leaking into durable artifacts. A `test-binding` whose arg is `V-UT-7` writes a `V-*` id into the promoted SPEC — re-importing the leak at a different layer.
- **Direction:** `test-binding` should name the **test function identifier** (e.g. `actuator_parse_roundtrips`), not the `V-*` bookkeeping id. The `V-*` ↔ test-fn mapping stays in the PLAN's Acceptance Mapping only.

### `R-007` — G-3 / G-5 thin validation; C-12 unvalidated `MEDIUM`

- **Where:** `00_PLAN.md` Acceptance Mapping.
- **Problem:** G-5 (audit skill) maps only to "V-F-1 + manual REVIEW," and C-12 (audit skill read-only-by-default) has no validation. G-3 relies on negative unit tests only.
- **Direction:** Add at least a smoke validation for the skill's read-only default, or explicitly mark the skill body as REVIEW-gated (no code path) and accept the gap knowingly.

### `R-008` — migration rule counts and Exceptions tagging unquantified `LOW`

- **Where:** `00_PLAN.md` Implementation Phase 3.
- **Problem:** Phase 3 says "tag every rule" but does not state whether `[**Exceptions**]` entries (EX-N / NX-N) and `LAYOUT.md` L-rules also get actuators, nor the expected per-bucket counts to verify completeness.
- **Direction:** State the denominator (N rules across the 3 files + whether L-rules and Exceptions are in scope) so V-IT-1 can assert "every rule tagged" against a known count.

---

## Trade-off Advice

- `TR-1`: (advisory — user explicitly chose one task) The 6-phase scope is large. If EXECUTE stalls, a natural seam is: foundation (engine + LAYOUT grammar + migrate 3 SPECs) vs propagation (extract path + workflow templates + audit skill + verifier). Not a blocking finding.
- `TR-2`: The "Untagged = reported, not fatal" choice is endorsed — it keeps the build green during the migration while making the gap visible. Promote to fatal in a follow-up once the count reaches zero.

---

## Acceptance

- [ ] R-001 (CRITICAL) resolved — DeadScan inversion fixed.
- [ ] R-002, R-003, R-004 (HIGH) resolved or explicitly deferred with reasoning in `01_PLAN.md` `## Log`.
- [ ] Every Goal still maps to ≥1 validation after redraft.
- [ ] MEDIUM/LOW (R-005..R-008) addressed or consciously accepted.
