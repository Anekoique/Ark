# `tier-aware-plan-naming` PRD

---

[**What**]
Standard- and quick-tier tasks seed the design plan as `PLAN.md` (no `NN_` prefix). Deep tier continues to seed `NN_PLAN.md` to support the iteration loop. All existing locators accept both forms so legacy `00_PLAN.md` files in archives remain discoverable.

[**Why**]
Standard tier never iterates the PLAN — only deep tier does. The `00_` prefix on the lone standard-tier plan is dead weight and reads as "the first of several" when it's actually "the only one". Mirrors how `VERIFY.md` is named (no NN, single-shot).

[**Outcome**]
- `task plan` on a standard-tier task seeds `PLAN.md`. Quick tier doesn't seed a plan (unchanged).
- `task plan` on a deep-tier task continues to seed `00_PLAN.md`, `01_PLAN.md`, ... per iteration.
- All readers accept both `PLAN.md` and `NN_PLAN.md`:
  - `latest_plan_goals` (`commands/agent/task/phase.rs`)
  - `find_latest_plan_path` / `plan_iteration_nn` (`commands/agent/spec/extract.rs`)
  - the gather regex / artifact-listing logic (`commands/context/gather.rs`)
  - `read_plan_goals` callers (`commands/agent/task/verify_seed.rs`)
  - any `promote.rs` paths
- `cargo test -p ark-core` is green.

[**Verified**]

- `cargo build` clean.
- `cargo test` clean (402 lib + integration suites green).
- `cargo clippy --all-targets` clean.
- `phase.rs::artifact_for` is now `(Phase, Tier, u32)`; standard/quick → `PLAN.md`, deep → `NN_PLAN.md`.
- All locators accept both forms: `phase::latest_plan_goals` (now via shared `locate_latest_plan`), `verify_migration::latest_plan_goals`, `spec::extract::find_final_plan`, `gather::classify_artifact`, `discard::template_for`.
- `task_promote(Standard → Deep)` renames `PLAN.md` → `00_PLAN.md` to preserve the body and prep for deep-tier iteration. Test `legal_promotion_preserves_artifacts` updated.
- Test `standard_design_to_plan_to_execute_to_verify` (phase.rs) and `standard_tier_archive_after_commit` (agent_lifecycle.rs integration) updated to expect `PLAN.md`.
- Re-checked: existing committed tasks and archived tasks under `.ark/tasks/archive/` still use `00_PLAN.md`; the dual-form locators read them unchanged.

[**Spec note**]
No feature SPEC pinned the filename (`ark-workflow-refactor/SPEC.md` doesn't enumerate artifact filenames). No spec amendment needed. The behavior is documented inline at `phase.rs::artifact_for` and `promote.rs` module docs.

[**Related Specs**]

- `.ark/specs/features/ark-workflow-refactor/SPEC.md` — describes the artifact-naming convention. Amend if it pins `NN_PLAN.md` for non-deep tiers.
