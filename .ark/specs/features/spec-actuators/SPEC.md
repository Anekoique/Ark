
> Promoted verbatim to `specs/features/spec-actuators/SPEC.md` on deep commit.

[**Goals**]

- G-1: A SPEC constraint may declare how it is enforced via an inline actuator tag.
- G-2: The grammar defines four actuator kinds with one honest default (judgment).
- G-3: `ark-verifier` honors a rule's actuator tag instead of re-judging mechanical rules.
- G-4: An `/ark:spec-audit` skill reports tag health and offers self-fix or agent-assisted.
- G-5: Workflow templates state that workflow IDs never appear in shipped source.

[**Non-goals**]

- NG-1: No native enforcement engine in `ark-core`; tags are convention + agent behavior, not Rust code.
- NG-2: No mechanization of `judgment` rules; agent reasoning stays the actuator.
- NG-3: No language-specific assumption in the schema; tags name commands/tests/patterns, never a toolchain.
- NG-4: Tagging is optional for users; an untagged rule is a valid judgment rule, never an error.

[**Architecture**]

```
.ark/specs/project/
├── LAYOUT.md                                  (*) new L-9: inline actuator tag grammar; L-3 permits trailing tag
└── rust/{COMMENTS,STYLE,ERRORS}.md           (*) every [**Rules**] bullet carries a tag (Ark's own dogfooding)

templates/ark/specs/project/INDEX.md           (*) optional-tags note (shipped scaffold; user-facing)
templates/ark/templates/SPEC.md                (*) feature-SPEC constraints may carry a tag (optional)
templates/ark/workflow.md                      (*) EXECUTE: workflow IDs are bookkeeping, never in source
templates/{claude,codex,opencode}/commands|skills/.../spec-audit   (NEW) audit skill, per platform
templates/{claude,codex,opencode}/agents/ark-verifier.*            (*) honor actuator tags; fail-closed on unresolved tool/test
```

Enforcement is entirely **convention + agent**: there is no Ark binary code for actuators. `ark-verifier` (and `/ark:spec-audit`) read a rule's tag and act — running the project's `tool`/`test` command via the verifier's existing auto-discovery, grepping for a `source-scan` pattern, or reasoning over a `judgment` rule. Ark ships the grammar and the agent instructions, nothing more.

[**Data Structure**]

No Rust types. The actuator tag is a Markdown token; its grammar lives in `LAYOUT.md` L-9.

[**API Surface**]

No code API. The shipped surface is: the L-9 grammar, the `/ark:spec-audit` skill (three platforms), and the `ark-verifier` rubric addition.

[**Constraints**]

- C-1: An actuator tag is the final element of a rule's first line, `⟨@<kind>: <arg>⟩` before terminal punctuation; `<arg>` is bounded by the closing `⟩`, so a backticked path token inside it never terminates the tag.
- C-2: `<kind>` is `tool` | `source-scan` | `test-binding` | `judgment`; any other kind is malformed.
- C-3: A rule with no tag is reported as un-enforced (a judgment rule), never silently treated as enforced and never an error.
- C-4: `tool` names a project lint/format command key resolved by the verifier's existing auto-discovery; the schema names no toolchain.
- C-5: `source-scan` carries `<pattern> @ <glob>`; the pattern is a forbidden token checked against comment lines of files matching the glob, run by the agent.
- C-6: `test-binding` names a concrete project test id that must exist and pass, never a `V-*` workflow label.
- C-7: `judgment` may carry an optional proxy pattern flagging review candidates; it never fails the build.
- C-8: A `tool`/`test-binding` whose key resolves to no command is a VERIFY FAIL, never a silent pass.
- C-9: L-9 documents the supported `<glob>` (`**`, `*`, literals) and `<pattern>` token grammar (`\d`, `\d+`, `\d{N}`, `(a|b)`).
- C-10: `ark-verifier` applies every project-SPEC rule; for `tool`/`source-scan`/`test-binding` it consumes that kind's deterministic result, reasoning only over `judgment` and untagged rules.
- C-11: The `/ark:spec-audit` skill is read-only by default; it applies edits only after a per-run user choice of agent-assisted mode.
- C-12: The three platform bodies (`ark-verifier`, `spec-audit`) are byte-identical modulo per-platform frontmatter/heading idioms (subagent-support C-22).
- C-13: Tagging is optional for user projects; Ark's own convention SPECs adopt full tagging as a self-imposed convention, not a shipped requirement.
- C-14: No workflow ID (`V-*-N`, `C-N`, `G-N`, `R-NNN`) appears in any `crates/` comment; the constraint goes inline as prose, the label nowhere.

---
