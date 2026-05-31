# Research: Error Messages and Recovery

- Query: Ark's structured errors (`NothingStaged`, `VerifyIncomplete`, `IllegalPhaseTransition`, `NoFocus`, `TaskNotFound`) — each names a recovery path. Comparison to other harnesses; self-healing patterns; "did you mean" precedents.
- Scope: mixed
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-core/src/error.rs` | The single source of truth for all Ark errors. 60+ variants, each annotated with `#[error(...)]` via `thiserror`. |
| `crates/ark-cli/src/main.rs` (l. 543-556) | Main entry. On error, walks the source chain and prints `error: <top>` followed by `  caused by: <each_source>`. |
| `crates/ark-cli/src/main.rs` (l. 462-501) | The developer-identity prompt's "try up to 3 times" loop — one of Ark's few interactive recovery surfaces. |
| `crates/ark-core/src/commands/agent/state.rs` | The phase / tier state machine. `IllegalPhaseTransition` is generated here, with the actual values printed. |
| `.ark/workflow.md` (l. 244, l. 350) | The user-facing docs naming each error and what to do about it. |

### Code patterns

**Recovery hints embedded directly in error messages.** From `crates/ark-core/src/error.rs`:

```rust
/// Ark is already loaded in the target project.
#[error("ark is already loaded at {path}; pass --force to replace it")]
AlreadyLoaded { path: PathBuf },
```

The error names both the problem (already loaded) and the recovery (pass `--force`). The user does not need to consult docs.

```rust
/// `task_commit` invoked with no staged work.
#[error("task `{slug}` cannot be committed without staged work; run `git add <files>` first")]
NothingStaged { slug: String },
```

`NothingStaged` literally tells the user the next command to run.

```rust
/// VERIFY.md has unresolved checklist items or findings.
#[error(
    "VERIFY.md at {path:?} has {items} pending item(s) and {findings} pending finding(s); \
     resolve before commit"
)]
VerifyIncomplete { path: PathBuf, items: u32, findings: u32 },
```

`VerifyIncomplete` includes the path, the item count, the finding count — enough information to grep or open the file directly.

```rust
/// This checkout has no focused task.
#[error(
    "no focus set in `{}`; run `ark agent task new` or `task resume --slug <one-of>` to bind \
     this checkout (active: {})",
    project_root.display(),
    if candidates.is_empty() { "<none>".to_string() } else { candidates.join(", ") },
)]
NoFocus { project_root: PathBuf, candidates: Vec<String> },
```

`NoFocus` is the most elaborate: names the recovery command, lists the available slugs the user could resume, handles the empty-candidates case. The error format string itself contains conditional rendering (`if candidates.is_empty()`) — unusual sophistication.

**The error chain printer** (`crates/ark-cli/src/main.rs:543-556`):

```rust
fn main() -> ExitCode {
    match Cli::parse().command.dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            std::iter::successors(Some(&*err as &dyn std::error::Error), |e| e.source())
                .enumerate()
                .for_each(|(i, e)| match i {
                    0 => eprintln!("error: {e}"),
                    _ => eprintln!("  caused by: {e}"),
                });
            ExitCode::FAILURE
        }
    }
}
```

Output shape:
```
error: io error at /path/to/.ark/config.toml: ...
  caused by: Permission denied (os error 13)
```

Source-chain walking is standard `anyhow`-style. The `error:` prefix and `  caused by:` indent are consistent with `cargo`'s error output.

**Refusal of dangerous defaults.** From `crates/ark-cli/src/main.rs:262-266`:

```rust
anyhow::bail!(
    "init requires at least one of --claude, --codex, or --opencode when stdin is not a TTY \
     (use --no-claude / --no-codex / --no-opencode to opt out per platform)"
);
```

The non-TTY case refuses to silently pick "all platforms." The error message both explains the rule and lists every flag the user could use to satisfy it. Six flags named in one error message.

**Per-error variant doc-comments.** Every variant in `error.rs` has a `///` doc comment explaining what the case means *and* what its fields contain. E.g.:

```rust
/// Task slug did not resolve to an existing task.
#[error("task not found: {slug}")]
TaskNotFound {
    /// Missing task slug.
    slug: String,
},
```

This is for the source-reader (a future agent or maintainer). The structured doc comments mean a code-search tool can find the canonical description of every error.

**Errors with conditional render.** `MissingIdentity` (l. 313-318):

```rust
#[error(
    "no developer identity set; run `ark init --developer <name>` or set [workspace] \
     developer in .ark/config.toml"
)]
MissingIdentity,
```

Names two recovery paths (flag *or* config file). Acknowledges that users might prefer different mechanisms.

**Workflow doc explicitly enumerates error recovery.** From `.ark/workflow.md:244`:

> **Failure modes:** `NothingStaged` → user runs `git add`. `VerifyIncomplete` → resolve PENDING items. `CommitMessageRequired` → slash command logic bug. `GitCommitFailed` → surface stderr; rollback already happened. `IllegalPhaseTransition` → wrong phase; tell the user the current one.

Five error variants listed, each with a one-clause recovery. The workflow doc treats errors as first-class workflow vocabulary — the user is supposed to recognize these names from the CLI output.

### External references

#### "Did you mean" — clap's typo suggestions

clap automatically suggests corrections for misspelled flags ([Kevin Knapp's blog](https://kbknapp.dev/rust-cli/)):

> "Suggestions: Suggests corrections when the user enters a typo. For example, if you defined a --myoption argument, and the user mistakenly typed --moyption (notice y and o transposed), they would receive a Did you mean '--myoption'?"

Ark inherits this for free because it uses clap. The mechanism: Levenshtein distance against the set of registered flags / subcommands; if close enough, suggest. Same pattern as `cargo`, `rustup`, `gh`.

#### Cargo's error messages

`cargo`'s error format is the de facto Rust standard. Format:

```
error: failed to compile `foo` v0.1.0
Caused by:
  process didn't exit successfully: `rustc ...` (exit status: 1)
  caused by: ...
```

Top-line summary, indented chain. Some errors include help footers ("note: `--release` would optimize the build"). Cargo emits `--explain <code>` references for compiler errors. Ark doesn't have an analog because Ark errors don't have numeric codes.

#### Rust compiler errors — `--explain E0382`

`rustc` errors have codes (`E0382` for borrow-after-move, etc.) and `rustc --explain <code>` prints a long-form explanation with example code. The pattern is heavyweight (you have to maintain an explanation file per error) but pays off for tools whose error vocabulary is large and shared with a broader audience.

Ark's error vocabulary is large (60+ variants) and the workflow doc treats them as first-class names. An `ark explain NothingStaged` would fit naturally — print a paragraph with examples and links. Currently no such command.

#### Aider's error UX

Aider's error handling is conversational. From the [aider docs](https://aider.chat/docs/troubleshooting/support.html):

> "If you encounter errors, copy and paste the error messages into Cline's chat. This will help it understand the issue and provide a solution."

(That quote is from a Cline blog post but the pattern is shared.) Aider's errors are mostly plain Python tracebacks for internal failures and structured "I can't do this because…" prose for usage failures. The `/help <question>` command lets users ask the tool itself for guidance.

The conversational-error pattern is less precise than Ark's named errors but easier for new users — they don't need to learn an error vocabulary.

#### Claude Code's error UX

Claude Code emits errors through the normal chat surface ("I can't run that command because…", "I don't have permission to read this file"). The user has no separate error window to consult. Recovery is conversational: "ok, do X instead."

#### Self-healing patterns

Some tools attempt self-healing on common errors:

- `npm install` retries failed downloads.
- `cargo build` re-resolves dependencies when the lock file is stale.
- `git pull --rebase` re-tries after merge-conflict resolution.

Ark does not self-heal in this sense. Each error returns to the user. The design choice is defensible — Ark's errors are mostly state-machine-shaped (`IllegalPhaseTransition`), and silently transitioning a phase to recover from a user mistake would mask the workflow problem.

#### The "suggested next command" pattern

Modern CLIs increasingly emit a suggested next command on success and failure alike:

- `git init` → `Use "git add" to track files.` (in newer versions)
- `gh repo create` → `https://github.com/<user>/<repo> created. Run "cd <repo>" or "git clone ...".`
- `cargo new` → no suggestion (still the minimal-output model).
- `npm install` → no suggestion.

On failure:
- `git commit` (empty index) → `nothing to commit, working tree clean` (suggests `git add` implicitly).
- `gh repo clone <nonexistent>` → `GraphQL: Could not resolve to a Repository with the name 'xxx'`.

Ark's errors *do* suggest next commands (`NothingStaged` says `git add`). The asymmetry is on the success path: `ark agent task new --slug s --title "t"` succeeds with a one-line summary but doesn't say "now run `ark context` and start the PRD."

#### Error-driven workflow as a feature

Ark's error model treats errors as part of the workflow, not as exceptions. `NoFocus`, `IllegalPhaseTransition`, `WrongTier` — these are *workflow guidance* dressed as errors. Hit `IllegalPhaseTransition` and you've learned something about the state machine; the next time you run the command, you'll get the order right.

This is structurally similar to `git`'s error UX: hit `error: failed to push some refs to ...` and the message tells you to `git pull` first. The error teaches the rule.

### How Ark stacks up

**Strong patterns:**

- Every variant has a clear name. `NothingStaged` is easier to discuss than "error code 47."
- Every relevant variant names the recovery command.
- Source chain is printed, not flattened — root causes are visible.
- Workflow doc enumerates the user-facing variants by name.
- Conditional render (`NoFocus` listing active candidates) helps the user without requiring a separate command.
- TypeNotFound / TaskAlreadyExists / TaskStillActive trio covers "the slug doesn't exist," "the slug already exists," "the slug exists but isn't ready to leave."
- Error messages are factual without being cryptic. Compare `WorktreeDirty`: `"worktree at {path:?} has uncommitted changes; pass --force to override"` — names the check, names the workaround.

**Mediocre patterns:**

- No URLs in error messages. A user who hits `VerifyIncomplete` has no link to "what is VERIFY.md?"
- No `ark explain <ErrorName>` command. The `rustc --explain` analog would fit naturally.
- Success-path next-step hints are absent. `ark agent task new` succeeds silently; no "now write your PRD at <path>" hint.
- The `caused by:` chain can stack deeply for I/O errors. Hard to tell at a glance which layer is the actionable one.

**Missing:**

- No structured-output for errors (JSON / TOML). Slash commands parsing CLI output rely on `eprintln!` text format.
- No exit-code differentiation. Every error exits with code 1; a CI system can't distinguish "user mistake" from "config corruption" from "git failure" without parsing stderr.
- No telemetry on which errors are hit most often in the wild — the design optimizes the errors named in workflow.md, but there's no feedback loop to confirm those are the hot ones.

### Error name × recovery pattern × user agency

Mapping every CLI-visible Ark error to (a) what it names and (b) whose action recovers:

| Error | Names | Recovery agent |
| --- | --- | --- |
| `AlreadyLoaded` | "ark is already loaded" | User: pass `--force` |
| `NotLoaded` | "no ark installation found" | User: run `ark init` or `ark load` |
| `UnsafeSnapshotPath` | "refusing unsafe snapshot path" | User: inspect / regenerate `.ark.db` |
| `IllegalPhaseTransition` | "illegal phase transition" | Agent: figure out the correct phase verb |
| `WrongTier` | "wrong tier" | Agent: choose tier-appropriate verb |
| `TaskNotFound` | "task not found" | User: verify slug |
| `TaskAlreadyExists` | "task already exists" | User: pick a new slug |
| `NoFocus` | "no focus set in {path}" | User: `task new` or `task resume` |
| `UnknownTemplate` | "unknown template" | Bug in Ark or template build |
| `SpecSectionMissing` | "PLAN at {path} has no `## Spec` section" | Agent: write the Spec section |
| `SpecAlreadyExists` | "feature SPEC already exists" | User: chose a different feature path |
| `NoPlanFound` | "no `NN_PLAN.md` found" | Agent: ensure plan files exist |
| `TaskTomlCorrupt` | "task.toml corrupt at {path}" | User: edit by hand |
| `ManagedBlockCorrupt` | "managed block corrupt" | User: hand-repair the file |
| `DowngradeRefused` | "refusing to downgrade" | User: pass `--allow-downgrade` |
| `WorktreeDirExists` | "worktree directory already exists" | User: cleanup first |
| `WorktreeNotFound` | "no worktree found for slug" | User: check slug |
| `WorktreeDirty` | "worktree at {path} has uncommitted changes" | User: stash / commit / pass `--force` |
| `BranchInUse` | "branch already checked out" | User: switch worktrees |
| `InvalidBranchName` | "invalid branch name" | User: fix name |
| `MissingIdentity` | "no developer identity set" | User: `ark init --developer` or config |
| `ArchiveIndexNotEmpty` | "ark archive requires a clean staging area" | User: `git stash` / commit |
| `NothingStaged` | "cannot be committed without staged work" | User: `git add <files>` |
| `VerifyIncomplete` | "{N} pending items" | User+Agent: resolve PENDING |
| `GitCommitFailed` | "git commit failed at {path}: {stderr}" | User: read stderr |
| `CommitMessageRequired` | "commit message is required" | Slash command bug |
| `FeaturePathMissing` | "PRD at {path} has no `[**SPEC Path**]` block" | Agent: add the block |
| `InvalidFeaturePath` | "invalid SPEC path" | Agent: fix the path |

The pattern is consistent: each error knows whose mistake produced it and which command (or hand-edit) makes it go away. This is the load-bearing UX work that puts Ark ahead of most agent-harness peers.

### What's truly distinctive

Most CLI errors describe *what happened*. A subset adds *what to do*. Ark's errors do both *and* name the actor (user vs agent). When an agent reads `IllegalPhaseTransition`, it knows the next verb is wrong; when a user reads `NothingStaged`, they know to `git add`. The error message is workflow guidance.

The closest peer in the surveyed tools is `git` itself — `git push` failures often suggest `git pull --rebase`, `git commit` failures explain the index state, `git rebase` failures walk through `--continue` / `--abort` / `--skip`. `git`'s error UX is the gold standard, and Ark approximates it for its smaller surface.

Compared to AI-agent peers (Aider, Cline, Claude Code, Continue.dev): those tools' errors are mostly conversational ("I can't do this because…"). Ark's errors are structured types with stable names. The advantage: testable, greppable, citeable. The cost: a vocabulary the user has to acquire.

### Caveats / Not found

- No data on which Ark errors are hit most often. The list above is comprehensive but unranked.
- The exit-code situation (every error = 1) was not exhaustively verified across the CLI; only the main dispatch path was read.
- Aider's actual error wording for common cases (e.g. "this file is too big to load") was not sampled directly; the conversational characterization is based on docs.
- No survey of `gh`'s error UX, which is another good peer.
- `rustc --explain` content depth is not directly comparable since rustc errors are about language semantics, not workflow semantics.

## Directions for Ark

1. **Add `ark explain <ErrorName>` as a top-level subcommand.** Mirror `rustc --explain`. Body content: one paragraph per error name explaining the cause, listing the named recovery command from `error.rs`, and linking to mdBook. The error-name set is closed (60+ variants); each page is ~10 lines. Single biggest learnability win for the error vocabulary.

2. **Embed mdBook URLs in error messages.** Pattern:
   ```rust
   #[error(
       "VERIFY.md at {path:?} has {items} pending item(s) and {findings} pending finding(s); \
        resolve before commit (https://anekoique.github.io/ark/errors/verify-incomplete)"
   )]
   ```
   The URL doesn't need a page on day one — adding the page later just deepens the help; the URL itself is the affordance.

3. **Differentiate exit codes by error class.** User-input errors exit with 2, state errors with 3, I/O errors with 4, git failures with 5. CI systems can branch on exit code without parsing stderr. The current "every error = 1" forces stderr-grepping in scripts.

4. **Emit success-path next-step hints.** After `ark agent task new`, print:
   ```
   created task agent-harness-infra (research/research) at .ark/tasks/agent-harness-infra
   next: open .ark/tasks/agent-harness-infra/PRD.md and fill in What/Why/Outcome
   ```
   The error-side ("next command to run on failure") work is excellent; the success-side equivalent is absent. Cheap to add per verb.

5. **Add structured JSON error output to `ark agent`.** `ark agent task commit --json` should emit `{"error_type": "NothingStaged", "slug": "foo", "message": "..."}` on failure. Slash commands currently parse text; structured output reduces fragility. Pairs with the existing `ark context --format json` style.
