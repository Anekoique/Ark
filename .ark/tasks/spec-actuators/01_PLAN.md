# `spec-actuators` PLAN `01`

> Status: Draft
> Feature: `spec-actuators`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Make enforcement a *declarable* property of a SPEC constraint via an optional inline actuator tag — `tool`, `source-scan`, `test-binding`, or `judgment`. Enforcement is **convention + agent**, not Ark code: `ark-verifier` (and an `/ark:spec-audit` skill) read a rule's tag and act — running the project's discovered lint/test command, grepping a `source-scan` pattern, or reasoning over `judgment`. Ark ships the L-9 grammar, the skill, and the verifier rubric; tagging is **optional for users** (an untagged rule is a valid judgment rule). Ark's own `COMMENTS`/`STYLE`/`ERRORS` adopt full tagging as a self-imposed convention, with `C-23` as the leak-catching pilot and all `crates/` workflow-ID labels stripped. The workflow EXECUTE step is amended so IDs never enter source — the root-cause (Layer B) fix.

## Log

[**Added**]

- Hybrid-by-kind enforcement model: deterministic kinds (`source-scan` native, `tool`/`test-binding` delegated) vs agent-judged (`judgment`).
- Language-agnostic `source-scan`: glob + extension-derived comment syntax; no Rust assumption.
- Per-file migration rule counts pinned (R-008).
- G-3 integration validation: a broken enforcer must make the check fail (R-007).
- C-13 (audit skill read-only default) given an explicit validation row (R-007).

[**Changed**]

- Enforcement is no longer "a `cargo test` engine." `source-scan` is Ark binary code over any project tree; `tool`/`test-binding` are discovered project commands (R-003, language-agnostic correction).
- `@tool` arg names a project lint/format command resolved by auto-discovery, never `clippy`/`rustfmt` in the schema (language-agnostic correction).
- `test-binding` arg names a concrete project test id, never a `V-*` bookkeeping label (R-006).
- Inline tag grammar pinned to an unambiguous fixed position parseable identically by Ark's scanner and the verifier agent (R-002).

[**Removed**]

- **The native Rust engine entirely** (`crates/ark-core/src/specs/` + its `ark-cli` dogfood test). Maintainer finding: it had exactly one caller — its own test — and coupled the shipped `ark-core` library to language-specific scanning. Enforcement is now uniformly convention+agent across all four kinds. This withdraws former G-2 ("Ark runs source-scan natively") and the whole engine Data Structure / API Surface.
- `DeadScan` and "zero matches is fatal" — moot once there is no engine; dead-rule cleanup is a human + CHANGELOG concern.
- Former feature-SPEC `extract` actuator-recording (old C-12) — deferred; `extract` copies the `## Spec` verbatim, so author-written tags already survive without code changes.

[**Unresolved**]

- None blocking. TR-1 (single-task vs split) noted as advisory; per-phase commit boundary stated in Implementation.

[**01_REVIEW response**]

- R-101 (HIGH, tool silent-pass) Accepted → C-8 fail-closed (undiscovered tool/test = FAIL) is in the Spec and the verifier rubric.
- R-102 (MED, unknown-extension) Accepted → the agent-run `source-scan` skips files it cannot comment-scope, never whole-line.
- R-103 (MED, mapping gaps) Accepted → Acceptance Mapping covers every C-N.
- R-104 (LOW, C-1 delimiter) Accepted → closing-`⟩` bound folded into C-1.

[**EXECUTE response (maintainer findings during implementation)**]

- Native engine deleted (see Removed) — it was dead code in the shipped library; enforcement is convention+agent.
- Shipped scaffold confirmed clean: only `INDEX.md` ships under `templates/ark/specs/project/`; Ark's Rust conventions never reach users.
- Tagging reframed as optional/freeing for users (NG-4); Ark's strict bar is self-imposed (C-13).

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `DeadScan` removed; zero matches = PASS (C-5 deleted from 00, replaced). Dead rules removed by human + CHANGELOG per maintainer guidance. |
| Review | R-002 | Accepted | Tag grammar pinned: actuator token is the final element of the rule's **first line**, immediately before terminal `.`, fenced as `⟨@kind: arg⟩`; arg bounded by closing `⟩`. Worked examples in Spec; V-UT-5 tests it. |
| Review | R-003 | Accepted | No compile-time embedding. `source-scan` reads project files via glob at run time; `tool`/`test-binding` resolve to discovered project commands. Matches the language-agnostic design. |
| Review | R-004 | Accepted | Verifier change is **additive**: it still applies every rule, but defers mechanical kinds to their deterministic result instead of re-judging. No `subagent-support` Constraint superseded (C-10/C-11 unchanged); C-22 tri-platform byte-identity preserved. Logged here. |
| Review | R-005 | Accepted | `source-scan` is comment-syntax-aware per file extension; self-non-flagging is a consequence of `CommentsOnly` scope (pattern data lives in code literals, never comments). V-UT-7. |
| Review | R-006 | Accepted | `test-binding` arg names a concrete test id resolvable by the project test runner, never a `V-*` label. V-UT-9. |
| Review | R-007 | Accepted | Added V-IT-4 (broken enforcer ⇒ check fails) and V-E-2 for C-13. |
| Review | R-008 | Accepted | Migration counts pinned: COMMENTS C-1..C-23 + EX-1..EX-5; STYLE S-1..S-39; ERRORS E-1..E-15; LAYOUT L-1..L-8. `[**Rules**]` bullets are tagged; `[**Exceptions**]` are not (carve-outs, not enforceable rules). |
| Review | TR-1 | Deferred | User chose one task. Per-phase commit boundary stated so a late-phase failure does not unwind earlier phases. |
| Review | TR-2 | Accepted | `Untagged` stays reported-not-fatal; V-IT-1 asserts the convention-SPEC `Untagged` count is zero after migration so the count is a real signal. |

---

## Spec

> Promoted verbatim to `specs/features/spec-actuators/SPEC.md` on deep commit.

[**Goals**]

- G-1: A SPEC constraint may declare how it is enforced via an inline actuator tag.
- G-2: The grammar defines four actuator kinds with one honest default (judgment).
- G-3: `ark-verifier` honors a rule's actuator tag instead of re-judging mechanical rules.
- G-4: An `/ark:spec-audit` skill reports tag health and offers self-fix or agent-assisted.
- G-5: Workflow templates state that workflow IDs never appear in shipped source.

[**Non-goals**]

- NG-1: No native enforcement engine in `ark-core`; tags are convention + agent behavior, not Rust code.
- NG-2: No mechanization of `judgment` rules; agent reasoning stays the actuator.
- NG-3: No language-specific assumption in the schema; tags name commands/tests/patterns, never a toolchain.
- NG-4: Tagging is optional for users; an untagged rule is a valid judgment rule, never an error.

[**Architecture**]

```
.ark/specs/project/
├── LAYOUT.md                                  (*) new L-9: inline actuator tag grammar; L-3 permits trailing tag
└── rust/{COMMENTS,STYLE,ERRORS}.md           (*) every [**Rules**] bullet carries a tag (Ark's own dogfooding)

templates/ark/specs/project/INDEX.md           (*) optional-tags note (shipped scaffold; user-facing)
templates/ark/templates/SPEC.md                (*) feature-SPEC constraints may carry a tag (optional)
templates/ark/workflow.md                      (*) EXECUTE: workflow IDs are bookkeeping, never in source
templates/{claude,codex,opencode}/commands|skills/.../spec-audit   (NEW) audit skill, per platform
templates/{claude,codex,opencode}/agents/ark-verifier.*            (*) honor actuator tags; fail-closed on unresolved tool/test
```

Enforcement is entirely **convention + agent**: there is no Ark binary code for actuators. `ark-verifier` (and `/ark:spec-audit`) read a rule's tag and act — running the project's `tool`/`test` command via the verifier's existing auto-discovery, grepping for a `source-scan` pattern, or reasoning over a `judgment` rule. Ark ships the grammar and the agent instructions, nothing more.

[**Data Structure**]

No Rust types. The actuator tag is a Markdown token; its grammar lives in `LAYOUT.md` L-9.

[**API Surface**]

No code API. The shipped surface is: the L-9 grammar, the `/ark:spec-audit` skill (three platforms), and the `ark-verifier` rubric addition.

[**Constraints**]

- C-1: An actuator tag is the final element of a rule's first line, `⟨@<kind>: <arg>⟩` before terminal punctuation; `<arg>` is bounded by the closing `⟩`, so a backticked path token inside it never terminates the tag.
- C-2: `<kind>` is `tool` | `source-scan` | `test-binding` | `judgment`; any other kind is malformed.
- C-3: A rule with no tag is reported as un-enforced (a judgment rule), never silently treated as enforced and never an error.
- C-4: `tool` names a project lint/format command key resolved by the verifier's existing auto-discovery; the schema names no toolchain.
- C-5: `source-scan` carries `<pattern> @ <glob>`; the pattern is a forbidden token checked against comment lines of files matching the glob, run by the agent.
- C-6: `test-binding` names a concrete project test id that must exist and pass, never a `V-*` workflow label.
- C-7: `judgment` may carry an optional proxy pattern flagging review candidates; it never fails the build.
- C-8: A `tool`/`test-binding` whose key resolves to no command is a VERIFY FAIL, never a silent pass.
- C-9: L-9 documents the supported `<glob>` (`**`, `*`, literals) and `<pattern>` token grammar (`\d`, `\d+`, `\d{N}`, `(a|b)`).
- C-10: `ark-verifier` applies every project-SPEC rule; for `tool`/`source-scan`/`test-binding` it consumes that kind's deterministic result, reasoning only over `judgment` and untagged rules.
- C-11: The `/ark:spec-audit` skill is read-only by default; it applies edits only after a per-run user choice of agent-assisted mode.
- C-12: The three platform bodies (`ark-verifier`, `spec-audit`) are byte-identical modulo per-platform frontmatter/heading idioms (subagent-support C-22).
- C-13: Tagging is optional for user projects; Ark's own convention SPECs adopt full tagging as a self-imposed convention, not a shipped requirement.
- C-14: No workflow ID (`V-*-N`, `C-N`, `G-N`, `R-NNN`) appears in any `crates/` comment; the constraint goes inline as prose, the label nowhere.

---

## Runtime

[**Main Flow**]

1. The agent (`ark-verifier` at VERIFY, or `/ark:spec-audit` on demand) reads a convention SPEC and, per rule, reads its actuator tag.
2. For `tool`/`test-binding`: resolve the command via auto-discovery and run it; non-zero or unresolved → FAIL.
3. For `source-scan`: grep the `<pattern>` against comment lines of files matching `<glob>`; any match → FAIL.
4. For `judgment` / untagged: the agent reasons, optionally guided by a proxy pattern.

[**Failure Flow**]

1. A malformed tag → the audit reports it; the author corrects it.
2. A `tool`/`test-binding` that resolves to no command → FAIL (C-8), never a silent pass.
3. A `source-scan` match or a failing command → a VERIFY FAIL Finding.

[**State Transitions**]

- A rule moves un-enforced → enforced when its author adds an actuator tag (optional; via hand-edit or the audit skill).

---

## Implementation

[**Phase 1**] — LAYOUT.md: add `L-9` (the actuator tag grammar incl. glob/pattern subset, C-8 fail-closed, optional-for-users); amend `L-3` to permit the trailing tag.

[**Phase 2**] — migrate Ark's own `COMMENTS`/`STYLE`/`ERRORS`: tag every `[**Rules**]` bullet per the audit buckets (`tool` for rustfmt/clippy rules, `source-scan` for pattern rules, `judgment` for taste rules); `C-23` pilot carries the id-leak `source-scan`. Strip every `V-*`/`C-N`/`G-N` label from `crates/` comments (C-14), prose preserved.

[**Phase 3**] — shipped user scaffold: add the optional-tags note to `templates/ark/specs/project/INDEX.md` and the optional tag mention to `templates/ark/templates/SPEC.md`. Ark ships the *option*, never Ark's own conventions.

[**Phase 4**] — Tier-3: amend `templates/ark/workflow.md` EXECUTE step so workflow IDs are PLAN/VERIFY bookkeeping, never carried into source.

[**Phase 5**] — ship `/ark:spec-audit` (claude/codex/opencode, byte-identical bodies per C-12) and add the actuator-aware rubric bullet to all three `ark-verifier` templates (C-10, C-8 fail-closed, byte-identical).

---

## Trade-offs

- T-1: Convention+agent vs native Rust engine — a native engine had exactly one caller (its own test); deleting it removes dead, language-coupling code from the shipped library and makes enforcement uniform across all four kinds (all agent-run). Cost: `source-scan` is no longer a can't-be-skipped `cargo test`; it relies on the agent running the grep, like `tool`/`test` already do.
- T-2: Optional vs mandatory tags — optional respects that Ark adapts to any project and that some conventions are irreducibly judgment; an untagged rule is honest, not a gap. Cost: a user must opt in to get machine-checking.
- T-3: Auto-discovery vs config section — reusing the verifier's discovery avoids a new `[spec-actuators]` config (and its upgrade-merge burden); cost is discovery's heuristic guesswork, already an accepted verifier property.
- T-4: Inline tag vs sidecar — inline keeps rule and enforcer in one place (no rule/guard drift); cost is machine syntax in prose, mitigated by a terse fenced token on the first line.

---

## Validation

[**Repo dogfood (Ark's own SPECs)**]

- V-1: Ark's `COMMENTS`/`STYLE`/`ERRORS` have every `[**Rules**]` bullet tagged; counts COMMENTS=23, STYLE=39, ERRORS=15 (manual + audit-skill run).
- V-2: `grep -rnE '//.*(V-(UT|IT|E|F)-[0-9]|V-[0-9]{3})' crates/` is empty — no workflow-ID leaks in source comments (C-14).

[**Shipped-surface checks**]

- V-3: `find templates -path '*specs/project*' -type f` returns only `INDEX.md` — no Ark conventions ship to users.
- V-4: `crates/ark-core/` and `crates/ark-cli/` contain no actuator engine or actuator test (no dead code shipped).
- V-5: the `/ark:spec-audit` skill ships on all three platforms and survives `load`/`unload`/`load`.
- V-6: the three `ark-verifier` bodies and the three `spec-audit` bodies are byte-identical modulo frontmatter (C-12); the OpenCode parity test passes.

[**Gate**]

- V-7: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all green.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-1, LAYOUT L-9 present |
| G-2 | LAYOUT L-9 (four kinds + optional default) |
| G-3 | V-6 + REVIEW of verifier bodies |
| G-4 | V-5, V-6 + REVIEW of skill bodies |
| G-5 | workflow.md EXECUTE note present |
| C-1..C-9 | LAYOUT L-9 documents the grammar; V-6 ships it |
| C-10 | verifier rubric bullet (V-6) |
| C-11 | spec-audit skill body (V-5) |
| C-12 | V-6 |
| C-13 | V-1 (Ark tagged) + V-3 (users get empty scaffold) |
| C-14 | V-2 |
