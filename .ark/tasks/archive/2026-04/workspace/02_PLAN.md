# `workspace` PLAN `02`

> Status: Approved for Implementation
> Feature: `workspace`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`
> - PRD: `PRD.md`
> - Related specs: `specs/features/worktree-support/SPEC.md`, `specs/features/ark-agent-namespace/SPEC.md`, `specs/features/ark-context/SPEC.md`

---

## Summary

Iteration 02 closes out `01_REVIEW.md`'s remaining 1 HIGH and 4 non-blocking findings. Headline change: add `PathExt::append_text` as a new helper (Option A from R-001) — the journal write needs append semantics that `PathExt` doesn't currently expose, and read-modify-rewrite is a worse fit for the no-bare-`std::fs::*` constraint. `parse_oneline` is now declared as a `pub(crate)` helper in `journal.rs` with explicit splitting rules (R-002). C-19's fenced-code-block predicate is tightened to handle backtick AND tilde fences with length-aware closing-fence matching, and indented code blocks are explicitly out of scope (R-003). C-20 weakens canonicalize requirements: lexical normalization is sufficient; canonicalize is best-effort (R-004). C-18's call graph annotation makes `archive_path`'s post-rename lifetime explicit (R-005). No goals removed; no new HIGH or CRITICAL issues to address. The `## Spec` section remains self-contained.

## Log

[**Added**]
- G-17: new `PathExt::append_text` method declared as the documented append primitive.
- `parse_oneline` declared as `pub(crate)` helper in `journal.rs` Data Structure block. (R-002)
- C-19 tightened: tilde fences, length-aware closing-fence matching, indented-code exclusion noted. (R-003)
- NG-13: parser limitations explicitly out of scope (4-backtick mid-fence pasting that breaks the simpler 3-backtick toggle is rare; documented). (R-003)
- C-20 step 3 reworded: lexical normalization preferred; canonicalize is best-effort with graceful error. (R-004)
- C-18 annotation clarifying `archive_path` is the post-rename value passed via `RecordTaskOptions`. (R-005)
- Phase 2 test: `path_ext_append_text_creates_and_appends`.
- T-7 trade-off rationale captured (chose Option A: add API).

[**Changed**]
- C-2 footnote: `append_text` is the documented append primitive; raw `std::fs::OpenOptions` is still forbidden under `commands/agent/workspace/`. (R-001)
- G-4 step 5 wording: "append via `PathExt::append_text`" stays, but is now backed by the actual API addition. (R-001)
- C-13 unchanged in semantics but cross-references `parse_oneline`'s public-internal location. (R-002)

[**Removed**]
- None.

[**Unresolved**]
- None. All findings actioned.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (HIGH) | Accepted | Adopted Option A: added `PathExt::append_text` as new API (G-17). Updated Data Structure, API Surface, C-2, Phase 2 implementation + tests. Journal append now backed by a real method. |
| Review | R-002 (MEDIUM) | Accepted | Declared `pub(crate) fn parse_oneline(stdout: &str) -> Vec<JournalCommit>` in journal.rs Data Structure with documented splitting rule. Cross-referenced from C-13. Added V-UT-12. |
| Review | R-003 (MEDIUM) | Accepted | Tightened C-19 fence-handling predicate (length-aware matching, tilde support). Added NG-13 documenting limits. V-UT-5 expanded; V-E-8 added for tilde fences. |
| Review | R-004 (LOW) | Accepted | C-20 step 3 reworded: lexical normalization is the default; `canonicalize` is best-effort and its failure converts to lexical pop-`.git` rather than propagating raw I/O errors. Added V-UT-8 lexical-fallback subcase. |
| Review | R-005 (LOW) | Accepted | C-18 gained an annotation clarifying `archive_path` lifetime — passed via `RecordTaskOptions.archive_path` from `task_archive`'s frame, post-rename. Call graph annotated. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec `Core specification`

[**Goals**]

- **G-1:** New `commands/agent/workspace/` module with public API: `workspace_init(WorkspaceInitOptions) -> Result<WorkspaceInitSummary>` and `workspace_record(WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary>`. Both follow the existing `ark agent` patterns: write to disk, return one-line `Display` summary, no `println!` in command bodies, all FS via `io::PathExt`, all `.ark/`-relative paths via `Layout` helpers. A third public function `record_task(RecordTaskOptions) -> Result<WorkspaceRecorded>` is the bridge called by `task::archive`; it shares the journal-write path with `workspace_record` but is invoked internally rather than from the CLI.

- **G-2:** Identity has two bootstrap paths. **(a)** Default: `ark init` prompts for a developer name (parallel to the platform prompts; see G-18). **(b)** Manual: `ark agent workspace init --name <x>` for already-installed projects, idempotent re-init, and scripted setup. Missing `.developer` → identity-required commands return `Error::DeveloperNotInitialized { path }`; auto-record on `task archive` becomes a no-op (returns `WorkspaceRecorded::SkippedNoIdentity` and emits one-line stderr "no developer set; skipping workspace record"). Auto-record may also be globally disabled via `workspace.toml`'s `auto_record_on_archive = false` (G-8) — in that case it returns `WorkspaceRecorded::SkippedDisabled`. **There is no per-archive `--no-record` CLI flag.** The two skip conditions above are the only paths that disable auto-record.

- **G-3:** `workspace_init` sequence (creates):
  1. Validate `name` per C-3 (leading letter, then ASCII alphanumeric + `_-`, 1..=40 chars). Reject otherwise → `Error::InvalidDeveloperName`.
  2. If `.ark/.developer` exists with a `name=` line and the existing name ≠ `<x>` → `Error::DeveloperAlreadyInitialized { name }` (re-init must remove file first). If existing name == `<x>`, fall through (idempotent re-scaffold).
  3. Write `.ark/.developer` as `name=<x>\ninitialized_at=<RFC3339>\n`.
  4. Ensure `.ark/workspace/<x>/` exists.
  5. Seed `<dev>/index.md` from the embedded `_workspace_index.md` template (managed-block markers `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS` already present in the template; bodies start empty).
  6. Seed `<dev>/journal-1.md` from the embedded `_workspace_journal.md` template.
  7. Return `WorkspaceInitSummary { name, dev_dir, created: bool }`.

  Idempotent: if `.ark/.developer` exists with the **same name** AND both files exist with their seeded content, return `created: false` without modification.

- **G-4:** `workspace_record` sequence (appends):
  1. Resolve **parent layout** via `parent::resolve_parent_layout(&layout)` (C-20) — auto-record from inside a worktree writes to parent's workspace.
  2. Read `.developer` (parent's). Missing → `Error::DeveloperNotInitialized` for the manual path. (Caller-controlled skip for the task-archive path is handled in `record_task`, not here — see G-7.)
  3. Load `WorkspaceConfig::load_or_default(&parent_layout)`.
  4. Determine active journal file: enumerate `<dev>/journal-N.md`, pick the highest-numbered (numeric, not lexical). If its line count ≥ `cfg.journal_max_lines`, rotate: next file is `journal-{N+1}.md`. `cfg.journal_max_lines < 100` → `Error::InvalidConfigField`. `N+1 > 9999` → `Error::JournalRotationLimit`.
  5. Render the session entry per G-5 and append via `target_journal.append_text(&entry)`. The `append_text` primitive is declared on `PathExt` (G-17) with semantics: opens the file with `create(true).append(true)`, writes the bytes, and closes — atomic at the syscall level for the single write call. No prior file content is modified.
  6. Re-render `<dev>/index.md`'s two managed blocks via `update_managed_block`:
     - `ARK:WORKSPACE_STATUS` body: bullet list with active file name + total sessions + last-active date.
     - `ARK:WORKSPACE_SESSIONS` body: GFM table `# | Date | Title | Kind | Slug | Branch | Commits` sorted by session number desc. The journal files are the source of truth; `index.md`'s blocks are derived by re-scanning all `<dev>/journal-*.md` files (C-21 caps the scan).
  7. Return `WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated: bool }`.

- **G-5:** Session entry markdown shape (Trellis-style trimmed). Each entry begins with the unique anchor heading `## Session {N}: {Title}` and follows this exact field order:

  ```markdown
  ## Session {N}: {Title}

  **Date**: {YYYY-MM-DD}
  **Kind**: {task|manual}
  **Slug**: {slug | -}
  **Branch**: `{branch}`

  ### Summary
  {body — one or more paragraphs}

  ### Commits
  | Hash | Message |
  |------|---------|
  | `{short}` | {message} |

  ### Next Steps
  - {item}
  ```

  Two consecutive entries are separated by a single blank line. The anchor regex used by the parser (C-19) is `^## Session (\d+):\s+(.+?)\s*$`.

- **G-6:** Commit collection. **Task-cwd resolution** for `record_task`:
  1. If `worktree_path.is_some()` AND that path exists on disk AND is a directory → use it.
  2. Else if `archive_path` exists on disk AND is a directory → use it.
  3. Else → use the parent project root (`parent_layout.root()`).

  **Git invocation** chosen by available metadata:
  - If `base_branch.is_some()` → `git log <base_branch>..HEAD --oneline -n 20` from the resolved cwd.
  - Else (no `--worktree` was used; pre-existing tasks) → `git log -n 20 --oneline` from the resolved cwd.

  For manual `workspace_record`: always `git log -n 20 --oneline` from the **caller's cwd** (not the parent layout's root) — captures whichever branch the user is on.

  Non-zero git exit (non-git directory, no commits) → empty commits table, no error. Output is parsed via `parse_oneline` (C-13).

- **G-7:** `task::archive::task_archive` integration. The new call site is **after** the `.current` cleanup block AND **before** the `Ok(summary)` return, **regardless of tier**. Quick / standard / deep all auto-record. The insertion is a single statement:

  ```rust
  let workspace_recorded = workspace::record::record_task(RecordTaskOptions {
      project_root: opts.project_root.clone(),
      slug: opts.slug.clone(),
      title: toml.title.clone(),
      tier,
      branch: toml.branch.clone(),
      base_branch: toml.base_branch.clone(),
      worktree_path: toml.worktree_path.clone(),
      archive_path: archive_path.clone(),
      archived_at: now,
  })?;
  ```

  `record_task` is a thin wrapper over the shared journal-write path: it gathers task-derived inputs, then invokes the same append + index-re-render logic as `workspace_record`. It MUST internally swallow `Error::DeveloperNotInitialized` (returning `WorkspaceRecorded::SkippedNoIdentity` instead) so that absent identity does not abort archive. All other errors propagate. **No-rollback policy**: if `record_task` returns an error, the archive directory rename is already committed and the user runs `/ark:record` to reconcile. `TaskArchiveSummary` gains `workspace_recorded: WorkspaceRecorded`.

- **G-8:** `WorkspaceConfig` mirrors `WorktreeConfig`:

  ```rust
  pub struct WorkspaceConfig {
      pub journal_max_lines: u32,        // default 2000
      pub auto_record_on_archive: bool,  // default true
  }
  ```

  `auto_record_on_archive = false` → `record_task` returns `SkippedDisabled` without reading identity or invoking git. `journal_max_lines < 100` → `Error::InvalidConfigField { field: "journal_max_lines", reason: "must be >= 100" }`. Loaded via `WorkspaceConfig::load_or_default(&parent_layout)`. Corrupt → `Error::WorkspaceConfigCorrupt { path, source }` with source chained.

- **G-9:** Parent-root resolution. Helper `parent::resolve_parent_layout(layout: &Layout) -> Result<Layout>` returns the parent checkout's `Layout` when called from inside a git worktree, or `layout.clone()` otherwise. Detection algorithm is specified in C-20.

- **G-10:** New CLI subcommand group:

  ```
  ark agent workspace init   --name <x>
  ark agent workspace record [--title "<t>"] [--summary "<s>"] [--next "<n>"]
  ```

  Both subcommands flatten `TargetArgs` and resolve layout via `TargetArgs::resolve_with_discovery`. Both outputs go through `Display` summary types; no ad-hoc `println!`.

- **G-11:** `templates/ark/.gitignore` adds `.developer` as a second line. Currently a flat 1-line file (`worktrees/`); becomes a flat 2-line file. **No managed block** — fully Ark-owned. `ark upgrade` re-applies the canonical content unconditionally per the existing upgrade `.gitignore` policy.

- **G-12:** `templates/ark/workspace.toml` ships a commented config with default values surfaced as comments (mirrors `templates/ark/worktree.toml`). Created by `ark init`; **NEVER overwritten** by `ark upgrade`.

- **G-13:** `templates/ark/_workspace_index.md` is the per-developer `index.md` template (the leading underscore prevents top-level auto-copy by `ark init`):

  ```markdown
  # Workspace Index — {{name}}

  > Per-developer session journal. Updated automatically by `ark agent workspace {init|record}` and `ark agent task archive`.

  <!-- ARK:WORKSPACE_STATUS:START -->
  - **Active File**: `journal-1.md`
  - **Total Sessions**: 0
  - **Last Active**: -
  <!-- ARK:WORKSPACE_STATUS:END -->

  ## Sessions

  <!-- ARK:WORKSPACE_SESSIONS:START -->
  | # | Date | Title | Kind | Slug | Branch | Commits |
  |---|------|-------|------|------|--------|---------|
  <!-- ARK:WORKSPACE_SESSIONS:END -->
  ```

  `{{name}}` is a single literal token replaced at scaffold time by string substitution (no templating engine). Marker syntax matches `update_managed_block`'s existing convention.

- **G-14:** `templates/ark/_workspace_journal.md` is the per-developer `journal-N.md` template:

  ```markdown
  # Journal — {{name}} (Part {{n}})

  > AI development session journal. Started: {{date}}

  ---

  ```

  `{{name}}`, `{{n}}`, `{{date}}` are literal-token substitutions. Trailing blank line is intentional (separates header from first appended entry).

- **G-15:** `/ark:record [<title>]` slash command. Shipped at `templates/claude/commands/ark/record.md`, with peers `templates/codex/skills/ark-record/SKILL.md` and `templates/opencode/commands/ark/record.md`. The slash command's body instructs the agent to:
  1. Pull `ark context --scope session --format json`.
  2. If `<title>` provided: invoke `ark agent workspace record --title "<title>" --summary "<gen>" --next "<gen>"` where summary/next are agent-generated from the conversation context.
  3. If `<title>` absent: agent first summarizes the most recent topic from conversation context into `<title>` + `<summary>` + `<next>`, then invokes the CLI. If conversation context has no work yet (fresh session), the agent asks the user for a title rather than fabricating one.

  The CLI is the source of truth; the slash command is a recipe for content generation.

- **G-16:** Workflow doc additions. `.ark/workflow.md` (and `templates/ark/workflow.md`) gain a `### Workspace (optional)` subsection under §6 Mechanics — terse, table-row style. AGENTS.md command table gains one short row matching the existing terseness of utility-command rows. Slash commands `quick.md`, `design.md`, `archive.md` are NOT modified.

- **G-17:** New `PathExt::append_text(&self, contents: &str) -> Result<()>` method on `crates/ark-core/src/io/path_ext.rs`. Opens the file with `OpenOptions::new().create(true).append(true)`, writes `contents.as_bytes()` via `write_all`, and drops the handle. The single `write_all` call is atomic at the OS level for buffer sizes below the pipe-write atomicity limit (PIPE_BUF on POSIX); journal entries are well below this threshold (~1KB typical). Errors map to `Error::Io` via `PathExt`'s existing wrapping pattern. Used internally by `commands/agent/workspace/record.rs`; available to all of `ark-core`. Re-exported alongside the existing `PathExt` re-export.

- **G-18:** `ark init` developer-identity bootstrap. The init flow gains a fourth interactive prompt parallel to the existing `claude / codex / opencode` prompts: `set up workspace identity?`. On `Y`, the user is asked for a name (default suggestion = `whoami` output if available; otherwise no default). Validation reuses `identity::validate_developer_name` per C-3. On valid input, the init flow calls `workspace_init` *after* the platform-template extraction completes — so `.ark/workspace.toml` exists by the time identity is bootstrapped. Two new CLI flags mirror the platform pattern:
  - `--developer <name>` — explicit non-interactive bootstrap (validated; runs `workspace_init` once at the end of init).
  - `--no-developer` — opt out of the prompt; `.ark/.developer` and `.ark/workspace/` are NOT created.

  Non-interactive init (no TTY) requires one of `--developer <name>` or `--no-developer`; running `ark init` in a script without either flag now errors with the same UX as the existing platform prompts (`init requires at least one of …`). When the user already has identity from a prior init (idempotent rerun), the prompt offers to keep the existing name (default = `Y`) or accept a new one (which errors with `DeveloperAlreadyInitialized` since rename is NG-7). The `InitSummary` gains a `developer: Option<String>` field summarizing the outcome.

- **NG-1:** No team / multi-developer aggregation.
- **NG-2:** No PR / Slack / GitHub publishing. Sessions are local markdown.
- **NG-3:** No edit/delete history of recorded sessions. Append-only.
- **NG-4:** No structured-output JSON for journal entries. Trellis-style markdown only.
- **NG-5:** No `ark unload`/`load` schema changes. `.ark/workspace/` is plain content; existing capture logic handles it. `.ark/.developer` is the lone file-level exclusion (C-7).
- **NG-6:** No automatic `task archive` rollback when `record_task` fails. Archive succeeded; user runs `/ark:record` to reconcile.
- **NG-7:** No identity migration / rename. Re-init requires deleting `.ark/.developer` first.
- **NG-8:** No `ark context` scope changes for the workspace MVP.
- **NG-9:** No worktree management of `.ark/workspace/`. Auto-record always writes to parent regardless of branch.
- **NG-10:** No `.ark/workspace/` shipped in `templates/ark/`. Created lazily by `workspace init`.
- **NG-11:** No CLI for `workspace deinit`. Hand-delete `.ark/.developer` to remove identity.
- **NG-12:** No per-archive `--no-record` CLI flag. Auto-record is gated by missing `.developer` (skip silently with stderr note) or `auto_record_on_archive = false`.
- **NG-13:** No exotic-markdown parsing in the entry-boundary predicate (C-19). The parser handles 3+ backtick fences and 3+ tilde fences with length-aware close matching, but it does NOT handle: (a) HTML `<pre>` blocks; (b) raw `<!-- ... -->` HTML comments containing `## Session N:` lines; (c) lines that begin with `## Session N:` indented to 4 or more leading spaces (CommonMark indented-code-block territory). Authors who paste markdown-with-headings into summaries should verify the index re-render after recording. Misparsed entries are silently skipped from the index table; the journal markdown is not modified.

[**Architecture**]

```
crates/
├── ark-cli/src/main.rs                              — adds Workspace(WorkspaceCliArgs)
│                                                     under AgentCommand; two subcommands
│                                                     Init, Record
└── ark-core/src/
    ├── lib.rs                                       — re-exports public workspace API
    ├── error.rs                                     — adds 6 variants (see Data Structure)
    ├── io/path_ext.rs                               — adds PathExt::append_text (G-17)
    ├── layout.rs                                    — adds workspace_dir,
    │                                                  workspace_developer_dir,
    │                                                  workspace_index, workspace_journal,
    │                                                  workspace_config_file, developer_file;
    │                                                  ensures Layout: Clone
    ├── commands/
    │   ├── unload.rs                                — extends file-level skip in BOTH
    │   │                                              walk sites (Stage A unload loop AND
    │   │                                              Stage B capture_orphan_hook_entries)
    │   │                                              to include layout.developer_file()
    │   └── agent/
    │       ├── mod.rs                               — pub mod workspace;
    │       ├── state.rs                             — unchanged
    │       ├── task/
    │       │   └── archive.rs                       — calls super::workspace::record::record_task
    │       │                                          after .current cleanup, before Ok return,
    │       │                                          regardless of tier;
    │       │                                          TaskArchiveSummary gains workspace_recorded
    │       └── workspace/                           — NEW
    │           ├── mod.rs                           — public types + dispatch
    │           ├── config.rs                        — WorkspaceConfig
    │           ├── identity.rs                      — read/write .developer; name validation
    │           ├── parent.rs                        — resolve_parent_layout via run_git
    │           ├── journal.rs                       — JournalEntry render; rotation;
    │           │                                      ParsedEntry; entry-boundary parser (C-19);
    │           │                                      pub(crate) parse_oneline
    │           ├── index.rs                         — render_status_block, render_sessions_block
    │           ├── init.rs                          — workspace_init
    │           └── record.rs                        — workspace_record + record_task wrapper
templates/
├── ark/
│   ├── .gitignore                                   — adds `.developer` line
│   ├── workspace.toml                               — NEW: shipped default config
│   ├── _workspace_index.md                          — NEW: per-dev index.md template
│   ├── _workspace_journal.md                        — NEW: per-dev journal-1.md template
│   └── workflow.md                                  — adds §6 Workspace subsection
├── claude/commands/ark/record.md                    — NEW
├── codex/skills/ark-record/SKILL.md                 — NEW
└── opencode/commands/ark/record.md                  — NEW
```

**Module coupling.** `task::archive` imports `super::workspace::record::record_task` (one-way: `task → workspace`). `workspace::{init, record}` import `workspace::{config, identity, journal, index, parent}`. Within workspace: `init → identity, journal, index`; `record → identity, parent, config, journal, index`. `parent` calls `io::git::run_git`; `journal` and `index` are leaves (markdown rendering only). `workspace::*` MUST NOT import `super::task` or `super::spec`.

**Call graph for `ark agent workspace init`:**

```
workspace::init::workspace_init(opts)
  ├── identity::validate_developer_name(&opts.name)         → Error::InvalidDeveloperName
  ├── if layout.developer_file().exists():
  │     ├── existing = identity::read_developer_name(&layout)?
  │     └── if existing != Some(opts.name):
  │           return Error::DeveloperAlreadyInitialized { name: existing.unwrap_or_default() }
  │     // else: fall through to idempotent re-scaffold (created = false)
  ├── identity::write_developer_file(&layout, &opts.name, now)
  ├── dev_dir = layout.workspace_developer_dir(&opts.name)
  ├── dev_dir.ensure_dir()
  ├── index_path = layout.workspace_index(&opts.name)
  ├── if !index_path.exists():
  │     └── index::seed_index(&index_path, &opts.name)
  │            // seeded with markers + empty body; first record populates
  │            // managed-block bodies via update_managed_block.
  ├── journal_path = layout.workspace_journal(&opts.name, 1)
  ├── if !journal_path.exists():
  │     └── journal::seed_journal(&journal_path, &opts.name, 1, today)
  └── return WorkspaceInitSummary { name, dev_dir, created: <bool> }
```

**Call graph for `ark agent workspace record` (manual):**

```
workspace::record::workspace_record(opts)
  ├── parent_layout = parent::resolve_parent_layout(&layout)?
  ├── name = identity::require_developer_name(&parent_layout)?
  │           → if missing: Error::DeveloperNotInitialized
  ├── cfg = WorkspaceConfig::load_or_default(&parent_layout)?
  ├── (active_journal, active_n) = journal::find_active(&parent_layout, &name)?
  ├── needs_rotate = journal::line_count(&active_journal)? >= cfg.journal_max_lines
  ├── (target_journal, target_n) = if needs_rotate {
  │       let n = active_n + 1;
  │       if n > 9999 { return Error::JournalRotationLimit { dev: name, max: 9999 } }
  │       (parent_layout.workspace_journal(&name, n), n)
  │   } else { (active_journal, active_n) };
  ├── if needs_rotate: journal::seed_journal(&target_journal, &name, target_n, today)
  ├── session_n = journal::scan_session_count(&parent_layout, &name)? + 1
  ├── branch = run_git(["symbolic-ref", "--short", "HEAD"], cwd).ok()
  │             .filter(|o| o.exit_code == 0)
  │             .map(|o| o.stdout.trim().to_string())   // None → "unknown" rendered
  ├── commits = run_git(["log", "-n", "20", "--oneline"], cwd)
  │             .map(|o| journal::parse_oneline(&o.stdout))
  │             .unwrap_or_default()                    // non-zero → empty
  ├── entry = journal::render_entry(JournalEntry { kind: Manual, session_number: session_n, ... })
  ├── target_journal.append_text(&entry)?              // PathExt::append_text per G-17
  ├── index::rerender(&parent_layout, &name)?           // re-scans journals, capped per C-21
  └── return WorkspaceRecordSummary { dev, journal_path, journal_index, session_number, rotated }
```

**Call graph for `task archive` → workspace auto-record:**

```
task::archive::task_archive(opts)
  ├── ... existing flow (validate, load TaskToml, check_transition, reserve archive path,
  │     rename to archive, save TOML with phase = Archived, deep-tier spec promotion)
  ├── ... clear .ark/tasks/.current if it pointed at slug
  │
  │     // archive_path is computed earlier in task_archive's frame and is the
  │     // post-rename location of the task dir. record_task receives a clone
  │     // through RecordTaskOptions.archive_path; G-6's fallback chain may
  │     // use it as a git cwd (it exists on disk after the rename).
  │
  ├── outcome = workspace::record::record_task(RecordTaskOptions {
  │       project_root: opts.project_root, slug, title, tier,
  │       branch, base_branch, worktree_path, archive_path, archived_at: now,
  │   })
  │     ├── parent_layout = parent::resolve_parent_layout(&layout)?
  │     ├── cfg = WorkspaceConfig::load_or_default(&parent_layout)?
  │     ├── if !cfg.auto_record_on_archive:
  │     │     └── return Ok(WorkspaceRecorded::SkippedDisabled)
  │     ├── name = match identity::read_developer_name(&parent_layout)? {
  │     │       Some(n) => n,
  │     │       None => {
  │     │           eprintln!("no developer set; skipping workspace record");
  │     │           return Ok(WorkspaceRecorded::SkippedNoIdentity);
  │     │       }
  │     │   };
  │     ├── task_cwd = resolve_task_cwd(&worktree_path, &archive_path, &parent_layout)
  │     │      // 1. worktree_path if Some + exists + is_dir
  │     │      // 2. else archive_path if exists + is_dir
  │     │      // 3. else parent_layout.root()
  │     ├── commits = match base_branch {
  │     │       Some(base) => run_git(["log", &format!("{base}..HEAD"), "--oneline", "-n", "20"], task_cwd),
  │     │       None       => run_git(["log", "-n", "20", "--oneline"], task_cwd),
  │     │   }.map(|o| journal::parse_oneline(&o.stdout)).unwrap_or_default()
  │     ├── entry = journal::render_entry(JournalEntry { kind: Task { slug }, ... })
  │     ├── target_journal.append_text(&entry)?
  │     ├── index::rerender(&parent_layout, &name)?
  │     └── return Ok(WorkspaceRecorded::Recorded { journal_path, session_number })
  ├── summary.workspace_recorded = outcome
  └── return Ok(summary)
```

[**Data Structure**]

```rust
// ark-core/src/io/path_ext.rs (NEW method on PathExt; G-17)

pub trait PathExt {
    // ...existing methods unchanged...

    /// Appends UTF-8 text to the file, creating it if absent.
    /// Equivalent to `OpenOptions::new().create(true).append(true).open(self)?
    ///                .write_all(contents.as_bytes())`.
    /// Single `write_all` call; atomic for journal-sized writes.
    /// Errors map to `Error::Io` via the existing wrapping pattern.
    fn append_text(&self, contents: &str) -> Result<()>;
}
```

```rust
// ark-core/src/commands/agent/workspace/config.rs
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_journal_max_lines")]
    pub journal_max_lines: u32,
    #[serde(default = "default_auto_record_on_archive")]
    pub auto_record_on_archive: bool,
}

impl WorkspaceConfig {
    pub fn load_or_default(layout: &Layout) -> Result<Self>;
}

impl Default for WorkspaceConfig {
    fn default() -> Self { Self { journal_max_lines: 2000, auto_record_on_archive: true } }
}

fn default_journal_max_lines() -> u32 { 2000 }
fn default_auto_record_on_archive() -> bool { true }
```

```rust
// ark-core/src/commands/agent/workspace/mod.rs (public types)

pub mod config;

#[derive(Debug, Clone)]
pub struct WorkspaceInitOptions {
    pub project_root: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct WorkspaceInitSummary {
    pub name: String,
    pub dev_dir: PathBuf,
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspaceRecordOptions {
    pub project_root: PathBuf,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceRecordSummary {
    pub dev: String,
    pub journal_path: PathBuf,
    pub journal_index: u32,
    pub session_number: u32,
    pub rotated: bool,
}

#[derive(Debug, Clone)]
pub enum WorkspaceRecorded {
    Recorded { journal_path: PathBuf, session_number: u32 },
    SkippedNoIdentity,
    SkippedDisabled,
}

// task -> workspace bridge:
#[derive(Debug, Clone)]
pub struct RecordTaskOptions {
    pub project_root: PathBuf,
    pub slug: String,
    pub title: String,
    pub tier: Tier,
    pub branch: Option<String>,
    pub base_branch: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub archive_path: PathBuf,
    pub archived_at: DateTime<Utc>,
}

// All summaries impl Display (one-line).

pub fn workspace_init(opts: WorkspaceInitOptions)     -> Result<WorkspaceInitSummary>;
pub fn workspace_record(opts: WorkspaceRecordOptions) -> Result<WorkspaceRecordSummary>;
pub fn record_task(opts: RecordTaskOptions)            -> Result<WorkspaceRecorded>;
```

```rust
// ark-core/src/commands/agent/workspace/journal.rs

#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind: JournalKind,
    pub branch: Option<String>,    // None → renders as "unknown"
    pub summary: String,
    pub commits: Vec<JournalCommit>,
    pub next_steps: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum JournalKind {
    Task { slug: String },
    Manual,
}

#[derive(Debug, Clone)]
pub struct JournalCommit {
    pub short: String,
    pub message: String,
}

/// Output of `parse_entries` — one row per `## Session N: Title` heading found.
/// Used by `index::rerender` to rebuild the sessions table without re-reading
/// the full entry body.
#[derive(Debug, Clone)]
pub struct ParsedEntry {
    pub session_number: u32,
    pub title: String,
    pub date: NaiveDate,
    pub kind_label: String,        // "task" or "manual"
    pub slug: Option<String>,      // None for manual
    pub branch: String,            // "unknown" if missing
    pub commits_count: u32,
}

pub fn render_entry(entry: &JournalEntry) -> String;
pub fn parse_entries(text: &str) -> Vec<ParsedEntry>;
pub fn find_active(layout: &Layout, dev: &str) -> Result<(PathBuf, u32)>;
pub fn line_count(path: &Path) -> Result<u32>;
pub fn scan_session_count(layout: &Layout, dev: &str) -> Result<u32>;
pub fn seed_journal(path: &Path, dev: &str, n: u32, date: NaiveDate) -> Result<()>;

/// Parse `git log --oneline` output into commit rows. Each non-empty line
/// splits at the first ASCII space: prefix is `short`, suffix (trimmed) is
/// `message`. Empty lines are skipped. Caller is responsible for capping
/// `git log` output (`-n 20`); `parse_oneline` does not impose its own cap.
pub(crate) fn parse_oneline(stdout: &str) -> Vec<JournalCommit>;
```

```rust
// ark-core/src/commands/agent/workspace/identity.rs

pub fn validate_developer_name(name: &str) -> Result<()>;
pub fn read_developer_name(layout: &Layout) -> Result<Option<String>>;
pub fn require_developer_name(layout: &Layout) -> Result<String>;
pub fn write_developer_file(layout: &Layout, name: &str, now: DateTime<Utc>) -> Result<()>;
```

```rust
// ark-core/src/commands/agent/workspace/parent.rs

pub fn resolve_parent_layout(layout: &Layout) -> Result<Layout>;
```

```rust
// ark-core/src/commands/agent/workspace/index.rs

pub fn seed_index(path: &Path, dev: &str) -> Result<()>;
pub fn rerender(layout: &Layout, dev: &str) -> Result<()>;
fn render_status_block(active_file: &str, total: u32, last_active: NaiveDate) -> String;
fn render_sessions_block(entries: &[ParsedEntry]) -> String;

const INDEX_RERENDER_JOURNAL_CAP: usize = 100;
const INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP: usize = 100;
```

```rust
// ark-core/src/error.rs (additions)

Error::DeveloperNotInitialized { path: PathBuf },
Error::DeveloperAlreadyInitialized { name: String },
Error::WorkspaceConfigCorrupt { path: PathBuf, source: toml::de::Error },
Error::JournalRotationLimit { dev: String, max: u32 },
Error::InvalidDeveloperName { name: String, reason: &'static str },
Error::ParentRootResolution { reason: String },
```

```rust
// ark-core/src/layout.rs (additions)

pub const WORKSPACE_DIR: &str = ".ark/workspace";
pub const WORKSPACE_CONFIG_FILE: &str = ".ark/workspace.toml";
pub const DEVELOPER_FILE: &str = ".ark/.developer";

#[derive(Debug, Clone)]
pub struct Layout { /* ... */ }

impl Layout {
    pub fn workspace_dir(&self) -> PathBuf;
    pub fn workspace_developer_dir(&self, dev: &str) -> PathBuf;
    pub fn workspace_index(&self, dev: &str) -> PathBuf;
    pub fn workspace_journal(&self, dev: &str, n: u32) -> PathBuf;
    pub fn workspace_config_file(&self) -> PathBuf;
    pub fn developer_file(&self) -> PathBuf;
}
```

```rust
// ark-core/src/commands/agent/task/archive.rs (additions to TaskArchiveSummary)

pub struct TaskArchiveSummary {
    // existing fields...
    pub workspace_recorded: WorkspaceRecorded,
}
```

[**API Surface**]

CLI shape (in `ark-cli/src/main.rs`):

```rust
#[derive(Subcommand)]
enum AgentCommand {
    Task(TaskArgs),
    Spec(SpecArgs),
    Workspace(WorkspaceCliArgs),    // NEW
}

#[derive(clap::Args)]
struct WorkspaceCliArgs {
    #[command(subcommand)]
    command: WorkspaceSubcommand,
}

#[derive(Subcommand)]
enum WorkspaceSubcommand {
    /// Initialize developer identity. Opt-in; not run by `ark init`.
    Init(WorkspaceInitCliArgs),
    /// Append a manual session entry to the developer's active journal.
    Record(WorkspaceRecordCliArgs),
}

#[derive(clap::Args)]
struct WorkspaceInitCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] name: String,
}

#[derive(clap::Args)]
struct WorkspaceRecordCliArgs {
    #[command(flatten)] target: TargetArgs,
    #[arg(long)] title: Option<String>,
    #[arg(long)] summary: Option<String>,
    #[arg(long)] next: Option<String>,
}
```

Library re-exports added to `ark-core/src/lib.rs`:

```rust
agent::{
    // ...existing...
    workspace::{
        WorkspaceConfig,
        WorkspaceInitOptions, WorkspaceInitSummary,
        WorkspaceRecordOptions, WorkspaceRecordSummary,
        WorkspaceRecorded,
        RecordTaskOptions,
        workspace_init, workspace_record, record_task,
    },
}
```

`PathExt::append_text` is added via the existing `PathExt` re-export (no new public name in `lib.rs`). Internal helpers (`identity::*`, `journal::*` except `parse_oneline` which is `pub(crate)`, `index::*`, `parent::*`) are NOT re-exported.

[**Constraints**]

- **C-1 (process spawn locality):** All git invocations route through `io::git::run_git`. `Command::new` MUST NOT appear under `commands/agent/workspace/`. Extends the existing source-scan test.
- **C-2 (path/io discipline):** All filesystem access in `commands/agent/workspace/` routes through `io::PathExt` (no bare `std::fs::*`, no bare `std::fs::OpenOptions`). All `.ark/`-relative paths route through `Layout` helpers (no string concatenation). The append primitive is `PathExt::append_text` (G-17); raw `OpenOptions::append` is forbidden in this module.
- **C-3 (developer name validation):** Name MUST match `^[A-Za-z][A-Za-z0-9_-]{0,39}$` — leading ASCII letter, then up to 39 more characters from `[A-Za-z0-9_-]`. Empty, whitespace-only, leading digit, leading dash, contains `/` or `\\`, contains `.`, > 40 chars, non-ASCII → `Error::InvalidDeveloperName { name, reason }`. Filesystem hostility: name is also the *directory name* under `.ark/workspace/<name>/`. Cross-platform OS-reserved names are NOT pre-validated.
- **C-4 (`.developer` file format):** Two key=value lines, `\n`-separated, UTF-8: `name=<x>\ninitialized_at=<RFC3339>\n`. Reader is line-oriented; ignores blank lines and unknown keys for forward compat. Trailing newline required.
- **C-5 (idempotent init):** Running `workspace init --name <x>` twice with the same name on a fully-scaffolded workspace is a no-op (`created: false`). With a different name → `Error::DeveloperAlreadyInitialized` (no automatic rename).
- **C-6 (parent-write invariant):** `workspace_record` and `record_task` MUST write to `parent_layout.workspace_dir()`, never to `layout.workspace_dir()` when those differ. The git invocations (commit collection) inside `workspace_record` use the **caller's cwd**, but all file writes target the parent.
- **C-7 (snapshot exclusion):** `commands/unload.rs` MUST skip `layout.developer_file()` in **both** filesystem walks: the Stage A `unload` capture loop AND the Stage B `capture_orphan_hook_entries` walk. Implementation: append `layout.developer_file()` to each call's existing `skip_under` slice. The existing `walk_files_excluding` matches via `Path::starts_with`; a single file path is a valid prefix, so no API change is needed. The file is gitignored and per-machine — capturing it would leak identity. `.ark/workspace/<dev>/` is captured normally.
- **C-8 (managed-block re-render):** `index.md` is regenerated from journal contents on every `workspace_record` and `record_task` via `update_managed_block` for `ARK:WORKSPACE_STATUS` and `ARK:WORKSPACE_SESSIONS`. Hand-edits *outside* the managed blocks are preserved.
- **C-9 (journal rotation):** Rotate when `line_count(active) >= cfg.journal_max_lines` AT THE START of a record call. `cfg.journal_max_lines < 100` → `Error::InvalidConfigField`. `N+1 > 9999` → `Error::JournalRotationLimit`.
- **C-10 (workspace.toml lifecycle):** Created by `ark init` from `templates/ark/workspace.toml`. `ark upgrade` does NOT overwrite it.
- **C-11 (auto-record best-effort):** `record_task` MUST NOT propagate `Error::DeveloperNotInitialized` — returns `WorkspaceRecorded::SkippedNoIdentity`. `auto_record_on_archive = false` → returns `WorkspaceRecorded::SkippedDisabled` without reading identity or invoking git. All other errors propagate. `task_archive` does NOT roll back the archive on workspace failure.
- **C-12 (output stability):** Every workspace command writes a single `Display` summary; no ad-hoc stdout writes. The single `eprintln!` in `record_task`'s skip-no-identity path is the documented exception (stderr, not stdout).
- **C-13 (commit collection cap):** `git log` invocations cap at 20 entries via `-n 20`. For `record_task`, range is `<base_branch>..HEAD` when `base_branch` is `Some`; fallback to `-n 20 --oneline` when `None`. For manual `workspace_record`, always `-n 20 --oneline`. Output silently truncated; no truncation header. Parsed via `journal::parse_oneline` (a `pub(crate)` helper documented in Data Structure): splits each non-empty line at the first ASCII space, prefix is the short hash and suffix (trimmed) is the message.
- **C-14 (CLI hidden under agent):** `ark agent workspace` follows the same hidden semantics as the rest of `ark agent`. `ark --help` does NOT list `agent`; `ark agent workspace --help` is reachable and includes "Not covered by semver".
- **C-15 (one-way coupling):** `task::archive` imports `super::workspace::record::record_task`. `workspace::*` modules MUST NOT import `super::task` or `super::spec`. Dependency direction enforced by code review.
- **C-16 (gitignore template lifecycle):** `templates/ark/.gitignore` is fully Ark-owned (no managed block). `ark upgrade` reapplies the canonical content.
- **C-17 (slash command parity):** `templates/claude/commands/ark/record.md`, `templates/codex/skills/ark-record/SKILL.md`, and `templates/opencode/commands/ark/record.md` MUST stay in lockstep on every change to the slash command body.
- **C-18 (auto-record placement, tier-agnostic):** `task::archive::task_archive` MUST invoke `record_task` exactly once per call, **after** the `.current` cleanup block AND **before** the `Ok(summary)` return, **regardless of tier** (quick / standard / deep). The deep-tier `spec_extract` + `spec_register` block runs earlier in `task_archive` and is unaffected by this change. `record_task` receives `archive_path` via `RecordTaskOptions` — it is a clone of the same value `task_archive` computed earlier in its frame, after the rename, and points to the now-archived task dir on disk. Regression tests: `quick_tier_archive_records_session`, `standard_tier_archive_records_session`, `deep_tier_archive_records_session`.
- **C-19 (entry-boundary parser predicate):** `parse_entries(text: &str) -> Vec<ParsedEntry>` operates on the concatenated text of one journal file. Boundary rules:
  - **Entry start:** a line matching `^## Session (\d+):\s+(.+?)\s*$` outside of a fenced code block.
  - **Entry end:** the *next* line matching the start regex outside of a fenced code block, OR EOF.
  - **Fenced code blocks:** the parser recognizes both backtick fences and tilde fences. The opening-fence regex is `^([` + "`" + `~]{3,})(\w+)?\s*$` (line-start, 3+ of either character, optional language tag). The matching close fence MUST use the **same character type** AND be **at least as long** as the opening fence. Lines inside an active fenced block are ignored for boundary detection. A fence opened but never closed at EOF is treated as still-open (everything after the open fence is excluded from boundary detection — defensive against malformed pastes).
  - **Indented code blocks (CommonMark 4-space rule) are NOT treated as fenced** by this parser. A `## Session N:` line with 4 leading spaces is technically code in CommonMark, but the parser still matches it as an entry start. This is a deliberate simplification (NG-13).
  - **Within an entry body**, the parser extracts: `**Date**: YYYY-MM-DD` (first match), `**Kind**: (task|manual)`, `**Slug**: <slug>` (or `-`), `**Branch**: \`<branch>\``, and the `### Commits` table row count (lines under `### Commits` matching `^\| \``).
  - Malformed entries (missing required fields) are silently skipped during re-render with a single stderr warning per re-render call; the journal file itself is not modified.
- **C-20 (parent-root resolution detection algorithm):** `parent::resolve_parent_layout(layout)` follows this branch tree:
  1. Let `dot_git = layout.root().join(".git")`.
  2. If `dot_git.is_dir()` → regular checkout (or symlink-to-dir, which `is_dir()` follows): return `layout.clone()`.
  3. If `dot_git.is_file()` → worktree pointer. Run `run_git(&["rev-parse", "--git-common-dir"], layout.root())`. On non-zero exit → `Error::ParentRootResolution { reason: stderr.trim() }`. The output may be relative (to `layout.root()`) or absolute. Resolve to absolute via `layout.root().join(stdout.trim())` (lexical join).
     - **Best-effort canonicalize:** if `canonicalize` succeeds, use the canonical form; if it fails (read-only FS, permission denied, missing component), fall back to the lexical path with a single `pop()` to strip the trailing `.git` component. The result is the parent root path. Return `Layout::new(parent_root)`.
     - Only error (`Error::ParentRootResolution { reason }`) if even the lexical fallback cannot produce a usable path (e.g. the git stdout is empty or doesn't end in `.git`).
  4. If `dot_git` does not exist (file-not-found) → return `layout.clone()` (non-git project; record locally).
  5. Otherwise (`dot_git` is symlink-to-non-existent, pipe, etc.) → `Error::ParentRootResolution { reason: "unrecognized .git type" }`.

- **C-21 (index re-render hard ceiling):** `index::rerender(&parent_layout, &dev)` reads at most **100 journal files** (`journal-1.md` … `journal-100.md`) and parses at most **100 entries per file**. Beyond the cap, additional entries are silently truncated from the rendered sessions table; the journal files themselves remain canonical. The cap is a constant in `index.rs`: `INDEX_RERENDER_JOURNAL_CAP: usize = 100`, `INDEX_RERENDER_ENTRIES_PER_JOURNAL_CAP: usize = 100`.

---

## Runtime `runtime logic`

[**Main Flow — `workspace init`**]
1. CLI parses `--name <x>` and `TargetArgs`.
2. `validate_developer_name(&x)`.
3. Read `.ark/.developer`; if present and name ≠ `<x>` → `Error::DeveloperAlreadyInitialized`. If present and name == `<x>`, fall through (idempotent re-scaffold).
4. Write `.ark/.developer` (`name=<x>\ninitialized_at=<now>\n`).
5. `mkdir -p .ark/workspace/<x>/`.
6. Seed `index.md` from `_workspace_index.md` template with `{{name}}` substitution.
7. Seed `journal-1.md` from `_workspace_journal.md` template with `{{name}}`, `{{n}}=1`, `{{date}}=today`.
8. Print one-line summary `developer "<x>" initialized at .ark/workspace/<x>`.

[**Main Flow — `workspace record` (manual)**]
1. CLI parses `[--title <t>] [--summary <s>] [--next <n>]` and `TargetArgs`.
2. `resolve_parent_layout(&layout)` → parent or self per C-20.
3. `require_developer_name(&parent_layout)` → required, else `Error::DeveloperNotInitialized`.
4. `WorkspaceConfig::load_or_default(&parent_layout)`.
5. `find_active(&parent_layout, &dev)` → `(active_path, active_n)`.
6. Check `line_count(active_path) >= cfg.journal_max_lines` → maybe rotate (C-9).
7. `scan_session_count(&parent_layout, &dev)` → next session number.
8. Branch from `run_git(["symbolic-ref","--short","HEAD"], cwd)`.
9. Commits from `run_git(["log","-n","20","--oneline"], cwd)`, parsed via `parse_oneline`.
10. Build `JournalEntry`.
11. Append rendered entry to `target_journal` via `PathExt::append_text`.
12. `index::rerender(&parent_layout, &dev)` rebuilds both managed blocks (capped per C-21).
13. Print one-line summary `recorded session <N> to <journal_path>`.

[**Main Flow — `task archive` auto-record**]
1. `task_archive` completes its existing flow up through `.current` cleanup.
2. After `.current` cleanup AND before `Ok(summary)` return: invoke `record_task(opts)` with task-derived inputs.
3. `record_task` resolves parent layout, loads config. If `auto_record_on_archive = false` → returns `SkippedDisabled`.
4. Reads identity. Missing → returns `SkippedNoIdentity` + stderr line.
5. Resolves task-cwd via the three-step fallback chain (G-6).
6. Renders a `kind = task` entry with commits from `<base_branch>..HEAD` (or `-n 20` fallback).
7. Appends to journal via `PathExt::append_text`, re-renders index — same as manual flow.
8. Returns `WorkspaceRecorded` enum back into `TaskArchiveSummary`.

[**Failure Flow**]
- `Error::DeveloperNotInitialized` (path) — manual `workspace record` errors out cleanly. Auto-record converts to `SkippedNoIdentity` + stderr line, archive succeeds.
- `Error::DeveloperAlreadyInitialized` (name) — `workspace init` aborts before any disk write.
- `Error::WorkspaceConfigCorrupt` (path, source) — both `workspace_record` and `record_task` propagate. `task_archive` propagates (archive directory rename is already committed).
- `Error::InvalidDeveloperName` (name, reason) — pre-flight, no writes.
- `Error::JournalRotationLimit` (dev, max) — defensive; only reachable at >9999 journals.
- `Error::ParentRootResolution` (reason) — `git rev-parse --git-common-dir` failed inside a presumed worktree, or `.git` is an unrecognized type, or even the lexical fallback for path resolution fails. Both `workspace_record` and `record_task` propagate.
- `Error::Io` — wraps any FS error from `PathExt` (including `append_text` failures).
- `update_managed_block` failure (orphan marker) → `Error::ManagedBlockCorrupt` (existing). User must hand-fix `index.md`. Journal data is unaffected.

[**State Transitions**]
- `.developer` absent → identity-uninitialized; auto-record skips silently. Manual record errors.
- `.developer` present, `<dev>/` missing → init failure mid-flight from a previous run; `workspace init` (with same name) idempotent-completes the scaffold.
- `<dev>/journal-N.md` exists, `<dev>/journal-{N+1}.md` does not, current at line cap → next record creates `journal-{N+1}.md`. The old file is closed for new writes.
- `index.md` managed block missing → on next record, `update_managed_block` writes the marker (idempotent insert); orphan marker → `Error::ManagedBlockCorrupt` (no auto-recovery).

---

## Implementation `split task into phases`

[**Phase 1 — core module + identity + Layout**]
- Add `Layout` constants/methods. Verify `Layout: Clone` is derived; add if missing.
- Add error variants in `error.rs`.
- Create `commands/agent/workspace/{mod.rs, identity.rs, parent.rs, config.rs}` with stubs returning sensible defaults. `identity.rs` complete.
- Wire up `lib.rs` re-exports for the public types declared so far.
- Tests: `validate_developer_name`, `read_developer_name_round_trip`, `WorkspaceConfig::load_or_default`.

[**Phase 2 — `PathExt::append_text` + journal + index rendering + entry parser**]
- Add `PathExt::append_text` to `crates/ark-core/src/io/path_ext.rs` per G-17.
- `journal.rs`: `JournalEntry`, `render_entry`, `parse_entries` (per C-19), `find_active`, `line_count`, `scan_session_count`, `seed_journal`, `parse_oneline`. `ParsedEntry` struct.
- `index.rs`: `seed_index`, `rerender` (capped per C-21), internal `render_*_block` helpers.
- Templates: add `templates/ark/_workspace_index.md` and `_workspace_journal.md` to embedded tree.
- Tests: `path_ext_append_text_creates_and_appends`, `render_entry_golden`, `parse_entries_round_trip` (multi-entry with embedded backtick + tilde fences), `parse_entries_skips_malformed_with_stderr`, `parse_oneline_strips_and_skips_blanks`, `rerender_idempotent`, `rerender_caps_at_100_journals_x_100_entries`, `find_active_picks_highest_N`, `seed_index_includes_markers`.

[**Phase 3 — `workspace init` + `workspace record` CLI**]
- `init.rs`: full implementation per the call graph.
- `record.rs`: full `workspace_record`. `record_task` is a separate fn with its own option struct.
- CLI wiring in `ark-cli/src/main.rs`: add `Workspace(WorkspaceCliArgs)` to `AgentCommand`; dispatch + render.
- Tests: `workspace_init_creates_files`, `workspace_init_idempotent_same_name`, `workspace_init_rejects_different_name`, `workspace_record_appends_to_journal`, `workspace_record_rotates_at_cap`, `workspace_record_rerenders_index`, `workspace_record_no_identity_errors`, `cli_shape_workspace_init_record_parse`.

[**Phase 4 — `task archive` integration + parent resolution**]
- `parent.rs`: `resolve_parent_layout` per C-20 four-branch detection with lexical-fallback path resolution.
- Wire `record_task` into `task::archive::task_archive` per C-18 placement (after `.current` cleanup, before `Ok` return, regardless of tier). Add `workspace_recorded` to `TaskArchiveSummary`; update `Display`.
- Tests: `quick_tier_archive_records_session`, `standard_tier_archive_records_session`, `deep_tier_archive_records_session`, `archive_skips_when_no_identity_with_stderr`, `archive_skips_when_auto_record_disabled`, `archive_from_worktree_writes_to_parent`, `parent_resolution_in_regular_repo_returns_self`, `parent_resolution_in_worktree_returns_parent`, `parent_resolution_canonicalize_falls_back_to_lexical`, `parent_resolution_unrecognized_dot_git_errors`, `record_task_falls_back_when_worktree_dir_missing`, `archive_records_to_workspace_when_identity_set`.

[**Phase 5 — `templates/`, `.gitignore`, slash command, snapshot exclusion**]
- `templates/ark/.gitignore`: append `.developer`.
- `templates/ark/workspace.toml`: ship commented defaults.
- `templates/claude/commands/ark/record.md`: new slash command body.
- Codex skill + OpenCode command parity.
- `commands/unload.rs`: file-level skip for `layout.developer_file()` in BOTH walk sites (C-7).
- Workflow doc: §6 Workspace subsection. AGENTS.md: one row.
- Tests: `unload_skips_developer_file_in_stage_a`, `unload_skips_developer_file_in_orphan_walk`, `gitignore_includes_developer_after_init`, `upgrade_does_not_overwrite_workspace_toml`, slash-command parity tests.

---

## Trade-offs `ask reviewer for advice`

(All resolved in iter 00; iter 01 introduced no new trade-offs; iter 02 introduces one minor design choice already adopted as Option A.)

- **T-1 (slash command shape):** Option A (one-shot). Resolved iter 00.
- **T-2 (`workspace.toml` placement):** Option A. Resolved iter 00.
- **T-3 (parent-root resolution):** Resolved via C-20 (now with lexical fallback per iter 02 R-004).
- **T-4 (auto-record toggle plumbing):** Option A. Resolved iter 00.
- **T-5 (index re-render strategy):** Resolved via C-21.
- **T-6 (`record_task` location):** Option B. Resolved iter 00.
- **T-7 (NEW iter 02, append API choice):** **Resolved as Option A** (add `PathExt::append_text`). Rationale: Option B (read-modify-rewrite) costs O(file_size) per record, bounded by `journal_max_lines × ~50 bytes ≈ 100 KB`, but more importantly violates the spirit of `PathExt` as the documented FS surface — adding an append primitive to `PathExt` makes the API complete, not bypassed. Other ark-core consumers (e.g. future logging) benefit. Implementation is ~10 lines.

---

## Validation `test design`

[**Unit Tests**]
- **V-UT-1 (G-3, C-3):** `validate_developer_name` accepts `kleinhe`, `dev_1`, `a-b`, `A`, 40-char strings starting with a letter; rejects empty, whitespace, `..`, `dev/path`, `/abs`, `dev\\back`, `-leading`, `1leading`, `0`, 41-char strings, non-ASCII.
- **V-UT-2 (G-3, C-4):** `read_developer_name` returns `None` for missing file, `Some(name)` for valid file, `None` for malformed file. Tolerates blank lines and unknown keys.
- **V-UT-3 (G-8, C-9, C-10):** `WorkspaceConfig::load_or_default` returns `Default` for missing file, parsed config for valid TOML, `Error::WorkspaceConfigCorrupt` for malformed TOML, `Error::InvalidConfigField` for `journal_max_lines < 100`.
- **V-UT-4 (G-5):** `render_entry` produces byte-identical output to a golden fixture.
- **V-UT-5 (G-5, C-19):** `parse_entries` round-trips: render → parse → render produces the same bytes for a multi-entry fixture, including entries with embedded **backtick AND tilde** fenced code blocks containing `## Session 999: trick` lines that MUST NOT be parsed as new entries. Also tests length-aware close-fence matching: a 4-backtick block containing 3-backtick text inside is parsed as a single fence.
- **V-UT-6 (G-4):** `find_active` picks the highest-numbered `journal-N.md` numerically.
- **V-UT-7 (G-4, C-9):** `line_count` returns exact line count; rotation logic triggers when count ≥ cap.
- **V-UT-8 (G-9, C-20):** `resolve_parent_layout` returns `layout.clone()` when `<root>/.git` is a directory; returns parent layout when `.git` is a file (mocked); returns `layout.clone()` when `.git` is absent; errors with `Error::ParentRootResolution` when `.git` is unsupported; **lexical-fallback subcase**: when `canonicalize` fails (mock unsupported FS), the lexical pop-`.git` path is returned successfully.
- **V-UT-9 (G-2, G-18):** `ark_init_with_no_developer_flag_skips_identity` — `ark init --no-developer` creates platform integrations; `.ark/.developer` and `.ark/workspace/` do NOT exist.
- **V-UT-10 (G-10):** `cli_shape_workspace_init_record_parse`.
- **V-UT-13 (G-18):** `ark_init_with_developer_flag_bootstraps_identity` — `ark init --developer alice` creates `.ark/.developer` (`name=alice`), `.ark/workspace/alice/index.md` (with markers), and `.ark/workspace/alice/journal-1.md`.
- **V-UT-14 (G-18, C-3):** `ark_init_with_invalid_developer_name_errors` — `ark init --developer 1leading` errors with `InvalidDeveloperName` before platform extraction.
- **V-UT-11 (G-17):** `path_ext_append_text_creates_and_appends` — calling `append_text` on a missing file creates it; calling again appends; final content matches concatenation of inputs.
- **V-UT-12 (C-13):** `parse_oneline_strips_and_skips_blanks` — input `"abc1234 first\n\nfedcba9 second\n"` → two `JournalCommit` entries; whitespace trimmed; empty lines skipped.

[**Integration Tests**]
- **V-IT-1 (G-1, G-3, G-13, G-14):** `workspace_init_creates_files`.
- **V-IT-2 (C-5):** `workspace_init_idempotent_same_name` + `workspace_init_rejects_different_name`.
- **V-IT-3 (G-1, G-4, G-5, C-8, G-17):** `workspace_record_appends_to_journal` (also exercises `append_text`).
- **V-IT-4 (G-4, C-9, C-21):** `workspace_record_rotates_at_cap` + `rerender_caps_at_100_journals_x_100_entries`.
- **V-IT-5 (G-7, C-18):** `deep_tier_archive_records_session`.
- **V-IT-6 (C-6):** `archive_from_worktree_writes_to_parent`.
- **V-IT-7 (C-7, C-10, C-16):** `unload_load_round_trip_workspace`, `upgrade_does_not_overwrite_workspace_toml`, `gitignore_includes_developer_after_init`.
- **V-IT-8 (G-7, C-18):** `quick_tier_archive_records_session` and `standard_tier_archive_records_session`.
- **V-IT-9 (C-7):** `unload_skips_developer_file_in_orphan_walk`.

[**Failure / Robustness Validation**]
- **V-F-1 (G-2, C-11):** `archive_skips_when_no_identity_with_stderr`.
- **V-F-2 (NG-6, C-11):** `archive_propagates_workspace_config_corrupt`.
- **V-F-3 (G-9, C-20):** `parent_resolution_unrecognized_dot_git_errors` + `parent_resolution_canonicalize_falls_back_to_lexical`.
- **V-F-4 (G-2):** `workspace_record_no_identity_errors`.
- **V-F-5 (C-8):** `index_managed_block_orphan_marker_errors`.
- **V-F-6 (G-7, C-11):** `archive_skips_when_auto_record_disabled`.

[**Edge Case Validation**]
- **V-E-1 (C-9):** `journal_max_lines = 99` → `Error::InvalidConfigField`.
- **V-E-2 (C-3):** Developer name `con` (Windows-reserved) — accepted at validation; OS may reject. Documented in C-3.
- **V-E-3 (G-6):** `record_task` with `base_branch == None` → fallback to `git log -n 20 --oneline`.
- **V-E-4 (G-6):** Task archived inside a non-git directory → empty commits table, no error.
- **V-E-5 (G-4):** Two rapid records — last-writer's index re-render wins. We don't lock.
- **V-E-6 (G-15):** `/ark:record` with empty conversation context — slash-command-only behavior.
- **V-E-7 (G-6):** `record_task_falls_back_when_worktree_dir_missing`.
- **V-E-8 (C-19):** `parse_entries_with_tilde_fences` — fenced code block opened with `~~~` must hide an enclosed `## Session N:` line. Distinct from V-UT-5's backtick-only fixture.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1, V-IT-3, V-UT-10 |
| G-2 | V-UT-9, V-F-1, V-F-4, V-F-6 |
| G-3 | V-UT-1, V-UT-2, V-IT-1, V-IT-2 |
| G-4 | V-UT-6, V-UT-7, V-IT-3, V-IT-4, V-E-5 |
| G-5 | V-UT-4, V-UT-5, V-IT-3 |
| G-6 | V-IT-5, V-E-3, V-E-4, V-E-7 |
| G-7 | V-IT-5, V-IT-8, V-F-1, V-F-2, V-F-6 |
| G-8 | V-UT-3, V-IT-4, V-F-6 |
| G-9 | V-UT-8, V-IT-6, V-F-3 |
| G-10 | V-UT-10, V-IT-1, V-IT-3 |
| G-11 | V-IT-7 |
| G-12 | V-IT-7 |
| G-13 | V-IT-1 |
| G-14 | V-IT-1 |
| G-15 | V-E-6 |
| G-16 | (doc-only; verified by review at archive time) |
| G-17 | V-UT-11, V-IT-3 |
| G-18 | V-UT-9, V-UT-13, V-UT-14 |
| C-1  | (extends existing source-scan test) |
| C-2  | (extends existing source-scan test; `OpenOptions::append` forbidden) |
| C-3  | V-UT-1 |
| C-4  | V-UT-2 |
| C-5  | V-IT-2 |
| C-6  | V-IT-6 |
| C-7  | V-IT-7, V-IT-9 |
| C-8  | V-IT-3, V-IT-4, V-F-5 |
| C-9  | V-UT-7, V-IT-4, V-E-1 |
| C-10 | V-IT-7 |
| C-11 | V-F-1, V-F-2, V-F-6 |
| C-12 | (existing pattern) |
| C-13 | V-IT-5 (range form), V-E-3 (fallback), V-E-4 (non-git), V-UT-12 (parse_oneline) |
| C-14 | (existing) |
| C-15 | (review-time inspection) |
| C-16 | V-IT-7 |
| C-17 | (slash-command parity test extension) |
| C-18 | V-IT-5, V-IT-8 |
| C-19 | V-UT-5, V-E-8 |
| C-20 | V-UT-8 (incl. lexical-fallback subcase), V-IT-6, V-F-3 |
| C-21 | V-IT-4 |
