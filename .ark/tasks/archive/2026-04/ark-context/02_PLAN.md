# `ark-context` PLAN `02`

> Status: Revised
> Feature: `ark-context`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Iteration 02 is a focused delta over `01_PLAN`. It resolves the two HIGH findings (R-101 snapshot serde-default; R-102 cwd-discovery carve-out for `init` and `load --force`) and lands four MEDIUM/LOW tightenings (R-103 enforcement test for the process-spawn locality rule, R-104 V-IT-12 negative assertion for user siblings, R-105 explicit upgrade-SPEC body amendment, R-106 idempotence integration test, R-107 V-UT-22 wording sharpened).

The full Spec / Architecture / Data Structure / API Surface / Constraints / Runtime / Implementation / Validation set from `01_PLAN.md` is **carried forward unchanged** except for the deltas explicitly enumerated below in Log §Added / §Changed / §Removed. This document is read together with `01_PLAN.md`; a reader should treat 01's body as the baseline and apply 02's deltas on top.

## Log

[**Added**]

- **C-21 (revised — see §Changed) replaces the wholesale wording in 01.** The new constraint scopes `Layout::discover_from` precisely; see §Changed for the full text.
- **C-27 (snapshot forward compatibility, addresses R-101):** New fields added to `Snapshot` in this task carry `#[serde(default)]`. Specifically, `Snapshot::hook_bodies` defaults to an empty vec when absent. Older `.ark.db` files written before `hook_bodies` existed deserialize successfully and produce `hook_bodies: vec![]`. `SCHEMA_VERSION` is **not** bumped — the change is purely additive at the serde level. Future fields added under this constraint follow the same pattern (default-when-absent, no version bump for additive changes).
- **C-28 (process-spawn locality enforcement, addresses R-103):** A test mirroring `upgrade.rs::tests::upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` lives in `commands/context/mod.rs::tests::commands_no_bare_command_new`. It uses `include_str!` to read every non-test source file under `crates/ark-core/src/commands/` and asserts no occurrence of the literal `Command::new`. The test excludes `tests` modules within those files (heuristic: lines after `#[cfg(test)]` until module end); a tighter approach is fine if simpler. C-26 is upgraded from "honor system" to "enforced by test."
- **C-29 (settings-hook idempotence at the upgrade level, addresses R-106):** Running `ark upgrade` twice in a row on a project initialized via `ark init` produces a byte-identical `.claude/settings.json`. This is in addition to V-UT-20's idempotence at the helper level; C-29 covers the integration path (full `upgrade()` invocation, including the unconditional `update_settings_hook` call interacting with the rest of the upgrade pipeline).

- **V-UT-28 (R-101):** `Snapshot::read` succeeds against a fixture JSON lacking the `hook_bodies` key, returning `Snapshot { hook_bodies: vec![], .. }`. Fixture is the byte-equivalent of an `0.1.x` snapshot.
- **V-UT-29 (R-102 carve-out):** `init` from inside an Arked tree without `--dir`: setup creates `<tmp>/parent/.ark/` (a fully Arked parent), then `cd <tmp>/parent/sub/` and runs `init`. Asserts: `<tmp>/parent/sub/.ark/` exists (newly scaffolded), `<tmp>/parent/.ark/` is unchanged. No silent re-target.
- **V-UT-30 (R-102 carve-out):** `load --force` from a directory with no `.ark/` ancestor succeeds (scaffolds fresh). Without `--force`, same setup errors per existing semantics; the test confirms `--force` opts out of discovery.
- **V-UT-31 (R-103):** `commands_no_bare_command_new` source-scan test. Negative case (insert `Command::new` into a fixture string passed to the same scanner) asserts the scanner's match logic.
- **V-UT-32 (R-107):** `update_settings_hook` invoked when the on-disk Ark entry has user-added subkeys (e.g. `"timeout": 99999`) **replaces the entire entry** with the canonical form; `timeout` is not preserved. Documented behavior, asserted by the test fixture.
- **V-IT-14 (R-106):** `upgrade_settings_hook_idempotent` integration test — `ark init` → `ark upgrade` → snapshot file bytes → `ark upgrade` again → assert byte-identical (modulo `installed_at` if it lands inside the same file, which it does not for `settings.json`).
- **V-IT-15 (R-104 — extends V-IT-12):** After `unload → load` round-trip, assert (i) Ark `SessionStart` entry present, AND (ii) any user-added `PreToolUse` entry that existed at unload time is **absent** at load time. Documents the intentional non-feature: user siblings outside `Snapshot::hook_bodies` do not survive. Future work could capture them generically; out of scope here.

[**Changed**]

- **C-21 (revised per R-102):** The new wording is:
  > **C-21:** New `Layout::discover_from(cwd) -> Result<Self>` walks ancestors of `cwd` until `.ark/` is found, else returns `Error::NotLoaded { path: cwd }`. Used by **commands that require an existing `.ark/`**: `context`, `unload`, `remove`, `upgrade`, and `load` *without* `--force`. **NOT used by `init` or `load --force`** — these scaffold a fresh project and must operate on `cwd` (or `--dir`) verbatim with no walk-up. `--dir`, when supplied, always wins over discovery for every command.
  The implementation split: `TargetArgs::resolve()` returns the explicit target without discovery (current behavior); commands that need discovery call a new `TargetArgs::resolve_with_discovery() -> Result<PathBuf>` which calls `Layout::discover_from` on the resolved target if `--dir` is absent. `init` and `load --force` continue calling `resolve()`. `load` with `--force == false` calls `resolve_with_discovery()`.
- **T-7 revised:** "Apply `Layout::discover_from` uniformly to all commands" is replaced by "Narrow carve-out: discovery for read-only / `.ark/`-requiring commands; `init` and `load --force` keep current semantics." Reviewer-endorsed (TR-7).
- **Phase 1 step 2 (Implementation) revised:** Was: "update `init`, `load`, `unload`, `remove`, `upgrade` callers." Now reads:
  > Add `Layout::discover_from`. Add `TargetArgs::resolve_with_discovery()`. Update `unload`, `remove`, `upgrade`, and `load` (without `--force`) to call `resolve_with_discovery()`. **Leave `init` and `load --force` on `resolve()`.** V-UT-25, V-UT-29, V-UT-30 cover the carve-out.
- **Phase 4 step 1 (upgrade-SPEC amendment, per R-105):** Was: "Append CHANGELOG row to `specs/features/ark-upgrade/SPEC.md`." Now reads:
  > **(a)** Amend `specs/features/ark-upgrade/SPEC.md` constraint **C-8 body** to:
  > > "The `CLAUDE.md` managed block AND the `.claude/settings.json` Ark `SessionStart` hook entry are re-applied on every upgrade. Not hash-tracked. Identity for the hook entry: `command == ark context --scope session --format json` (constant `ARK_CONTEXT_HOOK_COMMAND`). The hook entry is unconditionally rewritten to canonical form on every `init` / `load` / `upgrade`; user customizations to the entry are not preserved (rationale: matches CLAUDE.md managed-block precedent)."
  > **(b)** Append a CHANGELOG row to the same SPEC noting the C-8 extension and the date.
- **V-UT-22 wording (per R-107):** Re-stated as: "When the user modifies the Ark entry by changing **any** subkey (`command`, `timeout`, or any user-added key), `update_settings_hook` replaces the entire entry with the canonical Ark template. Users who want custom hook configuration add a sibling array entry with a different `command` value." Test fixture seeds `timeout: 99999` on the Ark entry; assertion verifies `timeout` is gone after `update_settings_hook`. V-UT-32 above is the formal addition; V-UT-22 is updated in-place for clarity.
- **V-IT-12 wording (per R-104):** V-IT-12's scope is restricted to "Ark entry round-trips" (positive). The negative assertion ("user siblings do not survive") is split into V-IT-15. This avoids a single test asserting two compound facts.
- **C-26 wording:** Changed from "Enforced by code review + manual grep on PRs." to "Enforced by `commands_no_bare_command_new` source-scan test (V-UT-31)." See C-28.

[**Removed**]

- **The "applies uniformly" wording from C-21 in 01_PLAN** is removed and replaced by the carve-out version above. Concretely: the sentence "Applies uniformly to `init`, `load`, `unload`, `remove`, `upgrade`, and `context` to keep cwd semantics consistent." in 01_PLAN's C-21 is struck.
- **Unresolved §2 from 01_PLAN** is closed: TR-7 endorses the carve-out as the correct end state (no sibling task needed).

[**Unresolved**]

- **Slash-command update count and wording (G-10):** Carried forward from 01_PLAN's Unresolved §1. Still no per-file mechanical assertion. The next reviewer can elevate this if desired; my position is that prose for an LLM-read prompt isn't well-served by a string-equality test.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|-----|----------|------------|
| Review | R-101 (HIGH — Snapshot::hook_bodies serde default) | Accepted | New C-27 mandates `#[serde(default)]` and an empty-vec default; no `SCHEMA_VERSION` bump. V-UT-28. |
| Review | R-102 (HIGH — C-21 discovery breaks init / load --force) | Accepted | C-21 narrowed to `context`, `unload`, `remove`, `upgrade`, `load --force=false`. `init` and `load --force` keep current `resolve()` semantics. T-7 revised. V-UT-29, V-UT-30. |
| Review | R-103 (MEDIUM — C-26 enforcement test) | Accepted | New C-28 + V-UT-31 mirroring `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`. C-26 wording upgraded from honor-system to test-enforced. |
| Review | R-104 (MEDIUM — V-IT-12 lacks negative assertion) | Accepted | V-IT-12 scoped to positive case; new V-IT-15 covers the user-sibling-not-surviving negative case. Documented as intentional non-feature. |
| Review | R-105 (MEDIUM — upgrade SPEC body amendment) | Accepted | Phase 4 step 1 expanded to amend `ark-upgrade/SPEC.md` C-8 body in addition to the CHANGELOG row. |
| Review | R-106 (MEDIUM — upgrade idempotence test) | Accepted | New C-29 + V-IT-14 (`upgrade_settings_hook_idempotent`). |
| Review | R-107 (LOW — V-UT-22 escape hatch) | Accepted | V-UT-22 wording sharpened; new V-UT-32 with `timeout: 99999` fixture asserts whole-entry replacement. |
| Review | TR-7 (cwd-discovery scope) | Accepted | T-7 revised to carve-out per reviewer position. No sibling task. |
| Review | R-001/R-002/R-003 (resolved in 01) | Confirmed resolved | No change; reviewer R-marked these as resolved in 01_REVIEW. |

---

## Spec

*Carried forward from `01_PLAN.md` §Spec, with the following deltas:*

- **G-1 through G-12:** unchanged.
- **NG-1 through NG-11:** unchanged.
- *Constraints:* C-1 through C-26 are inherited from 01_PLAN (C-3, C-11, C-21, C-26 with their 01-revised wording, plus the C-21 narrowing in §Changed above). C-27, C-28, C-29 added in this iteration.

## Runtime

*Carried forward from `01_PLAN.md` §Runtime. No structural changes.*

The only behavioral note specific to 02: the dispatch arm for `context` in `ark-cli/src/main.rs` calls `target.resolve_with_discovery()`; `init` and `load --force` continue to call `target.resolve()`; the rest of `load`, `unload`, `remove`, `upgrade` call `target.resolve_with_discovery()`. This is a refactor of the existing dispatch shape, not a new flow.

## Implementation

*Carried forward from `01_PLAN.md` §Implementation, with these revisions:*

- **Phase 1 step 2 revised** (see §Changed above) — `resolve_with_discovery()` helper + carve-out applied to the right subset of commands.
- **Phase 1 step 5** unchanged structurally, but `Snapshot::hook_bodies` declaration in `state/snapshot.rs` carries `#[serde(default)]` per C-27.
- **Phase 3 step 5** unchanged but extended: V-IT-12 / V-IT-13 / V-IT-14 / V-IT-15 are the integration tests.
- **Phase 4 step 1 revised** per R-105 (see §Changed above): SPEC body amendment in addition to CHANGELOG row.
- New step in Phase 1 (between current 7 and 8): "Add `commands_no_bare_command_new` source-scan test (V-UT-31)."

## Trade-offs

*Carried forward from `01_PLAN.md` §Trade-offs except:*

- **T-7 revised:** Choice is "narrow carve-out" not "uniform". See §Changed. Reviewer-endorsed (TR-7) as the correct end state.

No new trade-offs introduced in this iteration.

## Validation

[**Unit Tests**]

V-UT-1 through V-UT-27 carried forward from `01_PLAN.md`. New in 02:

- V-UT-28: snapshot forward-compat (R-101).
- V-UT-29: `init` from Arked subtree carve-out (R-102).
- V-UT-30: `load --force` from non-Ark directory carve-out (R-102).
- V-UT-31: `commands_no_bare_command_new` source scan (R-103).
- V-UT-32: `update_settings_hook` whole-entry replacement (R-107).

V-UT-22 wording revised (no new test, fixture updated to seed `timeout: 99999`).

[**Integration Tests**]

V-IT-1 through V-IT-13 carried forward. New in 02:

- V-IT-14: upgrade idempotence at file-byte level (R-106).
- V-IT-15: user-sibling-not-preserved negative assertion (R-104).

V-IT-12 wording scoped to positive case only (negative split into V-IT-15).

[**Failure / Robustness Validation**]

V-F-1 through V-F-6 unchanged.

[**Edge Case Validation**]

V-E-1 through V-E-7 unchanged.

[**Acceptance Mapping**]

*Carried forward from `01_PLAN.md` Acceptance Mapping with these additions / revisions:*

| Goal / Constraint | Validation |
|-------------------|------------|
| C-21 (revised) | V-UT-25, V-UT-29, V-UT-30, V-E-5 |
| C-26 (revised) | V-UT-31 |
| C-27 (new) | V-UT-28 |
| C-28 (new) | V-UT-31 |
| C-29 (new) | V-IT-14 |
| C-11 (revised) | V-UT-22 (revised wording), V-UT-32 |

C-21's row replaces 01_PLAN's row for C-21. C-26's row replaces 01_PLAN's "Clippy / manual review" entry. All other 01_PLAN acceptance rows persist.
