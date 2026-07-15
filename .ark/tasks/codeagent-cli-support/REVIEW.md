# `codeagent-cli-support` REVIEW

> Status: Open
> Feature: `codeagent-cli-support`
> Owner: Reviewer
> Target Plan: `PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Rejected
- Blocking: 2
- Non-blocking: 5

## Summary

The PLAN is architecturally sound and fits the existing platform-registry pattern precisely. The Spec is largely self-contained, template correctness is verified against the live `.cac/agents/` files, and no contradictions with existing feature SPECs were found. However, two HIGH-severity findings block approval: (1) the Data Structure section contains a factual error (`templates: &CODEX_TEMPLATES` instead of `&CODEAGENT_TEMPLATES`), which violates self-containment because a reader cannot derive the correct value from the Spec alone; (2) the PRD outcome for `ark upgrade` has no Goal or Validation, leaving a coverage gap. Five non-blocking findings address missing Goals for `ark context`, goal phrasing, constraint validation gaps, constraint length, and non-goal clarity.

---

## Findings

### R-001 Data Structure has incorrect templates field reference

- **Severity:** HIGH
- **Section:** [**Data Structure**]
- **Problem:** The `CODEAGENT_PLATFORM` const definition reads `templates: &CODEX_TEMPLATES, // WRONG, see below` (PLAN line 84). The comment "WRONG, see below" is the only indicator this is an error, and no correction appears below in the Spec. A reader implementing from the Spec alone would produce a `CODEAGENT_PLATFORM` that points at Codex's template tree.
- **Why it matters:** The Spec's Data Structure section must be self-contained and correct as written. A "see below" note that never corrects the value breaks the self-containment contract. If followed literally, the implementation would extract Codex skill templates into `.cac/commands/ark/`, producing wrong frontmatter and content.
- **Recommendation:** Change line 84 to `templates: &CODEAGENT_TEMPLATES,` and remove the "WRONG, see below" comment. The `CODEAGENT_TEMPLATES` static is defined later in the same section; the reference is unambiguous once corrected.

### R-002 No Goal or Validation for ark upgrade

- **Severity:** HIGH
- **Section:** [**Goals**], [**Validation**]
- **Problem:** PRD outcome 4 states "`ark upgrade` refreshes CodeAgent CLI templates alongside other platforms." No Goal addresses upgrade behavior, and the Acceptance Mapping has no V-IT covering `ark upgrade` for CodeAgent CLI.
- **Why it matters:** The architecture implies upgrade works (commands iterate `PLATFORMS`), but the PRD outcome is an explicit acceptance criterion. Without a Goal and validation, a regression that skips CodeAgent CLI during upgrade would pass the test suite. This is the same pattern that caught Codex initially — the codex-support SPEC has C-7 (`load_after_replay_re_applies_canonical_entries`) and the upgrade SPEC has template-refresh coverage for each registered platform.
- **Recommendation:** Add a Goal (e.g., G-6: "ark upgrade refreshes codeagent-cli templates alongside other platforms") and an integration test (e.g., V-IT-5: "`ark upgrade --dir $TMP` refreshes `.cac/` command and agent templates when codeagent-cli is installed"). Map G-6 to V-IT-5 in the Acceptance Mapping.

### R-003 No Goal for ark context reporting CodeAgent CLI

- **Severity:** MEDIUM
- **Section:** [**Goals**]
- **Problem:** PRD outcome 5 states "`ark context --scope session` reports CodeAgent CLI as an installed platform." V-IT-4 covers this validation, but no Goal maps to it and the Acceptance Mapping has no row for V-IT-4.
- **Why it matters:** A PRD outcome without a backing Goal can be silently dropped during implementation. The validation exists but is orphaned from the goal structure.
- **Recommendation:** Add a Goal (e.g., G-7: "ark context lists codeagent-cli as an installed platform") and map it to V-IT-4 in the Acceptance Mapping.

### R-004 Goals are state-descriptions, not capability-oriented verb-led statements

- **Severity:** MEDIUM
- **Section:** [**Goals**]
- **Problem:** All five Goals (G-1 through G-5) describe current state rather than leading with a verb. Per the review criterion, Goals should be capability-oriented (verb-led, the *what*). Examples: G-1 "PLATFORMS registry grows to 4" should be "Register codeagent-cli as 4th platform in PLATFORMS"; G-2 "CodeAgent CLI artifacts ship at..." should be "Ship codeagent-cli artifacts at .cac/commands/ark/ and .cac/agents/".
- **Why it matters:** This matches the existing SPEC convention (codex, opencode, subagent all use the same descriptive pattern), so the deviation is inherited rather than introduced. However, the review criterion is explicit about verb-led goals, and the inherited pattern should not propagate further.
- **Recommendation:** Rewrite G-1 through G-5 with verb-led openings. This is non-blocking for this review but should be addressed to set a better precedent.

### R-005 C-14 lacks a proper validation mapping

- **Severity:** MEDIUM
- **Section:** [**Validation**] — Acceptance Mapping
- **Problem:** C-14 (`@judgment`: "No changes to Snapshot schema, HookFileSpec struct, or any command body apart from registry growth and layout consts") maps to "No Snapshot/HookFileSpec changes" in the Acceptance Mapping, which is a restatement of the constraint, not a validation.
- **Why it matters:** A judgment-tagged constraint still needs a validation entry. Without one, there is no mechanical or review gate confirming that the structs were not modified. A future contributor could add a field to `HookFileSpec` or `SnapshotHookBody` without any test catching the regression.
- **Recommendation:** Replace the restatement with a concrete validation. Options: (a) add a source-scan test confirming `HookFileSpec` has exactly 6 fields and `SnapshotHookBody` has exactly 5 fields; or (b) add a judgment-gate V-J entry in the Acceptance Mapping that the reviewer must explicitly sign off on.

### R-006 Several Constraints exceed the 120-char single-sentence limit

- **Severity:** LOW
- **Section:** [**Constraints**]
- **Problem:** C-1, C-6, C-11, and C-14 each contain two sentences or compound clauses exceeding 120 characters. Per the review criterion, Constraints should be one declarative sentence <= 120 chars.
- **Why it matters:** This is a formatting concern that does not affect correctness. The constraints are semantically clear despite the length.
- **Recommendation:** Split multi-sentence constraints into separate entries or tighten phrasing. Non-blocking.

### R-007 Non-goal NG-1 could be misread

- **Severity:** LOW
- **Section:** [**Non-goals**]
- **Problem:** NG-1 says "No `.cac/config.toml` or other extra config file; CodeAgent CLI needs none." This could be misread as "CodeAgent CLI has no extra files at all," when in fact it has `extra_files: &[]` (empty but present as a field) and still gets the `AGENTS.md` managed block and `.cac/settings.json` hook entry.
- **Why it matters:** Minor clarity issue. The current wording contrasts with Codex which does ship a `config.toml`, so the intent is clear in context.
- **Recommendation:** Consider rewording to: "No `.cac/config.toml`; CodeAgent CLI uses JSON hooks natively and needs no config file." Non-blocking.

---

## Trade-off Advice

### TR-1 `>/dev/null` in hook command is platform-appropriate

- **Related Plan Item:** T-1
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Keep with clarification
- **Advice:** The separate `CODEAGENT_CONTEXT_HOOK_COMMAND` const (distinct from `ARK_CONTEXT_HOOK_COMMAND`) is the correct pattern. It keeps `identity_value` self-consistent within the `HookFileSpec`. The orphan-hook scan in `unload` (line 229 of `unload.rs`) hard-codes `ARK_CONTEXT_HOOK_COMMAND`, but this only scans unregistered files; CodeAgent CLI's hook is registered in `PLATFORMS`, so the first-stage `capture_hook` handles it correctly using its own `identity_value`.
- **Rationale:** Adding `>/dev/null` to `ARK_CONTEXT_HOOK_COMMAND` itself would break Claude's hook (Claude may use stdout). A separate const is the minimal correct approach.
- **Required Action:** Keep with clarification — add a note in the Spec or a comment in the code explaining why the orphan-scan does not need to match `CODEAGENT_CONTEXT_HOOK_COMMAND`.

### TR-2 `--codeagent` flag naming is consistent

- **Related Plan Item:** T-2
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer `--codeagent`
- **Advice:** The flag names the platform identity, not the directory, following the precedent of `--claude`, `--codex`, `--opencode`.
- **Rationale:** Directory names may change; platform identity is stable. Consistency across the flag set aids discoverability.
- **Required Action:** Keep.

### TR-3 Agent frontmatter format is verified correct

- **Related Plan Item:** T-3
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Keep
- **Advice:** The existing `.cac/agents/ark-*.md` files in the repository confirm the exact format: `name`, `description`, `permissionMode: bypassPermissions`, `tools` (YAML list). This matches the PLAN's specification and the review criterion for CodeAgent CLI agent frontmatter.
- **Rationale:** Per-platform frontmatter differences are expected and documented. The PLAN correctly identifies the format.
- **Required Action:** Keep.

---

## PRD Coverage Matrix

| PRD Outcome | Covered By | Gap? |
|---|---|---|
| `ark init --codeagent` scaffolds `.cac/` artifacts | G-2, G-3, G-4, G-5 | No |
| `ark unload` / `ark load` round-trips CodeAgent CLI artifacts | G-1 (implied), V-IT-2 | No |
| `ark remove` cleans up all CodeAgent CLI artifacts | G-1 (implied), V-IT-3 | No |
| `ark upgrade` refreshes CodeAgent CLI templates | **None** | **Yes — R-002** |
| `ark context --scope session` reports CodeAgent CLI | V-IT-4 only, no Goal | **Yes — R-003** |
| All existing tests pass; new tests cover shape/parity | Phase 3 | No |

## Spec Consistency Check

| Existing SPEC | Consistent? | Notes |
|---|---|---|
| codex-support | Yes | CodeAgent CLI mirrors Codex's hook pattern (seconds timeout, JSON shape). No contradictions. CHANGELOG entry planned (Phase 3 step 11). |
| opencode-support | Yes | CodeAgent CLI shares `AGENTS.md` managed block (manifest dedupes on `(file, marker)`). Command frontmatter uses same `description`-only format. No contradictions. CHANGELOG entry planned. |
| subagent-support | Yes | CodeAgent CLI ships 3 agents via `agents_templates` + `agents_dest_dir`. `extra_dirs = &[]` (`.cac/agents/` nests under `removal_root`). Agent body parity constraint (C-7, C-8) extends existing pattern. No contradictions. CHANGELOG entry planned. |
