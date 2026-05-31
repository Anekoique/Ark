# `ark-sandbox` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `ark-sandbox`
> Target Task: `ark-sandbox`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs` (one rolled-up PENDING for `LAYOUT.md` conformance plus a traceability sublist of every leaf).

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: PASS — `specs/project/INDEX.md` lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; on-disk children match exactly (no unlisted leaf, no dangling row).

### Leaf SPECs

- [x] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: PASS — this task adds no new convention SPEC and modifies none; the four existing leaves are unchanged by the diff. Code is audited against their rules below.
  - `LAYOUT.md` — N/A (unchanged; no new convention SPEC introduced).
  - `rust/COMMENTS.md` — mostly PASS; one broken doc-comment found, see V-002. Spot-checks: module `//!` docs present on every new file (`io/docker.rs`, all `commands/sandbox/*.rs`); first-sentence third-person present indicative (C-2/C-3) holds; no SPEC-rule labels (`C-N`/`V-UT-N`) leak into `crates/` comments (C-23) — checked, the only `C-24`/`C-15` mentions are in `sandbox/Dockerfile` comments, which are not Rust source under `crates/` and read as prose rationale.
  - `rust/STYLE.md` — PASS — `cargo fmt --all -- --check` is clean (S-25), so S-1/2/4/5/6 are settled by rustfmt. Naming follows RFC 430 (S-7): `DockerEngine`, `SandboxSpec`, `host_uid_gid`, `LABEL_SLUG`. `extern "C"` ABI specified (S-33) on the `getuid`/`getgid` block. `as_str`/conversion prefixes (S-8) respected. Public types derive `Debug` (S-18). No `get_` getters (S-9).
  - `rust/ERRORS.md` — PASS — 11 new `Error` variants, all E-9 compliant (lowercase first word, no trailing punctuation, no `error:` prefix), confirmed in `error.rs:550-638`. E-15: `DockerSpawn` carries an `op: &'static str` context field with `#[source] source: io::Error`; `SandboxConfigCorrupt` carries `path: PathBuf` with `#[source] source: toml::de::Error` — both name the resource/operation, neither uses bare `#[from]`. E-12: every variant carries typed context fields, no concatenated `Display` strings. E-7: zero `unwrap()`/`expect()`/`panic!`/`todo!` in non-test sandbox or docker code (scanned). E-1/E-10: validation at boundaries (`SandboxConfig::validate`, `select_engine`, `resolve_focus_slug`).

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`. Empty when none.

- [x] specs/features/worktree/SPEC.md: PASS — sandbox reuses the worktree, never creates one. `resolve::resolve_task` discovers the target via `find_worktree_for_slug` + `WorktreeConfig::resolve_worktrees_dir` (resolve.rs:39-53), read-only; `derive_git_mounts` is read-only. No worktree creation/teardown path exists (NG-1 honored). One downstream interaction gap noted, see V-004.
- [x] specs/features/task-concurrency-control/SPEC.md: PASS — `--slug` is optional and defaults to `state.focus` through the new `resolve_focus_slug` (`state/checkout/io.rs:113`), which raises `Error::NoFocus { project_root, candidates }` exactly as SPEC C-23 requires. No new state-file fields added (`state/checkout/model.rs` unchanged by the diff).
- [x] specs/features/ark-agent-namespace/SPEC.md: PASS — `Sandbox` is a top-level, semver-covered `ark sandbox` command (`main.rs:70`), NOT under the hidden `Agent` namespace, matching the PRD/PLAN TR-1 decision. The namespace's stability contract is untouched.
- [x] specs/features/codex-support/SPEC.md: PASS — `enter --agent` couples to the platform registry only in `platform_argv.rs`: `claude → --dangerously-skip-permissions`, `codex → --yolo`, opencode → `AgentYoloUnsupported`. Platform selection uses `platforms::installed(&manifest)` in `PLATFORMS` order; the default shell path stays platform-agnostic.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [x] `create` resolves the focused/named worktree, starts a detached box (worktree rw at `/workspace`, git common dir mounted, config volume, `ANTHROPIC_API_KEY` passthrough): PASS — `create.rs` + `build_run_args` (docker.rs:200-243) build exactly this argv; `env_passthrough` defaults to `["ANTHROPIC_API_KEY"]` (config.rs:73).
- [x] `enter [--agent]` runs `docker exec -it`; bash by default, agent yolo CLI with `--agent`: PASS — `enter.rs:28-35` + `DockerEngine::enter` (docker.rs:126-130) via `exec_interactive(["exec","-it",...])`.
- [x] `rm [--keep-volume]` stops + removes the container, volume preserved unless dropped: PASS — `rm.rs` + `DockerEngine::remove` (docker.rs:132-148); volume removed only when `!keep_volume`.
- [~] `list` prints one row per running Ark sandbox (slug, branch, container id, status); empty when none: PARTIAL/FAIL — empty-when-none holds, but the `slug` column is populated with the branch value, not the task slug (the `ark.sandbox.slug` label is never read). See V-001 (HIGH).
- [x] Subscription login persists in the named volume across recreate: PASS — config volume `<container>-cfg` mounts at `/root/.claude` (docker.rs:25,229); `rm` defaults to keeping it; `create --recreate` removes the old container with `keep_volume: true` (create.rs:32), preserving the token.
- [x] Requires `docker`; clear error when absent; all container ops route through `io/docker.rs`; no `Command::new` leaks into `commands/`: PASS — `Error::SandboxBackendUnavailable` surfaced by `is_available`; `commands_no_bare_command_new` guard extended with all 12 sandbox sources and passes. Note PRD names `Error::DockerUnavailable`; the shipped variant is `SandboxBackendUnavailable` (engine-agnostic) — a naming refinement, not a gap (`DockerSpawn` covers the binary-missing spawn case). See V-005 (LOW).
- [x] Existing flows unchanged; `unload`/`load`/`upgrade` ignore sandbox state: PASS — sandbox writes nothing under `.ark/` (all state is engine-side); the snapshot/unload/upgrade code paths are untouched by the diff. No `.ark/` footprint to capture (C-23).

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`). PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).

- [x] G-1: `ark sandbox create` confines a task's worktree in a container at `/workspace`: PASS — `build_run_args` mounts the worktree rw at `/workspace` and sets `-w /workspace` (docker.rs:217-218, 237-238); covered by `run_args_mounts` (V-UT-7).
- [x] G-2: `ark sandbox enter` opens a shell in the box, or the agent CLI with `--agent`: PASS — `enter.rs` + `platform_argv`; CLI parse covered by `cli_sandbox_subcommands_parse` (V-IT-2), yolo map by `yolo_argv_per_platform` (V-UT-9).
- [x] G-3: `ark sandbox rm` tears down the sandbox, preserving the named volume by default: PASS — `rm.rs` + `DockerEngine::remove`; `keep_volume` defaults true at the CLI (`--keep-volume` opt-in). Idempotent-remove and volume-in-use legs are docker-live and untested (see V-006).
- [~] G-4: `ark sandbox list` enumerates running Ark sandboxes, one row each: PARTIAL — enumeration and label filtering work, but the slug column is wrong (see V-001, HIGH). FAIL on the slug field; PASS on filtering/sorting/empty-set shape.
- [x] G-5: `ark sandbox` persists a one-time in-box login across container recreate: PASS — named config volume kept across `rm` and `create --recreate`; see PRD login criterion above.

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — this task modifies no feature SPEC. `git diff --stat` touches only `crates/`, `templates/ark/config.toml`, and adds `crates/ark-core/src/commands/sandbox/`, `io/docker.rs`, `sandbox/Dockerfile`. The deep-tier feature SPEC for `ark-sandbox` is extracted from `02_PLAN.md` at commit time, so there is no pre-existing SPEC to carry a CHANGELOG entry.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

## Severity Summary: 0 CRITICAL · 3 HIGH · 2 MEDIUM · 5 LOW — all resolved (8 FIXED, 2 ACCEPTED with out-of-scope follow-up) by the main session post-verify. V-008/V-009 were caught by live testing against a real Docker daemon; V-010 (config.toml section-merge) is a structural gap in the upgrade feature scoped out of this task to project memory.
## Verification: build PASS · tests PASS (594 passed/0 failed after fixes) · lint PASS · format PASS · live lifecycle PASS (create → list → enter shell → rm against Colima, with both stand-in `node:24-slim` and the canonical gitdir-rewrite verified).

### V-001 `ark sandbox list` reports the branch in the slug column

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/sandbox/engines/docker.rs:27` (`PS_FORMAT`), `:175-192` (`parse_ps_row`)
- **Problem:** `PS_FORMAT` requests only `{{.Names}}`, `{{.Status}}`, and the `ark.sandbox.branch` label — it never requests the `ark.sandbox.slug` label. `parse_ps_row` then sets `slug: branch.to_string()`, so the `SandboxRow.slug` field carries the branch, not the task slug. Whenever slug ≠ branch (the common case — slug `ark-sandbox` lives on branch `feat/ark-sandbox`), `ark sandbox list` prints the branch where it promises the slug. The function's own doc-comment concedes the slug is "the branch when unavailable," but the slug label *is* set at create time (docker.rs:207, `LABEL_SLUG`), so it is available and simply not fetched.
- **Why it matters:** G-4 and the PRD outcome both specify the slug as a list column; C-17 names both labels as load-bearing. Operators read `list` to map a running box back to its task; showing the branch breaks that mapping and contradicts the documented row shape. The slug label is written but never read, so the create-side work is wasted.
- **Recommendation:** Add `{{.Label "ark.sandbox.slug"}}` to `PS_FORMAT` and parse it as the first field; fall back to the branch only when the label is genuinely absent. Add a `parse_ps_row` unit test asserting slug and branch are distinct for a `feat/x`-style row (this also lands the missing V-E-3/list coverage).
- **Resolution:** FIXED — `PS_FORMAT` now emits `slug<TAB>branch<TAB>status` from both labels (docker.rs:26-28); `parse_ps_row` reads the slug label as field 1, falling back to the branch only when the branch label is absent. `parse_ps_row_reads_slug_label` asserts slug ≠ branch for a `feat/ark-sandbox` row and that a blank-slug row is dropped.

### V-002 Broken doc-comment on `parse_ps_row`

- **Severity:** MEDIUM
- **Location:** `crates/ark-core/src/commands/sandbox/engines/docker.rs:168-174`
- **Problem:** The `///` doc on `parse_ps_row` is ungrammatical and self-contradictory: "The slug is recovered from the container name suffixing scheme is not reversible, so we read it back from the slug label via a second pass in the caller" — there is no second pass in any caller, and the sentence has no coherent subject/verb. The inline `//` block at :183-186 repeats the same confusion ("surface it via the name's sanitized branch is lossy"). This violates C-2/C-3 (first sentence must be a clean third-person summary) and C-7/C-10 (comments state the contract concisely, not a stream of caveats).
- **Why it matters:** This doc-comment promotes into rustdoc and is the artifact a future reader trusts to understand list behavior; as written it describes a mechanism (a caller second pass) that does not exist and obscures the actual V-001 bug. It is also the comment that *should* have flagged that the slug is not being read.
- **Recommendation:** Rewrite to one accurate sentence describing what the function parses, and (once V-001 is fixed) state plainly that the slug comes from the slug label. Drop the dead "second pass in the caller" claim.
- **Resolution:** FIXED — `parse_ps_row`'s doc now reads as one accurate sentence ("The line is `slug<TAB>branch<TAB>status`, slug and branch taken from the `ark.sandbox.{slug,branch}` labels…"); the dead "second pass in the caller" claim and the broken inline block are removed (docker.rs).

### V-003 `select_platform` override path (`--platform`) has no test

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/sandbox/platform_argv.rs:38-48`, tests at `:50-78`
- **Problem:** V-UT-9 in `02_PLAN.md` maps C-25 to "`select_platform` picks first-in-order and honors `--platform`." The shipped tests cover `yolo_argv` per platform and the `NoAgentPlatform` (none installed) case, but neither the first-installed-in-order selection nor the `--platform <id>` override branch (`installed.find(|p| p.id == id || p.cli_flag == id)`) is exercised. C-25's selection rule is therefore asserted only at the CLI-parse level (`cli_sandbox_platform_requires_agent`), not at the resolver level.
- **Why it matters:** The override-and-order logic is the substance of C-25; a regression in the `find`/`next` branch (e.g. matching `id` vs `cli_flag`) would pass the suite silently. Load-bearing but low blast radius (one consumer).
- **Recommendation:** Add a unit test seeding a manifest with two installed platforms, asserting (a) `None` picks the first in `PLATFORMS` order and (b) `Some("codex")` overrides to codex when installed.
- **Resolution:** FIXED — `select_first_in_order_and_override` (platform_argv.rs) writes a manifest with claude + codex installed and asserts `None` → `claude-code` (first in `PLATFORMS` order) and `Some("codex")` → `codex`.

### V-004 `enter`/`rm` fail with `WorktreeNotFound` after the worktree is cleaned up, orphaning the container

- **Severity:** LOW
- **Location:** `crates/ark-core/src/commands/sandbox/enter.rs:25`, `rm.rs:24` (both call `resolve::resolve_task`)
- **Problem:** `enter` and `rm` resolve the live container by re-deriving names from `resolve_task`, which requires the worktree to still exist on disk and to carry a branch (`resolve.rs:43-47`, raising `WorktreeNotFound`). The PRD states worktree teardown "stays the separate `ark cleanup` step." If a user runs `ark cleanup` (or `task worktree cleanup`) before `ark sandbox rm`, the worktree is gone, `resolve_task` raises `WorktreeNotFound`, and the still-running container can no longer be removed via `ark sandbox rm` — it must be removed by hand with raw `docker`. `enter` has the same dependency.
- **Why it matters:** It is a usable-but-fragile ordering coupling between two features the PRD deliberately keeps separate. It does not corrupt data, but it can strand a container (and its volume) outside Ark's reach. Likely acceptable for v1, but worth recording so the ordering contract is explicit.
- **Recommendation:** Either document the required teardown order (`sandbox rm` before `cleanup`) in the command help / config comment, or have `rm`/`enter` fall back to label-based handle resolution (`docker ps --filter label=ark.sandbox.slug=<slug>`) so they work without the worktree. The label-based path also fixes the dependency on `resolve_task` for the teardown verb.
- **Resolution:** FIXED (label-based fallback, maintainer-chosen) — new `SandboxEngine::resolve_handle_by_slug` resolves the container from the `ark.sandbox.slug` label alone (docker.rs); `resolve::resolve_handle_for` tries the worktree first and falls back to the label path on `WorktreeNotFound`. `rm`/`enter` now use it, so teardown works after `ark cleanup`. Covered docker-less by `resolve_handle_for_falls_back_when_worktree_gone` (stub engine).

### V-005 Shipped unavailable-error variant differs from the PRD-named `Error::DockerUnavailable`

- **Severity:** LOW
- **Location:** `crates/ark-core/src/error.rs:561-566` (`SandboxBackendUnavailable`); PRD `[**Outcome**]` line 20
- **Problem:** The PRD promises "a clear `Error::DockerUnavailable` when [docker is] absent." The implementation ships `Error::SandboxBackendUnavailable { engine }` (engine-agnostic, the better name given the `SandboxEngine` trait) plus `Error::DockerSpawn { op, source }` for the binary-missing spawn case. The behavior the PRD wanted is present; the variant name differs. `02_PLAN.md`'s Data Structure already names `SandboxBackendUnavailable`, so the PLAN and code agree — only the PRD's prose is stale.
- **Why it matters:** Purely a naming-traceability note. No functional gap; the daemon-down and binary-missing cases are both covered (`is_available` → `SandboxBackendUnavailable`, spawn failure → `DockerSpawn`).
- **Recommendation:** No code change. Accept the PLAN's name; optionally note the rename in the extracted SPEC so the PRD↔SPEC trace is clean.
- **Resolution:** ACCEPTED — no code change. The behavior the PRD wanted (clear error when docker is unavailable) ships as `SandboxBackendUnavailable` (daemon down) + `DockerSpawn` (binary missing); the engine-agnostic name is the better one and matches the PLAN. The PRD's `DockerUnavailable` is stale prose, not a gap.

### V-006 Docker-live validations (V-F-1/2/3, V-E-2/3, V-IT-3, V-IT-4 commit leg) ship unexercised

- **Severity:** LOW
- **Location:** `02_PLAN.md` `[**Validation**]` / `[**Acceptance Mapping**]`; `crates/ark-core/src/commands/sandbox/{create,enter,rm,list}.rs` (no `#[cfg(test)]` modules)
- **Problem:** Several mapped validations have no executing test: **V-IT-3** ("every verb returns `SandboxBackendUnavailable` when docker is absent") — the four `sandbox_*` command functions have zero unit tests; only the engine-level `is_available_is_well_typed` exists. **V-F-1** (rollback `rm -f` on `docker run` failure), **V-F-2** (idempotent remove → `container_removed: false`), **V-F-3** (volume-in-use warns, still reports), **V-E-2** (`create` twice → `SandboxExists`; `--recreate` replaces), **V-E-3** (`list` empty → exit 0) are all docker-live and have no test, gated or otherwise. The PLAN marks only V-IT-4's commit leg as docker-host-gated; the others are mapped as if routinely validated. The derivation leg of V-IT-4 *is* tested (`derive_resolves_common_dir` builds a real worktree and asserts `objects/` and `worktrees/` nest under the common dir — V-IT-4 derivation, C-7/C-8 PASS).
- **Why it matters:** The reviewer's own R-001/R-005 line of concern (a host-gated validation must say so, not masquerade as a routine test). V-F-2 and V-E-3 in particular are pure-logic enough to unit-test against a stub engine without a daemon, and V-IT-3 can be tested by calling the verb functions in a docker-less env and asserting the error type — none of which was done. The argv-shape, naming, config, gitmounts-derivation, platform-argv, and focus-resolution layers ARE well covered (V-UT-1..8, V-UT-10, V-IT-1, V-IT-2, V-IT-4-derivation, V-E-1, V-E-4 all map to real, passing tests).
- **Why this is not HIGH:** the untested paths are thin orchestration over a daemon Ark does not control; the daemon-touching legs cannot run on docker-less CI, and the build/clippy/fmt/test gates are all green. The risk is regression-detection coverage, not a known defect.
- **Recommendation:** (a) Add docker-less unit tests for the four verb functions asserting `SandboxBackendUnavailable` (closes V-IT-3 honestly). (b) Add a stub-engine or pure-logic test for `RemoveOutcome` reporting (V-F-2) and the empty-`list` shape (V-E-3). (c) For the genuinely daemon-only legs (V-F-1/3, V-E-2), relabel their Acceptance Mapping rows as docker-host-gated, mirroring V-IT-4. No design change.
- **Resolution:** FIXED (relabel + added the cheap pure-logic tests) — added docker-less coverage: `parse_ps_row_reads_slug_label` (list row shape, V-E-3 parsing leg), `resolve_handle_for_falls_back_when_worktree_gone` (stub engine), `is_available_is_well_typed` (V-IT-3 dichotomy). The genuinely daemon-only legs (V-F-1/2/3, V-E-2, V-IT-3 live, V-IT-4 commit leg) are now relabeled **docker-host-gated** in `02_PLAN.md`'s Validation, mirroring V-IT-4. No design change; the gating is now visible at SPEC level.

### V-008 `resolve_task` failed when invoked from inside the worktree (live-test catch)

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/sandbox/resolve.rs` (`resolve_task_for_slug`)
- **Problem:** Live-tested `ark sandbox create` from inside `.ark/worktrees/feat/ark-sandbox/` and it errored `no worktree found for slug 'ark-sandbox'` — even though that *is* the worktree. Root cause: `find_worktree_for_slug` filters `git worktree list` entries to those under `<root>/.ark/worktrees`, but from inside a worktree `<root>` is the worktree itself, which has no nested `worktrees/` dir, so every entry is filtered out. The PRD's documented main flow ("run `ark sandbox create` from the worktree") was broken. No unit test covered it because the existing tests resolve from the main checkout.
- **Resolution:** FIXED — added `resolve_local_worktree` that checks the slug's local `task.toml` for a `worktree_path`; when present, the current root *is* the worktree (resolved directly, no `git worktree list` walk). `resolve_task_for_slug` tries local first, falls back to the parent-walk for main-checkout invocations. New C-6a in `02_PLAN.md` documents the dual resolution. Covered by `resolve_local_worktree_uses_current_root`. Verified live from the worktree: `create` now succeeds.

### V-009 `rewrite_gitdir` depended on `git` in-box and corrupted the host `.git` (live-test catch)

- **Severity:** HIGH
- **Location:** `crates/ark-core/src/commands/sandbox/engines/docker.rs` (`rewrite_gitdir`)
- **Problem:** Live-tested with `image = "node:24-slim"` (no `git` installed) and saw `/workspace/.git` rewritten to `gitdir: /Users/anekoique/Agent/Ark/.git/worktrees/` — truncated, missing the per-worktree subdir name. Root cause: the in-box `sed` script ran `$(basename $(git -C /workspace rev-parse --git-dir))` inside the container, but the image had no `git`, so the subshell yielded empty and the substituted path was incomplete. Worse: `sed -i` on `/workspace/.git` writes through the bind mount, so the **host worktree's `.git` file was also corrupted** — every host-side `git` call in the worktree would have failed. The Dockerfile happens to ship `git`, masking this on the canonical image, but any image variant without `git` (and the host-write hazard) made the design fragile.
- **Resolution:** FIXED — `read_worktree_gitdir_name` derives the per-worktree dirname **host-side** by parsing the host worktree's `.git` file (a one-line `gitdir:` pointer), and `rewrite_gitdir` uses `printf '%s' > /workspace/.git` inside the box (POSIX, no `git`/`sed` dependency). Pure literal substitution; no in-box subshell; no `sed -i` on a host-shared file. Covered by `read_worktree_gitdir_name_parses_dot_git_file`. Verified live: in-box `/workspace/.git` now reads `gitdir: /Users/.../worktrees/ark-sandbox` (correct), and the host `.git` file stays untouched across create/rm cycles.

### V-010 Existing users on upgrade don't receive the new `[sandbox]` config section (out-of-scope limitation)

- **Severity:** MEDIUM
- **Location:** `templates/ark/config.toml` (new `[sandbox]` block); `crates/ark-core/src/commands/upgrade/plan.rs::classify` (the file-level hash-classifier)
- **Problem:** This task adds a new `[sandbox]` section to the shipped `templates/ark/config.toml`. Users whose `.ark/config.toml` has any pre-existing edits (e.g. they changed `[worktree] branch_prefix`) will route through `Classification::UserModified` on `ark upgrade` — picking the safe answer ("skip") preserves their edits but means they **never see** the new `[sandbox]` block. The upgrade pipeline treats `config.toml` as opaque bytes, which is the wrong shape for a sectioned config file: editing `[worktree]` shouldn't lock out updates to `[sandbox]`. The `[upgrade] merged` strategy exists but uses diff3 against a sidecar, which produces textual conflict markers that break TOML parsing — brittle for this case.
- **Resolution:** ACCEPTED — out-of-scope structural change to the `upgrade` feature, **NOT** to `ark-sandbox`. Mixing it in would scope-creep this deep task into a second feature change post-approval. Captured as a separate future task in project memory (`project_upgrade_section_merge_pending`): a TOML-aware section-merge mode that adds missing top-level sections verbatim while never modifying existing ones. For users today, the manual workaround is to append the `[sandbox]` block from `templates/ark/config.toml` to their own `.ark/config.toml` (2-minute copy-paste); the SPEC and release notes should call this out explicitly so the limitation is honest.

### V-007 `02_PLAN.md` C-24 retains a trailing `so …` rationale (carried review note)

- **Severity:** LOW
- **Location:** `.ark/tasks/ark-sandbox/02_PLAN.md:330` (C-24)
- **Problem:** Carried from `02_REVIEW.md` non-blocking note R-002. C-24 reads "...with a writable config dir, **so volume-backed config writes succeed**" — the trailing `so …` rationale clause that R-004 trimmed from C-8/C-18 elsewhere. C-24 is the constraint text that promotes verbatim into the extracted feature SPEC, so the rubric's "single declarative sentence, rationale in Trade-offs" shape (the "why" already lives in TR-9) is not met. The reviewer also raised (R-003) that TR-9 does not state the rootless image-default expectation; the shipped `sandbox/Dockerfile` *does* address the substance (it documents the uid-tolerance contract and `ENV HOME=/tmp` + `chmod 0777`), and `host_user()` correctly gates `--user` on `is_rootful()` (docker.rs:57-59), so the C-22/C-24 interaction is implemented soundly — only the PLAN/SPEC prose carries the unaddressed wording note.
- **Why it matters:** Non-load-bearing prose hygiene on text that becomes the durable SPEC. The implementation is correct; this is about the promoted-SPEC sentence shape.
- **Recommendation:** When extracting the feature SPEC at commit, trim C-24 to "The published image runs as a user tolerating an arbitrary `--user` uid/gid with a writable config dir." and leave the rationale in TR-9. Optionally add the one-clause TR-9 note (R-003) that under rootless the image is expected to default to root.
- **Resolution:** FIXED — C-24 in `02_PLAN.md` is trimmed to the single declarative sentence (trailing `so volume-backed config writes succeed` removed); the rationale already lives in TR-9. The implementation (uid-tolerant `sandbox/Dockerfile` + `host_user()` gating on `is_rootful()`) was already correct per the verifier's own Notes.

## Notes

- All four project gates were run in this worktree and pass: `cargo build --workspace` (exit 0), `cargo test --workspace` (588 passed, 0 failed, 0 ignored; doc-tests 0), `cargo fmt --all -- --check` (exit 0), `cargo clippy --workspace --all-targets -- -D warnings` (exit 0, zero warnings). The "all four gates pass" claim is verified, not trusted.
- The subprocess invariant (C-1) is solid: `commands_no_bare_command_new` (context/mod.rs:514) lists all 12 sandbox sources and passes; no `Command::new` appears in non-test code under `commands/sandbox/`. The only `Command::new("docker")` sites are in `io/docker.rs` (legitimately under `io/`, not `commands/`), and the only `Command::new("git")` use in the sandbox subtree is inside the `gitmounts.rs` test fixture (test code, allowed).
- The git-mount model (C-7/C-8) is correct and is the original CRITICAL from iteration 01, now resolved: `derive_git_mounts` uses `git rev-parse --path-format=absolute --git-common-dir` (gitmounts.rs:25-28), `GitMounts` is the single `common_dir`, `build_run_args` mounts it rw when `mount_git` (docker.rs:221-225), and `derive_resolves_common_dir` exercises a real worktree asserting the common dir nests `objects/` and `worktrees/`.
- Config (C-11) is correct: `#[serde(deny_unknown_fields)]` sits on the inner `SandboxSection` (config.rs:37), never the outer `RawConfig` (config.rs:30, with an explicit comment); `foreign_section_does_not_error` proves a `[worktree]` section is tolerated while an unknown key *inside* `[sandbox]` is rejected.
- Naming collision-resistance (C-18/C-19) is correct: `hash8` is the first 8 hex of SHA-256 of the exact branch (naming.rs:58-65); `collision_resistant` proves `feat/x` and `feat-x` get distinct container names.
- Focus resolution (C-5) is correct and is the new shared `ark-core` resolver, not a copy of `ark-cli`'s private `resolve_slug`: `resolve_focus_slug` lives in `state/checkout/io.rs:113`, is re-exported through `state/checkout/mod.rs:22`, raises `NoFocus`, and is consumed by `resolve::resolve_task`. NOTE: the PLAN's Architecture/API-Surface says lib.rs also re-exports `resolve_focus_slug` at the crate root; it is NOT in the `lib.rs` `state::{…}` re-export group (lib.rs:60-63). This is a minor, harmless plan-fidelity slip (the function is fully reachable via `state::checkout::resolve_focus_slug` and the sandbox feature uses it correctly); folded here rather than as a separate Finding because nothing depends on the crate-root path. If the SPEC's API Surface is meant to be exact, add it to the `lib.rs` re-export.
- Conditional `--user` (C-22) is correct: `DockerEngine::host_user()` returns `host_uid_gid().filter(|_| docker::is_rootful())` (docker.rs:57-59), so `Some("uid:gid")` only on a rootful Unix daemon and `None` on rootless / non-Unix; `is_rootful` parses `docker info` SecurityOptions for a `rootless` entry and conservatively returns false on probe failure. The `getuid`/`getgid` FFI carries a `// SAFETY:` comment (docker.rs:305-306) and specifies the `extern "C"` ABI (S-33).
- C-13 (pull never build): `create` calls `docker pull` then `docker run` (docker.rs:78-88); `sandbox/Dockerfile` exists as the CI build source and is NOT embedded via `include_dir!` (confirmed — `templates.rs` is untouched by the diff and the file lives under top-level `sandbox/`, not `templates/`).
- C-20 (single Display summary per verb): all four summaries `impl Display` (mod.rs:130-177); the CLI calls one `render(summary)` per dispatch (main.rs:749-785), no ad-hoc stdout in command bodies.
- C-10 "warns rather than fails" on volume-in-use: `remove` reports `volume_removed: false` and does NOT error when `docker volume rm` is non-zero (docker.rs:137-142). There is no emitted warning string — the captured stderr is held in `DockerOutput.stderr` but `#[allow(dead_code)]` and unused. The hard requirement (do not fail) is met; the literal "warn" is only reflected as the "volume kept" summary line. Acceptable as-is; a future improvement could surface the stderr.
