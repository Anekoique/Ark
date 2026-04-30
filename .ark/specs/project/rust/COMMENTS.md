[**Goals**]

Rules every agent and contributor follows when writing comments and doc-comments in this repository's Rust source. Sourced from the official Rust API Guidelines, [RFC 0505 — API Comment Conventions](https://rust-lang.github.io/rfcs/0505-api-comment-conventions.html), and [The rustdoc Book — How to write documentation](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html). Where the reference disagrees with what feels natural, the reference wins.

- **G-1: Comment kind matches what's being annotated.**
  - `///` for items: `fn`, `struct`, `enum`, `trait`, `const`, `static`, `type`, `impl` blocks, `mod` declarations.
  - `//!` for module/crate inner documentation. Goes at the top of `mod.rs` / `lib.rs`, before any `use` or item.
  - `//` for inline notes inside function bodies and for genuinely private helpers whose doc would never reach rustdoc.
  - `////` is invalid — that's three slashes plus a comment slash. Don't write it.

- **G-2: First sentence is the summary.** One short complete sentence, ending in a period. Goes on the first `///` line and is what rustdoc shows in indices and search. Subsequent paragraphs (separated by a blank `///` line) carry detail.

- **G-3: Third-person singular present indicative.** "Returns the foo." — not "Return the foo.", not "Returns foo" without a period. Applies to every function/method doc and every test docstring. Module/crate `//!` summaries may use noun phrases ("Embedded template trees.") because the subject is the module itself.

- **G-4: Sections use exact rustdoc-recognized headings.** When present, in this order:
  - `# Examples` — plural even for one example.
  - `# Panics` — when the function may panic.
  - `# Errors` — when the function returns `Result` and the error variants matter to the caller.
  - `# Safety` — required on every `unsafe` item; describes the invariants the caller must uphold.

  Each section is a `# H1` (single hash) inside the doc-comment, preceded by a blank `///` line.

- **G-5: Code examples use `?`, not `unwrap()` or `try!`.** Example code is copy-pasted by readers; `unwrap()` in examples normalizes a bad pattern. Annotate language explicitly on triple-grave blocks (` ```rust `, ` ```toml `, ` ```text `, etc.) so external markdown renderers (GitHub, the book) highlight them.

- **G-6: Link to types, functions, and modules using rustdoc reference syntax.** `[`Foo`]`, `[`Foo::bar`]`, or `[Foo](module::Foo)` — never bare `Foo` for a linkable item. Cross-crate references use the absolute path (`[`std::io::Error`]`).

- **G-7: Comments inside function bodies explain *why*, not *what*.** If a `//` comment paraphrases the line below it ("// increment counter" above `counter += 1;`), delete it. Keep `//` comments only when they capture a non-obvious invariant, the reason for a workaround, or a constraint the type system can't express. Even then, keep them tight.

- **G-8: Don't reference task names, archived task slugs, PR numbers, or other out-of-tree process artifacts in source comments.** The constraint (the *why*) stays; its *label* goes. Process metadata belongs in commit messages, PR descriptions, archived task PRDs, and feature SPECs — not in `crates/`.

- **G-9: Don't restate the type system.** "Returns a `Result<()>`" or "Takes a `&str`" tells the reader nothing the signature didn't. Document the *meaning* of success and failure, not the shape.

- **G-10: Concision over completeness.** No "Note that..." preambles — just state the note. No "Used by..." caller lists (they rot). No "Per the spec" hedges — document the contract directly. Short comments that read cleanly beat long ones that hedge.

- **G-11: Test docstrings follow G-3.** `/// Verifies that …` not `/// Verify …`. `/// Round-trips writes through reads.` not `/// Round-trip render → parse.` Tests are still public-shaped items inside the `tests` module; their docstrings show up in rustdoc when test code is included.

- **G-12: Trivial private helpers don't need a docstring.** `///` on a one-line `fn parse_oneline` whose body is `s.lines().filter(...)` adds no information. Reserve `///` for items where the comment communicates intent the body doesn't.

[**Non-goals**]

- **NG-1: No required minimum doc length.** A one-sentence `///` is fine. A trait method may be longer when it has invariants; a getter usually isn't.
- **NG-2: No mandatory `# Examples` everywhere.** The API Guidelines' C-EXAMPLE recommends one for every public item, but on internal `pub(crate)` items in `ark-core` the bar is judgment, not policy.
- **NG-3: No comments on every line.** Code that reads cleanly is the goal; comments are the fallback when the code can't carry the meaning alone.
- **NG-4: No prose translation of every `unsafe` block — but every `unsafe` operation needs a `// SAFETY:` comment** explaining why the invariants hold here. This is non-negotiable; clippy's `undocumented_unsafe_blocks` lint enforces it where enabled.
- **NG-5: No copy-paste from `cargo expand` or macro-generated code into doc-comments.** Docs are written; they aren't generated artifacts.

[**Architecture**]

```
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

Inside function bodies: `//` comments explain why a line of code exists. They don't paraphrase what it does.

[**Data Structure**]

Not applicable — this SPEC is a coding convention, not a runtime structure. The "data" is the comment characters themselves and the order they appear in.

[**API Surface**]

Worked examples follow.

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
// Validate identity *first*: a malformed `--developer` should fail
// before we scaffold any platform files (rolling back a partial scaffold
// is harder than refusing the call).
identity::validate_developer_name(name)?;
```

[**Constraints**]

- **C-1: One sentence on the first `///` line.** Multi-sentence summaries make rustdoc's index column unreadable. If you need two sentences, the second goes after a blank `///` line.

- **C-2: Punctuation is not optional.** Every doc paragraph ends with `.`, `?`, or `!`. Trailing `:` cliffhangers ("...the body opens with:") must be followed immediately by the thing they introduce — usually a code block.

- **C-3: Code-block language annotations are not optional.** Triple-grave blocks default to Rust inside rustdoc but render unhighlighted on GitHub and inside the book. Always annotate.

- **C-4: `unsafe` blocks carry `// SAFETY:` comments.** Even when the invariant feels obvious. Future readers don't share your context.

- **C-5: Avoid contractions in doc-comments.** "does not" beats "doesn't"; "cannot" beats "can't". This isn't pedantry; rustdoc's tone across the standard library is uncontracted, and matching it makes Ark's docs feel native.

- **C-6: Don't link to private items from public docs.** `[`pub fn foo`] -> [`crate::internal::detail`]` produces a broken hyperlink in rustdoc when the link target is `pub(crate)`. Either make the target public or describe it inline.

- **C-7: Re-export documentation lives with the original item.** `pub use crate::module::Foo;` does not need its own `///`; rustdoc renders the underlying item's docs at both sites.

- **C-8: Doc-comments on `impl` blocks are rare and should be exceptional.** They don't appear on the rendered impl item itself; rustdoc lifts them oddly. Document the trait/struct, not the `impl`.

- **C-9: First-sentence verbs match the kind of item:**
  - Function returning a value → "Returns ...".
  - Function with side effects → "Writes ...", "Removes ...", "Updates ...".
  - Function that may fail → describe the success path; use `# Errors` for the failure modes.
  - Function predicate (`is_*`, `has_*`) → "Returns `true` iff ..." or "Reports whether ...".
  - Constructor → "Creates a new ...".
  - Builder method → "Sets the ... and returns the builder."

- **C-10: Hyperlinks use the shortest unambiguous form.** Inside the same module, `[`Foo`]` is enough. Across modules, `[`crate::module::Foo`]` or just `[Foo]` if rustdoc can resolve it from `use` statements in scope.

- **C-11: Never use task-mark tags in our project.** Never appear `V-UT-1` / `C-11` / `G-7` ...task-mark tags in comments.
