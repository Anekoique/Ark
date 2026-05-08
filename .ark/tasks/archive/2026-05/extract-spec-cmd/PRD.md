# `extract-spec-cmd` PRD

---

[**What**]

Add `/ark:extract-spec <feature-name> [hint]` slash command and a backing `ark agent spec import` CLI verb that together let users author a feature SPEC from an existing codebase (brownfield adoption), instead of producing one as a deep-tier promotion byproduct.

[**Why**]

Ark's feature-SPEC layer assumes SPECs are emitted by the deep-tier workflow (`task commit` extracts the final PLAN's `## Spec`). For projects adopting Ark mid-life — an OS kernel with copy-on-write already implemented, a web app with auth already shipped — there is no PLAN to extract from, but the project still needs SPECs under `specs/features/` so VERIFY can check adherence and future deep-tier work has something to reference. Today the only options are (a) write SPECs entirely by hand or (b) reverse-engineer a fake deep-tier task. Both bypass discipline (no managed-block INDEX upsert; no provenance trace).

[**Outcome**]

A user in a brownfield project can run:

```
/ark:extract-spec copy-on-write "memory subsystem COW for fork()"
```

and, after a confirm gate (mandatory: review candidate files/symbols/commits, supply one-line intent), end up with:

1. `.ark/specs/features/copy-on-write/SPEC.md` — body matches the feature-SPEC template (Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints), authored by the AI from confirmed evidence.
2. A first `[**CHANGELOG**]` entry on that SPEC: `YYYY-MM-DD extracted from <git-head-sha>: initial extraction from codebase`.
3. A row in `.ark/specs/features/INDEX.md` with `from-task = "extracted"`, registered through the existing managed-block discipline (not a hand-rolled write).

The CLI verb `ark agent spec import` performs steps 1–3 atomically; the slash command performs the discovery + confirm + synthesis, then hands the SPEC body to the CLI. No CLI changes to existing `spec extract` (PLAN-based) or `spec register` (INDEX-only). Concretely:

- `ark agent spec import --feature <slug> --scope "<one-line>" --from-file <path> [--from-commit <sha>] [--date YYYY-MM-DD]` succeeds end-to-end on a clean target, refuses on existing SPEC, validates `--feature`/`--scope` per existing `Error::InvalidSpecField` rules, and prints a one-line `Display` summary.
- `/ark:extract-spec` is published for all three platforms (claude/opencode/codex) under `templates/<platform>/commands/ark/extract-spec.md`, and the materialized copies under `.claude/`, `.opencode/`, `.codex/` are regenerated.
- `ark upgrade` recognizes the new template (no manual reconciliation needed on existing installs).
- A first end-to-end smoke test from this very repo: extract a SPEC for an existing module (e.g. the `commands/agent/spec/` group), confirm the produced SPEC is correctly registered and the INDEX row appears, then discard it.

[**Related Specs**]

- `specs/features/ark-agent-namespace/SPEC.md` — adds a third verb under `spec` (alongside `extract` and `register`); inherits the namespace's hidden-subcommand, no-semver, `Display`-summary, managed-block, and error-variant disciplines (G-1, G-5, C-1..C-13).
- `specs/features/project-spec/SPEC.md` — produced SPEC body must be authored to match feature-SPEC template shape (the convention layer already governs project SPECs; feature SPECs follow the runtime-system template under `.ark/templates/SPEC.md`). The CHANGELOG provenance entry reuses the same `[**CHANGELOG**]` convention deep-tier amendments use.
- `specs/features/ark-context/SPEC.md` — no direct interaction; extraction does not consume context envelopes (it operates outside the design/plan/review/execute/verify lifecycle).
