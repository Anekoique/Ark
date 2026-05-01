# `fix-project-index-upgrade` PRD

---

[**What**]
Exempt `.ark/specs/project/INDEX.md` (and the rest of `.ark/specs/project/`) from `ark upgrade`'s classification pass so users are never prompted to overwrite their project-spec index.

[**Why**]
`project/INDEX.md` is documented as user-authored ("only edited and modified by user, never grant agents the power to modify"). Today `ark upgrade` still hashes it against the shipped template; once the template ships any wording change, real installs hit `Classification::UserModified` and prompt `[o]verwrite / [s]kip / [c]reate .new?`. That risks clobbering the user's spec roster on a careless keypress. The same exemption already applies in spirit to `.ark/config.toml` (layout.rs:51-55 says "upgrade does NOT overwrite") but is also unenforced.

[**Outcome**]
- [x] Running `ark upgrade` on a project whose `.ark/specs/project/INDEX.md` differs from the template produces no prompt and leaves the file byte-for-byte unchanged. — verified end-to-end against a fresh `/tmp` sandbox: upgrade summary `0 modified-preserved`, file content matches the user edit byte-for-byte.
- [x] Same for `.ark/config.toml` and any user file under `.ark/specs/project/`. — covered by `is_exempted` returning true for `.ark/config.toml`, `.ark/specs/project/INDEX.md`, and any path under `.ark/specs/project/`.
- [x] New unit test added: `is_exempted_matches_manifest_and_seed_only_paths` (plan.rs) plus integration-style test `upgrade_does_not_prompt_for_seed_only_paths_under_interactive_policy` (mod.rs) using `PanicPrompter` to prove no prompt fires.
- [x] `cargo test -p ark-core -p ark-cli` passes (322 ark-core + 20 ark-cli upgrade tests, all suites green).
- [x] `ark upgrade` smoke run on a sandbox project with a mutated `project/INDEX.md` and `config.toml` returns without prompting and preserves both files.

[**Related Specs**]

- `.ark/specs/features/ark-upgrade/SPEC.md` — adds a "seed-only paths" carve-out to the classification model.
- `.ark/specs/features/project-spec/SPEC.md` — formalizes that `project/INDEX.md` is user-owned post-seed.
