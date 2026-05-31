# `project-spec` PLAN `02`

> Status: Draft
> Feature: `project-spec`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Address the three findings from REVIEW 01: R-101 (HIGH — V-F-4 should not assume `git commit --amend` is always viable), R-102 (MEDIUM — E-15 exception clause is self-defeating), R-103 (LOW — NG-1's "reference document" vs. "template" category needs a definition). All three accepted. No regressions to other Goals/Constraints/Validations. The Spec body is carried forward from PLAN 01 with only the listed targeted edits.

## Log `Iteration 02 deltas`

[**Added**]

- Sentence in `LAYOUT.md` (drafted in Phase 1) defining "reference document" vs. "template" (per R-103). Wording: "A reference document describes a convention or format. A template contains placeholder sections intended to be copied and filled in."

[**Changed**]

- **V-F-4** (per R-101): replaces the `--amend`-only mechanism with a two-option path — amend if commit 1 is HEAD and unpushed; otherwise create an adjacent fixup commit between commits 1 and 2. The mandatory invariant is unchanged: `git log -- .ark/specs/project/rust/` shows the SPEC change in a commit that precedes commit 2.
- **F-3** (per R-101): clarified that the F-3 path ("relax via `[**Exceptions**]` or scope to follow-up FU-N") is the preferred mechanism; the V-F-4 commit-fixup path is the secondary mechanism for unavoidable amendments.
- **Phase 4 — E-15 wording** (per R-102): the self-defeating exception clause is removed. New rule reads: "When wrapping a foreign error, the variant must include at least one context field identifying the resource or operation (e.g. `path: PathBuf`, `command: String`)." Single-purpose error enums whose name supplies context (rare in `ark-core`) may be added as an explicit `[**Exceptions**]` entry inside ERRORS.md only if such a case actually arises; the PLAN does not pre-declare it.

[**Removed**]

- The "permitted-exception" sub-clause of E-15 (per R-102). No content lost — the exception was vacuous; its removal restores E-15 to a single concrete obligation.

[**Unresolved**]

- None. R-101, R-102, R-103 all incorporated.

[**Response Matrix**]

| Source | ID    | Decision | Resolution                                                                                  |
| ------ | ----- | -------- | ------------------------------------------------------------------------------------------- |
| Review | R-101 | Accepted | V-F-4 reworded with two-option path; F-3 clarified as preferred. No `--amend` requirement.  |
| Review | R-102 | Accepted | E-15 exception clause removed; rule simplified to a single concrete obligation.             |
| Review | R-103 | Accepted | Definition sentence added to LAYOUT.md drafting (Phase 1 deliverable).                      |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here. *(R-101 is the only HIGH this iteration.)*
> - Rejections must include explicit reasoning. *(None this iteration — all accepted.)*

---

## Spec `Convention-SPEC layout, plus three convention SPECs`

[**Goals**]

- **G-1: Convention-SPEC layout (Layout A) is defined once and applied uniformly.** Layout A's section structure is `[**Purpose**] / [**Rules**] / [**Exceptions**] / [**Examples**] / [**See Also**]`, with a flat numbered list under `[**Rules**]` and a single rule prefix per file. The layout is documented authoritatively in `.ark/specs/project/LAYOUT.md`; convention SPECs link to it from their `[**Purpose**]` paragraph. LAYOUT.md also defines what qualifies as a "reference document" vs. a "template."

- **G-2: COMMENTS.md migrated to Layout A with zero rule loss.** Every existing rule (G-1..G-12, NG-1..NG-5, C-1..C-11) is preserved verbatim or with mechanical relabeling, mapped onto the new section structure under prefix `C-N`. NG-* rules become items under `[**Exceptions**]`; G-* and C-* rules become items under `[**Rules**]`. Citations preserved.

- **G-3: STYLE.md authored in Layout A from authoritative sources.** Every rule cites at least one of: the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/), [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), or the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Single rule prefix `S-N`. Phase 3 emits a rule-source mapping into commit 1's commit body.

- **G-4: ERRORS.md authored in Layout A from authoritative sources.** Every rule cites at least one of: [The Rust Programming Language Book — Chapter 9 (Error Handling)](https://doc.rust-lang.org/book/ch09-00-error-handling.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (especially [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err) and [C-VALIDATE](https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate)), [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html), [`thiserror` crate documentation](https://docs.rs/thiserror/), [`anyhow` crate documentation](https://docs.rs/anyhow/), [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html). Single rule prefix `E-N`.

- **G-5: `crates/` mechanically compliant.** `cargo fmt --check` passes; `cargo clippy --all-targets --all-features -- -D warnings` passes; no `get_*` getters; casing per RFC 430; conversion-method semantics correct; comment formatting per COMMENTS.md.

- **G-6: `crates/` error-handling-compliant per ERRORS.md.** Canonical `Error` enum unchanged in identity but rule-conformant in shape; consistent `?` discipline; no `unwrap()` in non-test code; error messages follow phrasing rules; `Result<T>` alias used at the boundary; `#[from]` and `#[source]` consistent; **wrapped-error variants carry context fields per E-15**.

- **G-7: `crates/` structurally cleaner.** Files exceeding the 800-line cap are decomposed. Module organization remains feature-grouped. **Soft target: each post-split file ≤400 LOC; deviations explained in commit 2's commit body.**

- **G-8: Two commits on `feat/project-spec`.** Commit 1 — `feat(specs): add project-spec` — touches only `.ark/specs/project/`. Commit 2 — `refactor(crates): comply with project specs` — touches only `crates/`. Each commit is independently `cargo check --all-targets` green. *(See V-F-4 for the rare-case fixup-commit mechanism.)*

[**Non-goals**]

- **NG-1: No new template file under `.ark/templates/`.** Template files there are reserved for feature-SPEC artifacts. **`LAYOUT.md` and other reference documents under `specs/project/` are permitted.** LAYOUT.md itself defines the boundary: "A reference document describes a convention or format. A template contains placeholder sections intended to be copied and filled in."

- **NG-2: No new functionality in `crates/`.**

- **NG-3: No edit to `project/INDEX.md` by the agent.**

- **NG-4: No edit to feature SPECs under `specs/features/`.**

- **NG-5: No additional convention SPECs in this task.**

- **NG-6: No public API change.** CLI `--help` text and argument names are unchanged (V-IT-6).

- **NG-7: No editorial deletion in COMMENTS.md migration.**

[**Architecture**]

```text
.ark/specs/project/
├── INDEX.md                            ← user-edited only (NG-3)
├── LAYOUT.md                           ← NEW: defines Layout A; defines reference-vs-template
├── rust/
│   ├── COMMENTS.md   ← Layout A; prefix C-N; 1:1 migration
│   ├── STYLE.md      ← Layout A; prefix S-N; from official sources
│   └── ERRORS.md     ← Layout A; prefix E-N; new

crates/ (refactor target):
├── ark-core/                                    (thiserror)
│   ├── src/error.rs              (354 LOC; minor reshape per E-rules incl. E-15)
│   ├── src/io/fs.rs              (1035 LOC → split)
│   ├── src/commands/upgrade.rs   (1124 LOC → split)
│   └── ... (other files: format / comment / naming only)
└── ark-cli/                                     (thiserror; anyhow permitted at the binary boundary)
    ├── src/main.rs               (1093 LOC → split with --help diff guard, F-6)
    └── tests/                    (format-only)
```

[**Data Structure**]

`LAYOUT.md` records the formal grammar:

```text
ConventionSpec := Purpose Rules Exceptions Examples? SeeAlso?
Purpose        := "[**Purpose**]" Paragraph (last sentence: link to LAYOUT.md)
Rules          := "[**Rules**]" Rule+
Rule           := "- **<PREFIX>-<N>: <Title>.**" Statement Rationale? Citation
Exceptions     := "[**Exceptions**]" Carveout*
Examples       := "[**Examples**]" CodeBlock+
SeeAlso        := "[**See Also**]" CrossRef+
Citation       := text containing one of {"http", "RFC", "API Guidelines",
                  "Style Guide", "Rust Book", "thiserror", "anyhow", "std::error"}
PREFIX         := "C" | "S" | "E" | "T" | "M" | …    # one per file

ReferenceDocument vs. Template:
- ReferenceDocument: describes a convention or format.
- Template: contains placeholder sections intended to be copied and filled in.
```

[**API Surface**]

Three convention documents and one reference document. No code API surface change. CLI surface preserved (V-IT-6).

[**Constraints**]

- **CN-1: Every rule in every convention SPEC cites a source.** Citation must contain at least one of: `http`, `RFC`, `API Guidelines`, `Style Guide`, `Rust Book`, `thiserror`, `anyhow`, `std::error`.

- **CN-2: 1:1 rule preservation in COMMENTS.md migration.** ≥23 entries under `[**Rules**]` and ≥5 entries under `[**Exceptions**]`. Mapping table in commit 1's commit body.

- **CN-3: Each commit is independently green.**

- **CN-4: No silent SPEC drift.** *Validated by V-F-4 (revised this iteration per R-101).*

- **CN-5: STYLE.md preserves every rule from the discarded draft that has an authoritative source.** Anchored by the rule-source mapping table in commit 1's body.

- **CN-6: ERRORS.md does not contradict global Rust style rules.** `thiserror` for `ark-core`; `anyhow` permitted in `ark-cli`; `?` for propagation; no `unwrap()` in production.

- **CN-7: Source files contain no task-mark process annotations.** Grep over `crates/**/*.rs` for `\b[CSE]-[0-9]+\b` and `\bV-[A-Z]+-[0-9]+\b` returns zero matches **unless** the match appears in a `// CITATION:` line, a doc-comment citing a SPEC rule, or a string literal that names a SPEC rule for a legitimate reason.

- **CN-8: No file in `crates/` exceeds 800 LOC after refactor.** Soft target: post-split files >400 LOC require a one-line explanation in commit 2's commit body.

## Runtime `Document and refactor flow`

[**Main Flow**]

1. **Phase 1 — Author `LAYOUT.md`.** ~15 lines: section grammar, prefix-per-file rule, citation format, **plus the reference-vs-template definition (per R-103)**.
2. **Phase 2 — Migrate COMMENTS.md.**
3. **Phase 3 — Author STYLE.md.**
4. **Phase 4 — Author ERRORS.md** including E-15 with the simplified wording (no exception clause; per R-102).
5. **Phase 5 — Commit 1.**
6. **Phase 6 — Refactor `crates/`** in sub-passes 6a..6g.
7. **Phase 7 — Validation gate.**
8. **Phase 8 — Commit 2.**
9. **Phase 9 — Verify.**

[**Failure Flow**]

1. **F-1: Migrated rule cannot fit Layout A.** Halt; expand LAYOUT.md schema.
2. **F-2: Source cannot be cited for a STYLE rule.** Drop the rule (CN-5).
3. **F-3: ERRORS.md or STYLE.md rule conflicts with `crates/` reality during Phase 6.** *Preferred mechanism (per R-101 clarification):* relax the rule by adding an entry under the SPEC's `[**Exceptions**]`, or scope the conflict to a follow-up FU-N at VERIFY. *Secondary mechanism (when amendment is genuinely required):* see V-F-4 for the commit-fixup path.
4. **F-4: Phase-7 gate fails.** Roll back failing change(s); re-attempt; if rule un-enforceable, demote to soft.
5. **F-5: Splitting introduces circular module deps.** Choose a different split axis.
6. **F-6: `clap` derive breakage during Phase 6g.** Pre-Phase-6g captures `--help` for every subcommand. Post-split diff. Any drift → roll back the split for that subcommand; try a different decomposition.

[**State Transitions**]

Standard deep-tier path.

## Implementation `Phases mapped to commits`

[**Phase 1 — `LAYOUT.md` (commit 1)**]

- Author `.ark/specs/project/LAYOUT.md`. Sections, in order:
  1. **Purpose** — what Layout A is, who must follow it.
  2. **Section grammar** — the EBNF block from `## Spec → Data Structure` above.
  3. **Citation requirement** — the recognized-token list (CN-1).
  4. **Reference document vs. template** — the definition sentence from R-103: "A reference document describes a convention or format. A template contains placeholder sections intended to be copied and filled in."
  5. **Pointer** — "See `INDEX.md` for the registered list of project SPECs."

[**Phase 2 — COMMENTS.md migration (commit 1)**]

- Translate every G/NG/C rule under prefix `C-N`. NG-* → `[**Exceptions**]`; G-*+C-* → `[**Rules**]`.
- Mapping table (old IDs → new IDs) into commit 1's body.

[**Phase 3 — STYLE.md authoring (commit 1)**]

- Source from cached + re-fetched material.
- Rule-source mapping into commit 1's body.

[**Phase 4 — ERRORS.md authoring (commit 1)**]

- **Pre-step (per TR-3):** fetch fresh `https://doc.rust-lang.org/std/error/trait.Error.html`, `https://docs.rs/thiserror/latest/thiserror/`, `https://docs.rs/anyhow/latest/anyhow/`. Re-fetch Rust Book ch9 and RFC 2504 if not cached.
- Author E-1..E-15 (15 rules total).
  - E-1..E-14 as outlined in PLAN 00 (unchanged).
  - **E-15 (final wording per R-102):** "When wrapping a foreign error, the variant must include at least one context field identifying the resource or operation (e.g. `path: PathBuf`, `command: String`)." Cite [API Guidelines C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err). No exception clause; if a rare single-purpose error enum needs the exception, the executor adds it under ERRORS.md's `[**Exceptions**]` only when such a case is encountered.
  - **E-2 / E-3 (per TR-6):** name `ark-core` (uses `thiserror`) and `ark-cli` (may use `anyhow` at the binary boundary) directly.

[**Phase 5 — Commit 1 (`feat(specs): add project-spec`)**]

- Stage: `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`.
- Commit body: COMMENTS migration mapping + STYLE source mapping.
- Verify: `cargo check --all-targets --workspace` green.

[**Phase 6 — `crates/` refactor (commit 2)**]

Sub-passes 6a..6g (unchanged from PLAN 01):

- **6a:** `cargo fmt --all`.
- **6b:** `cargo clippy --fix` + manual review.
- **6c:** Naming pass.
- **6d:** Comment-formatting pass.
- **6e:** Task-mark-tag pass.
- **6f:** Error-handling pass per ERRORS.md (including E-15 audit).
- **6g:** Structural split. Pre/post `--help` diff guard for `main.rs` (F-6). Soft 400-LOC target.

[**Phase 7 — Validation gate**]

- `cargo check --all-targets --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --workspace -- -D warnings`
- `cargo test --all-targets --workspace`
- CLI `--help` diff zero (V-IT-6).

[**Phase 8 — Commit 2**]

- `git commit -m "refactor(crates): comply with project specs"`. Body lists rename-driven public-API changes and any post-split file >400 LOC.

[**Phase 9 — Verify**]

- `ark agent task verify`. VERIFY.md surfaces proposed `INDEX.md` rows for the user.

## Trade-offs `ask reviewer for advice`

- **T-2: Single ERRORS.md vs. ERRORS.md + ERROR-MESSAGES.md.** Default upheld (single file). Reviewer reaffirmed across two iterations. Kept here as documented reasoning for future SPEC authors.

*(All other trade-offs resolved in iterations 00–01.)*

## Validation `test design`

[**Unit Tests**]

- **V-UT-1: Rule-count preservation.** ≥23 entries under `[**Rules**]` and ≥5 entries under `[**Exceptions**]`. Maps to G-2, CN-2.
- **V-UT-2: Citation presence with recognized source token.** Maps to G-3, G-4, CN-1.
- **V-UT-3: ERRORS.md rule coverage matrix.** Each E-1..E-15 maps to ≥1 cited source. Maps to G-4.

[**Integration Tests**]

- **V-IT-1: `cargo check` after commit 1.** Maps to G-8, CN-3.
- **V-IT-2: `cargo check` after commit 2.** Maps to G-8, CN-3.
- **V-IT-3: `cargo fmt --check` after commit 2.** Maps to G-5.
- **V-IT-4: `cargo clippy -D warnings` after commit 2.** Maps to G-5.
- **V-IT-5: `cargo test` after commit 2.** Maps to G-8, NG-2.
- **V-IT-6: CLI `--help` diff is empty.** Maps to NG-6, F-6.

[**Failure / Robustness Validation**]

- **V-F-1: Scoped commit verification.** Commit 1 touches only `.ark/specs/project/`; commit 2 touches only `crates/`. *Tolerance:* a fixup commit between commit 1 and commit 2 is permitted under V-F-4 and counts as part of the "specs-only" group for V-F-1's purposes. Maps to G-8.
- **V-F-2: No `unwrap()` in non-test code.** Maps to G-6, E-7, E-8.
- **V-F-3: No task-mark process annotations in `crates/`.** Maps to G-5, CN-7.
- **V-F-4 (revised per R-101): SPEC amendments precede commit 2.** If `.ark/specs/project/rust/ERRORS.md` or `STYLE.md` is modified during Phase 6, the modification must land in a commit that precedes commit 2. Acceptable mechanisms:
  - **(a) Amend commit 1** if it is HEAD and unpushed (`git commit --amend`).
  - **(b) Create an adjacent fixup commit** between commit 1 and commit 2 that touches only `.ark/specs/project/` (commit message: `feat(specs): amend project-spec for <reason>`).
  Verifier check: `git log --oneline -- .ark/specs/project/rust/` shows the change in a commit that precedes commit 2's hash. Maps to CN-4. *(Note: this is the secondary mechanism per F-3; the preferred mechanism is to relax the rule via `[**Exceptions**]` inside the SPEC body and let the change land in commit 1 normally.)*

[**Edge Case Validation**]

- **V-E-1: 800-LOC hard cap; 400-LOC soft target.** Maps to G-7, CN-8.
- **V-E-2: No `get_*` getters in production code.** Maps to G-5.
- **V-E-3: SPEC self-conformance.** All three SPECs use Layout A. Maps to G-1, G-3, G-4.

[**Acceptance Mapping**]

| Goal / Constraint | Validation                                                 |
| ----------------- | ---------------------------------------------------------- |
| G-1               | V-E-3                                                      |
| G-2               | V-UT-1, V-UT-2                                             |
| G-3               | V-UT-2, V-E-3                                              |
| G-4               | V-UT-2, V-UT-3, V-E-3                                      |
| G-5               | V-IT-3, V-IT-4, V-E-2, V-F-3                               |
| G-6               | V-F-2 (E-15 covered transitively via V-UT-3)               |
| G-7               | V-E-1                                                      |
| G-8               | V-IT-1, V-IT-2, V-IT-5, V-F-1                              |
| CN-1              | V-UT-2                                                     |
| CN-2              | V-UT-1                                                     |
| CN-3              | V-IT-1, V-IT-2                                             |
| CN-4              | V-F-4                                                      |
| CN-5              | V-UT-2 + commit-1-body mapping table (Phase 3 deliverable) |
| CN-6              | V-UT-3 + reviewer judgment                                 |
| CN-7              | V-F-3                                                      |
| CN-8              | V-E-1                                                      |
| NG-6              | V-IT-6                                                     |

Every Goal G-1..G-8 has at least one executable Validation. CN-4 has V-F-4. CN-5 anchored by the commit-body mapping table.
