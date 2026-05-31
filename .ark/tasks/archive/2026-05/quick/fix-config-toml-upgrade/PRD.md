# `fix-config-toml-upgrade` PRD

---

[**What**]

Teach `ark upgrade` to append top-level TOML sections that the current template ships but the user's `.ark/config.toml` is missing, while still never overwriting an existing section.

[**Why**]

Today `is_seed_only(CONFIG_FILE) == true`, so upgrade skips `config.toml` entirely. Consequence: when a release adds a new top-level section (e.g. the `[sandbox]` block shipped with `ark-sandbox`), users who initialized before that release never see it — their `config.toml` silently lacks the new defaults and they have to hand-copy from the template. This very repo's `.ark/config.toml` is missing `[workspace]`, `[upgrade]`, and `[sandbox]`.

[**Outcome**]

- `ark upgrade` on a project whose `.ark/config.toml` lacks one or more template-shipped top-level tables appends each missing table verbatim (header + body, including leading comments) to the user's file, preserving the user's existing content byte-for-byte.
- A top-level table the user already has — even with edits — is **never** rewritten or merged. Section identity is the `[name]` header at column 0.
- A `--dry-run` reports the planned append as a single action row per missing section and mutates nothing.
- The file is backed up like any other touched path; `--restore` rolls it back.
- `config.toml` listed under `[upgrade] ejected` still wins — no append at all.
- Existing tests stay green; a new unit test asserts the append-missing-sections behavior, and `upgrade_does_not_overwrite_config_toml` still passes unchanged.
- `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` all green.

[**Related Specs**]

- `specs/features/ark-upgrade/SPEC.md` — adds a narrow exception to the seed-only carve-out (C-1 area): `config.toml` is still never overwritten or hash-classified, but missing top-level sections are appended. No change to ejection (C-2/C-3) or merge (C-7..C-11) semantics.

[**SPEC Path**]

n/a (quick tier).
