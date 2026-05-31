# `fix-workspace-init` PRD

---

[**What**]

`ark init` scaffolds the top-level `.ark/workspace/index.md` and, when a developer identity is established, the per-developer `.ark/workspace/<dev>/index.md` skeleton.

[**Why**]

Today `ark init` only writes `.ark/.developer` (when `--developer` / prompt resolves). The workspace tree the SPEC describes (top-level Active Developers index, per-developer Session History index) is created lazily on the first `workspace_record`, so a freshly-initialized project has no visible `.ark/workspace/` at all. The user reports this is a gap — init should leave the workspace ready, not deferred.

[**Outcome**]

After `ark init` (default flags, on a TTY with developer entered or `--developer alice`):

- `.ark/workspace/index.md` exists and contains the static preface and an empty `<!-- ARK:DEVELOPERS -->` managed block (with a row for the resolved developer if one was provided).
- `.ark/workspace/<dev>/index.md` exists with the static preface and an empty `<!-- ARK:SESSIONS -->` managed block when an identity was resolved.
- After `ark init --no-developer`, `.ark/workspace/index.md` still exists (empty `ARK:DEVELOPERS` block); no per-developer dir is created.
- Re-running `ark init` is idempotent: existing developer rows and session-history rows are preserved.
- `cargo test -p ark-core` and `cargo test -p ark-cli` pass.

[**Related Specs**]

- `.ark/specs/features/workspace/SPEC.md` — G-3 (top-level + per-developer indices), G-4 (`ark init --developer` bootstrap). Behaviour stays within the existing managed-block contract; scaffolding is moved earlier in the lifecycle.
