[**Purpose**]

Rules every agent and contributor follows when designing error types and propagating errors in this repository's Rust source. Sourced from [The Rust Programming Language Book — Chapter 9 (Error Handling)](https://doc.rust-lang.org/book/ch09-00-error-handling.html), [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html), the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (especially [C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err) and [C-VALIDATE](https://rust-lang.github.io/api-guidelines/dependability.html#functions-validate-their-arguments-c-validate)), the [`thiserror` crate documentation](https://docs.rs/thiserror/), the [`anyhow` crate documentation](https://docs.rs/anyhow/), and [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html). Layout: see `specs/project/LAYOUT.md`.

[**Rules**]

- **E-1: Use `Result<T, E>` for recoverable errors; reserve panics for invariant violations.** Recoverable failure (file missing, parse error, validation failure, network down) returns an error; unrecoverable invariant violation (out-of-bounds index that the type system proved impossible, mutex poisoning the program cannot continue past) panics ⟨@judgment⟩.

- **E-2: `ark-core` defines a single canonical `Error` enum with `thiserror`.** Library code in `ark-core` exposes one `Error` type (`crate::error::Error`) for every fallible operation. The enum is annotated `#[derive(Debug, thiserror::Error)]` ⟨@judgment⟩.

- **E-3: `ark-cli` may use `anyhow::Result` at the binary boundary.** The `ark-cli` binary is allowed to consume `ark-core::Result<T>` and re-wrap it via `anyhow::Result<T>` for top-level `main()`-style propagation. Library functions inside `ark-cli` itself still return `crate::Result<T>` over `ark_core::Error`; only the binary's outermost layer reaches for `anyhow` ⟨@judgment⟩.

- **E-4: `Error` implements `std::error::Error`, `Debug`, and `Display`.** `thiserror`'s `#[derive(Error)]` produces all three; hand-written impls must too. The `Display` impl returns a concise lowercase sentence with no trailing punctuation; the `Debug` impl is never empty ⟨@judgment⟩.

- **E-5: Provide `pub type Result<T> = std::result::Result<T, Error>;`.** Each crate that defines an `Error` enum also exposes a same-module `Result<T>` alias. Functions return `Result<T>`, not `std::result::Result<T, Error>` ⟨@judgment⟩.

- **E-6: Use `#[from]` for unambiguous foreign-error wrapping; use `#[source]` when the variant carries additional context fields.** Per `thiserror`'s rule: "The variant using `#[from]` must not contain any other fields beyond the source error." If a variant needs both a `source: io::Error` and a `path: PathBuf`, the source field is annotated `#[source]` (not `#[from]`); the `From` conversion is then either omitted or written by hand ⟨@judgment⟩.

- **E-7: Use `?` for propagation; never `unwrap()` outside tests.** Every fallible call inside non-test code propagates with `?` or handles the error explicitly. `unwrap()` in production code is forbidden; clippy's `unwrap_used` lint should treat it as a warning at minimum ⟨@tool: clippy⟩.

- **E-8: `expect("invariant reason")` is permitted only when the failure is genuinely impossible.** Hardcoded regex compilation, mutex `lock()` in single-threaded test contexts, and statically-known indices are the only legitimate sites. The `"reason"` argument states the invariant being relied on, not "this should not fail." ⟨@judgment⟩

- **E-9: Error message phrasing: lowercase first word, no trailing punctuation, no `"error: "` prefix.** `#[error("io error at {path}: {source}")]` not `#[error("Error: I/O Error at {path}.")]`. The CLI front-end and any error-rendering layer is responsible for the user-facing presentation; the `Display` impl provides the raw sentence ⟨@judgment⟩.

- **E-10: Validate at boundaries; return `Err(...)` rather than panic for recoverable misuse.** Public functions verify their preconditions (path safety, slug shape, tier validity) on entry and return an `Error` variant when violated. Panics are reserved for invariants the caller cannot violate even by misuse ⟨@judgment⟩.

- **E-11: `unreachable!()` is reserved for branches the type system proves dead; `todo!()` and `unimplemented!()` are scaffolding and must not appear in committed non-test code.** A reachable `unreachable!()` is a logic bug, not an error to handle ⟨@source-scan: (todo|unimplemented)!() @ crates/**/*.rs⟩.

- **E-12: Error variants carry context as fields, not as concatenated strings inside the `Display` template.** `Error::Io { path: PathBuf, source: io::Error }` — not `Error::Io(String)` formed by `format!("io at {}: {}", path, e)`. Programmatic access to context (e.g. for error-message rendering, retries, structured logging) requires the data to remain typed ⟨@judgment⟩.

- **E-13: `Error` types are `Send + Sync + 'static`.** Required for compatibility with `anyhow::Error`, `Box<dyn Error + Send + Sync>`, async tasks, and standard error-handling middleware. `thiserror`'s default derive produces this when the variant fields are themselves `Send + Sync + 'static` ⟨@judgment⟩.

- **E-14: Convert from foreign error types via `#[from]` or explicit `From` impls; never via `e.to_string()`.** Stringifying an error at conversion time discards the source chain, the type information, and any structured data the foreign error carried. The `?` operator with a `#[from]` impl preserves all three ⟨@judgment⟩.

- **E-15: Wrapped foreign errors carry at least one context field identifying the resource or operation.** A variant that wraps `io::Error`, `serde_json::Error`, `git2::Error`, or any other foreign error includes a `path: PathBuf`, `command: String`, or analogous field that names the resource the operation was acting on. The `source` field is annotated `#[source]` (not `#[from]`, since `#[from]` forbids additional fields per E-6); a hand-written `From` impl may be added if `?`-conversion is desired. A bare `#[from]` wrapper with no context field produces user-visible messages like "No such file or directory" with zero indication of which file or operation failed ⟨@judgment⟩.

[**Exceptions**]

- **EX-1: `unwrap()` in `#[cfg(test)]` and `#[test]` code is permitted.** Tests are allowed to assume happy paths; a panic inside a test is just a failure mode. The lint exclusion is the path-glob `tests/**` plus any module gated on `#[cfg(test)]`.

- **EX-2: `expect("static-known invariant")` in initialization of `static`/`const` items.** A `Regex::new("...").expect("...")` at module init is acceptable because the regex literal is checked at compile-tested test runs and a failure is a build-level bug, not a runtime error to handle.

- **EX-3: A single-purpose error enum whose name itself supplies context may use bare `#[from]` variants.** E.g. `enum ConfigLoadError { #[from] Io(io::Error), #[from] Parse(toml::de::Error) }` — the type name `ConfigLoadError` already contextualizes; no per-variant `path` field is required. This exception applies only when (a) the enum has a single domain, and (b) every consumer can identify the operation from the type name alone.

- **EX-4: `anyhow!`, `bail!`, and `ensure!` in `ark-cli`'s `main()` body are permitted.** These macros produce `anyhow::Error` ad-hoc errors; in `ark-core` they are not used.

[**Examples**]

**Canonical `Error` enum in `ark-core` (illustrating E-2, E-4, E-5, E-6, E-9, E-12, E-15):**

```rust
use std::{io, path::PathBuf};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// I/O failure on a specific path.
    ///
    /// Note `#[source]` (not `#[from]`) because the variant carries an extra
    /// `path` context field — see E-6 and E-15.
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Snapshot path failed safety validation.
    #[error("refusing unsafe snapshot path {path:?}: {reason}")]
    UnsafeSnapshotPath {
        path: PathBuf,
        reason: &'static str,
    },

    /// Manifest JSON failed to parse.
    #[error("manifest corrupt at {path}: {source}")]
    ManifestCorrupt {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}
```

**Hand-written `From` impl when context fields preclude `#[from]` (E-6):**

```rust
impl Error {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }
}

// At a call site, instead of `?`:
let bytes = std::fs::read(&path).map_err(|e| Error::io(&path, e))?;
```

**Permitted bare `#[from]` (EX-3 — single-purpose enum):**

```rust
#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("io while loading config: {0}")]
    Io(#[from] io::Error),

    #[error("parse error in config: {0}")]
    Parse(#[from] toml::de::Error),
}
```

**Validation at the boundary (E-10):**

```rust
pub fn open(path: &Path) -> Result<File> {
    if path.is_absolute() {
        return Err(Error::UnsafeSnapshotPath {
            path: path.to_owned(),
            reason: "absolute paths are not permitted",
        });
    }
    File::open(path).map_err(|e| Error::io(path, e))
}
```

**Binary-boundary `anyhow` usage in `ark-cli` (E-3, EX-4):**

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run().with_context(|| format!("running `{}`", cli.command_name()))?;
    Ok(())
}
```

**Forbidden — bare `#[from]` with no context (E-15 violation):**

```rust
// DO NOT DO THIS in ark-core:
#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] io::Error),         // no path, no operation — fails E-15
}
```

[**See Also**]

- `LAYOUT.md` — the convention-SPEC layout.
- `STYLE.md` — code-shape rules; error-type derivation discipline (S-17, S-18, S-19) and `Send + Sync` (S-39) interact directly with E-13.
- `COMMENTS.md` — `# Errors` section requirements (C-4) on `Result`-returning functions document which `Error` variants matter to the caller.
- [The Rust Book §9 — Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html).
- [`std::error::Error` documentation](https://doc.rust-lang.org/std/error/trait.Error.html).
- [Rust API Guidelines — Interoperability §C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#error-types-are-meaningful-and-well-behaved-c-good-err).
- [`thiserror` crate documentation](https://docs.rs/thiserror/).
- [`anyhow` crate documentation](https://docs.rs/anyhow/).
- [Rust RFC 2504 — Fixed `Error` trait](https://rust-lang.github.io/rfcs/2504-fix-error.html).
