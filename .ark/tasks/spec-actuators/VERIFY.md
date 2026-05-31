# `spec-actuators` VERIFY

> Status: Closed by self-verify (main session). Convention+agent scope.
> Feature: `spec-actuators`
> Target Task: `spec-actuators`
> Owner: Implementer

---

## Severity Summary: 0 CRITICAL · 0 HIGH · 0 MEDIUM · 0 LOW
## Verification: build PASS · tests PASS · lint PASS · format PASS

All four gates run green from the worktree (`cargo build/test/clippy -D warnings/fmt --check`, each exit 0).

## Project Spec Compliance

- [x] LAYOUT.md L-9 + amended L-3 conform to Layout A (Ark's own SPEC). **PASS**
- [x] COMMENTS/STYLE/ERRORS: every `[**Rules**]` bullet carries one actuator tag (23/39/15); Exceptions untagged. **PASS**
- [x] C-23 (no task-mark tags in `crates/` comments): `grep -rnE '//.*(V-(UT|IT|E|F)-[0-9]|V-[0-9]{3})' crates/` is empty. **PASS**
- [x] No `crate::*` path bypasses; no `Command::new` introduced; no `reference/`/`target/` edits. **PASS**

## Related Feature Spec Compliance

- [x] `project-spec` (Layout A; NG-2/NG-3 relaxed for founding migration): only tags appended to rule bodies; ids/first-sentences preserved. **PASS**
- [x] `detachable-feature-spec`: `spec extract` unchanged (copies `## Spec` verbatim; author tags survive). **PASS**
- [x] `ark-context`: no JSON-contract change; only 1-line comment edits under `context/`. **PASS**
- [x] `subagent-support` (C-22 tri-platform byte-identity): the `ark-verifier` rubber bullet and the `spec-audit` body are byte-identical across claude/opencode/codex modulo frontmatter. **PASS**

## PRD Outcomes (convention+agent scope)

- [x] A constraint may declare an actuator tag (`tool`/`source-scan`/`test-binding`/`judgment`); grammar shipped self-contained. **PASS**
- [x] Tagging optional for users; untagged = judgment, never an error (NG-4). **PASS**
- [x] `ark-verifier` honors a rule's actuator tag; fail-closed on an unresolved `tool`/`test-binding` (never a silent pass). **PASS**
- [x] `/ark:spec-audit` skill ships on all three platforms; read-only by default; offers self-fix vs agent-assisted. **PASS**
- [x] workflow.md EXECUTE step states workflow IDs are bookkeeping, never in source (Layer-B root-cause fix). **PASS**
- [x] No Ark conventions ship to users: `find templates -path '*specs/project*' -type f` → only `INDEX.md`; no `L-9`/`LAYOUT.md` reference in any shipped template. **PASS**
- [x] No dead actuator code in the shipped library: `crates/ark-core/src/specs/` removed; no actuator test in `ark-cli`. **PASS**

## Plan Fidelity (01_PLAN Goals, convention+agent revision)

- [x] G-1 constraint may declare a tag — **PASS** (LAYOUT L-9; Ark SPECs tagged).
- [x] G-2 four kinds + judgment default — **PASS** (L-9).
- [x] G-3 verifier honors tags — **PASS** (rubric bullet, 3 platforms).
- [x] G-4 spec-audit skill — **PASS** (3 platforms, byte-identical).
- [x] G-5 workflow IDs never in source — **PASS** (workflow.md EXECUTE note + C-23 swept clean).

## SPEC Drift

- [x] No feature SPEC modified; the project convention-SPEC migration is PRD-sanctioned (founding migration). The `spec-actuators` SPEC is promoted at commit, not pre-existing. **N/A — no drift.**

## Findings

No findings. The earlier engine-era findings (V-001..V-010) are obsoleted by the maintainer decision to delete the native engine; the design is now convention+agent and the surface that produced those findings no longer exists.

## Notes

- Scope collapsed from the engine design during EXECUTE per maintainer direction: the native Rust engine (`crates/ark-core/src/specs/`) was deleted as dead code (its only caller was its own test); enforcement is now uniformly convention + agent. 01_PLAN `## Spec` was rewritten to match before this gate.
- `templates/ark/templates/VERIFY.md`'s `LAYOUT.md` mention is pre-existing (unchanged from HEAD) and refers to a user's own optional layout SPEC, not the actuator grammar — out of scope, left as-is.
