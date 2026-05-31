[**Purpose**]

Rules every agent and contributor follows when writing Rust source in this repository. Sourced from the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) (the `rustfmt` reference), [RFC 430 — Finalizing naming conventions](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html), and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). Where the reference disagrees with what feels natural, the reference wins. Layout: see `specs/project/LAYOUT.md`.

[**Rules**]

- **S-1: Indent with four spaces, never tabs.** Each level of indentation outside string literals and comments must be a multiple of 4 ⟨@tool: rustfmt⟩.

- **S-2: Maximum line width is 100 characters.** Comment text (excluding the leading `//`/`///` and indentation) is limited to 80 characters or the line-width cap, whichever is smaller ⟨@tool: rustfmt⟩.

- **S-3: Prefer block indent over visual indent.** When a call or expression must wrap, break after the opening delimiter and put each argument on its own block-indented line — do not align continuation lines to the column of an opening paren. Block indent yields smaller diffs and avoids rightward drift ⟨@tool: rustfmt⟩.

- **S-4: Trailing comma on the last element of any multi-line comma-separated list.** Function arguments, struct fields, enum variants, array literals, generic parameter lists, derive lists. Single-line lists do not take a trailing comma ⟨@tool: rustfmt⟩.

- **S-5: No trailing whitespace.** Applies to blank lines, comment lines, code lines, and (with care) string literals ⟨@tool: rustfmt⟩.

- **S-6: Items and statements separated by zero or one blank line.** Two or more consecutive blank lines are not permitted inside a function body or at module scope ⟨@tool: rustfmt⟩.

- **S-7: Casing follows RFC 430.** Modules, functions, methods, vars, macros use `snake_case`; types, traits, enum variants use `UpperCamelCase`; statics and consts use `SCREAMING_SNAKE_CASE`; type parameters use a single uppercase letter; lifetimes use a single lowercase letter. Acronyms count as a single word in `UpperCamelCase`: `Uuid`, `HttpClient`, `IoError` — not `UUID`, `HTTPClient`, `IOError` ⟨@tool: clippy⟩.

- **S-8: Conversion methods use the standard prefixes.** `as_*` is free (borrowed → borrowed), `to_*` is potentially expensive, `into_*` consumes the receiver and returns owned. Wrappers expose `into_inner` to surrender the inner value ⟨@judgment⟩.

- **S-9: No `get_` prefix on getters.** A field accessor is named after the field: `first()`, `name()`, `len()` — not `get_first()`, `get_name()`. Mutable accessors use the `_mut` suffix: `first_mut()` ⟨@tool: clippy⟩.

- **S-10: Iterator methods follow the `iter` / `iter_mut` / `into_iter` triple.** Their iterator types are named `Iter`, `IterMut`, `IntoIter` ⟨@judgment⟩.

- **S-11: Constructors are static inherent methods.** The default name is `new`. Use a domain-specific verb (`open`, `connect`, `with_capacity`) when it carries more meaning. Reach for the builder pattern when construction has more than three meaningful options ⟨@judgment⟩.

- **S-12: Prefer line comments over block comments.** Use `//`; reserve `/* … */` for the rare in-expression case where line comments don't fit. Single space after `//`. Comments form complete sentences, start with a capital letter, end with `.` ⟨@tool: rustfmt⟩.

- **S-13: Imports group then version-sort.** Groups are separated by blank lines; tools must not merge groups, but within each group `use` lines are version-sorted. `self` and `super` come first inside a brace list; glob (`*`) and group (`b::{…}`) imports come last ⟨@tool: rustfmt⟩.

- **S-14: Module-item ordering at file scope.** `extern crate` declarations come first (alphabetical), then `use` and `mod` declarations (imports first, version-sorted), then other items ⟨@tool: rustfmt⟩.

- **S-15: Spaces around binary operators; none around unary.** `x + 1`, `a == b`, `&mut x` — but `!flag`, `*ptr`, `&x`. The single exception is `&mut`, which takes a space after `mut` ⟨@tool: rustfmt⟩.

- **S-16: No extraneous parentheses around control-flow conditions.** Write `if cond { … }`, not `if (cond) { … }`. Parentheses are still permitted to disambiguate compound arithmetic or boolean expressions ⟨@tool: clippy⟩.

- **S-17: One attribute per line; one `derive` attribute per item.** Multiple derived names combine into a single `#[derive(A, B, C)]` rather than stacked `#[derive(A)] #[derive(B)]` ⟨@tool: rustfmt⟩.

- **S-18: Public types implement `Debug`.** Every `pub` type that escapes the crate gets a `#[derive(Debug)]` (or a hand-written `Debug` impl when derive isn't usable). The `Debug` output is never empty ⟨@judgment⟩.

- **S-19: Implement common traits eagerly.** When the type semantics permit them, derive or implement: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd`, `Ord`, `Default` ⟨@judgment⟩.

- **S-20: Use newtype wrappers to make distinctions the type system can see.** Prefer `struct UserId(u64)` over `u64` whenever the value has a domain-specific identity ⟨@judgment⟩.

- **S-21: Public structs have private fields.** Expose accessors and (where needed) mutators rather than `pub` fields. Tuple structs and intentionally-transparent types are the documented exception ⟨@judgment⟩.

- **S-22: Functions return values; they do not take out-parameters.** Multiple return values use a tuple or a small struct, not `&mut T` arguments ⟨@judgment⟩.

- **S-23: Implement `Deref` / `DerefMut` only on smart pointers.** Never on domain types that happen to wrap a value. Method-resolution magic on `Deref` is reserved for genuinely pointer-shaped types ⟨@judgment⟩.

- **S-24: No `bool` or `Option<T>` arguments where a domain enum would do.** A function `fn write(path: &Path, append: bool)` becomes `fn write(path: &Path, mode: WriteMode)` with a two-variant enum ⟨@judgment⟩.

- **S-25: Run `rustfmt` before committing.** The Style Guide is what `rustfmt` enforces; if `cargo fmt --check` fails, the file is non-conformant by definition ⟨@tool: rustfmt⟩.

- **S-26: Single-letter generic parameters by default.** `T`, `U`, `K`, `V`, `E`. Multi-letter type parameters require a reason. Lifetimes are single lowercase letters: `'a`, `'b`, `'src` ⟨@judgment⟩.

- **S-27: `where` clauses go on their own line.** Even when a single bound would fit inline, prefer `where` once a function has more than one bound or a generic parameter list of more than two parameters ⟨@tool: rustfmt⟩.

- **S-28: Casts use `as` with surrounding spaces; chained casts stay on one line.** `x as *const u8 as *const c_char`. Break before the *first* `as` when the line overflows; subsequent `as` clauses follow on the same continuation line ⟨@tool: rustfmt⟩.

- **S-29: Method chains break before the dot, never after.** Each link sits on its own block-indented line starting with `.` or `?` ⟨@tool: rustfmt⟩.

- **S-30: `match` arm bodies are either single expressions or block expressions — never bare statements.** A side-effecting arm uses a block: `pat => { do_thing(); }` ⟨@judgment⟩.

- **S-31: Range expressions have no internal spaces.** `0..10`, `..=n`, `start..`. Compound operands are parenthesized: `(x.f)..(x.f.len())` ⟨@tool: rustfmt⟩.

- **S-32: Hexadecimal literals pick one case and stay there.** `0xDEAD_BEEF` and `0xdead_beef` are both legal; mixing within a literal is not. Match what the file already uses ⟨@judgment⟩.

- **S-33: Always specify the ABI on `extern`.** `extern "C" fn` and `unsafe extern "C" { … }`. Bare `extern fn` is not permitted ⟨@tool: clippy⟩.

- **S-34: `let-else` uses the multi-line form unless the else branch is a single short expression.** `let Some(x) = opt else { return };` is fine; anything more goes multi-line with `else {` on the same line as the closing delimiter of the initializer ⟨@tool: rustfmt⟩.

- **S-35: Operator overloads must satisfy the operator's algebraic intuition.** Implement `Add` only when the operation is associative and commutative (or document the deviation prominently). Implement `PartialOrd` consistently with `PartialEq`. Surprising overloads are not permitted ⟨@judgment⟩.

- **S-36: Sealed traits are a deliberate design choice, not a default.** Use a sealed-trait pattern only when downstream impls would break invariants the crate must maintain ⟨@judgment⟩.

- **S-37: Validate function arguments at the boundary, not the body.** Reject invalid input as early as possible and prefer an error variant over a panic for recoverable misuse ⟨@judgment⟩.

- **S-38: Destructors do not fail and do not block.** A `Drop` impl that may panic or block must offer an explicit alternative the caller can run instead ⟨@judgment⟩.

- **S-39: Send + Sync where possible.** Types that have no internal interior mutability bound to a single thread should be `Send + Sync`. Public error types are `Send + Sync + 'static` ⟨@judgment⟩.

[**Exceptions**]

- **EX-1: This SPEC does not redefine commenting and doc-comment rules.** Those live in `COMMENTS.md`. STYLE.md covers code shape; COMMENTS.md covers the prose attached to it.

- **EX-2: No mandatory function-size or file-line limit beyond global coding-style guidance.** "Small" is a judgment call within the line-width cap.

- **EX-3: No project-specific divergence from `rustfmt` defaults.** If `rustfmt` formats it one way, that is the way. Local overrides require an entry in this SPEC justifying the divergence.

- **EX-4: No formatting rules for Cargo manifests, TOML, or YAML.** Out of scope.

- **EX-5: No banned-crates or dependency policy.** Belongs to a future `DEPENDENCIES.md` if needed.

[**Examples**]

**Function with a multi-line signature:**

```rust
pub fn resolve_safe(
    layout: &Layout,
    relative: impl AsRef<Path>,
) -> Result<PathBuf> {
    /* … */
}
```

**Struct with derives and private fields:**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperName {
    raw: String,
}

impl DeveloperName {
    pub fn new(raw: impl Into<String>) -> Result<Self> { /* … */ }
    pub fn as_str(&self) -> &str { &self.raw }
}
```

**Newtype with conversion methods following S-8:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskSlug(String);

impl TaskSlug {
    pub fn as_str(&self) -> &str { &self.0 }     // free borrow → as_*
    pub fn into_inner(self) -> String { self.0 } // owned → into_*
}
```

**Imports, properly grouped and version-sorted:**

```rust
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::error::Result;
use crate::layout::Layout;
```

**Match expression:**

```rust
match outcome {
    Outcome::Ok(value) => handle(value),
    Outcome::Skipped => return Ok(()),
    Outcome::Failed { reason, .. } => {
        log::warn!("skipped: {reason}");
        return Err(Error::Skipped);
    }
}
```

**Method chain breaking before the dot:**

```rust
let foo = source
    .parse()?
    .into_iter()
    .filter(|x| x.is_visible())
    .collect();
```

[**See Also**]

- `LAYOUT.md` — the convention-SPEC layout.
- `COMMENTS.md` — comment and doc-comment rules; STYLE.md governs code shape, COMMENTS.md governs the prose attached to it.
- `ERRORS.md` — error-handling rules; `Result` types and error-variant design follow the E-rules.
- [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) — the `rustfmt` reference.
- [RFC 430 — Finalizing naming conventions](https://rust-lang.github.io/rfcs/0430-finalizing-naming-conventions.html).
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
