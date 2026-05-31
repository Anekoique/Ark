# `categorize-ark-archive` PRD

---

[**What**]

Reorganize the archive on disk into `tasks/archive/<YYYY-MM>/<tier>/<slug>/` and add a tier-grouped `tasks/archive/INDEX.md`, so archived tasks are categorized by tier as well as by month. Update the `ark archive` write path, the `ark context` archive scan, `ark cleanup`, their tests, and the `workflow.md` layout convention to match.

[**Why**]

The archive is bucketed only by `YYYY-MM`. Finding a past task of a given tier means opening every month directory and reading each `task.toml`. Grouping by tier — both physically (`<month>/<tier>/<slug>/`) and in an index — makes archived tasks quick to locate. The month stays the top bucket so chronology is preserved.

[**Outcome**]

- Archive layout is `tasks/archive/<YYYY-MM>/<tier>/<slug>/`. All 32 existing archived tasks are moved into the correct `<month>/<tier>/` directory; no `task.toml` content changes (tier already recorded there).
- `ark archive` (`commands/archive.rs`) writes new archives to `<month>/<tier>/<slug>/`. The tier is read from the task being archived.
- `ark context` (`commands/context/gather.rs`) scans the new layout and still lists recent archived tasks correctly.
- `ark cleanup` (`commands/cleanup.rs`) operates on the new layout.
- `tasks/archive/INDEX.md` exists: one section per tier present (deep / standard / quick / research), each headed by its count; every archived task appears exactly once with month, a relative link to its directory, and its title; rows sorted by month then slug.
- `workflow.md` documents the new `tasks/archive/<YYYY-MM>/<tier>/<slug>/` convention (replacing `YYYY-MM/<slug>/`).
- `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` all pass.
- E2E smoke test (load → unload → load → remove round-trip) still passes; archived dirs survive an unload/load round-trip.

[**Related Specs**]

(none currently — the archive layout is defined in code/workflow.md, not a feature SPEC. PLAN to confirm during REVIEW.)
