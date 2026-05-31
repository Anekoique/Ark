# `spec-actuators` PRD

---

[**What**]

Make every SPEC constraint declare *how it is enforced* — one of four actuators (`tool`, `source-scan`, `test-binding`, `judgment`) — inline in the rule bullet, backed by a generic engine that runs the declared checks and a health meta-check that fails on any claimed-but-broken enforcer.

[**Why**]

Today a SPEC constraint is prose. Whether it is actually enforced is invisible: rules with a tool/test behind them hold (`C-1`/`commands_no_bare_command_new`, the rustfmt/clippy rules), while prose-only rules drift silently — `C-23` ("no task-mark tags in source") leaked ~21 times during `ark-sandbox` and the verifier rubber-stamped it because it could only check by reading semantics. The dividing line is not clarity or importance; it is whether an actuator outside the model's attention enforces the rule. A constraint whose enforcement mechanism is undeclared defaults to the weakest actuator: an LLM guessing from prose. The fix is to make the actuator a declared, executable property of every constraint, so enforcement status is never unknown.

[**Outcome**]

- Every rule in `COMMENTS.md` / `STYLE.md` / `ERRORS.md` carries an inline actuator tag; no rule is silently un-tagged.
- A generic engine (a `cargo test` target in `ark-core`) reads SPEC files via `include_str!`/`include_dir!`, parses each rule's actuator, and runs `source-scan` regexes and `test-binding` assertions against `crates/` sources.
- A health meta-check **hard-fails the build** when a rule claims a `tool`/`source-scan`/`test-binding` enforcer that does not parse, does not run, or matches nothing (a dead/broken guard that *looks* enforced). It **reports** the `judgment` count and any un-tagged rules.
- `judgment` rules carry an optional non-failing proxy hint that flags review candidates without failing the build; the standing `judgment` count is surfaced.
- `C-23` is migrated as the pilot and its leak class is caught by a `source-scan` actuator (scanning test docstrings, not just non-test code).
- `ark agent spec extract` records an actuator for each promoted feature-SPEC constraint (default `test-binding` to the PLAN's validation).
- The workflow / EXECUTE / PLAN templates state that validation/SPEC/Goal IDs are PLAN/VERIFY bookkeeping only and never appear in `crates/`.
- An audit skill scans all SPECs (project + feature), reports SPEC-structure and actuator-tag problems with proposed fixes, and prompts per run: fix-yourself or agent-assisted.
- `LAYOUT.md` grammar (Layout A) is extended to define the inline actuator syntax; the feature-SPEC `SPEC.md` template documents it. Full workspace `build` / `test` / `clippy -D warnings` / `fmt --check` green.

[**Related Specs**]

- `specs/features/project-spec/SPEC.md` — defines Layout A and the convention-SPEC files (`COMMENTS`/`STYLE`/`ERRORS`). This task extends Layout A's rule grammar (`L-3`) with the inline actuator syntax and migrates all three files. `project-spec` NG-2/NG-3 ("agent does not edit `specs/project/`") are relaxed *for this founding migration only*, with maintainer diff review; the audit skill maintains the files thereafter.
- `specs/features/detachable-feature-spec/SPEC.md` — establishes the `commands_no_bare_command_new` source-scan precedent (referenced there as `C-28`) and the `ark agent spec extract` / `register` path. The engine generalizes that precedent (rules read from SPEC files, not hard-coded); the extract path gains actuator recording.
- `specs/features/ark-context/SPEC.md` — the existing guard lives beside the `context` module; the engine is sited near it and the health summary may surface through `ark context`. No change to the session/phase JSON contract unless health reporting is added there.
- `specs/features/subagent-support/SPEC.md` — `ark-verifier` behavior changes: for `tool`/`source-scan`/`test-binding` rules it consumes the engine's pass/fail instead of reading prose; it applies semantic judgment only to `judgment`-tagged rules.

[**SPEC Path**]

`spec-actuators`
