# `ark agent` namespace PRD

---

[**What**]

Introduce a hidden `ark agent` subcommand namespace that packages the structural operations Ark's workflow currently asks the agent to perform by hand (file creation, TOML edits, template copies, managed-block updates, directory moves). Ship the namespace with the concrete subcommands needed to drive the existing quick/standard/deep lifecycle end-to-end.

[**Why**]

`.ark/workflow.md` spells out the lifecycle as a sequence of `mkdir`/`cp`/`echo`/manual-TOML-edit steps the agent executes via Bash. Each step is mechanical but easy to get subtly wrong — mangled TOML, malformed managed blocks, archive moves that forget `.ark/tasks/.current`, deep-tier SPEC extraction that drops the final PLAN's section header. Packaging these as hidden `ark agent` subcommands gives the workflow a single correctness contract: the binary owns the mechanical mutations, the agent owns the judgment (PRD content, review verdicts, spec prose).

The namespace is also the foundation the separately-planned workspace/journal feature will sit on top of; designing it in isolation now avoids retrofitting a namespace contract later.

[**Outcome**]

- `ark agent` exists as a hidden top-level subcommand group; `ark --help` does not list it.
- `ark agent --help` prints a banner describing the namespace's purpose and explicit non-stability (not covered by semver).
- The following subcommands exist and each performs exactly one workflow step:
  - `ark agent task new` — creates task dir, copies PRD template, writes `task.toml`, sets `.ark/tasks/.current`.
  - `ark agent task plan` / `task review` / `task execute` / `task verify` / `task archive` — explicit phase transitions, each guarded against illegal transitions per tier.
  - `ark agent task iterate` — deep-tier review iteration (copies `NN_PLAN.md` + `NN_REVIEW.md` at next iteration number).
  - `ark agent task promote --to <tier>` — tier change mid-flight.
  - `ark agent task reopen --slug <s>` — restore an archived task; refuse on slug collision.
  - `ark agent spec extract` — pull the final PLAN's `## Spec` section into `specs/features/<slug>/SPEC.md`; append CHANGELOG when overwriting.
  - `ark agent spec register` — add/update the corresponding row in `specs/features/INDEX.md`'s managed block.
  - `ark agent template copy --name <t> --to <path>` — write an embedded template file to disk.
- Every subcommand writes to disk and prints a one-line summary; no structured-data piping between commands.
- `task archive` is a single command that reads `task.toml.tier` and performs the deep-tier SPEC extract + register automatically.
- Illegal phase transitions return a named error (e.g. `Error::IllegalPhaseTransition`) — they do not silently succeed or panic.
- `workflow.md` is updated to reference `ark agent` commands instead of raw `mkdir`/`cp`/`echo` recipes.
- Full test coverage: each subcommand has a `tempfile::tempdir()`-backed unit test; a round-trip integration test walks a synthetic standard-tier task from `new` → `archive`.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all pass.

[**Related Specs**]

None. No project or feature specs are populated in `.ark/specs/` at the time of this task.
