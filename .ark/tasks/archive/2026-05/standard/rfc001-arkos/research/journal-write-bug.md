# Bug: `task_commit` workspace-journal write misplaces stamp + emits empty `### Git Commits`

Investigation of the malformed journal diff produced by `ark agent task commit -m "feat(rfc): add rfc001-arkos"` (commit `fcfd341`, branch `docs/rfc001-arkos`).

## TL;DR

The root cause is a **missing slash-command-side contract step**: the agent never appended a `## Session 15: rfc001-arkos` heading to `journal-1.md` before invoking `ark agent task commit`. The CLI's `stamp_task` is defined to find the *last* `## Session N:` heading and inject auto-fields beneath it — so it stamped beneath the already-stamped Session 14 heading instead, producing two metadata blocks under one heading. The empty `### Git Commits` rendering is a separate, structurally-unavoidable CLI cosmetic issue: at write-time the closing commit hasn't landed yet.

---

## Q1: Where does the journal-write code decide *where* to insert the new entry?

The insertion point is the result of `locate_last_session_block` in
`crates/ark-core/src/commands/agent/workspace/stamp.rs:153-170`:

```rust
fn locate_last_session_block(text: &str) -> Result<SessionSplit> {
    let mut last: Option<(usize, usize)> = None;
    let mut cursor = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("## Session") {
            last = Some((cursor, cursor + line.len()));
        }
        cursor += line.len();
    }
    ...
}
```

The function returns the byte range of the **last** `## Session ...` heading in the file. `stamp_task` (`stamp.rs:74-98`) then splits the journal at that heading and inserts the rendered auto-fields immediately after the heading's trailing newline, before whatever body followed.

Caller chain:
- `task_commit` → `record_workspace_journal` (`commands/agent/task/commit.rs:441-492`)
- → `workspace_record` (`workspace/record.rs:124-222`)
- → `stamp_task` (`workspace/record.rs:162-172`, `workspace/stamp.rs:74-98`)

The Git Commits sub-table is rendered separately by `render_git_commits_block` (`stamp.rs:192-203`) and inserted by `insert_after_main_changes` (`stamp.rs:210-233`), which walks from the located heading forward to the **last `|`-prefixed line** within whatever follows the heading and inserts the new block after it.

## Q2: Why did it land between `## Session 14: ark cleanup` and that session's body?

Because the agent never appended a `## Session 15: rfc001-arkos` heading. Looking at the file state immediately before this commit:

- Pre-stamp (`e0e8499:.ark/workspace/Anekoique/journal-1.md`): file ends at Session 14's Git Commits table (line 385).
- Post-stamp (`fcfd341`): metadata block injected at line 361 (between the `## Session 14: ark cleanup` heading on line 359 and Session 14's existing auto-fields starting line 368).

`locate_last_session_block` scans for `^## Session` — the only matching heading was Session 14's. The function has no awareness of whether a heading "already has" a stamp; it always picks the textually-last one. So the new auto-fields nested under Session 14 and the new `### Git Commits` block landed after Session 14's existing Git Commits table — see the second `### Git Commits` at line 394 (`stamp.rs:217-219` walks line-by-line for `|`-prefixed lines, finding Session 14's old commit rows last).

There are **no managed-block markers** (`<!-- ARK:... -->`) for journal sessions — only the personal index uses managed blocks (`record.rs:316-318`, marker `SESSIONS_MARKER`). Journal-side, the "anchor" is purely the last `## Session` heading; the stamp is irreversibly destination-dependent on the agent having written one.

Tertiary effects of the same root cause:
- `scan_session_count` (`record.rs:279-304`) returned `14` (the number of existing `## Session` headings), so the personal-index row was upserted as session `#14` — duplicating row 14 (see `.ark/workspace/Anekoique/index.md:23-24`, two rows numbered 14).
- `extract_last_session_title` (`stamp.rs:131-142`) returned `"ark cleanup"` (Session 14's title), so the duplicate row 14 reads `ark cleanup` instead of `rfc001-arkos`.

## Q3: Who is responsible for writing the `## Session N: <title>` heading?

**The slash command is contractually responsible.** Both Claude and Codex commit prompts spell this out:

- `templates/claude/commands/ark/commit.md:42-66` ("Step 4: Append the journal entry"):
  > If `.ark/.developer` exists, append a session block to the active journal … `## Session N: <title>` / `### Summary` / `### Main Changes` table.
  > The CLI inserts `**Date**`, `**Slug**`, … after your `## Session N:` heading. **Do not write them.**

- `templates/codex/skills/ark-commit/SKILL.md:42-66` — identical wording.
- `templates/opencode/commands/ark/commit.md` — same (per `grep "Session N"`).

The CLI **assumes** the agent did this:
- `record.rs:138-141` errors `EntryFileMalformed` only when the journal file does not exist (not when the file exists but lacks a fresh heading).
- `record.rs:271-273` only errors when no `journal-N.md` files exist at all.
- `stamp.rs:163-166` errors only when there is **no** `## Session` heading whatsoever (file has zero historic headings).

There is **no precondition** in `record_workspace_journal` or `stamp_task` that verifies a *new* `## Session N:` heading was added since the previous stamp. The contract is fragile and silently misbehaves when violated. Answer: **(a) the slash command is responsible and failed**, and the CLI did not detect the failure.

## Q4: Is the empty `### Git Commits` table cosmetic or buggy?

It is **structurally unavoidable** with the current ordering and a deliberate-but-misleading rendering:

`render_git_commits_block` (`stamp.rs:192-203`):
```rust
if commits.is_empty() {
    s.push_str("| Hash | Message |\n|------|---------|\n| _(none)_ |   |\n");
}
```

The empty list comes from `collect_commits_in_range` (`commands/agent/task/commit.rs:498-526`), which runs `git log <start_head>..HEAD --format=%h %s` *before* the closing commit is created (`task_commit` writes the journal at line 201-205, then commits at line 250). For the `rfc001-arkos` task, `start_head = e0e8499` and pre-commit `HEAD = e0e8499` → empty range.

So the rendering `| _(none)_ |   |` is what the code emits whenever the closing commit is the first commit of the task — which is the common case for quick-tier tasks created and committed within one session. The closing commit's own SHA is never available at journal-write time (it doesn't exist yet); a subsequent backfill step would be required (compare `ec56fc0` "fix: fix historical workspace record", which manually rewrote `<PENDING:...>` sentinels to real SHAs).

Compare to the `**Closing Commit**: <PENDING:rfc001-arkos>` sentinel (`stamp.rs:174-181`) — that field acknowledges the same write-time-before-commit constraint by using an explicit placeholder. The Git Commits table should arguably do the same (e.g. `| <PENDING> | _(closing commit pending)_ |`) or be omitted entirely when empty.

**Verdict: structurally-correct logic, misleading UX. A cosmetic bug worth a small fix; not the load-bearing defect.**

## Q5: Recent history — regression candidate?

`git log --oneline -- crates/ark-core/src/commands/agent/workspace/`:
```
f14837c fix: init workspace correctly
6a796a1 feat(workflow): add workspace support
73d46ba feat!: remove workspace support
7ed2b8b refactor(workflow): refactor ark-task lifecycle and `ark archive`
a6513b8 feat(workflow): add ark-task concurrency control
f0fbb56 style: format all comments with SPEC
7962c24 feat(workflow): add workspace management
```

`git log --oneline -- crates/ark-core/src/commands/agent/task/commit.rs`:
```
79500fd refactor(state): replace per-session focus map with per-checkout
1a129d8 chore: archive and bump version
6a796a1 feat(workflow): add workspace support
73d46ba feat!: remove workspace support
7ed2b8b refactor(workflow): refactor ark-task lifecycle and `ark archive`
```

The workspace feature was re-introduced in **`6a796a1`** (reverted by `73d46ba`, then re-added). The current stamping logic (last-heading-wins) is **part of the initial design**, not a regression. `f14837c` ("init workspace correctly") fixed scaffolding-during-`ark init`, not the stamp behaviour. `ec56fc0` ("fix historical workspace record") was a one-off manual data-fix on Session 1, not a code change.

This is not a regression — it is the same fragility the workspace feature has carried since `6a796a1`. The contract simply was never enforced.

## Q6: Did Session 14 ever have this same bug?

No. Inspecting `e0e8499` (the commit that added Session 14) — the journal diff appends a fresh `## Session 14: ark cleanup` heading followed by its own properly-stamped block, then `### Summary`, `### Main Changes`, `### Git Commits` with real SHAs (`ec56fc0`, `258f187`). Session 14 was written correctly because the agent **did** append the heading that time.

Earlier sessions (1–13) all show normal layout (one heading, one metadata block, one Summary, one Main Changes, one Git Commits) per `git --no-pager show e0e8499:.ark/workspace/Anekoique/journal-1.md | grep -n "## Session"`. The bug is unique to `fcfd341` and is contract-violation-induced.

---

## Suggested next step

Scope for an Ark task to fix this — **standard tier**, single CLI change + template clarification + one extra rendering touch-up:

1. **CLI-side enforcement (load-bearing fix).** In `stamp_task`, detect that the last `## Session N:` heading already has a stamped `**Date**:` / `**Slug**:` line immediately following it. If yes, refuse with a new `Error::JournalEntryMissing` variant carrying the journal path and a hint string ("agent must append `## Session N+1: <title>` before invoking `ark agent task commit`"). Suggested location: insert the check between `locate_last_session_block` and the body split at `stamp.rs:78-80`. Add equivalent guard in `stamp_manual`. Mirror in `scan_session_count` so the off-by-one in personal-index row numbering can't slip past the guard.

2. **Empty Git Commits cosmetic.** Change `render_git_commits_block` (`stamp.rs:192-203`) to render `| <PENDING> | _(closing commit pending)_ |` when empty (consistent with `<PENDING:slug>` sentinel) — or omit the table block entirely when empty. Decision worth a sentence in PRD.

3. **Template clarification (defence-in-depth).** Both `templates/claude/commands/ark/commit.md` (Step 4) and `templates/codex/skills/ark-commit/SKILL.md` (Step 4) phrase the heading rule as "append a session block"; reinforce with an explicit *imperative* line: "ALWAYS write a fresh `## Session N: <title>` heading even when reusing the same journal file — the CLI does not infer or generate this heading."

4. **Tests.** Add to `stamp.rs` tests: a case where the journal already ends in a stamped block (re-invoking `stamp_task` without a new heading) — must error, not double-stamp. Add to `record.rs` tests: parallel case asserting `Error::JournalEntryMissing` is surfaced and the personal index is **not** updated.

5. **Data fix (one-off, manual).** Rewrite `.ark/workspace/Anekoique/journal-1.md` to split Session 14's nested block into a proper `## Session 15: rfc001-arkos` block, and fix the duplicate `| 14 |` row in `.ark/workspace/Anekoique/index.md`. The user is already doing this in parallel (see `faba3af` diff).

Coverage: the fix is contained to `stamp.rs` + `record.rs` + the two template files; ~40 LOC + tests. Quick-tier might be tempting, but the new error variant and the template change cross enough boundaries to justify standard tier with a short PLAN.
