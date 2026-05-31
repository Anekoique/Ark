[**Goals**]

- G-1: Convention SPECs under `specs/project/` follow Layout A.
- G-2: Migrate `COMMENTS.md` to Layout A under prefix `C-N` with zero rule loss.
- G-3: Author `STYLE.md` under prefix `S-N`, citing official Rust style sources.
- G-4: Author `ERRORS.md` under prefix `E-N`, citing official Rust error-handling sources.
- G-5: `crates/` complies with the new convention SPECs (fmt, clippy, naming, errors).

[**Non-goals**]

- NG-1: No new functionality in `crates/`.
- NG-2: No edits to `specs/project/INDEX.md` by the agent.
- NG-3: No edits to feature SPECs.

[**Architecture**]

```
.ark/specs/project/
├── INDEX.md                                    (user-edited only)
├── LAYOUT.md                                   (NEW; defines Layout A;
│                                                  defines reference-vs-template boundary)
└── rust/
    ├── COMMENTS.md                             (Layout A; prefix C-N; 1:1 migration)
    ├── STYLE.md                                (Layout A; prefix S-N; new)
    └── ERRORS.md                               (Layout A; prefix E-N; new)

crates/ (refactor target):
├── ark-core/                                   (thiserror)
│   ├── src/error.rs                            (~354 LOC; reshape per E-15)
│   ├── src/io/fs.rs                            (1035 LOC → split)
│   ├── src/commands/upgrade.rs                 (1124 LOC → split)
│   └── ...                                     (other files: format / comments / naming only)
└── ark-cli/                                    (thiserror; anyhow at the binary boundary)
    ├── src/main.rs                             (1093 LOC → split with --help diff guard)
    └── tests/                                  (format-only)
```

Two commits on `feat/project-spec`:
- Commit 1 — `feat(specs): add project-spec` — touches only `.ark/specs/project/`.
- Commit 2 — `refactor(crates): comply with project specs` — touches only `crates/`.

Each commit is independently `cargo check --all-targets` green.

[**Data Structure**]

`LAYOUT.md` records the formal grammar:

```text
ConventionSpec := Purpose Rules Exceptions Examples? SeeAlso?
Purpose        := "[**Purpose**]" Paragraph
                  // last sentence references LAYOUT.md
Rules          := "[**Rules**]" Rule+
Rule           := "- **<PREFIX>-<N>: <Title>.**" Statement Rationale? Citation
Exceptions     := "[**Exceptions**]" Carveout*
Examples       := "[**Examples**]" CodeBlock+
SeeAlso        := "[**See Also**]" CrossRef+
Citation       := text containing one of {"http", "RFC", "API Guidelines",
                  "Style Guide", "Rust Book", "thiserror", "anyhow", "std::error"}
PREFIX         := "C" | "S" | "E" | "T" | "M" | …    (one per file)

ReferenceDocument vs Template:
- ReferenceDocument: describes a convention or format.
- Template: contains placeholder sections intended to be copied and filled in.
```

[**API Surface**]

No code API change; CLI `--help` text and argument names unchanged.

Three convention documents and one reference document published under `.ark/specs/project/`:

```
LAYOUT.md     reference document defining Layout A
rust/COMMENTS.md  rules C-1..C-N
rust/STYLE.md     rules S-1..S-N
rust/ERRORS.md    rules E-1..E-N
```

[**Constraints**]

- C-1: @judgment
Every rule cites an authoritative source (RFC, Style Guide, API Guidelines, Rust Book, std/crate docs).
- C-2: @judgment
COMMENTS.md migration preserves every prior rule 1:1 (≥23 entries under `[**Rules**]`, ≥5 under `[**Exceptions**]`); mapping table in commit body.
- C-3: @tool: cargo check --all-targets
Each commit on `feat/project-spec` is independently `cargo check --all-targets` green.
- C-4: @judgment
STYLE.md preserves every rule from any prior draft that has an authoritative source; rule-source mapping in commit body.
- C-5: @source-scan: unwrap( @ crates/ark-core/src/**/*.rs
ERRORS.md is consistent with Rust style: `thiserror` for libraries, `anyhow` for binaries, `?` for propagation, no `unwrap()` in production.
- C-6: @source-scan: V-(UT|IT|E|F)-\d @ crates/**/*.rs
Source files contain no SPEC-rule-id annotations (`C-N` / `S-N` / `E-N` / `V-*-N`) outside citation comments and string literals.
- C-7: @judgment
No file in `crates/` exceeds 800 LOC after refactor; soft target: post-split files >400 LOC require a one-line explanation in commit 2's body.
- C-8: @judgment
No new template file under `.ark/templates/`; that tree is reserved for feature-SPEC artifacts. Reference documents under `specs/project/` are permitted (LAYOUT.md is one).

[**CHANGELOG**]

- 2026-05-08 `doc-tighten`: rewritten to match tightened SPEC contract; semantic content preserved.
