# Research: kitkit-cli — the agent-as-orchestrator-client pattern

- Query: kitkit-cli — concrete "agent acts as cross-session main process over a Stello/KitKit session forest via a Rust CLI"; the closest prior art to Ark
- Scope: internal (reference corpus, read-only)
- Date: 2026-06-25
- Source root: `reference/stello/kitkit-cli/`

## Findings

### 1. Mental model: Space (Kit) = one StelloAgent; CLI = orchestrator SDK exposed as shell commands

A **space** is the boundary for one conversation tree, one shared-memory store, and
one Kitty/main-agent context. Each space owns a forest of **sessions** (tree nodes).
The CLI is explicitly *not* ordinary session chat — it is global, cross-node,
orchestrator-facing work.

`reference/stello/kitkit-cli/src/cli.rs:8-13` (the `about`):
> "kitkit-cli is a command-line wrapper around KitKit's REST APIs for agents that need
> Kitty-like, space-level awareness. It intentionally focuses on global conversation-tree
> operations instead of ordinary session chat: list spaces, inspect a space topology,
> read a single session L2 digest, push insights, edit shared memory, and create forks."

`skills/kitkit-cli/SKILL.md:11-20`:
> "A space owns one conversation tree and one shared-memory store. A session is a node…
> `topology` shows the tree… `digest` reads one session's L2 view… `insight` writes one
> cross-session finding… `shared-memory` stores durable space-level context… `fork`
> creates a child session. Do not treat the CLI as ordinary chat."

**CLI command group → Stello SDK category map** (dispatch in `src/main.rs:28-39`):

| CLI command | SDK module / fn | REST path (from SDK) | SDK category |
| ----------- | --------------- | -------------------- | ------------ |
| `auth login/status/logout` | `auth` (local token cache) | — | identity / token store |
| `spaces list` | `spaces::list` | `GET /spaces` | space identity |
| `spaces get <SPACE_ID>` | `spaces::get` | `GET /spaces/{id}` | space metadata |
| `topology <SPACE_ID>` | `spaces::topology` | `GET /spaces/{id}/topology` | conversation tree |
| `digest <SPACE_ID> <SESSION_ID>` | `sessions::digest` | `GET /spaces/{id}/sessions/{sid}/digest` | per-session L2 |
| `insight put …` | `sessions::put_insight` | `PUT /spaces/{id}/sessions/{sid}/insight` | cross-session push |
| `shared-memory list` | `shared_memory::list` | `GET /spaces/{id}/shared-memory` | space-global memory |
| `shared-memory upsert` | `shared_memory::upsert` | `POST /spaces/{id}/shared-memory` | space-global memory |
| `shared-memory delete` | `shared_memory::delete` | `DELETE /spaces/{id}/shared-memory/{slug}` | space-global memory |
| `fork <SPACE_ID> <SRC> …` | `spaces::fork_session` | `POST /spaces/{id}/sessions/{sid}/fork` | topology mutation |

(SDK paths: `kitkit-sdk/src/spaces.rs:90-122`, `sessions.rs:39-73`, `shared_memory.rs:26-53`.)

The CLI is a thin clap → SDK adapter: every `run_*` builds an `authenticated_client`
then calls one SDK fn (`src/space.rs`, `src/session.rs`, `src/shared_memory.rs`). This
mirrors Ark's "ark-cli is a boring clap adapter; ark-core holds the logic" split.

### 2. The "Kitty" role — the application-layer reflection loop made concrete

`skills/kitkit-as-kitty/SKILL.md` instructs an external agent (Claude/Codex) to *act as*
the space-level main process. It composes the four lower-level skills (cli, tree, insight,
fork) and forbids local state when the CLI can read/write KitKit state (`:19`).

**Startup routine** (`kitkit-as-kitty/SKILL.md:22-40`):
1. `auth status` → 2. select space (`spaces list`) → 3. **read shared memory early**
(`shared-memory list`) → 4. load topology → 5. read most-relevant digests →
6. state what was inspected before drawing global conclusions.

**Global awareness loop** (`:86-94`) — the reflection cycle:
1. keep a compact map of important sessions + which digests were read;
2. re-read topology after forks / tree changes;
3. re-read shared memory on a new major task;
4. **push insight when cross-branch context should move immediately**;
5. fork when work needs a separate branch rather than a note;
6. keep evidence boundaries explicit (inspected digests vs. inferred state).

So the concrete loop is: **batch-read digests → reflect (find contradictions/stale
assumptions) → push a targeted insight OR upsert durable memory OR fork**. Shared memory
is treated as "durable but not automatically current" (`:42-50`) — the agent must check
topology/recent digests against it and warn the user when an entry looks stale.

### 3. Conversation-tree reading (topology + targeted digests)

`skills/kitkit-conversation-tree/SKILL.md` frames the tree as a *document library*: read
topology first, then read only the nodes you need — not every node.

Topology fields as navigation signals (`:19-25`): `id` (target for digest/insight/fork),
`label` (branch purpose), `status` (active can receive insight; archived is historical),
`turn_count` (activity/depth), `children` (local neighborhood). The SDK node is recursive:

`kitkit-sdk/src/spaces.rs:30-40`:
```rust
pub struct SessionTreeNode {
    pub id: String, pub label: String, pub status: SessionStatus,
    pub turn_count: u64,
    #[serde(default)] pub children: Vec<SessionTreeNode>,
    pub source_session_id: Option<String>,
}
```
Human rendering is an ASCII tree with `├──`/`└──` connectors and a per-node
`label (id) [status, N turns]` line (`src/space.rs:103-148`).

**Targeted digest workflow** (`:30-47`): read root/nearest-common-ancestor for project
background; read sibling branches before comparing decisions; read the target node before
pushing insight or forking from it; read high-turn/recently-active nodes for current state.
A digest is a compact **L2** view (metadata + memory + current insight) and is *not*
guaranteed to contain every L3 conversation record (`:38`).

### 4. Insight push workflow — when/how cross-session info propagates

`skills/kitkit-insight/SKILL.md`. Insight = **one slot per session**; a write replaces the
previous one (`src/cli.rs:82-83`). Server caps content at **4000 chars** (`cli.rs:159`,
`insight/SKILL.md:66`). It is for cross-branch findings, contradictions, warnings,
reusable conclusions for an active target — *not* chat, durable memory, or a scratchpad.

**Safety workflow** (`:23-33`) — the overwrite-protection ritual:
1. load/refresh topology; 2. choose the target *active* session; 3. `digest` it;
4. **read the current insight because the write replaces it**; 5. write only if the new
insight is more valuable than the current slot.

Recipient choice (`:69-75`): push to the session that can act; if a finding affects *all*
future work consider shared memory instead; never push to archived sessions; tailor each
insight per session if several need the same finding.

Response struct confirms the slot semantics — `PutInsightResponse { ok, session_id, label,
content_length }` (`kitkit-sdk/src/sessions.rs:30-37`); the CLI accepts content via
`--content`, `--content-file`, or `--stdin` (mutually exclusive, `src/cli.rs:178-198`).

### 5. Fork workflow + context-mode selection

`skills/kitkit-fork/SKILL.md`. A fork is a new topology node for separate exploration,
specialization, or handoff. **Note the brief said "none/inherit"; the actual code has
THREE context modes.**

`kitkit-sdk/src/spaces.rs:42-48` and `src/cli.rs:359-364`:
```rust
pub enum ForkContext { None, Inherit, Compress }   // serde rename_all = "lowercase"
```
Mode meanings (`src/cli.rs:286-292`, `fork/SKILL.md:33-45`):
- `--context compress` — summarized parent handoff; **the conservative default for most
  branch work**; KitKit generates the initial compressed handoff.
- `--context inherit` — carry exact parent context (more volume).
- `--context none` — clean start, no parent conversation context.

Pre-fork checklist (`:14-22`): load topology → identify source id → read source digest →
choose context mode → decide whether the visible topology parent differs from source →
decide profile/prompt/skills/initial-prompt.

Rich fork knobs (`ForkSessionRequest`, `spaces.rs:50-74`; args `src/cli.rs:270-357`):
`label`, `system_prompt`, `consolidate_prompt`, `compress_prompt`, `fork_compress_prompt`,
`skills: Vec<String>` (repeatable `--skill`), `prompt` (first user message), `context`,
`topology_parent_id` (place node elsewhere than the source), `profile` + repeatable
`--profile-var KEY=VALUE` (parsed by `parse_key_value`, `cli.rs:366-374`). Response carries
`{ id, parent_id, children, refs, depth, index, label }` (`spaces.rs:76-88`). Skill advises
re-reading topology after a fork when later steps need the new node id (`:83-87`).

### 6. Shared-memory commands — and the spec/code drift

`src/shared_memory.rs` + `kitkit-sdk/src/shared_memory.rs`. Shared memory is **global to the
space**, injected into Kitty's system context every turn (`src/cli.rs:90`), so it is for
durable facts (preferences, project background, long-lived goals, stable constraints) — not
one-branch state, transcripts, or cross-session findings (those go to insight).

**Confirmed drift (slug + body, NOT slug+summary+body):** both the wire struct and the
upsert request carry only two text fields — there is **no `summary` field** in this CLI/SDK.

`kitkit-sdk/src/shared_memory.rs:4-8` and `:15-19`:
```rust
pub struct SharedMemoryEntry      { pub slug: String, pub body: String }
pub struct UpsertSharedMemoryRequest { pub slug: String, pub body: String }
```
Human `list` renders a 2-column `slug | body` table (`src/shared_memory.rs:51-60`); `upsert`
echoes `slug` + `body` (`:62-71`). Body comes from `--body` / `--body-file` / `--stdin`
(`src/cli.rs:247-267`); slug max 128 chars, body must be non-empty (`cli.rs:215,243`).

**Tool-shape drift (recall vs edit):** there is **no `recall` / read-one-entry command** —
only `list` (read-all), `upsert`, `delete`. So the "recall vs edit" split named in the
Stello spec is collapsed here into list+upsert. HTTP-verb drift is also visible: the CLI
*help* calls upsert "edit"/"create or replace" (`cli.rs:214-216`) and insight is documented
as a write, but in the SDK **upsert is `POST`** (`shared_memory.rs:41`, `client.post_json`)
while **insight is `PUT`** (`sessions.rs:61`, `client.put_json`) — inconsistent verb choice
for two conceptually similar "overwrite a slot" operations. Delete is a `DELETE` treated as
idempotent (missing slug = no-op, `cli.rs:220`), and the CLI synthesizes its own
`DeleteSharedMemoryOutput { ok, space_id, slug }` since the SDK `delete` returns `()`
(`src/shared_memory.rs:12-18, 40-47`).

### 7. Tokens / auth / output — CLI-for-agents ergonomics

**Auth / tokens** (`README.md:56-90`, `src/cli.rs:104-137`): `auth login` is interactive by
default, or non-interactive via `--email` + `--password-stdin` (reads `$KITKIT_PASSWORD`
from stdin). Access + refresh tokens are cached in the platform config dir via the
`directories` crate (XDG / macOS App Support / Windows APPDATA), `0600` on Unix. Token is
scoped to the active `--base-url`. `auth status` validates + refreshes; `auth logout` clears
the local cache only. The HTTP client attaches the token as `bearer_auth`
(`kitkit-sdk/src/client.rs:114-121`); base URL defaults to `https://api.kitkit-agent.com`
(`client.rs:8`), overridable via `--base-url` / `$KITKIT_BASE_URL` (`cli.rs:22-30`).

**Output (JSON vs human) from typed SDK structs** (`src/output.rs`, `src/main.rs:18-24`):
a single global `--json` flag (`cli.rs:14-20`) selects the printer. `Output` is an enum
wrapping each typed SDK response (`output.rs:53-66`); a `Printer` trait has a `JsonPrinter`
(`serde_json::to_string_pretty`) and a per-type `ReadablePrinter` impl that builds
`comfy_table` tables. So **JSON is serialized directly from the strongly typed SDK response
structs** — there is no separate JSON schema to drift from the human view (`README.md:107`,
`cli.rs:16-19`). The CLI runs on a single-threaded tokio runtime (`main.rs:18`) and uses
`anyhow::Result` end-to-end. The API error envelope is normalized to
`KitKitError::Api { status, code, message }` with fallbacks (`client.rs:136-205`).

## Caveats / Not found

- No `recall` / get-one shared-memory-entry command exists in this CLI; only `list`,
  `upsert`, `delete`. If a Stello SPEC names a `recall` tool, it is not surfaced here.
- No `summary` field on shared-memory entries in this CLI/SDK — schema is `{slug, body}`.
  Could not cross-check against the server-side Stello SPEC (out of this corpus's scope).
- `put_json` / `delete_empty` SDK methods carry `#[allow(dead_code)]` (`client.rs:94,108`)
  even though the CLI calls them — likely a workspace-wide lint allowance, not a sign the
  paths are unused; insight (`PUT`) and delete (`DELETE`) do run via the CLI.
- `refs` on `ForkSessionResponse` (`spaces.rs:84`) and `source_session_id` on
  `SessionTreeNode` are deserialized but never rendered by the human printers; their exact
  semantics (cross-links?) are not documented in the CLI/skills.
- README marks the project WIP ("documentation refined" TODO, `README.md:24`); some `about`
  text ("edit shared memory") lags the actual POST/upsert behavior.

## Why this matters for Ark

- **Direct analogue.** kitkit-cli is a Rust CLI whose *primary user is an agent* acting as a
  cross-session "main" — exactly Ark's posture (`ark agent`, hidden, not-semver, driven by
  slash commands). It is the closest prior art for "an agent drives a memory/session system
  through typed CLI commands."
- **Read-state vs mutate-state split, typed.** kitkit separates read commands
  (`spaces`/`topology`/`digest`/`shared-memory list`) from mutations (`insight put`,
  `shared-memory upsert/delete`, `fork`) and ships JSON straight off SDK structs — the same
  shape as `ark context --format json` (read) vs `ark agent task …` (mutate). Validates Ark's
  "one `render(summary)` per dispatch, JSON from typed structs" convention.
- **The orchestrator skill IS the workflow doc.** `kitkit-as-kitty` encodes the
  reflection loop (read shared memory early → batch digests → reflect → push targeted
  insight/upsert/fork) in a *skill file the agent reads*, not in CLI code — mirroring how
  Ark pushes process into `.ark/workflow.md` + slash commands while keeping the binary
  mechanical. Memory for Ark could likewise be "agent reads digests/insights and decides,"
  with the CLI staying judgment-free.
- **Two memory tiers to steal.** Per-session **insight** (single overwritable slot, target
  the session that can act) vs space-global **shared memory** (durable facts, slug-keyed,
  treated as possibly-stale and re-validated against the tree). This is a clean model for an
  Ark "session memory" feature: ephemeral cross-task notes vs durable project-level memory,
  with explicit discipline on what belongs where.
- **Drift is a warning.** kitkit shows real spec/code drift (help says "edit", SDK does POST;
  insight PUT vs memory POST; no `recall`; `{slug,body}` not `{slug,summary,body}`). Ark's
  SPEC-actuator / verify discipline is the antidote — if Ark adds memory commands, pin the
  struct shape and HTTP/verb semantics in the SPEC and guard them, or the same drift appears.
