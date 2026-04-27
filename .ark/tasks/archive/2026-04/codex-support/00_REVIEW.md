# `codex-support` REVIEW `00`

> Status: Closed
> Feature: `codex-support`
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
- Blocking Issues: 4
- Non-Blocking Issues: 9



## Summary

The platform-registry shape is right and the parity invariant is enforceable. The PLAN largely composes, the call graphs match the data structures, and the spec extensions mostly follow ark-context / ark-upgrade precedent symmetrically. However, there are four blocking defects that will produce a wrong artifact if implemented as written: (1) a Codex hook-timeout unit mismatch — the spec sets `5000` but Codex's `hooks.json` schema (per the Trellis template under review) takes seconds, not milliseconds, so the PLAN ships a 5000-second timeout; (2) `Snapshot::hook_bodies` claims to already accommodate Codex without schema migration, but the `entry_builder` field on `HookFileSpec` is a function pointer that cannot round-trip through serde — `load`'s identity-by-pointer reconstruction needs an explicit story; (3) the new `update_hook_file` API is described as "parameterized over `(json_pointer, identity_key)`" yet the `upsert` and `remove` helpers in the existing `fs.rs` are hard-coded to navigate `hooks.SessionStart` (not via JSON Pointer) — the implementation effort and test surface are larger than the PLAN's "rename + parameterize" framing admits; (4) the `pub use` aliases that C-4 mentions are not actually re-exportable as written (Rust does not allow `pub use foo as bar` for free functions in stable as a deprecation alias with attached `#[deprecated]`; you can but must declare it as `pub use` not `pub use as` in lib.rs — also the PLAN never says the deprecation is enforced via `#[deprecated]`). Beyond those, the source-scan test C-18 will fail given the proposed call sites in `init.rs`/`upgrade.rs` (they currently contain bare `std::fs::*` for `walk_files` indirection — the assertion needs broadening), the C-19 RFC 6901 escapes are over-engineering for two concrete platforms that share a static pointer, and the failure-flow #5 round-trip-lossy carve-out is fixable without much cost.



## Findings

### R-001 `Codex hook timeout uses wrong unit`

- Severity: CRITICAL
- Section: `[**Data Structure**]` (`ark_codex_hook_entry()`); G-6.
- Problem:
  The PLAN's `ark_codex_hook_entry()` body sets `"timeout": 5000` and the comment claims "5000ms timeout matches the Claude side (per ark-context C-15)." But Codex's `hooks.json` schema — per the Trellis reference template at `reference/Trellis/packages/cli/src/templates/codex/hooks.json` — uses **seconds**, not milliseconds. Trellis ships `"timeout": 15`. A literal `5000` in `.codex/hooks.json` is a 5000-second (~83-minute) timeout, not 5 seconds. Claude's `.claude/settings.json` separately uses milliseconds and 5000 there is correct.
- Why it matters:
  The Codex SessionStart hook will not behave as intended; the failure mode is silent — the timeout is so long that nobody will hit it, but the file ships a wrong number that will be copied by anyone who looks at it. C-5 explicitly carves out independent test coverage for the two builders ("They are NOT generated from a shared template — different platforms can diverge their entries (timeout, type, custom matcher) without one breaking the other"), so the fix is local: pick the right unit and document the divergence.
- Recommendation:
  Change `ark_codex_hook_entry()` to `"timeout": 15` (matching Trellis) or whatever value reflects the desired wall-clock budget. Update C-15 in spec to record that ark-context C-15 ("Hook timeout is 5000ms (Claude Code-side)") is Claude-only; Codex has its own constant. Add a unit test (V-UT-7) that asserts the value AND a `#[doc]` line on `ark_codex_hook_entry()` clarifying the unit so future readers don't repeat the mistake.



### R-002 `Snapshot round-trip story for entry_builder fn pointer is undefined`

- Severity: CRITICAL
- Section: `[**Data Structure**]` (`HookFileSpec`); C-8; G-9; Failure flow #7.
- Problem:
  `HookFileSpec` carries `entry_builder: fn() -> serde_json::Value`. C-8 claims "`Snapshot` schema is **unchanged**" and "Codex entries fit without schema migration." But `SnapshotHookBody` (per ark-context SPEC) stores `entry: serde_json::Value` only — it does not store the builder. That's fine for `load` (which uses `body.entry` directly), but the PLAN does not say what `load` does for snapshots whose `path` corresponds to a Codex install in a future version that has changed `ark_codex_hook_entry()`'s shape: replay-old vs replay-new is undefined. Worse, Failure flow #7 says "legacy `load` calls `update_hook_file` for every Platform whose `dest_dir` appears in `snapshot.files`" — that recovery path uses the *current* builder, but the rest of `load` (C-10) uses the captured `body.entry` verbatim. Same project, same call, two different sources of truth.
- Why it matters:
  This is the classic snapshot-versioning trap. ark-context handled it for Claude by making `update_settings_hook` (C-17) re-apply canonically *after* `load` re-applies the snapshot entry — the canonical write wins. The current `load.rs` already does this for Claude (lines 103–114). The PLAN needs to commit to: after replaying every `snapshot.hook_bodies` entry, `load` *also* runs `update_hook_file` with the **current** `(platform.hook_file.entry_builder)()` for every installed platform, so the final on-disk state is canonical regardless of snapshot age.
- Recommendation:
  Add a new constraint C-22 (Codex-specific equivalent of ark-context C-17): "After `load` replays `snapshot.hook_bodies`, it iterates `PLATFORMS` and re-applies the canonical entry via each platform's `entry_builder`. This makes the post-load on-disk state independent of snapshot age." Update Failure flow #7 to make this explicit. Drop the "manifest-derived recovery for legacy snapshots" framing — it's the same path for legacy and current.



### R-003 `update_hook_file parameterization is not just a rename`

- Severity: HIGH
- Section: `[**Data Structure**]` (parameterized helper signatures); C-4; Phase 2 step 3.
- Problem:
  The PLAN frames the io/fs.rs change as a rename + parameter addition: `update_settings_hook` → `update_hook_file(path, entry, json_pointer, identity_key)`. But the current implementation in `fs.rs` (lines 289–336, function `upsert_session_start_entry`) hard-codes the navigation: it calls `root_obj.entry("hooks")...entry("SessionStart")` literally; there is no JSON Pointer parser. The same is true of `navigate_session_start` (lines 356–362), which is invoked from `remove_settings_hook` and `read_settings_hook`. To support C-19 (RFC 6901 escapes incl. `~0`/`~1`) the PLAN requires a real JSON Pointer parser, not a string parameter — and that parser must support the *creation* of intermediate objects and arrays, not just navigation, since `upsert` may run on a missing-or-empty file. `serde_json::Value::pointer_mut` does not create intermediates; it returns `None` if any segment is absent.
- Why it matters:
  Phase 2 step 3 says "Same for `remove_*`, `read_*`. Old names retained as `pub use` aliases for one release." That underestimates the work. A correct implementation needs (a) a JSON Pointer parser that handles RFC 6901 escapes for both navigate and create-or-navigate semantics, (b) tests for the escape rules (V-UT-6 already lists this but the PLAN doesn't say what code lives where), and (c) a deliberate decision about what "create the intermediate" means when a non-final segment is `0` (numeric — should it create an array slot or an object key?). Given both shipping platforms use the same pointer `/hooks/SessionStart`, this is over-engineering — see TR-7.
- Recommendation:
  Either (a) keep the navigator hard-coded to `hooks.<KEY>` with `KEY` being the parameter (a much smaller change that fits both shipping platforms — Claude `SessionStart` and Codex `SessionStart`) and drop C-19 entirely, or (b) commit to a real RFC 6901 implementation and add an explicit data-structure section showing the parser API, the create-intermediates rules, and the array-vs-object disambiguation. Either path is acceptable; the current PLAN sits between them and underspecifies both. Add a Constraint making the chosen path explicit.



### R-004 `Source-scan test C-18 will fail given existing call sites`

- Severity: HIGH
- Section: C-18; V (acceptance mapping for C-18: "source-scan test `platforms_source_no_bare_std_fs`").
- Problem:
  C-18 promises: "All filesystem access in `platforms.rs` and the platform-iteration sites in `init`/`upgrade`/`unload`/`load`/`remove` routes through `io::PathExt` / `io::fs` helpers. No bare `std::fs::*`. New source-scan test `platforms_source_no_bare_std_fs` mirrors `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals`." But the existing source-scan test in `upgrade.rs` (lines 770–805) only scans `upgrade.rs` itself; the proposed `platforms_source_no_bare_std_fs` would presumably scan only `platforms.rs`. The PLAN never says the existing scan in `upgrade.rs` will be extended to `init.rs`, `unload.rs`, `load.rs`, `remove.rs`. As-is, C-18's invariant is not enforced where the bulk of the platform iteration *lives* (the command bodies), only in `platforms.rs` (which is mostly data declarations). Also: the `walk_files` helper in `fs.rs` itself uses `std::fs::read_dir` (line 385) — the existing test specifically excludes `fs.rs` because it's the sanctioned site, and the new test must apply the same carve-out.
- Why it matters:
  The C-18 invariant has no teeth in the places it matters most. The platform-iteration sites in `unload.rs` (line 66, `walk_files`) and `init.rs` (line 100+, `extract`) already use `io::*` helpers — the test would pass today — but a sloppy refactor in Phase 3 could regress without anyone noticing.
- Recommendation:
  Make C-18 explicit about *which files* the scan covers. Either (a) extend the existing `upgrade_source_has_no_bare_std_fs_or_dot_ark_literals` to a multi-file scanner that checks every file under `commands/` minus `mod.rs` and the agent subdir already covered by C-26/C-28, or (b) add five separate source-scan tests, one per command file. State the chosen approach in C-18.



### R-005 `pub use deprecation alias contract is undocumented`

- Severity: MEDIUM
- Section: `[**API Surface**]` (lib.rs re-exports); C-4 (mention only); Phase 2 step 3.
- Problem:
  The PLAN says: "Old function names kept as `pub use` aliases for one release." `lib.rs` re-exports list `update_settings_hook, remove_settings_hook, read_settings_hook` next to `update_hook_file, remove_hook_file, read_hook_file`. Three problems: (1) `pub use update_hook_file as update_settings_hook` *can* be done in `io/fs.rs` itself (or `io/mod.rs`), but the alias loses the original docstring unless rewritten; (2) Rust does not let you attach `#[deprecated]` to a `pub use` re-export of a function in stable — the deprecation must live on a wrapper `pub fn update_settings_hook(...) { update_hook_file(...) }` that carries the attribute, OR on the `pub use` itself (which works on items in stable but not always on aliases — verify); (3) "for one release" has no enforcement — what happens to the alias at version `0.2.0`? The PLAN should commit either to alias-removal-on-bump or alias-with-deprecation-now.
- Why it matters:
  This is the kind of thing that gets shipped, forgotten, and eventually deleted out of band, breaking downstream library consumers. The PLAN does not commit to a concrete deprecation policy, so reviewers can't sign off on the migration path.
- Recommendation:
  Add a numbered Constraint (e.g. C-22) stating: "The renamed helpers ship with `#[deprecated(since = "0.2.0", note = "Use update_hook_file")]` thin-wrapper functions (not `pub use` aliases) for backward compatibility. They are removed at `0.3.0`." Or drop them entirely: the PLAN itself notes the helpers are internal-call-site-only inside Ark; the public API surface is small. If no external consumers depend on them, just rename and skip the alias dance.



### R-006 `Failure flow #5 round-trip-lossy carve-out is wider than necessary`

- Severity: MEDIUM
- Section: `[**Failure Flow**]` #5; explicit reviewer prompt (Q6).
- Problem:
  The PLAN accepts as a documented limitation: "snapshot from a future Ark version → `walk_files` captures dir contents into `snapshot.files`. Hook capture iterates `PLATFORMS`, so unregistered platforms' hook files are NOT captured into `hook_bodies`. **Risk: round-trip lossy for unknown platforms.**" The implication is that a future Ark with a third platform (Cursor) would see a mid-stream user run `unload` with the older Ark and then upgrade — the Cursor hook entry would be wiped on `unload` and never restored on `load` because the older Ark doesn't know about Cursor. But the file on disk IS captured (via `walk_files`), so its non-hook content survives; only the surgical hook entry is lost.
- Why it matters:
  The snapshot format already carries enough information (`json_pointer`, `identity_key`, `identity_value`, `entry`) to restore an entry from a path-only descriptor. The capture side is the asymmetry: only platforms in the *currently-registered* `PLATFORMS` slice are walked. If the descriptor were instead derived from the file's *presence* (any `.<dir>/*.json` file under a known set of suffixes), or — more simply — if `unload` checked every JSON file under owned dirs for an Ark-identity hook entry, the round-trip would be lossless for any Ark hook entry written by any past or future version that uses the same identity-key contract.
- Recommendation:
  Add a constraint making the trade-off explicit. Two acceptable resolutions: (a) keep current behavior, but make the error visible — `unload` warns to stderr when it finds Ark-shaped hook entries in unregistered files (search every `.json` under owned dirs for entries containing `ARK_CONTEXT_HOOK_COMMAND`); (b) capture by file-existence and identity-value-presence, not by `PLATFORMS` membership. (b) is two extra `walk_files`+`read_hook_file` calls and yields strict round-trip safety — recommended.



### R-007 `Default-install on non-TTY without flags is a footgun`

- Severity: MEDIUM
- Section: G-3; Failure flow #3; explicit reviewer prompt (Q7).
- Problem:
  G-3: "With no flags on a non-TTY, both platforms install (matches `ConflictPolicy::Interactive` non-TTY safe-default precedent in upgrade C-7)." The analogy to C-7 is wrong: upgrade C-7 says "non-TTY stdin → `ConflictChoice::Skip` without reading" — i.e. the safe default is to do *less*, not more. Defaulting to install-both-platforms on CI machines that wanted only Claude (or only Codex) is the opposite of safe — it scaffolds files that will then need to be `git rm`'d by hand. Trellis's actual behavior, per `getInitToolChoices` and the per-platform `defaultChecked` flags, is to mark only one platform as default-checked even in interactive mode (in their case, only `claude-code`); they explicitly do not install everything blindly.
- Why it matters:
  Most CI setups invoke `ark init` non-interactively; an unmodified upgrade from 0.1.x to 0.2.x will silently start scaffolding `.codex/` directories into Claude-only projects. That's a behavior change that breaks user expectations.
- Recommendation:
  Change G-3's non-TTY behavior to require an explicit flag: non-TTY without any platform flag → error "init requires --claude, --codex, or both when stdin is not a TTY." Document the flag in the error message. This still lets `--claude` (or `--no-codex`) work scripted; it just refuses the silent default. Update Failure flow #3 to match. (Alternatively: keep the current default but make `--claude` alone a no-codex-implied flag, so the upgrade path is `ark init --claude`.)



### R-008 `"Adding a platform to an already-initialized project" is not a follow-up`

- Severity: MEDIUM
- Section: `[**State Transitions**]`; explicit reviewer prompt (Q8).
- Problem:
  "**Out of scope:** adding a platform to an already-initialized project. The user runs `init --force` or `init --codex` (additive) to install a new platform; this PLAN does NOT cover the 'expand selectively' path explicitly — it's a follow-up." This carve-out leaves a sharp edge: a Claude-only project upgrading from 0.1.x to 0.2.x runs `ark upgrade`, which (per G-11) "re-applies all platforms' hook entries unconditionally per `Platform::hook_file`" — this writes `.codex/hooks.json` and `.codex/config.toml` even though the project never opted into Codex. G-11 specifically says "every platform's managed block + hook file that the project has installed (manifest-driven)", but `manifest.files` won't contain Codex paths in a Claude-only-pre-existing project, so `upgrade` won't write those files. **But** the hook file write site in step 6 of upgrade's main flow says "For each `Platform` whose `dest_dir` appears in `manifest.files`" — that's correct only if `dest_dir` literally `.codex` appears as a substring of some manifest entry, which it does once `templates/codex/prompts/ark-quick.md` is in the manifest. There's a chicken-and-egg: the project never got those files installed, so they're never in `manifest.files`, so upgrade doesn't write them, so a Claude-only project stays Claude-only. That's the *correct* behavior, but the PLAN doesn't state it cleanly.
- Why it matters:
  Migration semantics for existing 0.1.x-Claude-only projects need to be a stated invariant, not a side effect. Users running `ark upgrade` after the 0.2.0 release should see their Claude-only project remain Claude-only with no `.codex/` material added — *unless* they explicitly opt in. "Explicitly opt in" needs a story, not just "it's a follow-up." The simplest version: `ark init --codex` rerun in an Arked project adds Codex without touching Claude. Document that or document its absence with intent.
- Recommendation:
  Add a numbered Goal G-14: "An existing Claude-only project upgraded with the new CLI version remains Claude-only on `ark upgrade`. To add Codex, the user re-runs `ark init --codex` (which is additive — installs only the requested platform's artifacts and records them in the manifest). This works because `init` is already idempotent and platform-keyed iteration on a per-flag basis amounts to a no-op for unselected platforms." Add a corresponding integration test V-IT-15 (`upgrade_on_claude_only_project_does_not_install_codex`).



### R-009 `RFC 6901 escapes are over-engineered for two static pointers`

- Severity: LOW
- Section: C-19; T-2 (parameterization decision).
- Problem:
  Both shipping platforms use `/hooks/SessionStart` (verbatim — no escapes). A future Cursor/OpenCode platform might use a different path, but neither requires `~0`/`~1` escapes; their config dirs and hook keys are normal alphanumerics. Implementing C-19 (full RFC 6901 escape handling) is a code+test surface that buys nothing for the two-platform shipping set. The reviewer prompt explicitly asks: "Is C-19 (RFC 6901 escapes) overkill?" — yes.
- Why it matters:
  Adds a second-system bias to the hook helper: a parser, error path, two extra unit tests (V-UT-5, V-UT-6), and a maintenance burden. None of it is exercised by the shipping code paths.
- Recommendation:
  Drop C-19. Replace with a simpler constraint: "The pointer string is a slash-prefixed dot-segmented identifier; segments must match `[A-Za-z0-9_-]+`. Other shapes error with a clear message." If a future platform needs RFC 6901, add it then.



### R-010 `Snapshot::hook_bodies is "schema unchanged" but identity_value semantics shift`

- Severity: LOW
- Section: C-8; ark-context C-27 cross-reference.
- Problem:
  C-8 says the schema is unchanged; ark-context C-27 covers the `#[serde(default)]` on `hook_bodies` itself. Older snapshots (pre-Codex) deserialize to `vec![]`. Fine. But the PLAN does not address: a new-Ark snapshot of a Claude+Codex project, deserialized by an old-Ark binary (downgrade scenario). The `hook_bodies` entries for Codex would be ignored — old Ark's `load` only knows the Claude path. Old Ark would then call `update_settings_hook` for Claude only, leaving the captured-but-unrestored Codex entry orphaned. That's mostly fine (old Ark can't run Codex anyway), but worth stating explicitly.
- Why it matters:
  Forward compatibility from new snapshots into old binaries is non-obvious. Most users won't downgrade; some will (corporate locked CLI versions).
- Recommendation:
  Add a one-line note under C-8: "Forward direction (older binary reading newer snapshot): the unknown `hook_bodies` entries deserialize successfully (the field shape is stable). Older Ark's `load` simply does not re-apply them. The user upgrading the binary picks up the entry on the next `unload`/`load` cycle." If this isn't true (i.e. older Ark validates `hook_bodies[*].path` against a known set), say so.



### R-011 `Parity test G-12 only checks existence; PRD asks for body content invariants too`

- Severity: LOW
- Section: G-12; C-7; PRD "Authoring + parity invariant".
- Problem:
  PRD: "Codex prompt bodies are mechanical translations of the Claude command bodies: same prose, YAML frontmatter dropped (Codex prompts don't support it), `:` → `-` in any `/ark:foo` references." C-7 says: "Body content is byte-for-byte identical to the matching Claude command except: (a) frontmatter stripped, (b) any `/ark:foo` reference within the body rewritten to `/ark-foo`. Verified once at authoring time; no automated content-diff check (would over-constrain future divergence)." That's defensible, but the parity test G-12 is named `templates_codex_prompts_match_claude_commands` — the name promises more than the test delivers. Future readers will assume it asserts content parity; it asserts only file-name parity.
- Why it matters:
  Bait-and-switch test names make code review harder. A reader sees the test name and thinks "ah, parity is enforced" without reading the test body.
- Recommendation:
  Rename to `templates_codex_prompts_exist_for_every_claude_command` or `every_claude_command_has_a_codex_sibling`. Or, and this is cheap to do: add a content-parity check that, after stripping frontmatter and applying the `/ark:foo` → `/ark-foo` rewrite, the bodies are byte-identical. Document the auto-rewrite explicitly. This pins the parity invariant without forbidding future divergence — when the bodies need to diverge, the test fails loudly and the engineer makes the decision.



### R-012 `Layout::owned_dirs grows from 2 to 3 — touches every consumer`

- Severity: LOW
- Section: G-13; C-15; `Layout::owned_dirs` change.
- Problem:
  The current `Layout::owned_dirs() -> [PathBuf; 2]` (line 191 of `layout.rs`) is consumed by `unload.rs` (line 65, `for owned in layout.owned_dirs()`), `load.rs` (line 73), and `remove.rs` (line 76 — destructured: `let [ark_dir, claude_commands] = layout.owned_dirs();`). The PLAN says `owned_dirs` returns 3 entries. The destructure in `remove.rs` will break. The PLAN's Phase 1 step 4 says "Update existing `owned_dirs`-using tests" but doesn't explicitly call out the `remove.rs` destructure.
- Why it matters:
  Concrete refactor item that must not be missed. Easy to fix once flagged; easy to miss in a fast read.
- Recommendation:
  Add an explicit Phase 1 sub-bullet: "`remove.rs` line 76 currently destructures `owned_dirs` into a 2-array; switch to slice iteration (`for d in layout.owned_dirs()`) to allow growth." Same for any future consumer.



### R-013 `Codex prompts under .codex/prompts/ — confirm the path is canonical`

- Severity: LOW
- Section: G-4; PRD outcome.
- Problem:
  The PLAN places Codex prompts at `.codex/prompts/ark-{quick,design,archive}.md`. The Trellis reference at `reference/Trellis/packages/cli/src/configurators/codex.ts` does NOT install prompts under `.codex/prompts/` — it installs `skills/`, `agents/`, and `hooks/`. The Codex CLI's own native prompt-discovery path is `~/.codex/prompts/` (user scope) and Trellis ships skills as the project-scope prompt mechanism instead. The PLAN doesn't cite a Codex doc URL. Whether `.codex/prompts/` (project-scope) is in fact a real Codex feature — vs. just a name inherited from the agentskills.io world — is not established.
- Why it matters:
  If the Codex CLI doesn't auto-load `.codex/prompts/*.md` from the project root, the prompts ship and never get found by Codex. The user types `/ark-quick` and gets "unknown command." The PRD's outcome ("`.codex/prompts/ark-{quick,design,archive}.md` (new)") becomes a no-op.
- Recommendation:
  Cite the Codex docs URL or behavior establishing that `.codex/prompts/` is a discovery path. If it isn't, switch to whatever Codex's actual project-scope-prompt mechanism is (skills, per Trellis precedent? Or a different file?). Block until confirmed.



## Trade-off Advice

### TR-1 `Static slice vs dynamic Vec`

- Related Plan Item: `T-1`
- Topic: Compile-time safety vs flexibility
- Reviewer Position: Prefer Option A (static)
- Advice:
  Stick with static `&[&Platform]`. The conditional-compile-out concern is theoretical and the precedent (`EMPTY_DIRS`) is right.
- Rationale:
  Two platforms today, three or four within 18 months. The set is bounded and known at build time; making it dynamic would be over-engineering. Trellis uses a static record literal in TS — same idiom.
- Required Action:
  Adopt as-is.



### TR-2 `Parameterize vs sibling helper`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Need More Justification — see R-003.
- Advice:
  Parameterize, but more narrowly than the PLAN proposes. Both shipping platforms use the same JSON Pointer (`/hooks/SessionStart`), so the parameterization is solely on the *array key under `hooks`*, not a full RFC 6901 pointer. That collapses the work to one keyword arg and zero parser code.
- Rationale:
  The PLAN's "RFC 6901" framing buys flexibility for hypothetical platforms; the cost is a parser, escape handling, and two unit tests that exercise behavior not used by any shipping caller. A simple `hooks_array_key: &str` parameter is enough.
- Required Action:
  Rewrite C-19 and the parameterized API to take `&str` array-key, not full JSON Pointer. Drop V-UT-5, V-UT-6. Keep V-UT-4 (idempotence with explicit key). Or, if you really want full RFC 6901, justify it with a concrete future-platform shape that needs it.



### TR-3 `Default install both vs prompt vs explicit-only`

- Related Plan Item: `T-3`
- Topic: User Safety vs Convenience
- Reviewer Position: Prefer Option B (prompt) but reject the non-TTY-installs-both default — see R-007.
- Advice:
  Prompt on TTY (as planned). Non-TTY without flags → error, not "install both."
- Rationale:
  CI and scripted workflows that don't pass flags should fail loud, not silently expand the install footprint. The "safe default" framing in G-3 is backwards.
- Required Action:
  Update G-3, Failure flow #3, V-IT-11, V-F-1.



### TR-4 `Mechanical translation vs reauthor`

- Related Plan Item: `T-4`
- Topic: Cost vs UX
- Reviewer Position: Prefer Option A (mechanical)
- Advice:
  Confirm mechanical. Add the body-parity test (R-011 recommendation) so future divergence is intentional.
- Rationale:
  Reauthoring doubles maintenance forever; mechanical translation lets the parity invariant be enforced by a 30-line test.
- Required Action:
  Adopt with R-011 strengthening.



### TR-5 `AGENTS.md managed block`

- Related Plan Item: `T-5`
- Topic: Cleanliness vs surface area
- Reviewer Position: Prefer Option A (managed block) — agree with PLAN.
- Advice:
  Agree.
- Rationale:
  Symmetric with `CLAUDE.md`; reuses `MANAGED_BLOCK_BODY`; respects user-authored content above/below.
- Required Action:
  Adopt as-is.



### TR-6 `Platform as struct vs trait`

- Related Plan Item: `T-6`
- Topic: Closed-set safety vs extensibility
- Reviewer Position: Prefer Option A (struct)
- Advice:
  Adopt struct.
- Rationale:
  Closed-set; debuggable; serializable if ever needed; Rust idiom for static registries. Trait would force `Box<dyn>` and lose the `const` declaration.
- Required Action:
  Adopt as-is.



### TR-7 `Parity test home`

- Related Plan Item: `T-7`
- Topic: Code locality
- Reviewer Position: Prefer Option B (`templates.rs`) — agree with PLAN.
- Advice:
  Place in `templates.rs` next to `templates_have_expected_structure`.
- Rationale:
  The test reads both template trees; co-locating with other `include_dir!`-driven tests is right. `tests/` would force a separate crate; `platforms.rs` would couple data declarations to template content.
- Required Action:
  Adopt as-is.
