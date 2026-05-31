# `opencode-support` REVIEW `00`

> Status: Open
> Feature: `opencode-support`
> Iteration: `00`
> Owner: Reviewer (Opus 4.7)
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

- Decision: Rejected
- Blocking Issues: 5
- Non-Blocking Issues: 6

Per the verdict rule (any open CRITICAL → Rejected), this iteration cannot advance to EXECUTE. Two CRITICAL findings (R-001, R-002) describe claims about runtime behavior and existing code that are contradicted by the cited reference / source. R-003 / R-004 are HIGH issues that would ship broken or weak code if left uncorrected. R-005 makes the validation/spec mapping for forward-compat (C-14) circular.

## Summary

The plan has the right *shape*: it correctly identifies that opencode-support is mostly an additive registry entry, the `apply_managed_state` / `capture_hook` / `remove_hook` plumbing genuinely no-ops on `hook_file = None`, and the `Manifest::record_block` `(file, marker)` dedupe is real (verified at `state/manifest.rs:79–91`). The Layout/templates plumbing is plausible and well-scoped. However, two load-bearing claims — (a) the runtime contract for the TS plugin (G-9, G-15, NG-4) and (b) the CLI-flag exclusivity mechanism (Data Structure block) — are factually wrong against the cited reference (`reference/Trellis/.../session-start.js`) and the existing source (`crates/ark-cli/src/main.rs:65–77`). Either error alone is a CRITICAL because the plan as written cannot be executed without rediscovering the truth and editing the spec mid-flight. Fix both, tighten the C-14 forward-compat validation (V-F-4 doesn't actually test what C-14 claims), restate the cross-references to codex-support so `## Spec` is self-contained per workflow §3.3, and the plan is implementable.

## Findings

### R-001 Plugin uses only `chat.message`, but that hook cannot mutate the user message — message mutation requires `experimental.chat.messages.transform`, which NG-4 bans

- Severity: CRITICAL
- Section: `## Spec` G-9, G-15, NG-4; `## Runtime` "Main Flow — runtime (user side, opencode session)" steps 5–7; `## Trade-offs` T-2 (implicitly assumes A is wired correctly)
- Problem:
  G-9 states: "Hooks `chat.message` (stable, non-experimental)." NG-4 states: "No use of `experimental.*` opencode hooks (`experimental.chat.messages.transform` etc.). Plugin uses only stable hooks (`chat.message`)." The Runtime section step 6 then says: "Find the first text part of the user message; prepend `<ark-context>\n${additionalContext}\n</ark-context>\n\n---\n\n` to it."
  This contradicts the cited community reference. `reference/Trellis/packages/cli/src/templates/opencode/plugins/session-start.js` lines 350–453 prove the actual contract: `chat.message` is a *notification* fired when a message arrives — the handler receives `(input)` (line 356) and stores into `contextCollector` (line 387) but does *not* see or return the messages array. Mutation happens in `experimental.chat.messages.transform` (line 396), which receives `(input, output)` where `output.messages` is mutable (line 419: `lastUserMessage.parts[textPartIndex].text = ...`). The Trellis header comment at line 6 makes this explicit: "Uses OpenCode's chat.message + experimental.chat.messages.transform hooks." If `chat.message` could mutate, Trellis would not need the second hook.
  PRD outcome #5 *requires* the prefix to land in the model's input ("a `<ark-context>...</ark-context>`-tagged prefix on the first user message"). With NG-4 banning the only mechanism that achieves that, the plan ships a plugin that runs without effect.
- Why it matters:
  The shipped plugin will not satisfy PRD outcome #5. The integration "works" (no crash) but injects nothing — a silent regression that the proposed `#[ignore]`d `bun check` (T-3, V-F-3) cannot catch and the documented manual smoke test (Phase 5 #22) is the only thing that would catch, *after* shipping. This is the central feature of the task; getting it wrong invalidates the entire registry-extension exercise.
- Recommendation:
  Choose one of:
  (a) Drop NG-4 and use the two-hook pattern Trellis uses: `chat.message` to gate-and-prepare via a per-session `Set<string>`, `experimental.chat.messages.transform` to actually prepend. Document the experimental status in C-7 and add an "if upstream renames the hook, this plugin breaks; see opencode plugin docs" caveat. This is the realistic option.
  (b) Find a stable hook that does mutate the user message (research item — confirm by reading current opencode plugin docs via context7 or upstream source). If one exists, name it explicitly in G-9 and rewrite NG-4 to ban only the *unused* experimental hooks.
  (c) If neither (a) nor (b) is acceptable to the planner, scope the plugin down to a logging-only diagnostic ("ark context loaded; agent should consult `ark context --scope session`") and update PRD outcome #5 to match. This is the worst option — it strips the user value of the plugin.
  Default recommendation: (a). Rewrite G-9, NG-4, and the Runtime "Main Flow — runtime" section to use both hooks. Add C-7 language acknowledging the experimental-hook risk. Keep T-2 (log-and-continue) as is.



### R-002 CLI exclusivity claim — `conflicts_with` is not what the existing `--claude` / `--no-claude` flags use

- Severity: CRITICAL
- Section: `## Spec` G-3 ("existing clap `conflicts_with` arg-group logic handles this; no new logic"); `## API Surface` Data Structure block lines 195–202 of the plan
- Problem:
  The plan's `InitArgs` example reads:
  ```
  #[arg(long, conflicts_with = "no_claude")]    pub claude: bool,
  #[arg(long = "no-claude", conflicts_with = "claude")]    pub no_claude: bool,
  ```
  G-3 says: "existing clap `conflicts_with` arg-group logic handles this; no new logic."
  Source check at `crates/ark-cli/src/main.rs:65–77` shows the existing code uses bare `#[arg(long)]` for all four flags with NO `conflicts_with` and NO `ArgGroup`. Mutual exclusivity is enforced *behaviorally* in `resolve_platforms_pure` (lines 132–161): a positive flag wins over a negative flag (`f.on && !f.off`), so passing `--claude --no-claude` silently coerces to "claude on" rather than rejecting at parse time. V-IT-8 ("`init --opencode --no-opencode` errors at clap parse (`conflicts_with`)") therefore tests a behavior that doesn't exist on the existing flag pairs and would be a behavior *change* (not a continuation) for the new pair.
- Why it matters:
  Two breakages: (1) The Data Structure block, copy-pasted into source, will produce different runtime semantics for the OpenCode flag pair than for the Claude/Codex pairs — `--opencode --no-opencode` errors out, `--claude --no-claude` does not. Inconsistency violates G-1's "no command-body refactor" intent. (2) V-IT-8 would either pass for OpenCode and fail for the others (proving the inconsistency) or, if the executor "fixes" Claude/Codex to match OpenCode, that's a silent change to existing user-visible CLI behavior outside this task's scope — a SPEC drift that REVIEW must catch now, not VERIFY later.
- Recommendation:
  Either:
  (a) Drop `conflicts_with` from the new flags. Match the existing shape: bare `#[arg(long)]` and `#[arg(long = "no-opencode")]`. Remove V-IT-8 (or rewrite it to assert "passing both leaves opencode on", matching the existing semantic). G-3 sentence "(existing clap `conflicts_with` arg-group logic handles this; no new logic)" → "(matches the existing positive-wins resolution in `resolve_platforms_pure`)". This is the lowest-risk option.
  (b) Add a deliberate consistency upgrade: introduce `#[group(id = "claude-toggle", multiple = false)]` (and codex/opencode equivalents) on all three platform pairs in *one* commit, document it as a deliberate user-visible CLI change, and either include it as a new G in this task or split it into a follow-up. If you choose (b), update G-3 and the Data Structure block to use `ArgGroup` (matching `UpgradeArgs`'s shape at `main.rs:189–207`), not `conflicts_with`.
  Default: (a). The task scope is "add OpenCode"; tightening unrelated CLI parsing belongs in a separate task.



### R-003 `owned_dirs()` return type wrong in plan — should be `[PathBuf; 4]`, not `[&'static str; 4]`

- Severity: HIGH
- Section: `## Spec` G-13 / `## Data Structure` layout.rs additions lines 162–166 of the plan
- Problem:
  Plan's snippet:
  ```
  pub fn owned_dirs(&self) -> [&'static str; 4] {
      [ARK_DIR, CLAUDE_COMMANDS_ARK_DIR, CODEX_DIR, OPENCODE_DIR]
  }
  ```
  Actual existing signature at `crates/ark-core/src/layout.rs:238–244`:
  ```rust
  pub fn owned_dirs(&self) -> [PathBuf; 3] {
      [self.ark_dir(), self.claude_commands_ark_dir(), self.codex_dir()]
  }
  ```
  Callers use the result as iterables of `PathBuf` (e.g. `unload.rs:66` does `for owned in layout.owned_dirs() { for path in walk_files(&owned)? { ... } }`; `unload.rs:108` does `.try_for_each(|d| d.remove_dir_all().map(|_| ()))`). A return-type change from `[PathBuf; 3]` to `[&'static str; 4]` would fail to compile at every caller.
- Why it matters:
  The Data Structure block is presented as the source-of-truth signature for the executor. As written, it requires editing every owned_dirs caller — `unload.rs` (3 sites), `load.rs` (1 site) — none of which the plan acknowledges. That violates G-1's "no refactor of any command body." More importantly it converts a 1-line registry growth into a 4-file refactor and makes V-UT-10 (which asserts the return shape) fail.
- Recommendation:
  Rewrite the snippet:
  ```rust
  /// `owned_dirs()` returns four absolute paths (Layout-resolved). On a
  /// platform-X-only install, missing dirs walk to empty.
  pub fn owned_dirs(&self) -> [PathBuf; 4] {
      [
          self.ark_dir(),
          self.claude_commands_ark_dir(),
          self.codex_dir(),
          self.opencode_dir(),
      ]
  }
  ```
  Update V-UT-10 to assert "4-entry array of `PathBuf` rooted at `layout.root()`" (not "the 4-entry array `[ARK_DIR, …]`"). Drop the `&'static str` return-type wording from G-13.



### R-004 V-F-4 does not test C-14's actual claim (old binary reading new manifest)

- Severity: HIGH
- Section: `## Validation` V-F-4; `## Acceptance Mapping` row for C-14
- Problem:
  C-14 says: "An older `ark` binary (pre-OpenCode) reading a manifest written by a newer binary that recorded `.opencode/` paths in `manifest.files`: the older binary doesn't iterate OpenCode in `PLATFORMS` (it has 2 entries) and doesn't know about `.opencode/`. … `.opencode/` files are NOT captured by the old binary's `unload`."
  V-F-4 says: "a single unit test asserts `Snapshot` deserialization is forward-compatible by adding an `.opencode/`-prefixed path to a hand-rolled `Snapshot` JSON and round-tripping through `serde_json::from_str` → `serde_json::to_string`."
  These don't connect. The C-14 claim is about runtime behavior — the older binary's `PLATFORMS` slice has 2 entries, `owned_dirs()` returns 3 paths, the older binary's `unload` skips `.opencode/`. A serde round-trip on `Snapshot` (which has no OpenCode-specific fields anyway — it's just `Vec<SnapshotFile>` keyed by `PathBuf`) tests whether the deserializer accepts an unknown-shape path, which it always has. The test passes vacuously and rolls forward-compat into a property that has nothing to do with OpenCode.
- Why it matters:
  The plan claims C-14 is documented-but-validated; the Acceptance Mapping row reads "C-14 → V-F-4". If the executor takes the row at face value, they'll write a one-line test that "validates" C-14 without actually validating it, and VERIFY can't catch it because there's nothing to compare against the SPEC's claim.
- Recommendation:
  Two options:
  (a) Drop V-F-4. Mark C-14 as "documented; not unit-tested (would require multi-version test harness — out of scope)." Update the Acceptance Mapping accordingly. This is honest.
  (b) Write a real test: simulate a 2-entry `PLATFORMS` slice (or factor `owned_dirs` to take an explicit dir list) and run `unload` against a tempdir containing `.opencode/` files. Assert `.opencode/` survives. This adds engineering complexity worth its own trade-off discussion.
  Default: (a). Strip V-F-4 entirely, change the Acceptance Mapping row to match the plan's existing pattern for documented-only constraints (cf. C-7, C-8, C-12).



### R-005 `## Spec` is not self-contained per workflow §3.3 — multiple cross-references to codex-support without restating the rule

- Severity: HIGH
- Section: `## Spec` G-1 ("the codex-support test … is renamed and updated"), G-3 ("continues codex-support R-007 precedent"), G-12 ("continue codex-support G-12 pattern", "continues codex-support C-7 REVISED rationale"), G-13 ("continues codex-support pattern"), G-14 ("continues codex-support G-14"), C-1 ("continues codex-support C-18"), C-9 ("Continues codex-support C-18 REVISED"), C-10 ("Continues codex-support G-12"), NG-7 ("continues codex-support NG-6"), C-2 ("mirroring Codex's `config.toml` pattern exactly")
- Problem:
  Workflow §3.3 (PLAN gate, Rule): "`## Spec` must be self-contained every iteration (deltas go in `## Log`). It is copied verbatim to `specs/features/<name>/SPEC.md` on archive." Workflow §4 REVIEW: "Reject (HIGH) if the latest PLAN's `## Spec` references prior iterations instead of restating in full."
  Several of the cross-references *name the specific rule* (e.g. G-1 names the test being renamed, G-12 (b) restates the frontmatter shape rule, G-3 implicitly restates non-TTY behavior). Those are fine. But several are bare "continues codex-support X" without restating: G-3's "continues codex-support R-007 precedent" relies on the reader knowing R-007; C-1's "continues codex-support C-18" is unspecified; C-9's "Continues codex-support C-18 REVISED" is a re-pointer; G-13's "continues codex-support pattern" is vague; NG-7's "continues codex-support NG-6" is bare. When this `## Spec` is extracted on archive (workflow §4 ARCHIVE), the resulting `specs/features/opencode-support/SPEC.md` will have dangling references.
- Why it matters:
  Per workflow §4 REVIEW, this is an explicit Reject-HIGH trigger. The archived SPEC must stand alone; future readers (humans or the LLM running a later phase) will not have codex-support's R-007 / C-18 / NG-6 in context.
- Recommendation:
  Sweep the Spec section. For each "continues codex-support X" reference:
  - If the referenced rule is small (one sentence): inline it. E.g. G-3 → "On a non-TTY with no platform flags, init errors with a message naming all three flags `--claude`, `--codex`, `--opencode` (and their `--no-*` counterparts)."
  - If the rule is long but the only thing this PLAN needs is a *property* (e.g. "source-scan tests forbid hand-composed Ark paths"): restate the property without naming the codex-support ID.
  - Drop the bare "continues codex-support X" suffix; the new Spec doesn't need to advertise lineage (the codex-support SPEC's `## CHANGELOG` will record the lineage on archive).



### R-006 V-IT-2 frontmatter assertion is borderline — a single-line `description:` Claude copy-paste passes both negative and positive checks

- Severity: MEDIUM
- Section: `## Validation` V-IT-2; `## Spec` G-12 (b)
- Problem:
  V-IT-2 asserts: "body starts with `---\n` and the first non-`---` line begins with `description:`. Assert no `argument-hint:` line in the frontmatter block." Claude command frontmatter has both `description:` and `argument-hint:` (verified at `templates/claude/commands/ark/quick.md:1–4`). A copy-paste that retains both fields fails the test (good). But a copy-paste that drops only `argument-hint:` and keeps `description:` (e.g. an editor-assisted reformat) will pass — yet the resulting OpenCode command may carry verbatim Claude `description:` text including phrasing that doesn't suit OpenCode's UX.
- Why it matters:
  The codex-support test (`codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter`) has the same weakness (catching only frontmatter shape, not body drift), and codex-support ate that cost via the C-7 REVISED rationale: body drift is policed by code review. The plan inherits this rationale (G-12). Acceptable, but the wording of V-IT-2's coverage in the Acceptance Mapping ("body verbatim is policed by code review") should be clearer about the residual risk.
- Recommendation:
  Either:
  (a) Keep V-IT-2 as is. Add to G-12 (b): "Body content drift between Claude and OpenCode is not mechanically asserted; the no-`argument-hint:` check catches the most common copy-paste failure. Deeper drift is policed by code review at template-edit time (continues codex-support C-7 REVISED, restated in this Spec)." Inline the codex-support C-7 REVISED text per R-005.
  (b) Tighten V-IT-2: also assert each command body contains `# /ark:<name> $ARGUMENTS` as a sanity check on slash-invocation idiom retention (T-4 Option A premise).
  Default: (a). (b) over-constrains and reintroduces the problem codex-support C-7 REVISED solved.



### R-007 T-3 `bun check` syntax — likely wrong subcommand name

- Severity: MEDIUM
- Section: `## Trade-offs` T-3; `## Implementation` Phase 5 #21
- Problem:
  Phase 5 #21: "run `bun --version` then `bun --check .opencode/plugins/ark-context.ts`". T-3 (a) names the same: "`bun check templates/opencode/plugins/ark-context.ts`."
  Bun's syntax-check / type-check entry points are `bun build --no-bundle <file>` (parses) or `bunx tsc --noEmit <file>` (type-checks). `bun check` is not a documented Bun subcommand as of 1.x; `bun --check` is not a flag. The closest is `bun build --target=bun --no-bundle` which exits non-zero on parse error.
- Why it matters:
  Phase 5 #21 is a manual checklist item for the executor. As written, the command will fail with "unknown command" on a developer machine and the executor will either skip the check (T-3 Option B by accident) or invent something. C-12 references the same command in the spec. Acceptance Mapping row C-12 → "Bun smoke test" inherits the issue.
- Recommendation:
  Rewrite Phase 5 #21 and C-12 to use a real syntax-check command. Options, in order of preference:
  (a) `bun build --no-bundle templates/opencode/plugins/ark-context.ts > /dev/null` — exits non-zero on parse error.
  (b) `bunx tsc --noEmit --allowJs --strict templates/opencode/plugins/ark-context.ts` — type-checks. Requires no extra setup beyond `bun`.
  (c) `node --check templates/opencode/plugins/ark-context.ts` — works for `.js` but not `.ts`; would require porting C-12 / G-15 to a transpile-first step.
  Pick (a) for minimal toolchain. Update T-3, C-12, V-F-3, Phase 5 #21 in lockstep.



### R-008 Missing trade-off — plugin language (TypeScript vs. JavaScript)

- Severity: MEDIUM
- Section: `## Trade-offs` (none discusses language choice)
- Problem:
  G-15 asserts "plain TypeScript executable by Bun without compilation." The Trellis reference plugin (`reference/Trellis/.../session-start.js`) is JavaScript, not TypeScript. The Codex hooks shipped under `templates/codex/skills/` (`session-start.js`, `inject-subagent-context.js` — confirmed via `ls templates/codex/skills/`) are also JavaScript. Choosing TypeScript for OpenCode-only diverges from the existing convention and forces every developer touching the plugin to know TS *and* the Bun runtime's TS transpile semantics.
  The plan does not discuss this; the reader cannot tell whether TS was a deliberate ergonomic upgrade or an unexamined default.
- Why it matters:
  Trade-offs the planner skips are trade-offs the executor or future maintainer pays. If TS adds maintenance friction without buying type safety on a 30-line file, JS is the cheaper choice. If TS is right for richer types around the SessionStart envelope, that argument should be on the page.
- Recommendation:
  Add a T-6:
  ```
  T-6: Plugin source — TypeScript vs. JavaScript.
    Option A: TS (Bun runs it directly; loose typing on `client.app.log` payload).
      Adv. Slightly clearer on the SessionStart envelope shape via types.
      Disadv. Diverges from existing `templates/codex/skills/*.js`; introduces TS-specific tooling assumptions in C-12.
    Option B: Plain JS with JSDoc type comments.
      Adv. Matches Trellis reference + existing Codex JS scripts; no transpile.
      Disadv. Marginally less ergonomic for envelope shape.
    Recommendation: B (planner to decide).
  ```
  Either rewrite or keep TS, but state the call. If switching to JS, the file path becomes `.opencode/plugins/ark-context.js` and `OPENCODE_PLUGIN_FILE` updates in lockstep.



### R-009 Plugin runtime contract handwaves over pluginID / handler-export shape

- Severity: MEDIUM
- Section: `## Spec` G-9, G-15; `## Runtime` "Main Flow — runtime"
- Problem:
  G-9 says the plugin "Hooks `chat.message`" and uses "a module-local `Set<sessionID>`." Trellis's reference shows the actual export shape (line 350): `export default async ({ directory }) => { return { "chat.message": async (input) => { ... } } }` — i.e. a default-exported async factory that receives the project context (`directory`, etc.) and returns a handler object. The plan never names this shape. C-7 says "Keep ≤80 lines" but the Trellis pattern alone (factory + hook + dedupe) is 30 lines before any context-injection logic.
  Combined with R-001 (the chat.message-only contract is wrong), the plan's runtime section is too vague to translate into code without re-deriving the shape from the reference.
- Why it matters:
  Phase 1 #4 says "templates/opencode/plugins/ark-context.ts (hand-authored, ≤80 lines)" without specifying the export shape. Without it, the executor falls back to copying Trellis. Specifying the shape now lets the executor write tests for the plugin's pure helpers (PRD outcome #5: "verified by reading the plugin source and by a unit test that exercises the plugin's pure-function helpers").
- Recommendation:
  In the Spec, add to G-9 (after rewriting per R-001):
  > Plugin export shape: `export default async ({ directory, client }) => ({ "chat.message": async (input) => ..., "experimental.chat.messages.transform": async (input, output) => ... })`. The chat.message handler stores into a module-local `Map<sessionID, string>` keyed by sessionID. The transform handler consumes that map and prepends `<ark-context>...</ark-context>\n\n---\n\n` to the first text part of the last user message. Pure helpers (`build_envelope_prefix(additionalContext: string): string`, `should_inject(sessionID: string, processed: Set<string>): boolean`) are exported for unit testing.
  This makes V-F-3 ("Validated by reading the plugin source") into something a Rust-side test can exercise via a TS-as-text scan plus a Bun-side unit test the developer runs locally.



### R-010 Acceptance Mapping has gaps

- Severity: LOW
- Section: `## Validation` Acceptance Mapping
- Problem:
  - G-15 row says "(manual, Phase 5 #21–#22; documented in C-7, C-12)" — fine, but G-15 covers more than the manual smoke (it also asserts no `package.json` ships, which a Rust-side test could trivially check via `OPENCODE_TEMPLATES.get_file("../package.json").is_none()` style).
  - C-13 row says "(no diff to assert; absence of changes verified by `cargo test --workspace` continuing to pass)" — workspace tests don't actually assert *non-modification* of `Snapshot`, `HookFileSpec`, etc. They assert the modified code still works. C-13 is genuinely "documented-only"; the row should say so.
  - C-2 maps to V-UT-4, but V-UT-4 only asserts the plugin file is written byte-for-byte equal — it does not assert the OPENCODE_TEMPLATES rooting at `commands/` (vs. the whole `opencode/` tree). A separate assertion ("`OPENCODE_TEMPLATES.get_file('plugins/ark-context.ts').is_none()`") would lock C-2 down.
- Why it matters:
  Soft validation rows leave VERIFY without a concrete check to run. Each gap is small but they accumulate.
- Recommendation:
  - Add V-UT-13 ("`OPENCODE_TEMPLATES` does not contain `plugins/`; the plugin file is shipped via `extra_files`") and map C-2 to both V-UT-4 and V-UT-13.
  - Add V-UT-14 ("no `package.json` extracted under `.opencode/`") and map G-15 to it alongside the manual rows.
  - Rewrite the C-13 row: "(documented; no unit test — absence-of-diff is enforced by code review at PR time)."



### R-011 Phase ordering nit — interactive-prompt mock harness exists or doesn't?

- Severity: LOW
- Section: `## Implementation` Phase 3 step 16 last bullet
- Problem:
  Step 16 says "`init_interactive_prompt_offers_three_platforms` (gated on TTY mock if the existing prompt code is testable)." Looking at `crates/ark-cli/src/main.rs:165–177`, `interactive_select_platforms` reads `std::io::stdin()` directly with no injection seam. Testing it from CLI tests requires either a `Box<dyn FnOnce>` injection (which `resolve_platforms_pure` already exposes — verified at lines 132–161) or a real TTY harness.
  The phrasing "if the existing prompt code is testable" suggests the planner didn't verify; in fact, the testable seam is `resolve_platforms_pure` with a synthesized closure, which is *already* the code under test. The proposed test name should target that seam.
- Why it matters:
  Minor — leaves the executor uncertain about which test to write.
- Recommendation:
  Rephrase step 16 last bullet: "Add `resolve_platforms_pure_offers_all_three_when_no_flags_and_tty` exercising the closure-injected branch at `main.rs:154–156`, asserting the closure is called once. The interactive-select stdin reading itself remains untested (continues existing pattern)."



## Trade-off Advice

### TR-1 Plugin file location — `extra_files` vs. embedded in `OPENCODE_TEMPLATES`

- Related Plan Item: T-1
- Topic: Compatibility vs Clean Design (mirror Codex's `config.toml` precedent vs. uniform template extraction)
- Reviewer Position: Prefer Option A
- Advice:
  Keep Option A (chosen). Mirror Codex's `config.toml` treatment via `extra_files`. The hash-tracking divergence the plan flags is the right reason — an Ark-owned plugin file should not be subject to "user customized this; prompt on upgrade" semantics.
- Rationale:
  Symmetry with Codex's `config.toml` is the load-bearing argument; the alternative would force the planner to either special-case `OPENCODE_PLATFORM` in the upgrade hash logic (more code) or accept "user edits the plugin to fix a bug → next upgrade prompts about the conflict" (worse UX). Option A's one extra `include_str!` is the cheapest path.
- Required Action:
  Adopt as-is. No SPEC change.



### TR-2 Plugin error handling — log-and-continue vs. fail-loud

- Related Plan Item: T-2
- Topic: Robustness vs Discoverability
- Reviewer Position: Prefer Option A
- Advice:
  Keep Option A (log-and-continue) but with a stronger discoverability commitment than the head-comment note suggests.
- Rationale:
  The failure modes in the plan's Failure Flow (PATH issues, `ark context` non-zero in non-Ark dirs, JSON parse) are genuine user-environment issues, not bugs in the plugin. Hard-failing breaks opencode for those users. However, "swallow + log" is only useful if users discover `opencode logs`. A one-time stderr write on the *first* swallowed failure would help.
- Required Action:
  Add to C-7: "On the first swallowed failure per session, additionally write a single-line note to stderr: `ark-context: skipped context injection (see opencode logs for details)`. Suppress on subsequent failures to avoid noise." Otherwise keep Option A.



### TR-3 Plugin syntax test — `bun check` (#[ignore]) vs. no check

- Related Plan Item: T-3
- Topic: Build hygiene vs Toolchain coupling
- Reviewer Position: Need More Justification (and fix the command — see R-007)
- Advice:
  After R-007 is applied, prefer Option A only if the executor commits to running it before each release. Otherwise Option B.
- Rationale:
  An `#[ignore]`d test is documentation that pretends to be a check. It does not run on `cargo test`, it does not run in CI by default, and it requires Bun installed locally. The `argument-hint:` parity test (V-IT-2) and the manual Phase 5 #22 cover most of the realistic failure modes. The marginal value of a `bun build --no-bundle` test is "catch a syntax error before VERIFY"; the marginal cost is a Bun dependency on the developer machine.
- Required Action:
  Either: (a) write the `#[ignore]`d test as a *runnable* helper (e.g. `xtask ts-check`) that release-candidate workflows invoke, not a stale test. Document that workflow in C-12. Or (b) drop T-3 entirely; rely on V-IT-2 + Phase 5 #22 + code review. The plan should pick one; "ignored test you might run" is the worst of both.



### TR-4 Body translation — keep slash idioms verbatim or rewrite

- Related Plan Item: T-4
- Topic: Compatibility vs Correctness
- Reviewer Position: Prefer Option A (with elevated verification)
- Advice:
  Adopt Option A but elevate the Phase 5 #22 manual check to a *gating* pre-merge step rather than a documentation note. If `$ARGUMENTS` does not substitute under opencode, the parity test passes but the user-visible behavior is broken.
- Rationale:
  Option A's premise — that opencode commands use `$ARGUMENTS` substitution like Claude — is plausible per opencode docs but unverified at planning time. The plan acknowledges this in the parenthetical "(verified against the docs at planning time; if it diverges, the translation rule is …)" but does not commit to the verification.
- Required Action:
  Before EXECUTE: pull current opencode plugin docs (via context7) and confirm `$ARGUMENTS` is the right token for command argument substitution. If yes, keep T-4 Option A as-is; record the verification in `## Log`. If no, rewrite C-6 to use the correct token and update the three `templates/opencode/commands/ark/*.md` bodies in Phase 1.



### TR-5 AGENTS.md sharing — single block vs. per-platform markers

- Related Plan Item: T-5
- Topic: Compatibility vs Future-Proofing
- Reviewer Position: Prefer Option A
- Advice:
  Keep Option A (chosen). Single `<!-- ARK:START --> ... <!-- ARK:END -->` block, deduped by `(file, marker)`.
- Rationale:
  The codex-support SPEC G-5 already establishes "managed block parallel to `CLAUDE.md`'s. Body identical." Adding per-platform markers now would create two visually identical blocks (the body comes from `MANAGED_BLOCK_BODY`, identical for every platform), which is uglier and grows the manifest by one entry per platform for zero functional gain. Per-platform divergence, if it ever matters, is a future-task concern with a clear migration path (introduce a marker variant, walk the manifest, rewrite).
- Required Action:
  Adopt as-is. No SPEC change.



### TR-6 (NEW) Plugin source language — TypeScript vs. JavaScript

- Related Plan Item: (none — see R-008)
- Topic: Convention Consistency vs Type Ergonomics
- Reviewer Position: Prefer Option B (JavaScript)
- Advice:
  Author the plugin as `.opencode/plugins/ark-context.js` with JSDoc type annotations, not `.ts`. Update G-15, OPENCODE_PLUGIN_FILE, OPENCODE_ARK_CONTEXT_TS (rename to `OPENCODE_ARK_CONTEXT_JS`), and the templates path.
- Rationale:
  Trellis's reference plugin and the existing `templates/codex/skills/session-start.js` and `inject-subagent-context.js` are all JavaScript. A 30-line file gains little from TS types but loses convention parity and adds a transpile-implicit dependency on Bun. JSDoc gets ~80% of the type benefit at zero tooling cost.
- Required Action:
  Add T-6 to the trade-offs section per R-008. Resolve by switching to JS or by stating in T-6 why TS earns its keep on this specific file.

