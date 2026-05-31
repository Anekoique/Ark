# spec-extract-self-contained PRD

---

[**What**]
Require the final iteration's `## Spec` section in deep-tier PLANs to be **self-contained** — restate Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints in full, not as a delta against earlier iterations. Document the rule in `templates/ark/workflow.md` (synced to `.ark/workflow.md`), in the `/ark:design` slash command's iteration step, and in the `templates/PLAN.md` seed.

[**Why**]
The `ark-context` task surfaced this gap in archive (2026-04-27): `02_PLAN.md` was written as a delta document ("inherited from 01 with C-21 narrowed, plus C-27/C-28/C-29"). On archive, `ark agent spec extract` faithfully copied that 6-line delta into `.ark/specs/features/ark-context/SPEC.md`, producing a feature SPEC that was useless to anyone reading it without the iteration history. The full SPEC had to be hand-synthesized from 00/01/02_PLANs after the fact (see `archive/2026-04/ark-context/VERIFY.md` and the SPEC's first commit).

The simplest fix is a documentation-and-template change: tell future deep-tier authors the rule, and seed it visibly in the PLAN template so it's read every iteration. No code change in `ark agent spec extract` — the extractor is correct; the input was wrong.

[**Outcome**]

- `templates/ark/workflow.md` §4 PLAN stage carries an explicit rule: "The latest iteration's `## Spec` section MUST be self-contained. Restate Goals / NG / Architecture / Data Structure / API Surface / Constraints in full, including unchanged items. The `## Log` above is for the delta narrative." (and `.ark/workflow.md` is synced from the template).
- `templates/claude/commands/ark/design.md`'s "iteration" step (3.3, the deep-tier copy-and-revise step) names the rule in plain terms: when copying `NN_PLAN.md` → `NN+1_PLAN.md`, the new `## Spec` must be self-contained.
- `templates/ark/templates/PLAN.md` (the seed copied into `NN_PLAN.md` by `ark agent task plan`) carries an inline reminder above the `## Spec` heading so authors see it every iteration.
- The REVIEW gate is reinforced by the workflow doc: a final-iteration PLAN whose `## Spec` references prior iterations by phrases like "carried forward," "unchanged from," or "see NN_PLAN" is a HIGH finding and must be rejected.
- No Rust code change. `ark agent spec extract` is unchanged.
- `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets -- -D warnings` all green (no regressions; tests don't cover doc text but the build picks up `include_dir!()` template changes).

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — touches the meaning of `ark agent spec extract`'s contract, but does NOT change its behavior. The fix is upstream in PLAN-authoring discipline. We add a sentence to `ark-agent-namespace`'s SPEC documenting that the extractor's correctness depends on a self-contained `## Spec` section being passed in. CHANGELOG entry will be appended on archive of this task.
