# `ark-upgrade` REVIEW `00`

> Status: Open
> Feature: `ark-upgrade`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `9`



## Summary

The PLAN is structurally sound: the core idea (extend `Manifest` with a `hashes` map, classify each desired template against on-disk + recorded-hash, resolve conflicts via an injectable `Prompter`) fits the existing architecture cleanly and aligns with the house style of `commands/init.rs` and `commands/agent/`. The trade-off section is thoughtful, test coverage is broad, and backwards-compat is mostly thought through via the `AmbiguousNoHash` path.

However, there are three hard problems that must be resolved before implementation:

1. The protected-path rule as specified (`.ark/specs/` blanket-excluded) conflicts with the fact that `init` actually ships three files into `.ark/specs/` — those templates will never be upgradable, and an `AmbiguousNoHash` refresh on a v0.1.1 install will falsely "preserve" them even though they are shipped-template files.
2. The PRD and PLAN disagree on the `.ark/.version` / `.ark/.hashes.json` sidecars — the PRD mentions both as tracked artifacts, the PLAN drops them via NG-6 and T-1. One must win; the commit history against the PRD must be explicit.
3. The `load` path claims "compute hashes at restore time" restores the same bytes as init, but the snapshot is lossy for the `managed_blocks` entry (body only, markers re-synthesized by `update_managed_block`). That's fine for files but the hash-on-restore step for files is correct only if we use the decoded bytes, not the on-disk file after re-render (which we're writing byte-for-byte — verified against `snapshot.rs`). Minor but worth calling out in the PLAN so the implementer doesn't hash the file post-write and race with a concurrent editor.

The remaining findings are API-level polish (clap `conflicts_with_all` self-reference, `Prompter: Send` not needed, path-separator normalization, deterministic action ordering) and test coverage gaps (no V-* for G-10 block re-application, no coverage for `update_managed_block`'s `record_block` idempotency, no coverage of the Windows CRLF case even as a skipped/ignore test). Fix the three blockers, address the HIGH findings, and the plan is ready to execute.



## Findings

### R-001 `Protected-path rule blanket-excludes shipped spec INDEX.md files`

- Severity: CRITICAL
- Section: `Spec / Goals G-9, Constraints C-3, C-11`
- Problem:
  C-3 declares `.ark/specs/` a protected path that `upgrade` "NEVER reads, writes, deletes, or hashes". But `init` ships three files under `.ark/specs/`: `.ark/specs/INDEX.md`, `.ark/specs/project/INDEX.md`, `.ark/specs/features/INDEX.md` (see `crates/ark-core/src/commands/init.rs:163-165` test, and `templates/ark/specs/**`). Under the proposed rule, these files:
  - get no hash recorded at init (filter applied to desired list and manifest),
  - are unreachable by upgrade forever (they cannot leave `AmbiguousNoHash` because they are filtered out before classification),
  - are invisible to `--force`, so a legitimate bug-fix to `INDEX.md` template content in a future release cannot be rolled out.

  The PRD actually names the correct granularity: "the upgrade's 'protected paths' rule is exactly these directories plus `.ark/specs/project/`" — i.e. `.ark/specs/features/` (for promoted SPECs) and `.ark/specs/project/` (for user conventions), plus `.ark/tasks/`. Top-level `.ark/specs/INDEX.md` is a shipped template and should be upgradable; so are the two `INDEX.md` files *nested inside* `specs/features/` and `specs/project/` IF they are shipped as templates (they are).

- Why it matters:
  The whole upgrade feature is undermined: three of the shipped templates silently become unmanaged. Worse, the C-3 filter is described as applying to both the desired list AND the manifest `files` list, so even the *existing* manifest entries for these three INDEX.md files from v0.1.1 installs get orphaned at upgrade time.

- Recommendation:
  Replace the blanket `.ark/specs/` exclusion with a file-granular rule. Options, in order of preference:
  1. Protect specific *subtrees where users author files*: `.ark/specs/features/<anything>/` (not the `INDEX.md` itself) and `.ark/specs/project/<anything>/` (not the `INDEX.md` itself). The two `INDEX.md` files and the top-level `specs/INDEX.md` are shipped templates; upgrade should manage them.
  2. Simpler: protect exactly the paths that `init` does NOT write — compute "protected = any path under `.ark/` that is not in the embedded template set". This naturally covers user-authored files without hardcoding prefixes.
  3. Even simpler: drop C-3's blanket prefix filter entirely. Upgrade only acts on (a) files in the desired templates list and (b) files in `manifest.files`. Neither `.ark/tasks/...` nor `.ark/specs/features/<slug>/...` nor `.current` is in either set today, because `init` never records them and `record_file` is only called by `init`. So the filter is already implicit in "only touch what the manifest or the template set says to touch". Explicitly rename C-3 to "Upgrade never acts on paths outside `manifest.files ∪ desired_templates`; and never on `.ark/.installed.json` itself".

  Option (3) is cleanest and matches the existing invariants. Keep `.ark/.installed.json` as a single-file exemption.

  Also: add a V-IT case that verifies `.ark/specs/INDEX.md` round-trips through an upgrade (modify a byte, upgrade, verify auto-update).

### R-002 `PRD/PLAN disagreement on .ark/.version and .ark/.hashes.json sidecars`

- Severity: HIGH
- Section: `PRD Outcome bullets vs. PLAN NG-6, T-1`
- Problem:
  The PRD explicitly promises:
  - "`ark init` on a fresh project writes the hash file alongside the manifest"
  - "`ark unload` and `ark remove` continue to work — `.ark/.hashes.json` and `.ark/.version` are tracked in the installed manifest and cleaned up like any other Ark-written file"
  - "A `.ark/.version` file records the CLI version..."

  The PLAN contradicts both:
  - NG-6: "No `.version` sidecar file. The manifest already carries `version`."
  - T-1 option A (chosen): "Extend the existing manifest" — no separate `.ark/.hashes.json`.

  The PLAN is arguably better (one file, one source of truth), but the PRD is the spec of record. This disagreement is silent in the plan — it reads as if the PLAN ignored the PRD.

- Why it matters:
  Reviewers downstream (verify phase, acceptance tests) will check the PRD literally. If the PRD says `.ark/.hashes.json` exists and the code doesn't produce one, verification fails. Also, if a future contributor reads the PRD first, they'll be surprised.

- Recommendation:
  Before execution: update the PRD to match the PLAN's chosen architecture (manifest-embedded hashes, no sidecar files, no `.version` file). Add a one-line "Changed from PRD" note in the PLAN's Log section pointing at the PRD diff. If the PRD cannot be edited, the PLAN must pivot to Option B of T-1 (separate `.hashes.json` and `.version`) — even though that's worse. Resolve explicitly, don't paper over.

### R-003 `Prompter trait object boundary and dyn compatibility`

- Severity: HIGH
- Section: `Data Structure / Prompter trait + API Surface`
- Problem:
  `pub trait Prompter: Send { fn prompt(&mut self, relative_path: &Path) -> Result<ConflictChoice>; }` is passed through the library as `&mut dyn Prompter`. Two issues:
  1. `Send` is unnecessary and misleading. Ark is single-threaded; the trait object is never sent across threads. Adding the bound forces implementers to make their prompter `Send`-able, which for stdin-backed prompters that hold a `StdinLock` is *not* `Send` on some platforms. Drop the bound.
  2. `&mut dyn Prompter` is fine, but note the subtle requirement that `Prompter` is dyn-compatible (no generic methods, no `Self: Sized`). The PLAN doesn't state this invariant, and a well-intentioned future change (e.g. adding `fn configure<T: Display>(&mut self, ...)`) would silently break the public API. Worth a one-line note.

- Why it matters:
  `Send` leaks an implementation detail and blocks obvious implementations. Dyn-compatibility is a silent API-stability constraint that the PLAN should name.

- Recommendation:
  Remove `: Send`. Add to Constraints: "C-X: `Prompter` is dyn-compatible; do not add generic methods or `Self: Sized` bounds."

### R-004 `Partial-write failure semantics under-specified for manifest update`

- Severity: HIGH
- Section: `Runtime / Failure Flow (4) + Implementation 2.2`
- Problem:
  The Failure Flow case (4) says "Files written up to that point are on disk, manifest is NOT yet rewritten". But 2.2 says "Produce a `Vec<PlannedAction>` first, then apply in a single pass" — which is a plan-then-apply pattern. The gap: if action #5 out of 10 fails, what is the state of `manifest.hashes`? Two options are implied but never chosen:
  - (a) `manifest` is an in-memory value that we only `.write()` at the very end (step 10). On mid-apply failure, manifest on disk reflects the pre-upgrade state. This leaves the on-disk `.ark/.installed.json`'s `hashes` out of sync with files we *did* rewrite in this run. Next `upgrade` sees them as `AmbiguousNoHash` (or `UserModified`), not `Unchanged`, and prompts spuriously.
  - (b) We `.write()` the manifest after each successful action. Expensive (small JSON, but many disk writes) and means partial failure leaves an inconsistent on-disk state that is at least self-consistent per-file.

  The PLAN implicitly picks (a) via step 10's "manifest.write" placement, but doesn't acknowledge the re-classification surprise on the next run.

- Why it matters:
  A crash during upgrade produces a subtle "phantom modifications" experience on the next run. User edits a file, upgrade half-completes, user re-runs upgrade, is prompted to overwrite a file that Ark itself just wrote. Confusing and erodes trust.

- Recommendation:
  Option (a) is correct; make it explicit and document the recovery story in the PLAN: on a subsequent `upgrade`, files whose on-disk content matches the new embedded template are classified `Unchanged` (because the "match-desired" branch of the state table triggers *regardless of recorded-hash mismatch*, per the state transitions section's parenthetical). Verify this with a new test: `V-F-8: partial_write_then_rerun_classifies_written_files_as_unchanged` — simulate a failure mid-apply, verify the next upgrade does NOT prompt for the successfully-written files.

  Separately, consider writing the manifest *before* deletions in the apply order: deletions are the least-recoverable action. If we've committed the new hashes to the manifest before a delete fails, at least the file → hash mapping is consistent.

### R-005 `classify: "match-desired, mismatch-recorded" edge case loses hash-refresh`

- Severity: HIGH
- Section: `Runtime / State Transitions + T-5`
- Problem:
  The state table says `(present, match-desired, mismatch-recorded)` → "Unchanged + refresh hash". T-5 confirms this is folded into `Unchanged`. But `Classification::Unchanged` in Data Structure has no "refresh hash" side effect — it maps to the `Unchanged` counter with `apply_action` being a noop. If the user edited and reverted, we must refresh the hash (from the stale one) so future upgrades correctly classify it. Otherwise next run it's again `AmbiguousNoHash`/`UserModified` after any template change.
- Why it matters:
  Silent staleness in the hash map. The very reason we track hashes is to make classification stable.
- Recommendation:
  Split `Classification::Unchanged` into two internal variants or add a "refresh_hash: bool" tag. Actions for `Unchanged { refresh: true }` do write the new hash to the manifest but do not write to disk. Cover with V-UT-15: after `(present, match-desired, mismatch-recorded)`, upgrade returns `Unchanged` count + manifest's hash now matches the current file bytes.

### R-006 `clap conflicts_with_all references the same flag that declares the attr`

- Severity: MEDIUM
- Section: `API Surface / UpgradeArgs`
- Problem:
  The PLAN writes:
  ```
  #[arg(long, conflicts_with_all = ["skip_modified", "create_new"])]
  force: bool,
  ```
  This is fine, but the other two bind symmetrically to `force`, which means the conflict graph is fully-connected. clap handles this, but the idiomatic (and slightly smaller) pattern is an `ArgGroup` with `multiple = false`:
  ```
  #[group(id = "policy", multiple = false)]
  ```
  plus `#[arg(long, group = "policy")]` on each. This also surfaces nicely in `--help` and avoids manual graph maintenance if a fourth policy is added later.
- Why it matters:
  Minor maintainability; the PLAN's form is not wrong. Flag but don't block.
- Recommendation:
  Switch to `ArgGroup`. Add V-UT-14 sub-case: two policy flags together → clap's "cannot be used with" error message.

### R-007 `Hash-on-restore in load.rs is correct only if the manifest is also updated`

- Severity: MEDIUM
- Section: `Implementation 1.5 — load/snapshot hash survival`
- Problem:
  The PLAN claims `load` should "compute hashes at restore time" so hashes survive round-trips. Reviewing `crates/ark-core/src/commands/load.rs:85-101`: the restore path writes bytes from the snapshot but does NOT touch any manifest (the manifest IS one of the files in the snapshot if it existed at unload time, since it's under `.ark/`). So in practice:
  - At unload time, the snapshot captures the manifest JSON verbatim (including the `hashes` field written at init/upgrade).
  - At load time, restoring rewrites the manifest file byte-for-byte. Hashes are automatically preserved because the manifest itself is a file in the snapshot.

  So 1.5 ("compute at restore") is actually *unnecessary*. The snapshot round-trip preserves hashes for free, as long as init/upgrade both kept `manifest.hashes` up to date before the unload. The only case where "compute at restore" would be needed is if someone loaded from a pre-hashes snapshot — but those are v0.1.1 snapshots with no `hashes` field at all, and deserialization with `#[serde(default)]` produces an empty map. In that case, the classification `AmbiguousNoHash` handles it correctly on next upgrade.

- Why it matters:
  Phase 1.5 does extra work that can introduce bugs (e.g. hashing bytes before they're written, hashing with a different encoding than the manifest stored). The simpler correct answer is "don't touch load at all".
- Recommendation:
  Drop 1.5's "compute hashes at restore time" and replace with: "Verify by test V-IT-11 that hashes survive unload/load round-trip via the manifest itself being snapshotted. No load.rs changes needed; just add the test." This also simplifies the Failure Flow (fewer moving parts).

### R-008 `No coverage for G-10 managed-block re-application in the happy path`

- Severity: MEDIUM
- Section: `Validation / Acceptance Mapping G-10`
- Problem:
  Acceptance Mapping says G-10 is covered by V-F-6 (corrupt case) and "manual observation in V-IT-1 (block re-applied)". Manual observation is not a test. V-IT-1 is `fresh_install_then_upgrade_is_noop` — if the block body hasn't changed between init and upgrade, `update_managed_block` would no-op and the test can't actually assert that the call happened.

  Two gaps:
  1. The body literal `MANAGED_BLOCK_BODY` may change between versions. If the project's CLAUDE.md has the old body and upgrade has the new body, we need a test proving the body changed.
  2. If `manifest.managed_blocks` is empty (pre-hashes install that somehow dropped the entry) but the block exists in CLAUDE.md, does upgrade still call `update_managed_block`? The spec says "always — marker-based, not hash". Needs an integration test.

- Why it matters:
  G-10 is a named Goal. Goals must map to real tests (the template rule in Validation).
- Recommendation:
  Add V-IT-12: `managed_block_body_refreshed` — initialize with a contrived old body (write manifest, manually tamper CLAUDE.md to have a different block body), run upgrade, assert CLAUDE.md now contains `MANAGED_BLOCK_BODY` verbatim.

  Add V-IT-13: `managed_block_reapplied_when_manifest_lacks_entry` — init, strip `managed_blocks` from the JSON, upgrade, assert block still present.

### R-009 `is_protected_path under-specified for separator tricks and case`

- Severity: MEDIUM
- Section: `Constraints C-3, C-13`
- Problem:
  Assuming R-001 is resolved by option (3) (drop the prefix filter), this finding is moot. But if option (1) or (2) is chosen, the PLAN doesn't specify how `is_protected_path` normalizes:
  - Trailing slash: `.ark/tasks` vs `.ark/tasks/`
  - Separator: on Windows, `Path::components` normalizes `/` and `\`, but string-prefix comparison does not.
  - Case: macOS default volumes are case-insensitive; `.ark/Tasks/foo` vs `.ark/tasks/foo`.
  - Unicode NFC/NFD (macOS decomposes): `.ark/tâsks` — unlikely but possible.

  Simple `PathBuf::starts_with` handles separator-per-component correctly. The PLAN should explicitly say "uses `Path::starts_with`, not string-prefix comparison".
- Why it matters:
  If the filter is a string-prefix check, a malicious or accidental `.ARK/tasks/foo` on case-insensitive FS bypasses protection. Not a security bug in Ark's trust model (the user owns the filesystem), but it surfaces as inconsistent behavior across platforms.
- Recommendation:
  Add C-15: "Protected-path matching uses `Path::starts_with` on component-normalized paths; no string-prefix comparison. Case-sensitivity inherits from the filesystem."

  Cover in V-UT-12b: `is_protected_path(".ark/tasks/foo")` and `is_protected_path(".ark/tasks/")` both true; `.ark/tasks` (no trailing) true; `.ARK/tasks/foo` platform-dependent (document).

### R-010 `AmbiguousNoHash + --force: backfill behavior unstated`

- Severity: MEDIUM
- Section: `Runtime / Main Flow step 7 + Spec C-11`
- Problem:
  V-IT-13 in Phase 3 (`hash_backfill_after_same_content`) asserts "init, delete hashes, run `upgrade --force` → every file hashed fresh in the new manifest". But the main flow says `--force` only applies to `UserModified + AmbiguousNoHash`. If the user deletes `hashes` but the on-disk content still matches the embedded template, classification is `Unchanged` (not `AmbiguousNoHash`). What triggers the hash-write in that case?

  Re-reading: `Unchanged` is the "(present, match-desired, anything)" branch and the parenthetical says "rare, treat as Unchanged + refresh hash". So `--force` is irrelevant; the hash gets refreshed on `Unchanged` too. But the PLAN's apply table says `Unchanged → noop`. Inconsistent.

- Why it matters:
  See R-005. Same root cause: `Unchanged` is overloaded into "noop" and "noop + refresh-hash" without a discriminator. Without a fix, V-IT-13 fails (hashes won't be populated) on same-content unchanged files.
- Recommendation:
  Fix R-005 first. Then V-IT-13 passes because `Unchanged` with a stale/missing hash does write the hash.

### R-011 `Concurrent upgrade / editor race not addressed`

- Severity: LOW
- Section: `Runtime / Main Flow`
- Problem:
  Between `classify` (reads file bytes to hash) and `apply` (writes or skips), a concurrent editor could modify the file. No TOCTOU lock. Classify sees "unchanged", apply overwrites, user loses edits.

  This is genuinely minor for a CLI tool where the user runs upgrade deliberately. But it's worth a one-line mention.
- Why it matters:
  Low-probability, low-impact. Mentioned for completeness.
- Recommendation:
  Add a one-line note in Constraints: "C-16: Upgrade is not safe against concurrent file modification. Users should close editors before running upgrade. No locking is attempted."

### R-012 `Missing init test for hash recording`

- Severity: LOW
- Section: `Validation / Unit Tests + Implementation 1.4`
- Problem:
  Phase 1.4 changes `init`'s `extract` to record hashes. No unit/integration test is listed to verify this. V-UT-1 covers `Manifest::record_file_with_hash` in isolation; V-IT-1 covers init-then-upgrade end-to-end but does not explicitly assert `manifest.hashes` is populated after the init step.
- Why it matters:
  A regression in `init` that silently stops recording hashes would only surface as downstream upgrade prompts. Direct coverage is cheap.
- Recommendation:
  Add V-UT-16: `init_populates_manifest_hashes` — after `init`, the manifest's `hashes` map has one entry per file in `manifest.files`, and each hash matches `hash_bytes(file_contents)`.



## Trade-off Advice

### TR-1 `Where to store hashes — manifest vs. sidecar`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (extend manifest), with conditions
- Advice:
  Accept Option A (manifest extension). The reasoning is sound: one file, `#[serde(default)]` makes old manifests deserialize cleanly, existing unload/remove infrastructure handles it for free. The "mixes two concerns" objection is weak; the manifest is already a mixed-concern file (files + managed blocks + install timestamp).
- Rationale:
  Serde's default-field pattern is exactly designed for this kind of additive migration. A separate `.hashes.json` doubles the surface area of failure modes (what if files list says X but hashes say Y?). The PLAN's assessment of ~30 entries is correct — the manifest will not grow problematically.
- Required Action:
  Adopt. But resolve R-002 first: the PRD currently promises a sidecar, so either amend the PRD or document the deviation clearly in the PLAN's Log section.

### TR-2 `TTY detection — atty vs is-terminal vs std::io::IsTerminal`

- Related Plan Item: `T-2`
- Topic: Simplicity
- Reviewer Position: Prefer Option A (stdlib)
- Advice:
  Use `std::io::IsTerminal` (stable since Rust 1.70). The crate is edition 2024 (MSRV 1.85+ per the PLAN claim), well above 1.70.
- Rationale:
  No dependency, no bit-rot exposure (atty is unmaintained), no audit burden. The API is identical in intent.
- Required Action:
  Adopt as stated. Remove `is-terminal = "0.4"` from the Phase 1.1 dependency list; the PLAN already flags this in T-2 but the dependency add list at 1.1 contradicts itself. Fix the inconsistency.

### TR-3 `Prompt UX — per-file vs batch "apply to all"`

- Related Plan Item: `T-3`
- Topic: Simplicity vs UX
- Reviewer Position: Prefer Option A (per-file)
- Advice:
  Per-file. Ark's template set is tiny (~12 files per the `templates/` directory listing). Batch prompts add state (remember-choice) and complicate the `Prompter` trait. `--force` / `--skip-modified` / `--create-new` already cover the "I have many conflicts and want to decide once" case.
- Rationale:
  Simplicity wins when the scale is small. Batch prompts matter at Trellis's scale (hundreds of files); Ark will rarely see more than 2-3 conflicts per upgrade.
- Required Action:
  Adopt as stated. No change needed.

### TR-4 `Exempt .installed.json or include it in the managed set`

- Related Plan Item: `T-4`
- Topic: Design Cleanliness
- Reviewer Position: Prefer Option A (exempt)
- Advice:
  Exempt. It's Ark's own state; treating it as a template creates a chicken-and-egg problem (how do you hash the file that stores hashes?).
- Rationale:
  Same argument the PLAN gives, plus: `.installed.json` changes every run (`installed_at` timestamp, version bump). Hashing it would flag spurious modifications.
- Required Action:
  Adopt as stated.

### TR-5 `Merge "user-reverts-to-template" into Unchanged`

- Related Plan Item: `T-5`
- Topic: Simplicity vs State Machine Completeness
- Reviewer Position: Need More Justification
- Advice:
  The intent is right but the implementation is under-specified (see R-005). Folding the case into `Unchanged` is good for user-facing counters, but internally we still need to distinguish "unchanged, hash fresh" from "unchanged, hash stale, must refresh". Split `Classification::Unchanged { refresh_hash: bool }` (or add a companion action `RefreshHashOnly`).
- Rationale:
  If we quietly drop the hash refresh, every subsequent upgrade re-triggers the same `AmbiguousNoHash` classification, defeating the purpose of the hash map.
- Required Action:
  Adopt the user-facing counter merge; add the internal discriminator. Pair with a V-UT case (see R-005) that asserts the manifest hash matches current file bytes after an `Unchanged` classification with a stale recorded hash.

### TR-6 `Same-version upgrade — no-op or re-apply`

- Related Plan Item: `T-6`
- Topic: Simplicity vs Repair Capability
- Reviewer Position: Prefer re-apply (as stated)
- Advice:
  Re-apply is cheap (hash match → noop per-file anyway) and gives users a "repair" tool without inventing a new subcommand. Keep as stated.
- Rationale:
  Zero extra complexity; strictly dominant over the "skip same-version" alternative.
- Required Action:
  Adopt as stated. Add V-E-1 (already present) to lock the behavior.

### TR-7 `Newline normalization (CRLF on Windows)`

- Related Plan Item: `T-7`
- Topic: Cross-platform Compatibility
- Reviewer Position: Prefer Option A (accept for now, document) — but add a kill-switch
- Advice:
  The PLAN's "accept, document `git config core.autocrlf false`" is pragmatic. But add a defensive measure: before hashing, normalize `\r\n` → `\n` in hash_bytes for files we know are text (based on destination extension or just always for Ark templates, which are all markdown). This would silently fix the Windows user's problem without them needing to know git config.

  Counter-argument: normalization means the stored hash doesn't equal `sha256(on-disk bytes)`, which is confusing when debugging ("my file's sha matches what I computed but Ark says it doesn't"). Ship without normalization; revisit if CRLF-flagged-as-modified reports come in.
- Rationale:
  Silent normalization is a rabbit hole (binary files, intentional CRLF, YAML-sensitive files). Since Ark ships only `.md` templates today, a future `normalize_newlines_for_hashing` flag could be added when the first complaint arrives.
- Required Action:
  Keep as stated for phase 1. Add a NG-8 ("No CRLF normalization; documented as a known gotcha") to be explicit.
