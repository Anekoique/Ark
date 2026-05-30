# BENCHMARK — Plan

A plan for building an evaluation harness that measures the quality contribution of Ark's current workflow design — specifically, whether the PLAN ⇄ REVIEW loop, the sub-agent dispatch model, and the artifact-driven discipline produce measurable benefit over thinner agent scaffolds.

This document describes **what to build** and **how to use it**. Execution happens in a separate session.

---

## Goal

Build a reproducible benchmark suite that, given a fixed task PRD and a fixed starting git state, runs Ark under multiple harness configurations and reports per-configuration quality and cost metrics.

The suite must:

1. Run on the **current Ark codebase** — no future-task dependency.
2. Use **already-shipped Ark behaviour** (deep / standard / quick tiers, REVIEW loop, sub-agent dispatch) as comparison points.
3. Produce **comparable numeric output** (resolve rate, finding counts, token cost, wall-clock) across runs.
4. Be **cheap enough to iterate on** (single-fixture run ≤ $5; full suite ≤ $300).

What this benchmark is *not*:

- Not a third-party benchmark integration (SWE-bench etc.) — too expensive and indirect.
- Not a one-off audit of historical archives — those measure convergence, not counterfactual quality.
- Not a marketing artefact — the audience is Ark's own design decisions.

---

## Architecture

```
benchmark/
├── README.md                 # how to run
├── runner/                   # the harness adapter (Rust binary or shell script)
│   ├── run.sh                # single (fixture × variant) entrypoint
│   ├── aggregate.sh          # post-process all runs → summary tables
│   └── score.sh              # blind-scoring driver
├── fixtures/                 # task definitions, frozen
│   └── <fixture-slug>/
│       ├── PRD.md            # the frozen prompt
│       ├── start.ref         # git SHA the run starts from
│       ├── expected.md       # human-authored acceptance criteria
│       └── checks.sh         # automated quality checks (build/test/clippy)
├── runs/                     # per-run artefacts (gitignored except metrics)
│   └── <fixture>/<variant>/<timestamp>/
│       ├── PRD.md            # copy of fixture PRD (sanity)
│       ├── workflow-artefacts/  # PLAN/REVIEW/VERIFY produced by Ark
│       ├── final.diff        # git diff at end of run
│       ├── metrics.json      # tokens, time, build/test results
│       ├── transcript.jsonl  # full Claude Code session log
│       └── blind-score.md    # blind reviewer's verdict (added in scoring phase)
└── reports/
    └── YYYY-MM-DD-<topic>.md # per-experiment writeup
```

### Why this shape

- **Fixtures decouple from current development.** A fixture is a frozen PRD + git SHA. Once written, it can be re-run against any future Ark version without modification. This is the key adaptation that makes the benchmark usable on the current codebase: we capture tasks that Ark *could* do today, not tasks we plan to do.
- **Variants are the experimental axis.** A variant is a configuration of Ark's workflow (tier, review-loop on/off, sub-agent dispatch on/off). Adding a variant is a config change, not a code change.
- **Runs are append-only.** Every execution lands in a timestamped directory; old runs are never overwritten, so regression detection works.
- **Blind scoring is structurally separated.** The scorer reads `final.diff` without seeing `metrics.json` or the variant label, so quality scores are not contaminated by knowing which config produced them.

---

## Fixtures — sourcing tasks from the current codebase

The hard problem: **the benchmark must run today, not on imagined future work**. Three sources for fixtures, in order of fidelity:

### Source 1 — Reverse-engineered from archived tasks (highest fidelity)

Pick 6–8 archived deep / standard tasks from `.ark/tasks/archive/`. For each:

1. Read the original `PRD.md`.
2. Identify the git SHA from `task.toml`'s `start_head` field.
3. Copy the PRD verbatim into `fixtures/<slug>/PRD.md`.
4. Record the SHA into `fixtures/<slug>/start.ref`.
5. Translate the original `VERIFY.md` items into `fixtures/<slug>/checks.sh` (automated) + `fixtures/<slug>/expected.md` (human-readable).

The fixture is now a "replay" of a real task. We know the original commit succeeded, so we have a ground-truth pass case. Re-running variants on this fixture produces direct comparison: did variant X produce a diff that satisfies the same checks the original did?

**Candidates** (small enough to fit a 1-2 hour run budget, large enough to exercise review value):
- `ark-context` — adds a CLI command, multiple files touched
- `worktree-sync-defaults` — config plumbing
- `task-concurrency-control` — locking semantics (interesting because it shipped Rejected)
- `extract-spec-cmd` — pure CLI addition
- `guard-journal-stamp` — adds a contract guard
- `recursive-verify-seeding` — small recursive change
- `manifest-aware-init` — touches install path
- `drop-installed-at` — small schema change

Avoid huge ones (`workspace`, `ark-workflow-refactor`) for the first round — they exceed a reasonable single-run budget.

### Source 2 — Synthetic fixtures derived from open issues (medium fidelity)

If there's a backlog of small known-good improvements (TODOs scattered in source, or items on `ROADMAP.md` Tier A), write a fresh PRD for one and freeze it.

Useful when archived tasks don't cover a particular surface area (e.g., no archived task touches `crates/ark-mcp` because that crate doesn't exist yet — irrelevant here, but illustrates the pattern).

### Source 3 — Micro-fixtures (lowest fidelity, cheapest)

Single-file, single-function changes. A fixture might be: "add a `--budget <bytes>` flag to `ark context --format json` that elides `archive` and caps `dirty_files` to keep payload under the byte budget."

These run in ~5 minutes. Useful for fast iteration on the runner itself, not for serious quality measurement.

**Target mix for the first benchmark run:** 6 from Source 1 + 2 from Source 3. Source 2 added only if Source 1 doesn't cover something we care about.

---

## Variants — configurations to compare

Each variant is a recipe for *how* the runner drives Ark and the host agent. Defined declaratively in `runner/variants.toml`:

```toml
[variants.thin-baseline]
description = "Claude Code direct, no Ark workflow"
ark_enabled = false
prompt_template = "Read the PRD at {PRD_PATH}. Implement it. Commit when done."

[variants.quick]
description = "Ark quick tier — PRD only, no PLAN"
ark_enabled = true
slash_command = "/ark:quick"
review_passes = 0

[variants.standard]
description = "Ark standard tier — PRD + PLAN + VERIFY, no REVIEW"
ark_enabled = true
slash_command = "/ark:design"
deep = false
review_passes = 0

[variants.standard-with-single-review]
description = "Standard tier + one forced review pass"
ark_enabled = true
slash_command = "/ark:design"
deep = false
review_passes = 1

[variants.deep]
description = "Ark deep tier — full PLAN ⇄ REVIEW loop"
ark_enabled = true
slash_command = "/ark:design"
deep = true
review_passes = "loop"   # iterate until reviewer says APPROVE or 3-cap hit

[variants.deep-capped-1]
description = "Deep tier but reviewer runs exactly once"
ark_enabled = true
slash_command = "/ark:design"
deep = true
review_passes = 1
```

The first benchmark run uses **four variants**: `thin-baseline`, `standard`, `standard-with-single-review`, `deep`. This is enough to answer the most important questions:

- Does any Ark workflow beat the thin baseline? (`thin-baseline` vs everything else)
- Does a single review pass help over no review? (`standard` vs `standard-with-single-review`)
- Does the full loop add value over a single review? (`standard-with-single-review` vs `deep`)

The `quick` and `deep-capped-1` variants are reserved for follow-up experiments.

---

## The runner

A single shell script that, given `(fixture, variant)`, produces a complete `runs/<fixture>/<variant>/<timestamp>/` directory.

### Responsibilities

1. **Set up an isolated workspace.** Clone the repo into a tempdir, `git checkout` the fixture's `start.ref`. Either a fresh tempdir per run, or a worktree off the main checkout — depends on what's cheaper.
2. **Install Ark if the variant needs it.** `ark init` or skip.
3. **Launch Claude Code headless** with the variant's prompt template and the fixture's PRD as input. Use `claude --print` or whatever headless mode exists; this is a dependency to spike on (see "Open questions" below).
4. **Capture the session transcript** (`transcript.jsonl`) — every tool call, every model output. Sources: Claude Code's own session log files in `~/.claude/projects/...` plus any Ark journal entries.
5. **Run the fixture's `checks.sh`** against the resulting workspace. Record build/test/clippy pass/fail.
6. **Compute `metrics.json`:**
   - `tokens_input`, `tokens_output`, `tokens_cache_read`, `tokens_cache_write` (parsed from transcript)
   - `wall_clock_seconds`
   - `build_passed` / `test_passed` / `clippy_clean`
   - `verify_findings_total`, `verify_findings_high` (from any `VERIFY.md` produced)
   - `review_iterations` (from count of `NN_PLAN.md` files, or 0 if variant didn't use review)
   - `final_diff_lines_added` / `_removed`
7. **Snapshot artefacts:**
   - Copy the produced `PRD.md` / `PLAN.md` / `REVIEW.md` / `VERIFY.md` into `workflow-artefacts/`
   - `git diff start.ref HEAD > final.diff`
8. **Tear down.** Remove the tempdir / worktree.

### Failure handling

- Build/test failure is a recorded outcome, not a runner error.
- Runner errors (Claude Code crashed, ran out of tokens, network failure) write `metrics.json` with `status: "failed", reason: "..."` and exit non-zero so aggregation can ignore the run.
- Resumable: if a run dies partway, the timestamped directory is left in place with a `.incomplete` marker, never confused with a clean run.

### Cost guard

A `max_tokens` cap on the runner. If a single run exceeds the cap (e.g., 500K tokens), kill it and record `status: "over-budget"`. Prevents one runaway run from burning the whole experiment budget.

---

## Scoring

After all runs complete, run a separate **blind-scoring pass**:

### Mechanism

1. Aggregate all `final.diff` files across all runs into one pool.
2. Strip identifying metadata — rename to opaque hashes (e.g., `diff-7a3f.patch`).
3. Build a shuffled work-list: each blind scorer (a fresh `ark-reviewer` sub-agent) gets the diff + the fixture's `expected.md`. No knowledge of variant.
4. The scorer outputs (into `blind-score.md`):
   - `mergeable: yes | yes-with-fixes | no`
   - `issue_count` with severity breakdown
   - `quality_score: 1-10`
   - One-paragraph rationale
5. After all scoring complete, `aggregate.sh` joins blind scores back to (fixture, variant) tuples for analysis.

### Why blind

If the scorer sees `variant: deep`, expectation bias inflates its score. The whole point of the benchmark is to measure variants honestly; the scoring step is where bias creeps in most easily.

### Why a sub-agent and not a human

Cost. A human scoring 32 diffs × 5 minutes each = 2.5 hours per experiment round. A sub-agent scorer is ~$0.50 per diff and parallelisable. Bias is different (the model has its own quirks) but at least it's *consistent* bias.

### Optional: human spot-check

For a sample of 4-6 runs, also have a human score. If sub-agent and human scores correlate well across the sample, trust the sub-agent for the rest. If not, recalibrate.

---

## Outputs

### Per-run

- `runs/<fixture>/<variant>/<timestamp>/metrics.json`
- `runs/<fixture>/<variant>/<timestamp>/final.diff`
- `runs/<fixture>/<variant>/<timestamp>/workflow-artefacts/`
- `runs/<fixture>/<variant>/<timestamp>/transcript.jsonl`
- `runs/<fixture>/<variant>/<timestamp>/blind-score.md` (added in scoring phase)

### Per-experiment

`reports/YYYY-MM-DD-<topic>.md` containing:

- Setup: which fixtures × which variants, total runs, total cost.
- Aggregate table: per-variant means/medians for each metric, broken down by fixture.
- Distribution plots if N permits.
- Caveats: failed runs, fixture peculiarities, model version drift across the run window.
- Findings: 3-5 bullet-point takeaways grounded in the numbers.
- Recommendation: what Ark should change (or not) based on this experiment.

### Per-fixture-set

`fixtures/INDEX.md` is the registry: which fixtures exist, what they cover, what their typical run-cost is, when they were last validated. Append-only; old fixtures don't get removed when new ones are added.

---

## What "good" looks like for v1

A v1 of this benchmark is successful if:

1. **Reproducibility:** Two consecutive runs of `(fixture-X, variant-Y)` produce metrics within a stated noise band (~±20% on tokens; build/test pass-fail must match unless flake declared).
2. **Coverage:** ≥6 fixtures from real archived tasks, ≥4 variants, ≥24 total runs in the first experiment.
3. **Cost:** Full experiment ≤ $300; single-fixture iteration cost ≤ $10.
4. **Signal:** The aggregate table shows ≥1 metric where variants differ by a meaningful margin (loosely, the spread exceeds the within-variant noise band). If all variants look identical on all metrics, either the fixtures are too easy or the metrics are wrong.

---

## Open questions to resolve before v1 build

These need spikes (1-2 day investigations each), not architectural decisions:

1. **Claude Code headless driving.** Can `claude --print "..."` reliably complete an `/ark:design` flow, run through PLAN/REVIEW/EXECUTE/VERIFY, and exit cleanly? If yes, the runner is shell-script-thin. If no, the runner must drive Claude Code interactively (via `expect` or similar), which is fragile. **Spike: try driving one full `/ark:design` cycle headless.**
2. **Transcript capture.** Where exactly does Claude Code write per-session token counts? Is there a parseable format, or do we need to scrape from log files? **Spike: find the source-of-truth for one session's token totals.**
3. **Workspace isolation.** Is tempdir + clone OK, or do we need git worktrees because tempdir clones are too slow / lose pack files? **Spike: time both for a fixture-sized run.**
4. **Fixture replay validity.** When we replay archived task `ark-context` against its original `start_head`, does the codebase at that SHA still build? If not, fixtures rot and we need a periodic re-validation step. **Spike: try one replay.**
5. **Cost-per-fixture in practice.** The $5/$300 budget assumes ~50K-150K tokens per run. The first end-to-end run will calibrate this — if real cost is 3× higher, fixture count drops or fixtures get smaller.

---

## Build sequence

The order matters. Each step depends on the previous and is small enough to validate independently.

### Step 1 — Resolve the open questions

Time-boxed, ~3-5 days. Outputs: a short decision memo per open question. If headless Claude Code doesn't work, the whole benchmark plan changes shape — better to learn this before writing the runner.

### Step 2 — Single-fixture, single-variant runner

Build `runner/run.sh` such that one fixture × `thin-baseline` produces a complete `runs/.../` directory with all required outputs. Manual cost; no automation around it.

### Step 3 — Add real Ark variants

Extend the runner to handle `standard`, `standard-with-single-review`, `deep`. Validate one fixture × each variant. Now the matrix is 1×4.

### Step 4 — Fixture set construction

Build the first 6 archived-task fixtures (Source 1) + 2 micro-fixtures (Source 3). Verify each replays cleanly against its `start.ref`.

### Step 5 — `aggregate.sh`

Cross-run aggregation. Produces the metric tables. Test against the matrix from step 3 (already in `runs/`).

### Step 6 — Blind scoring

Build `score.sh`. Test by scoring the existing runs from step 3.

### Step 7 — First full experiment

8 fixtures × 4 variants = 32 runs. ~$300 budget. Write `reports/YYYY-MM-DD-baseline.md` from the output.

---

## What we'll learn from the first experiment

The first run answers (or rules out) the following design questions:

1. **Does Ark's workflow produce measurably better code than thin-baseline?** If `thin-baseline` matches `standard` on quality scores, the workflow has no measurable value on this fixture set — either the fixtures are too easy, or Ark needs harder cases to shine.
2. **Where does the REVIEW loop's value come from?** Three break-points: 0 → 1 review pass, 1 → loop. The shape of the cost-vs-quality curve here directly informs whether to keep the loop, cap it, or remove it.
3. **What's the cost ratio?** If `deep` costs 5× `standard` for marginal quality gain, the loop is hard to defend on cost. If it costs 1.5× for clear gain, the loop pays.
4. **Is there a variant that strictly dominates?** Best case: one variant wins on quality at lower cost than another. Worst case: every variant Pareto-front, no clear winner — then we need richer fixtures.

---

## Non-goals (explicit)

- Not measuring developer productivity (no humans in the loop except spot-check).
- Not measuring Ark's effect on Codex / OpenCode (Claude Code only for v1; cross-platform comes later if v1 works).
- Not validating Ark's correctness as software (that's the job of `cargo test` in the Ark repo itself).
- Not producing publication-quality results — internal decision support is the audience.
- Not optimising for benchmark scores. If a finding suggests changing Ark, we change Ark and re-run; we never gradient-descent the workflow against the benchmark.

---

## Maintenance

Once v1 lands:

- Run the full suite after any change to `crates/ark-core/src/commands/agent/` or `templates/`. These are the surfaces that move quality numbers.
- Add a new fixture every time Ark ships a new tier / workflow capability so coverage tracks the codebase.
- Re-validate fixtures quarterly (Step 4 says how) — codebase drift may break some.
- Archive old reports under `reports/` indefinitely; trend-line analysis becomes possible after the third experiment.
