# `ark-context` PLAN `01`

> Status: Revised
> Feature: `ark-context`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 01 resolves the three HIGH findings from `00_REVIEW.md` and absorbs all MEDIUM / LOW findings that had concrete recommendations. The two structural revisions are:

1. **`.claude/settings.json` is not hash-tracked.** It is handled analogously to `CLAUDE.md`'s managed block (upgrade SPEC C-8) — a new helper `update_settings_hook` is re-applied on every `init` / `load` / `upgrade`; on-disk body is never hashed. For `unload` round-trip, `Snapshot` gains a `hook_bodies` slot that captures the Ark-owned `SessionStart` entry by identity key. This resolves R-001 and R-002 with one cohesive mechanism.
2. **Projection filter, artifact iteration rule, cwd discovery, and parser grammars are made explicit and testable.** Every MEDIUM / LOW with a concrete recommendation becomes a new Constraint or pinned Spec clause.

The forward-looking `merge_json_managed` helper is dropped in favor of the thinner single-purpose `update_settings_hook` / `remove_settings_hook` pair — YAGNI, no second consumer in scope.

## Log

[**Added**]

- **C-17:** `.claude/settings.json` is **not hash-tracked.** The Ark-owned `SessionStart` hook entry is re-applied on every successful `init`, `load`, and `upgrade` via `update_settings_hook`, mirroring the `CLAUDE.md` managed-block treatment (upgrade SPEC C-8). The file is not listed in `manifest.files` and has no entry in `manifest.hashes`. User-owned siblings (other `hooks.*` keys, unrelated top-level keys) are preserved byte-identically where JSON object-key order is stable.
- **C-18:** `Snapshot` gains a `hook_bodies: Vec<SnapshotHookBody>` slot. `unload` captures the Ark-owned `SessionStart` entry (matched by `command == ARK_CONTEXT_HOOK_COMMAND`) into this slot. `load` restores it by re-applying `update_settings_hook` after scaffolding. User-edits to *unrelated* hook entries are NOT captured by `hook_bodies` and do NOT survive a round-trip — they are user-owned content outside Ark's owned dirs. Documented trade-off; matches the existing rule that `.claude/` (apart from `commands/ark/`) is user space.
- **C-19 (Artifact iteration rule):** `gather_context` emits all files matching `^(\d{2})_PLAN\.md$` and `^(\d{2})_REVIEW\.md$` in the current task dir, sorted ascending by parsed `NN`. `ArtifactKind` gains a helper `pub fn iteration(&self) -> Option<u32>`. Projections that need "latest" call `artifacts.iter().filter(...).max_by_key(|a| a.kind.iteration())`. Deterministic ordering makes `TextSummary` golden fixtures stable.
- **C-20 (Related-specs parser + projection filter grammar):** Implemented in `related_specs.rs`:
  > Locate the line starting with `[**Related Specs**]`. Scan forward until the next line matching `^\[\*\*.*\*\*\]` or EOF. Inside that range, extract every token matching `specs/features/[a-z0-9_-]+/SPEC\.md` (case-sensitive). Dedupe preserving first-seen order. Return `Vec<String>`. Empty / missing section → empty vec, no error.
  The `specs.features` projection filter (R-003): a `SpecRow` `f` is kept iff any `r ∈ current_task.related_specs` satisfies `normalize(r).ends_with(&normalize(f.path))`, where `normalize` strips leading `./` and leading `.ark/`.
- **C-21 (cwd discovery):** New `Layout::discover_from(cwd) -> Result<Self>` walks ancestors of `cwd` until `.ark/` is found, else returns `Error::NotLoaded { path: cwd }`. `TargetArgs::resolve` becomes "absolutize then discover (unless `--dir` is set, in which case `--dir` is the explicit root)". Applies uniformly to `init`, `load`, `unload`, `remove`, `upgrade`, and `context` to keep cwd semantics consistent.
- **C-22 (Git helper):** New module `ark-core/src/io/git.rs` exposing `run_git(args: &[&str], cwd: &Path) -> Result<GitOutput>` where `GitOutput = { exit_code: i32, stdout: String, stderr: String }`. Non-zero exit returns `Ok(GitOutput { exit_code: n, .. })` (callers soft-fail on non-git dirs). Spawn failure returns `Error::GitSpawn { source }`. Replaces three raw `Command::new("git")` calls in `gather.rs`.
- **C-23 (JSON output shape lock):** JSON mode uses `serde_json::to_writer_pretty(stdout, &projected)` followed by `stdout.write_all(b"\n")`. Indent is 2 spaces. Field order follows `Serialize` derive order in `ProjectedContext`. Text mode emits a trailing `\n`. Both asserted by byte-level tests.
- **C-24 (Specs-INDEX parser grammar):**
  - `specs/features/INDEX.md`: parsed via `read_managed_block(path, "ARK:FEATURES")` → body; body parsed as a GFM table where row matches `r"^\|([^|]*)\|([^|]*)\|([^|]*)\|?$"`; cells trimmed; lines failing to match, lines whose first cell equals `Feature`, and separator lines matching `^-+$` are skipped.
  - `specs/project/INDEX.md`: no managed block; locate first line matching `^##\s+Index\b`, then scan for first GFM table (first line matching `^\s*\|`); table terminates at first line not starting with `\|`; same cell parse and skip rules.
- **C-25 (Dirty files cap):** Named constant `DIRTY_FILES_CAP: usize = 20` in `model.rs`, referenced by `gather.rs`. Tests assert against the constant symbolically.
- **C-26 (Process-spawn locality):** C-4 (filesystem access via `PathExt`/`io::fs`) is unchanged; in addition, `std::process::Command` may be spawned **only** from inside `io/git.rs`. No other `commands/` module spawns processes. Enforced by code review + manual grep on PRs.

- **V-UT-20:** `update_settings_hook` on a missing/empty file produces `{"hooks":{"SessionStart":[<entry>]}}` with the Ark entry; called twice produces byte-identical output (idempotence).
- **V-UT-21:** `update_settings_hook` on a file with a user-authored `PreToolUse` entry preserves that entry verbatim; only the Ark `SessionStart` entry is appended/updated.
- **V-UT-22:** `update_settings_hook` invoked when the user has modified the Ark entry (changed the `command` string) **overwrites** the entry back to canonical form. Documented behavior; matches `CLAUDE.md` precedent. Users wanting custom commands should add siblings, not edit the Ark entry.
- **V-UT-23:** Related-specs parser (C-20) on a fixture PRD with two valid `specs/features/<name>/SPEC.md` bullets inside `[**Related Specs**]`, a stray path outside the section, and a malformed-slug path inside — parser returns exactly the two valid paths.
- **V-UT-24:** Projection filter (C-20 second half) — `ctx.specs.features = [foo, bar, baz]`, `current_task.related_specs = ["specs/features/foo/SPEC.md"]`; `project(ctx, Scope::Phase(Plan))` returns `specs.features = [foo]` only.
- **V-UT-25:** `Layout::discover_from` walks ancestors; tests for (i) cwd == project root, (ii) cwd == 3 levels deep, (iii) cwd with no `.ark/` ancestor → `Error::NotLoaded`.
- **V-UT-26:** `ArtifactKind::iteration()` returns `Some(n)` for `Plan{n}`/`Review{n}`, `None` for `Prd`/`Verify`/`TaskToml`.
- **V-UT-27:** `io::git::run_git` on a non-git tempdir returns `Ok(GitOutput { exit_code != 0, .. })`. With `args = ["--not-a-command"]` inside a git repo: `Ok(GitOutput { exit_code != 0, .. })`.
- **V-IT-12:** Snapshot round-trip with `Snapshot::hook_bodies`: `ark init` → `ark unload` → snapshot contains the Ark hook entry → `ark load` → `.claude/settings.json` contains the Ark `SessionStart` entry.
- **V-IT-13:** Upgrade re-adds a deleted Ark hook entry: `ark init` → user deletes the Ark `SessionStart` entry → `ark upgrade` → entry is re-added (no prompt, no hash check).

[**Changed**]

- **G-8 revised:** A `SessionStart` hook is installed into `.claude/settings.json` via `update_settings_hook(path, entry)` at `ark init` (scaffold path), `ark load` (after restore), and `ark upgrade` (unconditionally, similar to `CLAUDE.md` block re-application). The file is **not** added to the embedded template tree and **not** listed in `manifest.files`. Idempotent; preserves sibling hooks and unrelated keys.
- **G-9 removed:** No longer applicable (the file is not in the template tree). Replaced by C-17.
- **G-11 revised:** End-to-end round-trip asserts: (i) `ark init` writes Ark hook, (ii) `ark unload` captures it into `Snapshot::hook_bodies`, (iii) `ark load` re-applies via `update_settings_hook`, (iv) `ark remove` removes Ark entry via `remove_settings_hook` while leaving sibling user hooks intact, (v) `ark upgrade` re-applies unconditionally.
- **G-7 revised (R-003):** Replaces "filtered to `current_task.related_specs` ∪ project specs" with the explicit predicate from C-20:
  > `plan` / `review` projection: `specs = Some(SpecsState { project: <ctx.specs.project unchanged>, features: <filtered> })`. Filter per C-20 second half. Empty `related_specs` → `features: []`.
- **C-3 revised (R-006):** Dropped the text-mode schema-header clause. Text output carries no schema version. C-10 stands authoritative. C-3 now reads: "JSON output's first field is `\"schema\": 1`. Asserted by a byte-level unit test."
- **T-3 revised:** Replaced the three-option formulation with a single adopted choice (exempt from hash tracking). See Trade-offs §T-3.
- **Architecture revised:** `commands/context/` gains no `merge_json.rs` submodule. Instead `io/fs.rs` grows the `update_settings_hook` / `remove_settings_hook` pair, co-located with the existing `update_managed_block` family.
- **Data Structure revised:** `Error::ContextProjectionMismatch` is **removed** per R-009 — invariant checks use `debug_assert!`. `Error::GitSpawn { source: std::io::Error }` added per C-22.
- **Templates dir change:** `templates/claude/settings.json` is **NOT** created in this task. The file only exists on disk in host projects after `update_settings_hook` is called — consistent with `CLAUDE.md`, which has no template file in the embedded tree either.

[**Removed**]

- `merge_json_managed(path, pointer, identity_key, value)` — replaced by the named pair. The general pointer-based helper had no second call site in scope; YAGNI.
- V-UT-15 / V-UT-16 / V-UT-17 / V-UT-18 (the four `merge_json_managed` tests) — superseded by V-UT-20 / V-UT-21 / V-UT-22.
- Phase 3 step 2 (the generic JSON-pointer merge helper implementation) — replaced with "implement `update_settings_hook` / `remove_settings_hook` as small dedicated functions in `io/fs.rs`".
- Phase 3 step 3 (hash-tracking of post-merge contents) — superseded by C-17.
- `Error::ContextProjectionMismatch` — per R-009.

[**Unresolved**]

- **Slash-command update count and wording (G-10):** R-eviewer flagged this as "manual review" in 00_REVIEW. It remains a bulk edit of three Markdown files; no per-file assertion is added because the prompt wording is the agent's content, not a mechanical check. If the next reviewer wants this elevated to a Constraint with a fixed template-string requirement, I'll add one — but I'd rather not over-formalize prose that the agent will read and follow.
- **cwd-discovery scope expansion:** C-21 applies `Layout::discover_from` to **all** commands, not just `context`. This is wider blast radius than the task title implies. Question for next reviewer: acceptable in this task, or carve out into a sibling task `ark-cwd-discovery` and accept V-E-5's "without --dir, NotLoaded" in this iteration?

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|-----|----------|------------|
| Review | R-001 (HIGH — settings.json round-trip) | Accepted | New `Snapshot::hook_bodies` + `update_settings_hook` / `remove_settings_hook`. C-17, C-18. V-IT-12. G-11 rewritten. |
| Review | R-002 (HIGH — JSON-merge vs hash model) | Accepted | `.claude/settings.json` exempted from hash tracking (option (b)); re-applied on every `init` / `load` / `upgrade` analogously to `CLAUDE.md`. C-17. T-3 restructured. V-IT-13. |
| Review | R-003 (HIGH — projection filter underspecified) | Accepted | G-7 rewritten with explicit predicate; new C-20 defines normalizer + parser. V-UT-23, V-UT-24. |
| Review | R-004 (MEDIUM — artifact iteration rule) | Accepted | C-19 + helper. V-UT-26. |
| Review | R-005 (MEDIUM — cwd discovery) | Accepted with scope-expansion flag | C-21 applies uniformly to all commands. V-UT-25. See Unresolved. |
| Review | R-006 (MEDIUM — text schema header contradiction) | Accepted | C-3 dropped text-mode clause; C-10 authoritative. |
| Review | R-007 (MEDIUM — git invocation helper) | Accepted | C-22 + `io/git.rs`. V-UT-27. |
| Review | R-008 (MEDIUM — specs INDEX parser) | Accepted | C-24 formalizes both grammars. |
| Review | R-009 (LOW — unused error variant) | Accepted | `Error::ContextProjectionMismatch` removed; `debug_assert!` instead. |
| Review | R-010 (LOW — JSON pretty-print) | Accepted | C-23 pins indent / trailing newline / field order. |
| Review | TR-1 | Accepted | Two-flag design retained. |
| Review | TR-2 | Accepted | Schema-from-v1 retained; R-006 also resolves the C-3/C-10 contradiction. |
| Review | TR-3 | Accepted (option (b)) | Exempt from hash tracking. Rationale in §Trade-offs T-3. |
| Review | TR-4 | Accepted | `DIRTY_FILES_CAP: usize = 20` per C-25. |
| Review | TR-5 | Accepted | Section names locked: `## GIT STATUS`, `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE`. V-UT-13 golden fixtures. |
| Review | TR-6 | Accepted | Parser regex tightened per C-20. |

---

## Spec

[**Goals**]

- **G-1:** `ark context` is a top-level, visible, semver-covered subcommand (in `ark --help`).
- **G-2:** Two flags: `--scope {session|phase}` (default `session`), `--for {design|plan|review|execute|verify}` (required iff `--scope=phase`). Clap rejects mismatched combinations.
- **G-3:** `--format {json|text}` (default `text`).
- **G-4:** JSON output carries `"schema": 1` as the first field. Schema is additive-only.
- **G-5:** Payload contains paths and summaries only — no file bodies inlined.
- **G-6:** Session projection: git state, active tasks (flat), project specs index, feature specs index, recent-archive (last 5).
- **G-7 (revised per R-003):** Phase projections:
  - `design`: full `specs` (project + features unfiltered), `archive`.
  - `plan` / `review`: `specs.project` unchanged, `specs.features` filtered via C-20. No `archive`. No `tasks`.
  - `execute`: `specs.project` only; `specs.features = []`. `git.dirty_files` present. No `archive`. No `tasks`.
  - `verify`: `specs.project` only. Current task artifacts include PRD + latest PLAN + VERIFY.md if present. No `archive`. No `tasks`.
- **G-8 (revised):** `SessionStart` hook applied via `update_settings_hook` at `init` / `load` / `upgrade`. Not in template tree. Not hash-tracked. Idempotent. Preserves siblings.
- **G-9 (removed — see C-17).**
- **G-10:** Three slash commands (`quick.md`, `design.md`, `archive.md`) updated to call `ark context --scope phase --for <phase> --format json` at entry.
- **G-11 (revised per R-001):** End-to-end round-trip:
  1. `ark init` → `update_settings_hook` writes Ark entry.
  2. `ark unload` → `Snapshot::hook_bodies` captures Ark entry.
  3. `ark load` → re-applies via `update_settings_hook`.
  4. `ark remove` → `remove_settings_hook` removes Ark entry; sibling user hooks survive.
  5. `ark upgrade` → re-applies unconditionally.
- **G-12:** Text mode layout: `## GIT STATUS`, `## CURRENT TASK`, `## ACTIVE TASKS`, `## SPECS`, `## ARCHIVE` with blank-line separators. Sections absent from a projection are omitted entirely.

Non-goals NG-1 through NG-11 unchanged from 00_PLAN.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                        — adds Context(ContextArgs) top-level
└── ark-core/src/
    ├── lib.rs                                  — re-exports context + discover_from
    ├── error.rs                                — adds Error::GitSpawn
    ├── layout.rs                               — adds claude_settings(), discover_from(),
    │                                             specs_project_index()
    ├── io/
    │   ├── path_ext.rs                         — unchanged
    │   ├── fs.rs                               — adds update_settings_hook,
    │   │                                         remove_settings_hook,
    │   │                                         ARK_CONTEXT_HOOK_COMMAND
    │   └── git.rs                              — NEW (C-22)
    ├── state/
    │   ├── manifest.rs                         — unchanged
    │   └── snapshot.rs                         — adds hook_bodies
    ├── commands/
    │   ├── init.rs                             — calls update_settings_hook
    │   ├── load.rs                             — restores snapshot.hook_bodies + apply
    │   ├── unload.rs                           — captures Ark hook into snapshot.hook_bodies
    │   ├── remove.rs                           — calls remove_settings_hook
    │   ├── upgrade.rs                          — calls update_settings_hook unconditionally
    │   └── context/
    │       ├── mod.rs
    │       ├── gather.rs
    │       ├── model.rs                        — DIRTY_FILES_CAP, SCHEMA_VERSION
    │       ├── projection.rs
    │       ├── render.rs
    │       └── related_specs.rs                — PRD section parser (C-20)
└── templates/
    ├── ark/                                    — unchanged
    └── claude/
        └── commands/ark/                       — three .md files updated per G-10
```

**Module coupling.** `mod.rs → gather → model`; `mod.rs → projection → model`; `mod.rs → render → model`. `related_specs.rs` is a leaf used only by `gather.rs`. `io/git.rs` is a leaf used only by `gather.rs`.

**Call graph for `ark context`:**

```
context(opts)
  ├── layout = Layout::discover_from(opts.cwd)?            (C-21)
  ├── ctx = gather::gather_context(&layout)
  │     ├── io::git::run_git(["branch","--show-current"], root) → branch
  │     ├── io::git::run_git(["status","--porcelain"], root)    → dirty files (cap 20) + count
  │     ├── io::git::run_git(["log","--oneline","-5"], root)    → commits
  │     ├── list active tasks (skip "archive", ".current")
  │     ├── list archive (5 most recent by archived_at)
  │     ├── parse specs/project/INDEX.md (C-24)
  │     ├── parse specs/features/INDEX.md via read_managed_block (C-24)
  │     └── if .current exists: load task.toml, list NN_PLAN/NN_REVIEW (C-19),
  │         parse PRD [**Related Specs**] (C-20)
  ├── projected = projection::project(ctx, opts.scope)
  └── write per C-23 (JSON: to_writer_pretty + "\n"; Text: Display + "\n")
```

**Call graph for `update_settings_hook`:**

```
update_settings_hook(path, ark_entry) -> Result<bool>
  ├── read settings file → serde_json::Value (or {} if missing)
  ├── navigate to "hooks"."SessionStart" (creating intermediates if absent)
  ├── find entry where entry["command"] == ARK_CONTEXT_HOOK_COMMAND
  ├── replace if found, append if not
  ├── serialize back (pretty, 2-space)
  └── write atomically via PathExt::write_bytes
  → Ok(true) if a write happened, Ok(false) if idempotent no-op
```

[**Data Structure**]

Context model unchanged from 00_PLAN except:

```rust
// ark-core/src/commands/context/model.rs
pub const SCHEMA_VERSION: u32 = 1;
pub const DIRTY_FILES_CAP: usize = 20;

impl ArtifactKind {
    pub fn iteration(&self) -> Option<u32> {
        match self {
            Self::Plan { iteration } | Self::Review { iteration } => Some(*iteration),
            _ => None,
        }
    }
}
```

Snapshot extension:

```rust
// ark-core/src/state/snapshot.rs
pub struct Snapshot {
    pub files: Vec<SnapshotFile>,
    pub managed_blocks: Vec<SnapshotBlock>,
    pub hook_bodies: Vec<SnapshotHookBody>,                  // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHookBody {
    pub path: PathBuf,                                       // .claude/settings.json
    pub json_pointer: String,                                // "/hooks/SessionStart"
    pub identity_key: String,                                // "command"
    pub identity_value: String,                              // ARK_CONTEXT_HOOK_COMMAND
    pub entry: serde_json::Value,                            // the JSON object itself
}
```

Error additions:

```rust
// ark-core/src/error.rs
Error::GitSpawn { source: std::io::Error },                  // C-22
// REMOVED: Error::ContextProjectionMismatch
```

`io/fs.rs` additions:

```rust
pub const ARK_CONTEXT_HOOK_COMMAND: &str =
    "ark context --scope session --format json";

pub fn update_settings_hook(path: &Path, entry: serde_json::Value) -> Result<bool>;
pub fn remove_settings_hook(path: &Path, identity_value: &str) -> Result<bool>;
```

`io/git.rs`:

```rust
pub struct GitOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_git(args: &[&str], cwd: &Path) -> Result<GitOutput>;
```

Layout addition:

```rust
impl Layout {
    pub fn claude_settings(&self) -> PathBuf;             // .claude/settings.json
    pub fn specs_project_index(&self) -> PathBuf;         // .ark/specs/project/INDEX.md
    pub fn discover_from(cwd: impl AsRef<Path>) -> Result<Self>;   // C-21
}
```

[**API Surface**]

CLI shape unchanged from 00_PLAN. Library re-exports gain `run_git`, `GitOutput`, `update_settings_hook`, `remove_settings_hook`, `ARK_CONTEXT_HOOK_COMMAND`, `Layout::discover_from`.

The `merge_json_managed` helper is **not** exposed — replaced by the dedicated pair.

[**Constraints**]

C-1 through C-16 unchanged from 00_PLAN except:
- **C-3 (revised per R-006):** "JSON output's first field is `\"schema\": 1`. Asserted by a byte-level unit test." (Text-mode clause dropped.)
- **C-11 (revised):** identity value referenced via `ARK_CONTEXT_HOOK_COMMAND` constant; tests reference the constant, not the literal.

C-17 through C-26 as enumerated in Log §Added above.

## Runtime

[**Main Flow**]

1. User / hook runs `ark context [--scope …] [--for …] [--format …]` from any cwd inside an Ark project.
2. CLI parses args, validates `--scope`/`--for` relationship, builds `ContextOptions`, dispatches to `commands::context::context`.
3. `context` calls `Layout::discover_from(cwd)` — error `Error::NotLoaded` if no `.ark/` ancestor.
4. `context` calls `gather_context(&layout)` → full `Context`.
5. `context` calls `project(ctx, scope)` → `ProjectedContext`.
6. Output per C-23: JSON via `to_writer_pretty` + newline; Text via `Display` + newline.

[**Failure Flow**]

1. `.ark/` not found in any ancestor → `Error::NotLoaded { path: cwd }` from `Layout::discover_from`. Exit 1.
2. `.current` points at nonexistent slug → `current_task: None`. Not an error.
3. `task.toml` of current task corrupt → `Error::TaskTomlCorrupt` (reused from `ark-agent-namespace`).
4. Specs INDEX malformed → empty list, no error. Stderr warning in text mode only.
5. `git` missing from PATH → `Error::GitSpawn { source }`. Exit 1.
6. Non-git directory → soft fail; `GitState { branch: "unknown", is_clean: true, .. }`.
7. `update_settings_hook` write failure → propagated as `Error::Io`.

[**State Transitions**]

- `.claude/settings.json` exists with Ark entry → stable.
- `.claude/settings.json` exists without Ark entry → next `update_settings_hook` inserts.
- `.claude/settings.json` absent → next `update_settings_hook` creates with only Ark entry.
- `ark remove` → Ark entry deleted; `SessionStart` array remains (possibly empty) so user can add siblings without re-init.

## Implementation

[**Phase 1 — Core data model, gather engine, layout + git helpers**]

1. Add `io/git.rs` with `run_git` + `GitOutput` (C-22). V-UT-27.
2. Add `Layout::discover_from` + update `TargetArgs::resolve` to use it (C-21). V-UT-25. Update `init`, `load`, `unload`, `remove`, `upgrade` callers.
3. Add `Layout::claude_settings()`, `specs_project_index()`.
4. Add `Error::GitSpawn`; remove `Error::ContextProjectionMismatch` (R-009).
5. Define `Context`, sub-structs in `model.rs` with `Serialize`. `SCHEMA_VERSION`, `DIRTY_FILES_CAP`.
6. Add `ArtifactKind::iteration()` (C-19). V-UT-26.
7. Implement `gather::gather_context(&layout)` per call graph. Test each piece.
8. Implement `related_specs.rs` parser (C-20). V-UT-23.
9. Implement specs-INDEX parsers (C-24).

[**Phase 2 — Projection, rendering, CLI**]

1. Implement `projection::project(ctx, scope)` per G-7. V-UT-7 through V-UT-12 + V-UT-24.
2. Implement `render.rs` with locked section names (G-12). V-UT-13 golden fixtures.
3. Implement `commands/context/mod.rs::context()`; wire CLI with arg-relation validation.
4. Wire `lib.rs` re-exports.
5. Byte-level assertions for JSON pretty-print (C-23) + text trailing-newline.

[**Phase 3 — Hook lifecycle integration**]

1. Add `io::fs::update_settings_hook` + `remove_settings_hook` + `ARK_CONTEXT_HOOK_COMMAND`. V-UT-20, V-UT-21, V-UT-22.
2. Wire `init.rs` to call `update_settings_hook` after scaffold. Add `Snapshot::hook_bodies` capture in `unload.rs` + restore in `load.rs`. Wire `remove.rs` to call `remove_settings_hook`. Wire `upgrade.rs` to call `update_settings_hook` unconditionally after template writes.
3. Update `templates/claude/commands/ark/{quick,design,archive}.md` to prepend `ark context --scope phase --for <phase> --format json` recipe.
4. Update `.ark/workflow.md` §7 mechanics table to add: `| Session / phase orientation (read-only) | ark context [--scope …] [--for …] |` placed above the `ark agent` rows.
5. Integration tests V-IT-1 through V-IT-13 (V-IT-12, V-IT-13 are new).

[**Phase 4 — Docs, upgrade SPEC CHANGELOG, cleanup**]

1. Append CHANGELOG row to `specs/features/ark-upgrade/SPEC.md`: terse note about `.claude/settings.json` joining `CLAUDE.md` as re-applied-not-hashed.
2. Update `AGENTS.md` Repository Layout & module map.
3. Update `README.md` user-facing command list with `ark context`.
4. Update `docs/ROADMAP.md` — mark `ark context` shipped under Phase 1.
5. `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`.
6. End-to-end smoke (AGENTS.md:83) extended with `ark context --scope session --format json`.

## Trade-offs

- **T-1 (two flags vs single `--mode`):** Unchanged. Two flags. TR-1 endorsed.
- **T-2 (schema from v1):** Unchanged. TR-2 endorsed.
- **T-3 (revised per TR-3):**
  - **Chosen:** exempt `.claude/settings.json` from hash tracking. Re-apply Ark entry via `update_settings_hook` on every `init` / `load` / `upgrade`. Matches `CLAUDE.md` managed-block precedent (upgrade SPEC C-8).
  - *Advantages:* simpler mental model; no new mechanism; `reconcile_managed_blocks` doesn't need JSON semantics; upgrade prompt path not triggered by settings file at all.
  - *Disadvantages:* `ConflictPolicy` (`--force` / `--skip-modified` / `--create-new`) does not apply to the Ark hook entry — it's always re-applied. Users who customize the hook command see it reverted on every upgrade. Documented (V-UT-22). Users wanting custom commands add siblings, not edit the Ark entry.
  - *Rejected (option (a), extending `reconcile_managed_blocks` to JSON):* more correct in theory but requires JSON-pointer-aware analogs of `scan_managed_markers` / `splice_managed_block`. Significant new code for one consumer. Defer until a second JSON-template surface emerges.
- **T-4 (dirty_files list + count):** Unchanged. `DIRTY_FILES_CAP` constant (C-25).
- **T-5 (multi-section text):** Unchanged. Section names locked.
- **T-6 (parse from PRD):** Unchanged. Parser tightened (C-20).
- **T-7 (NEW — cwd discovery scope):**
  - **Chosen:** apply `Layout::discover_from` uniformly to all commands.
  - *Advantages:* consistent cwd semantics; hooks launched from arbitrary cwd work for every command; no asymmetry.
  - *Disadvantages:* wider blast radius than the task title implies; touches dispatch in 5 commands. Integration tests that relied on "non-root cwd → error" may need updating (most use `--dir` explicitly; sweep needed).
  - *Rejected (context-only discovery):* inconsistent semantics; hook users would hit `NotLoaded` on other commands too. Worse.
  - See Unresolved §2 — flagged for next reviewer.

## Validation

[**Unit Tests**]

V-UT-1 through V-UT-14 unchanged from 00_PLAN.
- V-UT-15 / V-UT-16 / V-UT-17 / V-UT-18 **removed** (merge helper deleted; superseded).
- V-UT-19 unchanged.
- V-UT-20: `update_settings_hook` empty-file + idempotence.
- V-UT-21: `update_settings_hook` preserves unrelated hook entries.
- V-UT-22: `update_settings_hook` overwrites user-modified Ark entry back to canonical form.
- V-UT-23: Related-specs parser grammar (C-20).
- V-UT-24: Projection filter (C-20 second half).
- V-UT-25: `Layout::discover_from` walks ancestors.
- V-UT-26: `ArtifactKind::iteration()`.
- V-UT-27: `io::git::run_git` soft-fail and non-git-dir handling.

[**Integration Tests**]

V-IT-1 through V-IT-11 unchanged.
- V-IT-12: Snapshot round-trip with `Snapshot::hook_bodies`.
- V-IT-13: Upgrade re-adds deleted Ark hook.

[**Failure / Robustness Validation**]

V-F-1 through V-F-6 unchanged.

[**Edge Case Validation**]

V-E-1 through V-E-7 unchanged except:
- V-E-5 is now a **tested invariant** (not a bug risk): with C-21, `ark context` from a subdir succeeds by walking up.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-11 |
| G-2 | V-IT-5, V-IT-6, V-UT-7 through V-UT-12 |
| G-3 | V-IT-2, V-IT-3, V-IT-4, V-UT-13, V-UT-14 |
| G-4 | V-UT-14, V-E-6 |
| G-5 | V-UT-4 |
| G-6 | V-UT-7, V-IT-1 |
| G-7 (revised) | V-UT-8 through V-UT-12 + V-UT-24 |
| G-8 (revised) | V-UT-20, V-UT-21, V-UT-22, V-IT-7 |
| G-10 | Template content review + V-IT-7 |
| G-11 (revised) | V-IT-12, V-IT-13 |
| G-12 | V-UT-13 |
| C-1 | V-IT-11 |
| C-2 | V-IT-11 (extended) |
| C-3 (revised) | V-UT-14 |
| C-4 | Clippy / manual review |
| C-5 | Clippy / manual review |
| C-6 | Reviewer gate (policy) |
| C-7 | V-IT-2 |
| C-8 | Dedicated cap test (>5 commits, >20 dirty files) |
| C-9 | Dedicated cap test (archive >5 entries) |
| C-10 (revised) | V-UT-13 (no schema field in text) |
| C-11 (revised) | V-UT-22 |
| C-12 | V-UT-20 |
| C-13 | V-UT-1, V-E-1 |
| C-14 | V-IT-10 |
| C-15 | V-IT-7 fixture |
| C-16 | V-UT-21, V-IT-8 |
| C-17 | V-UT-20 through V-UT-22, V-IT-13 |
| C-18 | V-IT-12 |
| C-19 | V-UT-26 + V-UT-4 (artifact list ordering) |
| C-20 | V-UT-23, V-UT-24 |
| C-21 | V-UT-25, V-E-5 |
| C-22 | V-UT-27 |
| C-23 | V-UT-14 (byte-level JSON) + text trailing-newline test |
| C-24 | V-UT-1 / V-UT-3 (gather tests seed INDEX fixtures) |
| C-25 | Symbolic dirty-files cap test |
| C-26 | Clippy / manual review (no `Command::new` outside `io/git.rs`) |
