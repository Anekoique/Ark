# `detachable-feature-spec` PRD

---

[**What**]

Allow feature SPECs to live anywhere in a recursive `.ark/specs/features/` tree (mirroring `specs/project/`), with each deep-tier task declaring its SPEC home via a required `[**SPEC Path**]` block in the PRD.

[**Why**]

Today `.ark/specs/features/` is a flat namespace: every deep-tier commit writes to `features/<slug>/SPEC.md` and appends a row to a single `features/INDEX.md`. This breaks down for repositories whose work spans multiple sub-projects.

The motivating case is ProjectX, which is adding a second sub-project (`xvisor`) alongside the existing `xemu`, both sharing infrastructure (`xam`, `xlib`, `xkernels`). With the flat layout:

- A `csr` feature would collide between `xemu` and `xvisor` — same global slug, different scope.
- The features INDEX becomes an unscanable mixed list spanning unrelated sub-projects.
- There is no structural way to ask "show me xvisor's SPECs" — the slug is the only key, and slugs have no grouping.

The fix is structural: let users organize SPECs in a directory tree that mirrors source layout (`features/xemu/csr/SPEC.md`, `features/xvisor/csr/SPEC.md`, `features/klib/SPEC.md`). This is exactly the shape `specs/project/` already supports via recursive `INDEX.md`. The block — `specs/features/` should match `specs/project/` in flexibility.

The non-trivial design question is identification: when `task commit` extracts a deep-tier `## Spec` to disk, *where* in the tree does it go? Today the answer is "always `features/<slug>/`" — a flat decision baked into the CLI. With a recursive tree, the destination must be declared somewhere. The PRD is the natural home: it is written before any code, reviewable during REVIEW, late-bindable across iterations, and already carries scope-level metadata (`[**Related Specs**]`).

[**Outcome**]

1. **Recursive `features/` tree.** `.ark/specs/features/` accepts arbitrary depth. A subdirectory can hold either `SPEC.md` (leaf) or `INDEX.md` + nested subtrees, in the same shape as `specs/project/`. `features/INDEX.md` at the root remains; subtrees gain their own `INDEX.md` files when they contain leaves.

2. **PRD `[**SPEC Path**]` block.** Deep-tier PRD template gains a required `[**SPEC Path**]` block. Its body is a single line: a `/`-separated path **relative to `features/`**, ending in the task slug. Examples: `xemu/csr` → `features/xemu/csr/SPEC.md`; `klib` → `features/klib/SPEC.md`. Quick and standard tiers ignore the block.

3. **`task commit` reads the block on deep tier.** Extraction destination is computed by parsing the *latest* PRD (so iterations can correct placement). Validation: kebab-case segments, no `..` / absolute / empty components, last segment equals `task.toml.slug`. Errors with `FeaturePathMissing` (block absent or empty) or `InvalidFeaturePath { reason }` (bad shape). No silent fallback.

4. **Iterative INDEX upsert.** After SPEC write, `task commit` walks the path from leaf to root: for every intermediate directory it creates `INDEX.md` from a shipped template if missing and upserts a row in the parent's managed block. The root `features/INDEX.md` lists either `<feature>/SPEC.md` leaves or `<area>/INDEX.md` subtrees — same recursive shape as `specs/project/`.

5. **`ark context` surfaces nested SPECs.** `specs.features` in JSON output gains a `path` field carrying the path relative to `features/` (e.g. `"xemu/csr"`); text-mode rendering shows the nested path. `[**Related Specs**]` PRD parser accepts the same nested notation (e.g. `xemu/csr`); bare-slug references continue to resolve at root for backwards compatibility.

6. **No auto-migration.** Existing flat SPECs stay at `features/<slug>/SPEC.md` — they are already valid leaves in the recursive shape. New tasks opt into nesting by writing a multi-segment `[**SPEC Path**]`. Single-segment paths preserve current behavior bit-for-bit.

7. **Tasks tree stays flat.** `.ark/tasks/<slug>/` is unchanged. Tasks are ephemeral and indexed by month on archive; the recursive shape applies only to the durable `specs/features/` layer.

8. **Templates updated.** Ark's deep-tier PRD template ships with the `[**SPEC Path**]` block (with placeholder explaining the format). The shipped `features/INDEX.md` template documents the leaf-or-subtree shape and points at `specs/project/INDEX.md` as the parallel pattern. A new `features/<subtree>/INDEX.md` seed template is added for the auto-created intermediate INDEXes.

9. **Verification.** A deep-tier task creating `features/foo/bar/baz` on commit produces `features/foo/bar/baz/SPEC.md` plus three INDEXes (`features/INDEX.md`, `features/foo/INDEX.md`, `features/foo/bar/INDEX.md`) all carrying the right rows. A second task at `features/foo/qux` reuses `features/foo/INDEX.md` and appends rather than duplicating. A task whose PRD lacks `[**SPEC Path**]` is refused at commit time with `FeaturePathMissing`.

[**Related Specs**]

- `specs/features/ark-agent-namespace/SPEC.md` — `task commit` is the structural mutation point; this task extends what it extracts and where it writes.
- `specs/features/project-spec/SPEC.md` — the recursive `INDEX.md` shape this task imports into `features/` is already proven in `specs/project/`. The constraint language and "leaf or nested index" convention come from there.
- `specs/features/ark-context/SPEC.md` — `ark context`'s features projection gains a `path` field; the PRD parser at `commands/context/related_specs.rs` accepts nested notation.
- `specs/features/ark-workflow-refactor/SPEC.md` — the shipped deep-tier PRD template ships from `templates/ark/templates/PRD.md`; this task adds the `[**SPEC Path**]` block there and updates `workflow.md`'s PRD-authoring section.

[**SPEC Path**]

detachable-feature-spec
