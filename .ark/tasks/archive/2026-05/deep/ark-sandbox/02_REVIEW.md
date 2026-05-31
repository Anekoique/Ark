# `ark-sandbox` REVIEW `02`

> Status: Closed
> Feature: `ark-sandbox`
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

The gating self-containment check passes: scanning the entire `## Spec` block (02_PLAN.md:52–331) finds zero references to any prior iteration, and the three R-001 subjects (`SandboxCreateOptions`, `SandboxRmOptions`, `SandboxListOptions`) are now inlined in Data Structure (lines 218/233/238). Every prior finding is genuinely fixed in substance, not merely asserted. Per-finding status: **R-001 (HIGH) RESOLVED** — `## Spec` is self-contained; only the front-matter `Depends on` and the Response Matrix (both outside `## Spec`) mention `01_PLAN`. **R-002 (MED) RESOLVED** — `GitMounts` is a single `common_dir` (line 173), C-8 mounts only the common dir, and TR-8 (line 382) now names the true blast radius (object store, all `refs/heads/*`+`packed-refs`, `config`/`hooks`, every sibling per-worktree gitdir; only sibling working trees stay out). **R-003 (MED) RESOLVED** — both halves present: C-24 (uid-tolerant image) + C-22 qualified for rootless, `host_user()` trait method (line 163) and TR-9 (line 383); the rootless reasoning is technically sound and matches the research (`resolveBuildUid` returns 0/0 under rootless, so container-root maps to the host user and an explicit `--user <hostuid>` would mis-own through the userns offset). **R-004 (LOW) RESOLVED** — G-1 is 79 chars, the trailing `so …` is gone from C-8/C-18, C-14 split into C-14/C-15. **R-005 (LOW) RESOLVED** — V-IT-4 (line 407) and the C-8 mapping row both mark the in-box commit leg docker-host-gated. The renumber (01 C-1..C-23 → 02 C-1..C-25) drops no constraint: every 01 rule maps forward, C-14 split accounts for one new slot and C-24 (uid image) for the other. This is the final pass and the substance is clean, so it is Approved; three non-blocking notes remain — a non-validating mapping row for the new C-24, a re-introduced trailing-`so` rationale on C-24 (the same class R-004 trimmed elsewhere), and a subtle rootless-uid edge worth flagging for EXECUTE.

---

## Findings

### R-001 `C-24 maps to V-UT-8, which cannot validate the image-tolerance claim`

- **Severity:** MEDIUM
- **Section:** `[**Acceptance Mapping**]` C-24 row (02_PLAN.md:454); `[**Constraints**]` C-24 (line 330); `[**Validation**]` V-UT-8 (line 398)
- **Problem:** C-24 is an assertion about the *published image* — that it runs as a user tolerating an arbitrary `--user` uid/gid with a writable config dir. Its mapping row points at V-UT-8 with the note "uid-independent volume write target." But V-UT-8 is a pure argv-shape unit test: it asserts `build_run_args` includes `--user <uid>:<gid>` when `spec.user` is `Some` and omits it when `None`. That test never touches the image, never runs a container, and never writes to the config dir — it validates C-22 (the conditional flag), not C-24 (the image's uid tolerance). C-24's only faithful validation surface is the Dockerfile build plus a runtime write test, which would be docker-host-gated exactly like V-IT-4's commit leg; no such test exists in the plan.
- **Why it matters:** This is the same finding-shape the prior pass raised for V-IT-4 (R-005): a constraint whose real validation is host-gated must say so, or a reader of the promoted SPEC will believe C-24 is exercised by a routine unit test when it is not. The image is a separate CI artifact; if EXECUTE trusts the V-UT-8 mapping, the uid-tolerant image property ships unvalidated.
- **Recommendation:** Either (a) add a docker-host-gated robustness test asserting a config-volume write succeeds under an arbitrary `--user` uid and map C-24 to it, or (b) re-label the C-24 mapping row to state plainly that C-24 is validated by the Dockerfile build (CI) and is not exercised by V-UT-8, mirroring the honest gating already applied to C-8/V-IT-4. No design change is needed.

### R-002 `New C-24 reintroduces a trailing "so …" rationale that R-004 trimmed elsewhere`

- **Severity:** LOW
- **Section:** `[**Constraints**]` C-24 (02_PLAN.md:330)
- **Problem:** C-24 reads: "The published image runs as a user tolerating an arbitrary `--user` uid/gid with a writable config dir, **so volume-backed config writes succeed**." The trailing `so …` clause is rationale — the exact pattern R-004 asked to remove from C-8 and C-18 in iteration 01 (and which 02 correctly trimmed there). C-24 is new in 02 and reintroduces it. The "why" already lives in TR-9 (line 383).
- **Why it matters:** Non-load-bearing, but C-24 promotes verbatim into the SPEC; the constraint should be the single declarative sentence the rubric asks for, with rationale in Trade-offs.
- **Recommendation:** Trim to: "The published image runs as a user tolerating an arbitrary `--user` uid/gid with a writable config dir." Leave the "so writes succeed" reasoning in TR-9.

### R-003 `C-22/C-24 interaction under rootless: a non-root baked image can still mis-own through the userns offset`

- **Severity:** LOW
- **Section:** `[**Constraints**]` C-22 (02_PLAN.md:328), C-24 (line 330); `[**Trade-offs**]` TR-9 (line 383)
- **Problem:** C-22's rootless reasoning ("container-root already maps to the host user, so `--user` is omitted") is sound *only when the container process actually runs as uid 0*. C-24 simultaneously requires the image to "tolerate an arbitrary uid" — a security-hardened image often bakes a fixed non-root default user. Under rootless Docker with `--user` omitted, a non-root baked default lands writes at a sub-uid offset inside the user namespace, not at the host user, which is the opposite of C-22's intent. The two constraints are internally consistent for the config *volume* (Docker-managed, so ownership is reconciled), but a bind-mounted path written by a non-root baked user under rootless can still surface as a mis-owned host file. The research flags this exact uid/gid mapping subtlety (risk 6, "may land as uid 0 unless handled").
- **Why it matters:** It does not break the design — the worktree bind mount under rootless is the narrow remaining edge, and the volume (the C-24 target) is safe — but the C-22/C-24 pairing reads as if the rootless case is fully closed when one image-design choice (non-root default + omitted `--user`) reopens it.
- **Recommendation:** Add one clause to TR-9 (or a Dockerfile note) stating that under rootless the image is expected to default to root (uid 0, which maps to the host user) while still tolerating an explicit `--user`; that keeps C-22 and C-24 jointly correct. Addressable in EXECUTE; no restructure.

---

## Trade-off Advice

### TR-1 `Commit-in-box rw common-dir mount — design fixed, characterization now honest`

- **Related Plan Item:** TR-8 / C-8 / `GitMounts`
- **Topic:** Flexibility vs Safety
- **Reviewer Position:** Keep with clarification
- **Advice:** No change. The maintainer-authorized commit-in-box decision and the single rw `common_dir` mount are correct, and TR-8's restated blast radius now matches the data model (one mount, nesting the worktree gitdir, objects, and all refs) and C-8. The understated clause the prior pass flagged is gone and the redundant `worktree_gitdir` field is removed with a Log [Removed] entry naming the supersede.
- **Rationale:** The honest surface — writable object store, all branch refs, `config`/`hooks`, every sibling per-worktree gitdir, sibling working trees excluded — is an accepted cost on the maintainer's authorization, and the SPEC now records it accurately, which was the whole point of writing the trade-off down.
- **Required Action:** Keep as drawn.

### TR-2 `Conditional --user + uid-tolerant image — sound; close the rootless image-default gap`

- **Related Plan Item:** TR-9 / C-22 / C-24 (see R-003)
- **Topic:** Compatibility vs Safety
- **Reviewer Position:** Keep with clarification
- **Advice:** The two halves now travel together as TR-9 states, and the rootful/rootless/non-Unix split is correct. The only refinement is to make the image's expected default-uid posture explicit so the rootless case is unambiguously closed (R-003), and to give C-24 a validation surface that actually exercises the image rather than the argv (R-001).
- **Rationale:** The image is a separate CI artifact pinned to the crate version; its uid posture and its validation are part of the same contract as the `--user` flag and should not be left to inference.
- **Required Action:** Adopt the R-003 TR-9 clarification and the R-001 mapping/test fix during EXECUTE; both are additive.
