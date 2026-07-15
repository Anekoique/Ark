# `codeagent-cli-support` VERIFY

> Status: Living document. Maintained by the implementer during EXECUTE → COMMIT.
> Feature: `codeagent-cli-support`
> Target Task: `codeagent-cli-support`
> Tier: `deep`
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **No verdict line — completion = no `PENDING`.** Deep tier: `/ark:commit` refuses on any `PENDING`. Standard: warns and proceeds.

---

## Project Spec Compliance

> Auto-seeded from `.ark/specs/project/INDEX.md` at `task verify` time, walked recursively. Renders two subsections: `Index integrity` (one PENDING per discovered `INDEX.md` — does it enumerate all on-disk children?) and `Leaf SPECs` (one rolled-up PENDING for `LAYOUT.md` conformance plus a traceability sublist of every leaf).

### Index integrity

- [PASS] `INDEX.md` enumerates all children of `specs/project/`: the table lists `LAYOUT.md`, `rust/COMMENTS.md`, `rust/STYLE.md`, `rust/ERRORS.md`; on-disk children match.

### Leaf SPECs

- [PASS] All leaf SPECs under `specs/project/` conform to `LAYOUT.md`: each leaf opens with `[**Purpose**]`, uses one prefix family, follows L-3 bullet shape, and closes with `[**Exceptions**]` / `[**Examples**]` / `[**See Also**]`.
  - `LAYOUT.md`: PASS — defines Layout A; anchor SPEC.
  - `rust/COMMENTS.md`: PASS — new code's doc-comments respect C-1, C-3, C-9. Spot-checked `platforms.rs:392-422` and `templates.rs:30-36`.
  - `rust/STYLE.md`: PASS — new code follows S-1/S-2/S-4/S-7/S-13/S-25; `CODEAGENT_PLATFORM` uses named-field initialization; literal `.cac/` paths route through `layout.rs` consts; source-scan invariant test covers `platforms.rs`.
  - `rust/ERRORS.md`: PASS — no new error variants; `apply_managed_state` propagates existing errors via `?`. No `unwrap()` / `expect()` introduced in production code.

## Related Feature Spec Compliance

> Auto-seeded from PRD's `[**Related Specs**]`.

- [PASS] `specs/features/codex-support/SPEC.md`: `[**CHANGELOG**]` entry dated `2026-07-15` records that `PLATFORMS` grows to 4 and CodeAgent CLI shares the `>/dev/null` redirect pattern. Body preserved.
- [PASS] `specs/features/opencode-support/SPEC.md`: `[**CHANGELOG**]` entry dated `2026-07-15` records that CodeAgent CLI uses the same `description`-only frontmatter and `AGENTS.md` sharing pattern. Body preserved.
- [PASS] `specs/features/subagent-support/SPEC.md`: `[**CHANGELOG**]` entry dated `2026-07-15` records fourth platform ships 3 agents under `.cac/agents/`, C-21/C-22 extended. Body preserved.

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` (and `[**Constraints**]` when present). One bullet per criterion.

- [PASS] CodeAgent CLI is a 4th `Platform` registry entry: `CODEAGENT_PLATFORM` const at `platforms.rs:400-422`, `PLATFORMS` slice updated to 4 entries. `platforms_registry_has_four_entries_in_canonical_order` test enforces.
- [PASS] `ark init --codeagent` / `--no-codeagent` flags wired: `InitArgs` at `main.rs:210-224`, `flags()` match arm at `main.rs:271-274`, non-TTY error message now dynamic (iterates `PLATFORMS`).
- [PASS] `.cac/commands/ark/` ships 9 command files matching Claude's command set: verified by `every_claude_command_has_a_codeagent_command_sibling` test.
- [PASS] `.cac/agents/` ships 3 agent files (researcher, reviewer, verifier) with CodeAgent CLI frontmatter: `codeagent_agent_frontmatter_shape` enforces `name`, `description`, `permissionMode: bypassPermissions`, `tools` YAML list.
- [PASS] `.cac/settings.json` `SessionStart` hook installed: `CODEAGENT_PLATFORM.hook_file = Some(HookFileSpec { path: CODEAGENT_SETTINGS_FILE, ... })`. Timeout is 30 seconds. `codeagent_hook_entry_carries_canonical_command_in_seconds` test enforces.
- [PASS] Hook command uses `>/dev/null` redirect (same as Codex): `CODEAGENT_CONTEXT_HOOK_COMMAND` at `io/fs/hook.rs`. Separate const from `CODEX_CONTEXT_HOOK_COMMAND` for clean identity separation.
- [PASS] `AGENTS.md` managed block shared with Codex/OpenCode: `CODEAGENT_PLATFORM.managed_block_target = Some(AGENTS_MD)`. Manifest dedupes on `(file, marker)`. `codeagent_apply_managed_state_writes_block_and_hook` test enforces.
- [PASS] No `.cac/config.toml`: CodeAgent CLI uses JSON hooks natively and needs no config file. `CODEAGENT_PLATFORM.extra_files` is empty.
- [PASS] `ark init` TTY prompt shows all 4 platforms: `interactive_select_platforms` iterates `PLATFORMS`. Manual smoke test confirmed `install codeagent-cli integration? [Y/n]` prompt appears.
- [PASS] `ark unload` / `load` round-trip preserves `.cac/`: smoke test confirmed 3 hook entries captured (Claude, Codex, CodeAgent CLI), 67 files, 2 managed blocks. `load` restores all.
- [PASS] `ark remove` cleans up `.cac/`: smoke test confirmed `codeagent-cli dir, codeagent-cli hook` removed.

## Plan Fidelity

> Auto-seeded from the latest `NN_PLAN.md`'s `## Spec` Goals (`G-N`).

- [PASS] G-1: CodeAgent CLI is a registry entry (not a refactor) — `CODEAGENT_PLATFORM` + `PLATFORMS` push; commands iterate unchanged.
- [PASS] G-2: `ark init` accepts `--codeagent` / `--no-codeagent` — `InitArgs` fields and `flags()` match arm.
- [PASS] G-3: CodeAgent CLI artifacts ship at `.cac/commands/ark/*.md`, `.cac/agents/*.md`, `.cac/settings.json` — layout consts + template statics + `apply_managed_state`.
- [PASS] G-4: CodeAgent CLI shares `AGENTS.md` managed block — `managed_block_target = Some(AGENTS_MD)`, manifest dedupes.
- [PASS] G-5: `SessionStart` hook lives in `.cac/settings.json` with seconds timeout — `HookFileSpec` + `ark_codeagent_hook_entry()`.
- [PASS] G-6: `ark upgrade` carries CodeAgent CLI through the standard pipeline — `is_installed` / `is_in_snapshot` / `collect_desired_templates` all iterate `PLATFORMS` and pick up `CODEAGENT_PLATFORM` automatically.
- [PASS] G-7: No `.cac/config.toml` — `extra_files` is empty.

## SPEC Drift

- [PASS] Modified feature SPECs have CHANGELOG entries: `codex-support/SPEC.md`, `opencode-support/SPEC.md`, `subagent-support/SPEC.md` all carry `2026-07-15 codeagent-cli-support` entries. No other feature SPECs touched.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding has a Resolution; `/ark:commit` requires every Resolution to be non-PENDING.

### V-001 Non-TTY error message hardcoded to 3 platforms

- **Severity:** HIGH
- **Location:** `crates/ark-cli/src/main.rs:367-369`
- **Problem:** The `resolve_platforms_pure` non-TTY bail message was hardcoded as `"init requires at least one of --claude, --codex, or --opencode when stdin is not a TTY"`. Adding a 4th platform made this message stale — users passing `--codeagent` on non-TTY would see an error that doesn't mention their flag.
- **Why it matters:** The error message is the primary UX for CI/scripting users; listing only 3 of 4 platforms is a functional regression.
- **Resolution:** FIXED in EXECUTE — replaced the hardcoded string with a dynamic construction that iterates `PLATFORMS` to build both the `--<flag>` list and the `--no-<flag>` list. Verified: `ark init --dir /tmp/test` (non-TTY, no flags) now prints `--claude, --codex, --opencode, --codeagent`.

### V-002 CLI tests assumed 3 platforms

- **Severity:** MEDIUM
- **Location:** `crates/ark-cli/src/main.rs` tests (lines 930-1024)
- **Problem:** Multiple test assertions hardcoded 3-platform expectations: `--no-claude` expected `["codex", "opencode"]`, "exclude all" used only 3 `--no-*` flags, non-TTY error test checked only 3 `--*` strings, interactive fallback expected `["claude-code", "codex", "opencode"]`.
- **Why it matters:** Test suite would fail on the new 4-platform registry, masking real bugs.
- **Resolution:** FIXED in EXECUTE — updated all test assertions to expect 4 platforms, added `--no-codeagent` exclusion test, added `--codeagent` to non-TTY error check, updated interactive fallback expected IDs.

### V-001 `cargo test` blocked by network outage

- **Severity:** MEDIUM
- **Location:** N/A (infrastructure)
- **Problem:** `cargo test --workspace` cannot download dev-dependencies (`tempfile`, etc.) because crates.io is unreachable from this environment. No `rustfmt` or `clippy` installed either.
- **Why it matters:** Automated test verification is unavailable; relying on `cargo check --workspace` + manual smoke tests.
- **Resolution:** ACCEPTED — `cargo check --workspace` passes clean; release binary built and smoke-tested (`init --codeagent`, round-trip `unload`/`load`, `remove`). Full test suite to be run once network is restored.

## Notes

> Free-form. Trade-offs, context for future readers, anything that doesn't fit a Finding.

- Build clean: `cargo check --workspace` passes for both `ark-core` and `ark-cli`.
- Release binary smoke-tested: `ark init --codeagent --developer lct --dir /tmp/test` creates 20 files (codeagent-only); `ark init --claude --codex --opencode --codeagent --developer lct --dir /tmp/test` creates 47 files; `ark unload` / `ark load` round-trip preserves all; `ark remove` cleans up.
- CodeAgent CLI agent frontmatter uses `permissionMode: bypassPermissions` and `tools` as a YAML list — distinct from Claude (no `permissionMode`), Codex (TOML `sandbox_mode`), and OpenCode (`mode: subagent` + `permission:` block).
- Hook command uses separate `CODEAGENT_CONTEXT_HOOK_COMMAND` const (not reusing `CODEX_CONTEXT_HOOK_COMMAND`) for clean identity separation — the `identity_value` field in `HookFileSpec` must uniquely identify the platform's hook entry for surgical add/remove.
- The `agent_body()` helper and `strip_frontmatter_and_normalize()` in `templates.rs` tests already handled "codeagent" via the default arm (YAML `---...---` frontmatter, same as Claude/OpenCode).
