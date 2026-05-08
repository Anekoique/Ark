# `worktree-sync-defaults` PLAN `00`

> Status: Approved for Implementation
> Feature: `worktree-sync-defaults`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - PRD: `PRD.md`
> - Worktree SPEC: `.ark/specs/features/worktree/SPEC.md`
> - Workspace SPEC: `.ark/specs/features/workspace/SPEC.md`

---

## Summary

Add a dedicated **identity-sync** step to the `task new --worktree` create flow, reusing the existing `commands/agent/workspace/identity` module (`identity_resolve` / `identity_prompt` / `identity_write`). Step runs between `git worktree add` and `cfg.copy`, inside the existing rollback boundary. Add `git submodule update --init --recursive` to `WorktreeConfig::default().post_create`. No new schema, no `copy`-semantics change.

## Log

*None in 00_PLAN.*

---

## Spec

[**Goals**]

- **G-1:** `WorktreeConfig::default().post_create` defaults to `vec!["git submodule update --init --recursive".to_string()]`. The empty-vec default is replaced. Projects without `.gitmodules` are unaffected (git exits 0). Users can override by setting `post_create = []` (or any other value) in `.ark/config.toml`.

- **G-2:** `task new --worktree` performs an **identity-sync** step inside the rollback boundary, immediately after `scaffold_inside_worktree` writes `task.toml` and before the `cfg.copy` loop. Sequence:
  1. Read parent identity via `identity_resolve(ResolveOptions::new(project_root))`.
  2. If `Ok(id)` → call `identity_write(worktree_path, &id)` to mirror it; done.
  3. If `Err(MissingIdentity)`:
     - **TTY branch** (stdin is a terminal): call `identity_prompt(&mut stdin, &mut stderr, MAX_PROMPT_ATTEMPTS)`, then `identity_write(project_root, &id)` (mirror back to parent so subsequent worktrees skip the prompt), then `identity_write(worktree_path, &id)`.
     - **Non-TTY branch**: return `Error::MissingIdentity` unchanged. Existing message ("no developer identity set; run `ark init --developer <name>` ...") is sufficient — agents surface it as-is.
  4. Any other error from `identity_resolve` / `identity_write` propagates unchanged.
  Failure of any branch triggers `rollback_worktree` (already wired via `inspect_err` at the call site).

- **G-3:** TTY detection uses `std::io::IsTerminal` on `std::io::stdin()`. No new dependency. Constant `MAX_PROMPT_ATTEMPTS: u8 = 3` mirrors `ark init`'s existing prompt cap.

- **G-4:** `templates/ark/config.toml` `[worktree]` section is updated:
  - `post_create = []` line is removed; replaced by `post_create = ["git submodule update --init --recursive"]` (active, matching the new code default).
  - Comments updated: note that identity sync is automatic and that this `post_create` entry is the documented default — clearing it disables submodule init.

- **G-5:** Worktree SPEC additions:
  - New Goal **G-13** (identity sync) describing the step and its rollback semantics.
  - **G-1** amended: `post_create` default changes from `[]` to `["git submodule update --init --recursive"]`.
  - **NG-7** clarified, not removed: "No automatic submodule logic in Ark code; submodule init is achieved via the documented default `post_create` shell command, which the user can override."

[**Non-goals**]

- **NG-1:** No new config keys. Identity sync is unconditional; users who want to opt out can edit `.ark/.developer` directly inside the worktree (it's a plain text file).
- **NG-2:** No change to `cfg.copy` semantics. Source-missing remains a hard error (`WorktreeCopySourceMissing`) for explicit user entries.
- **NG-3:** No `--developer <name>` flag on `task new`. The existing `ark init --developer <name>` covers the bootstrap case once; `task new --worktree` only resolves and mirrors.
- **NG-4:** No new error variants. Reuses `Error::MissingIdentity` and `Error::DeveloperWriteFailed` from the workspace feature.
- **NG-5:** No retry/backoff on submodule init. Network failures inside `git submodule` surface as `PostCreateHookFailed` (existing behavior); the worktree rolls back. User retries `task new --worktree` after fixing connectivity.

[**Architecture**]

```
crates/ark-core/src/commands/agent/task/new.rs
└── scaffold_inside_worktree(...)
    ├── ensure task_dir, copy PRD template, save task.toml      (unchanged)
    ├── register_focus                                          (unchanged)
    ├── sync_identity(project_root, worktree_path)              ← NEW
    ├── for rel in &cfg.copy { ... }                            (unchanged)
    └── for cmd in &cfg.post_create { ... }                     (unchanged)

new fn sync_identity(parent: &Path, worktree: &Path) -> Result<()>
    delegates to commands::agent::workspace::identity::*
```

The identity module is already pub-exposed under `crate::commands::agent::workspace`. Add a `pub use` re-export at `commands/agent/workspace/mod.rs` if needed (most are likely already accessible to siblings under `commands/agent/`).

[**Data Structure**]

No new types. Existing structures used:

```rust
// commands/agent/workspace/identity.rs (already exists, unchanged)
pub struct Identity { name: String }
pub struct ResolveOptions<'a> { pub project_root: &'a Path }

pub fn identity_resolve(opts: ResolveOptions<'_>) -> Result<Identity>;
pub fn identity_prompt<R, W>(reader: &mut R, writer: &mut W, max_attempts: u8) -> Result<Identity>;
pub fn identity_write(project_root: &Path, identity: &Identity) -> Result<()>;
```

```rust
// commands/agent/task/worktree/config.rs (default tweak only)
fn default_post_create() -> Vec<String> {
    vec!["git submodule update --init --recursive".to_string()]
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorktreeConfig {
    // ...existing fields...
    #[serde(default = "default_post_create")]
    pub post_create: Vec<String>,
}
```

[**API Surface**]

```rust
// new in commands/agent/task/new.rs (private; called only from scaffold_inside_worktree)
fn sync_identity(parent: &Path, worktree: &Path) -> Result<()>;
```

No public API changes. CLI surface unchanged.

[**Constraints**]

- **C-1 (identity-sync placement):** `sync_identity` runs after `register_focus` and before the `cfg.copy` loop. Inside the rollback boundary established by `scaffold_inside_worktree`'s caller (`inspect_err` → `rollback_worktree`). Failure cleans up the worktree dir.
- **C-2 (TTY detection):** Use `std::io::IsTerminal::is_terminal(&std::io::stdin())`. No third-party crate. Tests inject a `bool` via a thin trait or `#[cfg(test)]` override — see V-UT-3.
- **C-3 (writer for prompt):** `identity_prompt` receives `&mut std::io::stderr()` as the writer (so the prompt text appears in agent transcripts as a tool-result side-channel, not stdout). Reader is `std::io::stdin().lock()` wrapped in `BufReader`.
- **C-4 (parent write before worktree write):** When the prompt path runs, write to **parent** first, then to worktree. If parent write fails, the worktree write is skipped and the error propagates. This avoids a state where the worktree has identity but the parent does not — which would re-prompt every subsequent `task new --worktree`.
- **C-5 (no copy-loop interference):** `.ark/.developer` MUST NOT also be copied via `cfg.copy`. If a user adds `.ark/.developer` to `cfg.copy`, the second write is harmless (same content) but redundant. Documented in the template comment (G-4); not enforced in code (no schema change).
- **C-6 (submodule default override):** `post_create` is now defaulted via `#[serde(default = "default_post_create")]`. Users who want no hooks set `post_create = []` explicitly in their `config.toml`. The empty `Vec::new()` `Default` impl path is gone for this field.
- **C-7 (template parity):** `templates/ark/config.toml`'s `[worktree].post_create` value must match `default_post_create()` byte-for-byte. Regression test reads both and asserts equality.
- **C-8 (test isolation for identity_prompt):** Existing tests in `identity.rs` mutate `USER`/`USERNAME` env vars via `unsafe`. New tests in `new_tests.rs` MUST NOT prompt — they configure parent identity up-front via `identity_write`, so the resolve branch is taken. The prompt branch is exercised only at the `identity` module level (already covered by existing `identity_prompt_*` tests).
- **C-9 (rollback on identity-sync failure):** Identity-sync errors propagate through `scaffold_inside_worktree`'s `Result`. The existing `inspect_err(|_| rollback_worktree(...))` chain at `task_new`'s call site cleans up the worktree dir and branch. No new rollback code.
- **C-10 (existing test rename / preservation):** `worktree_copy_missing_source_hard_fails_and_rolls_back` (new_tests.rs:220) is preserved verbatim — `cfg.copy` semantics unchanged. No test renames.

---

## Runtime

[**Main Flow** — parent has `.developer`, TTY irrelevant]

1. `task_new(opts)` validates and calls `task_new_worktree`.
2. `git worktree add -b <branch> <wt> <base>` succeeds.
3. `scaffold_inside_worktree`:
   1. ensure `<wt>/.ark/tasks/<slug>/`, copy PRD, write task.toml.
   2. register focus in `<wt>/.ark/.state.toml`.
   3. **`sync_identity(parent, wt)`**:
      - `identity_resolve(parent)` → `Ok(id)`.
      - `identity_write(wt, &id)` → writes `<wt>/.ark/.developer`.
   4. `cfg.copy` loop runs.
   5. `cfg.post_create` loop runs `git submodule update --init --recursive` in `<wt>` cwd. Repos without `.gitmodules` exit 0.
4. Return `TaskNewSummary`.

[**Main Flow** — parent missing `.developer`, TTY available]

3.3 `sync_identity(parent, wt)`:
- `identity_resolve(parent)` → `Err(MissingIdentity)`.
- `is_terminal(stdin)` → `true`.
- `identity_prompt(&mut stdin, &mut stderr, 3)` → `Ok(id)`.
- `identity_write(parent, &id)` → writes `<parent>/.ark/.developer`.
- `identity_write(wt, &id)` → writes `<wt>/.ark/.developer`.

[**Failure Flow** — parent missing `.developer`, non-TTY (CI / agent)]

3.3 `sync_identity(parent, wt)`:
- `identity_resolve(parent)` → `Err(MissingIdentity)`.
- `is_terminal(stdin)` → `false`.
- Return `Err(MissingIdentity)` unchanged.
- `scaffold_inside_worktree` returns `Err`.
- Caller `task_new`'s `inspect_err` runs `rollback_worktree(parent, wt, branch)` → `git worktree remove --force` + `git branch -D`.
- Error propagates to CLI; user sees existing message *"no developer identity set; run `ark init --developer <name>` or set [workspace] developer = ..."*.

[**Failure Flow** — `identity_write` fails on parent (e.g., read-only fs)]

3.3 `sync_identity(parent, wt)`:
- Prompt succeeds, `identity_write(parent, &id)` returns `Err(DeveloperWriteFailed)`.
- Worktree write is skipped.
- Caller rolls back the worktree dir via the same `inspect_err` chain.

[**Failure Flow** — `git submodule update` fails (network, auth, broken submodule)]

3.5 `cfg.post_create` loop:
- `run_shell("git submodule update --init --recursive", wt)` → exit ≠ 0.
- Return `Err(PostCreateHookFailed { command, exit_code })`.
- Existing rollback runs.

[**State Transitions**]

- `parent.has_developer = false` → `parent.has_developer = true` after the prompt path runs (idempotent for subsequent `task new --worktree`).
- `wt.has_developer = (always true)` after step 3.3 succeeds — invariant of every worktree created via this flow.

---

## Implementation

[**Phase 1 — submodule default + template parity**]

1. `crates/ark-core/src/commands/agent/task/worktree/config.rs`:
   - Add `fn default_post_create() -> Vec<String>` returning the submodule init command.
   - Annotate `post_create` field with `#[serde(default = "default_post_create")]`.
2. `templates/ark/config.toml`: replace `post_create = []` with `post_create = ["git submodule update --init --recursive"]`. Update preceding comment.
3. Add unit test `worktree_config_default_post_create_has_submodule_init` and template-parity test `worktree_template_default_matches_code` (reads embedded template via `include_str!` and parses it).

[**Phase 2 — identity-sync step**]

1. `crates/ark-core/src/commands/agent/task/new.rs`:
   - Add `use std::io::{BufReader, IsTerminal, stderr, stdin};`
   - Add `use crate::commands::agent::workspace::identity::{identity_resolve, identity_prompt, identity_write, ResolveOptions};` (verify the path; may need a `pub use` re-export in `commands/agent/workspace/mod.rs`).
   - Add `const MAX_PROMPT_ATTEMPTS: u8 = 3;` (or import the workspace module's existing constant if one exists).
   - Add `fn sync_identity(parent: &Path, worktree: &Path) -> Result<()>`:
     ```rust
     fn sync_identity(parent: &Path, worktree: &Path) -> Result<()> {
         match identity_resolve(ResolveOptions::new(parent)) {
             Ok(id) => identity_write(worktree, &id),
             Err(Error::MissingIdentity) if stdin().is_terminal() => {
                 let mut reader = BufReader::new(stdin().lock());
                 let mut writer = stderr().lock();
                 let id = identity_prompt(&mut reader, &mut writer, MAX_PROMPT_ATTEMPTS)?;
                 identity_write(parent, &id)?;
                 identity_write(worktree, &id)
             }
             Err(e) => Err(e),
         }
     }
     ```
   - Insert `sync_identity(project_root, worktree_path)?;` between `register_focus(...)?;` and the `for rel in &cfg.copy` loop.
2. Verify rollback chain: `scaffold_inside_worktree` already returns `Result` and the call site wraps it in `.inspect_err(|_| rollback_worktree(...))` — no change needed.

[**Phase 3 — tests + SPEC update**]

1. `crates/ark-core/src/commands/agent/task/new_tests.rs`:
   - `worktree_creation_mirrors_parent_identity` — write `.ark/.developer` in parent before `task_new`; assert worktree has matching `.ark/.developer` afterward.
   - `worktree_creation_fails_on_missing_identity_when_non_tty` — parent has no identity; with stdin redirected (Cursor over `/dev/null`-equivalent in test harness), assert `MissingIdentity` and worktree dir absent.
   - `worktree_post_create_default_runs_submodule_init` — initialize a parent repo with no `.gitmodules`; create worktree with default config (no override); assert `task_new` succeeds (proves the default command is a safe no-op).
   - All three tests `identity_write` parent identity *or* explicitly set `post_create = []` to keep submodule-init out of the way when irrelevant.
2. `.ark/specs/features/worktree/SPEC.md`: append G-13, amend G-1 default for `post_create`, refine NG-7 wording.

---

## Trade-offs

- **T-1: Identity sync as a code step vs. a `copy` entry.**
  - **Code step (chosen).** Adv: clean semantics; uses the existing identity module's validation and prompt; doesn't conflate "files the user lists" with "Ark-managed sync"; no schema change. Disadv: more code than a one-line config edit.
  - **`copy` entry with skip-missing.** Adv: minimal code. Disadv: changes documented `cfg.copy` behavior (source-missing → error → silent), invalidates `worktree_copy_missing_source_hard_fails_and_rolls_back`, breaks user expectation that explicit `copy` entries hard-fail.
  - **`optional_copy` schema.** Adv: keeps `copy` semantics. Disadv: invents new config key for a single use case; dilutes the schema.

- **T-2: TTY prompt vs. error-only.**
  - **Prompt + non-TTY error (chosen).** Adv: matches `ark init` UX; humans get the prompt, agents get a clear error pointing to `ark init --developer`. Disadv: requires `IsTerminal` plumbing, makes the function harder to test (TTY is process-global).
  - **Always error if missing.** Adv: simpler, fully deterministic, easier to test. Disadv: forces every human user to remember `ark init --developer` before their first `task new --worktree`, even though we have a prompt module sitting unused.

- **T-3: Submodule init default in code vs. only in template.**
  - **Both (chosen via `#[serde(default = "...")]` parity with the template).** Adv: users who delete or never had `config.toml` still get the safe default; template is the documentation of that default. Disadv: needs a parity test (C-7) to keep them in sync.
  - **Template only.** Adv: minimal code change. Disadv: `WorktreeConfig::default()` (used when `config.toml` is absent — see C-1 of worktree SPEC) silently disables submodule init, contradicting the template.

---

## Validation

[**Unit Tests**]

- **V-UT-1:** `worktree_config_default_post_create_has_submodule_init` — `WorktreeConfig::default().post_create` equals `vec!["git submodule update --init --recursive"]`.
- **V-UT-2:** `worktree_template_default_matches_code` — parse embedded `templates/ark/config.toml` via `toml::from_str`; assert the parsed `[worktree].post_create` equals `default_post_create()`.
- **V-UT-3:** `sync_identity_mirrors_when_parent_has_developer` — unit-level test on `sync_identity` with a fake `parent` containing `.ark/.developer`; assert worktree gets the same content. Does not exercise TTY branch.

[**Integration Tests**]

- **V-IT-1:** `worktree_creation_mirrors_parent_identity` — full `task_new` with parent identity present; assert worktree has `.ark/.developer` with matching content; assert task succeeds.
- **V-IT-2:** `worktree_creation_fails_on_missing_identity_when_non_tty` — parent has no `.developer`; in test process stdin is non-TTY; assert `MissingIdentity` and worktree dir is absent (rollback happened).
- **V-IT-3:** `worktree_post_create_default_runs_submodule_init` — parent repo has no `.gitmodules`; `task_new` with default config (no `post_create` override) succeeds and worktree exists.

[**Failure / Robustness Validation**]

- **V-F-1:** `worktree_rolls_back_when_identity_write_fails` — make `<wt>/.ark/` unwritable (chmod 0500) after `register_focus`; assert `Err(DeveloperWriteFailed)` and worktree dir cleaned up. (Skip on Windows; gate with `#[cfg(unix)]`.)

[**Edge Case Validation**]

- **V-E-1:** `worktree_creation_succeeds_when_user_overrides_post_create_to_empty` — user `config.toml` sets `post_create = []`; `task_new` skips submodule init, succeeds.
- **V-E-2:** `worktree_copy_missing_source_still_hard_fails` — regression: explicit `copy = [".env"]` with no `.env` file still returns `WorktreeCopySourceMissing` (proves `cfg.copy` semantics unchanged).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (submodule default) | V-UT-1, V-IT-3, V-E-1 |
| G-2 (identity-sync step) | V-UT-3, V-IT-1, V-IT-2, V-F-1 |
| G-3 (TTY detection) | V-IT-2 |
| G-4 (template active default) | V-UT-2 |
| G-5 (SPEC additions) | covered by spec amendment + worktree SPEC tests |
| C-1 (placement) | V-IT-1 (placement implied by mirror succeeding before copy) |
| C-2 (IsTerminal) | V-IT-2 |
| C-4 (parent before worktree) | V-IT-2 (when parent write would be skipped, worktree write also doesn't happen — verified via rollback) |
| C-5 (no double copy) | V-E-2 (cfg.copy untouched) |
| C-6 (override) | V-E-1 |
| C-7 (template parity) | V-UT-2 |
| C-9 (rollback) | V-F-1, V-IT-2 |
| C-10 (preserve test) | V-E-2 |
