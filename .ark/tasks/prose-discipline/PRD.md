# `prose-discipline` PRD

---

[**What**]
Tighten the slash-command instructions for the `Main Changes` table in `/ark:commit` so agent-authored journal entries stay short. Also rewrite the existing Sessions 2–4 in `.ark/workspace/Anekoique/journal-1.md` under the new style.

[**Why**]
Each `task commit` produces a `## Session N` block whose `Main Changes` table grows into multi-paragraph rows with nested clauses, repeated context, and ceremony rows like `tests` / `template parity` / `spec` that don't carry independent meaning. Reading three back-to-back sessions feels like reading three release notes when one row each would do. The session-level redesign is deferred; the prose discipline is independent and lands first.

[**Outcome**]
- `templates/claude/commands/ark/commit.md`, `templates/codex/skills/ark-commit/SKILL.md`, and `templates/opencode/commands/ark/commit.md` each gain a short `Style` subsection in step 4 capping the table at ≤4 rows with ≤80-char single-line cells, no nested code blocks, no rows for incidental ceremony (tests, template parity, follow-on doc updates) unless the test/template *is* the change.
- The matching `Summary` instruction reinforces "≤1 line, lead with the user-visible effect."
- All three templates stay in sync (existing parity tests in `crates/ark-core/src/templates.rs` will fail if they diverge — that's the regression net).
- Existing Sessions 2, 3, 4 in `.ark/workspace/Anekoique/journal-1.md` are rewritten under the new style. Auto-fields (`**Date**`, `**Slug**`, `**Branch**`, `**Base Branch**`, `**Start Head**`, `**Closing Commit**`, `### Git Commits`) are preserved verbatim.
- `cargo test -p ark-core` is green (template-parity tests still pass).

[**Verified**]

- All three commit-slash templates carry the new `Style — keep it tight` paragraph (Claude has the full version after the example block; Codex/OpenCode carry the condensed paragraph). Template parity tests pass.
- Sessions 2/3/4 in `.ark/workspace/Anekoique/journal-1.md` rewritten: row counts dropped from 5/4/5 to 2/3/2; max description-cell length now 80/67/69 (was 250+/200+/300+). Auto-fields and `### Git Commits` tables preserved verbatim.
- `cargo test` clean across all suites (402 + integration).

[**Related Specs**]

- None. The journal block schema is documented inline in the slash-command templates; no feature SPEC pins prose length.
