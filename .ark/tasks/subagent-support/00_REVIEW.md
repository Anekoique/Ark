# `subagent-support` REVIEW `00`

> Status: Closed
> Feature: `subagent-support`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 11

## Summary

The PLAN is structurally sound: Spec is Layout-A-shaped, every PRD Outcome is reflected in Goals/Constraints, and Trade-offs are concrete. Two issues need attention before EXECUTE: (1) **the install-vs-cleanup paths for Claude agents are not coherent** — `CLAUDE_PLATFORM.removal_root` is `.claude/commands/ark/`, so agents written to `.claude/agents/` would not be captured by `Layout::owned_dirs()`, removed by `ark remove`, or round-tripped by `unload`/`load`; (2) **the Codex agent file format (TOML with `name/description/tools/prompt` keys, "per Trellis precedent") is asserted as Constraint C-23 but is unverified** — Trellis's checked-in agents are `.md` with YAML frontmatter, not TOML, and Phase 1 step 4 itself hedges that the format "adapts" if Codex's runtime contract differs. Beyond those, several constraints are validated only by "code review" when a string-level template assertion would be straightforward.

---

## Findings

### R-001 Claude agent install path is outside `removal_root` and `owned_dirs`

- **Severity:** HIGH
- **Section:** `[**Architecture**]` (claim "owned_dirs unchanged — agents live under existing removal_root for each platform"); `[**Data Structure**]` (`CLAUDE_AGENTS_DIR = ".claude/agents"`); C-4 / C-24 / V-F-2 / V-F-3
- **Problem:** `CLAUDE_PLATFORM.removal_root = CLAUDE_COMMANDS_ARK_DIR = ".claude/commands/ark"` (verified in `crates/ark-core/src/platforms.rs:236-251`). The proposed Claude agents path is `.claude/agents/`, which is **not** under that `removal_root`. Likewise `Layout::owned_dirs()` returns `[ark_dir, claude_commands_ark_dir, codex_dir, opencode_dir]` (verified in `layout.rs:437-444`); it would not capture `.claude/agents/`. Consequence chain: (a) `ark remove` would not delete Claude agent files (C-24's "inherit existing membership semantics" claim is false for Claude); (b) `unload`/`load` would not snapshot/restore them (V-F-2 fails as written); (c) `is_installed` checks `manifest.files.starts_with(dest_dir)` where `dest_dir = ".claude"` — that part is OK, but `manifest.files` is populated from template extraction, and the proposed `apply_managed_state` block writes via `layout.resolve(dest).write_bytes()` without recording into `manifest.files`, so even `is_installed` may not register agent presence. Codex and OpenCode are unaffected (their `removal_root` is the platform root).
- **Why it matters:** The plan reuses Codex/OpenCode's whole-platform-root semantics but Claude was deliberately narrowed to `.claude/commands/ark/` to coexist with user-authored Claude artifacts. Three of the explicit constraints (C-4, C-24, T-4 mitigation) and two of the validation tests (V-F-2 round-trip, V-F-3 removal) silently depend on a property that does not hold.
- **Recommendation:** In the next PLAN iteration, choose one and document the choice in `## Log`:
  - **Option A:** widen Claude's surface — add `.claude/agents/` to `Layout::owned_dirs()` (5-tuple), and either expand `CLAUDE_PLATFORM.removal_root` to `.claude/` (changes existing remove semantics — needs CHANGELOG on `codex-support` SPEC's `removal_root` invariants) or introduce a separate `Platform::extra_dirs` field that `remove`/`owned_dirs` iterate;
  - **Option B:** keep `removal_root` narrow but add explicit handling: a `Platform::agents_dest_dir`-aware path that `unload`/`load`/`remove` walk in addition to `removal_root`. Either way, add a unit test exercising round-trip for Claude agents specifically (not just Codex/OpenCode where it works incidentally).

### R-002 Codex `.toml` agent format is unverified prior art

- **Severity:** HIGH
- **Section:** `[**Data Structure**]` Codex bullet; C-23; Implementation Phase 1 step 4
- **Problem:** PLAN states "Codex (`.codex/agents/<name>.toml`) — TOML with `name`, `description`, `tools`, `prompt` keys per Trellis precedent." The Trellis reference agents at `reference/Trellis/.claude/agents/*.md` are markdown with YAML frontmatter (`name:`, `description:`, `tools:` line), **not TOML**, and they live under `.claude/agents/`, not `.codex/agents/`. There is no `.codex/agents/` directory in the Trellis reference at all. PLAN Phase 1 step 4 itself hedges: "if Codex's runtime contract differs from Trellis's, the file format adapts but the agent prompt body stays unchanged" — yet C-23 fixes the format as a hard constraint. This is internally inconsistent and asserts a Codex feature (`.codex/agents/*.toml`) that may not exist (the existing `codex-support` SPEC explicitly says NG-2: "No `.codex/agents/*.toml` custom subagents", which the plan supersedes — but supersedes need to point at a real verified contract, not the same unverified one).
- **Why it matters:** If Codex does not support `.codex/agents/*.toml` subagents at all, the entire Codex slice of this feature is non-functional regardless of how good the prompts are. V-UT-1 / V-UT-6 only check that the `.toml` file is well-formed and present — they do not verify Codex actually loads or dispatches it.
- **Recommendation:** Before EXECUTE, dispatch researcher (or the author runs research manually) to confirm: (a) does Codex CLI support project-local custom subagents? (b) at what path? (c) in what file format? Then either: tighten C-23 to match the verified contract and cite the Codex doc URL in the SPEC; or, if Codex has no subagent surface, drop Codex from G-1 ("ship across all installed platforms") and adjust the PRD Outcome accordingly. Soften Phase 1 step 4's hedge to a hard reference once verified.

### R-003 Slash-command and agent-body constraints validated only by "code review"

- **Severity:** MEDIUM
- **Section:** `[**Validation**]` Acceptance Mapping (rows for C-7, C-8, C-9, C-10, C-11, C-12, C-13, C-15, C-16, C-17, C-18, C-19, C-20, C-25)
- **Problem:** Fourteen of the 25 constraints are mapped to "agent body inspection at code review" or "`/ark:design` body inspection at code review" or "SPEC CHANGELOG entries inspected at code review". Per the workflow's REVIEW guidance, code review is a weak validation; everything that can be a structural assertion should be. Examples that are trivially testable as unit tests:
  - C-7 / C-8 / C-9 — assert each agent body contains the literal Write-ALLOWED path strings (`.ark/tasks/<slug>/research/*.md`, `NN_REVIEW.md`, `VERIFY.md` respectively) and explicit Write-FORBIDDEN entries.
  - C-11 / C-12 — assert reviewer body contains the literal phrases "references prior iterations" and "contradicts an existing feature SPEC" + severity tags HIGH / CRITICAL.
  - C-13 — assert researcher body contains the literal "return … paths plus one-line summaries" / equivalent contract phrasing.
  - C-15 / C-16 / C-17 — assert `templates/claude/commands/ark/design.md` contains literal section headers naming `ark-researcher` / `ark-reviewer` / `ark-verifier` in the right steps.
  - C-18 / C-19 / C-20 — assert each SPEC body contains a `[**CHANGELOG**]` section with a date marker (string scan).
- **Why it matters:** These constraints will all silently rot the moment someone edits the templates without re-reading the PLAN. Cheap structural tests catch drift; manual review at commit catches none of it once the task is archived.
- **Recommendation:** Promote at least the agent-body and slash-command literal-string assertions into `templates.rs::tests` (or a new `agents_test.rs`). Keep "code review" only for prose-coherence judgments (e.g. "the trigger-signal paragraph reads naturally") that string-matching cannot capture.

### R-004 V-UT-8 contradicts C-22 on tool-name normalization

- **Severity:** MEDIUM
- **Section:** C-22 vs V-UT-8
- **Problem:** C-22 says "every agent prompt body is identical across platforms (modulo per-platform frontmatter and tool-name spellings)". V-UT-8 says "extract the prompt body from each platform's `ark-researcher` (similarly for reviewer, verifier); assert string equality after normalizing whitespace." Whitespace normalization does not normalize tool-name spellings (`Task` vs `task` vs whatever Codex uses). The test as written will fail when the bodies differ on a tool name; the test as documented is too strict to express C-22.
- **Why it matters:** Either the test fails on shipping (the bodies must differ on tool names) or the bodies are forced identical including tool names (then C-22's escape clause is unused and Codex/OpenCode invocation idioms regress). The PLAN doesn't pick.
- **Recommendation:** Either (a) factor the tool-name dispatch line into a per-platform-frontmatter or a per-platform substitution and keep V-UT-8 strict, or (b) document the substitution rule in V-UT-8 (e.g. "after normalizing whitespace and replacing tool-name tokens `Task | task | <codex-token>` with a single placeholder"). Prefer (a) if the dispatch is one line; (b) if it's threaded throughout.

### R-005 `Manifest::files` registration not specified for the new write path

- **Severity:** MEDIUM
- **Section:** `[**API Surface**]` `apply_managed_state` body sketch
- **Problem:** The proposed insertion writes via `layout.resolve(dest).join(entry.relative_path).write_bytes(entry.contents)?;` — note: this does not call into the existing template-extraction loop, so it does not register the written file in `manifest.files`. The existing `templates` field's extraction (separately done in `init.rs` / `upgrade.rs`) is what populates `manifest.files`. PLAN Architecture comment says `is_installed` "continues to work unchanged; agent files appear under each platform's `dest_dir` prefix and inherit the existing membership semantics" (C-24) — but they only inherit it if they are recorded in `manifest.files`. Without manifest registration, `is_installed` only sees `.claude/`-prefixed paths via the regular template tree, which masks the issue for Claude/OpenCode but means agent files aren't tracked individually.
- **Why it matters:** Round-trip semantics depend on `manifest.files` containing every Ark-managed path. Hash tracking is opt-out per C-4, but presence-tracking is not. Combined with R-001, this risks Claude agent files being neither captured by `unload` (not in `owned_dirs`) nor recorded in `manifest.files` — they become unmanaged on disk.
- **Recommendation:** Specify in the PLAN whether the new write path registers into `manifest.files` (mirroring the main `templates` extraction loop) or stays purely whole-file-write (mirroring `extra_files` semantics, where the file is not in `manifest.files` either). If the latter, also document how `unload`'s `walk_files(owned_dirs)` will reach `.claude/agents/` (per R-001).

### R-006 Goal G-2 is a procedure, not a capability

- **Severity:** LOW
- **Section:** `[**Goals**]` G-2
- **Problem:** Layout-A discipline (per `LAYOUT.md` and your existing feature SPECs) treats Goals as verb-led capabilities and Constraints as procedural rules. G-2 ("`Platform` registry gains optional `agents_templates` + `agents_dest_dir` so agent install reuses the iterate-the-slice pattern") is a description of an internal mechanism — closer in shape to a Constraint. G-1, G-3, G-4, G-5 are correctly user-facing. G-2 also half-overlaps with C-1, C-2, C-21.
- **Why it matters:** This SPEC promotes verbatim. Future readers will read G-2 as "the user-visible capability is registry growth", which is wrong. Goal sections that mix capabilities and procedures train the wrong instinct for the next deep task.
- **Recommendation:** Either (a) demote G-2 into the Constraint block (e.g. "C-26: `Platform` exposes `agents_templates` / `agents_dest_dir` as the install path; agent install reuses the iterate-the-slice pattern"), or (b) reword as a capability (e.g. "G-2: Adding subagents to a future platform is a registry entry, not a refactor."). Mirror the `codex-support` SPEC G-2 phrasing.

### R-007 No-recursion guard's failure mode is undocumented

- **Severity:** LOW
- **Section:** `[**Failure Flow**]` step 5; C-5 / C-14
- **Problem:** Failure-flow step 5 says: "Agent attempts to write outside its allowed paths → prompt-level refusal. The Recursion Guard and Write FORBIDDEN sections are the contract; out-of-scope writes are operator error caught at review." The guard is a markdown paragraph in the prompt. Nothing prevents an agent from ignoring it (LLM compliance is probabilistic). PLAN names this as an out-of-scope risk implicitly but does not document the *operator-side* mitigation (e.g. "if researcher writes outside `<task>/research/`, revert via `git restore` before incorporating its findings").
- **Why it matters:** A researcher that writes a `.rs` file under `crates/` and the main session doesn't notice ⇒ silent code injection on a research dispatch. The PRD Outcome says "Each agent enforces a tight scope wall via prompt"; a prompt is not enforcement, it is request. Acknowledging the gap and giving the operator a recovery procedure is honest; pretending the prompt is enforcement is not.
- **Recommendation:** Add to `## Runtime` a one-paragraph "scope-violation recovery": main session's expected response when an agent returns having written outside its allowed scope. Optionally add a Validation row that `git status` after agent dispatch is clean except for the documented allowed paths (manual smoke step in Phase 6).

### R-008 V-IT-3 (`upgrade_re_applies_modified_agents`) and C-4 collide with user-authored siblings (V-F-4)

- **Severity:** LOW
- **Section:** V-IT-3 vs V-F-4 vs C-4
- **Problem:** V-IT-3 asserts "modify a checked-in agent, run `ark upgrade`, assert the file is restored to the embedded canonical body". V-F-4 asserts a user file `.claude/agents/my-agent.md` survives. These are consistent (different filenames). But `V-E-4` asserts a user-authored file *named the same as an Ark agent* (`.claude/agents/ark-researcher.md`) gets overwritten with the canonical body — also consistent with C-4 ("user-authored siblings under `<agents_dest_dir>/` are preserved" — siblings, not collisions). Documenting "user must rename their override" is a UX cliff that the PLAN doesn't elsewhere acknowledge — a user who hand-installs their own `ark-researcher.md` to customize behavior loses it silently on `ark upgrade`.
- **Why it matters:** Naming-collision semantics are a user-facing invariant. The PLAN currently buries it in V-E-4. If this is intentional, it deserves a `Constraint` (e.g. C-26: "agent filenames `ark-{researcher,reviewer,verifier}.{md,toml}` are reserved by Ark; user-authored siblings with the same stem are overwritten on `init`/`upgrade`/`load`"). If unintentional, consider an interactive prompt or a `--force` gate before overwrite.
- **Recommendation:** Promote the V-E-4 behavior into an explicit constraint and document it once in the user-facing slash-command guide (the design.md edits). Keeps V-E-4 honest as a regression test rather than a documentation site.

### R-009 V-F-2 and V-F-3 over-claim coverage given R-001

- **Severity:** LOW
- **Section:** Acceptance Mapping (G-5 row) and V-F-2 / V-F-3
- **Problem:** G-5 row maps to "manual smoke + V-F-2 (research/ travels with task in snapshot/restore)" — but `<task>/research/` is *under* `.ark/tasks/<slug>/`, which IS in `owned_dirs`, so this part is fine. V-F-2 also claims agent files round-trip — that's the part R-001 breaks for Claude. Once R-001 is resolved, V-F-2 should explicitly assert one Claude agent path (not just "agent files").
- **Why it matters:** Test names that promise too much paper over real gaps. Reading the test list as PLAN reviewer should reveal coverage; here it doesn't until you know the platform-specific layout.
- **Recommendation:** Split V-F-2 into `unload_load_round_trips_claude_agent_files`, `unload_load_round_trips_codex_agent_files`, `unload_load_round_trips_opencode_agent_files`; same for V-F-3.

### R-010 `Researcher returns NoFocus`-style error-shape unverified against current `state.toml` API

- **Severity:** LOW
- **Section:** `[**State Transitions**]` first bullet
- **Problem:** "Researcher dispatched while no task focused → returns `NoFocus`-shaped error" — the prompt-level agent doesn't return Rust errors; it returns a string. The `NoFocus` shape lives in `ark-core::error::Error` (per ERRORS.md E-2). The PLAN should clarify whether the agent emits the literal string `NoFocus` (matching the error variant), a free-form prose explanation, or runs `ark agent task resume` itself (it can't — that's a write, see C-7 forbidding everything outside `<task>/research/`).
- **Why it matters:** The state-transition bullet conflates Rust error shapes with prompt-level agent output. Future readers may assume `ark context` itself returns `NoFocus` from the researcher dispatch path, which it doesn't.
- **Recommendation:** Reword to "Researcher detects no focus via `ark context` (which surfaces `NoFocus`); researcher returns a textual instruction to the main session naming the recovery command. No literal error variant is round-tripped."

### R-011 No documented `ark` CLI version-bump policy parallel to codex-support's `0.2.0 → 0.3.0` deprecation

- **Severity:** LOW
- **Section:** `[**Trade-offs**]` (missing trade-off)
- **Problem:** `codex-support` SPEC carries an explicit version-bump policy (`#[deprecated(since = "0.2.0", ...)]` thin wrappers, removed at 0.3.0). This task adds `Platform.agents_templates` / `agents_dest_dir` fields — a pub-struct shape change. PLAN does not mention semver impact. `Platform` is re-exported from `lib.rs`, so the addition is at minimum a minor-version bump for `ark-core`, and any downstream consumer of `Platform` literals (test code, documentation snippets) needs updating.
- **Why it matters:** The codex-support SPEC normalized version-aware deprecation. This task introduces breaking-shape addition silently.
- **Recommendation:** Add a Trade-off entry "T-9: ark-core public-API impact." Note: the addition is additive (new optional fields default to None for old struct literals — except struct literals require *all* fields, so this IS breaking). Either bump the minor version with a note in the codex-support / opencode-support SPEC CHANGELOGs, or use the builder pattern / `#[non_exhaustive]` on `Platform` to keep additions non-breaking going forward.

---

## Trade-off Advice

### TR-1 Reuse `extra_files` vs new `agents_templates` field

- **Related Plan Item:** T-4
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (new fields) — but with caveats
- **Advice:** Adopt T-4's choice (new fields) is correct given `extra_files` is a flat `&[(&str, &str)]` of whole-file constants generated by `include_str!`. Embedding three agent files per platform via `include_str!` would inflate the registry and force a `static` per file. The new `Option<&'static Dir>` matches the existing `templates` field shape and lets `walk` traverse subdirectories. **However**, the PLAN should answer: why introduce `agents_templates` instead of *making `templates` accept a vec of `(Dir, dest)` pairs*? That alternative would also handle Claude's split-root problem (R-001) by attaching `.claude/agents/` as a second template-tree-with-dest pair. The current `templates` field is hard-coded to extract under the *single* `dest_dir`.
- **Rationale:** The clean-design alternative (vec of trees) makes Claude's narrow `removal_root` workable: `.claude/commands/ark/` AND `.claude/agents/` as two extracted-tree pairs, both registered into `manifest.files`, both round-trippable. The current proposal is a one-off field that addresses the immediate case but leaves the next "I need two trees per platform" feature in the same dead end.
- **Required Action:** Expand comparison — in iteration 01, contrast `Option<&Dir>` (current) vs `&[(&Dir, &str)]` (vec of trees). Pick one with reasoning; the latter resolves R-001 implicitly.

### TR-2 Single design.md vs split commands for REVIEW/VERIFY

- **Related Plan Item:** T-6
- **Topic:** Flexibility vs Simplicity
- **Reviewer Position:** Prefer A (single design.md) for now
- **Advice:** Keep T-6's choice. Re-running the verifier after fixes is rare enough in standard tier (no agent dispatch) that the friction is bounded. If the verifier is dispatched 3+ times in deep tasks, revisit with `/ark:verify` as a separate slash command in a follow-up task; do not pre-build the surface.
- **Rationale:** Adding `/ark:review` and `/ark:verify` now duplicates dispatch instructions across three slash commands per platform = nine new files. T-6's argument (single source of truth) holds.
- **Required Action:** Keep with clarification — add to T-6: "if verifier is dispatched manually outside `/ark:design` (e.g. after EXECUTE fixes), main session can re-dispatch directly via the platform's subagent tool; the slash command is not a gate."

### TR-3 Researcher findings checked-in vs gitignored

- **Related Plan Item:** T-5
- **Topic:** Compatibility vs Simplicity
- **Reviewer Position:** Prefer A (checked-in)
- **Advice:** Adopt T-5's choice. Aligns with existing Ark behavior — PRDs, PLANs, REVIEWs are all checked in.
- **Rationale:** Archive value > git-status noise. Plus the directory is bounded in size (markdown only).
- **Required Action:** Adopt — no PLAN change needed.
