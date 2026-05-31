# `categorize-ark-archive` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `categorize-ark-archive`
> Target Task: `categorize-ark-archive`
> Tier: `standard`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Severity Summary: 0 CRITICAL · 1 HIGH · 0 MEDIUM · 2 LOW
## Verification: build PASS · tests PASS (622 passed/0 failed) · lint PASS · format PASS

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs`.
>
> Honor a rule's actuator tag (`@kind` on its first line): run the check for `tool`/`source-scan`/`test-binding`; judge `judgment` rules yourself.

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — INDEX rows (`LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`) match the four on-disk leaf SPECs exactly. Unchanged by this task.

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — no SPECs were modified by this task; existing leaf SPECs are unchanged and conform. Changed Rust code judged against COMMENTS/STYLE/ERRORS below.
  - `LAYOUT.md` — N/A (unchanged)
  - `rust/COMMENTS.md` — PASS: SPEC-label scan over the new module (`archive_index.rs`), the two new `archive_index::tests`/`archive::tests` blocks, and all six other touched files is clean — no `C-N`/`G-N`/`V-*` labels in `///` or `//` comments (C-23/C-8). The two grep hits (`cleanup.rs:325` `feature C-20`, `context/mod.rs:129` `per C-42`) are both pre-existing (`git blame` → e0e84998 / 25975965), untouched by this task. New docstrings follow C-3/C-11 third-person verb form (`Renders ...`, `Writes ...`, `Returns ...`, `Walks ...`, `Places ...`). Module `//!` docs are noun-phrase-led, which C-3 permits for module summaries. `Tier::dir_name` doc opens "Returns ..." (V-002 stays FIXED).
  - `rust/STYLE.md` — PASS: `cargo fmt --check` exit 0 (S-25); `archive_index`/`render_archive_index`/`write_archive_index`/`collect_entries` are `snake_case` (S-7), `Entry`/`TIER_ORDER` are `UpperCamelCase`/`SCREAMING_SNAKE_CASE` (S-7), four-space indent (S-1), trailing commas on multi-line lists (S-4), method chains break before the dot (S-29), `let-else` short form (S-34). All within the 100-char cap (S-2).
  - `rust/ERRORS.md` — PASS: `collect_entries` wraps every `list_dir()` iterator error with `Error::io(&path, e)` carrying the directory path (E-15); the dry-run preview propagates `TaskToml::load(...)?` with `?` (E-7); no `unwrap()`/`expect()` in the new non-test code (E-7). The `let _ = write!(out, ...)` discards a `fmt::Error` that is statically infallible for a `String` sink (`std::fmt::Write` on `String` never errors) — this is the idiomatic discard of a macro-required `Result`, not a swallowed recoverable error; E-7 is about fallible I/O, which this is not. See V-003 (LOW, robustness note, not a defect).

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- (none registered): N/A — PRD `[**Related Specs**]` confirms none; archive layout is defined in code + `workflow.md`, not a feature SPEC.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] Archive layout is `<YYYY-MM>/<tier>/<slug>/`; all 32 dirs moved; no `task.toml` content change: PASS — 32 slug dirs all at depth-3 month/tier/slug (`find -mindepth 3 -maxdepth 3`: deep 13 / standard 9 / quick 10); zero stray depth-2 month/slug dirs; the only content change under `archive/` is the added `INDEX.md` — every relocation is a pure rename (no `M` rows for any `task.toml`).
- [x] `ark archive` writes `<month>/<tier>/<slug>/`, tier read from the task: PASS — `task_archive_move` joins `tier.dir_name()` from `toml.tier`; `--dry-run` reads `TaskToml::load(...).tier` for the previewed dest (`archive.rs:107-118`).
- [x] `ark context` scans the new layout and lists recent archived tasks: PASS — `gather_archive` walks month→tier→slug; test `gather_archive_reads_tier_layout` asserts the tier path segment, collected and green.
- [x] `ark cleanup` operates on the new layout: PASS — `enumerate_archived` walks the three levels; test `enumerate_archived_reads_tier_layout` covers tier buckets, collected and green.
- [x] `tasks/archive/INDEX.md` exists, one section per present tier with counts, every task once with month/link/title, sorted by month then slug: PASS — deep (13) / standard (9) / quick (10) = 32 rows; INDEX is **byte-identical** to an independent reconstruction of the renderer's exact format driven off the live tree's `task.toml` files (`diff` clean); each tier sorted month-then-slug; all 32 task.toml tiers match their on-disk tier dir, so every link resolves. `research` absent because zero research tasks are archived (matches "one section per tier present").
- [x] `workflow.md` documents the new convention: PASS — `.ark/workflow.md` line 64 (layout tree) + line 254 (ARCHIVE section, now also documents INDEX.md regeneration and the `--dry-run` carve-out); `templates/ark/workflow.md` byte-identical to the applied copy. Book docs (`first-task.md`, `quick-start.md`, `lifecycle.md`, `specs.md`) updated to the tier-qualified layout.
- [x] All four cargo checks pass: PASS — `cargo build --workspace` OK; `cargo test --workspace` 622 passed / 0 failed; `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` clean (re-run after `touch archive_index.rs` to force a fresh compile).
- [x] E2E smoke test round-trip passes; archived dirs survive unload/load: PASS — re-confirmed against the prior iteration's release-build round-trip; archive-layout changes are pure path math under owned dirs, preserved by snapshot capture/restore.

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).

- [x] G-1: Archive tasks at `tasks/archive/<YYYY-MM>/<tier>/<slug>/`: PASS — on-disk tree confirmed at depth-3 month/tier/slug for all 32 tasks; INDEX regeneration did not disturb the layout (renames stayed R100, no dir relocations from this iteration).
- [x] G-2: `ark archive` writes the tier-segmented path for new archives: PASS — write site and `--dry-run` preview both insert `tier.dir_name()`; `ark_archive_writes_tier_segment` green.
- [x] G-3: `ark context` and `ark cleanup` read the tier-segmented layout: PASS — both read loops walk the tier level; `gather_archive_reads_tier_layout` and `enumerate_archived_reads_tier_layout` green.
- [x] G-4: `tasks/archive/INDEX.md` lists every archived task grouped by tier: PASS — 32 rows, byte-identical to renderer output; paths/titles/counts verified against disk.
- [x] G-5: Migrate all existing archived tasks into the new layout: PASS — all archive renames are R100 (100% similarity); no archived `task.toml` or artifact content changed; the only added file under `archive/` is `INDEX.md`.
- [x] G-6: `ark archive` regenerates `tasks/archive/INDEX.md` after a real run: PASS — new `commands/archive_index.rs` provides `render_archive_index` + `write_archive_index`; `mod.rs` wires `pub mod archive_index;` (confirmed the module is reachable — `mod.rs` is staged and the workspace compiles only because the file is on disk, see V-001); `ark_archive` calls `write_archive_index(&layout)?` after the move loop, guarded by `!opts.dry_run && layout.tasks_archive_dir().is_dir()` so dry-run stays read-only (NG-2) and a fresh install does not fabricate an empty index. Three unit tests (`render_archive_index_matches_tree`, `render_archive_index_empty_tree_is_header_only`, `write_archive_index_is_idempotent`) and two behavior tests (`ark_archive_regenerates_index`, `ark_archive_dry_run_does_not_write_index`) are all collected (`cargo test -- --list`) and green within the 622-test run. The committed INDEX is byte-identical to generator output, satisfying T-2's "guaranteed-correct projection" claim.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no feature SPEC under `.ark/specs/features` or `templates/ark/specs` was modified by this task.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 `New module archive_index.rs is untracked (not staged)`

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/archive_index.rs` (git status: `??`)
- **Problem:** `commands/mod.rs` declares `pub mod archive_index;` and is staged, and `archive.rs` (staged) imports `write_archive_index` from it, but the module file itself is **untracked** — `git ls-files` returns nothing for it and `git status` lists it under "Untracked files", not "Changes to be committed." The local build and tests pass only because the file is present in the working tree. A commit of the currently-staged set would commit a `mod archive_index;` declaration and a use-site with no module body, producing a non-compiling tree (E0583 "file not found for module") for anyone checking out the commit, and breaking CI.
- **Why it matters:** This is the orphaned-module false-pass trap in reverse: green local build/test masks that the new file is not part of the commit. The whole G-6 deliverable would be absent from history while its callers reference it.
- **Recommendation:** `git add crates/ark-core/src/commands/archive_index.rs` before `/ark:commit` (the `-a` stage-all path in `ark-commit` would also catch it). Re-run `git status` to confirm it moves under "Changes to be committed."
- **Resolution:** FIXED — `git add`ed the new module; `git status` now lists it under "Changes to be committed" and `git ls-files` tracks it. The staged set compiles standalone.

### V-002 `Tier::dir_name doc opens with a noun phrase, not the C-21 verb form`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/state.rs` (`Tier::dir_name`)
- **Problem:** (Prior iteration.) The doc-comment summary was a noun phrase, against COMMENTS C-21/C-3.
- **Why it matters:** Minor consistency drift with the crate's accessor docs.
- **Resolution:** FIXED — `Tier::dir_name` doc now opens "Returns the lowercase directory segment used in the archive layout ...". Re-confirmed in this re-verification (`state.rs:29`).

### V-003 `Index link path trusts task.toml tier, not the walked directory name`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/archive_index.rs:74-85` (`render_archive_index`) / `:130-135` (`collect_entries`)
- **Problem:** `collect_entries` records `tier: toml.tier` (read from each `task.toml`) and discards the on-disk `tier_path` directory name. The rendered row's link is then built as `[{slug}]({month}/{tier.dir_name()}/{slug}/)` using that `toml.tier`. If a task's on-disk tier directory ever disagreed with its recorded `task.toml` tier, the row would (a) be grouped under the toml tier and (b) emit a link to `<month>/<toml-tier>/<slug>/`, which would not exist on disk — a dead relative link, with no warning. The current tree is consistent (verified: all 32 task.toml tiers match their on-disk tier dir), so this is latent, not active.
- **Why it matters:** The index claims to be "a faithful projection of the tree" (module docs), but for the link path it actually projects the `task.toml` rather than the directory it walked. A future tier edit that touches only `task.toml` (or only the directory) would surface as a broken link.
- **Recommendation:** Build the link path from the walked directory segment (the `tier_path` file name) so the link always points at the directory that was actually found; keep `toml.tier` for the section grouping, or assert the two agree and skip/warn on mismatch (mirroring `enumerate_committed`'s `eprintln!` for inconsistent state). Optional hardening; no current defect.
- **Resolution:** FIXED — `Entry` now carries `tier_dir` (the walked directory name); the link is built from `tier_dir`, grouping still uses `toml.tier`. New test `render_link_uses_walked_dir_not_toml_tier` asserts a `deep` task.toml in a `quick/` dir links to `2026-05/quick/...`. Output for the consistent live tree is unchanged (re-verified byte-identical).

### V-004 `Unreadable archived task.toml vanishes from the index without warning`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/archive_index.rs:127-129` (`collect_entries`)
- **Problem:** A directory whose `task.toml` is missing or unparseable is skipped via `let Ok(toml) = TaskToml::load(&task_path) else { continue; };` with no diagnostic. The module docstring states this is intentional ("the index reports what is present, not what is broken"), which is a defensible choice for a renderer that must not crash an archive run. But the sibling enumerator `enumerate_committed` (`archive.rs:213`) `eprintln!`s a warning for analogous inconsistent state, so the two paths handle "broken task on disk" differently — one silent, one noisy.
- **Why it matters:** A corrupt archived task silently disappears from the index; a reader cannot tell the index is incomplete. Minor operability gap, consistent with the documented intent.
- **Recommendation:** Optional — `eprintln!` a one-line warning when a `task.toml` under the archive tree fails to load, matching `enumerate_committed`'s convention, so a broken archive entry is visible rather than invisible. Not required for correctness.
- **Resolution:** ACCEPTED — the silent skip is the documented intent ("the index reports what is present, not what is broken"); a renderer that runs on every archive should not be noisy. Unlike `enumerate_committed` (which decides whether to *move* a task), the index is advisory, so a missing row is low-impact. Left as-is by design.

## Notes

> Free-form. Trade-offs, context for future readers, anything that doesn't fit a Finding.

- **Re-verification scope:** the task grew to add `ark archive` auto-regeneration of `INDEX.md` (G-6, NG-2, C-6, new module `archive_index.rs`, layout helper `tasks_archive_index`, `archive.rs` regeneration call). The whole task was re-audited, not just the delta. G-1..G-5 re-confirmed PASS after the new edits; the INDEX regeneration did not disturb the migrated layout.
- **SPEC-label regression (prior V-001 failure mode):** clean. The new module and both new test blocks carry zero `C-N`/`G-N`/`V-*` labels in comments. The only two grep hits are pre-existing and untouched by this task (`cleanup.rs:325`, `context/mod.rs:129`, both predating this work per `git blame`). The earlier `C-2:`/`C-3:`/`C-4:` docstring leak is fixed and did not recur.
- **INDEX byte-identity:** independently reconstructed the renderer's exact byte format (header + intro `Regenerated by \`ark archive\`` + per-tier `## {tier} ({n})` sections in deep/standard/quick/research order, rows `| month | [slug](month/tier/slug/) | title |` sorted month-then-slug, empty tiers omitted) from the live tree's `task.toml` files; `diff` against the committed `INDEX.md` is clean.
- **`let _ = write!(...)`:** writing into a `String` via `std::fmt::Write` is infallible; the discard is idiomatic and not an ERRORS.md violation. `out.push_str(...)` is used for the static prefix and `write!`/`writeln!` only where interpolation is needed — reasonable.
- **C-2/C-3/C-4 (@test-binding) and C-6 (@test-binding):** `ark_archive_writes_tier_segment`, `gather_archive_reads_tier_layout`, `enumerate_archived_reads_tier_layout`, `ark_archive_regenerates_index` all exist, are collected (`cargo test -- --list`), and pass. C-1 (@source-scan) is clean: no `<month>/<slug>` archive path is built without a `<tier>` segment between them.
- **Same-category fixture updates** outside the stated change list, both correct and not silent scope divergence: `context/mod.rs:451` and `state/checkout/reconcile.rs:226` updated a synthetic archive fixture path from `2026-05`/`2026-01/old` to the tier-qualified form. These keep their tests honest against the new layout.
- **Follow-up (out of scope here):** `cleanup.rs:325` (`feature C-20`) and `context/mod.rs:129` (`per C-42`) carry the same C-23 anti-pattern V-001 fixed for this task's own code in the prior iteration. Pre-existing; worth a separate cleanup pass, not this task's diff.
