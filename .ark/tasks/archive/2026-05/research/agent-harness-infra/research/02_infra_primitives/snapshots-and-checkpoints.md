# Snapshots and Checkpoints

## What the primitive means

"Snapshot" and "checkpoint" name *three structurally different* primitives
that get conflated. Pulling them apart is half the work of comparing
implementations:

| Primitive | Scope | When it fires | Restore semantic |
| --------- | ----- | ------------- | ---------------- |
| **Install snapshot** | The harness's footprint in a project | Explicit (Ark `unload`, Docker `commit`) | "Bring back the install I had" |
| **Edit checkpoint** | One file or set of files inside a session | Per LLM edit / per user prompt | "Undo the last few moves the agent made" |
| **Environment snapshot** | An entire VM / container state | Pre-warm pool, post-fail | "Boot from this image in ms" |

Plus git as the **universal fallback** — every code change is a commit;
`git reset` reverts.

The interesting platforms compose them. Ark uses install snapshots
(`.ark.db`). Claude Code uses edit checkpoints (`/rewind`). E2B uses
environment snapshots (Firecracker pre-warm pool). Aider uses git as the
checkpoint substrate.

## Install snapshots

### What they solve

A user pulls Ark out of a project temporarily (move it to a different
shell, swap to a different harness), then puts it back. The footprint —
files, managed blocks, hook entries — must round-trip *byte-for-byte*,
including user edits to those files.

### Ark's `.ark.db` model

The canonical example. `crates/ark-core/src/state/snapshot.rs`:

```rust
pub const SNAPSHOT_FILENAME: &str = ".ark.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: String,
    pub ark_version: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<SnapshotFile>,
    pub managed_blocks: Vec<SnapshotBlock>,
    #[serde(default)]
    pub hook_bodies: Vec<SnapshotHookBody>,
}
```

Three captured artifact kinds:

- **Files** — every project-relative path Ark owns
  (`Layout::owned_dirs()`, `layout.rs:506`) is captured byte-for-byte as
  base64 in `SnapshotFile.content_b64`.
- **Managed blocks** — every `ARK:START`/`ARK:END` block Ark inserted into
  shared files like `CLAUDE.md` / `AGENTS.md` (per-file marker tuple).
- **Hook bodies** — the platform's full `SessionStart` entry JSON object
  including any user-set timeout overrides; reapplied verbatim on restore.

Lifecycle:

| Verb | Effect |
| ---- | ------ |
| `ark unload` | Capture into `.ark.db`; delete live files |
| `ark load` | If `.ark.db` exists, restore from it and delete the snapshot; otherwise scaffold from embedded templates |
| `ark remove` | Unconditional wipe; ignores `.ark.db` |

Schema is **versioned** (`SCHEMA_VERSION = "1"`); `#[serde(default)]` on
`hook_bodies` means older `.ark.db` files load with an empty hook list,
forward-compat by design.

### Docker `commit` as install snapshot

`docker commit <container>` makes an image from a container's current
state — same shape, different scope. Not commonly used in agent
harnesses because the *project* state matters, not the *container*
state.

### Why install snapshots matter

The hard work of an install snapshot is preserving **user edits to managed
files**. Ark separates Ark-written content (`include_dir!` template
bytes, snapshot-replayable verbatim) from user-edited content (managed
blocks, hook siblings, custom commands under `.claude/commands/ark/`).
Round-trip preservation is asserted by `commands/load.rs::tests`.

Nothing else in the survey solves this primitive — because no other
harness *needs* to. Ark is unusual in being a "patch layer" over Claude /
Codex / OpenCode that may be temporarily removed.

## Edit checkpoints

### Claude Code `/rewind` (Anthropic, Sept 2025)

The canonical implementation. `code.claude.com/docs/en/agent-sdk/file-checkpointing`.

- **Captures:** file modifications made through `Write`, `Edit`,
  `NotebookEdit`. **NOT** captures `Bash` edits — `echo > file.txt` and
  `sed -i` are invisible to checkpointing.
- **Trigger:** every user prompt creates a checkpoint.
- **UI:** `Esc Esc` or `/rewind`.
- **Restore options:** code, conversation, or both.
- **Persistence:** "checkpoints persist across sessions and are cleaned up
  with session cleanup (~30 days, configurable)."

The blank corollary: **a `Bash` tool that runs `cargo fmt` between two
Claude edits leaves a hole in the rewind story.** This is a known
limitation, not a bug.

### Codex CLI — open feature requests

No native `/rewind` (as of late 2025). Issue #11626 explicitly requests
"Claude Code-style /rewind with checkpoint restore + context summarize
modes" (`github.com/openai/codex/issues/11626`); Issue #12558 similar;
Issue #6449 "Code and context rollback." Codex is going to ship something
in this shape; the open question is what storage substrate.

The implicit storage today is the rollout JSONL — file edits could be
reconstructed by walking the rollout and reverse-applying each
`Edit`/`Write` tool call. Probably what Codex will ship.

### Cursor

Cursor has had a "Restore Checkpoint" UI since the Composer era. Pre-edit
file states are stashed; users can rewind without leaving the editor.
Storage shape is undocumented.

### Aider — git is the checkpoint

`/undo` reverts the last Aider commit; Aider's policy of one-commit-per-edit
makes git the checkpoint store. "Whenever aider edits a file, it commits
those changes with a descriptive commit message" (`aider.chat/docs/git.html`).

This is **the simplest design** — let git carry the state. Trade-off:
your git log gets noisy and you can't easily rewind without polluting
history.

### Ark today — no edit checkpoint

Ark does not edit user code; it edits its own state. There is no edit
checkpoint primitive in Ark. The closest analogue is the per-task PLAN
iteration loop in deep tier (NN_PLAN / NN_REVIEW), which preserves
historical PLAN versions in the task directory.

## Environment snapshots

### Firecracker (E2B and similar)

The relevant primitive of microVM platforms:

- "A pool of VMs is started ahead of time, brought to a ready state, and a
  memory snapshot is taken, with incoming requests restoring directly
  from the snapshot rather than booting a kernel from scratch, reducing
  cold-start time to approximately 150 ms" — and as low as **3–8 ms** for
  recent restore optimizations (Medium / Particula writeups, March 2026).
- Snapshot file contains: full memory contents + CPU register state. On
  restore: memory-map the file, load CPU state, resume from exactly
  where execution stopped.

### How agent platforms exploit this

- **E2B Sandboxes:** "Each agent task gets a full E2B sandbox VM
  containing Chromium, a terminal, a filesystem, and 27 other tools."
- **AWS Lambda / Fargate snapshot warming model:** for 100+ VMs/host
  density.
- **Modal Sandboxes:** gVisor based; comparable snapshot story without
  full VM.

### Snapshot pre-warm tradeoff

| Approach | Cold start | Memory cost | Setup |
| -------- | ---------- | ----------- | ----- |
| Fresh boot | 1-5 s | None | None |
| Snapshot restore (cold) | 150 ms | Per-snapshot disk | Pre-built image |
| Snapshot restore (memory-mapped) | 3-8 ms | Per-VM RAM | Active pre-warm pool |
| Container restart | 0.5-2 s | Image disk | Cached image |

For an agent that runs many short tool calls, sub-100-ms cold start is the
difference between *interactive* and *batch*. This is why E2B chose
microVMs over containers.

### Ark and environment snapshots

Out of scope today. Ark would adopt this only via a `[worktree].post_create`
hook that boots a snapshot-warmed container — not a primitive Ark itself
owns.

## Git as universal fallback

Every harness leans on git for "if all else fails, the user can `git
reset --hard`." Specific patterns:

- **Aider:** atomic commit per edit; `/undo` is `git revert HEAD`.
- **Claude Code:** `/rewind` complements git, doesn't replace it.
- **OpenHands:** container holds working tree; git per-conversation.
- **Cursor:** Composer worktree per agent; git per worktree.
- **Ark:** worktrees per task; deep-tier branches for PLAN iteration.

Git's strength: every developer already understands it. Its weakness:
edit checkpoints WANT sub-second granularity, and a commit per LLM
edit pollutes history.

## Tradeoff: granularity vs persistence

| Granularity | Best mechanism | Persistence |
| ----------- | -------------- | ----------- |
| Per-second / per-tool-call | tmpfs scratch + in-process undo | None (per session) |
| Per-prompt | Edit checkpoint (`/rewind`) | Per session (~30 days) |
| Per-commit | Git | Forever |
| Per-task | Workflow archive (Ark `task archive`) | Forever |
| Per-install | Install snapshot (Ark `.ark.db`) | Manual lifecycle |
| Per-environment | Firecracker snapshot | Pool / on demand |

## What Ark does today (summary)

- **Install snapshot.** Yes. Mature. `.ark.db` round-trips byte-for-byte.
  See `crates/ark-core/src/state/snapshot.rs` and round-trip tests in
  `crates/ark-core/src/commands/load.rs::tests`.
- **Edit checkpoint.** No. Defers to the host harness's `/rewind` and to
  git.
- **Environment snapshot.** No. Out of scope.
- **Git as fallback.** Implicit — worktrees + per-task branches; deep-tier
  iteration preserves NN_PLAN history in-tree.

## Directions for Ark

1. **Per-task snapshot file.** Today the `.ark.db` install snapshot is
   *project-global*. Add a `task snapshot <slug>` verb that captures a
   per-task snapshot under `.ark/tasks/<slug>/.ark.task.db` (PRD, PLANs,
   VERIFY, research/, all of `task.toml`). Restore with `task snapshot
   restore`. Use case: spike a risky design change, snapshot, revert if
   the spike fails. Code site: extend
   `crates/ark-core/src/state/snapshot.rs`; new verb in
   `crates/ark-core/src/commands/agent/task/`.
2. **Pre-commit snapshot for atomic rollback expansion.** `task commit`
   already has scoped rollback on git-commit failure
   (`workflow.md:240`). Today rollback restores `task.toml` /
   spec/promotion / features INDEX. Extending it to snapshot the
   modified file set *before* staging would let `task commit` roll back
   any post-stage operation, not just its own internal mutations. Code
   site: `crates/ark-core/src/commands/agent/task/commit.rs`.
3. **Edit-checkpoint awareness in `ark context`.** When a host harness
   supports edit checkpoints (Claude `/rewind`), surface the last-
   checkpoint timestamp / id in `ark context` output so a workflow
   reviewer can correlate "this commit corresponds to checkpoint XYZ."
   Pairs with Direction 1 in `sessions-state-and-resumption.md`. Code
   site: `crates/ark-core/src/commands/context/gather.rs`.
4. **`.ark.db` schema bump for plugin marketplaces.** When the Claude
   plugin marketplace story matures, multiple plugins (not just Ark)
   will want `.ark.db`-shape backup. Promote the schema to a
   crate (`ark-snapshot-format`) + a `kind` field — Ark plugins
   register as participants. Cheap future-proofing today. Code site:
   `crates/ark-core/src/state/snapshot.rs`, `SCHEMA_VERSION` constant.
5. **`ark snapshot diff <a.ark.db> <b.ark.db>`.** Two install snapshots
   are routinely produced (before / after `ark upgrade`, before / after
   a re-init). A diff verb that shows changed files / blocks / hooks
   would close the "what did upgrade actually do to my installed state"
   question. Code site: new module `crates/ark-core/src/commands/snapshot/`.

## Caveats / Not found

- I did not find Anthropic primary-source documentation of the
  `~30 days, configurable` checkpoint retention; that's a community claim.
- Codex's exact `/rewind` plan as of May 2026 is GitHub issue threads,
  not shipped behaviour; treat as "imminent but unspecified."
- Cursor's checkpoint storage shape is undisclosed publicly.
- The "3–8 ms restore" Firecracker number is from a 2026 dev.to writeup
  reproducing recent advances; verify against firecracker-microvm's docs
  before quoting.
- I have not investigated CRIU (Linux process checkpoint/restore) as a
  primitive in agent platforms; worth a follow-up.

## Sources

- [Claude Code File Checkpointing](https://code.claude.com/docs/en/agent-sdk/file-checkpointing)
- [Claude Code /rewind: 5 Patterns (Medium)](https://alirezarezvani.medium.com/claude-code-rewind-5-patterns-after-a-3-hour-disaster-a9de9bce0372)
- [Codex Issue #11626 — Add /rewind](https://github.com/openai/codex/issues/11626)
- [Codex Issue #12558 — Claude Code-style /rewind](https://github.com/openai/codex/issues/12558)
- [Codex Issue #6449 — Code and context rollback](https://github.com/openai/codex/issues/6449)
- [Aider Git Integration](https://aider.chat/docs/git.html)
- [Firecracker snapshotting docs](https://github.com/firecracker-microvm/firecracker/blob/main/docs/snapshotting/snapshot-support.md)
- [AI agent sandboxes: microVM snapshots](https://addozhang.medium.com/ai-agent-code-execution-sandboxes-isolation-from-containers-to-microvms-e80848effea5)
- [28ms boot via Firecracker snapshots (DEV)](https://dev.to/adwitiya/how-i-built-sandboxes-that-boot-in-28ms-using-firecracker-snapshots-i0k)
- [Checkpoint/Restore Systems in AI Agents (Eunomia)](https://eunomia.dev/blog/2025/05/11/checkpointrestore-systems-evolution-techniques-and-applications-in-ai-agents/)
- [SmolVM vs Firecracker vs Docker](https://particula.tech/blog/smolvm-vs-firecracker-sandbox-ai-generated-code)
