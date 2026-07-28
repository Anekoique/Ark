# `optimize-journal-write-prompt` PRD

---

[**What**]

Make every commit-workflow prompt state the journal's append-at-EOF contract explicitly.

[**Why**]

Agents can interpret the current bare "append" instruction as placing a new session
beside numerically adjacent headings. That rewrites or reorders history and leaves the
fresh unstamped session somewhere other than the final journal entry, so
`ark agent task commit` rejects it.

[**Outcome**]

- Commit skills/slash commands and `ark-workflow` require one new block at physical
  EOF, without inserting beside or moving/reordering existing sessions.
- Before commit, the new unstamped block is verified as the last session block.
- Embedded templates and installed dogfood copies remain byte-for-byte aligned.

[**Related Specs**]

- `.ark/specs/features/workspace/SPEC.md` — clarifies the existing final-heading
  contract enforced by workspace journal stamping.

[**SPEC Path**]

<!-- ignored for quick tier -->
