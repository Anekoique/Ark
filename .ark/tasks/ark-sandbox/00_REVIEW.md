# `ark-sandbox` REVIEW `00`

> Status: Closed
> Feature: `ark-sandbox`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Rejected
- Blocking: `1`
- Non-blocking: `9`

## Summary

The PLAN is structurally strong: the `## Spec` is self-contained (no "see iteration N"), Goals are verb-led, the `SandboxEngine` trait surface matches research §D.1, the `io/docker.rs` subprocess invariant (C-1) is correctly stated, and `exec_interactive` faithfully mirrors `io/git.rs::run_shell`. The maintainer's seven decisions are executed honestly, including the open-network limitation (NG-3/C-15) which V-UT-4 does assert. It is rejected on one design-breaking issue: the git-access model (C-7 + Main Flow step 4) mounts the parent `.git` **read-only** yet claims in-box commits to the worktree index — but a linked worktree's index, HEAD, logs, and the shared object store all live under that ro mount, so `git commit` inside the cage cannot work as drawn. Compounding it, the `repo_git_dir` derivation is unspecified for the from-worktree invocation where `root` is the worktree (whose `.git` is a *file*, not the main repo's `.git` dir). Two research-flagged risks (branch-name collision, rootless uid/gid ownership) are dropped without a Non-goal, and several Goals/Constraints lack a validation that exercises the claim.

---

## Findings

### R-001 `Read-only .git mount contradicts in-box commit; repo_git_dir derivation unspecified for worktree invocation`

- **Severity:** CRITICAL
- **Section:** Constraints C-7/C-8, Architecture (create call graph, `build_spec`/`rewrite_gitdir`), Runtime Main Flow step 4
- **Problem:** A linked worktree's `.git` is a *file* containing `gitdir: <repo>/.git/worktrees/<branch>`. That per-worktree dir holds the worktree's own `HEAD`, `index`, `logs/`, `ORIG_HEAD`, `COMMIT_EDITMSG`, and `commondir`; the loose/packed objects live in `<repo>/.git/objects`. C-7 mounts the parent `.git` **ro**. Main Flow step 4 then states "Git reads history through the ro `.git` mount and commits to the worktree's index." A commit must write the worktree index, HEAD/logs, and new objects — all under the ro mount — so `git add`/`git commit` inside the box fails (`error: opening '.git/...': Read-only file system`). The two statements are mutually exclusive. Separately, the create call graph passes `root` into `build_spec` and labels `repo_git_dir = host <repo>/.git`, but when `ark sandbox create` is run from inside the worktree (the documented Main Flow step 1), `resolve_with_discovery()`/`Layout::discover_from` resolves `root` to the **worktree** root, whose `.git` is a file, not the main repo's `.git` directory. `Layout` has no `git_dir()`/common-dir accessor (confirmed: `layout.rs` knows only `.ark/` roots), so the mount source is both mis-derivable and unspecified.
- **Why it matters:** The central use case — an unsupervised yolo agent doing work *and committing* inside the cage (G-1, Main Flow) — is unimplementable with the drafted mount model. Shipping it yields a box where the agent can edit files but cannot commit, or (if "rw" is silently substituted) hands the agent write access to the parent repo's entire object store and other worktrees' refs, defeating the cage.
- **Recommendation:** Resolve the contradiction explicitly. Either (a) scope in-box git to **read-only history** (matching PRD line 15's "resolve history" wording) and move all commits to the host (PRD step 5 already allows "commits from host"), updating Main Flow step 4 and dropping the "commits to the worktree's index" claim; or (b) mount the per-worktree gitdir `<repo>/.git/worktrees/<branch>/` **rw** and `<repo>/.git/objects` **rw** (or the whole `.git` rw) and document the widened blast radius as a trade-off. In both cases specify how `repo_git_dir`/common-dir is derived — via a `git rev-parse --git-common-dir` (or `--path-format=absolute --git-common-dir`) query through `io::docker`/`io::git` rather than `root.join(".git")` — and add a Validation entry for the gitdir-rewrite + a chosen-mode in-box `git status`/`git commit` assertion (currently C-7's hardest mechanism has no test; V-UT-4 only toggles mount presence).

### R-002 `C-18 asserts branch-name sanitization is injective; it is not (research risk 7)`

- **Severity:** HIGH
- **Section:** Constraints C-18; Validation V-UT-3
- **Problem:** C-18 derives names by mapping non-`[A-Za-z0-9_.-]` → `-` and claims "the derivation is injective enough that distinct branches cannot collide on a name." That mapping is not injective: `feat/x` and `feat-x` both sanitize to `ark-sandbox-feat-x`. Research §D.5 risk 7 flagged exactly this. V-UT-3 then promises to test "distinct branches never collide on a name," which is unsatisfiable for the colliding pair under the stated derivation.
- **Why it matters:** A constraint stating a false invariant will either ship a test that cannot pass or (worse) a passing test that does not actually exercise the collision, leaving two tasks silently sharing one container/volume — cross-task contamination inside the cage.
- **Recommendation:** Make the derivation collision-resistant (e.g. append a short hash of the exact branch string: `ark-sandbox-<sanitized>-<hash8>`), then C-18/V-UT-3 become true and testable. Alternatively, drop the injectivity claim, accept collisions, and document the limitation — but that weakens isolation and is the weaker choice.

### R-003 `Rootless-Docker uid/gid ownership of worktree writes dropped without a Non-goal (research risk 6)`

- **Severity:** HIGH
- **Section:** Non-goals / Constraints / Failure Flow (absent)
- **Problem:** Research §D.5 risk 6 (MEDIUM) warns that worktree files written from inside a rootless container land as uid 0 unless uid/gid mapping is handled. The PLAN does not address it anywhere — no NG, no Constraint, no Failure Flow entry, no Trade-off. The Main Flow assumes the agent edits `/workspace` (the host worktree, mounted rw) and that the host user can subsequently operate on it.
- **Why it matters:** On a rootless-Docker host (a supported and common Linux setup, and the default the PLAN's C-3 `docker info` probe will happily pass), agent edits become root-owned on the host worktree, breaking subsequent host-side `git`/`ark cleanup` and silently corrupting file ownership of user data. Discovering this post-merge is rework.
- **Recommendation:** Either handle it (pass `--user $(id -u):$(id -g)` and ensure the image tolerates an arbitrary uid, noting agent-infra's `resolveBuildUid`) and add a Constraint + a `build_run_args` test asserting the uid flag, or scope it OUT with an explicit Non-goal naming rootless-Docker uid mapping and a documented warning, mirroring how NG-3 honestly scopes network out.

### R-004 `Slug→focus resolution is mis-attributed to ark cleanup and its wiring is unspecified`

- **Severity:** MEDIUM
- **Section:** Constraints C-5; Summary; Architecture (resolve / create call graph); API Surface (CLI shape)
- **Problem:** C-5 and the Summary state `--slug` absent resolves `state.focus` → `Error::NoFocus`, "mirroring `ark cleanup` and the concurrency SPEC." But `ark cleanup`'s `--slug` is a *filter* over a multi-worktree sweep (`cleanup.rs`: `with_slug` sets an `Option` filter; absent slug sweeps all prunable worktrees and never raises `NoFocus`). The actual focus-defaulting precedent is the `ark agent task` downstream verbs via `agent_cli.rs::resolve_slug` (reads `state.focus`, raises `NoFocus`) — and that helper is `private` to `ark-cli::agent_cli`, so a *top-level* `ark sandbox` cannot reuse it. The create call graph calls `find_worktree_for_slug(root, worktrees_dir, slug)`, which takes a **required `&str`**, not `Option`; the step that turns `Option<String>` into a focus-resolved slug is missing from `resolve.rs`.
- **Why it matters:** The intended contract (focus default + `NoFocus`) is correct per the concurrency SPEC (C-23/C-14), but the wrong precedent and the missing resolution step risk an executor copying cleanup's never-error filter semantics, or duplicating focus logic ad hoc. C-5 is the concurrency-parity contract and must be unambiguous.
- **Recommendation:** Drop the `ark cleanup` comparison; cite the concurrency SPEC's `load_state` + `Error::NoFocus` contract directly. Specify a focus-resolution step in `resolve.rs` (load the checkout state, default to `state.focus`, else `Error::NoFocus { project_root, candidates }`) ahead of `find_worktree_for_slug`, and add a re-usable resolver in `ark-core` (or thread the resolved slug from the CLI) since the existing `resolve_slug` lives in `ark-cli` and is not callable from a top-level command.

### R-005 `--agent yolo argv: ambiguous platform selection and unverified opencode yolo flag`

- **Severity:** MEDIUM
- **Section:** Constraints C-17; Architecture (enter call graph: `platform_yolo_argv(detect_platform(layout))`); Trade-offs TR-7
- **Problem:** TR-7's local `cli_flag`→argv map is viable (`Platform` has `cli_flag` and no yolo field — confirmed), but two gaps remain. (1) `detect_platform(layout)` assumes a single platform; with `claude-code` + `codex` + `opencode` all installable (and the registry now 3 entries), selection is ambiguous and unspecified. (2) The map must cover every installed platform's yolo flag. Research verified only `claude --dangerously-skip-permissions` and `codex --yolo`; the **opencode** yolo argv is not established in the research or the opencode-support SPEC, so C-17's "maps the installed platform's `cli_flag` to its yolo argv" has an undefined entry.
- **Why it matters:** `enter --agent` either picks the wrong CLI in a multi-platform checkout or has no defined argv for opencode, failing the user-facing flag silently or with a confusing error.
- **Recommendation:** Specify the selection rule when multiple platforms are installed (e.g. error with a `--platform` disambiguator, or pick the first installed per `PLATFORMS` order and document it). Enumerate the full `cli_flag`→yolo-argv map in C-17 (claude, codex) and either supply a verified opencode flag or exclude opencode from `--agent` with an explicit `NoAgentPlatform`-style error for it.

### R-006 `Goal G-1 and Constraint C-7 lack a validation that exercises the /workspace mount and gitdir rewrite`

- **Severity:** MEDIUM
- **Section:** Validation (Acceptance Mapping); Goals G-1; Constraints C-7
- **Problem:** Every G/C has a mapping row, but two are non-validating. G-1 ("create confines the worktree at `/workspace`") maps to V-IT-2 (clap parse), V-UT-4 (run args), V-F-1 (rollback) — none asserts the `/workspace` rw mount target. C-7 maps only to V-UT-4, whose description covers the git-mount toggle, env passthrough, and absence of `--network`, but not the `/workspace` rw target nor the `rewrite_gitdir` exec step (the riskiest mechanism, per R-001).
- **Why it matters:** A SPEC promoted with mappings that do not test the claim gives false assurance; the two highest-risk behaviors (workspace mount target, gitdir rewrite) would ship untested.
- **Recommendation:** Extend V-UT-4 (or add a V-UT) to assert `build_run_args` includes `-v <guest-workspace-path>:/workspace` rw and the correct `.git` mount mode chosen in R-001, and add a unit test over the gitdir-rewrite argv (the `docker exec` command and the rewritten `gitdir:` target string).

### R-007 `Config: deny_unknown_fields placement contradicts the cited upgrade precedent`

- **Severity:** MEDIUM
- **Section:** Data Structure (`SandboxConfig` RawConfig comment); Constraints C-11
- **Problem:** C-11 and the Data Structure comment say "`[sandbox]` parses via a private `RawConfig` … with `#[serde(deny_unknown_fields)]`" on `RawConfig { sandbox: Option<SandboxSection> }`. `.ark/config.toml` is a single shared file with `[worktree]`, `[workspace]`, `[upgrade]`, and now `[sandbox]` sections. Applying `deny_unknown_fields` to the *outer* per-feature `RawConfig` (which declares only `sandbox`) makes parsing reject every other feature's section → corrupt error on every real project. The cited precedent (`upgrade/strategy.rs`) places `deny_unknown_fields` on the **inner** `UpgradeSection`, with the outer `RawConfig { upgrade: Option<...> }` carrying no such attribute (verified). The PLAN's wording places it on the outer struct.
- **Why it matters:** As literally written the config loader breaks the moment any other section is present (i.e. always); the V-UT-1 "`deny_unknown_fields` rejection" test would also be testing the wrong struct.
- **Recommendation:** Restate C-11/Data Structure to match the precedent: outer `RawConfig { sandbox: Option<SandboxSection> }` with no `deny_unknown_fields`; `#[serde(deny_unknown_fields)]` on the inner `SandboxSection` only. Point V-UT-1 at rejecting an unknown key *inside* `[sandbox]`.

### R-008 `Published-image availability depends on a per-release CI job; single-point failure under-documented`

- **Severity:** LOW
- **Section:** Constraints C-13/C-14; Architecture (`sandbox/Dockerfile` + CI publish job); Failure Flow
- **Problem:** C-14 pins `cfg.image` to `ghcr.io/anekoique/ark-sandbox:<CARGO_PKG_VERSION>`, so every released crate version requires a matching pushed tag. If the CI publish job is skipped/fails for a release, every `ark sandbox create` on that version fails `ImagePullFailed`. Failure Flow item 4 covers offline/bad-tag generically but does not call out the release-coupling invariant.
- **Why it matters:** A missed publish silently bricks the feature for an entire release with only a pull error to debug.
- **Recommendation:** Note the invariant (a release must not ship without its image tag) in C-14 or the Implementation Phase 3 CI step, and consider a `:latest`/major fallback or a clearer `ImagePullFailed` message naming the expected tag.

### R-009 `New error variants omit Display templates and #[source]/context detail`

- **Severity:** LOW
- **Section:** Data Structure (error.rs additions)
- **Problem:** The new variants list shapes only. `Error::DockerSpawn { source: std::io::Error }` carries no context field; while it mirrors the existing `Error::GitSpawn { source }`, ERRORS.md E-15 prefers a context field (e.g. the docker subcommand) on foreign-error wrappers, and EX-3's name-supplies-context carve-out is for single-purpose enums, not the canonical multi-domain `Error`. No `#[error("…")]` strings are given, so E-9 (lowercase, no trailing punctuation, no `error:` prefix) and `#[source]` placement (E-6) are unverified at PLAN level.
- **Why it matters:** Minor; these are impl details, but `DockerSpawn` with zero context yields "No such file or directory"-class messages, the exact E-15 anti-pattern.
- **Recommendation:** Optionally add a `command`/`op` field to `DockerSpawn` (or accept the `GitSpawn` precedent and note it). Confirm each new variant gets an E-9-compliant `#[error]` and `#[source]` on the wrapped `toml::de::Error` per E-6/E-15.

### R-010 `G-5 phrased as mechanism; minor Spec-discipline polish on constraints carrying rationale`

- **Severity:** LOW
- **Section:** Goals G-5; Constraints C-12, C-14
- **Problem:** G-5 ("A persistent named volume lets a one-time in-box login survive container recreate") leads with the mechanism (the volume) rather than the capability; the capability is "in-box login survives recreate." Per Spec discipline a Goal is a verb-led capability and the mechanism belongs in Architecture/Trade-offs. Minor rationale leakage also appears in C-12 ("the trait exists for v2, but only DockerEngine is registered") and C-14 ("so binary and image version together") — the *why* is meant to live in Trade-offs.
- **Why it matters:** This block is promoted verbatim into the feature SPEC; tightening now keeps the SPEC clean. Non-load-bearing.
- **Recommendation:** Reword G-5 to "`ark sandbox` persists a one-time in-box login across container recreate." Trim the parenthetical rationale from C-12/C-14 (the reasons already live in TR-5/TR-2).

---

## Trade-off Advice

### TR-1 `Git mount mode: read-only history vs read-write commit-in-box`

- **Related Plan Item:** C-7 / Main Flow step 4 (see R-001)
- **Topic:** Compatibility vs Clean Design (and blast-radius safety)
- **Reviewer Position:** Prefer A (read-only history; commit from host)
- **Advice:** Adopt the read-only-history reading: mount the parent `.git` ro purely so in-box `git log`/`git diff` resolve, and perform commits from the host. This matches PRD line 15 ("resolve history") and Codex's precedent of keeping `.git` read-only even in workspace-write.
- **Rationale:** Granting the box rw on the shared object store and the per-worktree refs widens the cage to the whole repository's git state — contrary to the "confine blast radius" goal — and is more moving parts than v1 needs. Read-only history with host-side commits is the smaller, safer surface and is already consistent with PRD step 5.
- **Required Action:** Adopt, and update Main Flow step 4 + add the gitdir-rewrite/`repo_git_dir`-derivation specification and validation per R-001. If the maintainer wants commit-in-box in v1, the alternative (rw per-worktree gitdir + objects) must be justified with its widened-blast-radius trade-off written out.

### TR-2 `SandboxEngine trait now (TR-5)`

- **Related Plan Item:** TR-5 / NG-4 (Unresolved item)
- **Topic:** Flexibility vs Simplicity
- **Reviewer Position:** Prefer B (keep the trait)
- **Advice:** Keep the `SandboxEngine` trait in v1. The surface (id/is_available/create/enter/remove/list) matches research §D.1, costs one `DockerEngine` struct, and removes the Docker name from every command signature.
- **Rationale:** The cost is near-zero and it de-risks the additive v2 native backend; the trait is correctly narrower than agent-infra's daemon/VM surface. The Unresolved flag can be closed.
- **Required Action:** Keep with clarification — close the TR-5 Unresolved item.

### TR-3 `Top-level ark sandbox vs hidden ark agent sandbox (TR-1)`

- **Related Plan Item:** TR-1 (Unresolved item)
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (top-level)
- **Advice:** Keep `ark sandbox` top-level, semver-covered, alongside `ark cleanup`/`ark archive`.
- **Rationale:** It is a user-facing command, not a workflow-structural mutation the slash commands drive; the `ark agent` namespace is reserved for internal phase/SPEC machinery per its SPEC G-1/NG-2. The CLI shape (TargetArgs + `resolve_with_discovery`) is the right precedent.
- **Required Action:** Keep; close the TR-1 Unresolved item. Note this is independent of R-004 (the slug→focus resolution still needs an `ark-core` path since the top-level command cannot reach `ark-cli`'s private `resolve_slug`).

### TR-4 `Yolo argv via local map vs Platform.yolo_flag field (TR-7)`

- **Related Plan Item:** TR-7
- **Topic:** Flexibility vs Simplicity
- **Reviewer Position:** Prefer A (local map) with clarification
- **Advice:** The local `cli_flag`→argv map in `enter.rs` is acceptable for a single consumer; do not add a `Platform` field yet (`Platform` is `#[non_exhaustive]`, so a field stays additive later).
- **Rationale:** One consumer does not justify growing the registry struct. But the map must be complete and the multi-platform selection deterministic — see R-005.
- **Required Action:** Keep with clarification — resolve R-005 (selection rule + verified per-platform yolo argv, opencode included or explicitly excluded).
