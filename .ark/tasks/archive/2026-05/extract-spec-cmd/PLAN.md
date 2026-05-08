# `extract-spec-cmd` PLAN

> Status: Draft
> Feature: `extract-spec-cmd`
> Iteration: standard (single PLAN, no REVIEW loop)

---

## Summary

Add a brownfield path to the feature-SPEC layer: a new `ark agent spec import` CLI verb that writes a hand-authored SPEC body to `.ark/specs/features/<slug>/SPEC.md`, stamps an "extracted from `<sha>`" CHANGELOG entry, and registers the row in `features/INDEX.md` through the existing managed-block discipline. A multi-platform slash command (`/ark:extract-spec` for claude/opencode, `ark-extract-spec` skill for codex) drives the AI-side discovery → confirm → synthesis flow and feeds the resulting SPEC body to the CLI verb. Net effect: an Ark-adopting project with existing implementations can produce a registered, provenance-stamped feature SPEC without faking a deep-tier task.

## Log `None in 00_PLAN`

---

## Spec

[**Goals**]

- G-1: `ark agent spec import` writes a registered feature SPEC from an authored body.
- G-2: `/ark:extract-spec <name> [hint]` drives discover → confirm → synthesize → import.
- G-3: Extracted SPECs carry a CHANGELOG provenance entry naming the source commit.
- G-4: Brownfield SPEC registration uses the same managed-block path as deep-tier promotion.
- G-5: Slash command ships parity across claude / opencode commands and codex skills.

[**Non-goals**]

- NG-1: No autonomous extraction — the confirm gate is mandatory.
- NG-2: No new INDEX schema; extracted rows reuse existing columns with `from-task = "extracted"`.
- NG-3: No changes to existing `spec extract` (PLAN-based) or `spec register` semantics.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                       (registers `Spec::Import` subcommand variant)
└── ark-core/src/
    ├── error.rs                              (+ Error::SpecAlreadyExists)
    └── commands/agent/spec/
        ├── mod.rs                            (pub mod import; re-export types)
        ├── extract.rs                        (unchanged)
        ├── register.rs                       (extract a pub(crate) fn upsert_index_row)
        └── import.rs                         (new — orchestrates write + CHANGELOG + register)

templates/
├── claude/commands/ark/extract-spec.md       (new)
├── opencode/commands/ark/extract-spec.md     (new — content parity with claude)
└── codex/skills/ark-extract-spec/SKILL.md    (new — skill-shaped wrapper of same flow)

.claude/commands/ark/extract-spec.md          (materialized copy)
.opencode/commands/ark/extract-spec.md        (materialized copy)
.codex/skills/ark-extract-spec/SKILL.md       (materialized copy if .codex/ exists)
```

Module coupling: `import.rs` imports `super::register::upsert_index_row` (newly factored out) and `crate::layout::Layout` for path composition; reads SPEC body via `io::PathExt::read_text`; writes the SPEC file via `io::PathExt::write_text` (or equivalent). No shelling-out to `ark`. `register::register_spec` keeps its public CLI entry point unchanged; `upsert_index_row` is the shared helper both verbs call.

Call graph for `spec import`:

```
spec::import::spec_import(opts)
  ├── validate_feature_slug(opts.feature)               → Error::InvalidSpecField
  ├── validate_scope(opts.scope)                        → Error::InvalidSpecField
  ├── Layout::specs_feature_dir(slug)
  ├── if SPEC.md exists at that dir → Error::SpecAlreadyExists
  ├── PathExt::read_text(opts.from_file)                → body: String
  ├── compose_with_changelog(body, sha, date)           → body′
  ├── PathExt::write_text(spec_path, body′)
  ├── register::upsert_index_row(slug, scope, "extracted", date)
  └── return SpecImportSummary { feature, spec_path, index_row }
```

[**Data Structure**]

```rust
// ark-core/src/commands/agent/spec/import.rs
#[derive(Debug, Clone)]
pub struct SpecImportOptions {
    pub feature: String,         // slug for .ark/specs/features/<slug>/SPEC.md
    pub scope: String,           // one-line description for INDEX row
    pub from_file: PathBuf,      // path to authored SPEC body (≥ Goals/Constraints sections)
    pub from_commit: String,     // git SHA to record in the CHANGELOG provenance entry
    pub date: NaiveDate,         // CHANGELOG + INDEX date (defaults to today UTC)
}

#[derive(Debug, Clone)]
pub struct SpecImportSummary {
    pub feature: String,
    pub spec_path: PathBuf,
    pub index_path: PathBuf,
}

impl Display for SpecImportSummary { /* "imported feature `<slug>`: <spec_path>" */ }

// ark-core/src/error.rs (one new variant)
Error::SpecAlreadyExists { feature: String, path: PathBuf },
```

[**API Surface**]

```rust
// ark-core re-export
pub use commands::agent::spec::{
    SpecImportOptions, SpecImportSummary,
    spec_import,
};

pub fn spec_import(opts: SpecImportOptions) -> Result<SpecImportSummary>;
```

CLI:

```
ark agent spec import \
    --feature <slug> \
    --scope "<one-line>" \
    --from-file <path> \
    --from-commit <sha>      # required; slash command resolves via `git rev-parse --short HEAD`
    [--date YYYY-MM-DD]      # defaults: today UTC
```

`--from-commit` is required at the CLI layer (not defaulted) so the SPEC records the SHA the user *confirmed against*, not whatever HEAD happens to be when the CLI runs. The slash command resolves the short SHA at invocation time and passes it through. This also keeps `ark-core` independent of `io::git`, which is `pub(crate)`.

[**Constraints**]

- C-1: `ark agent spec import` is hidden under `ark agent` (inherits namespace G-1, no semver).
- C-2: `--feature` validated as a slug per existing `Error::InvalidSpecField` rules (no `|`/`\n`/`\r`, non-empty).
- C-3: `--scope` validated identically; INDEX row uses managed-block marker `ARK:FEATURES`.
- C-4: Refuses if `.ark/specs/features/<slug>/SPEC.md` exists → `Error::SpecAlreadyExists`.
- C-5: CHANGELOG entry format: `` - `YYYY-MM-DD` `extracted`: initial extraction from codebase at `<short-sha>`. ``
- C-6: If body has no `[**CHANGELOG**]` section, append one before writing; otherwise insert the entry at section top.
- C-7: INDEX row's `from-task` column is the literal string `extracted` (sentinel; no schema change).
- C-8: All filesystem access routes through `io::PathExt`; all `.ark/`-relative path composition through `layout::Layout`.
- C-9: Output is a single `Display`-rendered line on stdout (one stdout write).
- C-10: `register::upsert_index_row` is shared between `spec register` and `spec import`; behavioral parity is enforced by tests.
- C-11: Slash command refuses to call `spec import` without an explicit user confirmation of the candidate set.
- C-12: Slash command ships in three platform shapes (claude command, opencode command, codex skill) with content parity.

---

## Runtime

[**Main Flow** — slash command]

1. User runs `/ark:extract-spec copy-on-write "memory subsystem COW"`.
2. AI parses args; if `<feature-name>` would collide with an existing `specs/features/<slug>/`, halt with a message pointing at amend-via-deep-tier.
3. AI sweeps in parallel: `git grep` for symbol/file/comment matches, `find docs/ README* CHANGELOG*` for prose mentions, `git log --grep` + `git log -S<symbol>` for introducing commits.
4. AI presents the candidate set (files / symbols / doc sections / 3-5 key commits) and asks the user to trim, add anything missing, and supply a one-line intent.
5. AI reads confirmed sources in full; synthesizes SPEC body matching `.ark/templates/SPEC.md` (Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints).
6. AI writes the body to a temp file; runs `ark agent spec import --feature <slug> --scope "<intent>" --from-file <tmp> --from-commit $(git rev-parse --short HEAD)`.
7. CLI returns one-line summary; AI surfaces it to the user with next-step guidance (review the SPEC, commit when satisfied — extracted SPECs are not part of an Ark task, so the user commits manually).

[**Main Flow** — `spec import`]

1. Parse + validate `--feature`, `--scope`.
2. Compute SPEC path via `Layout::specs_feature_dir(feature).join("SPEC.md")`.
3. If SPEC path exists → return `Error::SpecAlreadyExists`.
4. Read body from `--from-file`.
5. Validate `--from-commit` (required, sanitized via `sanitize_table_field`).
6. Resolve `--date` (default: today UTC).
7. Compose CHANGELOG entry; insert into body (append section if missing).
8. Create parent dir; write composed body.
9. Call `register::upsert_index_row(slug=feature, scope, from_task="extracted", date)`.
10. Print `SpecImportSummary` Display line.

[**Failure Flow**]

1. Invalid slug / scope → `Error::InvalidSpecField`. No filesystem mutation.
2. SPEC path already exists → `Error::SpecAlreadyExists`. No filesystem mutation.
3. `--from-file` missing or unreadable → `Error::Io { path, source }` (existing variant).
4. SPEC body write succeeds but `upsert_index_row` fails → SPEC file is left in place; error surfaces. (Cleanup is the user's call — leaving the file makes a retry idempotent because step 3 will refuse on second invocation, forcing the user to delete and re-run; documented in the error message.)
5. Slash command: user declines confirmation → halt with no CLI invocation.

[**State Transitions**]

- No SPEC for `<slug>` → after `spec import` → SPEC present + INDEX row present (atomic at the step level; not git-atomic — user commits manually).

---

## Implementation

[**Phase 1: factor `upsert_index_row` out of `spec register`**]

- Move INDEX-row composition + managed-block update from `register::spec_register` into a `pub(crate) fn upsert_index_row(slug, scope, from_task, date) -> Result<PathBuf>` in `register.rs`.
- `spec_register` becomes a thin wrapper: validate, then call `upsert_index_row`.
- Verify existing `spec register` integration tests still pass with no behavior change.

[**Phase 2: implement `spec_import` library function**]

- Add `commands/agent/spec/import.rs` with `SpecImportOptions`, `SpecImportSummary`, `spec_import`.
- Add `Error::SpecAlreadyExists` variant per E-15 (carries `feature: String, path: PathBuf`).
- Compose CHANGELOG insertion as a small string-handling helper; cover both "section absent" and "section present" cases.
- Re-export from `commands/agent/spec/mod.rs` and `ark-core/src/lib.rs`.

[**Phase 3: wire `spec import` into the CLI**]

- Add `Spec::Import(SpecImportArgs)` variant in `ark-cli`'s spec subcommand enum (clap derive).
- `--from-commit` is required at the CLI layer; the slash command resolves the short SHA before invoking the verb.
- Default `--date` to today UTC.
- `Display` summary per C-9.

[**Phase 4: ship the slash command across platforms**]

- `templates/claude/commands/ark/extract-spec.md` — full flow doc with confirm gate.
- `templates/opencode/commands/ark/extract-spec.md` — content-parity copy (different frontmatter shape if needed).
- `templates/codex/skills/ark-extract-spec/SKILL.md` — skill-shaped wrapper; same body.
- Materialized copies: `.claude/commands/ark/extract-spec.md`, `.opencode/commands/ark/extract-spec.md`, `.codex/skills/ark-extract-spec/SKILL.md` (regenerated from templates per existing init/upgrade pipeline).
- Verify `ark upgrade` picks up the new template entries (no manual reconciliation).

[**Phase 5: dogfood smoke test**]

- In a throwaway branch on this repo, run `/ark:extract-spec spec-extract "PLAN-based feature SPEC promotion"` against the existing `commands/agent/spec/extract.rs` module.
- Verify: SPEC file created, INDEX row added with `from-task = "extracted"`, CHANGELOG entry references current HEAD.
- Discard via `rm -rf .ark/specs/features/spec-extract && git checkout .ark/specs/features/INDEX.md` (or hand-revert the row).
- Document the smoke result in VERIFY.md as the integration evidence for G-1, G-2, G-3, G-4.

---

## Trade-offs

- T-1: New CLI verb (`spec import`) vs extending `spec extract` with `--from-codebase`. Chose new verb: existing `spec extract` reads a PLAN file; brownfield extraction has no PLAN. Two unrelated input paths in one command violates single-responsibility; two verbs each with one job is cleaner. Cost: one more line in `--help`.
- T-2: `from-task = "extracted"` sentinel vs INDEX schema change. Chose sentinel: zero schema change, no INDEX migration, trivial filter for tooling that wants to distinguish (`row.from_task == "extracted"`). Cost: a magic string. Alternative was an `extracted: bool` column or splitting INDEX into two managed blocks — both heavier than the value.
- T-3: Confirm gate mandatory vs `--yes` opt-out. Chose mandatory: the confirm step *is* the value of brownfield extraction. Without it the AI tends to write SPECs that describe the codebase rather than the feature. Cost: one extra interaction. Power users who already know the candidate set are not the target audience for this command.
- T-4: SPEC body via `--from-file` vs stdin. Chose file: simpler in a multi-platform skill (avoids shell-quoting heredocs across claude/opencode/codex). Cost: a temp file. Stdin can be added later if a real use case appears.
- T-5: Atomic SPEC + INDEX vs separate steps. Chose separate (SPEC written first, then INDEX upsert). On a failure-after-write, the SPEC file remains; second invocation refuses with `SpecAlreadyExists`, forcing the user to clean up and retry. A two-phase commit (write to tempfile, rename SPEC, then upsert INDEX, with rollback on INDEX failure) is more correct but heavier; the current path leaves recovery to the user, who can `rm` the SPEC and retry. Acceptable for a brownfield one-shot tool — revisit if real-world failures show up.

---

## Validation

[**Unit Tests**]

- V-UT-1: `compose_changelog_entry` inserts into a body with no `[**CHANGELOG**]` section by appending the section.
- V-UT-2: `compose_changelog_entry` inserts into a body with an existing section by prepending the entry inside the section.
- V-UT-3: `validate_feature_slug` rejects empty / `|` / `\n` / `\r`-bearing strings (re-uses existing validator).
- V-UT-4: `SpecImportSummary::Display` renders a single line with feature + spec_path.

[**Integration Tests**]

- V-IT-1: `spec import` on a clean repo writes `specs/features/<slug>/SPEC.md` with the CHANGELOG entry at the expected position.
- V-IT-2: `spec import` upserts an INDEX row with `from-task = "extracted"` via the managed-block path.
- V-IT-3: `spec import` followed by `spec register --feature <other-slug>` leaves the prior `extracted` row untouched (`upsert_index_row` parity).
- V-IT-4: `ark upgrade` on an installed project picks up the new slash command + skill templates.

[**Failure / Robustness**]

- V-F-1: `spec import` against a path that already has a SPEC returns `Error::SpecAlreadyExists` and writes nothing.
- V-F-2: `spec import` with an unreadable `--from-file` returns `Error::Io { path, source }` with the file path in the variant.
- V-F-3: `spec import` with an empty `--scope` returns `Error::InvalidSpecField`.
- V-F-4: SPEC body authored without a `[**CHANGELOG**]` section is composed correctly (appends section).

[**Edge Cases**]

- V-E-1: `--from-commit` is required by clap; missing flag produces clap's standard "required argument" error before any logic runs.
- V-E-2: Feature slug already in INDEX but SPEC file missing (corrupt prior state) — `spec import` writes the SPEC and `upsert_index_row` updates the existing row idempotently rather than duplicating.
- V-E-3: SPEC body containing a `## Spec` H2 (a PLAN snippet pasted in) imports as-is — body content is the user's responsibility; CLI does not parse beyond inserting CHANGELOG.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1, V-IT-2, V-F-1 |
| G-2 | V-IT-1 (smoke test in Phase 5; covered by integration evidence in VERIFY) |
| G-3 | V-UT-1, V-UT-2, V-IT-1 |
| G-4 | V-IT-2, V-IT-3 |
| G-5 | V-IT-4 |
| C-2 | V-UT-3, V-F-3 |
| C-3 | V-F-3 |
| C-4 | V-F-1 |
| C-5 | V-IT-1 |
| C-6 | V-UT-1, V-UT-2, V-F-4 |
| C-7 | V-IT-2 |
| C-9 | V-UT-4 |
| C-10 | V-IT-3 |
| C-12 | V-IT-4 |
