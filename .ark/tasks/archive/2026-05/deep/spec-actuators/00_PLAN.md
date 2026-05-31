# `spec-actuators` — Plan (iteration 00)

---

## Summary

Make enforcement a declared property of every SPEC constraint. Each rule gains an inline actuator tag — `tool`, `source-scan`, `test-binding`, or `judgment` — parsed by a generic engine that runs the mechanical checks at `cargo test` time and a health meta-check that fails the build on any claimed-but-broken enforcer. The convention SPECs (`COMMENTS`/`STYLE`/`ERRORS`) are migrated as a self-dogfooding act with `C-23` as the pilot; the feature-SPEC extract path records actuators; the workflow templates sever the ID-leak coupling; and an audit skill maintains the tags going forward.

## Log

*None in 00_PLAN.*

## Spec

> This section is promoted **verbatim** to `specs/features/spec-actuators/SPEC.md` on deep commit.

### Goals

- G-1: Every convention-SPEC rule declares exactly one inline actuator tag.
- G-2: A generic engine parses actuators from SPEC files and runs the mechanical ones.
- G-3: A health meta-check fails the build on any claimed-but-broken or dead enforcer.
- G-4: Feature-SPEC promotion records an actuator per extracted constraint.
- G-5: An audit skill reports SPEC/tag problems and offers self-fix or agent-assisted.

### Non-goals

- NG-1: No removal or rewrite of existing rule *semantics*; migration adds tags only.
- NG-2: No attempt to mechanize genuinely-judgment rules; `judgment` stays a first-class actuator.
- NG-3: No new runtime CLI subcommand; the engine is a `cargo test` target, not `ark <verb>`.

### Architecture

```
crates/ark-core/src/specs/                    (NEW module: the actuator engine)
├── mod.rs                                     # public re-exports; hosts the engine #[test]s
├── actuator.rs                                # Actuator enum + parse from a rule bullet
├── rule.rs                                    # Rule { id, prefix, body, actuator }; parse a SPEC file
├── scan.rs                                    # source-scan executor over crates/**/*.rs comment lines
└── health.rs                                  # meta-check: claimed enforcer parses/runs/matches

crates/ark-core/src/specs/embedded.rs         # include_dir!/include_str! of .ark/specs/project/**

.ark/specs/project/
├── LAYOUT.md                                  (*) L-3 grammar extended: inline actuator syntax; new L-9
└── rust/{COMMENTS,STYLE,ERRORS}.md           (*) every rule gains an inline actuator tag

crates/ark-core/src/commands/agent/spec/
└── extract.rs                                 (*) records actuator per constraint (default test-binding)

templates/ark/workflow.md                      (*) Tier-3: IDs are PLAN/VERIFY-only, never in crates/
templates/ark/templates/{PLAN,SPEC}.md         (*) Tier-3 note + actuator syntax in the SPEC template
templates/{claude,codex,opencode}/...spec-audit (NEW) audit skill, per-platform
templates/{claude,codex,opencode}/agents/ark-verifier.* (*) consume engine results; judge only judgment
```

### Data Structure

```rust
// crates/ark-core/src/specs/actuator.rs
pub enum Actuator {
    Tool { name: String },                 // rustfmt | clippy::<lint> — delegated, declared only
    SourceScan { pattern: String, scope: ScanScope }, // token-grammar pattern run by the engine (no regex dep)
    TestBinding { test: String },          // a named #[test] must exist and pass
    Judgment { proxy: Option<String> },    // human-judged; optional non-failing review-candidate regex
}

pub enum ScanScope { CommentsOnly, AllSource }  // comment-line vs whole-file scan

// crates/ark-core/src/specs/rule.rs
pub struct Rule {
    pub id: String,        // "C-23"
    pub prefix: char,      // 'C'
    pub title: String,
    pub actuator: Actuator,
}

// crates/ark-core/src/specs/health.rs
pub enum HealthFinding {
    Untagged { rule: String },                       // no actuator — reported, not fatal by default
    DeadScan { rule: String, pattern: String },      // source-scan matches nothing — fatal
    UnparseableActuator { rule: String, reason: String }, // fatal
    MissingTest { rule: String, test: String },      // test-binding names a #[test] that doesn't exist — fatal
}

pub struct HealthReport {
    pub findings: Vec<HealthFinding>,
    pub judgment_count: usize,
    pub by_actuator: BTreeMap<&'static str, usize>,
}
```

### API Surface

```rust
// crates/ark-core/src/specs/
pub fn parse_rules(spec_markdown: &str) -> Result<Vec<Rule>>;
pub fn run_source_scan(rule: &Rule, sources: &[(&str, &str)]) -> Vec<ScanViolation>;
pub fn health_check(rules: &[Rule], sources: &[(&str, &str)]) -> HealthReport;

// crates/ark-core/src/commands/agent/spec/extract.rs (unchanged signature; behavior added)
pub fn spec_extract(opts: SpecExtractOptions) -> Result<SpecExtractSummary>;
```

### Constraints

- C-1: An actuator tag is the trailing inline token of a rule bullet, syntax `⟨@<kind>[: <arg>]⟩`, parsed by `actuator::parse`.
- C-2: `<kind>` is one of `tool` | `source-scan` | `test-binding` | `judgment`; any other kind is `UnparseableActuator`.
- C-3: A rule bullet with no actuator token parses as `HealthFinding::Untagged`; it is reported, never silently treated as enforced.
- C-4: A `source-scan` actuator carries a token-grammar pattern that parses; a pattern that fails to parse is `UnparseableActuator` (fatal).
- C-5: A `source-scan` whose pattern matches zero lines across its declared scope is `DeadScan` (fatal) — a guard for a deleted target must not masquerade as enforced.
- C-6: A `test-binding` names a `#[test]` whose identifier exists in the crate sources; an absent test is `MissingTest` (fatal).
- C-7: A `tool` actuator names `rustfmt` or `clippy::<lint>`; the engine asserts the name shape only, delegating execution to the existing CI steps.
- C-8: A `judgment` actuator may carry a proxy pattern; proxy matches are reported as review candidates and never fail the build.
- C-9: The engine reads project SPECs via `include_dir!`/`include_str!` at compile time; no runtime filesystem read for the test target.
- C-10: The migration preserves every existing rule's id and first-sentence body byte-for-byte; only the trailing actuator token is added.
- C-11: `spec_extract` writes an actuator token onto every constraint it promotes; the default is `test-binding` naming the constraint's mapped `V-*` test, downgraded to `judgment` when no test maps.
- C-12: The audit skill is read-only by default and applies edits only after a per-run user choice of agent-assisted mode.
- C-13: A `source-scan` executor never flags its own pattern string literals; pattern arguments live only inside `scan.rs` data, not as `crates/` comments.

## Runtime

The engine has no runtime path — it executes only under `cargo test`. Each `#[test]` in `specs/mod.rs` (one per convention SPEC) `include_str!`s the SPEC, calls `parse_rules`, then `health_check(rules, SOURCES)`. The test asserts the `HealthReport` has no fatal finding (`DeadScan`/`UnparseableActuator`/`MissingTest`) and prints the `judgment_count` + per-actuator histogram via `eprintln!` for visibility. `source-scan` rules additionally run `run_source_scan` and assert zero violations. The verifier, at VERIFY time, runs `cargo test` and reads pass/fail for every `tool`/`source-scan`/`test-binding` rule; it applies semantic judgment only to `judgment`-tagged rules and their proxy-flagged candidates.

## Implementation

- **Phase 1 — engine core.** Add `crates/ark-core/src/specs/{mod,actuator,rule,scan,health,embedded}.rs`. Pure parsing + scanning over in-memory strings; full unit coverage with inline fixtures. No SPEC files touched yet.
- **Phase 2 — LAYOUT.md grammar.** Extend Layout A: new rule `L-9` defining the inline actuator token syntax; amend `L-3`'s rule-bullet form. Self-dogfooding edit to a `specs/project/` file (agent-writes-with-review).
- **Phase 3 — migrate convention SPECs.** Add the trailing actuator token to every rule in `COMMENTS`/`STYLE`/`ERRORS` per the audit buckets (~`tool` for rustfmt/clippy rules, `source-scan` for pattern rules, `test-binding` for structural rules, `judgment` for taste rules). The pilot rule's `source-scan` arg carries the id-leak pattern (`comments-only` scope across all `crates/**/*.rs` incl. test docstrings; tokens `C-\d+`, `V-(UT|IT|E|F)-\d`, `V-\d{3}`). Wire the per-SPEC `#[test]`s.
- **Phase 4 — feature-SPEC extract.** Teach `spec_extract` to emit an actuator token per constraint (C-12). Update the `SPEC.md` template + PLAN template with the syntax and the Tier-3 note.
- **Phase 5 — Tier-3 workflow fix.** Amend `workflow.md` + EXECUTE/PLAN slash-command templates (all three platforms) that IDs are PLAN/VERIFY bookkeeping, never carried into source.
- **Phase 6 — audit skill + verifier.** Ship `spec-audit` (claude/codex/opencode) and update `ark-verifier` to consume engine results and judge only `judgment` rules.

## Trade-offs

- **Inline tag vs sidecar (chosen: inline).** Inline keeps each rule and its enforcer in one place — no drift between a rule and a separate guard file, the exact failure we are fixing. Cost: machine syntax in human prose; mitigated by a terse `⟨@kind⟩` token at line end.
- **`DeadScan` fatal vs warning (chosen: fatal).** A guard that matches nothing looks enforced but isn't — strictly worse than an honest `judgment` tag. Fatal forces the author to either fix the pattern or downgrade to `judgment`. Risk: a legitimately-empty-today scope (no current violations) fails; mitigated by allowing a `source-scan` to also assert "must match the rule's own example" or by scoping dead-detection to patterns expected to match structural anchors, resolved in REVIEW.
- **Untagged fatal vs reported (chosen: reported).** Forcing full migration before green would block the build during the migration itself. Reported-and-counted keeps the build green while making the gap visible; a follow-up may promote it to fatal once the count is zero.
- **Engine as `cargo test` vs `ark` subcommand (chosen: test).** A test target inherits CI for free and matches the `commands_no_bare_command_new` precedent. The audit *skill* covers the interactive need without a runtime command.
- **Risk:** the `subagent-support` verifier change is behavior to a shipped feature; scoped narrowly (consume results, judge judgment-only) and validated against the SPEC in REVIEW.
- **Resolved (no regex dep):** `regex` is absent from the workspace (`Cargo.lock` has no `regex` entry), and the `commands_no_bare_command_new` precedent uses plain `str::contains`. `source-scan` patterns are restricted to a small token grammar (id-shaped alternations like `C-\d+`, `V-(UT|IT|E|F)-\d`, `V-\d{3}`) matched by a hand-rolled scanner in `scan.rs` — no new dependency. C-4's "regex compiles" becomes "pattern parses into the token grammar".

## Validation

- V-UT-1: `actuator::parse` round-trips each of the four kinds and rejects unknown kinds (C-1, C-2).
- V-UT-2: `parse_rules` returns one `Rule` per bullet and an `Untagged` finding for a tag-less bullet (C-3).
- V-UT-3: a `source-scan` pattern that fails the token grammar yields `UnparseableActuator` (C-4).
- V-UT-4: a `source-scan` matching zero lines yields `DeadScan` (C-5).
- V-UT-5: a `test-binding` naming an absent `#[test]` yields `MissingTest` (C-6).
- V-UT-6: a `tool` actuator with a non-`rustfmt`/non-`clippy::` name is rejected (C-7).
- V-UT-7: a `judgment` proxy match is reported as a candidate and never fatal (C-8).
- V-UT-8: a `comments-only` scan flags a fixture with `/// V-UT-7:` in a test docstring and a `(C-24)` label (C-1, ScanScope).
- V-UT-9: a `source-scan` does not flag id-shaped tokens that appear only inside the engine's own pattern literals (C-13).
- V-IT-1: the per-SPEC `#[test]`s parse the real `COMMENTS`/`STYLE`/`ERRORS` with zero fatal findings (C-9, C-10).
- V-IT-2: `crates/**/*.rs` is clean under the migrated `C-23` rule's scan (C-9).
- V-IT-3: `spec_extract` over a fixture PLAN emits an actuator token on every promoted constraint (C-11).
- V-F-1: `cargo fmt --check`, `cargo clippy -D warnings`, full `cargo test` green.

### Acceptance Mapping

| Goal | Validation |
|------|------------|
| G-1 (every rule tagged) | V-UT-2, V-IT-1 |
| G-2 (engine parses + runs) | V-UT-1, V-UT-7, V-UT-8, V-UT-9, V-IT-1, V-IT-2 |
| G-3 (health fails broken) | V-UT-3, V-UT-4, V-UT-5, V-UT-6 |
| G-4 (extract records actuator) | V-IT-3 |
| G-5 (audit skill) | V-F-1 + REVIEW of skill body (no code path; manual) |
