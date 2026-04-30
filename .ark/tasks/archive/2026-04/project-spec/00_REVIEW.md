# `project-spec` REVIEW `00`

> Status: Closed
> Feature: `project-spec`
> Iteration: `00`
> Owner: Reviewer (independent `code-reviewer` agent)
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 2 (R-001, R-002 — both HIGH; no CRITICAL)
- Non-Blocking Issues: 5 (R-003..R-007)

## Summary

The PLAN is well-structured and internally coherent for its primary goals. Layout A is sound, the two-commit boundary is clean, and the Phase 5 sub-pass ordering is sensible. The reviewer flagged two HIGH findings that require revision before execution: an ambiguity in the rule-count claim (CN-2) that lets the validation gate pass vacuously, and a missing rule on error-context wrapping in ERRORS.md that is the highest-impact omission for a CLI tool. Three MEDIUM findings (CN-4/CN-5 lacking executable validation, `clap` derive coupling unaddressed in Phase 5g, V-UT-2 citation regex too permissive) and two LOW findings (V-F-3 false-negative risk, 800-LOC cap as ceiling vs. typical) merit incorporation but do not block execution.

## Findings

### R-001 — CN-2 rule-count claim is ambiguous

- Severity: HIGH
- Section: Phase 2 — COMMENTS.md migration; CN-2; V-UT-1
- Problem:
  CN-2 requires "at least 28 rule entries"; Phase 2 maps NG-* rules into `[**Exceptions**]`, leaving only 23 (G+C) entries under `[**Rules**]`. V-UT-1's `rg '\bC-[0-9]+\b'` against `[**Rules**]` would yield 23, not 28. An automated verifier would either fail (literal interpretation) or pass vacuously (loose interpretation).
- Why it matters:
  CN-2 is the foundational gate for "1:1 rule preservation." If the validation has no precise pass condition, the migration's correctness is unverifiable.
- Recommendation:
  Tighten CN-2 and V-UT-1 to specify section-scoped counts: "≥23 rule entries under `[**Rules**]` (one per original G-* and C-*) and ≥5 entries under `[**Exceptions**]` (one per original NG-*)." Reviewer's exact count of the current COMMENTS.md confirmed: 12 G + 5 NG + 11 C = 28.

### R-002 — ERRORS.md missing error-context wrapping rule

- Severity: HIGH
- Section: Phase 4 — ERRORS.md authoring; E-1..E-14
- Problem:
  No rule governs context attachment when a lower-level error is wrapped at a module boundary. A bare `#[from] source: io::Error` variant produces user-visible messages like "No such file or directory" with zero context on the failing operation or path. E-12 covers context fields conceptually but does not mandate them at wrap sites.
- Why it matters:
  This is the most common defect in Rust CLI error handling. The omission means the SPEC will be technically complete but fail the user-experience test. Looking at `ark-core/src/error.rs`, the existing variants (`Io { path, source }`, `ManifestCorrupt { path, source }`) already follow this pattern — the SPEC must codify it.
- Recommendation:
  Add E-15: "When wrapping a foreign error, the variant must include at least one context field identifying the resource or operation (e.g. `path: PathBuf`, `command: String`). A bare `#[from] source: io::Error` variant is permitted only when the variant's `Display` template includes a contextualizing field." Cite [API Guidelines C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err).

### R-003 — CN-4 / CN-5 lack executable validation

- Severity: MEDIUM
- Section: Validation — Acceptance Mapping
- Problem:
  CN-4 ("no silent SPEC drift") and CN-5 ("authoritative-source rules from discarded draft preserved") map to "(covered by REVIEW iteration loop)". The discarded STYLE.md draft is not committed, so the verifier has no baseline to diff CN-5 against.
- Why it matters:
  CN-5's absence from VERIFY means the verifier cannot confirm G-3's claim that no rule was silently dropped.
- Recommendation:
  - For CN-5: in Phase 3, executor produces a mapping table (in a comment header inside STYLE.md, removed before commit, OR in the Phase 5 commit body) listing each rule from the previously-fetched authoritative source pages and its new S-N identifier. VERIFY checks this artifact exists. Acceptable alternative: include the mapping in the commit body of commit 1.
  - For CN-4: add V-F-4 — "If ERRORS.md was amended during Phase 5 to accommodate a `crates/` reality, confirm the amendment landed in commit 1 (not commit 2) via `git log -p .ark/specs/project/rust/ERRORS.md`."

### R-004 — Phase 5g `main.rs` split risks `clap` derive coupling

- Severity: MEDIUM
- Section: Phase 5g; T-4
- Problem:
  `ark-cli/src/main.rs` (1093 LOC) likely holds the top-level `clap` `Commands` enum and all subcommand structs. Splitting by subcommand requires re-exports back to the derive site, and `clap`'s `#[derive(Subcommand)]` is sensitive to type resolution. F-5 covers circular module deps but not derive macro breakage.
- Why it matters:
  Highest mechanical risk in the refactor. A naive split could change help text or argument names, violating NG-6's "no public CLI surface change."
- Recommendation:
  Add a Phase 5g pre-step: capture a baseline (`cargo run --quiet -- --help` and `--help` of every subcommand, saved to a temp file) before splitting. After split, diff the help output. If any subcommand's help text or argument names change, the split is rolled back. Add F-6 to the Failure Flow covering this case.

### R-005 — V-UT-2 citation regex too permissive

- Severity: MEDIUM
- Section: V-UT-2; CN-1
- Problem:
  `(.*\.)$` matches any line ending in any parenthesized expression with a period. Vague citations like `(see above.)` would pass.
- Why it matters:
  CN-1 is the foundational quality constraint for ERRORS.md and STYLE.md. A weak gate undermines it.
- Recommendation:
  Tighten V-UT-2 to require the citation to contain at least one of: `http`, `RFC`, `API Guidelines`, `Style Guide`, `Rust Book`, `thiserror`, `anyhow`, `std::error`. Document the regex pattern explicitly: `\([^)]*\(http\|RFC\|API Guidelines\|Style Guide\|Rust Book\|thiserror\|anyhow\|std::error\)[^)]*\.\)`.

### R-006 — V-F-3 task-mark-tag regex risks false negatives in citations

- Severity: LOW
- Section: V-F-3; CN-7
- Problem:
  After migration, valid SPEC rule IDs use prefixes C-/S-/E-. A `// CITATION: see C-3` comment in source would be flagged as a forbidden task-mark tag, while a process-annotation comment using a non-letter prefix (e.g. `// TICKET-123`) would slip through.
- Why it matters:
  C-11 in COMMENTS.md is hard ("never appear"). Edge cases need a clarification, not a regex change.
- Recommendation:
  Clarify CN-7: citing a SPEC rule inside a `// CITATION:` or doc-comment line is permitted; using a SPEC-style ID as a process annotation (TODO marker, ticket reference, code-review note) is forbidden. Reviewer judgment governs ambiguous cases at VERIFY.

### R-007 — 800-LOC cap is the global max, not typical target

- Severity: LOW
- Section: G-7; CN-8; Phase 5g
- Problem:
  Global rule says "200-400 typical, 800 max." PLAN treats 800 as the pass threshold. A split producing two 799-LOC files passes the literal gate but violates the spirit.
- Why it matters:
  Same purpose-fit argument that motivates this whole task: ceiling targets normalize bloat.
- Recommendation:
  In Phase 5g: if any post-split file exceeds 400 LOC, the executor adds a one-line note in the commit body explaining why further decomposition is impractical. Soft target, not a gate.

## Trade-off Advice

### TR-1 — Convention-SPEC layout legend location

- Related Plan Item: T-1
- Topic: Single source of truth vs. self-contained documents
- Reviewer Position: Prefer Option B — `LAYOUT.md` reference document
- Advice:
  Create `.ark/specs/project/LAYOUT.md` (~15 lines, reference document, not a template). Each SPEC's `[**Purpose**]` references it: "Layout: see `specs/project/LAYOUT.md`."
- Rationale:
  NG-1 forbids template files under `.ark/templates/`, but `LAYOUT.md` is a reference document, not a template. Three inline legends will drift within 2-3 SPEC edits. Option (b) gives future authors a single file to read when authoring MODULES.md, TESTING.md, etc.
- Required Action:
  If executor stays on Option (a), add V-E-4 — "Layout A legend across all three SPECs is identical word-for-word."

### TR-2 — Single ERRORS.md vs. ERRORS.md + ERROR-MESSAGES.md

- Related Plan Item: T-2
- Reviewer Position: Default upheld (single file)
- Advice:
  E-9, E-11, E-12 cover message phrasing adequately at this rule count.
- Required Action: None.

### TR-3 — Re-fetch official sources for Phase 3/4

- Related Plan Item: T-3
- Reviewer Position: Default upheld with addition
- Advice:
  Re-fetch `std::error::Error`, `thiserror`, `anyhow` regardless of cache. These have had API changes since common training-data cutoffs.
- Required Action:
  Confirm three fresh fetches in Phase 4.

### TR-4 — Phase 5g split as "no functional change"

- Related Plan Item: T-4
- Reviewer Position: Default acceptable with concrete guard
- Advice:
  Pure re-exports preserve API only when `pub use` is at the original module. Add the help-text baseline check from R-004 as the concrete guard.
- Required Action:
  Adopt R-004's recommendation.

### TR-5 — INDEX.md update deferred to user

- Related Plan Item: T-5
- Reviewer Position: Default correct
- Advice:
  Surface the proposed INDEX.md row text verbatim in VERIFY.md so the user can paste it in one step.
- Required Action: None (already implicit).

### TR-6 — `anyhow` in `ark-cli` only vs. project-wide `thiserror`

- Related Plan Item: T-6
- Reviewer Position: Default upheld
- Advice:
  `ark-cli` is a binary, `ark-core` is a library. The boundary rule is correct.
- Required Action:
  E-2 and E-3 in ERRORS.md should name `ark-core` and `ark-cli` directly rather than abstract "library crates" / "application code."
