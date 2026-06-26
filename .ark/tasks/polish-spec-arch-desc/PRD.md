# `polish-spec-arch-desc` PRD

---

[**What**]

Rewrite the `[**Architecture**]` section guidance in the SPEC and PLAN templates (and their regenerated `.ark/` copies) so it asks for a design picture — components, responsibilities, and data/control flow — instead of defaulting to a bare file list.

[**Why**]

Today the template prompt reads "Module / file layout with a one-line note per file. Prefer a tree or diagram; avoid prose narration." That phrasing steers authors toward a flat `path — responsibility` directory dump, which conveys *where code lives* but not *how the design works* — the actual point of an Architecture section. The richer SPECs (e.g. `ark-context`) already add call graphs and module-coupling notes on their own; the template should make that the expected shape, not an accident.

[**Outcome**]

- SPEC.md and PLAN.md `[**Architecture**]` guidance leads with design intent: name the components/units, their responsibilities, and the data/control flow between them; a file/module map is allowed but secondary and must be paired with the relationships.
- The guidance shows a concrete ASCII component/flow diagram example (no new tooling — mdbook has no mermaid; keep diagrams as fenced text so they stay diffable).
- `spec-extract` one-line guidance updated to match, across all three platform copies (claude/codex/opencode).
- `templates/ark/templates/{SPEC,PLAN}.md` (source, embedded via `include_dir!`) and `.ark/templates/{SPEC,PLAN}.md` (regenerated) stay byte-identical after the edit.
- `cargo build --workspace` still passes (templates are embedded; build must succeed).

[**Related Specs**]

<none — template-prose change only>
