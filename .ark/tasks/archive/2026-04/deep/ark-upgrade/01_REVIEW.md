# `ark-upgrade` REVIEW `01`

> Status: Open
> Feature: `ark-upgrade`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: `1`
- Non-Blocking Issues: `6`



## Summary

Iteration 01 resolves every finding from `00_REVIEW`. The walk-through of R-001..R-012 and TR-1..TR-7 holds up against the code:

- R-001 (protected-path blanket exclusion) is gone; C-3/G-9 are now cleanly defined as "operate on the union, exempt `.installed.json` only". This matches how `manifest.files` is actually populated today (`init.rs:138-141` strips project root → project-relative PathBufs, so equality against `MANIFEST_RELATIVE_PATH` is a sound single-file check on normalized input).
- R-002 (PRD/PLAN disagreement) is closed: PRD.md now states "no sidecar files" and describes hashes living inside the manifest; no stale references remain.
- R-005 (hash-refresh loss) is fixed by the `Unchanged { refresh_hash: bool }` split, and the state transition table (01_PLAN:367-381) is exhaustive across `(desired ∈ {absent,present}) × (bytes match) × (recorded ∈ {sha,sha',None})`.
- R-007 rejection implicit — the new plan *accepts* the reviewer's recommendation and drops `load.rs` changes. Cross-check against `unload.rs:61-69` confirms `.ark/.installed.json` is captured by `walk_files(layout.ark_dir())`, so the manifest (including `hashes`) round-trips byte-for-byte through unload/load via `load.rs:85-89`'s `resolve_safe` write.
- R-009 rejection ("moot under option 3") is defensible: after dropping the prefix filter, the remaining `is_exempted` is a single-path equality, not a prefix match, so separator/case normalization is irrelevant to the exemption itself.

The Main-Flow reorder (manifest write at step 10, deletions at step 11) is consistent with Failure Flow step 5's recovery story: on mid-delete crash, the manifest already carries the new version + hashes, and residual-but-not-yet-deleted files re-classify as `SafeRemove`/`Orphaned` next run. No contradiction found.

One genuine hole remains (R-013): `manifest.files` entries are never re-validated for path safety at upgrade entry. `load.rs:87` protects itself with `layout.resolve_safe(&f.path)`; upgrade does not, so a tampered or legacy manifest entry of `/etc/passwd` or `../../something` will be opened, hashed, and potentially deleted. The rest of the findings are polish: non-deterministic iteration order for planned actions, unspecified `collect_desired_templates` dest-root mapping, the two-step manifest write in step 10/12 lacking a failure-mode note, and a couple of acceptance-mapping clarifications.



## Findings

### R-013 `Manifest file paths are not path-safety-validated at upgrade entry`

- Severity: HIGH
- Section: `Runtime / Main Flow step 5; Spec / Constraints C-3, C-12, C-13`
- Problem:
  The 01_PLAN rejects R-009 as moot because `is_exempted` is now a single equality check. That's true for the exemption, but it silently drops the entire path-safety story. `load.rs:87` guards itself: every path from an untrusted source goes through `layout.resolve_safe(...)`, which rejects absolute paths, `..` components, and drive prefixes (see `layout.rs:96-106, 164-180`, `Error::UnsafeSnapshotPath`). Upgrade reads the same kind of untrusted-ish input — `manifest.files` — but the plan classifies, hashes, writes, and deletes against those paths with no equivalent guard. A pre-existing manifest that was hand-edited, corrupted, or produced by a buggy earlier Ark could contain entries like `../../etc/passwd` or `/tmp/leak`. C-10 ("delete iff recorded hash matches current sha256") doesn't help — if an attacker can put the path in and *also* match its current content hash (or if the file doesn't exist yet, `SafeRemove` is a no-op but the file can still be written on a subsequent Add-if-hash-matches path — though Add only runs for desired entries, so the write-path exposure is mostly confined to `.new` files and hash reads).
  The bigger issue is the principle: load runs `resolve_safe` on every path from the snapshot; upgrade must do the same on every path from the manifest, consistently. Otherwise the trust boundary is inconsistent across commands.
- Why it matters:
  Breaks the same invariant `load.rs` was hardened against in the agent-namespace feature. Read-level exposure: `PathExt::hash_sha256` on `/etc/passwd` is benign today but leaks info into error paths. Delete-level exposure: an attacker who can write to `.installed.json` can arrange for a file outside the project to be unlinked. Even without malice, legacy junk in a hand-edited manifest can nuke user files during a "cleanup" SafeRemove.
- Recommendation:
  Add to Constraints: "C-17: Every path read from `manifest.files` is normalized through `layout.resolve_safe` before any read/write/delete. Entries that fail validation are surfaced as a named error (reuse `Error::UnsafeSnapshotPath` or add `Error::UnsafeManifestPath`) rather than silently acted on." Apply the same normalization to `desired_templates` output for symmetry (the walk always produces safe relative paths today, but hardening is cheap). Add V-F-9: `manifest_entry_outside_project_root_is_rejected` — inject `../escape.md` into `.installed.json`, run upgrade, assert `UnsafeManifestPath` and no filesystem activity outside the project.



### R-014 `collect_desired_templates dest-root mapping is left implicit`

- Severity: MEDIUM
- Section: `Runtime / Main Flow step 4; Architecture / Call graph`
- Problem:
  The plan says "enumerate desired templates by walking `ARK_TEMPLATES` and `CLAUDE_TEMPLATES`, yielding `Vec<(PathBuf, &'static [u8])>`" and otherwise asserts that upgrade's desired set must match init's. But `init.rs:127-145` derives the relative key as `dest_root.join(entry.relative_path).strip_prefix(project_root)` where `dest_root` is `layout.ark_dir()` for one tree and `layout.claude_dir()` for the other. If `upgrade`'s `collect_desired_templates` uses `entry.relative_path` directly (which is `specs/INDEX.md`, `workflow.md`, `commands/ark/quick.md`, etc. — the path *inside* the template tree, not the project-relative path), the keys will not match `manifest.files` entries like `.ark/specs/INDEX.md` or `.claude/commands/ark/quick.md`. Classification would then see every existing file as `Add` (on-disk absent because the key is wrong) and every manifest entry as not-in-desired (→ SafeRemove/Orphaned). Catastrophic silent failure.
  Nothing in the PLAN spells out the mapping. The call graph's `collect_desired_templates() → Vec<(PathBuf, &'static [u8])>` doesn't state the PathBuf is project-relative with the `.ark/`/`.claude/` prefix applied.
- Why it matters:
  The whole classification correctness hinges on this. A reviewer-visible, one-line constraint prevents a subtle regression.
- Recommendation:
  Add to Data Structure (or as a new Constraint): "`collect_desired_templates` produces project-relative PathBufs identical to the keys `init.rs::extract` stores in `manifest.files`: `.ark/<tree-relative>` for `ARK_TEMPLATES` and `.claude/<tree-relative>` for `CLAUDE_TEMPLATES`. Implementation: reuse the same `dest_root.join(entry.relative_path).strip_prefix(project_root)` shape." Add V-UT-17: `desired_template_keys_match_init_manifest_entries` — run `init` in a tempdir, compare `manifest.files` (sorted) to `collect_desired_templates().map(|(p,_)| p).sorted()`, assert equality.



### R-015 `PlannedAction for Unchanged{refresh_hash: false} is undefined`

- Severity: MEDIUM
- Section: `Data Structure / PlannedAction enum; Runtime / State Transitions`
- Problem:
  The `PlannedAction` enum (01_PLAN:234-243) lists `Write / RefreshHashOnly / CreateNew / Delete / DropManifestEntry / Preserve`. The state machine maps `Unchanged { refresh_hash: true }` → `RefreshHashOnly`. But `Unchanged { refresh_hash: false }` has no named action. Presumably it becomes "counter bump only, no `PlannedAction` emitted", but that's implicit. Also: `Preserve` is documented as "counter only" in the enum comment and arises from `UserModified → Skip`. The `modified_preserved` counter also includes the `ConflictPolicy::Skip` path. So there are two "counter-only" cases with different counters (`unchanged` vs `modified_preserved`) and only one action variant. When an implementer writes `apply_writes`, it's unclear whether `Preserve` bumps `modified_preserved` only, or whether a separate no-op variant handles `Unchanged{false}`.
- Why it matters:
  Small but concrete implementation-time ambiguity. Easy way for a developer to swap counters or miss one.
- Recommendation:
  Either (a) add an `Unchanged { refresh_hash: bool }` variant to `PlannedAction` that explicitly dispatches to the two counters, or (b) spell out in the plan: "`Unchanged{refresh_hash=false}` emits no PlannedAction; the classifier bumps `summary.unchanged` in place. `Preserve` emits a PlannedAction that bumps `summary.modified_preserved` during apply." Pick one and document it.



### R-016 `Action order within planned-action buckets is non-deterministic`

- Severity: MEDIUM
- Section: `Runtime / Main Flow step 8; Implementation / Phase 2.3`
- Problem:
  `plan_actions` iterates desired templates and manifest files. If either is walked through a non-ordered container (`include_dir::Dir::files` happens to be stable, `manifest.files` is a `Vec` so fine, `manifest.hashes` is a `BTreeMap` so also fine), the concatenation across the two loops has no specified total order. Recovery from partial-write failures depends on which actions ran before the failure. Two successive upgrades with the same inputs but different action orders can produce different intermediate states, making `V-F-8`'s recovery assertion flaky.
  Separately, test determinism for `Display` output (G-11, V-UT-13) is fine because it reports totals, not per-action details — but any future per-file log would need this ordering.
- Why it matters:
  Makes partial-failure behavior non-reproducible across CI runs. Minor but cheap to fix.
- Recommendation:
  Add a sentence to Phase 2.3: "`plan_actions` sorts its output by (action_bucket, relative_path) before returning so apply order is deterministic." Acceptance: extend V-F-8 to verify that re-running twice from the same partial-state produces identical manifest + disk state.



### R-017 `Two-step manifest write (steps 10 and 12) lacks a failure-mode entry`

- Severity: LOW
- Section: `Runtime / Main Flow step 12; Failure Flow`
- Problem:
  Step 10 writes the manifest (with new version + fresh hashes); step 12 writes it again "if any deletions ran" (to reflect `drop_file` calls from SafeRemove/Orphaned). Failure Flow documents steps 4 (mid-write), 5 (mid-delete), and 6 (managed block), but not step 12. If step 12 fails (disk full after deletions already succeeded), the on-disk manifest reflects the pre-deletion state — entries for files that no longer exist. Next upgrade classifies them as "not-desired, absent → drop manifest entry", which is clean, but undocumented.
  Also worth noting: the step-12 write is redundant on the happy path (it re-serializes the same manifest minus the dropped entries). A single write at step 13 after all mutations (including deletion bookkeeping) would be simpler and eliminates the partial-update failure class.
- Why it matters:
  Completeness of the failure matrix; one fewer write reduces the failure surface.
- Recommendation:
  Either (a) collapse to a single manifest write after deletions complete — with the concession that a mid-delete crash loses the fresh hashes — or (b) keep the two-write design and add Failure Flow entry: "step 12 write failure: the on-disk manifest still references now-deleted files; next upgrade re-classifies them as 'not-desired, absent' and drops the entries cleanly". The R-004 fix is why (a) was rejected; keep (b) and document it.



### R-018 `V-UT-14's coverage of --allow-downgrade co-occurrence is not called out`

- Severity: LOW
- Section: `Validation / V-UT-14; API Surface / UpgradeArgs`
- Problem:
  The `ArgGroup { id = "policy", multiple = false }` includes `--force`, `--skip-modified`, `--create-new`. `--allow-downgrade` has no `group` attribute (correct — it's orthogonal). V-UT-14 only asserts "two policy flags together → rejection"; no case asserts `--force --allow-downgrade` is legal. Trivially true today, but a future refactor could mis-group `--allow-downgrade` and silently break orthogonality — exactly the regression the prompt flagged as a risk.
- Why it matters:
  Prevents an easy future mistake; the test is one line.
- Recommendation:
  Extend V-UT-14: "`--force --allow-downgrade` parses without error; `--force --skip-modified` rejected by clap with 'cannot be used with'." Same for `--allow-downgrade` paired with each policy flag.



### R-019 `G-10 Acceptance Mapping no longer cites V-F-6`

- Severity: LOW
- Section: `Validation / Acceptance Mapping G-10, C-8`
- Problem:
  00_PLAN's mapping cited "V-F-6 (corrupt case), manual observation in V-IT-1". 01_PLAN replaces that with "V-IT-12, V-IT-13" — good, addresses R-008. But V-F-6 (`managed_block_corrupt_surfaced`) is still listed in the validation section and is genuinely a G-10-adjacent behavior (G-10's "re-applied via `update_managed_block`" implies the error surface is preserved). Leaving it unmapped is fine but worth a line to avoid the impression that V-F-6 is orphaned.
  C-8 mapping also changed from "V-F-6 + V-IT-1 (block re-applied)" to "V-IT-12, V-IT-13" — same shape, same question.
- Why it matters:
  Mapping hygiene; avoids the VERIFY phase asking "what validates V-F-6 if not G-10?"
- Recommendation:
  Add V-F-6 to the G-10 row (alongside V-IT-12/13) — the corrupt surface is part of the goal's contract. Keep V-F-6 in the validation section.



## Trade-off Advice

### TR-8 `Single manifest write at upgrade end (vs two writes)`

- Related Plan Item: (new; spans Main Flow steps 10 and 12 per R-017)
- Topic: Simplicity vs Partial-Failure Recovery
- Reviewer Position: Prefer the current two-write design (keep as stated), but document it
- Advice:
  The two-write approach (step 10 before deletions, step 12 after) is correct because R-004's fix explicitly wants fresh hashes to be durable *before* a deletion can fail. Collapsing to one write at the end would regress R-004. Keep the two writes; simply name the step-12 failure mode in Failure Flow (see R-017).
- Rationale:
  The extra write is cheap (single small JSON file). The resilience gain — not losing freshly-computed hashes if a deletion fails — is the whole point of R-004. Reverting now would reintroduce the "phantom modifications on next run" issue R-004 was opened to prevent.
- Required Action:
  Keep the two-write design. Add the step-12 failure note to Failure Flow as described in R-017.



### TR-9 `Hash manifest paths using resolve_safe vs trusting them`

- Related Plan Item: (new; relates to R-013)
- Topic: Safety vs Simplicity
- Reviewer Position: Prefer `resolve_safe` (consistent with `load.rs`)
- Advice:
  Mirror `load.rs:85-93`'s pattern — treat `manifest.files` paths as untrusted input and run them through `layout.resolve_safe` (or a `manifest.files`-specific equivalent). Upgrade currently treats them as trusted; that's inconsistent with the rest of the codebase's defensive posture and exposes a real hole for hand-edited or legacy manifests.
- Rationale:
  Cheap to add, fixes a real issue, keeps the trust model uniform across commands. The failure surface is tiny (one extra `resolve_safe` call per entry at plan-action time).
- Required Action:
  Adopt. Add C-17 and V-F-9 per R-013. Reuse `Error::UnsafeSnapshotPath` or introduce `Error::UnsafeManifestPath` — reviewer has no strong preference.
