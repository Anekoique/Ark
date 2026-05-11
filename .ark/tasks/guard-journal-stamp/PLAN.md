# `guard-journal-stamp` PLAN

> Status: Approved for Implementation
> Feature: `guard-journal-stamp`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none (standard tier)

---

## Summary

Add a hard-error guard inside `stamp_task` and `stamp_manual` (`crates/ark-core/src/commands/agent/workspace/stamp.rs`) that fires when the last `## Session N: ...` heading in the active journal is already followed by stamped auto-fields (i.e. a previous `stamp_*` call already ran against this heading). The new error variant `Error::JournalSessionHeadingMissing { journal_path, slug }` carries the path and slug, and renders a message telling the agent to append `## Session N: <title>` plus `### Summary` and `### Main Changes` before retrying. Three new tests guard the behavior: one for `stamp_task` (the rfc001-arkos shape), one for `stamp_manual`, and one for `workspace_record` end-to-end via the existing `record.rs::tests` harness. Workspace SPEC gets a CHANGELOG entry recording that the contract is now CLI-enforced.

## Log

None (00_PLAN, no prior iteration).

---

## Spec

[**Goals**]

- G-1: `stamp_task` refuses with `Error::JournalSessionHeadingMissing` when last `## Session ...` heading already has stamped auto-fields.
- G-2: `stamp_manual` refuses identically under the same condition.
- G-3: New error variant carries `journal_path: PathBuf` and `slug: String`; message names the missing line and the retry command.
- G-4: Refusal happens before any file write; journal byte-content is unchanged on failure.
- G-5: Happy path (fresh unstamped heading) continues to produce byte-identical output to today.
- G-6: `specs/features/workspace/SPEC.md` gains a `[**CHANGELOG**]` entry naming this task and the new failure mode.

[**Non-goals**]

- Slash-command template changes (`templates/{claude,codex,opencode}/...` left untouched).
- Empty-Git-Commits rendering change (`render_git_commits_block`'s `(none)` row stays as-is).
- Historical-data fixes for journal-1.md / personal index (already landed in `cd50a33` on `docs/rfc001-arkos`).

[**Architecture**]

The guard is a private helper `assert_unstamped(text: &str, split: &SessionSplit) -> Result<()>` in `stamp.rs`, called by both `stamp_task` and `stamp_manual` after `locate_last_session_block` returns and before the byte concatenation. The helper inspects `text[split.heading_end..]` and refuses when the next non-blank line begins with `**Date**`.

Why "next non-blank line begins with `**Date**`" is the detection rule:
- The auto-fields renderer always emits `**Date**: <date>` as the first line after the heading (see `render_task_auto_fields` at `stamp.rs:172` and `render_manual_auto_fields` at `stamp.rs:184`). Both modes share this prefix.
- A fresh heading written by the slash command is followed by a blank line and then `### Summary` (see the slash command's example body in `templates/{claude/commands/ark/commit.md, codex/skills/ark-commit/SKILL.md, opencode/commands/ark/commit.md}`).
- The two are unambiguous: `**Date**` cannot legally appear in agent-authored content at that position, and `### Summary` is the documented contract.
- The check tolerates leading blank lines between heading and content (the existing renderer emits a `\n` between heading and `**Date**`; the agent's content includes blank-line spacing too).

Slug propagation: `stamp_task` already has `fields.slug` available (used by `render_task_auto_fields`). The helper signature accepts an optional `slug: Option<&str>` so `stamp_manual` (which has no slug) can pass `None`. Error rendering substitutes `"-"` for `None`, matching `stamp_manual`'s existing dash-slug pattern in indices.

[**Data Structure**]

New error variant in `crates/ark-core/src/error.rs`:

```rust
/// Workspace journal's last `## Session N:` heading already carries
/// stamped auto-fields — the agent did not append a fresh heading before
/// invoking `task commit`.
#[error(
    "journal `{}` last `## Session` heading is already stamped; append a fresh `## Session N: \
     <title>` block (with `### Summary` and `### Main Changes`) for slug `{slug}` before re-running \
     `ark agent task commit`",
    journal_path.display()
)]
JournalSessionHeadingMissing {
    /// Active journal file path.
    journal_path: PathBuf,
    /// Slug of the task being committed (`"-"` for manual entries).
    slug: String,
},
```

[**API Surface**]

No public-API changes. `stamp_task`, `stamp_manual`, `workspace_record`, `task_commit` keep their existing signatures. New error variant is additive; downstream pattern-matching is restricted to internal modules and tests.

[**Constraints**]

- C-1: Guard helper is `≤30 LOC`, including doc-comment.
- C-2: New error variant uses structured fields (no `&'static str` reason), per `rust/ERRORS.md` `E-N`.
- C-3: No `unwrap` / `expect` in the guard or in error rendering; propagate via `?`.
- C-4: Detection rule: next non-blank line after the located heading begins with `**Date**` ⇒ stamped (refuse); begins with `###` or other ⇒ unstamped (proceed); EOF ⇒ unstamped (proceed; existing `EntryFileMalformed` happy path would have already errored at body-length zero if relevant).

---

## Runtime

**Main flow (happy path):**
1. Slash command invokes `ark agent task commit -m "<msg>"`.
2. `task_commit` calls `record_workspace_journal` → `workspace_record`.
3. `workspace_record` calls `stamp_task`.
4. `stamp_task` locates last `## Session N:` heading via `locate_last_session_block`.
5. *(new)* `stamp_task` calls `assert_unstamped` against the located block.
6. Heading is unstamped → guard returns `Ok(())`.
7. `stamp_task` writes auto-fields + git-commits table as before.
8. `workspace_record` upserts personal + top-level indices.
9. `task_commit` proceeds to git stage + commit.

**Failure flow (contract violated):**
1. Slash command invokes `ark agent task commit -m "<msg>"` without first appending a fresh heading.
2. `task_commit` calls `record_workspace_journal` → `workspace_record`.
3. `workspace_record` opens `RecordTransaction::begin` (snapshots taken).
4. `workspace_record` calls `stamp_task`.
5. `stamp_task` locates the last (already-stamped) heading.
6. *(new)* `stamp_task` calls `assert_unstamped`, which returns `Err(Error::JournalSessionHeadingMissing { journal_path, slug })`.
7. `stamp_task` propagates `Err` without writing.
8. `workspace_record`'s `apply` closure returns `Err`; `tx.rollback()` runs (no-op since nothing was written, but cheap).
9. `task_commit` propagates the error; `git commit` does not run; staging is unchanged.
10. CLI's `main.rs` prints `error: journal '...' last '## Session' heading is already stamped; ...` and exits 1.
11. Agent reads the error, appends the missing block to the journal, re-runs `ark agent task commit`.

**State transitions:** none. The new failure mode does not advance `task.toml.phase`; `task commit` is gated on `phase == Verify` (or `Execute` for quick) entering, and on success transitions to `Committed`. The guard fires before the transition write, so a failed call leaves phase unchanged.

---

## Implementation

1. **Add the error variant.** In `crates/ark-core/src/error.rs`, add `JournalSessionHeadingMissing { journal_path: PathBuf, slug: String }` with the `#[error(...)]` message from the *Data Structure* section. Keep alphabetical-ish ordering near `EntryFileMalformed` since they're peers.
2. **Add the guard helper.** In `crates/ark-core/src/commands/agent/workspace/stamp.rs`, define:
   ```rust
   /// Refuses when the located heading is already followed by stamped
   /// auto-fields (i.e. a previous `stamp_*` call wrote them).
   ///
   /// Detection: next non-blank line after `split.heading_end` begins with
   /// `**Date**`.
   fn assert_unstamped(
       text: &str,
       split: &SessionSplit,
       journal_path: &Path,
       slug: Option<&str>,
   ) -> Result<()> {
       let body = &text[split.heading_end..];
       let next_nonblank = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
       if next_nonblank.trim_start().starts_with("**Date**") {
           return Err(Error::JournalSessionHeadingMissing {
               journal_path: journal_path.to_path_buf(),
               slug: slug.unwrap_or("-").to_string(),
           });
       }
       Ok(())
   }
   ```
3. **Wire the guard into `stamp_task`.** Insert between `locate_last_session_block(...)?` and the `auto_fields` construction (between current lines 78 and 82):
   ```rust
   assert_unstamped(&original, &split, journal_path, Some(fields.slug))?;
   ```
4. **Wire the guard into `stamp_manual`.** Identical insertion between current lines 107 and 111:
   ```rust
   assert_unstamped(&original, &split, journal_path, None)?;
   ```
5. **Add three tests.**
   - In `stamp.rs::tests`: `stamp_task_refuses_when_heading_already_stamped` — prepare a journal containing a stamped block, call `stamp_task`, assert `Err(Error::JournalSessionHeadingMissing { .. })` and that the file's bytes are unchanged.
   - In `stamp.rs::tests`: `stamp_manual_refuses_when_heading_already_stamped` — symmetric test for manual.
   - In `record.rs::tests`: `record_errors_when_last_heading_already_stamped` — full `workspace_record` flow against a journal whose last heading is already stamped; assert the variant and that personal/top-level index are unchanged.
6. **Add the CHANGELOG entry.** Append to `specs/features/workspace/SPEC.md` under its `[**CHANGELOG**]` block: `2026-05-11 from task guard-journal-stamp — CLI enforces the session-heading contract; failure surfaces as Error::JournalSessionHeadingMissing.`
7. **Build + lint + test.**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

---

## Trade-offs

| Option | Adv | Disadv |
|--------|-----|--------|
| **Guard in `stamp_*` (chosen)** | Co-located with the locator; both call sites share one helper; minimal blast radius. | Pollutes the stamping module with a refusal concern. |
| Guard in `workspace_record` (`record.rs`) | Earlier; pre-transaction. | Re-reads the journal; duplicates `locate_last_session_block` work; further from the failure site. |
| Detect via re-running `locate_last_session_block` after a `### Summary` probe | More forgiving (e.g. agent writes Summary but no heading). | Out of scope — slash command requires both heading and Summary; partial-write is a separate failure to handle elsewhere. |
| New variant vs reuse `EntryFileMalformed` | Distinct variant lets tests assert by `matches!` rather than substring; structured fields render path + slug cleanly. | One extra `Error` variant. |

**Detection rule false-positive risk.** The check rules a heading "stamped" iff next non-blank line begins with `**Date**`. Hand-edited journals where someone wrote `**Date**: ...` inside an unstamped block (e.g. an agent who wrote auto-fields manually then forgot to remove them and re-added a fresh heading) would false-positive. Mitigation: the slash command's body template emits `### Summary` and `### Main Changes` headings, never raw `**Date**`; the false-positive shape is "agent wrote auto-fields by hand instead of letting CLI generate them," which is a different failure not handled by this task. Acceptable.

**Detection rule false-negative risk.** A stamped block where someone deleted the `**Date**` line but kept other auto-fields (`**Slug**`, etc.) would false-negative. Mitigation: again, hand-editing autofield content is out-of-band; we trust that `stamp_*` is the only writer for that region.

---

## Validation

[**Unit tests** — added to `stamp.rs::tests`]

- `stamp_task_refuses_when_heading_already_stamped` — fixture journal with `## Session 1: x\n**Date**: 2026-01-01\n...`; `stamp_task` returns `Err(JournalSessionHeadingMissing { journal_path, slug: "workspace" })`; `fs::read_to_string(journal)` matches the pre-call bytes.
- `stamp_manual_refuses_when_heading_already_stamped` — symmetric; slug field equals `"-"`.
- `stamp_*_still_succeeds_on_fresh_heading` (regression) — the existing `stamp_task_inserts_all_auto_fields` and `stamp_manual_omits_task_only_fields` cover this; verify they still pass byte-for-byte.

[**Integration tests** — added to `record.rs::tests`]

- `record_errors_when_last_heading_already_stamped` — uses the existing `setup_with_identity_and_journal` helper but writes a pre-stamped journal; calls `workspace_record` with `RecordMode::Task { ... }`; asserts `Err(JournalSessionHeadingMissing)`; asserts the personal index file is absent or unchanged (no row added), the top-level index unchanged.

[**Failure tests**]

- Covered by the unit tests above (each refusal is a failure path).

[**Edge tests**]

- Empty body after heading (heading is the last line of the file): `assert_unstamped`'s `next_nonblank` is empty string → does not start with `**Date**` → returns `Ok(())`. The existing `EntryFileMalformed { reason: "journal does not end with..." }` is not triggered because the heading *is* present. The downstream stamper handles this as a happy path; the resulting journal has auto-fields directly under the heading. **Confirmed acceptable** — this matches the slash-command's "append a session block" instruction even if the agent skipped Summary and Main Changes; subsequent agents reading the journal see a degraded entry but not a corrupted one.

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1: `stamp_task` refuses on stamped heading | V-U-1: `stamp_task_refuses_when_heading_already_stamped`. |
| G-2: `stamp_manual` refuses on stamped heading | V-U-2: `stamp_manual_refuses_when_heading_already_stamped`. |
| G-3: New variant has `path` + `slug`; message names retry command | V-U-3: assert `Error::JournalSessionHeadingMissing { journal_path, slug }`; assert `format!("{err}")` contains "ark agent task commit" and the journal's path. |
| G-4: No write on failure | V-U-4: file's byte-content equals pre-call bytes (covered in V-U-1 and V-U-2). |
| G-5: Happy path unchanged | V-Regression: existing `stamp_task_inserts_all_auto_fields` + `stamp_manual_omits_task_only_fields` pass byte-identical output. |
| G-6: workspace SPEC CHANGELOG entry | V-VERIFY: VERIFY checklist's "Modified feature SPECs have CHANGELOG entries" passes by inspection at commit-time. |
