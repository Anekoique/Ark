# Structured Summaries and Projections

The pattern: deliver a *curated, schemaed, versioned snapshot* of project state at session start, rather than asking the agent to discover state via tools. Ark's `ark context` is the most mature concrete instance; spec-kit, OpenSpec, Continue Hub all converge here.

## The pattern named

A "session-orientation packet" is:

1. **Curated** — a known, finite slice of state. Not a dump.
2. **Schemaed** — defined fields, predictable shape, machine-parseable.
3. **Versioned** — schema has a version number; old agents can degrade gracefully on new fields.
4. **Delivered once per session** (or per phase) — not retrieved per turn.

The mechanism is usually a CLI command or a host-platform hook that injects the output at the start of the conversation.

## Why projections beat raw dumps

A repo's full state ("everything in `.git/`, every file, every config") is too large to load and 99% irrelevant per session. A projection answers "for this session, what is load-bearing?" with:

- Git state (branch, recent commits, dirty files) — needed for "where are we"
- Active tasks (slug, tier, phase, artifacts) — needed for "what are we doing"
- Project SPECs (paths + scopes) — needed for "what conventions apply"
- Feature SPECs (paths + scopes) — needed for "what's been built nearby"
- Recent archive — needed for "what just shipped"

That's ~3K tokens, well under any model's effective window, and it answers "where, what, why" in one shot. Compare to the agent doing 20 tool calls to assemble the same picture each session.

## Ark's `ark context` — anatomy

Ark's projection is the most mature instance in the field. From `crates/ark-core/src/commands/context/`:

- **Scope variants:** `session` (default), `phase --for <X>`, `record` (workspace journals).
- **Format variants:** `text` (default, human-rendered), `json` (machine-readable, schema-stable).
- **Schema versioning:** `SCHEMA_VERSION` constant; consumers gate on it.
- **Phase filters:** `Design`, `Plan`, `Review`, `Execute`, `Verify`, `Commit` — different fields surface per phase (design pulls related SPECs; verify pulls VERIFY.md; commit returns paths-only since the body is sourced from VERIFY.md).
- **Delivered via:** SessionStart hook on Claude/Codex; chat-message plugin on OpenCode.

A `--scope session --format json` produces ~2-5K tokens. A `--scope phase --for verify --format json` is similar size but field-shifted.

Cited from the user's session-start: schema=1, scope=session, generated_at=timestamp, project_root, git={branch, head_short, is_clean, dirty_files, recent_commits}, tasks={active}, specs={project, features, features_warnings}, archive={recent}.

## Compared peers

### spec-kit (GitHub)

spec-kit ships a "spec stack" concept — Constitution + Specs + Plans + Tasks as four committed artifacts. At session start, the user (or a manual hook) feeds the stack into the agent's context.

**Compared to Ark:** spec-kit is *committed artifacts* (you read them as files); Ark's `ark context` is a *projection over committed state* (it summarises files into JSON). spec-kit's stack is per-project; Ark's projection is per-phase.

### OpenSpec (Fission-AI)

OpenSpec delivers proposals + changes + tasks via the `openspec` CLI. Similar shape — read state, emit a structured snapshot. JSON output exists but is less prominent than spec-kit's markdown-first approach.

### Continue Hub

Continue's 2026 Hub is a registry of configurations + skills that can be loaded into Continue at session start. Closer to a *package manager* for context than a *projection* — but the delivered payload (rules + skills + commands) plays the same role.

### Cursor `.cursor/rules/`

The `.cursor/rules/<name>.mdc` files with frontmatter (`alwaysApply` / `globs` / `description`) — Cursor's session-orientation. Frontmatter-driven conditional loading is a refinement of "always-loaded" with selectors.

### Claude Code SessionStart hook

The mechanism Ark uses on Claude. The hook runs at session start, output goes to the agent's context. Ark's hook calls `ark context`. The hook is the *delivery channel*; the projection is the *payload*.

### Codex `SessionStart` event

Same idea, slightly different schema (TOML config for the hook, seconds-based timeouts). Ark's per-platform registry handles the delta.

## Schema design lessons

From `ark context` and the peers:

1. **Stable top-level shape.** Even when sub-fields change, top-level keys (git, tasks, specs, archive) should be invariant. Old consumers parse without breaking.
2. **Forward-compatible extension.** New optional fields land without bumping schema. Bumping the schema number is the *last* tool, not the first.
3. **Human + machine readable.** Both `text` and `json` formats; render once, both render paths consume the same struct.
4. **Phase- or scope-conditional fields.** Not every field belongs in every projection — `verify_pending_counts` is only relevant in `--for verify`. Surface it conditionally.
5. **Generated-at timestamp.** Lets downstream tools cache and detect staleness.

Ark gets all five. spec-kit gets 1, 2, 3; phase-specific projection is less developed. OpenSpec gets 1, 2, 3, 5; less phase awareness. Cursor rules get 1, 3; the rest are out of scope (it's a static-file model, not a projection).

## When projections fail

1. **Stale generation.** The projection is sampled at one moment; if state changes mid-session, the agent operates on stale info. Mitigated by re-invoking the hook (Claude Code's `/context` slash command) or by refreshing in a tool call.
2. **Schema drift between client and server.** A new field appears; consumer breaks. Mitigated by additive-only changes + schema versioning.
3. **Payload size growth.** As tasks accumulate, projection grows. The 30+ archived task list in Ark's session context is already a smell — most are irrelevant per session. Mitigated by capping (Ark shows top-5 archive); should probably cap active tasks too.
4. **Wrong granularity per phase.** `--scope phase --for review` should include the latest plan + previous review; the current implementation hands back paths. Whether that's the right granularity is a per-phase design call.
5. **Cross-platform delivery variance.** Claude/Codex inject via hook; OpenCode injects via plugin. Codex's seconds-based timeout has bitten Ark before (`02_infra_primitives/hooks-and-lifecycle-events.md`).

## The opportunity Ark is uniquely positioned for

Most peers ship projections as *adjacent helpers* — Cursor rules are static files, spec-kit reads files at session start. Ark ships the projection as *the primary surface* — `ark context` is the canonical entry point; everything else (slash commands, agents) consumes it.

That makes `ark context` the right place to:
- Add tool-budget surfaces (`tool_calls_remaining` per phase).
- Add codemap pointers (`suggested_reads` per phase).
- Add cost-awareness signals (`token_budget_for_phase`).
- Add cross-platform schema (run `ark context` from Claude/Codex/OpenCode/CI alike).

It also makes the JSON schema stability commitment load-bearing: external tools (CI scripts, IDE plugins, sub-agents) can depend on it, and that dependency is a moat.

## Directions for Ark

1. **Promote `ark context --format json` as a public schema with semver promises.** The CLI already has it; documenting the schema in `docs/book/src/reference/context-schema.md` would let third-party tools depend on it.

2. **Cap active-tasks list in projection output.** Currently `tasks.active` returns everything; on busy projects this drowns the prompt. A `--limit-tasks N` flag with sensible default (top 3 + count of others) helps.

3. **Add a `--scope codemap` variant.** Once `docs/CODEMAPS/` exists (see `codemaps-and-repo-structure-summaries.md`), surface the slice relevant to the current task in `ark context`.

4. **Surface phase-specific guidance fields.** For `--for plan`, include `suggested_template_section_order`; for `--for review`, include `prior_finding_count`. Fields agents can act on.

5. **Document the projection pattern as a teachable harness primitive.** Ark embodies it well; the field is converging on it. A short blog/doc explaining "what `ark context` is and why your harness should ship one" extends Ark's intellectual footprint.
