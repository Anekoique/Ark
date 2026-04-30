[**Purpose**]

Defines **Layout A**, the document layout used by every project-level *convention SPEC* under `specs/project/` (e.g. `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`). Convention SPECs collect rules an agent or contributor must follow when working in this repository's source. They differ from *feature SPECs* (under `specs/features/`), which use the runtime-system template (`Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints`). A layout fit for runtime systems leaves the convention-SPEC document with empty slots and an arbitrary Goals/Constraints split; Layout A is the purpose-fit replacement. Modeled on the structure of the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) (a checklist of `C-NAME` rules with rationale) and the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) (formatting rules grouped by topic, each backed by an authoritative reference).

[**Rules**]

- **L-1: Section structure is fixed.** Every convention SPEC contains, in order: `[**Purpose**]`, `[**Rules**]`, `[**Exceptions**]`, optional `[**Examples**]`, optional `[**See Also**]`. No other top-level sections. The `[**Purpose**]` paragraph's last sentence references this file: "Layout: see `specs/project/LAYOUT.md`."

- **L-2: One rule prefix per file.** A SPEC picks a single uppercase letter (`C` for COMMENTS, `S` for STYLE, `E` for ERRORS, `T`, `M`, …) and uses it for every rule in `[**Rules**]`. Rules are numbered sequentially `<PREFIX>-1`, `<PREFIX>-2`, … No sub-prefixes (no `G-`/`NG-`/`C-` split inside one file).

- **L-3: Each rule is a bullet of the form `- **<PREFIX>-<N>: <Title>.** <Body>`.** The body is a single declarative sentence followed by optional rationale and a required citation. Multi-paragraph bodies are permitted; the rule's first sentence is the gate.

- **L-4: Authoritative sources are cited once per SPEC, in `[**Purpose**]` and `[**See Also**]`.** Every convention SPEC opens with a `[**Purpose**]` paragraph that lists its source documents (Rust Book, RFC numbers, API Guidelines pages, crate docs) and closes with a `[**See Also**]` section linking them. Individual rules do not need per-rule citation parentheticals — the rule body states the rule and its rationale; the SPEC-level source list anchors the authority. Project-internal patterns inferred from the codebase are not acceptable as the sole basis for a rule.

- **L-5: `[**Exceptions**]` carries non-requirements and carve-outs.** Items the SPEC explicitly does *not* mandate, plus narrow exemptions to specific rules ("rule X-N does not apply when Y"). The Exceptions section may be empty but its header is still required.

- **L-6: `[**Examples**]` carries worked code or document samples.** Optional. When present, examples illustrate the rules in `[**Rules**]`; they are not normative.

- **L-7: `[**See Also**]` links sibling SPECs and external references.** Optional. One-line bullets pointing to related SPECs (e.g. STYLE.md → COMMENTS.md, ERRORS.md), language references, or RFCs.

- **L-8: A *reference document* is not a *template*.** A reference document (this file, an INDEX.md) describes a convention or format. A template contains placeholder sections intended to be copied and filled in for each instance (the feature-SPEC template under `.ark/templates/`). Reference documents may live anywhere under `specs/project/`; templates live under `.ark/templates/`.

[**Exceptions**]

- **EX-1: `[**Examples**]` and `[**See Also**]` are optional.** A SPEC consisting only of `[**Purpose**]`, `[**Rules**]`, and `[**Exceptions**]` is valid.

- **EX-2: A non-Rust convention SPEC may pick a different prefix family.** L-2 prescribes one prefix per file but does not reserve specific letters; a future `python/STYLE.md` could legitimately use `S-N` and live alongside `rust/STYLE.md`'s `S-N` because each rule is qualified by its file path.

[**Examples**]

A minimal convention SPEC skeleton:

```markdown
[**Purpose**]

One paragraph: what this SPEC governs and the authoritative sources it draws from.
Layout: see `specs/project/LAYOUT.md`.

[**Rules**]

- **X-1: First rule title.** First rule body, ending with a citation (RFC <number>, https://example/, API Guidelines, etc.).

- **X-2: Second rule title.** Body with citation.

[**Exceptions**]

- **NX-1: Carve-out title.** When the rule does not apply.

[**Examples**]

Optional code blocks illustrating the rules.

[**See Also**]

- `sibling/SPEC.md` — one-line description of the relationship.
```

[**See Also**]

- `INDEX.md` — registered list of project SPECs.
- `https://rust-lang.github.io/api-guidelines/` — the API Guidelines book uses an analogous layout (a Checklist of `C-NAME` rules, each with a citation and rationale).
- `https://doc.rust-lang.org/nightly/style-guide/` — the Rust Style Guide is itself a convention SPEC that follows similar conventions in spirit.
