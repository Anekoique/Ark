# Sessions, State, and Resumption

## What the primitive means

A *session* is the unit the harness uses to bundle "one ongoing
conversation, in one working directory, with one model, against one set of
tools." The session-state primitive answers three operational questions:

1. **Resume.** After I close the terminal (or the laptop reboots), can I
   pick up where I left off — same context window, same scrollback, same
   memory of which tools succeeded?
2. **Coherence across checkouts.** I have the same repo open in two
   terminals (or one terminal + one IDE). Are they the same session or
   different sessions? What happens if both try to edit `task.toml` at
   once?
3. **Crash recovery.** The agent dies mid-tool-call. What survives?

These are *boring infrastructure problems* — but they show up the moment a
single user has more than one session, and any team picks up at least one
of them in practice.

## How leading harnesses implement it

### Claude Code (Anthropic)

**Storage.** `~/.claude/projects/<encoded-cwd>/*.jsonl` — one file per
session, one JSON object per line (`kentgigger.com/posts/claude-code-conversation-history`).
The `<encoded-cwd>` is the absolute working directory with non-alphanumeric
characters replaced by `-`, so two sessions in the same dir live as
siblings.

**Format.** JSON Lines: each line is one event (user message, assistant
message + tool uses, tool result, file modification record). The file is
append-only during the session.

**Resume.**
- `claude --continue` (`-c`) → last session in current dir.
- `claude --resume` (`-r`) → picker over recent sessions.
- `/resume` slash command works inside an active session.

**Coherence.** No file-locking across two terminals; both can write to
the same directory but each gets its own session file (because new session
= new file). The risk surfaces when two sessions edit *the same source
file*, not the *same session file*.

**Crash recovery.** JSONL is append-only and line-delimited — a partially
written line is detected and dropped on resume; everything before survives.
Known regression: "Conversation history missing on resume (except last
message)" (Issue #24304) — fragile in some edge cases.

**Checkpointing.** Independent primitive — `/rewind` operates on file edits
*within* a session, not session boundaries. See
`snapshots-and-checkpoints.md`.

### OpenAI Codex CLI

**Storage.** `~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<id>.jsonl`
(`developers.openai.com/codex/cli/features` and Inventive HQ writeup).
Date-partitioned, JSONL like Claude.

**Resume.**
- `codex resume` → picker over recent sessions in current dir.
- `codex resume --last` → skip the picker.
- `codex resume --all` → consider sessions from any dir.
- `/resume` inside an active session.
- `codex continue` → last session in CWD.
- `experimental_resume = "/path/to/session.jsonl"` in `config.toml` —
  **broken on current main** per Issue #4393; treat as historical.

**Coherence.** Same directory pattern as Claude; per-CWD picker scopes
resume to dir.

**Crash recovery.** "Resumed sessions append new content to the existing
rollout file rather than creating a new session" — true append-only design,
so a mid-session crash leaves a recoverable file.

### OpenHands

**Storage.** Session ↔ container coupling. "Each conversation must have
its own exclusive container" (Issue #6382). The container is the durable
state; the chat log is secondary.

**Resume.** Rejoining a conversation re-attaches to the same container if
`keep_runtime_alive = 1` in config. Without it, the container is
recreated and *all filesystem state from the session is lost*.

**Crash recovery.** A condenser keeps the conversation under context limit
via summarization; "persistent EventLog means full replay even after
compression" (OpenHands docs). Replay is the recovery primitive.

**Multi-session.** No memory between sessions explicitly — "each new
conversation starts fresh, so complex multi-session projects need you to
re-provide context."

### Aider

**Storage.** `.aider.chat.history.md` + `.aider.input.history` per repo.

**Resume.** Historically *no* native resume — Issue #118 requested it for
years. Workarounds: copy/paste from history file, use `--restore-chat-history`
to seed a new session. Aider's atomic-commit model means the *code state*
survives even if the *chat state* doesn't.

**Crash recovery.** The atomic Git commits ARE the crash-recovery primitive.
"Whenever aider edits a file, it commits those changes with a descriptive
commit message" (`aider.chat/docs/git.html`). `/undo` reverts the last
Aider commit. This trades chat-history fidelity for code-history fidelity
— a different bet than Claude / Codex.

### Cursor 2.0

Per-agent workspaces (worktree or remote machine). Composer plans can be
created with one model and executed with another, foreground or background.
Background/cloud agents have "99.9% reliability for faster startup and
better visibility of background work" — implying a real durable store
behind the scenes, but no public storage-format spec.

### Cline

Memory Bank as MCP server — pushes session state into structured markdown
files (`projectbrief.md`, `activeContext.md`, etc.) that load fresh into
each new session. The session is *intentionally* not durable; the memory
file is.

### Continue.dev

`.continue/` directory holds config and rules; no built-in session-persist
beyond editor state. Continue's stance is "stateless agent + persistent
rules."

### Goose (Block)

Sessions, recipes, and project management at the CLI. Recipes are
reusable workflow definitions; sessions can be resumed by name. Memory is
managed via MCP extensions.

## The "two terminals, one task" problem

A real production scenario: developer has the same repo open in two
terminals — VS Code shell + dedicated CLI tab. Both can run the harness.
Both observe the same `.ark/`. What guarantees do we have?

| Harness | Per-checkout state | Locking | Resolution |
| ------- | ------------------ | ------- | ---------- |
| Claude Code | One JSONL per session (terminal opens new file) | None | Two parallel sessions, no shared state |
| Codex CLI | Same as Claude | None | Two parallel sessions |
| OpenHands | One container per conversation | Container is the lock | Implicit serialization through container |
| Aider | One chat history file (per repo) | None | Last write wins on history file |
| Ark | One `.state.toml` per checkout / worktree | `flock` on `.state.toml.lock` | Serialised mutation; per-worktree focus |

Ark's stance is the **most disciplined** of the surveyed harnesses on
multi-checkout coherence — described below.

## What Ark does today

Ark's session-state model is *unusually* sharp because Ark is not
itself a long-running process: every transition is a one-shot CLI invocation
that must read, mutate, and re-persist the state durably.

### `.ark/.state.toml` — per-checkout state

`crates/ark-core/src/state/checkout/` is the load-bearing module. Truth
lives in the per-task `task.toml`; `.state.toml` is an **index** reconciled
against truth on every read.

Fields (`crates/ark-core/src/state/checkout/model.rs`):
- `tasks.active` — slug set, mirrors the on-disk `task.toml` set.
- `focus` — single slug naming the task this checkout is currently driving.
- Identity (`workspace` feature) — developer name resolved from
  `.ark/.developer`.

The two structural guarantees:

1. **Per checkout, per worktree.** "Deep-tier worktrees own their own
   `.state.toml` and so their own focus" (`workflow.md:376`). Two terminals
   in the same checkout share state; a worktree at `.ark/worktrees/<branch>/`
   gets a fresh `.state.toml`. This is why `Layout::slug_from_worktree_root`
   exists (`layout.rs:352`).
2. **One focus per checkout.** "There is no per-session map: one focus per
   checkout" (`checkout/mod.rs:11`). Setting focus on a busy checkout
   *warns* and suggests `--worktree` for parallel work — see
   `task new --slug <s>` behaviour in `workflow.md:379`.

### Atomic mutation under file-lock

`crates/ark-core/src/state/checkout/io.rs:64` — `state_mutate`:

```rust
pub fn state_mutate<F>(layout: &Layout, edit: F) -> Result<()>
where F: FnOnce(&mut StateFile) -> Result<()> {
    let lock_path = layout.state_lock_file();
    let _guard = LockGuard::acquire(&lock_path)?;
    sweep_tmp_orphans(layout)?;
    let mut state = read_or_synthesize(layout)?;
    reconcile_against_disk(layout, &mut state)?;
    edit(&mut state)?;
    // ... atomic write via temp + rename
}
```

Mechanism:
- Lock = `.ark/.state.toml.lock` sentinel + advisory `File::try_lock`
  (`layout.rs:98`).
- Backoff = five attempts over ≤320 ms (`LOCK_BACKOFF_MS = &[10, 20, 40,
  80, 160]`, `io.rs:32`).
- On lock failure → `Error::StateLockContended`.
- Writes go to `.state.toml.tmp.<pid>` and rename atomically; readers see
  pre-rename or post-rename, never partial (`STATE_TMP_PREFIX`,
  `layout.rs:109`).
- Orphan tmp files cleaned on next mutation under the lock (`sweep_tmp_orphans`).

### Reconciliation against `task.toml` truth

`crates/ark-core/src/state/checkout/reconcile.rs` runs on every read:
walks `.ark/tasks/*/task.toml`, builds the canonical active-set, drops
state entries for tasks that no longer exist, drops focus if its target
is gone. The state file *cannot* drift from on-disk truth.

### Legacy migration

`crates/ark-core/src/state/checkout/migrate.rs` — the now-legacy
`.ark/tasks/.current` and bare `.ark/.developer` files are synthesised
into a fresh `.state.toml` on first read; their files are deleted only
after a successful save. This is the "self-healing" guarantee mentioned
at `checkout/mod.rs:14`.

### Crash recovery shape

- A crash before lock acquisition: nothing to clean — the lock file is
  zero bytes and the next mutator drops it.
- A crash mid-edit: the tmp file is orphaned; the next `state_mutate`
  sweeps it.
- A crash mid-rename: filesystems guarantee atomicity for `rename(2)` on
  the same volume; either you have the old file or the new one.

### What Ark does *not* do

- **No conversation log.** Ark doesn't store the LLM's transcript — that's
  the host harness's job. `~/.claude/projects/...` / `~/.codex/sessions/...`
  remain the source of truth for chat.
- **No mid-task resume of the *agent*.** `task resume --slug` rebinds focus
  but does not restore the LLM's working memory; the host harness's
  `--continue` / `--resume` does that.
- **No multi-checkout chat sync.** If a user wants Claude in terminal A and
  Codex in terminal B to share understanding, that's not a problem Ark
  solves.

## Comparison to the surveyed harnesses

Ark's `state.toml` is a *workflow* state, not a *conversation* state. It
sits *alongside* `~/.claude/projects/.../session.jsonl`, not in
competition. The combination is what's interesting:

| Layer | Owner | Mechanism | Multi-checkout safety |
| ----- | ----- | --------- | --------------------- |
| LLM conversation | Host harness | JSONL append-only | None |
| Working dir / code | Git | Worktrees + commits | Git's own locking |
| Workflow state | Ark | `.state.toml` + flock + reconcile | Strong |
| Project memory | CLAUDE.md / AGENTS.md / SPECs | Markdown | None (file ACL) |

This is the right division. The risk Ark mitigates that competitors don't
is the **focus slot collision**: two terminals advancing `phase = plan` on
the same task. By making focus per-checkout *and* rebinding-warned, Ark
forces the user to pick a worktree or accept a single focus.

## Directions for Ark

1. **Surface session-id from the host harness in `ark context`.** Today
   `ark context --scope session` shows git + tasks + specs. If we also
   surfaced the *host* session id (Claude's CWD-encoded JSONL path; Codex's
   rollout filename), users could correlate Ark's workflow phase with
   their host transcript. Implementation: read it from the `SessionStart`
   hook's invocation env (Claude passes `CLAUDE_SESSION_ID`).
   Code site: `crates/ark-core/src/commands/context/gather.rs`.
2. **`ark session resume` — Ark-owned resume verb.** Light wrapper over
   `claude --resume` / `codex resume` that ALSO sets the per-checkout
   focus to the slug whose worktree matches the resumed session's CWD.
   Closes the "I resumed Claude but Ark thinks I'm on a different task"
   drift. Code site: new `crates/ark-core/src/commands/agent/session.rs`.
3. **Add a `last_committed_at` materialised field to focus.** Currently
   `[focus]` carries just the slug; surfacing the last commit timestamp
   lets `ark context` order tasks by recency without re-stat'ing every
   `task.toml`. Code site: `crates/ark-core/src/state/checkout/model.rs`.
4. **Lock-fail diagnostic.** Today `Error::StateLockContended` after
   320 ms gives just the path. Include the *holder PID* (read from the
   lock file owner) so users debugging multi-checkout collisions can find
   the offender. Code site:
   `crates/ark-core/src/state/checkout/io.rs:64`.
5. **State-file schema versioning.** `StateFile` is not versioned today;
   `task.toml` has implicit fields. Adding `schema = 1` to `.state.toml`
   future-proofs the unload/load round-trip and matches `Snapshot`'s
   `SCHEMA_VERSION` pattern (`crates/ark-core/src/state/snapshot.rs:21`).
   Code site: `state/checkout/model.rs`.

## Caveats / Not found

- I did not find a primary-source description of how Cursor 2.0 stores
  its background-agent state; the "99.9% reliability" line is marketing.
- The exact crash-resilience semantics of OpenHands' `condenser` are not
  documented at the file-format level; "EventLog full replay" is the
  abstract claim.
- I have not verified the specific corner cases of Claude Code's
  conversation-loss bug (Issue #24304) at the file-format level.
- Codex's `app-server` + JSON-RPC protocol opens a richer story than just
  CLI session resume; this file does not cover the desktop / cloud sync
  surface in depth (worth its own follow-up).

## Sources

- [Claude Code Sessions reference](https://code.claude.com/docs/en/agent-sdk/sessions)
- [Claude Code History Guide (kentgigger)](https://kentgigger.com/posts/claude-code-conversation-history)
- [Claude Code History Bug (Issue #24304)](https://github.com/anthropics/claude-code/issues/24304)
- [Codex CLI Resume / Continue / Save Chat](https://www.verdent.ai/guides/codex-cli-resume-continue-save-chat)
- [Codex CLI Reference](https://developers.openai.com/codex/cli/reference)
- [Codex Session Files Discussion #3827](https://github.com/openai/codex/discussions/3827)
- [Codex experimental_resume regression (Issue #4393)](https://github.com/openai/codex/issues/4393)
- [OpenHands container persistence (Issue #6382)](https://github.com/OpenHands/OpenHands/issues/6382)
- [Aider Git integration](https://aider.chat/docs/git.html)
- [Aider Resume request (Issue #118)](https://github.com/paul-gauthier/aider/issues/118)
- [Cline Memory Bank](https://github.com/dazeb/cline-mcp-memory-bank)
- [Cursor 2.0 changelog](https://cursor.com/changelog/2-0)
