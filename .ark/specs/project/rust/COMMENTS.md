[**Purpose**]

Rules every agent and contributor follows when writing comments and doc-comments in this repository's Rust source. Sourced from the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), [RFC 0505 — API Comment Conventions](https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html), and [The rustdoc Book — How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html). Where the reference disagrees with what feels natural, the reference wins. Layout: see `specs/project/LAYOUT.md`.

[**Rules**]

- **C-1: Comment kind matches what's being annotated.** `///` annotates items (`fn`, `struct`, `enum`, `trait`, `const`, `static`, `type`, `impl` blocks, `mod` declarations); `//!` carries module/crate inner documentation and goes at the top of `mod.rs` / `lib.rs` before any `use` or item; `//` carries inline notes inside function bodies and on genuinely private helpers whose doc would never reach rustdoc; `////` is invalid (three slashes plus a comment slash) — don't write it ⟨@judgment⟩.

- **C-2: First sentence is the summary.** One short complete sentence, ending in a period. Goes on the first `///` line and is what rustdoc shows in indices and search. Subsequent paragraphs (separated by a blank `///` line) carry detail ⟨@judgment⟩.

- **C-3: Third-person singular present indicative.** "Returns the foo." — not "Return the foo.", not "Returns foo" without a period. Applies to every function/method doc and every test docstring. Module/crate `//!` summaries may use noun phrases ("Embedded template trees.") because the subject is the module itself ⟨@judgment⟩.

- **C-4: Sections use exact rustdoc-recognized headings.** When present, in this order: `# Examples` (plural even for one example), `# Panics` (when the function may panic), `# Errors` (when the function returns `Result` and the error variants matter to the caller), `# Safety` (required on every `unsafe` item; describes the invariants the caller must uphold). Each section is a `# H1` (single hash) inside the doc-comment, preceded by a blank `///` line ⟨@judgment⟩.

- **C-5: Code examples use `?`, not `unwrap()` or `try!`.** Example code is copy-pasted by readers; `unwrap()` in examples normalizes a bad pattern. Annotate language explicitly on triple-grave blocks (` ```rust `, ` ```toml `, ` ```text `) so external markdown renderers (GitHub, the book) highlight them ⟨@judgment⟩.

- **C-6: Link to types, functions, and modules using rustdoc reference syntax.** `` [`Foo`] ``, `` [`Foo::bar`] ``, or `[Foo](module::Foo)` — never bare `Foo` for a linkable item. Cross-crate references use the absolute path (`` [`std::io::Error`] ``) ⟨@judgment⟩.

- **C-7: Comments inside function bodies explain *why*, not *what*.** If a `//` comment paraphrases the line below it ("// increment counter" above `counter += 1;`), delete it. Keep `//` comments only when they capture a non-obvious invariant, the reason for a workaround, or a constraint the type system can't express. Even then, keep them tight ⟨@judgment⟩.

- **C-8: Don't reference task names, archived task slugs, PR numbers, or other out-of-tree process artifacts in source comments.** The constraint (the *why*) stays; its *label* goes. Process metadata belongs in commit messages, PR descriptions, archived task PRDs, and feature SPECs — not in `crates/` ⟨@judgment⟩.

- **C-9: Don't restate the type system.** "Returns a `Result<()>`" or "Takes a `&str`" tells the reader nothing the signature didn't. Document the *meaning* of success and failure, not the shape ⟨@judgment⟩.

- **C-10: Concision over completeness.** No "Note that..." preambles — just state the note. No "Used by..." caller lists (they rot). No "Per the spec" hedges — document the contract directly. Short comments that read cleanly beat long ones that hedge ⟨@judgment⟩.

- **C-11: Test docstrings follow C-3.** `/// Verifies that …` not `/// Verify …`. `/// Round-trips writes through reads.` not `/// Round-trip render → parse.` Tests are still public-shaped items inside the `tests` module; their docstrings show up in rustdoc when test code is included ⟨@judgment⟩.

- **C-12: Trivial private helpers don't need a docstring.** `///` on a one-line `fn parse_oneline` whose body is `s.lines().filter(...)` adds no information. Reserve `///` for items where the comment communicates intent the body doesn't ⟨@judgment⟩.

- **C-13: One sentence on the first `///` line.** Multi-sentence summaries make rustdoc's index column unreadable. If you need two sentences, the second goes after a blank `///` line ⟨@judgment⟩.

- **C-14: Punctuation is not optional.** Every doc paragraph ends with `.`, `?`, or `!`. Trailing `:` cliffhangers ("...the body opens with:") must be followed immediately by the thing they introduce — usually a code block ⟨@judgment⟩.

- **C-15: Code-block language annotations are not optional.** Triple-grave blocks default to Rust inside rustdoc but render unhighlighted on GitHub and inside the book. Always annotate ⟨@judgment⟩.

- **C-16: `unsafe` blocks carry `// SAFETY:` comments.** Even when the invariant feels obvious. Future readers don't share your context. Clippy's `undocumented_unsafe_blocks` lint enforces this where enabled ⟨@tool: clippy⟩.

- **C-17: Avoid contractions in doc-comments.** "does not" beats "doesn't"; "cannot" beats "can't". This isn't pedantry; rustdoc's tone across the standard library is uncontracted, and matching it makes Ark's docs feel native ⟨@judgment⟩.

- **C-18: Don't link to private items from public docs.** `` [`pub fn foo`] -> [`crate::internal::detail`] `` produces a broken hyperlink in rustdoc when the link target is `pub(crate)`. Either make the target public or describe it inline ⟨@judgment⟩.

- **C-19: Re-export documentation lives with the original item.** `pub use crate::module::Foo;` does not need its own `///`; rustdoc renders the underlying item's docs at both sites ⟨@judgment⟩.

- **C-20: Doc-comments on `impl` blocks are rare and should be exceptional.** They don't appear on the rendered impl item itself; rustdoc lifts them oddly. Document the trait/struct, not the `impl` ⟨@judgment⟩.

- **C-21: First-sentence verbs match the kind of item.** Function returning a value → "Returns ..."; function with side effects → "Writes ...", "Removes ...", "Updates ..."; function that may fail → describe the success path and use `# Errors` for failure modes; predicate (`is_*`, `has_*`) → "Returns `true` iff ..." or "Reports whether ..."; constructor → "Creates a new ..."; builder method → "Sets the ... and returns the builder" ⟨@judgment⟩.

- **C-22: Hyperlinks use the shortest unambiguous form.** Inside the same module, `` [`Foo`] `` is enough. Across modules, `` [`crate::module::Foo`] `` or just `[Foo]` if rustdoc can resolve it from `use` statements in scope ⟨@judgment⟩.

- **C-23: Never use task-mark tags in source.** Identifiers like `V-UT-1`, `C-11`, `G-7`, or other SPEC-rule labels never appear inside `crates/` comments. The constraint they encode goes inline as prose; the label goes nowhere ⟨@source-scan: V-(UT|IT|E|F)-\d @ crates/**/*.rs⟩.

[**Exceptions**]

- **EX-1: No required minimum doc length.** A one-sentence `///` is fine. A trait method may be longer when it has invariants; a getter usually isn't.

- **EX-2: No mandatory `# Examples` everywhere.** [API Guidelines C-EXAMPLE](https://rust-lang.github.io/api-guidelines/documentation.html#all-items-have-a-rustdoc-example-c-example) recommends one for every public item, but on internal `pub(crate)` items in `ark-core` the bar is judgment, not policy.

- **EX-3: No comments on every line.** Code that reads cleanly is the goal; comments are the fallback when the code can't carry the meaning alone.

- **EX-4: Every `unsafe` operation needs a `// SAFETY:` comment.** No prose translation of every `unsafe` block is required, but each individual unsafe operation must carry a `// SAFETY:` comment explaining why the invariants hold here. This is non-negotiable; clippy's `undocumented_unsafe_blocks` lint enforces it where enabled.

- **EX-5: No copy-paste from `cargo expand` or macro-generated code into doc-comments.** Docs are written; they aren't generated artifacts.

[**Examples**]

```text
.rs file
├── //! file-level docs (if module/crate)
│   ├── one-line summary
│   ├── (blank //! line)
│   └── extended description (optional)
│
├── use statements
│
└── items
    ├── /// summary line (third-person, ends with period)
    ├── ///                              ← blank ///
    ├── /// extended description
    ├── ///
    ├── /// # Examples                   ← optional
    ├── /// ```rust
    ├── /// let x = foo();
    ├── /// ```
    ├── ///
    ├── /// # Panics                     ← optional
    ├── /// Detail.
    ├── ///
    ├── /// # Errors                     ← optional, on Result-returning fns
    └── /// Detail.
```

**Function:**

```rust
/// Returns the canonical Ark `SessionStart` hook entry for Claude Code.
///
/// The entry's `command` field is the identity key Ark uses to detect
/// (and replace) its own entry across runs.
///
/// # Panics
///
/// Never. The body uses `serde_json::json!` with literal values.
pub fn ark_session_start_hook_entry() -> serde_json::Value { /* ... */ }
```

**Method on a struct:**

```rust
impl Layout {
    /// Resolves a project-relative path, rejecting absolute paths,
    /// root/prefix components, and any `..` traversal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsafeSnapshotPath`] if `relative` is absolute,
    /// empty, contains a drive/UNC prefix, or contains a `..` component.
    pub fn resolve_safe(&self, relative: impl AsRef<Path>) -> Result<PathBuf> { /* ... */ }
}
```

**Module:**

```rust
//! Filesystem and text-content I/O.
//!
//! - [`path_ext::PathExt`] wraps `std::fs` calls with Ark's `Error::Io`.
//! - [`fs`] handles content-aware writes, managed-block editing, and
//!   directory walks.
//! - [`git`] is the only sanctioned `Command::new("git")` site.
```

**Test docstring:**

```rust
/// Round-trips writes through reads, preserving byte content.
#[test]
fn round_trip_preserves_bytes() { /* ... */ }
```

**Inline why-comment:**

```rust
// Validate the slug *first*: a malformed slug should fail before we
// scaffold any task files (rolling back a partial scaffold is harder
// than refusing the call).
validate_slug(&opts.slug)?;
```

[**See Also**]

- `LAYOUT.md` — the convention-SPEC layout.
- `STYLE.md` — Rust code-shape conventions; comments and prose follow C-rules here, code shape follows S-rules there.
- `ERRORS.md` — error-handling conventions; `# Errors` sections (C-4) describe error semantics defined by E-rules.
- [RFC 0505 — API Comment Conventions](https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html).
- [The rustdoc Book](https://doc.rust-lang.org/rustdoc/).
- [Rust API Guidelines — Documentation](https://rust-lang.github.io/api-guidelines/documentation.html).
