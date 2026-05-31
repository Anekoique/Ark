# `ark-agent-namespace` REVIEW `00`

> Status: Open
> Feature: `ark-agent-namespace`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 2
- Non-Blocking Issues: 9



## Summary

The plan is well-aligned with the PRD and AGENTS.md conventions: module layout is sensible, named errors fail loud, `Display`-returning summaries are enforced, and `PathExt`/`Layout` gating is explicit. However, two load-bearing details are wrong-as-written and must be corrected before execute: (a) `PathExt::rename_to` is asserted in C-8 / Runtime step 9 but does not exist on the trait today — it must be added in Phase 1; and (b) the `## Spec` header in `templates/ark/templates/PLAN.md` is `## Spec \`{Core specification}\`` (with an inline-code suffix), so a line-scanner that matches only `"## Spec"` as a prefix will work but one that matches the literal line `"## Spec"` will not — the plan's description is ambiguous and needs to spell out the exact matching rule. A handful of secondary issues (open questions in V-E-1 / T-6 / V-E-5, untested commands, missing `design.md` workflow doc) should be resolved in the next iteration but do not block execution on their own.



## Findings

### R-001 `PathExt::rename_to does not exist`

- Severity: CRITICAL
- Section: `[**Constraints**] C-8`, `[**Runtime**] task archive step 9`
- Problem:
  C-8 states `task archive`'s dir move "uses rename semantics" and Runtime step 9 says `fs::rename(...) via PathExt::rename_to`. The trait `io::PathExt` today exposes `read_optional`, `read_text_optional`, `read_bytes`, `write_bytes`, `ensure_dir`, `remove_if_exists`, `remove_dir_if_empty`, `remove_dir_all` — and nothing else. No `rename_to`, no `rename`. There is zero existing `fs::rename` call anywhere under `crates/`. AGENTS.md forbids direct `std::fs::*` from `commands/` (see "What Not to Do") and C-4 re-states that for the `agent` module. So as planned, the archive command has no legal way to move a directory.
- Why it matters:
  C-4 + C-8 combined require `PathExt::rename_to` to exist — the plan implicitly depends on a helper it does not ship. Executor would hit this on day one.
- Recommendation:
  Add a Phase 1 item: "extend `io::PathExt` with `fn rename_to(&self, dest: impl AsRef<Path>) -> Result<()>` that wraps `std::fs::rename` and maps errors via `Error::io`." Add a unit test for it alongside the other `path_ext` tests. Then cite that new method in C-8 + Runtime step 9 rather than citing it as pre-existing.

### R-002 `## Spec header format is inline-code, not a bare H2`

- Severity: HIGH
- Section: `[**Runtime**] spec extract step 3`, `[**Implementation**] Phase 3 step 14`
- Problem:
  The shipped `templates/ark/templates/PLAN.md` line 58 is `## Spec \`{Core specification}\`` (an H2 whose header text is `Spec \`{Core specification}\``). This PLAN itself uses `## Spec \`Core specification\`` (with the placeholder filled). The plan says the scanner "matches `## Spec` header, read[s] to next `## ` prefix or EOF" and elsewhere "find the `## Spec` section via a line-range parser that bounds on the next `##` or EOF." That's ambiguous:
  - If the scanner compares `line == "## Spec"`, it will miss every real PLAN.
  - If the scanner does `line.starts_with("## Spec")`, it will match `## Speculation` too — theoretical but worth nailing down.
  - The end boundary `starts_with("##")` matches `###` too, which is wrong; must be `starts_with("## ")` or `starts_with("##\n")`/`"##$"`.
- Why it matters:
  `spec extract` is the deep-tier archive promotion path. Getting this wrong silently promotes an empty or truncated SPEC and the reviewer never notices until someone reads the extracted file.
- Recommendation:
  Specify the exact predicate. Suggested form: start = `line.starts_with("## Spec") && (line.len() == 7 || line.as_bytes().get(7) == Some(&b' '))`; end = first subsequent line where `line.starts_with("## ")` or `line == "##"`. Document in the Constraints section as C-N. Add a V-UT test case for a PLAN whose `## Spec` line has the inline-code annotation (mirroring the real template format), and one for a PLAN where the *next* section header is `### Subheading` (must NOT terminate early).

### R-003 `Toml dep is net-new; justify briefly and version-pin via workspace`

- Severity: MEDIUM
- Section: `[**Implementation**] Phase 1 step 1`, `[**Constraints**] C-6`, `[**Trade-offs**] T-3`
- Problem:
  `grep '^name = "toml"' Cargo.lock` returns nothing — `toml` is not currently a transitive dep. The plan adds it as `toml = "0.8"` in `ark-core/Cargo.toml` (step 1). AGENTS.md's "What Not to Do" does not forbid new deps outright, but the project has kept dependencies tight (thiserror, serde, serde_json, include_dir, chrono, base64). T-3 argues for `toml` vs. a hand-rolled parser — that's fine, but the decision deserves a one-line acknowledgement that it's net-new, not already transitive.
- Why it matters:
  Reviewer should flag dep growth explicitly so VERIFY can confirm intent. Also, pinning `toml` at the workspace level (not just ark-core) avoids version drift if another crate later pulls it in.
- Recommendation:
  (a) In T-3, add one sentence: "This adds one direct dep not currently in `Cargo.lock`." (b) Consider pinning via `[workspace.dependencies]` in the root `Cargo.toml` so both crates can share it if ark-cli ever needs TOML. (c) Optional: confirm `toml = "0.8"` is the current stable — 0.8.x is fine as of April 2026.

### R-004 `Workflow doc rewrite must cover design.md, not just workflow.md`

- Severity: MEDIUM
- Section: `[**Goals**] G-8`, `[**Implementation**] Phase 3 steps 16-17`
- Problem:
  G-8 names two paths: `.ark/workflow.md` (live, this repo) and `templates/ark/workflow.md` (embedded). But `templates/claude/commands/ark/design.md` also contains the same raw-bash recipes the plan is replacing (`<!-- ARK:FEATURES:START -->` block, mkdir/cp instructions in the slash-command body). Grep for `ARK:FEATURES` in `templates/` returns both `specs/features/INDEX.md` AND `claude/commands/ark/design.md`. If the slash command still tells the agent to run raw `mkdir`/`cp`, G-8 is only half-done.
- Why it matters:
  The PRD's Outcome line "workflow.md is updated to reference `ark agent` commands instead of raw mkdir/cp/echo recipes" is defeated if the slash commands in `.claude/commands/ark/*.md` retain the old recipes.
- Recommendation:
  Extend Phase 3 step 16 to include `templates/claude/commands/ark/{quick,design}.md`. Add it to G-8's explicit path list. Update V-IT-1/V-IT-2 assertions or add V-IT-4 to grep the rendered slash commands for absence of raw `mkdir`/`cp`/`echo` for ark-managed paths.

### R-005 `No failure test for spec_register; no failure test for task_promote`

- Severity: MEDIUM
- Section: `[**Validation**] Unit Tests, Failure / Robustness Validation`
- Problem:
  Test count: 13 UT + 3 IT + 4 F + 6 E = 26. For ~12 commands plus state machine this is defensible, but two specific gaps:
  - `spec_register` has V-UT-12 (happy paths) but no failure test. What happens when `specs/features/INDEX.md` is missing entirely, or when the managed block has been tampered with (START but no END)? `update_managed_block` today *creates* the file if missing, so missing-INDEX would silently succeed rather than error — is that the intent?
  - `task_promote` has V-UT-9 (legal + illegal-by-phase), but no explicit coverage for "promote quick → deep when only PRD exists" (legal under state machine but leaves the task with no PLAN). The plan says promote "does not rewrite existing PLAN/PRD artifacts" — fine — but the test should assert that directly.
  - V-E-5 (`spec register` with pipes/special chars) is open. Must resolve before execute: sanitize (how?) or error (with what variant?).
- Why it matters:
  These commands touch the authoritative spec index and tier metadata; silent misbehavior here is the kind of bug Ark's "named errors or fail loud" principle was written to prevent.
- Recommendation:
  Add V-F-5: `spec_register` against a file with a malformed (unclosed) managed block → define the behavior (likely: append a fresh closed block or error) and assert it. Resolve V-E-5 before execute — recommended: error with `Error::InvalidSpecField { field, reason }` on `|` or newline in `feature`/`scope`; allow spaces. Add an assertion in V-UT-9 that PRD/PLAN bytes are unchanged after promote.

### R-006 `Phase enum rename_all="lowercase" clashes with on-disk "in_progress" status`

- Severity: LOW
- Section: `[**Data Structure**] TaskToml`
- Problem:
  The Phase enum uses `#[serde(rename_all = "lowercase")]` so variants serialize as `design`, `plan`, `review`, `execute`, `verify`, `archived`. The live `task.toml` in this repo already contains `phase = "review"` — so the plan's encoding is consistent with current practice. Good. However `status: String` is left as a free-form string that the live task.toml uses as `"in_progress"`. If the intent is to track status alongside phase (G-3 doesn't mention it), either make it a typed enum (`Status::{InProgress, Completed, Archived}`) or drop it from the model — free-form strings silently accumulate typos.
- Why it matters:
  Low impact today, but the whole point of the namespace is "agent owns judgment, binary owns structural correctness." A stringly-typed status field is exactly the kind of latent footgun the PRD wants to eliminate.
- Recommendation:
  Either (a) type `status` as an enum with the same `rename_all` treatment and define its transitions alongside `Phase`, or (b) drop `status` from TaskToml and compute it from `phase` (Archived ↔ `phase == Archived`; everything else ↔ InProgress). Prefer (b) — it removes a redundant field.

### R-007 `.current resolution error variant is under-specified`

- Severity: LOW
- Section: `[**Validation**] V-E-1`
- Problem:
  V-E-1 says `--slug` omitted with missing `.current` → `Error::TaskNotFound { slug: "<.current missing>" }` "or similar dedicated variant — open for review." A literal angle-bracketed string as a `slug` value breaks the invariant that `slug` is a filesystem-safe identifier and any error message that formats it (e.g. "no task found at `/root/.ark/tasks/<.current missing>`") will confuse users.
- Why it matters:
  Error messages are user-facing. "no task found" pointing at a nonsense path is a minor usability bug.
- Recommendation:
  Add a dedicated variant: `Error::NoCurrentTask { path: PathBuf }` with message "no active task set: `.ark/tasks/.current` is missing; pass `--slug <s>` or run `ark agent task new` first." Cite it in Runtime's slug-resolution step and in V-E-1.

### R-008 `clap hide=true semantics — confirm behavior`

- Severity: LOW
- Section: `[**API Surface**], [**Constraints**] C-1`
- Problem:
  `#[command(hide = true, about = "...")]` on the parent enum variant `Agent(AgentArgs)` hides the subcommand from `ark --help` (its parent's listing). That's what C-1 asserts. The plan should be explicit: `hide = true` applied on the `Agent` variant hides it from `ark --help`; running `ark agent --help` still prints the child's own help tree (i.e. it's hidden from the *parent* listing, not from itself). That is clap's actual behavior, matches the plan's intent, and is what V-IT-3 tests — but someone reading the plan might wonder, and the next PLAN should note this explicitly.
- Why it matters:
  Tiny, but the PRD explicitly says "ark agent --help prints a banner" — the plan should confirm the mechanism it relies on.
- Recommendation:
  One-liner in [**API Surface**]: "`hide = true` hides `agent` from `ark --help`; `ark agent --help` still renders its child subcommands and about-text. V-IT-3 asserts both."

### R-009 `Module layout: spec:: called by task:: — cycle risk if mod.rs re-exports carelessly`

- Severity: LOW
- Section: `[**Architecture**]`, `[**Runtime**] task archive internal call graph`
- Problem:
  `commands/agent/task/archive.rs` calls `commands/agent/spec/extract.rs` and `commands/agent/spec/register.rs`. Both subtrees live under `commands/agent/`. Rust modules don't have cycles in the compile sense, but if `commands/agent/mod.rs` re-exports `spec` using `pub use` and `task::archive.rs` imports via `crate::commands::agent::spec::...` while `spec::mod.rs` also imports `state` from a sibling, you can get surprising visibility issues. Not a cycle, more a coupling pattern worth naming.
- Why it matters:
  Low; only a heads-up.
- Recommendation:
  In `commands/agent/mod.rs`, keep `pub mod task; pub mod spec; pub mod template; mod state;` with `state` private + re-exported through `mod.rs` (`pub use state::{Phase, Tier, TaskToml};`). `task::archive` then imports `super::spec::{extract, register}` explicitly. Call it out in Architecture as a single line.

### R-010 `task_iterate from Design should be IllegalPhaseTransition, but plan says "shouldn't happen in practice"`

- Severity: LOW
- Section: `[**Validation**] Unit Tests, phase 2`
- Problem:
  Phase 2 test list says `task_iterate` with no prior PLANs "shouldn't happen in practice → errors with IllegalPhaseTransition (can't iterate from Design)". The legal-transition table for Deep has `Review -> Plan` as the iterate transition. So iterate-from-Design naturally falls out of the state-machine guard. But iterate-from-Plan is *also* currently illegal per the table (only `Review -> Plan` is listed), and yet iterate-from-Plan could be meaningful ("I got feedback outside the REVIEW doc"). Either the transition table has a gap, or the semantics of iterate need clarifying.
- Why it matters:
  The iterate command's state-machine semantics are muddled between "transition phase" and "bump NN". If iterate only runs from Review, say so; if iterate implicitly re-enters Plan from any phase, encode that.
- Recommendation:
  Clarify in [**State Transitions**]: `iterate` is specifically `Review -> Plan` for deep tier, AND bumps NN. It is illegal from any other phase. Document that "iterate" as a noun is the REVIEW-rejected-to-PLAN transition.

### R-011 `Integration test for deep tier does not exercise iterate`

- Severity: LOW
- Section: `[**Validation**] V-IT-2`
- Problem:
  V-IT-2 lists `task review → task iterate (loop once) → task execute`. That's good. But "loop once" isn't crisp — does the test call `task iterate` once, then `task review` again to reach the second iteration's review, then `task execute`? The sequence `review → iterate → execute` skips the second review, which under the state machine is `Review -> Execute` from iteration 01, which is legal but asymmetric (iterations 00 and 01 treated differently). Spell it out.
- Why it matters:
  The round-trip test is the best proof the state machine works end-to-end. Ambiguity here invites a brittle test.
- Recommendation:
  Specify V-IT-2 as: `new → plan → review → iterate → review → execute → verify → archive`. Assert `01_PLAN.md` and `01_REVIEW.md` exist alongside `00_*`, `phase = archived` at end, SPEC promoted, INDEX row present.



## Trade-off Advice

### TR-1 `Explicit per-transition subcommands vs. single phase --to`

- Related Plan Item: `T-1`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (explicit subcommands, as planned)
- Advice:
  Keep the explicit per-phase subcommands.
- Rationale:
  Typo-resistance is a real benefit here because the state machine is small and stable — five commands aren't onerous, and `--help` self-documents transitions. A single `phase --to <p>` would push the transition logic into a runtime string parse, which loses compile-time guarantees on the small enum `Phase`.
- Required Action:
  Keep as is; no change needed.

### TR-2 `task archive deep-tier — one command vs. three`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (one command, as planned)
- Advice:
  Keep `task archive` as the single entry point that internally dispatches `spec extract` + `spec register` when tier == deep.
- Rationale:
  The PRD's Outcome explicitly says "`task archive` is a single command that reads `task.toml.tier` and performs the deep-tier SPEC extract + register automatically." Splitting it would violate the PRD. Mitigation in T-2 (help text names deep-tier extras) is sufficient.
- Required Action:
  Keep as is; ensure `ark agent task archive --help` text mentions the deep-tier side effects.

### TR-3 `toml crate vs hand-rolled parser`

- Related Plan Item: `T-3`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (toml crate)
- Advice:
  Use the `toml` crate.
- Rationale:
  Silent acceptance of malformed TOML violates C-6/C-11. A hand-rolled parser for even "just a few fields" will eventually be wrong on a quoted-string edge case. See R-003 for the one small caveat: acknowledge it as a net-new dep.
- Required Action:
  Keep as planned; add the one-line dep acknowledgement from R-003.

### TR-4 `## Spec extraction — string scan vs Markdown parser`

- Related Plan Item: `T-4`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (string scan) with tightened predicate
- Advice:
  Keep the string scan, but formalize the match predicate as in R-002 and add the edge-case tests there.
- Rationale:
  `pulldown-cmark` is ~3k LOC of indirect deps for what is genuinely one regex-free function over a format we own. As long as R-002's tighter predicate lands, the string scan is strictly superior. Do not add a full Markdown parser.
- Required Action:
  Keep as planned; adopt R-002's predicate; add the `## Spec \`...\`` and `### subheading` tests.

### TR-5 `Hidden subcommand vs separate binary ark-agent`

- Related Plan Item: `T-5`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (hidden subcommand, as planned)
- Advice:
  Keep `ark agent` as a hidden subcommand of the `ark` binary.
- Rationale:
  A separate binary doubles the release surface (cargo-dist config, npm package layout, shell installer wiring — see `[workspace.metadata.dist]` in `Cargo.toml`). The single-binary approach with `hide = true` plus the "Not covered by semver" banner is the right call. Do not fork the binary.
- Required Action:
  Keep as planned; address R-008 (spell out `hide = true` semantics).

### TR-6 `spec extract tier check error variant`

- Related Plan Item: `T-6` (explicitly open by plan's own admission)
- Topic: Flexibility vs Clean Design
- Reviewer Position: Prefer Option B (dedicated `Error::WrongTier`)
- Advice:
  Add a dedicated variant `Error::WrongTier { expected: Tier, actual: Tier }`. Do NOT reuse `IllegalPhaseTransition` for a tier check.
- Rationale:
  `IllegalPhaseTransition { tier, from: Phase, to: Phase }` answers the question "what transition failed" with three pieces of data that have no meaning for a tier check — you'd have to synthesize bogus from/to. The error's message ("cannot transition from Design to Plan under tier Quick") would be nonsensical when printed for a tier-mismatch case. One extra enum variant costs ~4 LOC; clean error messages are worth far more than that. Plus: `spec extract` is not the only place this check might appear later (deep-only operations may grow), and the variant name will age well.
- Required Action:
  Replace the open question in Runtime step 1 of `spec extract` with a concrete decision: add `Error::WrongTier { expected, actual }` to the error enum additions in [**Data Structure**]; use it here. Update C-11 to reference it alongside `IllegalPhaseTransition`.

### TR-7 `.current file vs deriving from task.toml scan`

- Related Plan Item: `T-7`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer Option A (keep `.current`)
- Advice:
  Keep `.current`.
- Rationale:
  It's already the convention (the live task.toml layout uses it). Scanning every task.toml on every `--slug`-optional call is O(N) filesystem calls for no real benefit. The only catch is defining "missing `.current`" cleanly — see R-007 for that.
- Required Action:
  Keep as planned; address R-007 by adding a dedicated error variant.
