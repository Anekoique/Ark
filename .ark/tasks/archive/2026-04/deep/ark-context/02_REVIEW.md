# `ark-context` REVIEW `02`

> Status: Closed
> Feature: `ark-context`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 3



## Summary

Iteration 02 lands the two HIGH findings from 01_REVIEW cleanly and resolves all five MEDIUM/LOW items as recommended. R-101's snapshot serde-default is now C-27 with the right wording — `#[serde(default)]` on the new `hook_bodies: Vec<...>` field, no `SCHEMA_VERSION` bump (verified: `snapshot.rs:21` defines `SCHEMA_VERSION = "1"` but it is never read on the deserialize path, so additive forward-compat is the correct invariant). R-102's carve-out is now C-21 with explicit text: `init` and `load --force` call `TargetArgs::resolve()`; `unload`, `remove`, `upgrade`, `context`, and `load` without `--force` call `TargetArgs::resolve_with_discovery()`. The CLI dispatch shape supports this naturally (the `force` flag lives on `LoadArgs`, sibling to `target`, so the dispatch arm can branch). V-UT-29 covers the `init`-from-Arked-subtree foot-gun; V-UT-30 covers the `load --force` carve-out. R-103's source-scan enforcement (C-28 + V-UT-31) honestly mirrors the existing `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` precedent at `upgrade.rs:763`. R-105's SPEC body amendment is now Phase 4 step 1(a) with the exact rewrite text. R-106's byte-identical idempotence test (V-IT-14) is sound — `serde_json` default features (no `preserve_order`) yield BTreeMap-ordered output, so non-determinism is not a hidden risk. R-107 is sharpened with the `timeout: 99999` fixture. The remaining gaps are minor — see findings — and are not blockers; the executor can resolve them inline. This is the third iteration; the design is implementable as written.



## Findings

### R-201 (resolved) Snapshot forward-compat via serde default

- Severity: resolved
- Section: C-27, V-UT-28
- Status:
  Accepted. C-27's wording is unambiguous: `#[serde(default)] hook_bodies: Vec<SnapshotHookBody>`, defaults to empty vec, no version bump. Verified against `snapshot.rs:23-30` — `Snapshot` is a flat `Serialize, Deserialize` struct with no custom deserializer, so the attribute composes cleanly. Verified `SCHEMA_VERSION` at `snapshot.rs:21` is never compared on the read path (`Snapshot::read` at line 87 calls `from_str` without inspecting `version`), so "no bump" does not silently break a version gate. The fixture in V-UT-28 is realistic — an `0.1.x` snapshot serialized today literally lacks the key.

### R-202 (resolved) Discovery carve-out for `init` and `load --force`

- Severity: resolved
- Section: C-21 (revised), T-7 (revised), V-UT-29, V-UT-30
- Status:
  Accepted. C-21's revised wording is precise. The implementation split via `TargetArgs::resolve()` vs `TargetArgs::resolve_with_discovery()` is sound: `TargetArgs` lives at `main.rs:110-123` as a flat struct holding only `dir: Option<PathBuf>`, and the CLI's `--force` flag for `load` lives on `LoadArgs` as a sibling field. The dispatch arm at `main.rs:165-169` already reads `a.target.resolve()` and `a.force` separately, so adding a one-line branch (`if a.force { a.target.resolve() } else { a.target.resolve_with_discovery()? }`) is mechanical. V-UT-30 ("with `--force` succeeds, without `--force` errors") hits exactly this branch. V-UT-29 covers the `init` carve-out via the Arked-subtree fixture. The Trade-off T-7 reframing is correct (carve-out, not deferral).

### R-203 (resolved) C-28 source-scan mirrors precedent

- Severity: resolved
- Section: C-28, V-UT-31
- Status:
  Accepted. C-28 names the existing precedent at `upgrade.rs:764` and the new `commands_no_bare_command_new` follows the same pattern: `include_str!`-driven line scan, comment skip, `#[cfg(test)]` cutoff. Verified there are zero `Command::new` occurrences in `commands/` or `io/` today, so the test will pass once the new `io/git.rs` is the sole call site. The `commands/**/*.rs` scope correctly includes `commands/agent/` and the new `commands/context/`. The `#[cfg(test)]`-until-module-end heuristic inherits the same false-negative property as the precedent (an inline `#[test]` outside a `#[cfg(test)]` mod would be scanned as production), but that limitation is shared with the existing test and not a regression — see R-205.

### R-204 (resolved) Upgrade SPEC C-8 body amendment

- Severity: resolved
- Section: Phase 4 step 1, R-105 follow-up
- Status:
  Accepted. The Phase 4 step 1(a) rewrite of C-8 is the right shape — extends the existing constraint rather than adding C-8b, names `ARK_CONTEXT_HOOK_COMMAND` as the identity constant, and makes the "user customizations not preserved" trade-off explicit. CHANGELOG row appended in step 1(b). Aligns with workflow.md §5 ("Either the PLAN conforms or explicitly updates the SPEC").

### R-205 Source-scan heuristic still has a known false-negative shape

- Severity: LOW
- Section: C-28, V-UT-31
- Problem:
  `commands_no_bare_command_new` mirrors the precedent's `in_tests = true` latch (set on `#[cfg(test)]`, never reset). For files with no test module, the latch never trips and the scan covers the whole file — fine. For files where `#[cfg(test)]` appears late in the file, the scan correctly elides the tail. The shared limitation: a hypothetical `#[test]` function outside a `#[cfg(test)]` mod would be incorrectly classified as production. No file in the current `commands/` tree has this shape (every test block is gated by `#[cfg(test)] mod tests`), so the heuristic is sufficient today. The 02_PLAN already says "a tighter approach is fine if simpler", so this is documented latitude.
- Why it matters:
  The reason to flag at all is that the new test will reach a ~10-file scan instead of the precedent's 1-file scan, so any future contributor adding an inline `#[test]` in (say) `commands/init.rs` could ship a `Command::new` test fixture that the scan flags as a violation. The likelihood is low; the cost is annoyance.
- Recommendation:
  Optional, not blocking: in the executor's implementation, prefer scanning per-file with a `cfg(test)` latch that resets at file boundaries (since the test runs across multiple files), and document the inline-`#[test]` limitation in a comment near the test body. If the scan stays as a single concatenated walk like the precedent, no change needed.



### R-206 V-IT-14 idempotence does not pin `installed_at` ordering

- Severity: LOW
- Section: C-29, V-IT-14
- Problem:
  V-IT-14's wording — "byte-identical (modulo `installed_at` if it lands inside the same file, which it does not for `settings.json`)" — correctly notes that `manifest.installed_at` lives in `.ark/.installed.json`, not `.claude/settings.json`. But the assertion is on `.claude/settings.json` bytes only, not on the manifest. The hidden non-determinism risk in `update_settings_hook` is JSON object key ordering on round-trip serialize. `serde_json` defaults (no `preserve_order` feature; verified `Cargo.toml:21` plain dependency) use `BTreeMap`-backed `Map`, so output is deterministic by key. C-29 is therefore satisfiable. But the PLAN does not call out the `serde_json` default-features assumption — if a future contributor enables `preserve_order` workspace-wide, idempotence breaks silently.
- Why it matters:
  The constraint is provably true under current `Cargo.toml`, but it is implicit. A one-line note pinning the assumption ("idempotence depends on `serde_json` not having `preserve_order` enabled; if that feature is enabled in the future, `update_settings_hook` must canonicalize key order before write") would survive future feature-flag drift.
- Recommendation:
  Optional, not blocking: append to C-29 a sentence: "Depends on `serde_json` default-features (BTreeMap-backed `Map`); enabling `preserve_order` would require explicit key canonicalization in `update_settings_hook` to preserve idempotence."



### R-207 G-10 slash-command update remains unverified

- Severity: LOW
- Section: G-10, Unresolved §1 (carried forward)
- Problem:
  G-10 ("three slash commands updated to call `ark context --scope phase --for <phase> --format json`") is checked only by "Template content review + V-IT-7" in the Acceptance Mapping. There is still no mechanical assertion that the three template files contain the substring `ark context --scope phase --for`. 02_PLAN's Unresolved §1 carries this forward with the position that prose for an LLM-read prompt isn't well-served by string-equality.
- Why it matters:
  The risk is small (three files; visual diff catches drift). But the smoke test in `AGENTS.md:83` and the round-trip integration test V-IT-7 both depend on the rendered hook command landing in the right places. If a future executor edits the slash command prose and accidentally drops the `ark context` prefix, no test catches it.
- Recommendation:
  Optional, not blocking: add a unit test `slash_commands_invoke_ark_context` — `include_str!` each of `templates/claude/commands/ark/{quick,design,archive}.md` and assert each contains the substring `ark context --scope phase --for`. Five-line test; lower bound on prose drift. If 02's executor doesn't add it, the next iteration that touches these templates can.



## Trade-off Advice

### TR-7 cwd-discovery scope (carried, endorsed)

- Related Plan Item: `T-7` (revised)
- Topic: Flexibility vs Safety
- Reviewer Position: Endorsed — narrow carve-out
- Advice:
  02_PLAN's revised T-7 ("narrow carve-out: `init` and `load --force` keep current `resolve()` semantics; the rest use `resolve_with_discovery()`") is the correct end state. No follow-up task needed.
- Rationale:
  Verified against `commands/load.rs:60-79`: the existing `load` semantics already branch on `force` (force → wipe + scaffold; no force → require either snapshot or fresh scaffold-via-init). The carve-out aligns the discovery question with the same axis. `init` and `load --force` are the only commands whose job is *to create* `.ark/` rather than *to find* it; treating them as the carve-out is the principled cut.
- Required Action:
  None; PLAN already adopts this position. Iteration 02's response matrix correctly marks TR-7 Accepted.



### TR-others (unchanged from 01)

T-1 through T-6 unchanged from 01_PLAN, all previously endorsed. T-7 is the only trade-off that moved this iteration; it is now endorsed in its narrow form. No new trade-offs introduced.
