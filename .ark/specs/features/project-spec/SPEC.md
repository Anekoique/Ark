
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
