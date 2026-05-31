# `tag-feature-specs` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `tag-feature-specs`
> Target Task: `tag-feature-specs`
> Tier: `standard`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Standard: `/ark:commit` warns on any `PENDING` and proceeds.

---

## Severity Summary: 0 CRITICAL · 0 HIGH · 1 MEDIUM · 1 LOW
## Verification: build PASS · tests PASS (612 passed / 0 failed) · lint PASS (clippy `-D warnings` clean) · format PASS (`cargo fmt --check` clean — note: rustfmt does not lint Markdown)

---

## Project Spec Compliance

> Governing rules for this doc-only task are `LAYOUT.md` L-3 (two-line rule shape) and L-9 (actuator-tag grammar). The convention SPECs are Markdown documents — `rust/STYLE.md` / `rust/ERRORS.md` code-style rules are N/A to this change (no `crates/` edits). Compliance here is whether the reformatted `[**Rules**]` bullets conform to the new L-3/L-9 two-line shape.

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — the task did not add or remove any `specs/project/` file; the four governed docs (`LAYOUT.md`, `rust/{COMMENTS,STYLE,ERRORS}.md`) are unchanged in set, only reformatted. `templates/ark/specs/project/INDEX.md` got a one-line optional-tags note (per spec-actuators Architecture), not a structural change.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — line-1 grammar scan over `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`, and `LAYOUT.md` returned `OK` (zero malformed) against `^- <PREFIX>-<N>: @(test-binding: <fn>|source-scan: <p> @ <g>|tool: <cmd>|judgment)$`. Bold rule title (`**Title.** Body`) preserved on line 2 per L-3 (confirmed in `COMMENTS.md` C-1..C-17). `LAYOUT.md` L-3/L-9 rewritten to define the two-line form and its own worked example conforms.

## Related Feature Spec Compliance

> PRD `[**Related Specs**]` names `spec-actuators` (grammar source, same branch) and all 14 feature SPECs as the edit targets. `spec-actuators` is itself edited this task; see Plan Fidelity G-1 and V-001.

- [x] `spec-actuators` grammar respected: PASS — every reformatted constraint across the 13 feature SPECs + `spec-actuators` + 3 project SPECs is the two-line form `- <ID>: @<kind>[: <arg>]` / prose, `<kind>` ∈ {tool, source-scan, test-binding, judgment}, no blank line between constraints. `spec-actuators` C-12 (`agent_bodies_are_byte_identical_modulo_platform_idioms`) passes in the suite.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]`. Judged against delivered intent (the two-line reshape is a refinement of the same tagging deliverable).

- [x] spec-audit baseline captured: N/A for re-verification — a process artifact of EXECUTE, not gate-checkable at HEAD; the post-tag end-state (0 malformed / 0 mismatch) is the checkable invariant and is confirmed below.
- [x] Every concretely-enforceable constraint carries a well-formed tag: PASS — 179 `@test-binding` occurrences plus `@tool` / `@source-scan` across the feature SPECs; every line-1 well-formed (`ALL_FEATURE_TAGS_WELLFORMED`).
- [x] Each `test-binding` arg names a test fn that currently exists: PASS — 150 distinct `@test-binding` args (feature + project SPECs) all resolve via `grep -rE 'fn <name>\b' crates/`; **0 unresolved**.
- [x] Untagged constraints left as judgment, not fabricated: PASS — `proposals.tsv` maps every untagged constraint to `judgment`; reformatted SPECs carry `@judgment` for them.
- [x] No `V-*` / `C-N` / `G-N` in any tag arg: PASS — grep over feature-SPEC tag lines for `V-[A-Z]+-[0-9]|G-[0-9]` returns empty (`NO_LEAK`). The legitimate `@source-scan: V-(UT|IT|E|F)-\d @ ...` in project `COMMENTS.md` and `spec-actuators` C-5 is a forbidden-pattern *definition*, not a leak — correctly excluded.
- [x] `cargo test --workspace` stays green: PASS — 612 passed / 0 failed.
- [x] Post-tag audit shows 0 malformed / 0 mismatch: PASS — line-1 grammar scan over all feature SPECs reports zero malformed; all bindings resolve (zero mismatch).

## Plan Fidelity

> Goals from `PLAN.md ## Spec`. The PLAN predates the two-line reshape and scope growth (project SPECs + docs + templates); judged against delivered intent per the verifier brief.

- [x] G-1: Every concretely-enforceable feature-SPEC constraint carries a well-formed actuator tag: PASS — all 13 feature SPECs reformatted to the two-line form; every line-1 well-formed; tags carried from `proposals.tsv`.
- [x] G-2: Each `test-binding` / `tool` / `source-scan` arg resolves to a real enforcer: PASS — 0 unresolved `test-binding` (150 distinct); `@tool` args are `clippy` / `cargo build` / `cargo check --all-targets` (runnable); `@source-scan` args are `<pattern> @ <glob>` form.
- [x] G-3: Constraints with no concrete enforcer stay untagged-by-default (`@judgment`): PASS — judgment constraints carry `@judgment`, none fabricated.
- [x] G-4: `/ark:spec-audit` dogfooded (baseline before, clean after): PASS for the checkable end-state (clean post state). The literal baseline report is an EXECUTE artifact; see PRD note above. NG-1/NG-2 honored: the diff touches only SPECs, docs, templates, and `.installed.json` — no `crates/` change.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: PARTIAL — see V-001. The repo *does* carry a CHANGELOG convention: 11 of 13 feature SPECs ship a `[**CHANGELOG**]` section (only `ark-research`, `ark-sandbox`, `spec-actuators` lack one). The 13 feature SPECs touched this task are a pure format reshape — prose byte-identical, constraint counts unchanged (verified per-SPEC: HEAD == WORK for all 13) — so no new CHANGELOG entry is owed for them. The one genuine semantic change is `spec-actuators` (constraint rewrite + a dropped constraint, 14 → 13). It carries no CHANGELOG entry recording the change; it also carried none at HEAD. Recorded as V-001. Not a hard FAIL on standard tier: the dropped rule's intent is plausibly preserved elsewhere (see V-001) and `spec-actuators` has no enforced changelog test (unlike codex/opencode SPECs, which `*_spec_changelog_present` tests guard — both passing).

## Findings

> Cross-cutting observations. Each Finding has a Resolution; `/ark:commit` (standard) warns on PENDING and proceeds.

### V-001 `spec-actuators SPEC dropped a constraint (14 → 13) with no CHANGELOG — confirm intentional`

- **Severity:** MEDIUM
- **Location:** `.ark/specs/features/spec-actuators/SPEC.md` `[**Constraints**]` (C-1..C-13 at WORK vs C-1..C-14 at HEAD)
- **Problem:** Constraint-count parity holds exactly for all 13 *other* feature SPECs and the 3 project SPECs (pure format reshape, prose byte-identical). `spec-actuators` is the lone exception: it went from 14 constraints at HEAD to 13 at WORK. HEAD's `C-14` — "No workflow ID (`V-*-N`, `C-N`, `G-N`, `R-NNN`) appears in any `crates/` comment; the constraint goes inline as prose, the label nowhere." — is absent at WORK. This SPEC was deliberately rewritten this task to describe the two-line grammar (G-1 / NG-4 prose updated; the old C-1 inline-glyph rule replaced by the two-line rule), so the removal is plausibly intentional: the workflow-ID-in-source invariant is now carried by `project-spec` C-6 (`@source-scan: V-(UT|IT|E|F)-\d @ crates/**/*.rs`) and the rewritten `spec-actuators` C-5 (same source-scan pattern). But a constraint removal is a *semantic* change, not the format-only reshape the rest of the task was, and no `[**CHANGELOG**]` entry records it (the SPEC has no CHANGELOG section at all, though 11 of 13 sibling feature SPECs do).
- **Why it matters:** A silently dropped constraint is exactly the SPEC-drift class VERIFY exists to catch. The removal is benign only if C-14's intent is genuinely preserved elsewhere; if not, an enforced invariant has quietly disappeared with no record.
- **Recommendation:** Confirm the C-14 removal is intentional and that its intent (no workflow ID in `crates/` source) is in fact covered by `project-spec` C-6 / `spec-actuators` C-5. If covered, document the rewrite + removal — either add a `[**CHANGELOG**]` entry to `spec-actuators/SPEC.md` (matching the 11 sibling SPECs' convention) or note it in the commit message. If not covered, restore the rule reformatted to the two-line shape.
- **Resolution:** FIXED — the drop was accidental, not intentional. C-14 restored in two-line form (`@source-scan: (V-(UT|IT|E|F)|G|R)-\d @ crates/**/*.rs`); count parity with HEAD restored (14 = 14). A `[**CHANGELOG**]` section was also added to `spec-actuators/SPEC.md` recording the inline→two-line reformat and noting C-1..C-14 are preserved.

### V-002 `.ark/.installed.json modified — confirm it belongs in this commit`

- **Severity:** LOW
- **Location:** `.ark/.installed.json` (36 lines changed in `git diff --stat`)
- **Problem:** The diff includes `.ark/.installed.json` alongside the SPEC/template/doc edits. The PLAN scoped the task to SPEC files; the manifest churn is most likely an incidental side effect of an `ark` invocation in the worktree (template-hash refresh tracking the `templates/` edits), not part of the stated deliverable.
- **Why it matters:** Committing manifest drift unrelated to the feature muddies history and can mask a real install-state change.
- **Recommendation:** Confirm the `.installed.json` delta is the expected hash refresh for the touched `templates/` files and is safe to include; otherwise restore it before commit.
- **Resolution:** ACCEPTED — the `.installed.json` delta is the expected manifest hash-refresh tracking the edited `templates/` files (SPEC/PLAN/VERIFY/INDEX/workflow + spec-audit/verifier across platforms), plus the carry-over `extract-spec`→`spec-extract` rename from the prior spec-actuators commit on this branch. It is a correct install-state record, not unrelated drift; safe to include. The actual staging set is chosen at commit time (`ark agent task commit` stages only Ark-managed artifacts), so this is informational.

## Notes

- Live verification commands all green at HEAD of `feat/spec-actuators`: `cargo test --workspace` 612/0, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean (build is implied by clippy + test compiling). All four ran with captured exit code 0.
- Format reshape confirmed on representative SPECs (`ark-research` full diff, `spec-actuators` full diff, `COMMENTS.md` head): every constraint is `- <ID>: @<kind>[: <arg>]` on line 1, original prose byte-identical on line 2, no blank line between constraints. In `ark-research`, C-16 is correctly `@tool: clippy`, judgment constraints `@judgment`, and the old single-line `- C-N: <prose>` form is replaced with prose preserved verbatim.
- Constraint-count parity (HEAD vs WORK) was checked per SPEC: all 13 reshape-only feature SPECs match exactly (e.g. ark-context 46/46, subagent-support 28/28, ark-sandbox 26/26); only `spec-actuators` differs (14 → 13) — see V-001. The 3 project SPECs reshaped without rule-count change.
- Test-binding resolution is exact: 150 distinct args, 0 unresolved — the core actuator promise (no dead bindings) holds. The 179 figure is total (non-unique) occurrences across feature SPECs.
- No surviving inline glyph (`⟨@`) in any live file (`.ark/specs`, `.ark/templates`, `.ark/workflow.md`, `.claude`, `.codex`, `.opencode`, `templates`): glyph scan returned `NONE`.
- Cross-platform parity (ark-verifier ×3, spec-audit ×3, agent bodies) is asserted by the passing suite: `agent_bodies_are_byte_identical_modulo_platform_idioms`, `every_claude_command_has_a_codex_skill_sibling`, `every_claude_command_has_an_opencode_command_sibling`, `opencode_command_bodies_have_opencode_frontmatter_and_arguments_token`, plus the codex/opencode frontmatter-shape and `*_spec_changelog_present` tests — all pass (visible in test output).
- The incidental opencode `spec-audit.md` H1 `$ARGUMENTS` fix is covered by `opencode_command_bodies_have_opencode_frontmatter_and_arguments_token` (passing).
- The deleted `extract-spec` command/skill across the three platforms (visible in diff stat) is carry-over from the prior `spec-actuators` work on this branch (rename extract-spec → spec-extract per project memory); present in this worktree's working tree, not introduced by the tagging change.
- Grammar consistency: `LAYOUT.md` L-3/L-9, the rewritten `spec-actuators` SPEC (Architecture + C-1/C-2), and the SPEC/PLAN/VERIFY templates were all updated to the two-line form; no live file still describes the old inline `⟨@⟩` form (glyph scan `NONE` corroborates).
