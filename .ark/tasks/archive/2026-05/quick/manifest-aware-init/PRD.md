# `manifest-aware-init` PRD

---

[**What**]
`ark init` resolves its platform set from `.ark/.installed.json` when no `--<platform>` / `--no-<platform>` flag is passed and the manifest already exists. Skips the interactive prompt in that case. Drops the "init requires at least one platform" error path on re-init when the manifest carries a non-empty set.

[**Why**]
Today `resolve_platforms` (in `crates/ark-cli/src/main.rs`) only checks CLI flags and TTY state. On a project that's already Ark-installed, re-running `ark init --developer <name>` re-prompts for every platform. If the user answers `n` to all (because they don't want to add anything new), the call errors out with "init requires at least one platform" — even though the project already has a complete install.

The manifest at `.ark/.installed.json` records exactly which platform trees are installed (their files live under `.claude/`, `.codex/`, `.opencode/`). Reading it gives a deterministic default that matches the current install, removes the prompt, and avoids the empty-set error.

[**Outcome**]
- When `.ark/.installed.json` exists and no platform flags are passed:
  - Derive the platform set from manifest entries by matching each platform's `dest_dir` (`.claude`, `.codex`, `.opencode`) against `manifest.files` prefixes.
  - Skip the interactive prompt.
  - Print one stderr line stating which platforms were detected.
- When `.ark/.installed.json` is absent, behavior is unchanged: prompt on TTY, error on non-TTY.
- When platform flags are passed, behavior is unchanged: flags win over the manifest.
- The "init requires at least one platform" guard still fires, but only when the resolved set (from flags, manifest, or prompt) is genuinely empty.
- `cargo test -p ark-cli` is green; new unit tests cover (a) manifest-derived defaults, (b) flag override of manifest, (c) flag-only behavior on a fresh install.

[**Verified**]

- `cargo build` clean.
- `cargo test` clean (402 lib + all bin/integration suites green; 4 new tests in `crates/ark-cli/src/main.rs`).
- Smoke test against this repo: `ark init --developer Anekoique` (no flags, manifest present) prints `note: detected installed platforms (claude-code, codex, opencode); use --<platform> / --no-<platform> to override` and proceeds without prompting. 29 files / 0 created / 27 unchanged / 2 skipped — install was idempotent.
- Existing tests `cli_resolve_platforms_no_x_excludes`, `_positive_flags_narrow`, `_no_flags_non_tty_errors` still pass (helper now passes `installed = None`).
- New tests cover (a) manifest-derived defaults skip the prompt, (b) positive flag overrides manifest, (c) negative flag overrides manifest, (d) empty manifest falls through to interactive.

[**Spec note**]
Added `Manifest` to the public `ark_core` re-export list (`crates/ark-core/src/lib.rs`) so the CLI can read it. `Platform::dest_dir` was already public. No feature SPEC pinned the resolution policy.

[**Related Specs**]

- None. `Manifest` is already public via `crate::state` re-exports. `Platform::dest_dir` is already `pub`. No SPEC enumerates the `ark init` resolution policy.
