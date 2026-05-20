# `subagent-support` REVIEW `01`

> Status: Closed
> Feature: `subagent-support`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: 0
- Non-blocking: 4

## Summary

Iteration 01 substantively resolves both HIGH findings from iteration 00. R-001 is fixed by adding `Platform::extra_dirs` and deriving `Layout::owned_dirs()` from the registry; this is verified against the actual source — `Vec<PathBuf>` is compatible with both existing call sites in `unload.rs` (iter `for` and `.iter().try_for_each`) and `load.rs`. R-002 is fixed by locking C-25 to the verified Trellis prior art at `reference/Trellis/.codex/agents/trellis-research.toml`; the key set, `sandbox_mode = "workspace-write"`, `developer_instructions`, and the two `[features]` flags all match byte-for-byte. R-005 (manifest registration) is fixed via `manifest.record_file(&path)`; the API exists as a real, infallible setter on `Manifest` (`state/manifest.rs:60`). The `## Spec` is self-contained — no references to prior iterations — so the §3.2 mandatory-rejection rule is not triggered. Remaining issues are quality-of-life: a stray `?` on the infallible `record_file` call in the API-Surface snippet, the `apply_managed_state` ordering ambiguity vs the new manifest record, a trade-off blind spot around `#[non_exhaustive]` and the existing tests that still construct `Platform` literals, and one Acceptance-Mapping over-claim where C-3 maps to round-trip tests that don't exclusively exercise the derivation.

---

## Findings

### R-001 `manifest.record_file` is infallible — stray `?` in the API-Surface snippet

- **Severity:** LOW
- **Section:** `[**API Surface**]` (line 227, the `apply_managed_state` body sketch)
- **Problem:** The sketch shows `manifest.record_file(&path)?;`. The actual API at `crates/ark-core/src/state/manifest.rs:60` is `pub fn record_file(&mut self, path: impl Into<PathBuf>)` — return type is `()`, not `Result<()>`. The `?` will not compile. This is a sketch, but a Spec-level sketch is the contract Executor follows.
- **Why it matters:** A reader implementing Phase 2 step 6 will copy the snippet, hit a compiler error, and either remove the `?` (correct) or chase phantom error variants (wasteful). Since the SPEC is the artifact that promotes verbatim on commit, the snippet should compile in shape.
- **Recommendation:** Drop the `?`: `manifest.record_file(path.clone());` (and note that `record_file` takes `impl Into<PathBuf>` by value — the snippet's `&path` will not coerce into `PathBuf` either; pass `path` or `path.clone()`).

### R-002 `apply_managed_state` ordering vs `extra_files` is unspecified

- **Severity:** LOW
- **Section:** `[**API Surface**]` (the inserted block "before `extra_files`")
- **Problem:** The PLAN says the new agents block goes "before `extra_files`". The existing `apply_managed_state` (verified in `platforms.rs:90-104`) iterates: managed-block → hook → `extra_files`. The new block sitting before `extra_files` means: block → hook → agents → extras. That's plausibly fine, but the plan does not justify it relative to (a) the hook step, which can fail and short-circuit before any platform-tree state lands; (b) the idempotency claim in V-F-1 — second-call no-op depends on `write_bytes` being content-comparing or hash-tracked, neither of which is true for the agents path (C-5 says re-applied unconditionally). V-F-1 will pass only because `write_bytes` overwrites with identical content, which is not a "no-op" in the strict sense (mtime changes, FS sync occurs).
- **Why it matters:** "Idempotent" in V-F-1 will read as "no disk change on second call" to a future maintainer; the current write path does change mtimes. If a downstream tool (a watcher, a build system) keys on mtime, "idempotent" leaks. Worth one sentence in either C-5 or V-F-1.
- **Recommendation:** Tighten V-F-1 to assert "second call leaves file contents byte-identical" (matches the current implementation's behavior) and note in C-5 that re-application is content-idempotent but not mtime-idempotent. Or, alternatively, gate the write on `read_bytes() != entry.contents` to make it truly idempotent.

### R-003 C-27 (`#[non_exhaustive]`) introduces struct-literal breakage in this crate's own tests

- **Severity:** LOW
- **Section:** C-27 / T-9 / Phase 2 step 3
- **Problem:** `#[non_exhaustive]` on `Platform` *outside* the defining crate forces struct-update syntax (`Platform { id: …, ..base }`) on consumers. *Inside* `ark-core`, the existing three `<PLATFORM>_PLATFORM` consts (verified at `platforms.rs:236-291`) use full named-field initializers — adding `agents_templates`, `agents_dest_dir`, `extra_dirs` to the struct without populating them in those literals is a compile error regardless of `#[non_exhaustive]`. The PLAN says Phase 2 step 4 populates the consts — good. But T-9 promises that future field additions are "non-breaking" via `#[non_exhaustive]` — that's only true for *external* consumers. Inside the crate, every literal still needs every field. The trade-off as written overstates the protection.
- **Why it matters:** A future maintainer reading T-9 will believe they can add a field and only update test fixtures. They'll discover at compile time that the three registry consts also need updating. That's fine — but the trade-off should say so.
- **Recommendation:** Reword T-9: "future struct-field additions are non-breaking *for downstream consumers* of `Platform`; the registry consts inside `ark-core` continue to require explicit initialization." Or document a `Default for Platform` impl as a follow-up if the registry consts are expected to grow further.

### R-004 Acceptance-Mapping for C-3 doesn't exclusively exercise the derivation

- **Severity:** LOW
- **Section:** `[**Validation**]` Acceptance Mapping row for C-3
- **Problem:** C-3 maps to "V-IT-5a/b/c (round-trip exercises the derived `owned_dirs` for every platform)". The round-trip tests verify *behavior* (files round-trip), not the *derivation* (each entry of `owned_dirs()` is derived from `PLATFORMS`). A regression where a maintainer hard-codes the four entries while keeping the per-platform `extra_dirs` lists would still pass V-IT-5a/b/c (the file-set is the same), even though C-3's derivation invariant is broken.
- **Why it matters:** C-3 is the load-bearing mechanism that prevents Claude's split-root case from regressing if a fourth platform is added later. The structural test is cheap: assert `Layout::owned_dirs()` length equals `1 + sum(extra_dirs.len() + 1 for p in PLATFORMS)`, and assert each `removal_root` and each `extra_dirs` entry appears resolved against `layout.root()`.
- **Recommendation:** Add a unit test `owned_dirs_derives_from_registry` (`V-UT-16`) that asserts `Layout::owned_dirs()` is exactly `{ark_dir} ∪ {removal_root for p in PLATFORMS} ∪ {extra for p in PLATFORMS for extra in p.extra_dirs}`. Map C-3 to this test plus the round-trip set.

---

## Trade-off Advice

(No new trade-off advice. Iteration 00's TR-1 was rejected with a defensible reason in T-10; TR-2 was accepted; TR-3 needed no PLAN change. Iteration 01's T-9 and T-10 are honest about their respective trade-offs once R-003's wording fix lands.)
