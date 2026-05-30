# `ark-sandbox` REVIEW `01`

> Status: Closed
> Feature: `ark-sandbox`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `5`

## Summary

The redraft resolves the CRITICAL git-model contradiction substantively: I reproduced a linked-worktree layout and confirmed the new commit-in-box mount set is sufficient — `refs/heads/<branch>` lives in the common dir, the worktree gitdir is *nested inside* the common dir, and an in-box `git commit` succeeds when the common dir is mounted rw. Nine of ten prior findings (R-002..R-010) are genuinely fixed, not merely asserted: the SHA-256 `hash8` makes naming collision-resistant (R-002), `deny_unknown_fields` now sits on the inner section exactly as `upgrade/strategy.rs` does (R-007, verified), `resolve_focus_slug` correctly cites concurrency SPEC C-23 and is correctly placed beside `load_state`/`clear_focus_for_slug` in `state/checkout/` (R-004), the yolo map matches the verified `PLATFORMS` order/flags with opencode excluded (R-005, verified against `platforms.rs`), and the error templates are E-9/E-15 compliant (R-009). The verdict is held back from full Approval by one HIGH: the `## Spec` is **not self-contained** — line 226 delegates the option structs to "as 00_PLAN", which the mandatory self-containment rule forbids in a verbatim-promoted SPEC. Two MEDIUM issues remain: TR-8's blast-radius characterization is materially understated (mounting the whole common dir rw exposes every sibling worktree's git state and every branch ref, contradicting TR-8's "other worktrees' working state stays out of the mount set"), and R-003's image-must-tolerate-an-arbitrary-uid requirement was dropped while only the `--user` flag was added. All are addressable in one iteration without restructuring.

---

## Findings

### R-001 `## Spec delegates option structs to "as 00_PLAN" — not self-contained`

- **Severity:** HIGH
- **Section:** `[**Data Structure**]` (01_PLAN.md:226)
- **Problem:** Inside the `## Spec` block, line 226 reads `// options + summaries (all summaries impl Display) — as 00_PLAN, plus SandboxEnterOptions.platform:`. Only `SandboxEnterOptions` is restated in full (lines 227–230); the summaries are restated as inline comments (lines 231–234). But `SandboxCreateOptions`, `SandboxRmOptions`, and `SandboxListOptions` (defined in 00_PLAN.md:230/243/248) appear **nowhere** in 01_PLAN — the phrase "as 00_PLAN" is the only thing standing in for their field lists (`{ project_root, slug, recreate }`, `{ project_root, slug, keep_volume }`, `{ project_root }`). This trips the mandatory self-containment rule: the `## Spec` references a prior iteration rather than restating in full.
- **Why it matters:** This block is promoted verbatim into `specs/features/ark-sandbox/SPEC.md`. A reader of the promoted SPEC has no access to 00_PLAN (a superseded draft); the three option structs become unrecoverable from the SPEC alone. The rule exists precisely so the SPEC is the single source of truth.
- **Recommendation:** Inline `SandboxCreateOptions`, `SandboxRmOptions`, and `SandboxListOptions` (3 short struct definitions) where line 226 sits, and delete the "as 00_PLAN" phrase. No other change needed; this is the sole self-containment break in the document.

### R-002 `TR-8 understates the commit-in-box blast radius (whole common dir is rw)`

- **Severity:** MEDIUM
- **Section:** `[**Trade-offs**]` TR-8 (01_PLAN.md:372); `[**Data Structure**]` `GitMounts` (lines 182–185); C-8
- **Problem:** TR-8 closes with: "other worktrees' working state stays out of the mount set, so the widened surface is the object store + this branch's ref, not sibling worktrees." I reproduced the layout: the common dir is the main repo's entire `.git/`, which holds `refs/heads/*` for **every** branch (not just this task's), `packed-refs`, `config`, `hooks/`, and `.git/worktrees/<every-sibling>/` (each sibling's `HEAD`, `index`, `logs`, `refs`). Mounting it rw hands the box write access to all of those. The literal claim that "sibling worktrees' git state stays out of the mount set" is false — only the sibling *working trees* (separate directories) stay out; their per-worktree git dirs and refs are fully exposed and writable. (Note also that `GitMounts.worktree_gitdir` is redundant: it is nested inside `common_dir`, so the common-dir mount already covers it — harmless, but the two-mount framing in C-8/line 146 overstates what is needed.)
- **Why it matters:** TR-8 is the maintainer-authorized justification promoted into the SPEC's trade-off record. An honest blast-radius statement is the whole point of writing the trade-off out; the current wording would let a future reader believe the cage is narrower than it is, and could mask a real concern (a compromised yolo agent can rewrite `refs/heads/main` or corrupt a sibling worktree's HEAD).
- **Recommendation:** Restate TR-8's last clause to name the real surface: the box can write the shared object store, **all** branch refs (`refs/heads/*` + `packed-refs`), `.git/config`/`hooks`, and every sibling worktree's per-worktree git dir — only the sibling working *trees* stay out. The decision itself is not relitigated; only the characterization needs to match reality. Optionally drop the redundant `worktree_gitdir` mount from C-8/line 146 since the common-dir mount subsumes it (or keep it and note it is a sub-path of the common-dir mount).

### R-003 `--user added but the arbitrary-uid image-tolerance requirement was dropped`

- **Severity:** MEDIUM
- **Section:** Constraints C-22 (line 320); `[**Architecture**]` (`sandbox/Dockerfile`, lines 116–117)
- **Problem:** Prior R-003 required two things: pass `--user $(id -u):$(id -g)` **and** "ensure the image tolerates an arbitrary uid, noting agent-infra's `resolveBuildUid`." 01_PLAN adds the flag (C-22, V-UT-8) but says nothing about the image side. With `--user <uid>:<gid>` the container process runs as a uid that almost certainly has no entry in the image's `/etc/passwd` and no pre-owned `$HOME`; the agent config dir (mounted from the named volume, G-5/C-10) and any tool that writes under `$HOME` must work for an arbitrary uid. Research §D.5 risk 6 and line 141(c) also flag that rootless Docker already maps container-root to the host user (agent-infra's `resolveBuildUid` returns 0/0 there), so an unconditional `--user <hostuid>` on a rootless host can land writes at a sub-uid offset rather than the host user — the opposite of C-22's stated intent.
- **Why it matters:** A version-pinned published image that bakes a fixed non-root user (the security best practice the research cites) will break the config-volume write path under `--user`, and the rootless interaction can still produce mis-owned worktree files — the exact failure C-22 claims to prevent. Discovering this post-merge is rework, and the image is a separate CI artifact that must be designed for it.
- **Recommendation:** Add a Constraint (or a note on `sandbox/Dockerfile`) that the published image must tolerate an arbitrary uid/gid (writable config dir, e.g. `$HOME`/`HOME=/tmp` fallback or world-writable config mount), and either qualify C-22 to acknowledge the rootless-maps-to-host case or cite why unconditional `--user` is still correct. A test asserting the volume mount target is writable independent of uid would close it.

### R-004 `Spec-discipline: constraints carry trailing rationale / multi-clause procedures`

- **Severity:** LOW
- **Section:** Constraints C-8, C-14, C-18, C-23, C-11; Goal G-1
- **Problem:** Several constraints are not the single declarative sentence the rubric asks for. C-8 ends "...so in-box `git commit` to the task branch succeeds" (rationale belongs in TR-8); C-18 ends "...so distinct branches never collide" (rationale in TR / inherent to the hash); C-14 bundles two clauses (default + release-coupling invariant + message content); C-23 is a three-step selection algorithm joined by semicolons; C-11 packs parse mechanics across multiple clauses. G-1 is 82 chars (>80 verb-led-capability limit, trivially). These promote verbatim into the SPEC.
- **Why it matters:** Non-load-bearing, but the SPEC is the durable artifact; trailing `so ...` rationale duplicates the Trade-offs and the multi-clause constraints read as procedures.
- **Recommendation:** Trim the trailing `so ...` from C-8 and C-18 (the why already lives in TR-8 / the hash definition). Optionally split C-14 (default vs. release invariant) and tighten G-1 by two characters. C-23 may stay as a Constraint (a "procedure that controls X" is correctly a Constraint, not a Goal) but read it as one sentence.

### R-005 `V-IT-4 commit-sufficiency assertion is host-gated; note it explicitly`

- **Severity:** LOW
- **Section:** `[**Integration Tests**]` V-IT-4 (line 396); Acceptance Mapping C-8
- **Problem:** V-IT-4 asserts `derive_git_mounts` returns the real `--git-dir`/`--git-common-dir` and that "the computed rw mount set covers HEAD/index/objects/refs sufficient for `git commit` (gated on docker availability, else the docker leg is skipped)." On the common CI host (no docker), only the rev-parse derivation runs; the actual in-box `git commit` — the riskiest behavior and the whole point of the R-001 rework — is skipped. The plan is honest that it is gated, so this is not a coverage gap so much as a documentation nuance.
- **Why it matters:** C-8 maps to V-UT-7 (argv shape) + V-IT-4 (commit). If a reader assumes V-IT-4 always exercises a real commit, they overestimate routine coverage; the commit path is only exercised on a docker-equipped host.
- **Recommendation:** Keep V-IT-4 as drawn (a docker-gated real commit is the only faithful test), but state in the Acceptance Mapping or V-IT-4 that the commit leg is opt-in/host-gated so the gate is visible at SPEC level. No new test required.

---

## Trade-off Advice

### TR-1 `Commit-in-box rw git mounts — accepted, but document the surface honestly`

- **Related Plan Item:** TR-8 / C-8 (see R-002)
- **Topic:** Flexibility vs Safety
- **Reviewer Position:** Keep with clarification
- **Advice:** The maintainer-authorized commit-in-box decision is sound and the mount set is correct — I verified an in-box commit succeeds with the common dir rw. Do not change the design. The only required action is honesty in TR-8: the rw common-dir mount exposes all branch refs, the object store, `config`/`hooks`, and every sibling worktree's per-worktree git dir, not merely "this branch's ref."
- **Rationale:** A trade-off record whose blast-radius statement is narrower than reality defeats the purpose of writing it down. The accurate surface is still an acceptable cost for a full in-box workflow, but the SPEC should not understate it.
- **Required Action:** Adopt the corrected TR-8 wording per R-002; optionally drop the redundant `worktree_gitdir` mount.

### TR-2 `--user flag vs image design for rootless/arbitrary-uid`

- **Related Plan Item:** C-22 / `sandbox/Dockerfile` (see R-003)
- **Topic:** Compatibility vs Simplicity
- **Reviewer Position:** Need More Justification
- **Advice:** Pair the `--user` flag with an explicit image-tolerance constraint, or justify why the published image is uid-agnostic by construction. The two halves of the original R-003 fix must travel together; shipping only the flag risks breaking the config-volume write path on a hardened (fixed-user) image and mis-owning writes under rootless Docker.
- **Rationale:** The image is a separate CI artifact pinned to the crate version (C-14); its uid assumptions are part of the same contract as the `--user` flag and cannot be left implicit.
- **Required Action:** Expand — add the image-tolerance constraint and qualify C-22 for the rootless case per R-003.
