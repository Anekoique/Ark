# `project-spec` PLAN `01`

> Status: Draft
> Feature: `project-spec`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Revise PLAN 00 to address the two HIGH findings (R-001, R-002), three MEDIUM findings (R-003, R-004, R-005), and two LOW findings (R-006, R-007), plus reviewer trade-off advice TR-1 (switch to a `LAYOUT.md` reference document), TR-3 (force fresh source fetches in Phase 4), and TR-6 (name `ark-core` and `ark-cli` directly in error-handling rules). All seven findings accepted; no rejections.

## Log `Iteration 01 deltas`

[**Added**]

- New SPEC file: `.ark/specs/project/LAYOUT.md` (per TR-1). Defines Layout A in one place; SPECs link to it.
- Rule **E-15** in ERRORS.md (per R-002): mandates context fields when wrapping foreign errors.
- Failure-flow item **F-6** (per R-004): rolls back Phase 5g `main.rs` split if `clap` help text changes.
- Validation **V-F-4** (per R-003): confirms ERRORS.md amendments land in commit 1, not commit 2.
- Validation **V-IT-6** (per R-004): CLI `--help` output unchanged after Phase 5g.

[**Changed**]

- **CN-2** (per R-001): rule-count claim now section-scoped — "≥23 entries under `[**Rules**]`, ≥5 entries under `[**Exceptions**]`" instead of "≥28 rule entries."
- **V-UT-1** (per R-001): same section-scoped reframing.
- **V-UT-2** (per R-005): citation regex tightened to require a recognized source token (`http`, `RFC`, `API Guidelines`, `Style Guide`, `Rust Book`, `thiserror`, `anyhow`, `std::error`).
- **CN-7** (per R-006): clarifies that SPEC rule IDs in `// CITATION:` or doc-comment lines are permitted; only process-annotation use is forbidden.
- **CN-8 / G-7 / Phase 5g** (per R-007): adds a soft 400-LOC target with a commit-body explanation requirement when exceeded.
- **NG-1** (per TR-1): no longer prohibits `LAYOUT.md`; it now allows a `LAYOUT.md` reference document under `specs/project/` while still excluding template files under `.ark/templates/`.
- **E-2 / E-3** (per TR-6): name `ark-core` and `ark-cli` directly.
- **Phase 4** (per TR-3): adds an explicit fetch step for `std::error::Error`, `thiserror`, `anyhow` regardless of cache.
- **Phase 5g** (per R-004): adds a `--help` baseline capture and post-split diff.
- **Phase 3** (per R-003): adds a mapping table requirement (rule-from-source → S-N identifier) recorded in commit 1's commit body.

[**Removed**]

- T-1 from `## Trade-offs`: resolved (Option B selected per TR-1). Removed.
- T-3 from `## Trade-offs`: resolved (default upheld with addition; folded into Phase 4 procedure). Removed.
- T-4 from `## Trade-offs`: resolved (R-004 incorporated). Removed.
- T-5 from `## Trade-offs`: resolved (default upheld). Removed.
- T-6 from `## Trade-offs`: resolved (default upheld with naming clarification per TR-6). Removed.
- Only T-2 remains open (default upheld; reviewer reaffirmed; kept as a documented trade-off for future SPEC authors).

[**Unresolved**]

- None. All R-001..R-007 incorporated; all TR-1..TR-6 resolved or default upheld.

[**Response Matrix**]

| Source | ID    | Decision | Resolution                                                                                  |
| ------ | ----- | -------- | ------------------------------------------------------------------------------------------- |
| Review | R-001 | Accepted | CN-2 and V-UT-1 reframed as section-scoped counts (≥23 under Rules, ≥5 under Exceptions).   |
| Review | R-002 | Accepted | New rule E-15 added to ERRORS.md outline; covers context fields on wrapped foreign errors.  |
| Review | R-003 | Accepted | Phase 3 emits a rule-source mapping in commit 1 body; new V-F-4 confirms commit-1 location. |
| Review | R-004 | Accepted | New F-6 + Phase-5g pre/post `--help` diff guards `clap` derive coupling.                    |
| Review | R-005 | Accepted | V-UT-2 regex tightened to require a recognized source token.                                |
| Review | R-006 | Accepted | CN-7 clarified: citations permitted in `// CITATION:` and doc-comment contexts.             |
| Review | R-007 | Accepted | Phase 5g adds soft 400-LOC target with commit-body justification when exceeded.             |
| Review | TR-1  | Accepted | Switched to Option B: `specs/project/LAYOUT.md` reference document. NG-1 updated.           |
| Review | TR-2  | Default  | Single ERRORS.md retained.                                                                  |
| Review | TR-3  | Accepted | Phase 4 explicitly fetches `std::error::Error`, `thiserror`, `anyhow`.                      |
| Review | TR-4  | Accepted | Folded into R-004 acceptance.                                                               |
| Review | TR-5  | Default  | VERIFY.md will surface proposed INDEX.md row text.                                          |
| Review | TR-6  | Accepted | E-2 and E-3 name `ark-core` and `ark-cli` directly.                                         |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning. *(None this iteration — all accepted.)*

---

## Spec `Convention-SPEC layout, plus three convention SPECs`

[**Goals**]

- **G-1: Convention-SPEC layout (Layout A) is defined once and applied uniformly.** Layout A's section structure is `[**Purpose**] / [**Rules**] / [**Exceptions**] / [**Examples**] / [**See Also**]`, with a flat numbered list under `[**Rules**]` and a single rule prefix per file. The layout is documented authoritatively in a new file `.ark/specs/project/LAYOUT.md`; convention SPECs link to it from their `[**Purpose**]` paragraph.

- **G-2: COMMENTS.md migrated to Layout A with zero rule loss.** Every existing rule (G-1..G-12, NG-1..NG-5, C-1..C-11) is preserved verbatim or with mechanical relabeling, mapped onto the new section structure under prefix `C-N`. NG-* rules become items under `[**Exceptions**]`; G-* and C-* rules become items under `[**Rules**]`. Citations to RFC 0505, the rustdoc Book, and the API Guidelines are preserved.

- **G-3: STYLE.md authored in Layout A from authoritative sources.** Every rule cites at least one of: the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/), [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), or the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Single rule prefix `S-N`. Phase 3 emits a rule-source mapping (each S-N → its source page section) into commit 1's commit body for verifiability.

- **G-4: ERRORS.md authored in Layout A from authoritative sources.** Every rule cites at least one of: [The Rust Programming Language Book — Chapter 9 (Error Handling)](https://doc.rust-lang.org/book/ch09-00-error-handling.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (especially [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err) and [C-VALIDATE](https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate)), [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html), [`thiserror` crate documentation](https://docs.rs/thiserror/), [`anyhow` crate documentation](https://docs.rs/anyhow/), [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html). Single rule prefix `E-N`.

- **G-5: `crates/` mechanically compliant.** `cargo fmt --check` passes; `cargo clippy --all-targets --all-features -- -D warnings` passes; no `get_*` getters; casing per RFC 430; conversion-method semantics correct; comment formatting per COMMENTS.md.

- **G-6: `crates/` error-handling-compliant per ERRORS.md.** Canonical `Error` enum unchanged in identity but rule-conformant in shape; consistent `?` discipline; no `unwrap()` in non-test code; error messages follow phrasing rules (lowercase, no trailing punctuation, no "error:" prefix); `Result<T>` alias used at the boundary; `#[from]` and `#[source]` consistent; **wrapped-error variants carry context fields per E-15**.

- **G-7: `crates/` structurally cleaner.** Files exceeding the 800-line cap from `~/.claude/rules/common/coding-style.md` (`commands/upgrade.rs` 1124, `ark-cli/src/main.rs` 1093, `io/fs.rs` 1035) are decomposed. Module organization remains feature-grouped. **Soft target: each post-split file ≤400 LOC; deviations explained in commit 2's commit body.**

- **G-8: Two commits on `feat/project-spec`.** Commit 1 — `feat(specs): add project-spec` — touches only `.ark/specs/project/`. Commit 2 — `refactor(crates): comply with project specs` — touches only `crates/`. Each commit is independently `cargo check --all-targets` green.

[**Non-goals**]

- **NG-1: No new template file under `.ark/templates/`.** Template files there are reserved for feature-SPEC artifacts. *(Updated this iteration: `LAYOUT.md` under `specs/project/` is now permitted as a reference document — it is not a template.)*

- **NG-2: No new functionality in `crates/`.** No new commands, flags, or behavior. Only formatting, naming, error-handling, and module-organization changes.

- **NG-3: No edit to `project/INDEX.md` by the agent.** The verifier surfaces the proposed rows verbatim in VERIFY.md.

- **NG-4: No edit to feature SPECs under `specs/features/`.**

- **NG-5: No additional convention SPECs in this task.** `MODULES.md`, `TESTING.md`, `COMMITS.md` deferred.

- **NG-6: No public API change.** Crate-level public items keep their names and signatures unless they violate a rule. Where a rename is forced, document it in the commit message. CLI `--help` text and argument names are unchanged (validated by V-IT-6).

- **NG-7: No editorial deletion in COMMENTS.md migration.**

[**Architecture**]

```text
.ark/specs/project/
├── INDEX.md                            ← user-edited only (NG-3)
├── LAYOUT.md                           ← NEW: defines Layout A authoritatively (~15 lines)
├── rust/
│   ├── COMMENTS.md   ← Layout A; prefix C-N; 1:1 migration of existing rules
│   ├── STYLE.md      ← Layout A; prefix S-N; authored from official sources
│   └── ERRORS.md     ← Layout A; prefix E-N; new

Layout A (defined in LAYOUT.md, applied to every convention SPEC):
  [**Purpose**]      One paragraph; ends with "Layout: see specs/project/LAYOUT.md."
  [**Rules**]        Flat numbered list; one prefix per file (e.g. C-1 .. C-N).
                     Each rule: bold title, statement, optional rationale, REQUIRED citation.
  [**Exceptions**]   Carve-outs and explicit non-requirements.
  [**Examples**]     Worked code examples.
  [**See Also**]     Cross-refs to sibling SPECs.

crates/ (refactor target):
├── ark-core/
│   ├── src/error.rs             (354 LOC; minor reshape per E-rules including E-15)
│   ├── src/io/fs.rs             (1035 LOC → split, see Phase 5g)
│   ├── src/commands/upgrade.rs  (1124 LOC → split, see Phase 5g)
│   └── ... (other files: format / comment / naming only)
└── ark-cli/
    ├── src/main.rs              (1093 LOC → split with --help diff guard, see Phase 5g + F-6)
    └── tests/                   (format-only)
```

[**Data Structure**]

`LAYOUT.md` is a document structure description. Formal grammar of a convention SPEC (recorded in LAYOUT.md, summarized here):

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
```

The `crates/` data structure (the `Error` enum) is described inside ERRORS.md.

[**API Surface**]

The "API surface" of this task is three convention documents and one reference document:

- `specs/project/LAYOUT.md` — defines Layout A.
- `specs/project/rust/COMMENTS.md` — see [**Examples**] post-migration.
- `specs/project/rust/STYLE.md` — see [**Examples**].
- `specs/project/rust/ERRORS.md` — canonical `Error` definition + `thiserror` patterns + E-15 wrapped-error template.

No code API surface is changed. Rename-driven signature changes that escape `pub(crate)` are listed in commit 2's body. CLI surface is preserved (V-IT-6).

[**Constraints**]

- **CN-1: Every rule in every convention SPEC cites a source.** Citation must contain at least one of: `http`, `RFC`, `API Guidelines`, `Style Guide`, `Rust Book`, `thiserror`, `anyhow`, `std::error`.

- **CN-2: 1:1 rule preservation in COMMENTS.md migration.** *(Section-scoped per R-001):* the migrated COMMENTS.md must contain ≥23 entries under `[**Rules**]` (one per original G-* and C-*) and ≥5 entries under `[**Exceptions**]` (one per original NG-*). Mapping table is included in commit 1's commit body.

- **CN-3: Each commit is independently green.** `cargo check --all-targets --workspace` after commit 1; `cargo check`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` after commit 2.

- **CN-4: No silent SPEC drift.** If Phase 6 reveals a constraint a SPEC cannot accommodate, amend the SPEC and re-amend commit 1. *Validated by V-F-4.*

- **CN-5: STYLE.md preserves every rule from the discarded draft that has an authoritative source.** *Anchored by the rule-source mapping table emitted in commit 1's commit body (per R-003).*

- **CN-6: ERRORS.md does not contradict global Rust style rules.** `thiserror` for `ark-core`, `anyhow` permitted in `ark-cli`, `?` for propagation, no `unwrap()` in production. *(Updated per TR-6 to name crates directly.)*

- **CN-7: Source files contain no task-mark process annotations.** A grep over `crates/**/*.rs` for `\b[CSE]-[0-9]+\b` and `\bV-[A-Z]+-[0-9]+\b` returns zero matches **unless** the match appears in a `// CITATION:` line, a doc-comment citing a SPEC rule, or a string literal that names a SPEC rule for a legitimate reason. *(Per R-006.)*

- **CN-8: No file in `crates/` exceeds 800 LOC after refactor.** *Soft target (per R-007):* additionally, post-split files exceeding 400 LOC require a one-line explanation in commit 2's commit body.

## Runtime `Document and refactor flow`

[**Main Flow**]

1. **Phase 1 — Author `LAYOUT.md`.** ~15 lines defining Layout A.
2. **Phase 2 — Migrate COMMENTS.md.** Translate every G/NG/C rule under prefix `C-N`; produce mapping table.
3. **Phase 3 — Author STYLE.md.** Source from cached + re-fetched material; emit rule-source mapping for commit body.
4. **Phase 4 — Author ERRORS.md.** Fresh fetches per TR-3; cover E-1..E-15.
5. **Phase 5 — Commit 1.** Body includes both mapping tables.
6. **Phase 6 — Refactor `crates/`** in sub-passes 6a..6g.
7. **Phase 7 — Validation gate.** Cargo check, fmt, clippy, test, plus CLI `--help` diff (V-IT-6).
8. **Phase 8 — Commit 2.**
9. **Phase 9 — Verify.**

[**Failure Flow**]

1. **F-1: Migrated rule cannot fit Layout A.** Halt; expand LAYOUT.md schema.
2. **F-2: Source cannot be cited for a STYLE rule.** Drop the rule (CN-5).
3. **F-3: ERRORS.md rule conflicts with `crates/` reality unboundedly.** Relax via `[**Exceptions**]` or scope to a follow-up FU-N. Amendment must land in commit 1 (V-F-4).
4. **F-4: Phase-7 gate fails.** Roll back failing change(s); re-attempt; if rule un-enforceable, demote to soft.
5. **F-5: Splitting introduces circular module deps.** Choose a different split axis.
6. **F-6 (NEW per R-004): `clap` derive breakage.** Pre-Phase-5g captures `--help` for every subcommand. Post-split diff. Any drift → roll back the split for that subcommand and try a different decomposition (e.g. extract handler bodies into helpers within the same module).

[**State Transitions**]

- Standard deep-tier path: Design → Plan → Review → (loop) → Execute → Verify → Archive.

## Implementation `Phases mapped to commits`

[**Phase 1 — `LAYOUT.md` (commit 1)**]

- Author `.ark/specs/project/LAYOUT.md` (~15 lines): section grammar, prefix-per-file rule, citation format. Single source of truth.

[**Phase 2 — COMMENTS.md migration (commit 1)**]

- Read current COMMENTS.md (170 lines, 28 rules).
- Build mapping table: old IDs (G-1..G-12, NG-1..NG-5, C-1..C-11) → new IDs (C-1..C-23 + Exceptions list).
- Translate verbatim. NG-* → `[**Exceptions**]`; G-* and C-* → `[**Rules**]`.
- Preserve every citation.
- Mapping table goes into commit 1's commit body.

[**Phase 3 — STYLE.md authoring (commit 1)**]

- Use cached fetches from earlier in this conversation (Style Guide items / expressions / statements / naming / predictability, API Guidelines checklist).
- Author each S-N rule with citation. Cover formatting, naming, items, expressions, statements, imports, common-trait derivation, newtype/struct-private rules.
- Emit rule-source mapping (S-N → source page + section) into commit 1's commit body alongside the COMMENTS migration table.

[**Phase 4 — ERRORS.md authoring (commit 1)**]

- **Pre-step (per TR-3):** fetch fresh:
  - `https://doc.rust-lang.org/std/error/trait.Error.html`
  - `https://docs.rs/thiserror/latest/thiserror/`
  - `https://docs.rs/anyhow/latest/anyhow/`
- Re-fetch Rust Book chapter 9 and RFC 2504 if not in conversation cache.
- Author E-1..E-15:
  - E-1..E-14 as outlined in PLAN 00.
  - **E-15 (NEW per R-002):** "When wrapping a foreign error, the variant must include at least one context field identifying the resource or operation. A bare `#[from] source: io::Error` variant is permitted only when the variant's `Display` template includes a contextualizing field." Cite API Guidelines C-GOOD-ERR.
  - **E-2 / E-3 (per TR-6):** name `ark-core` (uses `thiserror`) and `ark-cli` (may use `anyhow`) directly.

[**Phase 5 — Commit 1 (`feat(specs): add project-spec`)**]

- Stage: `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`.
- Commit message body:
  - Mapping table 1: old COMMENTS IDs → new IDs (per CN-2).
  - Mapping table 2: STYLE S-N → source URLs (per CN-5).
- Verify: `cargo check --all-targets --workspace` green (no code touched).

[**Phase 6 — `crates/` refactor (commit 2)**]

Sub-passes, each followed by `cargo check`:

- **6a: `cargo fmt --all`.**
- **6b: `cargo clippy --fix --allow-dirty --allow-staged`** + manual review of remaining warnings.
- **6c: Naming pass.**
- **6d: Comment-formatting pass.**
- **6e: Task-mark-tag pass.**
- **6f: Error-handling pass per ERRORS.md** including E-15 audit.
- **6g: Structural split** of `commands/upgrade.rs`, `ark-cli/src/main.rs`, `io/fs.rs`.
  - **R-004 guard for `main.rs`:** capture `cargo run -p ark-cli -- --help` and every subcommand `--help` to a temp file before splitting. After each split, re-run and `diff`. Any change → roll back per F-6.
  - **R-007 soft target:** if any post-split file exceeds 400 LOC, add a one-line note in commit 2's body.

[**Phase 7 — Validation gate**]

- `cargo check --all-targets --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --workspace -- -D warnings`
- `cargo test --all-targets --workspace`
- CLI `--help` diff (V-IT-6) shows zero changes vs. pre-Phase-5g baseline.

[**Phase 8 — Commit 2**]

- `git commit -m "refactor(crates): comply with project specs"`. Body lists any rename-driven public-API changes and any post-split file >400 LOC.

[**Phase 9 — Verify**]

- `ark agent task verify`; fill VERIFY.md including: proposed `INDEX.md` rows for STYLE.md (updated) and ERRORS.md (new) for the user to apply manually.

## Trade-offs `ask reviewer for advice`

- **T-2: Single ERRORS.md vs. ERRORS.md + ERROR-MESSAGES.md.** Default upheld (single file). Reviewer reaffirmed. Kept here as documented reasoning for future SPEC authors weighing similar splits.
  - **adv. (single):** zero discoverability cost; rules cluster naturally.
  - **disadv. (single):** message-phrasing rules slightly buried.

*(T-1, T-3, T-4, T-5, T-6 resolved this iteration; see Log §Removed.)*

## Validation `test design`

[**Unit Tests**]

- **V-UT-1 (revised per R-001): Rule-count preservation in COMMENTS.md migration.** New COMMENTS.md contains ≥23 entries under `[**Rules**]` and ≥5 entries under `[**Exceptions**]`. Maps to G-2, CN-2.
- **V-UT-2 (revised per R-005): Citation presence with recognized source token.** Grep over each new SPEC: every line matching `^- \*\*[A-Z]-[0-9]+:` must end with a parenthesized citation containing one of `http`, `RFC`, `API Guidelines`, `Style Guide`, `Rust Book`, `thiserror`, `anyhow`, `std::error`. Maps to G-3, G-4, CN-1.
- **V-UT-3: ERRORS.md rule coverage matrix.** Each E-1..E-15 maps to ≥1 cited source from the approved list. Maps to G-4.

[**Integration Tests**]

- **V-IT-1: `cargo check --all-targets --workspace` passes after commit 1.** Maps to G-8, CN-3.
- **V-IT-2: `cargo check --all-targets --workspace` passes after commit 2.** Maps to G-8, CN-3.
- **V-IT-3: `cargo fmt --all -- --check` passes after commit 2.** Maps to G-5.
- **V-IT-4: `cargo clippy --all-targets --workspace -- -D warnings` passes after commit 2.** Maps to G-5.
- **V-IT-5: `cargo test --all-targets --workspace` passes after commit 2.** Maps to G-8, NG-2.
- **V-IT-6 (NEW per R-004): CLI `--help` output unchanged.** `diff` of pre-Phase-5g help capture vs. post-commit-2 help capture is empty. Maps to NG-6, F-6.

[**Failure / Robustness Validation**]

- **V-F-1: Scoped commit verification.** Commit 1 touches only `.ark/specs/project/`; commit 2 touches only `crates/`. Maps to G-8.
- **V-F-2: No `unwrap()` in non-test code.** `rg '\.unwrap\(\)' crates/ -g '!**/tests/**' -g '!**/*test*.rs'` returns zero non-test matches (or every remaining occurrence is justified by a `// SAFETY:` comment). Maps to G-6, E-7, E-8.
- **V-F-3 (revised per R-006): No task-mark process annotations in `crates/`.** `rg '\b[CSE]-[0-9]+\b' crates/ -g '*.rs'` excluding `// CITATION:` lines and doc-comment citations returns zero matches. Maps to G-5, CN-7.
- **V-F-4 (NEW per R-003): SPEC amendments land in commit 1.** If `.ark/specs/project/rust/ERRORS.md` or `STYLE.md` was modified during Phase 6 (per F-3), `git log --oneline -- .ark/specs/project/rust/` shows that modification in commit 1's history (e.g. via `git commit --amend`), not commit 2. Maps to CN-4.

[**Edge Case Validation**]

- **V-E-1 (revised per R-007): 800-LOC hard cap and 400-LOC soft target.** `find crates -name '*.rs' -not -path '*/tests/*' | xargs wc -l | awk '$1 > 800'` returns no rows. Files where 400 < LOC ≤ 800 are explained in commit 2's body. Maps to G-7, CN-8.
- **V-E-2: No `get_*` getters in production code.** `rg 'pub fn get_[a-z_]+\(' crates/` returns zero matches. Maps to G-5.
- **V-E-3: SPEC self-conformance.** All three SPECs use Layout A as defined in `LAYOUT.md`; visual section-header check confirms the five required headers in order. Maps to G-1, G-3, G-4.

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

Every Goal G-1..G-8 has at least one executable Validation. CN-4 now has V-F-4. CN-5 is anchored by the commit-body mapping table — a deliverable, not just process. NG-6 (no public CLI surface change) is anchored by V-IT-6.
