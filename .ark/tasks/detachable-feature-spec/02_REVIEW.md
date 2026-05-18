# `detachable-feature-spec` REVIEW `02`

> Status: Closed
> Feature: `detachable-feature-spec`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: `0`
- Non-blocking: `3`

## Summary

Iteration 02 lands every R-010..R-017 finding from iteration 01 with concrete, well-shaped changes. The two substantive pivots — INDEX-strict walk + drift warnings (C-12, C-12a, C-12b) and the fallible `Layout::specs_feature_dir(&[&str]) -> Result<PathBuf>` (C-2a) — are coherent with the surrounding code (`Layout::resolve_safe` precedent, `ProjectedContext` already carries optional top-level fields like `record` and `truncated`). The Response Matrix faithfully tracks each closure, and the `## Spec` block is self-contained — no "see iteration 01" hedges. The new C-9a scope/promoted provenance rule is unambiguous; V-UT-14's three-level fixture pins it. Remaining concerns are pinning, not redesign: (a) Phase 1 step 2 mis-labels archive.rs:488 as a production call site when it is actually a `#[cfg(test)]` module; (b) the `error.rs` C-2a path-propagation story for `Layout::specs_feature_dir` invokes a sentinel-PRD path which deserves one sentence in the failure flow; (c) V-UT-14 should name the test file to match the precedent set by V-UT-12. None of the three blocks EXECUTE. Approving for execution.

---

## Findings

### R-018 `archive.rs:488` listed as a production call site is actually a test

- **Severity:** LOW
- **Section:** `## Implementation` Phase 1 step 2.
- **Problem:** Phase 1 step 2 enumerates "four production call sites and one test": `extract.rs:96`, `import.rs:78`, `commit.rs:175`, `archive.rs:488`, plus `commit.rs:830` (test). Reading `crates/ark-core/src/commands/agent/task/archive.rs`, line 488 sits inside `#[cfg(test)] mod tests { … fn archive_writes_no_spec_files() { … !layout.specs_feature_dir("qd").exists() … } }` — that is a test, not production. The actual production call-site count is three (`extract.rs:96`, `import.rs:78`, `commit.rs:175`); two tests need updating (`commit.rs:830`, `archive.rs:488`). Practical effect on the executor is nil — both kinds of caller need a `?` or `.unwrap()` — but the wording mismatches the worktree.
- **Why it matters:** The PLAN's call-site enumeration is what V-UT-12 (`specs_feature_dir_no_single_str_invocations`) gates against. If the executor reads "four production call sites" and trusts it, they may miss that `archive.rs:488` lives behind `#[cfg(test)]` and reach for production-grade error machinery instead of `.unwrap()`-style test handling.
- **Recommendation:** Rephrase Phase 1 step 2 as "three production call sites (`extract.rs:96`, `import.rs:78`, `commit.rs:175`) and two tests (`archive.rs:488`, `commit.rs:830`)." Single-word fix.

### R-019 `Error::InvalidFeaturePath { prd_path }` from `Layout::specs_feature_dir` needs a sentinel rule

- **Severity:** LOW
- **Section:** `## Spec` `[**Data Structure**]` (`Error::InvalidFeaturePath`) + `## Implementation` Phase 1 step 2.
- **Problem:** `Error::InvalidFeaturePath { prd_path, value, reason }` carries a `prd_path: PathBuf` field. `Layout::specs_feature_dir` is *not* PRD-anchored — callers can be `spec_import` (CLI-anchored) or future `spec_move` (also CLI-anchored). Phase 1 step 2 hand-waves: "the `prd_path` field is filled by callers using `Layout::specs_features_index()` as a sentinel for 'not from a PRD'." That sentinel rule is load-bearing — it's how the same `Error` variant serves both PRD-rooted (deep-tier commit) and non-PRD-rooted (brownfield import) failures — but it lives only inside the Implementation phase, not in the `## Spec` body. A reader of the promoted SPEC alone cannot derive what `prd_path` carries in a `spec_import` failure.
- **Why it matters:** Self-containment. C-15 says "Error messages quote the offending value verbatim" — the `value` field gives the bad segment, which is enough for users — but `Display` for `InvalidFeaturePath` is `` invalid SPEC path `<value>`: <reason> `` (per the `Error Display` block) and does not surface `prd_path` at all. So the sentinel is internal-only; an inattentive future reader may rename or strip `prd_path` and break the contract. Pin the sentinel rule in a Constraint or rename the field to something neutral (`source_path`, `origin: ErrorOrigin`).
- **Recommendation:** Either (a) add a one-line Constraint: "When `Layout::specs_feature_dir` raises `InvalidFeaturePath` from a non-PRD caller, `prd_path` is `Layout::specs_features_index()` as a sentinel; the field is internal context, not user-facing"; or (b) rename `prd_path` → `source_path` and drop the sentinel framing. Either works for EXECUTE.

### R-020 V-UT-14 names the fixture but not the test file

- **Severity:** LOW
- **Section:** `## Validation` V-UT-14.
- **Problem:** V-UT-12 sets the precedent — name the test and its source location — and was the closure of TR-7 in this same iteration. V-UT-14 (`scope`/`promoted` provenance, the closure of R-012) names the fixture shape ("three-level fixture (`features/foo/INDEX.md` rowing `csr/SPEC.md` with scope `S1`; ...)") but doesn't name the test or where it lives. Same applies to V-UT-13 / V-UT-15 / V-UT-16 — they describe behavior precisely but don't pin a `fn` name or owning module.
- **Why it matters:** Soft consistency. The grading bar for V-UT-12 was "name the actual test name and which file owns it" (TR-7). V-UT-14 is the closure for R-012, a HIGH finding; it deserves the same locating discipline. Future maintenance ("where is the C-9a test?") is one grep faster if the test is named in writing.
- **Recommendation:** For V-UT-13/14/15/16, append the test file (`crates/ark-core/src/commands/context/gather.rs` for the gather-side checks; `crates/ark-core/src/commands/agent/spec/import.rs` for V-UT-15; `crates/ark-core/src/commands/context/related_specs.rs` for V-UT-16). Naming the test is optional — the location is what matters.

---

## Trade-off Advice

(No new trade-offs raised in iteration 02 beyond TR-5/TR-6/TR-7 closures.)
