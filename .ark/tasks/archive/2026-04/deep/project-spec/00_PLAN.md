# `project-spec` PLAN `00`

> Status: Draft
> Feature: `project-spec`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Introduce a purpose-fit "convention SPEC" layout (Layout A: `Purpose / Rules / Exceptions / Examples / See Also`) for project-level coding-convention SPECs, migrate `rust/COMMENTS.md` and `rust/STYLE.md` onto it 1:1 (no rule loss), add a new `rust/ERRORS.md` whose every rule cites an authoritative source, then refactor `crates/` (~16k LOC across `ark-core` + `ark-cli`) to comply with all three SPECs. Ship as two commits on `feat/project-spec`: `feat(specs): add project-spec` (specs only) and `refactor(crates): comply with project specs` (crates only).

## Log `None in 00_PLAN`

[**Added**] — *first iteration, no prior log*

[**Changed**] — *first iteration*

[**Removed**] — *first iteration*

[**Unresolved**] — *first iteration*

[**Response Matrix**] — *first iteration, no prior findings*

---

## Spec `Convention-SPEC layout, plus three convention SPECs`

[**Goals**]

- **G-1: Convention-SPEC layout (Layout A) is defined and applied.** Section structure: `[**Purpose**] / [**Rules**] / [**Exceptions**] / [**Examples**] / [**See Also**]`. A flat numbered list under `[**Rules**]`; one rule prefix per file. No `Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints` headings on convention SPECs.

- **G-2: COMMENTS.md migrated to Layout A with zero rule loss.** Every existing rule (G-1..G-12, NG-1..NG-5, C-1..C-11) is preserved verbatim or with mechanical relabeling, mapped onto the new section structure under prefix `C-N`. Citations to RFC 0505, the rustdoc Book, and the API Guidelines are kept.

- **G-3: STYLE.md (re)written in Layout A from authoritative sources.** Every rule cites at least one of: the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/), [RFC 430](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), or the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Single rule prefix `S-N`. The previous (uncommitted, discarded) draft serves as a content baseline for re-authoring under Layout A — no project-internal patterns.

- **G-4: ERRORS.md authored in Layout A from authoritative sources.** Every rule cites at least one of: [The Rust Programming Language Book — Chapter 9 (Error Handling)](https://doc.rust-lang.org/book/ch09-00-error-handling.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (especially [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err) and [C-VALIDATE](https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate)), [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html), [`thiserror` crate documentation](https://docs.rs/thiserror/), [`anyhow` crate documentation](https://docs.rs/anyhow/), [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html). Single rule prefix `E-N`.

- **G-5: `crates/` mechanically compliant.** `cargo fmt --check` passes; `cargo clippy --all-targets --all-features -- -D warnings` passes; no `get_*` getters; casing per RFC 430; conversion-method semantics correct; comment formatting per COMMENTS.md (third-person, sections, no task-mark tags).

- **G-6: `crates/` error-handling-compliant per ERRORS.md.** Canonical `Error` enum unchanged in identity but rule-conformant in shape; consistent `?` discipline; no `unwrap()` in non-test code (per user's global Rust style rules); error messages follow phrasing rules (lowercase, no trailing punctuation, no "error:" prefix); `Result<T>` alias used at the boundary; `#[from]` and `#[source]` used per ERRORS.md guidance.

- **G-7: `crates/` structurally cleaner.** Files exceeding the 800-line cap from `~/.claude/rules/common/coding-style.md` (`upgrade.rs` 1124, `main.rs` 1093, `fs.rs` 1035) are decomposed. Module organization remains feature-grouped (no by-type reshuffling).

- **G-8: Two commits on `feat/project-spec`.** Commit 1 — `feat(specs): add project-spec` — touches only `.ark/specs/project/`. Commit 2 — `refactor(crates): comply with project specs` — touches only `crates/`. Each commit's tree is independently buildable (`cargo check` green at each commit).

[**Non-goals**]

- **NG-1: No new feature SPEC, no new convention-SPEC template file.** The `.ark/templates/` directory is for feature-SPEC artifacts; the convention-SPEC layout lives in the documents themselves and is described informally in `project/INDEX.md` only by the user.

- **NG-2: No new functionality in `crates/`.** No new commands, flags, or behavior. Only formatting, naming, error-handling, and module-organization changes.

- **NG-3: No edit to `project/INDEX.md`.** Per its own rule (`Who editing only edited and modified by user`), the agent does not append the ERRORS.md row or update the STYLE.md row. The verifier surfaces the proposed rows; the user applies them.

- **NG-4: No edit to feature SPECs under `specs/features/`.** Their content stays under the feature-SPEC template; this task only governs `specs/project/`.

- **NG-5: No additional convention SPECs in this task.** `MODULES.md`, `TESTING.md`, `COMMITS.md` are deferred. Their absence is not a blocker.

- **NG-6: No public API change.** Crate-level public items in `ark-core` keep their names and signatures unless they violate a rule (e.g. a `get_foo` getter must be renamed). Where a rename is forced, document it in the commit message.

- **NG-7: No editorial deletion in COMMENTS.md migration.** Even if a rule looks redundant after migration, it stays. Re-numbering and section reassignment only.

[**Architecture**]

```text
.ark/specs/project/
├── INDEX.md                            ← user-edited only (NG-3)
├── rust/
│   ├── COMMENTS.md   ← Layout A; prefix C-N; 1:1 migration of existing rules
│   ├── STYLE.md      ← Layout A; prefix S-N; authored from official Rust Style Guide / RFC 430 / API Guidelines
│   └── ERRORS.md     ← Layout A; prefix E-N; new; cites Rust Book ch9 + API Guidelines + thiserror/anyhow + RFC 2504

Layout A (every convention SPEC):
  [**Purpose**]      One paragraph: what this SPEC governs + authoritative sources cited.
  [**Rules**]        Flat numbered list; one prefix per file (e.g. C-1 .. C-N).
                     Each rule: bold title, statement, optional rationale, REQUIRED citation.
  [**Exceptions**]   Carve-outs and explicit non-requirements. Replaces "Non-goals" semantically.
  [**Examples**]     Worked code examples. Replaces "API Surface" semantically.
  [**See Also**]     Cross-refs to sibling SPECs.

crates/ (refactor target):
├── ark-core/
│   ├── src/error.rs           (354 LOC; minor reshape per E-rules)
│   ├── src/io/fs.rs           (1035 LOC → split, see Phase 5)
│   ├── src/commands/upgrade.rs (1124 LOC → split, see Phase 5)
│   └── ... (other files: format-only, comment-only, naming-only)
└── ark-cli/
    ├── src/main.rs            (1093 LOC → split into subcommand modules, see Phase 5)
    └── tests/                 (format-only)
```

[**Data Structure**]

Layout A is a document structure, not a runtime data structure. Formal grammar of a convention SPEC:

```text
ConventionSpec := Purpose Rules Exceptions Examples? SeeAlso?
Purpose        := "[**Purpose**]" Paragraph
Rules          := "[**Rules**]" Rule+
Rule           := "- **<PREFIX>-<N>: <Title>.**" Statement Rationale? Citation
Exceptions     := "[**Exceptions**]" Carveout*
Examples       := "[**Examples**]" CodeBlock+
SeeAlso        := "[**See Also**]" CrossRef+
Citation       := "(<SourceName>, <Section> | <URL>.)"
PREFIX         := "C" | "S" | "E" | "T" | "M" | …    # one per file
```

The `crates/` data structures (the `Error` enum) are described in the ERRORS.md SPEC itself, not duplicated here.

[**API Surface**]

The "API surface" of this task is the set of conventions exposed to future contributors and agents. Three documents:

- `rust/COMMENTS.md` — see [**Examples**] in COMMENTS.md (post-migration). No prefix change in semantics; prefix collapses from `G-/NG-/C-` to single `C-`.
- `rust/STYLE.md` — see [**Examples**] in STYLE.md.
- `rust/ERRORS.md` — see [**Examples**] in ERRORS.md (canonical `Error` definition + `thiserror` patterns).

No code API surface is changed by this task. (A handful of rename-driven signature renames may surface during Phase 5; if any escape `pub(crate)`, list them in the Phase 5 commit body.)

[**Constraints**]

- **CN-1: Every rule in every convention SPEC cites a source.** No bare assertions. Citation format: `(<Source name or canonical short title>, <Section or URL>.)` placed at the end of the rule body.

- **CN-2: 1:1 rule preservation in COMMENTS.md migration.** The migrated COMMENTS.md must contain at least 28 rules (12 G + 5 NG + 11 C from the current file) — relabeled and possibly reordered, but each prior rule must map onto exactly one new rule. The PLAN's Validation section maps each old-prefix rule to its new identifier.

- **CN-3: Each commit is independently green.** `cargo check --all-targets` after commit 1 (specs only) must succeed. `cargo check --all-targets`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` after commit 2 must succeed.

- **CN-4: No silent SPEC drift.** If Phase 5 reveals a constraint the new ERRORS.md cannot accommodate, update ERRORS.md and re-amend commit 1 — do not silently violate the SPEC.

- **CN-5: Every existing rule in STYLE.md's discarded draft that came from an authoritative source remains in the new STYLE.md.** Project-internal conventions inferred from the codebase (none should exist after re-authoring) are dropped.

- **CN-6: ERRORS.md does not contradict global Rust style rules** (`~/.claude/rules/rust/coding-style.md`): `thiserror` for libraries, `anyhow` for applications, `?` for propagation, no `unwrap()` in production. These global rules become rules inside ERRORS.md with citations.

- **CN-7: COMMENTS.md C-11 (no task-mark tags in source) is enforced post-refactor.** A grep over `crates/**/*.rs` for `\b[A-Z]+-[0-9]+\b` patterns matching SPEC tags (e.g. `G-1`, `C-2`, `V-UT-1`, `E-3`) returns zero matches. False positives (e.g. `HTTP-1`, `IPV6-2026`) tolerated; the verifier judgmental.

- **CN-8: No file in `crates/` exceeds 800 LOC after refactor.** Aligns with `~/.claude/rules/common/coding-style.md`'s 800-line max (currently violated by 3 files).

## Runtime `Document and refactor flow`

[**Main Flow**]

1. **Phase 1 — Define Layout A.** Author the layout shape inside the new STYLE.md and COMMENTS.md headers (no separate template file per NG-1). Ensure both files demonstrate it identically.
2. **Phase 2 — Migrate COMMENTS.md.** Translate every G-/NG-/C- rule onto Layout A under prefix `C-N`. Verify rule-count preservation (CN-2).
3. **Phase 3 — Author STYLE.md.** Source from Rust Style Guide / RFC 430 / API Guidelines. Cite every rule. Cover formatting, naming, items, expressions, statements, imports, common-trait derivation, newtype/struct-private rules.
4. **Phase 4 — Author ERRORS.md.** Source from Rust Book ch9 / API Guidelines / `std::error::Error` / `thiserror` / `anyhow` / RFC 2504. Cite every rule. Cover `Error` enum design, `Result<T>` alias, `?` propagation, `#[from]` / `#[source]`, error message phrasing, panic vs. error policy, `unwrap`/`expect`/`unreachable!` discipline.
5. **Phase 5 — Commit 1.** `git commit -m "feat(specs): add project-spec"` containing the three SPEC files. Do not touch `INDEX.md` (NG-3).
6. **Phase 6 — Refactor `crates/`.** In order: (6a) `cargo fmt`; (6b) clippy fixes with `-D warnings`; (6c) naming-rule pass (rename violators); (6d) comment-formatting pass; (6e) error-handling pass per ERRORS.md; (6f) structural split of files >800 LOC.
7. **Phase 7 — Validate.** Run the gate (`cargo check`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`). Resolve any failures by amending Phase 6 (not by relaxing the SPEC).
8. **Phase 8 — Commit 2.** `git commit -m "refactor(crates): comply with project specs"`.
9. **Phase 9 — Verify.** `ark agent task verify` and write VERIFY.md.

[**Failure Flow**]

1. **F-1: A migrated rule cannot be expressed under Layout A.** Halt; expand Layout A's `[**Rules**]` schema (rare). Re-iterate plan.
2. **F-2: Source cannot be cited for a Phase-3 STYLE rule.** Drop the rule. Project-internal patterns are not SPEC material (CN-5).
3. **F-3: ERRORS.md rule conflicts with `crates/` reality and refactor cost is unbounded.** Two options: relax the rule with explicit rationale in `[**Exceptions**]`, or scope the conflict out to a follow-up task (record in VERIFY.md as a follow-up FU-N).
4. **F-4: Phase-7 gate fails after Phase 6.** Roll back the failing change(s); re-attempt; if a rule turns out to be impossible to enforce mechanically, demote to a soft rule with explicit `[**Exceptions**]`.
5. **F-5: Splitting a file >800 LOC introduces a circular module dependency.** Choose a different split axis (extract a leaf type/trait, not a top-level command). If no split is possible without introducing significant complexity, document the exception in `[**Exceptions**]` of STYLE.md.

[**State Transitions**]

- Design → Plan when PRD is filled and `ark agent task plan` runs.
- Plan → Review when this PLAN is filled and `ark agent task review` runs.
- Review → Plan when verdict is *Rejected* or *Approved with Revisions*; iteration bumps.
- Review → Execute when verdict is *Approved* with zero open CRITICAL.
- Execute → Verify after both commits land and `ark agent task verify` runs.
- Verify → Archive when the user runs `/ark:archive` (out of this PLAN's scope).

## Implementation `Phases mapped to commits`

[**Phase 1 — Layout A definition (commit 1)**]

- Define the section header pattern and rule-line grammar inline in COMMENTS.md and STYLE.md headers.
- The first three lines under each new SPEC's `[**Purpose**]` paragraph also act as a brief layout legend (one sentence per slot is enough).

[**Phase 2 — COMMENTS.md migration (commit 1)**]

- Read current COMMENTS.md (170 lines).
- Build a mapping table from old IDs (G-1..G-12, NG-1..NG-5, C-1..C-11) to new IDs (C-1..C-N) — preserve order where it makes sense.
- Translate each rule body verbatim; merge the leading paragraph and "Architecture / Data Structure / API Surface / Constraints" sections into the new layout: leading paragraph → `[**Purpose**]`; G/NG/C rules → `[**Rules**]` (NG-* rules become items in `[**Exceptions**]`); ASCII tree + worked examples → `[**Examples**]`; cross-ref to STYLE.md/ERRORS.md → `[**See Also**]`.
- Preserve every citation (RFC 0505, rustdoc Book).
- Verify rule count: 12 G + 5 NG + 11 C = 28 rules in total — must appear under the new prefix exactly once.

[**Phase 3 — STYLE.md authoring (commit 1)**]

- Author from scratch under Layout A using the previously-fetched material (Rust Style Guide / RFC 430 / API Guidelines) as source. Re-fetch any section the previous draft compressed too aggressively.
- Cover, in order: indentation, line width, blank lines, trailing whitespace, trailing commas, block-vs-visual indent, comments-vs-code spacing, attributes, casing (RFC 430), conversion-method prefixes (C-CONV), getter conventions (C-GETTER), iterator triple (C-ITER), constructors (C-CTOR), imports, item ordering, operator spacing, control-flow parens, `match` arms, ranges, hex literal case, extern ABI, let-else, public-struct privacy (C-STRUCT-PRIVATE), common-trait derivation (C-COMMON-TRAITS), newtype rule (C-NEWTYPE), no out-parameters (C-NO-OUT), `Deref` only on smart pointers (C-DEREF), validation at boundary (C-VALIDATE), drop discipline (C-DTOR-FAIL/BLOCK), operator overload semantics (C-OVERLOAD), sealed traits (C-SEALED), no `bool`/`Option<T>` arguments (C-CUSTOM-TYPE).
- Every rule ends with a citation in the form `(Style Guide §<section>.)` or `(API Guidelines, C-<NAME>.)` or `(RFC 430.)`.
- C-1 in STYLE.md: "Run `rustfmt` before committing" — restating user's global rule with a citation to `~/.claude/rules/rust/coding-style.md` is acceptable since that rule originates from Rust convention.

[**Phase 4 — ERRORS.md authoring (commit 1)**]

- Author from scratch under Layout A. Outline:
  - **E-1: Use `Result<T, E>` for recoverable errors; reserve panics for invariant violations.** Cite Rust Book §9.1, §9.2.
  - **E-2: Library crates use `thiserror` with a single `Error` enum.** Cite `thiserror` docs; API Guidelines C-GOOD-ERR.
  - **E-3: Application/binary code may use `anyhow::Result` for top-level error propagation.** Cite `anyhow` docs.
  - **E-4: `Error` implements `std::error::Error`, `Debug`, and `Display`.** Cite `std::error::Error`, RFC 2504, API Guidelines C-DEBUG.
  - **E-5: Provide `pub type Result<T> = std::result::Result<T, Error>;`.** Cite Rust Book §9.2 (idiomatic alias); API Guidelines.
  - **E-6: Use `#[source]` on the underlying error; `#[from]` only when the conversion is unambiguous.** Cite `thiserror` docs (transparent vs. source).
  - **E-7: Use `?` for propagation; never `unwrap()` outside tests.** Cite Rust Book §9.2; user global rust/coding-style.md.
  - **E-8: `expect("invariant reason")` is permitted only for genuinely-impossible failures (poisoned mutex, hardcoded regex, statically-known indices).** Cite Rust Book §9.3.
  - **E-9: Error message phrasing: lowercase, no trailing punctuation, no `"error: "` prefix.** Cite [Rust API Guidelines C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err) ("error messages should be lowercase without trailing punctuation").
  - **E-10: Validate at boundaries; return `Err(...)` rather than panic for recoverable misuse.** Cite API Guidelines C-VALIDATE.
  - **E-11: `unreachable!()` is reserved for branches the type system proves dead; `todo!()` is reserved for in-development scaffolding and must not appear in committed code.** Cite Rust Book §9.3; std docs.
  - **E-12: Error variants carry context fields, not concatenated strings.** Cite API Guidelines C-GOOD-ERR ("errors should provide programmatic access to context").
  - **E-13: `Error` types are `Send + Sync + 'static`.** Cite API Guidelines C-SEND-SYNC; std::error::Error documentation.
  - **E-14: Convert from foreign error types via `#[from]` or explicit `From` impls; never via `e.to_string()`.** Cite `thiserror` docs.
- Each rule cites at least one authoritative source per CN-1.

[**Phase 5 — `crates/` refactor (commit 2)**]

Sub-passes, in order, each followed by `cargo check`:

- **5a: `cargo fmt`.** Mechanical. Touches every `.rs` file.
- **5b: `cargo clippy --fix --allow-dirty --allow-staged`.** Apply auto-fixable clippy suggestions; review the rest manually.
- **5c: Naming pass.** Grep `fn get_` (none expected after fmt — confirm); audit `as_*` / `to_*` / `into_*` semantics by reading each conversion method; verify casing on `pub` items.
- **5d: Comment-formatting pass.** Grep `^/// [a-z]` (lowercase first letter on doc comment) — should yield zero matches per `COMMENTS.md` C-3 (third-person verbs start with capital). Audit `//` comments for `WHAT` paraphrases (drop them per C-7).
- **5e: Task-mark-tag pass.** Grep `\b[GCSE]-[0-9]+\b` and `\bV-[A-Z]+-[0-9]+\b` over `crates/**/*.rs`; remove any matches.
- **5f: Error-handling pass per ERRORS.md.** Audit `error.rs`: every variant has a `Display` impl with E-9 phrasing; `#[from]` and `#[source]` usage is consistent; `Result<T>` alias is used at the boundary; `unwrap()` count outside `#[cfg(test)]` is zero.
- **5g: Structural split of files >800 LOC.** Three files: `commands/upgrade.rs` (1124), `ark-cli/src/main.rs` (1093), `io/fs.rs` (1035). Strategies: split by command/subcommand for the first two; split by I/O concern (read / write / dir / link) for `fs.rs`. After split, no file exceeds 800 LOC.

[**Phase 6 — Validation gate**]

- `cargo check --all-targets --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --workspace -- -D warnings`
- `cargo test --all-targets --workspace`

All four must pass on commit 2.

## Trade-offs `ask reviewer for advice`

- **T-1: Where should the convention-SPEC layout legend live?** Options:
  - **(a)** Inline in each SPEC's `[**Purpose**]` paragraph (current plan, NG-1).
  - **(b)** A short `LAYOUT.md` in `specs/project/` describing the layout once.
  - **(c)** A user-edited `INDEX.md` paragraph.
  - **adv. (a):** zero new files; SPEC is self-contained.
  - **disadv. (a):** repeats the legend in three documents.
  - **adv. (b):** single source of truth.
  - **disadv. (b):** new file outside the user's INDEX.md model; user said "no new template".
  - **adv. (c):** matches the "INDEX is user-authored" model.
  - **disadv. (c):** asks the user to write something they didn't ask for.
  - **Default: (a).** Override available.

- **T-2: Single ERRORS.md vs. ERRORS.md + ERROR-MESSAGES.md.** Splitting message phrasing rules into a dedicated SPEC would mirror the COMMENTS/STYLE separation. Tradeoff: more files vs. cleaner rule clustering.
  - **Default: single file.** Error-message rules are too few to justify a separate SPEC.

- **T-3: Re-fetch official sources for every Phase-3/4 rule, or rely on the previously-fetched material from this conversation?** The earlier fetches covered the Style Guide, items.html, expressions.html, statements.html, naming.html, predictability.html, and the API Guidelines checklist. Phase 4 (ERRORS) needs new fetches: Rust Book ch9, `thiserror`, `anyhow`, std::error::Error.
  - **Default: re-fetch only what is missing.** Cached fetches from this session are still authoritative.

- **T-4: Does Phase 5g's structural split count as "no functional change"?** A pure split (move code into a new file, add `mod` declaration, re-export with `pub use`) is API-preserving. But splitting `main.rs` may change `clap` derive tree organization.
  - **Default: prefer pure splits; if `clap` reorganization is required, mention it in the commit body.** No public CLI surface change permitted.

- **T-5: Should commit 1 also touch `.ark/specs/project/INDEX.md`?** NG-3 says no. But the user must remember to update INDEX.md after merge.
  - **Default: VERIFY.md surfaces the proposed INDEX.md row** as a follow-up FU-N. The user applies it manually.

- **T-6: `~/.claude/rules/rust/coding-style.md` says "anyhow for application code".** `ark-cli` is technically an application, but it's a thin shim over `ark-core`. Rule E-3 currently allows `anyhow`. Should we forbid it project-wide and use only `thiserror`?
  - **Default: allow `anyhow` in `ark-cli` only; forbid in `ark-core`.** Reviewer should flag if project-wide consistency matters more.

## Validation `test design`

The bulk of validation here is *static document checks* and *workspace-level cargo checks*. There is no new runtime code to test; existing tests must continue to pass.

[**Unit Tests**]

- **V-UT-1: Rule-count preservation in COMMENTS.md migration.** Programmatic check (manual or scripted): the new COMMENTS.md contains ≥28 rule entries (one per old G-/NG-/C-). Maps to G-2, CN-2.
- **V-UT-2: Citation presence in every rule.** A grep over each new SPEC for `(.*\.)$` at the end of every `- **<PREFIX>-` rule line confirms a citation is present. Maps to G-3, G-4, CN-1.
- **V-UT-3: ERRORS.md rule coverage matrix.** Each of E-1..E-N maps to at least one cited source from the approved list (Rust Book ch9, API Guidelines, std::error, thiserror, anyhow, RFC 2504). Maps to G-4.

[**Integration Tests**]

- **V-IT-1: `cargo check --all-targets --workspace` passes after commit 1 (specs only).** Confirms commit 1 does not accidentally include code changes. Maps to G-8, CN-3.
- **V-IT-2: `cargo check --all-targets --workspace` passes after commit 2.** Maps to G-8, CN-3.
- **V-IT-3: `cargo fmt --all -- --check` passes after commit 2.** Maps to G-5.
- **V-IT-4: `cargo clippy --all-targets --workspace -- -D warnings` passes after commit 2.** Maps to G-5.
- **V-IT-5: `cargo test --all-targets --workspace` passes after commit 2.** Maps to G-8, NG-2 (no behavior change).

[**Failure / Robustness Validation**]

- **V-F-1: Scoped commit verification.** `git diff --stat <commit-1>~..<commit-1>` only touches files under `.ark/specs/project/`; `git diff --stat <commit-2>~..<commit-2>` only touches files under `crates/`. Maps to G-8.
- **V-F-2: No `unwrap()` in non-test code.** `rg '\.unwrap\(\)' crates/ -g '!**/tests/**' -g '!**/*_test.rs' -g '!**/*test*.rs'` returns zero matches in non-test code, OR every remaining occurrence has a justifying `// SAFETY:` or `// TODO:`-free comment. Maps to G-6, E-7, E-8.
- **V-F-3: No task-mark tags in `crates/`.** `rg '\b[GCSE]-[0-9]+\b' crates/` returns zero matches over `*.rs` files (excluding test fixtures and string literals where unavoidable). Maps to G-5, CN-7.

[**Edge Case Validation**]

- **V-E-1: 800-LOC cap.** `find crates -name '*.rs' -not -path '*/tests/*' | xargs wc -l | awk '$1 > 800'` returns no rows for production source. Maps to G-7, CN-8.
- **V-E-2: No `get_*` getters.** `rg 'fn get_[a-z_]+\(' crates/` returns zero matches (excluding tests if any test fixture uses the name). Maps to G-5.
- **V-E-3: ERRORS.md / STYLE.md self-conformance.** Both SPECs use Layout A (verified by visual section-header check). Maps to G-1, G-3, G-4.

[**Acceptance Mapping**]

| Goal / Constraint | Validation                       |
| ----------------- | -------------------------------- |
| G-1               | V-E-3                            |
| G-2               | V-UT-1, V-UT-2                   |
| G-3               | V-UT-2, V-E-3                    |
| G-4               | V-UT-2, V-UT-3, V-E-3            |
| G-5               | V-IT-3, V-IT-4, V-E-2, V-F-3     |
| G-6               | V-F-2                            |
| G-7               | V-E-1                            |
| G-8               | V-IT-1, V-IT-2, V-IT-5, V-F-1    |
| CN-1              | V-UT-2                           |
| CN-2              | V-UT-1                           |
| CN-3              | V-IT-1, V-IT-2                   |
| CN-4              | (covered by REVIEW iteration loop) |
| CN-5              | (covered by REVIEW)              |
| CN-6              | V-UT-3 + reviewer judgment       |
| CN-7              | V-F-3                            |
| CN-8              | V-E-1                            |

Every Goal G-1..G-8 has at least one Validation. CN-4 and CN-5 are deliberate process constraints validated by the REVIEW loop, not by a runtime test.
