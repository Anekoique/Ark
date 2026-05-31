# `commit-stage-all` PRD

---

[**What**]

Add a `-a` / `--all` option to `ark agent task commit` (surfaced through `/ark:commit`) that stages every tracked + untracked change before the closure commit.

[**Why**]

Users almost always `git add -A` the whole working tree right before `/ark:commit`. Folding that into a single `-a` flag removes the manual staging step and the `NothingStaged` round-trip.

[**Outcome**]

- `ark agent task commit -a -m "<msg>"` runs `git add -A` (in the task cwd) before the staged-work gate, then commits as usual.
- Without `-a`, behavior is unchanged: the user must stage work first or hit `NothingStaged`.
- `-a` is incompatible with `--no-commit` (nothing to stage for) — clap rejects the combination.
- `/ark:commit` doc documents `-a` in the argument hint and Step 6.
- `cargo build`, `cargo test --workspace`, `cargo clippy -- -D warnings`, `cargo fmt --check` all pass; a new test covers the stage-all path.

[**Related Specs**]

- `.ark/specs/features/ark-agent-namespace/SPEC.md` — `ark agent task commit` is part of the agent namespace; this adds a flag to it.

[**SPEC Path**]

