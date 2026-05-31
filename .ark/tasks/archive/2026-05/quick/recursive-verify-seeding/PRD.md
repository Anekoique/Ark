# `recursive-verify-seeding` PRD

---

[**What**]

Replace the flat per-leaf `Project Spec Compliance` checklist in seeded VERIFY documents with a recursive walk that emits two subsections: `Index integrity` (one PENDING per discovered `INDEX.md`) and `Leaf SPECs` (one rolled-up PENDING plus a traceability sublist of every leaf SPEC).

[**Why**]

`specs/project/INDEX.md` may now reference nested `INDEX.md` files (recursive layout). The current seeder reads only the top-level INDEX rows and projects each as a single bullet, so VERIFY ends up listing every leaf SPEC by name — verbose, hierarchy-blind, and silent on the new "is the index correct?" failure mode. Two-subsection layout matches the recursive design: indexes are checked individually for completeness, leaves are checked en bloc for `LAYOUT.md` conformance.

[**Outcome**]

1. `read_project_spec_tree` walks `specs/project/INDEX.md` recursively, classifying each row as nested-INDEX (first-cell path ends in `INDEX.md`) or leaf, with a visited-set cycle guard. Missing referenced files are skipped silently.
2. `SeedInputs.project_specs` is replaced by a `ProjectSpecTree { indexes, leaves }` whose paths are stored relative to `specs/project/`.
3. The seeded `Project Spec Compliance` section renders as two subsections: `### Index integrity` (one `- [ ] \`<rel-path>\` enumerates all children of \`<dir>/\`: PENDING` per discovered INDEX) and `### Leaf SPECs` (one `- [ ] All leaf SPECs under \`specs/project/\` conform to \`LAYOUT.md\`: PENDING` followed by a nested sublist of leaf paths). Empty tree keeps the existing `(none registered): N/A` placeholder.
4. Both call sites (`commands/agent/task/phase.rs`, `commands/upgrade/verify_migration.rs`) compile against the new shape.
5. `.ark/templates/VERIFY.md` comment above `{{PROJECT_SPEC_COMPLIANCE}}` describes the recursive walk and two-subsection layout.
6. `cargo test -p ark-core` passes. New tests cover: nested-INDEX walk, cycle guard, missing-file skip, two-subsection rendering, empty-tree placeholder.

[**Related Specs**]

<empty — touches internal seeding code only; no feature SPEC encodes the VERIFY rendering shape>
