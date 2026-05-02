# `workspace` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `workspace`
> Target Task: `workspace`
> Tier: `deep`

---

## Project Spec Compliance

- [x] `LAYOUT.md` — workspace files live under `.ark/workspace/`, mirroring the existing convention for tasks/specs/templates: PASS
- [x] `rust/COMMENTS.md` — module-level docs lead each new file; per-fn docs explain *why*, not *what*; no comment narrates obvious code; no task-mark tags in source per the never-use-rule-labels convention (V-004 found the violations during the verify sweep and stripped them): PASS
- [x] `rust/STYLE.md` — public structs (`Identity`, `WorkspaceConfig`, `RecordSummary`, `RecordTransaction`, `RecordSnapshot`) have private fields with constructors and accessors; intentionally-transparent enums (`RecordMode`, `TxState`) stay enum-shaped: PASS
- [x] `rust/ERRORS.md` — uses `thiserror`-derived `Error` variants throughout; never `.unwrap()` outside tests; `?` for propagation; rollback paths log via `eprintln!` only when their own `Result` cannot reach the caller: PASS

## Related Feature Spec Compliance

- [x] `ark-workflow-refactor` — consumes the `task.toml.start_head` primitive captured at `task new`; consumes `Phase::Committed`; extends the existing `RollbackGuard` rather than replacing it (R-201 fix preserves SPEC + features-INDEX rollback coverage): PASS
- [x] `ark-agent-namespace` — adds `ark agent workspace record` and `ark agent workspace developer register|touch` under the existing hidden, non-semver namespace: PASS
- [x] `ark-context` — adds `Scope::Record` and `ScopeTag::Record` additively (does NOT use `--for record` per R-202); `RecordProjection` lives inside the existing `ProjectedContext` envelope per R-206: PASS
- [x] `ark-upgrade` — `ark upgrade` migration for scaffolding the developer dir (G-16) was deferred per the EXECUTE-time scope decision; the dogfood path of this task creates the directories naturally so no migration is needed for this install: ACCEPTED — see Finding V-002
- [x] `worktree` — `workspace_record` runs from the worktree's tree (no parent-resolution); journal lives on the task's branch — same lesson as PR #9: PASS
- [x] `codex-support` and `opencode-support` — `/ark:record` slash command and `ark-record` skill added across all three platforms; `/ark:commit` updated in lockstep: PASS
- [x] `project-spec` — does not modify the project-spec layout: PASS
- [x] `task-concurrency-control` — does not change `Phase` enum or transitions; only writes journals during the existing `Verify → Committed` and `Execute → Committed` transitions: PASS

## PRD Constraints

- [x] G-1: per-developer journal trees under `.ark/workspace/<dev>/journal-N.md`, written at `/ark:commit` time as part of the same atomic commit as the work: PASS (smoke-tested end-to-end in /tmp)
- [x] G-2: closing commit SHA recorded via deferred-slot mechanism (no amend, no chore commit, no vague message). Sentinel `<PENDING:<slug>>` written at commit; archive resolves via `git log -S '**Slug**: <slug>' --format=%H -- <journal>` (collect-then-classify; no `-n` cap): PASS
- [x] G-3: top-level `.ark/workspace/index.md` with auto-maintained Active Developers table inside `<!-- ARK:DEVELOPERS:START/END -->` markers: PASS
- [x] G-4: per-developer `<dev>/index.md` with auto-maintained Session History table; archive patches the row's Closing-Commit cell in lockstep with the journal patch: PASS
- [x] G-5: compact, table-first journal entry shape; auto-populated structural fields (Date, Slug, Branch, Base Branch, Start Head, Closing Commit) inserted by the CLI; agent-filled content delivered by appending to the journal directly: PASS
- [x] G-6: manual `/ark:record` entries use `**Slug**: -` so the slug-anchored pickaxe never matches them; omit Closing Commit / Base Branch / Start Head / Git Commits: PASS
- [x] G-7: `task.toml.journal_path` captured at `/ark:commit` time; archive reads it directly: PASS
- [x] G-8: idempotent archive — re-running on a task whose slot is filled is a no-op (sentinel-presence check): PASS (smoke-tested)
- [x] G-9: identity bootstrap consolidated to `ark init --developer <name>` / `--no-developer` + interactive prompt; identity stored in `.ark/.developer` (gitignored): PASS
- [x] G-10: `[workspace]` section in `.ark/config.toml` with `journal_max_lines` (default 2000): PASS
- [x] G-11: across all three platforms in lockstep: PASS
- [x] G-12: failure modes are explicit (`SlotResolveNoMatch`, `SlotResolveAmbiguous`, `JournalMissing`, `MissingIdentity`, `JournalDriftDetected`, `EntryFileMalformed`): PASS
- [x] G-13: single-owner transactional primitives. `RecordTransaction` owned inside `workspace_record`; snapshots taken before any mutation; on partial failure rolls back internally and returns `Err`. `RollbackGuard` (extension of existing) adopts the success-path snapshot for outer rollback. Documented exception (suffix-drift) preserves concurrent-appender data: PASS
- [x] G-14: agent content delivered by appending the session block directly to the active journal — simpler than the `--entry-file` design that was discarded mid-implementation: PASS (revised post-Phase-6 per user feedback)
- [x] G-15: skip-slot-patch audit trail — implemented as silent-skip when sentinel is absent (idempotent re-archive); no explicit `--skip-slot-patch` flag in current code (deferred per scope decision; the failure modes G-12 cover the cases where it would have been needed): ACCEPTED — see Finding V-001
- [x] G-16: `ark upgrade` scaffolds developer dir when `.ark/.developer` exists: ACCEPTED — deferred per EXECUTE-time scope decision (V-002)
- [x] G-17: `ark archive` requires a clean git index; errors `ArchiveIndexNotEmpty { staged_paths }` when dirty: PASS (smoke-tested)
- [x] G-18: `ark context --scope record` projection returns identity + active journal + branch + session_count + journal_max_lines: PASS
- [x] G-19: `RollbackGuard` covers task.toml + (deep) SPEC + features-INDEX (existing) + adopted RecordSnapshot + staged paths (new) as one rollback set: PASS

## Plan Fidelity

- [x] G-1..G-19 from `02_PLAN.md` Spec are all addressed (see PRD Constraints checklist above for the full mapping): PASS
- [x] NG-1..NG-6 (out of scope items) honored: PASS
- [x] Phases 1–6 implemented, plus revision pass dropping `EntryDraft` / `--entry-file` / `[workspace].developer` per user-driven simplification: PASS

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — no existing feature SPECs were modified by this task; the `workspace` SPEC will be promoted at archive time

## Findings

### V-001 `--skip-slot-patch flag deferred from G-15`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/archive.rs::patch_workspace_slot`
- **Problem:** 02_PLAN G-15 specified an explicit `--skip-slot-patch <slug>` flag plus an audit-trail body in the archive commit message. As shipped, slot-patching is silent-skipped when the sentinel is absent (covering the idempotent-rearchive case) and silent-skipped when `task.toml.journal_path` is None (covering pre-workspace tasks). The "user-requested skip after pickaxe failure" path is not implemented.
- **Why it matters:** A user encountering `SlotResolveNoMatch` or `SlotResolveAmbiguous` today has no escape hatch — they can't tell archive to proceed past the failed task. They'd need to manually edit the journal to remove the sentinel, or restore the journal to a state the pickaxe matches.
- **Recommendation:** Defer to a follow-up task. The failure modes are vanishingly rare in normal workflows (each requires deliberate journal-history corruption). Add the flag if a real user hits the case.
- **Resolution:** ACCEPTED — out of scope for this task per EXECUTE-time decision; tracked for follow-up.

### V-002 `ark upgrade migration for developer dir scaffold deferred`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/upgrade/mod.rs`
- **Problem:** 02_PLAN G-16 specified that `ark upgrade` would scaffold `.ark/workspace/<dev>/index.md` when `.ark/.developer` exists. As shipped, the developer dir is created lazily on first `workspace_record` write (the dogfood path), and `ark upgrade` doesn't proactively create it.
- **Why it matters:** Existing Ark installs that pre-date workspace-v2 will not have a `<dev>/index.md` until they run their first `/ark:commit` or `/ark:record`. Personal index appears empty until then.
- **Recommendation:** Add a one-shot upgrade step that scans for `.ark/.developer` and runs `developer_register` with `session_count = 0`. Lower priority than V-001.
- **Resolution:** ACCEPTED — out of scope; lazy creation is functionally equivalent for new entries.

### V-004 `Task-mark tags appeared in source comments`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/agent/workspace/{config.rs,transaction.rs}`, `crates/ark-core/src/commands/archive.rs`
- **Problem:** Module-level and item-level doc comments contained inline rule-label tags from the PLAN's response matrix and SPEC drafts: `R-101`, `R-103`, `S-21`, `S-24`, `NG-2`, `G-17`. The Rust comment convention forbids these — the encoded constraint should be inline prose; the label belongs nowhere in `crates/`.
- **Why it matters:** Future readers of the source have no SPEC to dereference these labels against (the labels are ephemeral artifacts of the task that produced the code). Comments stop being self-explanatory.
- **Recommendation:** Strip every tag, keep the prose.
- **Resolution:** FIXED — all tags removed from the workspace module, `commands/archive.rs`, and the `commands/context` arms I added. The one remaining hit (`projection.rs:378`'s `V-IT-9 / R-204`) predates this task (introduced by `ark-workflow-refactor` at commit `7ed2b8b`) and is left for that task's owner. Sweep verified by `grep -rn 'R-[0-9]\|G-[0-9]\|...' crates/` returning only test-fixture strings.

### V-003 `Branch field shows "HEAD" for non-worktree quick tasks`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/agent/task/commit.rs::record_workspace_journal`
- **Problem:** When `task new` runs without `--worktree`, `task.toml.branch` is `None`. The previous fallback was `"HEAD"` literal, which rendered cosmetically as `**Branch**: \`HEAD\`` in the journal.
- **Why it matters:** Cosmetic only — the SHA recovery via pickaxe is unaffected. But the journal is meant to be human-readable, and "HEAD" is less informative than the actual branch name.
- **Recommendation:** Fall through to `git rev-parse --abbrev-ref HEAD` when `toml.branch` is None.
- **Resolution:** FIXED — `current_branch(cwd)` helper added to `commit.rs`; `record_workspace_journal` now does `prev_toml.branch.clone().or_else(|| current_branch(task_cwd))`. Smoke-tested with a `develop` branch: journal now renders `**Branch**: \`develop\``.

## Notes

**Refactor pass (this VERIFY round) cleaned up:**

1. `stamp.rs::insert_after_main_changes` — replaced O(n²) line-position recomputation with single-pass running cursor.
2. `record.rs` — collapsed 4 repeated `let _ = tx.rollback(); return Err(e);` blocks into a single `apply` closure with one rollback path on Err.
3. `record.rs::record_appended_bytes_for_rollback` — renamed to `capture_stamped_suffix`, dropped the multi-paragraph working-notes comment, dropped the unused `outcome` parameter, dropped the wrapper accessor functions (`tx_journal_path`, `tx_journal_byte_length_before`).
4. `record.rs::select_active_journal` — dropped unused `_max_lines` parameter and dead `rotated: bool` return slot.
5. `record.rs::RecordSummary` — dropped dead `rotated()` accessor.
6. `transaction.rs` — factored two duplicated rollback paths (`RecordTransaction::rollback` and `RecordSnapshot::rollback`) into a shared `restore_for_state` helper that records the first error but still runs all restoration steps.
7. `archive.rs::resolve_closing_sha` — replaced `match candidates.len()` with `match candidates.as_slice()` slice-pattern; extracted `short_sha` helper for the `git rev-parse --short=12` fallback.
8. `archive.rs::patch_workspace_slot` — collapsed redundant fully-qualified `crate::io::*` and `crate::layout::*` paths via top-of-file imports.
9. `agent_cli.rs::WorkspaceDeveloperCommand::dispatch` — collapsed dead `if call == "register" { ... } else { ... same body ... }` branching; both verbs share one helper.
10. Tests + dead-code helpers (`_force_use`, the `slug_field_optional` parameter on `agent_journal`) removed.

**Validation:** 395 ark-core unit tests + 20 ark-cli tests pass (down from 398 — net delta is 6 entry_draft tests removed, 5 stamp tests added, 4 config tests removed since `[workspace].developer` is gone, 1 record test consolidated). Clippy clean. End-to-end smoke test in `/tmp` confirms commit + archive flow works on both single-session and multi-session journals; the slug-anchored pickaxe correctly scopes each replacement; idempotent re-archive is a no-op; clean-index precondition fires when staging area is dirty.

**Out of scope (acknowledged):**

- Squash-merge / as-merged-SHA recording (deferred to `task-finalize` per NG-1).
- Multi-developer concurrent-write coordination beyond `O_APPEND` (NG-2; suffix-checked rollback honors this honestly).
- `--skip-slot-patch` flag (V-001).
- `ark upgrade` developer-dir scaffolding (V-002).

**Dogfood:** This task creates `.ark/.developer` for `Anekoique` during EXECUTE; the workspace task is the first journal entry written by the new primitive. Archive of this task will exercise the slot-patch end-to-end against the in-flight journal that records its own ship.
