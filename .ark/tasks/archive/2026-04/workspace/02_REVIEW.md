# `workspace` REVIEW `02`

> Status: Closed
> Feature: `workspace`
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
> - Verification of prior findings

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 2 (both LOW)

## Summary

Iteration 02 cleanly resolves all five findings from `01_REVIEW.md` (1 HIGH, 2 MEDIUM, 2 LOW). The headline fix — adding `PathExt::append_text` (G-17) — is the right call: it satisfies C-2's "no bare `std::fs::*`" rule, mirrors the existing `PathExt` wrapping pattern (verified at `crates/ark-core/src/io/path_ext.rs:96-102` for `write_bytes`), and keeps Data Structure / API Surface honest with the call graph. `parse_oneline` is now declared as `pub(crate)` with explicit splitting rules. C-19's fence predicate handles backtick AND tilde fences with length-aware close matching, and NG-13 properly bounds the parser's scope. C-20's lexical fallback gracefully degrades for read-only / containerized worktree FSes. C-18 annotation makes `archive_path`'s post-rename lifetime unambiguous.

Verified independently against source: `unload.rs` does have two `walk_files_excluding` sites (line 77 Stage A, line 162 Stage B) — C-7 is correctly scoped. `PathExt` does NOT currently have `append_text` (verified by reading the trait declaration at `path_ext.rs:27-65`) — G-17 is the legitimate addition needed. `Layout: Clone` exists already (per iter 01 verification).

The `## Spec` section remains self-contained: no external file-line references, no "iteration 02" or "R-NNN" leakage, no implicit dependency on `## Log`. Spec extraction at archive will produce a clean `specs/features/workspace/SPEC.md`.

Two LOW findings are noted below; both are documentation/clarity nits and explicitly NON-blocking. The plan is ready to advance to EXECUTE.

---

## Findings

### R-001 G-17 atomicity claim is overstated for journal writes

- Severity: LOW
- Section: `[**Goals**] G-17`
- Problem:
  G-17 says "the single `write_all` call is atomic at the OS level for buffer sizes below the pipe-write atomicity limit (PIPE_BUF on POSIX); journal entries are well below this threshold (~1KB typical)."

  This conflates two separate guarantees:
  1. **PIPE_BUF atomicity** applies to *pipe* writes, not regular file writes. POSIX makes no equivalent atomicity guarantee for `write(2)` on a regular file under `O_APPEND` — though in practice on most modern Linux/macOS filesystems `O_APPEND` does provide single-writer-or-concurrent-append safety up to one page (4KB).
  2. The relevant property here is `O_APPEND`'s **race-free seek-then-write** semantics: two concurrent processes calling `write` on an `O_APPEND`-opened fd will not interleave their writes mid-buffer (the seek-to-end and write are atomic w.r.t. other appends).

  V-E-5 already acknowledges that two rapid records may race the index re-render; it does not depend on append atomicity.
- Why it matters:
  Future implementer reading the SPEC may rely on PIPE_BUF semantics that don't strictly apply. The misclaim is unlikely to cause a real bug (journal entries are <4KB and `O_APPEND` does the right thing in practice), but the technical justification is wrong.
- Recommendation:
  Reword the atomicity sentence in G-17 to:
  > "Opens with `OpenOptions::new().create(true).append(true)`, which sets `O_APPEND` on POSIX. Concurrent appenders cannot interleave writes within a single `write_all` call (seek-to-end + write are atomic per POSIX `O_APPEND` semantics for sub-page writes). Journal entries are <4KB; this is sufficient for the parallel-archive race documented in V-E-5."

  Implementation does not change. Optional fix; could also be addressed in EXECUTE phase as a docstring tweak.

### R-002 Slug-vs-task-archive race: `record_task` may run before `task_archive` rename completes on slow filesystems

- Severity: LOW
- Section: `[**Architecture**]` `task archive` call graph; `[**Constraints**] C-18`
- Problem:
  The call graph shows `record_task` running after `.current` cleanup. C-18 says it runs after `.current` cleanup, before `Ok(summary)` return. By that point, the rename has already happened (per the existing `task_archive` flow). G-6 step 2 says the fallback uses `archive_path` if it `is_dir()`.

  Subtle issue: on networked filesystems (NFS, SMB) or under heavy IO load, `fs::rename` returning successfully does NOT guarantee that subsequent stat calls from a different code path immediately see the new path. Most modern setups are fine, but `archive_path.exists() && archive_path.is_dir()` could conceivably return false in a small window even though the rename succeeded.

  Mitigations are effectively in place: G-6 step 3 falls back to `parent_layout.root()` if neither path is a valid dir, so the worst case is "commits collected from the wrong cwd" — graceful, not broken.
- Why it matters:
  Pure edge case. Won't affect local-FS users. Worth a one-line note for future maintainers debugging on NFS/CI containers.
- Recommendation:
  Optional: add a one-line note to G-6 step 2:
  > "On networked filesystems, `archive_path.is_dir()` may briefly return false after a successful rename; the step-3 fallback handles this transparently."

  No code or test change needed.

---

## Verification of Prior Findings

| ID | Severity | Plan Claim | Verified |
|----|----------|------------|----------|
| R-001 (`PathExt::append_text` does not exist) | HIGH | Accepted Option A — added G-17 | ✅ G-17 (line 216) declares the method; Data Structure at line ~444 shows the trait addition with semantics; API Surface at line 666 notes re-export; C-2 (line 671) updated to mention it; Phase 2 (line 773) implements it; V-UT-11 (line 828) tests it. Independently verified `path_ext.rs:27-65` does not currently have an append method. Promoted SPEC will now describe a real API. |
| R-002 (`parse_oneline` undefined) | MEDIUM | Accepted — declared as `pub(crate)` in journal.rs | ✅ Data Structure (line ~566 in journal.rs block) has the declaration with documented splitting rule. C-13 (line 682) cross-references it. V-UT-12 tests the splitting + blank-skipping behavior. |
| R-003 (C-19 fence handling gaps) | MEDIUM | Accepted — tightened predicate; added NG-13 | ✅ C-19 (line 691) now specifies the opening-fence regex `^([\`~]{3,})(\w+)?\s*$`, length-aware close matching, same-character-type requirement, EOF-treated-as-still-open behavior. Indented code blocks explicitly NOT treated as fenced (deliberate simplification). NG-13 (line 232) documents the limits (HTML `<pre>`, HTML comments, indented headings). V-UT-5 expanded; V-E-8 added for tilde fences specifically. |
| R-004 (C-20 canonicalize on read-only FS) | LOW | Accepted — best-effort canonicalize with lexical fallback | ✅ C-20 step 3 (lines 698-700) now reads: "Best-effort canonicalize: if it succeeds, use the canonical form; if it fails (read-only FS, permission denied, missing component), fall back to the lexical path with a single `pop()` to strip the trailing `.git` component." Only errors if even the lexical fallback can't produce a usable path. V-UT-8 (line 825) adds the lexical-fallback subcase. V-F-3 covers the unrecognized-`.git` path. |
| R-005 (`archive_path` lifetime ambiguity) | LOW | Accepted — call graph + C-18 annotation | ✅ Call graph (lines 348-351) has the annotation: "archive_path is computed earlier in task_archive's frame and is the post-rename location of the task dir. record_task receives a clone through RecordTaskOptions.archive_path; G-6's fallback chain may use it as a git cwd (it exists on disk after the rename)." C-18 (line 687) reinforces: "record_task receives archive_path via RecordTaskOptions — it is a clone of the same value task_archive computed earlier in its frame, after the rename, and points to the now-archived task dir on disk." |

### Codebase verification

| Check | Result |
|-------|--------|
| `PathExt::append_text` does not currently exist | ✅ Verified at `crates/ark-core/src/io/path_ext.rs:27-65` — trait has `read_optional`, `read_text_optional`, `read_bytes`, `read_text`, `write_bytes`, `ensure_dir`, `list_dir`, `remove_*`, `rename_to`, `hash_sha256`. No append method. |
| `unload.rs` has two `walk_files_excluding` sites | ✅ Verified — line 77 (Stage A `unload`) and line 162 (Stage B `capture_orphan_hook_entries`). C-7 correctly names both. |
| `Layout: Clone` precondition | ✅ Already derived (verified iter 01 at `layout.rs:104`). Phase 1 note ("verify or add") is harmless redundancy. |
| `## Spec` self-containment | ✅ No external file-line references inside `## Spec`. No "iteration 02" or "R-NNN" leakage. No implicit dependency on `## Log`. Spec extraction will produce clean `specs/features/workspace/SPEC.md`. |
| Acceptance Mapping completeness | ✅ All 17 Goals (G-1..G-17) and 21 Constraints (C-1..C-21) mapped. G-17 → V-UT-11 + V-IT-3. C-19 → V-UT-5 + V-E-8. C-20 → V-UT-8 + V-IT-6 + V-F-3. NG-13 — appropriately not in mapping (non-goals don't get validations). |
| New tests are well-defined | ✅ V-UT-11, V-UT-12, V-E-8 each have a clear input → expected-output description. |
| T-7 trade-off rationale | ✅ Sound. Option A (add API) preserves C-2 invariant and benefits future ark-core consumers. Implementation cost (~10 lines) is low. |

---

## Trade-off Advice

(All prior trade-offs T-1..T-6 were resolved in iter 00; T-7 was introduced and resolved in iter 02. No new open trade-offs.)

### TR-1 No outstanding trade-offs

- Related Plan Item: N/A
- Topic: N/A
- Reviewer Position: N/A
- Advice:
  No outstanding trade-offs require advice. T-1..T-6 were converted into constraints (C-19/C-20/C-21) or design choices in earlier iterations. T-7 (append API) is correctly resolved as Option A — adding `PathExt::append_text` is the right call given C-2's "no bare `std::fs::*`" invariant.
- Rationale:
  The plan has converged. All ambiguity from prior iterations has been pinned down with concrete predicates, fallback behaviors, and tests.
- Required Action:
  None. Plan is ready to advance to EXECUTE.
