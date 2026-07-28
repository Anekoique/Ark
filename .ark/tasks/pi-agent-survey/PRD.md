# Survey Pi agent harness for Ark and ArkOS

---

[**What**]

Survey the Pi coding-agent project (the current `earendil-works/pi` ecosystem,
formerly `badlogic/pi-mono`) as a minimal, extensible personal agent harness,
then determine whether Ark should support or borrow from it and whether ArkOS
should treat it as a runtime, implementation reference, or neither.

The corpus will answer four bounded questions:

1. What Pi actually ships: repository/package structure, agent loop, model and
   provider abstraction, tools, sessions, context handling, event model, SDK,
   RPC/embedding surfaces, extensions, skills/prompts, UI modes, license, and
   maintenance signals.
2. Where Pi sits relative to Ark's human-gated workflow harness and ArkOS's
   proposed agent-facing workflow substrate; this comparison must preserve the
   layer boundaries in `docs/rfcs/001-arkos.md`.
3. Which adoption modes are credible: add Pi as an Ark host-platform target,
   reuse selected Pi conventions, use or wrap Pi components for an ArkOS
   runtime, or keep Pi as prior art only.
4. What should be tried next, if anything, including a small validation spike
   with explicit success and rejection criteria. No integration code is in
   scope for this research task.

[**Why**]

Pi is gaining attention as a compact, hackable coding-agent harness and may
cover runtime capabilities that Ark deliberately delegates to vendor agents and
that ArkOS eventually expects beneath its substrate. A source-backed survey is
needed before adding another Ark platform, depending on Pi APIs, or copying
architectural ideas across layers. The decision must be grounded in Pi's
current code and official documentation rather than popularity or positioning
claims.

[**Outcome**]

A curated research corpus under `research/` that:

- provides a concise start-here synthesis and a source-linked architecture map;
- separates verified facts, repository-derived inferences, and recommendations;
- includes a capability/boundary matrix for Pi, Ark, and ArkOS;
- gives explicit `adopt now`, `prototype`, `watch`, or `reject` calls for each
  plausible reuse or integration path;
- records risks around API stability, security/trust, session durability,
  portability, maintenance, and license compatibility; and
- ends with a concrete recommendation whose evidence is sufficient that more
  broad searching is unlikely to change it.

[**Related Specs**]

- `.ark/specs/features/codeagent-cli-support/SPEC.md` — comparison point for
  adding a new host-agent platform through Ark's registry and templates.
- `.ark/specs/features/subagent-support/SPEC.md` — defines the subagent runtime
  contract Pi would need to satisfy for first-class Ark support.
- `.ark/specs/features/ark-context/SPEC.md` — defines the structured context
  surface and phase projections that a Pi integration would need to consume.
- `.ark/specs/features/ark-sandbox/SPEC.md` — defines Ark's existing execution
  isolation boundary and prevents conflating Pi's tool execution with sandboxing.

[**SPEC Path**]

Ignored for this research-tier task.
