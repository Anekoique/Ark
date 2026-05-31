# `codex-support` VERIFY

> Status: Closed
> Tier: Deep
> Verdict: Approved with Follow-ups

## Summary

The implementation delivers G-1..G-14 cleanly. `cargo build`, `cargo test --workspace` (213 core tests + 2 cli tests, all passing), `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` all run clean. The `Platform` registry is the right shape and is iterated consistently in `init`, `upgrade`, `unload`, `load`. The Stage A/B unload split (C-24), the canonical re-apply phase in `load` (C-22), the source-scan invariant across all five command files (C-18), the deprecated thin wrappers in `io/fs.rs` (C-23), and the `hooks_array_key` charset validator (C-19) are all in place with matching tests (V-IT-15/16/17, V-UT-10, etc.). The four R-100 LOWs from review are addressed: R-101 (`derive_array_key` empty-key fallback to `"SessionStart"` at `load.rs:148-154`), R-102 (legacy hook-bodies fallback at `load.rs:128-135`), R-104 (Stage B explicitly bounded to owned dirs in `unload.rs:172-184`). R-103 was addressed by *narrowing* the G-12 parity test to existence-only — see V-001 below; this is a defensible deviation but worth recording. Two real follow-ups exist: `remove.rs` does not iterate `PLATFORMS` for hook removal (V-002), and the V-IT-11 non-TTY-error test from PLAN's validation matrix is missing (V-003). Neither blocks merge.

## Findings

### V-001 G-12 content-parity check was narrowed from PLAN to existence-only
- Severity: LOW
- Scope: Plan Fidelity / SPEC Drift
- Location: `crates/ark-core/src/templates.rs:55-74` (`every_claude_command_has_a_codex_skill_sibling`)
- Problem: PLAN G-12 (lines 105 of `01_PLAN.md`) specifies that the parity test asserts byte-equality between Claude command body and Codex skill body after stripping both frontmatters and rewriting `/ark:<name>` → `ark-<name>`. The shipped test only asserts that each Codex skill *exists*; it does not enforce body content parity. The test docstring says "Existence-only — content parity is not asserted because Codex skills carry different frontmatter and rewrite slash-specific tokens." This is exactly the R-103 risk that review flagged: Claude bodies use `$ARGUMENTS` and `# /ark:quick $ARGUMENTS` heading idioms that don't translate mechanically. Inspection confirms the actual skills do diverge in places (`# /ark:quick $ARGUMENTS` → `# ark-quick`), so a strict content-parity test would be infeasible without an expanded rewrite list.
- Why it matters: PLAN's `## Spec` section is the future feature SPEC; G-12 as written doesn't match the shipped code. The latest PLAN's `## Spec` will be promoted to `specs/features/codex-support/SPEC.md` at archive — the bytes-equality wording will then become a spec lie.
- Expected: Either (a) tighten the G-12 wording in `## Spec` to match what the test actually asserts before archive (existence-only with a sentinel test like `codex_skill_bodies_have_codex_frontmatter_not_claude_frontmatter` covering shape), or (b) extend the rewrite list in C-7 and re-introduce content parity. Option (a) is the lower-risk move and matches the executor's actual judgment.

### V-002 `remove.rs` hard-codes per-platform hook-file removal instead of iterating `PLATFORMS`
- Severity: LOW
- Scope: Abstraction / Plan Fidelity
- Location: `crates/ark-core/src/commands/remove.rs:91-104`
- Problem: PLAN G-10 says "removes both platforms' SessionStart hook entries" and the Architecture section (PLAN line 156) describes `remove.rs` as "per-platform hook removal". The implementation calls `remove_hook_file` twice with literal arguments (`"SessionStart"`, `"command"`, `layout.claude_settings()`, `layout.codex_hooks_file()`) rather than iterating `PLATFORMS` and using each platform's `HookFileSpec`. The `RemoveSummary` also has separate `removed_hook_entry` (Claude) and `removed_codex_hook_entry` fields rather than a per-platform vec.
- Why it matters: Adding a third platform requires editing `remove.rs`, which contradicts the registry's stated purpose (G-1: "adding a third platform later is a registry entry, not a refactor of the command bodies"). Today there are two platforms so the cost is low, but the abstraction is weaker than the other commands.
- Expected: Follow-up task — refactor `remove.rs` to iterate `PLATFORMS` and replace the two boolean summary fields with a per-platform Vec or BTreeMap. Out of scope for this verify; not blocking.

### V-003 V-IT-11 non-TTY-no-flags-error test is missing
- Severity: LOW
- Scope: Correctness / Plan Fidelity
- Location: `crates/ark-cli/src/main.rs:668-732` (tests module)
- Problem: PLAN's V-IT-11 (line 464 of `01_PLAN.md`) is `cli_resolve_platforms_no_flags_non_tty_errors` and asserts `Err(InitError::NoPlatforms)` on the non-TTY-no-flags path. The shipped tests are `cli_resolve_platforms_no_x_excludes` (V-IT-12-style) and `cli_resolve_platforms_positive_flags_narrow` — neither covers the non-TTY-no-flags branch added per R-007/G-3. The branch itself is implemented (`main.rs:114-119`) and has correct error wording, but it is exercised only by the production path, not by a test.
- Why it matters: G-3's reversed default (R-007) is a security/safety choice; without a test, a future refactor could regress to silent default-install on non-TTY without anyone noticing. The implementation is correct today, the gap is purely test coverage.
- Expected: Add a unit test that injects a non-TTY stdin (mockable by faking `std::io::stdin().is_terminal()` or by parameterizing `resolve_platforms` to take an `is_tty: bool`). Follow-up.

### V-004 Stage B's surgical orphan-removal write is dead-code (followed by `remove_dir_all`)
- Severity: INFO
- Scope: Code Quality
- Location: `crates/ark-core/src/commands/unload.rs:130-184` (Stage B), then `unload.rs:138-140` (`remove_dir_all` of every owned dir)
- Problem: Stage B (`scan_orphan_file` → `path.write_bytes(...)` at line 238) writes the orphan-stripped JSON back to disk for any `.json` file under owned dirs, but unload then calls `remove_dir_all` on every owned dir at line 138-140. The surgical write to disk is therefore wasted work — the snapshot capture is what matters, and the file is about to be deleted regardless. C-24 specifies surgical removal "in case of unregistered platforms", which makes sense for Stage A's hook files (which live under `.claude/`, NOT `.claude/commands/ark/`, so they survive `remove_dir_all`) — but Stage B is scoped to `owned_dirs()`, all of which are wiped.
- Why it matters: Performance is irrelevant at this scale, but the asymmetry between Stage A (precision matters because the file survives) and Stage B (precision doesn't matter because the file is deleted) suggests the C-24 spec text is more general than it needs to be. A reader trying to understand Stage B's purpose may be misled.
- Expected: No code change needed. Optionally, simplify `scan_orphan_file` to only capture-into-snapshot without rewriting the file, or document the asymmetry in C-24's wording before archive.

## Follow-ups

- FU-001: `remove-iterates-platforms` — Refactor `remove.rs` to iterate `PLATFORMS` and use `HookFileSpec` for each platform's hook removal; replace the two boolean summary fields with a per-platform map. Standard tier.
- FU-002: `cli-non-tty-test-coverage` — Add the missing `cli_resolve_platforms_no_flags_non_tty_errors` test (V-IT-11) by parameterizing `resolve_platforms` over a TTY predicate or via a stdin-injection harness. Quick tier.
- FU-003: `g12-content-parity-or-spec-edit` — Either (a) edit the latest PLAN's `## Spec` G-12 wording to match the shipped existence-only test before archive, or (b) extend the C-7 rewrite list (cover `$ARGUMENTS`, the `# /ark:<name>` heading idiom) and re-introduce body-content parity in the test. Quick tier; (a) is the recommended path.
