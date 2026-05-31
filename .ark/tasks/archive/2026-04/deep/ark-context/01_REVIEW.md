# `ark-context` REVIEW `01`

> Status: Closed
> Feature: `ark-context`
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
- Blocking Issues: 2
- Non-Blocking Issues: 5



## Summary

Iteration 01 lands the three HIGH findings cleanly in concept: the C-17 / C-18 split (no hash, captured in snapshot) is the right shape and matches the existing `CLAUDE.md` precedent at `commands/upgrade.rs:545` and `commands/init.rs:115`. R-003's predicate is now testable. R-004 / R-006 / R-009 / R-010 are resolved as recommended. However, two structural problems remain: (1) **C-21 cwd discovery is the wrong shape for `init` and `load --force`** — both commands legitimately operate on a directory that may not yet contain `.ark/`, so a uniform `discover_from` walk-up will either error on `init` in a fresh directory or silently re-target a parent that already has `.ark/`; the PLAN's "applies uniformly" wording is incorrect and Unresolved §2 punts the question to me. I take a position below: **carve out, don't expand scope.** (2) **C-18 `Snapshot::hook_bodies` lacks an explicit `serde(default)` declaration** — adding a non-`Option` `Vec` field to the existing `Snapshot` struct without `#[serde(default)]` will fail to deserialize older `.ark.db` files that lack the field, and the PLAN does not declare this. Phase 0 means few real-world `.ark.db` files exist, but this is a 1-line addition that makes the schema bump principled rather than accidental. Several MEDIUMs (C-26 process-spawn locality cross-check; upgrade-SPEC amendment scope; V-IT-13 verification path) need tightening but are not blocking. T-7 stance: **reject scope expansion in this task.**



## Findings

### R-001 (resolved) `.claude/settings.json` round-trip via Snapshot::hook_bodies

- Severity: resolved
- Section: C-17, C-18, G-11 (revised)
- Status:
  Accepted. The mechanism (no template-tree entry, no hash, captured into a typed snapshot slot, restored via `update_settings_hook`) is coherent. `Snapshot::hook_bodies: Vec<SnapshotHookBody>` with explicit `path` / `json_pointer` / `identity_key` / `identity_value` / `entry` is the right abstraction — strictly more typed than `SnapshotBlock`, no surprise. `unload` capture and `load` restore paths are realistic against the existing snapshot.rs shape (Vec field append + `update_settings_hook` call after scaffold mirrors how `managed_blocks` work today). Caveat: see R-101 (serde default missing).

### R-002 (resolved) Hash-tracking exemption matches CLAUDE.md precedent

- Severity: resolved
- Section: C-17, T-3 (revised)
- Status:
  Accepted. I verified the precedent: `commands/init.rs:115`, `commands/load.rs:92`, `commands/upgrade.rs:545` all call `update_managed_block` unconditionally with `MANAGED_BLOCK_BODY` for `CLAUDE.md`, and the manifest records a *block* not a hashed file. The 01 plan's claim "re-applied unconditionally on every upgrade, no hash, no prompt" is exactly what the existing code does for `CLAUDE.md`. C-17 / G-8 / T-3's choice (b) replicate this faithfully. See R-105 for an upgrade-SPEC amendment nit.

### R-003 (resolved) Projection filter predicate

- Severity: resolved
- Section: G-7 (revised), C-20
- Status:
  Accepted. C-20's normalize-and-suffix-match predicate is unambiguous and locally testable; V-UT-23 covers the parser, V-UT-24 covers the filter. The `normalize` function is fully specified ("strip leading `./` and leading `.ark/`"). One minor note: the C-20 grammar restricts feature slugs to `[a-z0-9_-]+`. That matches existing `specs/features/` convention but is not enforced anywhere else in the codebase; reasonable as a scoped invariant for this parser only. No change required.



### R-101 `Snapshot::hook_bodies` is a new non-Option field with no `#[serde(default)]`

- Severity: HIGH
- Section: `[**Data Structure**]` Snapshot extension; C-18
- Problem:
  The plan adds `pub hook_bodies: Vec<SnapshotHookBody>` to `Snapshot` without specifying a `#[serde(default)]` attribute or a snapshot-schema version bump. `Snapshot::read` at `crates/ark-core/src/state/snapshot.rs:92` calls `serde_json::from_str(&text)` which requires every non-`Option` field to be present in the JSON unless tagged `#[serde(default)]`. Existing `.ark.db` files written by Ark `0.1.0`–`0.1.2` have no `hook_bodies` key; deserializing them will fail with `Error::SnapshotCorrupt`. The existing `version: SCHEMA_VERSION` field (`"1"`) is checked nowhere — there is no version-gated migration path.
- Why it matters:
  Two failure modes: (a) a user upgrading from `0.1.x` who has an existing `.ark.db` (e.g. unloaded a project before upgrading the CLI) gets a corrupt-snapshot error on `ark load`. (b) `unload`/`load` round-trip tests written against fixture `.ark.db` files lacking the new field will fail. The fix is one line — add `#[serde(default)]` to the new field — but it must be specified in the plan so the executor doesn't ship the obvious default (no attribute) and break the smoke test in `AGENTS.md:83`.
  Phase-0 status mitigates the *external* impact (few users), but the round-trip test itself is constructed in fixtures, and forward-compat (older snapshots → newer reader) is the cheap-and-correct invariant the snapshot was clearly designed for (note the unused `version: SCHEMA_VERSION` constant).
- Recommendation:
  Add a constraint to 02_PLAN's Spec section:
  > **C-27 (snapshot forward compatibility):** New `Snapshot` fields added in this task carry `#[serde(default)]`. `Snapshot::hook_bodies` defaults to an empty vec when absent. Older `.ark.db` files (snapshot schema 1, no `hook_bodies` key) deserialize successfully and produce `hook_bodies: vec![]`. `SCHEMA_VERSION` is **not** bumped — the change is purely additive at the serde level.
  Add a unit test: `Snapshot::read` succeeds against a fixture JSON lacking the `hook_bodies` key, returning `Snapshot { hook_bodies: [], .. }`.



### R-102 C-21 "applies uniformly to all commands" is wrong for `init` and `load --force`

- Severity: HIGH
- Section: C-21, T-7, Unresolved §2
- Problem:
  C-21 declares: "Applies uniformly to `init`, `load`, `unload`, `remove`, `upgrade`, and `context` to keep cwd semantics consistent." But `init` is the command that *creates* `.ark/` from scratch; its precondition is by construction "no `.ark/` here yet" (or the user passes `--dir`). If `Layout::discover_from(cwd)` walks ancestors looking for `.ark/`, then on a fresh project where the user runs `ark init` from a subdirectory of an already-Arked parent, the walk-up will find the *parent's* `.ark/` and silently re-target the wrong project. If there is no Arked ancestor, the walk fails with `Error::NotLoaded` — which is a nonsensical error for `init`, whose entire job is to turn a not-loaded directory into a loaded one.
  `load --force` has the same issue: `load --force` is documented as "wipe and scaffold from templates" — see `commands/load.rs` semantics. It must be allowed to run in a directory without `.ark/`. The 01 plan has no carve-out.
  The same observation applies, less cleanly, to `unload` and `remove`: they need `.ark/` to exist at the *exact* target, not at some ancestor. If a user runs `ark unload` from `~/repo/.ark/tasks/ark-context/`, walking up to find `~/repo/.ark/` and unloading *that* may be correct OR may surprise the user — depending on intent. The current behavior (without discover) is unambiguous: target = cwd, no `.ark/` ⇒ error.
- Why it matters:
  C-21 expands cwd-discovery to commands that don't want it, in a single PR, with no carve-out. The `init`-from-Arked-subtree case is a real foot-gun: a user inside `~/repo/sub/` who runs `ark init` expecting to scaffold in `~/repo/sub/` would instead re-init `~/repo/`. The `load --force` case is plain broken (cannot scaffold a fresh project below an already-Arked parent without `--dir`).
  T-7 honestly flags this as scope expansion and asks for guidance. My answer is below in the T-7 stance.
- Recommendation:
  Restrict C-21 to discovery-needing commands. Replace the wording with:
  > **C-21 (revised):** New `Layout::discover_from(cwd)` walks ancestors of `cwd` until `.ark/` is found, else returns `Error::NotLoaded { path: cwd }`. Used by **read-only commands that require an Ark project** (`context`, plus `unload`, `remove`, `upgrade`, and `load` *without* `--force`). `init` and `load --force` continue to use the existing `TargetArgs::resolve` (cwd or `--dir`, no walk-up) since their job is to scaffold a project, not locate one. `--dir` always wins over discovery for every command.
  Add a unit test for the carve-out: `init` from inside an Arked tree without `--dir` writes a *new* `.ark/` at cwd, not at the ancestor.
  This is consistent with `load`'s asymmetric semantics today: `load` without `--force` requires `.ark.db` or wipes nothing; `load --force` is the reset switch.



### R-103 C-26 "no `Command::new` outside `io/git.rs`" lacks an enforcement test

- Severity: MEDIUM
- Section: C-26
- Problem:
  C-26 states: "Enforced by code review + manual grep on PRs." The Acceptance Mapping line for C-26 says "Clippy / manual review (no `Command::new` outside `io/git.rs`)". Clippy does not enforce this; manual grep on PRs is honor-system. Compare with the existing `upgrade.rs:763` test (`upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`) which does an `include_str!` line scan to enforce the analog C-12 / C-13 invariants at compile time. There is precedent in this exact codebase for enforcing locality constraints in tests.
- Why it matters:
  C-26 is the kind of constraint that drifts the moment someone adds a small helper in `gather.rs` "just temporarily" calling `Command::new("git")` to skip the round-trip through `io/git.rs`. Without a test, a reviewer reading a PR diff six months from now has nothing to point at. The 01 plan rejects `merge_json_managed`'s generality on YAGNI grounds; that's correct, but YAGNI cuts both ways — if C-26 doesn't merit a test, it doesn't merit being a constraint.
- Recommendation:
  Either:
  (a) Add to Validation: `commands_no_bare_command_new` — `include_str!` walk over every `commands/**/*.rs` file (excluding `tests` modules), assert no `Command::new` literal. Mirrors the existing `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` pattern. Add to the Acceptance Mapping as the C-26 row's validation.
  (b) Drop C-26 entirely; rely on code review for new modules. Personally I lean (a): the existing precedent makes it a 10-line test.



### R-104 C-18 trade-off (sibling user hooks not captured) is acceptable but documentation is thin

- Severity: MEDIUM
- Section: C-18, V-IT-12, G-11 step (iv)
- Problem:
  C-18 documents: "User-edits to *unrelated* hook entries are NOT captured by `hook_bodies` and do NOT survive a round-trip — they are user-owned content outside Ark's owned dirs." This is internally consistent (Ark's "owned dirs" model already excludes most of `.claude/`), and matches the precedent for `CLAUDE.md` (where only the managed-block body round-trips, not user content elsewhere in the file). I take this as the correct trade-off.
  However: V-IT-12 only verifies the *Ark* entry round-trips. There is no test for "user adds a `PreToolUse` entry, then `unload` → `load`, the user entry is gone." If the documented behavior is "user entries do not survive," the test should assert the documented behavior, not just the Ark entry.
- Why it matters:
  Future readers of V-IT-12 may interpret the absent assertion as "user entries also survive" and break the documented contract by accident. Make the asymmetry explicit in tests.
- Recommendation:
  Extend V-IT-12 to assert two facts after `unload → load`: (i) Ark entry present, (ii) any user-added `PreToolUse` entry present at unload time is **absent** at load time (or alternatively, document that `unload`'s pre-existing capture path catches `.claude/settings.json` somehow — which it does not, per Layout::owned_dirs — so the assertion stands).
  Take a position in the plan: this is a deliberate non-feature, not a bug. Future work could add a generic `hook_bodies` capture for non-Ark entries; out of scope here.



### R-105 Upgrade SPEC amendment: CHANGELOG row is sufficient but no SPEC body update is planned

- Severity: MEDIUM
- Section: Phase 4 step 1, T-3
- Problem:
  The 01 plan's Phase 4 step 1 says "Append CHANGELOG row to `specs/features/ark-upgrade/SPEC.md`: terse note about `.claude/settings.json` joining `CLAUDE.md` as re-applied-not-hashed." But the upgrade SPEC body itself contains constraint C-8 ("The `CLAUDE.md` managed block is re-applied on every upgrade via `update_managed_block` with `MANAGED_BLOCK_BODY`. Not hash-tracked.") This is a hand-rolled exemption, not a generic principle. Adding a sibling exemption for `settings.json` warrants either (a) extending C-8 to "the `CLAUDE.md` managed block AND the `.claude/settings.json` Ark hook entry" or (b) adding a sibling C-8b. A CHANGELOG row alone leaves future readers guessing what the new constraint actually is.
  The workflow doc (workflow.md §5) says: "If a PLAN contradicts an existing feature SPEC, REVIEW flags it. Either the PLAN conforms or explicitly updates the SPEC." The 01 plan's mechanism (settings.json bypasses upgrade's hash classifier) is consistent with C-8 in spirit but adds new on-disk behavior the SPEC body doesn't currently anticipate.
- Why it matters:
  CHANGELOG entries are append-only history; SPEC constraints are the live contract. A reader of the post-archive `ark-upgrade/SPEC.md` checking "does upgrade ever rewrite a hash-untracked file?" would see only C-8 (CLAUDE.md) — and miss settings.json — unless the SPEC body itself is amended.
- Recommendation:
  In Phase 4 step 1, expand to: "Amend `ark-upgrade/SPEC.md` C-8 to read: 'The `CLAUDE.md` managed block AND the `.claude/settings.json` Ark `SessionStart` hook entry are re-applied on every upgrade. Not hash-tracked. Identity for the hook entry: `command == ark context --scope session --format json`.' Append a CHANGELOG row noting the addition."



### R-106 V-IT-13 ("upgrade re-adds deleted Ark hook") is the only positive test for the upgrade path

- Severity: MEDIUM
- Section: V-IT-13, G-11 step (v)
- Problem:
  V-IT-13 covers: "user deletes the Ark `SessionStart` entry → `ark upgrade` → entry is re-added." That's the deletion-recovery path. But the more common case — user runs `ark upgrade` with the Ark entry intact — has no positive test. The 01 plan's claim is "re-applied unconditionally," which means `update_settings_hook` runs on every upgrade and is idempotent. There's no test that running `ark upgrade` twice in a row produces byte-identical `.claude/settings.json` (the analog of the `CLAUDE.md` `update_managed_block` idempotence implicit in `upgrade_is_noop_right_after_init` at upgrade.rs:830).
- Why it matters:
  If `update_settings_hook` rewrites the file on every call (even when no change is needed), `installed_at` updates aside, the on-disk JSON serialization could subtly drift (e.g., key reordering by `serde_json::Value`). For a user who diffs their `.claude/settings.json` against git after every `ark upgrade`, drift is annoying. The C-12 / V-UT-20 idempotence test covers `update_settings_hook` in isolation, but not in the context of a full `upgrade()` run where other writes happen.
- Recommendation:
  Add an integration test: `upgrade_settings_hook_idempotent` — `ark init` → `ark upgrade` → snapshot the file → `ark upgrade` again → assert byte-identical. Mirrors the spirit of `upgrade_is_noop_right_after_init`. Cheap; one extra test.



### R-107 V-UT-22 documents "overwrites user-modified Ark entry" without a clear escape hatch

- Severity: LOW
- Section: V-UT-22, T-3 disadvantages
- Problem:
  V-UT-22 says `update_settings_hook` overwrites a user-modified Ark entry back to canonical form, with the documented rationale "Users wanting custom commands add siblings, not edit the Ark entry." That's the same trade-off as `CLAUDE.md`'s managed block, fair enough. But V-UT-22's wording ("overwrites the entry back to canonical form") doesn't specify what happens to extra keys the user may have added to the Ark entry itself (e.g., they kept `command` intact but added `"timeout": 10000`). C-11's identity-key rule says the entry is matched by `command`; the resolution is replace, not merge. So extra keys are lost. That's defensible but worth making explicit.
- Why it matters:
  Future contributors maintaining `update_settings_hook` need to know whether "user added `timeout: 10000` to the Ark entry" should be preserved (merge) or stomped (replace). The 01 plan's wording leaves it ambiguous.
- Recommendation:
  Reword V-UT-22's documented behavior: "When the user modifies the Ark entry (any subkey: `command`, `timeout`, etc.), `update_settings_hook` replaces the entire entry with the canonical Ark template. Users who want custom hook configuration add a sibling array entry with a different `command` value." Update the test fixture to seed `timeout: 99999` on the Ark entry and assert it reverts.



## Trade-off Advice

### TR-7 cwd-discovery scope (NEW — Plan T-7)

- Related Plan Item: `T-7`
- Topic: Flexibility vs Safety; Compatibility vs Clean Design
- Reviewer Position: Prefer carve-out (reject scope expansion in this task)
- Advice:
  Apply `Layout::discover_from` only to commands that semantically require an existing `.ark/`: `context`, `unload`, `remove`, `upgrade`, and `load` *without* `--force`. Keep `init` and `load --force` on the current "cwd or `--dir`, no walk-up" semantics.
- Rationale:
  The whole point of cwd-discovery is so that `ark context` invoked from a hook (whose cwd may be a subdirectory) finds its project. That motive doesn't generalize to `init` — which needs a *target* directory, not an *ancestor*. An `init`-from-an-Arked-subtree foot-gun (silently re-targeting the parent) is exactly the kind of cross-cutting surprise that justifies the deep tier; the 01 plan should ship the narrow cut and revisit if a real call site demands wider semantics later.
  The "scope expansion" concern T-7 raises is real: 5 commands' dispatch arms touched in one task whose title is `ark-context`. The narrow carve-out keeps `context`'s motive intact while leaving `init` semantics unchanged — backward-compat-safe, no integration tests need rewriting, and `load --force` remains usable for fresh scaffold below an Arked parent.
  The `Unresolved §2` framing ("acceptable in this task, or carve out into a sibling task") presents a binary; my answer is the third option: **narrow C-21 in this task, no follow-up needed.** No sibling task because the carve-out is the correct end state, not a deferral.
- Required Action:
  Replace C-21 with the revised wording in R-102. Add the carve-out test (R-102's recommendation). Mark T-7 trade-off as *accepted with carve-out* in 02's response matrix.



### TR-others (unchanged)

T-1 through T-6 are unchanged from 00 and remain accepted; my prior advice (TR-1 through TR-6 in 00_REVIEW) stands. T-3's option (b) selection is correct. No new trade-off advice for those items.
