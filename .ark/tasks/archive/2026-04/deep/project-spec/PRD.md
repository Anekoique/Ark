# `project-spec` PRD

---

[**What**]

Refactor the existing project-level convention SPECs (COMMENTS.md, STYLE.md) onto a new "convention SPEC" layout (Layout A: `Purpose / Rules / Exceptions / Examples / See Also`, flat numbered list, single rule prefix per file), add a new `rust/ERRORS.md` SPEC sourced from authoritative references, and refactor the `crates/` codebase to comply with all three SPECs.

[**Why**]

The current convention SPECs reuse the feature-SPEC template (`Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints`). That template is built for runtime systems — its slots fit awkwardly on a coding-convention document:
- *Architecture* becomes a fake source-file diagram.
- *Data Structure* is "not applicable" (literally written in both COMMENTS.md and the discarded STYLE.md draft).
- The Goals/Constraints split forces semi-arbitrary classification of rules that are functionally identical (e.g. COMMENTS.md G-2 "first sentence is the summary" vs. C-1 "one sentence per `///` line" — both are formatting rules of the same kind).

A purpose-fit layout for convention SPECs improves readability, removes empty sections, and gives agents a single flat rule list per file. Adding ERRORS.md closes the largest remaining gap in Rust convention coverage (error handling is currently un-specified). Refactoring `crates/` to comply ensures the SPECs are not aspirational — they describe the codebase as it is.

[**Outcome**]

1. **Convention-SPEC layout in use.** Both `rust/COMMENTS.md` and `rust/STYLE.md` use Layout A: `[**Purpose**] / [**Rules**] / [**Exceptions**] / [**Examples**] / [**See Also**]`. Each file has a single rule prefix (e.g. `C-N` for COMMENTS, `S-N` for STYLE, `E-N` for ERRORS). Every existing rule from COMMENTS.md and STYLE.md is preserved 1:1 (no editorial deletions; renaming/renumbering only).

2. **ERRORS.md exists and is authoritative-sourced.** `.ark/specs/project/rust/ERRORS.md` is committed, every rule cites at least one of: [The Rust Programming Language Book — Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html), [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (especially C-GOOD-ERR), [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html), [`thiserror`](https://docs.rs/thiserror/) and [`anyhow`](https://docs.rs/anyhow/) crate documentation, [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html), and Rust API Guidelines on validation (C-VALIDATE).

3. **`crates/` complies with all three SPECs.** Verification checklist:
   - `cargo fmt --check` passes (mechanical compliance).
   - `cargo clippy --all-targets -- -D warnings` passes.
   - All naming-rule violations corrected (no `get_*` getters, casing per RFC 430, `as_*` / `to_*` / `into_*` conversion semantics correct).
   - All comment-formatting violations corrected per COMMENTS.md (third-person verbs, `# Examples` / `# Panics` / `# Errors` / `# Safety` sections where applicable, no task-mark tags in source).
   - Error types follow ERRORS.md (canonical `Error` enum, `thiserror` boundary discipline, error message phrasing, `Result<T>` alias usage).
   - Module/structural cleanup: any file or module that no longer makes sense after the above changes is reorganized (small unit (c)).

4. **Two commits on `feat/project-spec`:**
   - Commit 1 — `feat(specs): add project-spec` — touches only `.ark/specs/project/`.
   - Commit 2 — `refactor(crates): comply with project specs` — touches only `crates/` (and tests/build files as needed).

5. **`project/INDEX.md` updated by the user.** Per the rule in `project/INDEX.md` itself ("only edited and modified by user"), agents do not write the index row for ERRORS.md. The task surfaces the suggested row to the user at archive time.

6. **Existing tests pass; new tests added only where the refactor introduces new public API surface** (none expected — this task is convention + compliance, not new features).

[**Related Specs**]

This task touches no `specs/features/*` SPECs directly. It defines new project-level SPECs and refactors source code, but does not alter feature-SPEC contracts.

- `specs/project/rust/COMMENTS.md` — migrated to Layout A; rule prefix unifies to `C-N`.
- `specs/project/rust/STYLE.md` — re-authored under Layout A from official sources (Rust Style Guide, RFC 430, API Guidelines); rule prefix `S-N`.
- `specs/project/rust/ERRORS.md` — *new*; rule prefix `E-N`; cited sources listed in Outcome §2.
- `specs/project/INDEX.md` — needs a new row for ERRORS.md and an updated row for STYLE.md, but the file is user-edited only; flag for the user at archive.
