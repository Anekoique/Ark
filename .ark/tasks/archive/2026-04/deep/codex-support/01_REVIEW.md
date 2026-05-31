# `codex-support` REVIEW `01`

> Status: Closed
> Feature: `codex-support`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 4



## Summary

Iteration 01 lands every prior finding cleanly. The four blockers from 00 are resolved with concrete, testable mechanisms: `timeout: 30` (seconds) is in `ark_codex_hook_entry()` with a doc comment naming the unit (R-001 → C-25 / V-UT-10); `load`'s canonical re-apply phase iterates `PLATFORMS` after replaying `snapshot.hook_bodies`, and V-IT-16 hand-crafts a stale-entry snapshot to lock the invariant (R-002 → C-22); the JSON Pointer parser is gone, replaced by `hooks_array_key: &str` plus a charset validator, and V-UT-5/V-UT-6 are explicitly retired (R-003 → C-19 revised); the source-scan invariant now spans all five command files plus `platforms.rs` (R-004 → C-18 revised). The five MEDIUM-tier findings are addressed with the same level of rigor: `#[deprecated]` thin-wrapper functions in lieu of `pub use` (R-005 → C-23); the round-trip-lossy gap closed via Stage B's owned-dir-scoped JSON scan (R-006 → C-24 / V-IT-17); non-TTY no-flags errors instead of installing both (R-007 → G-3 / V-IT-11); G-14 codifies the Claude-only-stays-Claude-only invariant on upgrade and V-IT-15 enforces (R-008).

The biggest reshape — switching `.codex/prompts/` to `.codex/skills/` because the former is an open feature request (openai/codex#9848) — is correct on the facts and well-scoped. The G-12 parity test now also enforces body content modulo a documented rewrite, which is stronger than 00's existence-only check. Trade-off T-8 is added with a clear recommendation.

Four LOW findings remain, none blocking: an under-specified edge case in `derive_array_key` for malformed historical snapshots; an internal inconsistency between the PRD's "byte-identical round-trip" and C-22's canonical re-apply (cross-version only); a small risk that the G-12 content-parity rewrite mechanism catches a divergence the executor would prefer to allow; and one wording tightening for V-IT-17's owned-dirs scope.

The verdict is **Approved**. The plan is implementable as written; the four LOW findings can be folded into EXECUTE without re-running review.



## Findings

### R-101 `derive_array_key error path is unspecified for malformed snapshot pointers`

- Severity: LOW
- Section: `[**Call graph for `load`]` step "replay phase"; `[**Main Flow — `load`]` step 4.
- Problem:
  Step 4 of `load` calls `update_hook_file(body.path, body.entry, derive_array_key(body.json_pointer), body.identity_key)` and the PLAN narrates the derivation as `path.rsplit('/').next()`. For canonical snapshots (`"/hooks/SessionStart"`), this yields `Some("SessionStart")` — fine. But the PLAN does not say what happens for a malformed/empty pointer. `"/hooks/"` → `Some("")` (empty), which the new C-19 validator (`[A-Za-z0-9_-]+`) would reject with `Error::Io { source: "invalid hooks array key" }`, halting `load` mid-way and leaving the project in an inconsistent state (snapshot files restored, hook bodies partially applied). A pathological/handcrafted snapshot is realistic given the existing test in `load.rs` (`load_rejects_snapshot_with_absolute_file_path`) — adversarial snapshot inputs are part of the threat model.
- Why it matters:
  Two failure surfaces are coupled here: the snapshot deserializer accepts any string for `json_pointer`, but the new `update_hook_file` validates strictly. The transition point (`derive_array_key`) is the right place to make the contract explicit.
- Recommendation:
  Add a sub-bullet to the load main flow (or a one-liner to C-22): "If `derive_array_key(body.json_pointer)` yields an empty / out-of-charset key, skip the replay-phase entry and warn to stderr; the canonical-phase re-apply restores the registered platform's entry from `entry_builder` so the user-visible end state is still correct." Or, equivalently: validate `json_pointer` shape during snapshot deserialization and reject malformed values up-front via the existing `Error::SnapshotCorrupt` path. Either choice converts a partial-failure footgun into an explicit, testable contract.



### R-102 `G-9 byte-identical round-trip wording conflicts with C-22's canonical re-apply`

- Severity: LOW
- Section: G-9; C-22; PRD "Round-trip test".
- Problem:
  G-9 says "Round-trip: install both platforms → unload → load → byte-identical (modulo timestamps)." C-22 says: "after `load` replays `snapshot.hook_bodies`, it iterates `PLATFORMS` and re-applies the canonical entry via `(platform.hook_file.entry_builder)()` ... the post-load on-disk hook state is independent of snapshot age." These two are simultaneously true *only when the same Ark version performs both unload and load*. Across versions where `ark_codex_hook_entry()` changes shape (e.g. timeout bump from 30s to 60s), unload→load on a stale snapshot is intentionally NOT byte-identical — V-IT-16 (the C-22 test) actually asserts non-equality.
- Why it matters:
  G-9's "byte-identical" framing comes verbatim from PRD Outcomes, which a reviewer/QA reader will check. After this iteration, the invariant is conditional. Misreading is cheap and the executor may write a too-strict round-trip test (V-IT-4) that fails under future hook-entry edits.
- Recommendation:
  Tighten G-9 to: "Same-version round-trip is byte-identical (modulo timestamps); cross-version round-trip converges to the new version's canonical entry." Or scope V-IT-4 to assert byte-identity only for files NOT covered by the canonical re-apply set (i.e. assert byte-identity on `.ark/**` and `.claude/commands/ark/**`, but only schema-equivalence on hook files / config.toml).



### R-103 `Skill body translation may need divergence beyond /ark:foo → ark-foo`

- Severity: LOW
- Section: G-4; G-12; C-7.
- Problem:
  The G-12 content-parity test asserts byte-equality after stripping both frontmatters and rewriting `/ark:<name>` → `ark-<name>` in the Claude body. This works syntactically, but Claude command bodies almost certainly include phrasing that's slash-invocation-specific. Looking at the existing templates: `templates/claude/commands/ark/quick.md`, `design.md`, `archive.md` ship with prose that almost certainly says things like "Read this slash command's body" or references the `$ARGUMENTS` placeholder (Claude slash command convention), which doesn't exist in the Codex skill model. Codex skills are description-routed by the model itself, not slash-invoked, so the user input lands in chat context, not as `$ARGUMENTS`.
  Reading R-013's resolution again, the PLAN concedes "This is a UX departure from Claude's `/ark:quick` slash command" but does not propagate that into G-12's content-parity check. If the Claude bodies say `$ARGUMENTS`, the test will demand identical bytes including `$ARGUMENTS` in the Codex skill, which is wrong for Codex.
- Why it matters:
  The reviewer cannot verify this without reading the three command bodies. If they reference `$ARGUMENTS` or any other Claude-specific token, the G-12 test will be impossible to satisfy without diverging the bodies, and the executor will land here mid-EXECUTE.
- Recommendation:
  Either (a) extend the G-12 rewrite rules to cover `$ARGUMENTS` → `<user task description>` (or whatever Codex idiom is preferred) and any other slash-specific tokens, OR (b) downgrade G-12 to existence-only and revert R-011 (the rename is fine, but content-parity carries hidden risk). Option (a) is preferred — it pins the divergence as a deliberate, documented rewrite — but requires the executor to actually read the three Claude command bodies before EXECUTE and enumerate the tokens. Add a Phase 4 sub-bullet: "Before locking G-12, audit `templates/claude/commands/ark/{quick,design,archive}.md` for slash-invocation-specific tokens (`$ARGUMENTS`, references to `/ark:` syntax in prose, etc.) and extend the rewrite rules accordingly. Document the full rewrite list in C-7."



### R-104 `Stage B JSON-scan scope is implicit`

- Severity: LOW
- Section: C-24; `[**Main Flow — `unload`]` Stage B; V-IT-17.
- Problem:
  C-24 says Stage B walks "every `*.json` file under `owned_dirs()`". The new `owned_dirs()` is `[.ark, .claude/commands/ark, .codex]`. So Stage B finds JSON files under those three trees — *but not* `.claude/settings.json` itself, which is under `.claude/` (not `.claude/commands/ark/`). That's fine for the C-24 use case (Stage A already handles registered platforms; Stage B is for unknown platforms in `.codex/` or under the `.ark/` tree), but the asymmetry isn't called out. A future reader might assume Stage B is a global "find all Ark hook entries everywhere" sweep when in fact it's scoped to owned dirs. V-IT-17's example (`.codex/extras.json`) lives under `.codex/`, which IS owned, so the test passes — but it doesn't exercise the boundary.
- Why it matters:
  When the next reviewer sees C-24, they may conclude "Stage B catches Ark entries in `.claude/settings.json` siblings too." It does not. If a future hypothetical platform stores its hook somewhere outside `owned_dirs()`, Stage B silently misses it. The reviewer flagged R-006 to make round-trip strict; the resolution narrows it to JSON files under owned dirs, which is acceptable but worth being explicit about.
- Recommendation:
  Tighten C-24's wording: "Stage B scans every `*.json` file under `Layout::owned_dirs()`. Hook files outside owned dirs (e.g. user-installed `~/.config/...` per-machine settings) are out of scope — round-trip preservation is bounded by the owned-dir invariant." Also, optionally add a V-IT-17b case asserting that an Ark-shaped hook entry in a *non-owned* JSON file (e.g. user's `~/foo.json` or some other arbitrary `.json` outside owned dirs) is NOT captured — pins the negative.



## Trade-off Advice

### TR-1 `Static slice vs dynamic Vec`

- Related Plan Item: `T-1`
- Topic: Compile-time safety vs flexibility
- Reviewer Position: Prefer Option A (static) — no change from 00.
- Advice: Adopt as-is.
- Rationale: Unchanged. Static is right.
- Required Action: None.



### TR-2 `Parameterize narrowly (hooks_array_key vs full pointer)`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (narrow `&str` key) — fully addressed in 01.
- Advice: Adopt as-is. The narrowed parameterization (drops C-19 RFC 6901, drops V-UT-5/V-UT-6, retains a charset-only validator) is exactly what TR-2 in 00 recommended.
- Rationale: The PLAN now matches the recommendation.
- Required Action: None.



### TR-3 `Default install both vs prompt vs error`

- Related Plan Item: `T-3`
- Topic: User Safety vs Convenience
- Reviewer Position: Prefer Option B (prompt on TTY, error on non-TTY without flags) — fully addressed.
- Advice: Adopt as-is.
- Rationale: G-3 / V-IT-11 / V-F-1 all updated.
- Required Action: None.



### TR-4 `Mechanical translation vs reauthor`

- Related Plan Item: `T-4`
- Topic: Cost vs UX
- Reviewer Position: Prefer Option A (mechanical) with content-parity test — adopted.
- Advice: Adopt with the R-103 caveat. The G-12 content-parity test is the right shape; auditing the Claude bodies for slash-specific tokens before locking the rewrite list is a load-bearing prerequisite (see R-103).
- Rationale: Mechanical translation pins parity; the rewrite list must cover every Claude-specific token, not just `/ark:foo`.
- Required Action: Fold R-103 into Phase 4 / C-7.



### TR-5 `AGENTS.md managed block`

- Related Plan Item: `T-5`
- Topic: Cleanliness vs surface area
- Reviewer Position: Prefer Option A (managed block) — no change from 00.
- Advice: Adopt as-is.
- Rationale: Unchanged.
- Required Action: None.



### TR-6 `Platform as struct vs trait`

- Related Plan Item: `T-6`
- Topic: Closed-set safety vs extensibility
- Reviewer Position: Prefer Option A (struct) — no change from 00.
- Advice: Adopt as-is.
- Rationale: Unchanged.
- Required Action: None.



### TR-7 `Parity test home`

- Related Plan Item: `T-7`
- Topic: Code locality
- Reviewer Position: Prefer Option B (`templates.rs`) — no change from 00.
- Advice: Adopt as-is.
- Rationale: Unchanged.
- Required Action: None.



### TR-8 `Codex slash UX: skills (description-routed) vs prompts (slash-invoked)`

- Related Plan Item: `T-8` (NEW in 01)
- Topic: User Safety vs Feature Completeness
- Reviewer Position: Prefer Option A (skills now)
- Advice: Adopt skills as the project-scope mechanism. Prompts are not project-scope-discoverable on current Codex (openai/codex#9848 open); shipping prompt files would mean files-on-disk-that-Codex-never-finds, which is worse than no prompt files at all (it suggests the feature works when it doesn't). Skills work today and surface the commands when the user describes the task.
- Rationale: Ship a working integration on the actual platform behavior. The UX gap (no `/ark:quick` keystroke parity) is real but bounded — description routing on Codex is the canonical mechanism per Trellis precedent and Codex docs. When openai/codex#9848 lands, adding `.codex/prompts/` as a *second* mechanism is a one-task follow-up and the G-12 parity test mechanism extends naturally to two parallel sibling trees.
- Required Action: Adopt as-is. The PLAN's recommendation is correct.
