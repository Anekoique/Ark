# `ark-book` VERIFY

> Status: Closed
> Feature: `ark-book`
> Owner: Verifier (self-review)
> Target Task: `ark-book`
> Verify Scope:
>
> - Plan Fidelity         — does the code deliver what the final PLAN promised?
> - Functional Correctness — does it work under the Validation matrix?
> - Code Quality          — readability, naming, error handling, test depth
> - Organization          — module boundaries, file placement, cohesion
> - Abstraction           — appropriate abstractions; no premature, no leaky
> - SPEC Drift            — does PLAN's Spec section still match the shipped code?

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 1

## Summary

The shipped change matches the PLAN. Both phases — comment cleanup and book authoring — landed at the documented scope, with the documented exclusions, and all four toolchain gates pass.

**G-1: Zero task-mark tags in `crates/` source comments.** Verified by `grep -rEn '\b(V-IT-|V-UT-|V-E-|V-F-|G-|C-|R-|NG-|TR-|FU-|M-)[0-9]+\b' crates/` returning zero hits in comments. The remaining hits in `crates/ark-core/src/commands/agent/spec/extract.rs` (7 sites) and `crates/ark-cli/tests/agent_lifecycle.rs` (2 sites) are inside *string literals* — markdown fixtures the SPEC parser tests itself against. PLAN's V-E-1 explicitly carved these out; they are not comments.

**G-2: Stripped comments stay grammatical.** Per-edit review during execution; no orphan punctuation, no `Per :` artifacts, no broken doc-test code blocks. Test-name-derived docstrings (e.g. `/// V-IT-1: task_new_worktree_happy_path.`) were rewritten to behaviorally describe the test rather than just reference its old tag.

**G-3: Source-scan invariant tests preserved.** The `*_source_no_bare_std_fs_or_dot_path_literals` tests in `platforms.rs`, `init.rs`, `remove.rs`, `unload.rs`, `load.rs` continue to pass. Their docstrings/messages dropped tag references; their behavioral assertions (the actual `assert_source_clean` call) are byte-identical to pre-cleanup. The umbrella `commands_no_bare_command_new` test in `context/mod.rs` likewise survived.

**G-4: `docs/book/` builds cleanly.** `mdbook build docs/book` exits 0 with no `WARN` or `ERROR` lines on stderr, producing `docs/book/book/`. `book.toml` validates against mdBook 0.4.40.

**G-5: Five-part book.** Introduction + four parts as PLAN specified. Each part sourced from existing canonical material (README, AGENTS.md, .ark/workflow.md, the seven feature SPECs). The Contributing chapters add the most net-new prose, but their content is drawn from AGENTS.md (workspace layout) and the feature SPECs (adding-a-platform, adding-a-slash-command, release-process).

**G-6: GH Actions deploy workflow.** `.github/workflows/book.yml` triggers on `push` of `v*` tags and on `workflow_dispatch`. Uses `actions/configure-pages@v5` + `actions/upload-pages-artifact@v3` + `actions/deploy-pages@v4`. Permissions declared (`contents: read`, `pages: write`, `id-token: write`). Concurrency set to `pages` group, `cancel-in-progress: false` so concurrent tag pushes serialize.

**G-7: Toolchain gates green.** `cargo build --workspace`, `cargo test --workspace` (321 pass, 0 fail), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` — all pass.

PLAN's `## Spec` section still matches the shipped code; no SPEC drift.

## Findings

### V-001 — book.toml `multilingual = false` removed during execute

- Severity: LOW
- Scope: Plan Fidelity
- Location: `docs/book/book.toml`
- Problem:
  PLAN's Data Structure section listed `multilingual = false` as part of the `[book]` table. Local `mdbook build` rejected this field with `unknown field \`multilingual\``; mdBook 0.4.40 does not accept it. The field was removed during execute.
- Why it matters:
  Minor PLAN/code divergence. The intent (English-only, no i18n) is captured by NG-5 anyway, so removing the explicit setting is fine — it's the implicit default.
- Expected:
  No action; the divergence is documented here. Future PLANs should validate `book.toml` shape against the actual mdBook version before committing.

## Follow-ups

None. The task ships as a single milestone; the version bump is user-driven (NG-1) and there's no soak window or follow-up cleanup required.
