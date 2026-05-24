# `improve-ark-context` REVIEW `00`

> Status: Open
> Feature: `ark-context`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 7

## Summary

The PLAN delivers every PRD outcome and keeps `SCHEMA_VERSION = 1` via `#[serde(skip_serializing_if = ...)]`-gated additive fields; every prior `G-1..G-5`, `NG-1..NG-3`, and `C-1..C-29` is preserved verbatim in the new `## Spec`, with `C-30..C-43` and `NG-4` layered additively per the PRD. The placement matrix in C-30 lines up cleanly with the slash-command consumption sites that motivate each field. Two HIGH items require fixes before EXECUTE: C-41/V-UT-9 conflate the Codex *slash-command* directory layout with the Codex *agent* directory layout (`.codex/agents/` ships flat `.toml` files, not `<name>/SKILL.md` subdirectories), and C-38's enumerated tag set (`"claude"`) does not match `Platform::id` (which is `"claude-code"` for the Claude platform). Several MEDIUM/LOW items are scope-and-clarity nits.

---

## Findings

### R-001 `C-41 and V-UT-9 use the wrong Codex agent layout`

- **Severity:** HIGH
- **Section:** `## Spec → [**Constraints**] → C-41`; `## Validation → V-UT-9`
- **Problem:** C-41 states: "Stem derivation per platform: Claude / OpenCode = filename with trailing `.md` stripped; Codex = subdirectory name (stem of the directory containing `SKILL.md`)." V-UT-9 codifies the same shape: "`.codex/skills/ark-reviewer/SKILL.md` → stem `ark-reviewer`". This is the Codex *skill* (slash-command) layout, not the Codex *agent* layout. Per `subagent-support` SPEC's per-platform contract — "Codex (`.codex/agents/<name>.toml`) — TOML with `name`, `description`, ..." — and per the on-disk template tree (`templates/codex/agents/ark-{researcher,reviewer,verifier}.toml`, sibling to `templates/codex/skills/ark-design/SKILL.md`), Codex agents are flat `.toml` files under `.codex/agents/`. `enumerate_subagents` reads `Platform.agents_dest_dir`, which for Codex resolves to `.codex/agents/` (`CODEX_AGENTS_DIR` per `layout.rs`), and there is no `SKILL.md` underneath.
- **Why it matters:** Implemented as written, the Codex arm of `enumerate_subagents` would never return any stems (no `SKILL.md` files exist under `.codex/agents/`), and the validation V-UT-9 would assert against a layout that does not exist on disk. Either the test fixture is wrong or the implementation is wrong; both reading sites disagree with the actual platform layout the feature relies on.
- **Recommendation:** Rewrite C-41 to: "Stem derivation per platform: Claude = filename with `.md` stripped; OpenCode = filename with `.md` stripped; Codex = filename with `.toml` stripped. All three platforms expose flat agent files under `agents_dest_dir`; only `.codex/skills/` (slash commands, out of scope here) use the subdirectory + `SKILL.md` layout." Rewrite V-UT-9 to: "`enumerate_subagents` derives Codex stems from `.toml` filenames (`.codex/agents/ark-reviewer.toml` → stem `ark-reviewer`)." The Acceptance Mapping row for C-41 stands once both citations are corrected.

### R-002 `C-38 SubagentSet.platform tag does not match Platform::id`

- **Severity:** HIGH
- **Section:** `## Spec → [**Constraints**] → C-38`
- **Problem:** C-38 says: `SubagentSet.platform` is one of `"claude" | "codex" | "opencode"` — lowercase serde tag matches `templates::Platform`'s existing display form. The actual `Platform::id` literals in `crates/ark-core/src/platforms.rs` are `"claude-code"`, `"codex"`, `"opencode"` (lines 302 / 335 / 365). The enumerated tag for Claude (`"claude"`) therefore does not match `Platform::id` for Claude (`"claude-code"`).
- **Why it matters:** Two readings of the constraint are both broken: (a) if `SubagentSet.platform` is supposed to mirror `Platform.id` verbatim, the literal `"claude"` is wrong and slash commands keying off the tag will miss every Claude row; (b) if `SubagentSet.platform` is supposed to be a separate normalized vocabulary, then the "matches `templates::Platform`'s existing display form" clause is false. Either way the constraint as written is internally inconsistent and unimplementable.
- **Recommendation:** Pick one source of truth and align the constraint and Data Structure example to it. Two reasonable options: (a) reuse `Platform::id` verbatim — change the enumerated set in C-38 to `"claude-code" | "codex" | "opencode"`, drop the "lowercase serde tag" claim; or (b) introduce a derived short tag (`"claude" | "codex" | "opencode"`) computed by `enumerate_subagents` from `Platform::id`, and state the mapping rule explicitly (e.g. "first path segment of `agents_dest_dir` minus the leading dot"). Option (a) is the simpler choice and removes the need for a derivation rule.

### R-003 `V-F-3 envelope cap ceiling cites an undocumented number`

- **Severity:** MEDIUM
- **Section:** `## Validation → V-F-3`
- **Problem:** V-F-3 reads: "SessionStart envelope cap drops `archive` first; with `checkout` + `subagents` populated, the cap math still fits the documented 9,500-byte ceiling for a typical project (≤10 features, ≤6 stems per platform)." The 9,500-byte ceiling is not stated in the prior `ark-context` SPEC's body or in any other related SPEC reviewed. C-43 ("envelope cap behavior unchanged") asserts behavior preservation but does not name a byte budget.
- **Why it matters:** A test asserting against a magic number that is not documented in any SPEC is brittle and untraceable for the next reviewer. Either the ceiling is real (and should be elevated to a Constraint with its provenance — Anthropic API limits, prior measurement, etc.) or the test should assert "envelope size with new fields populated is within whatever ceiling the existing envelope-truncation code uses" by referencing the existing constant in code.
- **Recommendation:** Either (a) add a Constraint codifying the byte budget and its source (e.g. `C-44: SessionStart envelope cap is N bytes, sourced from <existing constant in code>`), or (b) rephrase V-F-3 to reference the in-code constant rather than the prose ceiling: "with the existing envelope-cap constant in `mod.rs`, an envelope with `checkout` + `subagents` populated for a fixture of ≤10 features and ≤6 stems per platform stays under the cap; `archive` is dropped first per C-43."

### R-004 `V-UT-1 / V-IT-1 worktree fixtures need invocation-context detail`

- **Severity:** MEDIUM
- **Section:** `## Validation → V-UT-1`, `V-IT-1`
- **Problem:** V-UT-1 reads: "`detect_checkout` returns `Main` in a freshly-init'd tempdir + `Worktree` when invoked from inside `.ark/worktrees/<branch>/`." But the call signature is `detect_checkout(&Layout)` and `Layout::root` is fixed at construction time; the assertion needs to clarify which `Layout` instance is passed — one built from the parent tempdir or one built from the worktree subdir. Without that, the test is ambiguous. V-IT-1 has a similar ambiguity: it asserts `root_kind: "main"` in a "tempdir with seeded `.claude/agents/`" but does not specify whether the tempdir was made a git worktree or just a plain dir.
- **Why it matters:** Implementation will pick whichever interpretation reads first. C-31 says detection failure (non-git) defaults to `Main`, so a plain tempdir is `Main` for a non-git reason, not for a "this is the parent checkout" reason — that conflates two distinct paths through `detect_checkout`. The unit test should exercise both Main-because-non-git and Main-because-parent-checkout independently.
- **Recommendation:** Split V-UT-1 into two cases: "Main when `Layout::root` points at a git repo's main checkout (single `git rev-parse --show-toplevel == git rev-parse --git-common-dir`'s parent)" and "Worktree when `Layout::root` points at a worktree created via `git worktree add` and `--show-toplevel` differs from the parent". Leave the V-UT-2 non-git default-Main as-is. V-IT-1 should explicitly note its tempdir is initialized as a plain git repo (so the Main branch is the "main checkout" case, not the "non-git fallback" case).

### R-005 `C-30 commit-scope record placement vs. RecordProjection default values`

- **Severity:** MEDIUM
- **Section:** `## Spec → [**Constraints**] → C-30`, `C-42`; `## Validation → V-IT-3`
- **Problem:** C-30 says `record` is populated on `Scope::Record` and `Phase(Commit)`. C-42 says commit-scope `record` is populated by reusing "the same record-gather helper that powers `Scope::Record`". The current `Scope::Record` arm in `projection.rs` always emits `record: Some(RecordProjection::default())`. V-IT-3 asserts that when no journal exists, the projection emits `"record": { "identity": null, "active_journal_path": null, "session_count": 0, ... }` — i.e. `Some(RecordProjection::default())`, not `None`. That is internally consistent, but it leaves no observable difference between "no journal yet" and "record-gather not invoked"; a slash command cannot tell the two apart.
- **Why it matters:** If commit-scope's `record` is `Some(RecordProjection::default())` in the steady state (e.g. the journal hasn't been written for this session yet), `/ark:commit` reading `record.session_count == 0` cannot tell whether the workspace was uninitialized or whether `gather` skipped the record helper. The slash command's behavior on those two states is plausibly different.
- **Recommendation:** Either (a) keep `Some(_)` semantics and document explicitly in C-42 that "`session_count == 0` indicates no journal entries yet for this slug/branch, not record-gather skip"; or (b) tighten the C-30 placement to "`record` is `Some(_)` on `Scope::Record` and on `Phase(Commit)` *when* identity is resolvable (i.e. `.ark/.developer` exists); `None` otherwise" so slash commands can branch on the option. Pick one and surface the choice in `## Trade-offs`.

### R-006 `[**Architecture**] module map drops the existing platforms.rs description`

- **Severity:** LOW
- **Section:** `## Spec → [**Architecture**]` (module-map block)
- **Problem:** The new architecture map adds `platforms.rs` with the parenthetical `(Platform::agents_dest_dir is the input subagent enumeration reads from)`. This wording reads as a refactor target ("does this PLAN modify platforms.rs?"); reviewers may infer modification. The PLAN's intent (per `enumerate_subagents` body) is read-only consumption.
- **Why it matters:** The clarifying parenthetical accidentally suggests `commands/context/` may mutate `platforms.rs`. Per `subagent-support` SPEC, `Platform.agents_dest_dir` is fixed at the registry; consumers read only.
- **Recommendation:** Adjust the line to read e.g. `platforms.rs (read-only consumer; Platform::agents_dest_dir is the input subagent enumeration scans)` to disambiguate.

### R-007 `C-39 SubagentSet.stems silently drops non-platform-extension entries`

- **Severity:** LOW
- **Section:** `## Spec → [**Constraints**] → C-39`, `C-41`; `## Validation → V-E-5`
- **Problem:** V-E-5 documents the behavior ("A platform whose `agents_dest_dir` contains a file with no recognized stem suffix (Claude: not `.md`) is skipped without erroring."), but C-39 and C-41 do not state this filter. A reader of the constraints alone would infer that every direntry yields a stem.
- **Why it matters:** Future contributors writing tests or fixtures need to know the silent-skip rule lives in `enumerate_subagents`. A constraint that an edge-case test enforces but no constraint mentions is hard to navigate.
- **Recommendation:** Add a one-clause extension to C-41: "Files whose extension does not match the platform's expected extension (`.md` for Claude/OpenCode, `.toml` for Codex) are skipped silently." This makes the V-E-5 contract explicit at the constraint layer.

---

## Trade-off Advice

### TR-1 `Additive vs schema-bump`

- **Related Plan Item:** `T-1`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (Adopt as proposed)
- **Advice:** Keep the additive approach; do not bump `SCHEMA_VERSION`.
- **Rationale:** Every new field is serde-gated with `#[serde(skip_serializing_if = ...)]`; consumers ignore unknown JSON fields by default. `ark-context` SPEC's stated additive-only contract (C-6) explicitly permits this; C-30 codifies exactly when each new field appears, removing the "shape varies inscrutably" downside.
- **Required Action:** Adopt.

### TR-2 `features_tree placement — session+design only`

- **Related Plan Item:** `T-2`
- **Topic:** Performance vs Information Surface
- **Reviewer Position:** Prefer A (Adopt as proposed)
- **Advice:** Keep `features_tree` on Session and Design only.
- **Rationale:** Plan/Review/Execute/Verify slash commands consume the projected feature list, not the whole tree; commit scope is body-free per G-5. Adding the tree to those scopes is dead weight per the consumption sites the PRD enumerates.
- **Required Action:** Adopt.

### TR-3 `Subagent detection — manifest vs filesystem scan`

- **Related Plan Item:** `T-3`
- **Topic:** Flexibility vs Safety
- **Reviewer Position:** Prefer A (Adopt as proposed, with R-001 + R-007 fixes)
- **Advice:** Keep the filesystem scan over manifest lookup.
- **Rationale:** The slash command's "which reviewer is installed?" prompt needs ground truth (user-installed agents and user-removed Ark canonicals both happen). Manifest lookup would lie when the user has hand-edited `.claude/agents/`. The robustness cost (per-platform stem derivation, symlink skip) is small once R-001 fixes the Codex stem rule and R-007 documents the silent-skip filter at the constraint layer.
- **Required Action:** Adopt with the two fixes above.

### TR-4 `Commit-scope record — reuse RecordProjection vs inline minimal fields`

- **Related Plan Item:** `T-4`
- **Topic:** Compatibility vs Footprint
- **Reviewer Position:** Prefer A (Adopt as proposed)
- **Advice:** Reuse `RecordProjection`.
- **Rationale:** Single rendering helper, single test surface, byte-for-byte parity between `Scope::Record` and `Phase(Commit)` makes it trivial to verify (V-IT-3 already asserts byte-for-byte match). The ~80-byte payload bump is negligible compared to the simplification.
- **Required Action:** Adopt; resolve R-005 as part of the wording cleanup.

### TR-5 `No --for research`

- **Related Plan Item:** `T-5`
- **Topic:** Forward Compatibility vs Surface Area Discipline
- **Reviewer Position:** Prefer A (Adopt as proposed)
- **Advice:** Do not add `PhaseFilter::Research`.
- **Rationale:** `ark-research` SPEC NG-4 is explicit; reusing the design projection for research tasks aligns with `/ark:research`'s actual `--for design` call. Bumping the projection surface for a tier that already lives without PLAN/REVIEW/VERIFY would be premature.
- **Required Action:** Adopt.

### TR-6 `In-place SPEC update vs new SPEC`

- **Related Plan Item:** `T-6`
- **Topic:** History Preservation vs Atomic Promotion
- **Reviewer Position:** Prefer A (Adopt as proposed)
- **Advice:** Update `specs/features/ark-context/SPEC.md` in place; append CHANGELOG.
- **Rationale:** The `detachable-feature-spec` C-7 path already handles overwrite-with-CHANGELOG; a new SPEC would fragment the history of the same feature across two directories with no offsetting benefit. The PLAN's `## Spec` is a verifiable superset of the prior body (every prior `G-*` / `NG-*` / `C-*` is present), which is the only precondition for safe overwrite.
- **Required Action:** Adopt.
