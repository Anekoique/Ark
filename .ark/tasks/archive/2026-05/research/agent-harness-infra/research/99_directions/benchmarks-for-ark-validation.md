# Research: Benchmarks for validating Ark workflow (PLAN↔REVIEW iteration impact)

- Query: Pick an agent benchmark suite for Ark's review-loop-count experiment (N = 1, 2, 3, 5).
- Scope: external (web), with internal context on Ark's deep-tier lifecycle.
- Date: 2026-05-21

## 0. Experiment framing (restated)

Ark's deep-tier lifecycle: `PRD → PLAN → REVIEW (×N) → EXECUTE → VERIFY → COMMIT`.
- **Independent variable:** N (number of review rounds, with the *same* model on both sides of the loop).
- **Dependent variables we care about:** (1) task completion rate (pass/fail), (2) latency to commit, (3) token cost, (4) regression rate, (5) plan stability between iterations.
- **Constraint:** each "task" must be re-runnable under a *different* harness config (same task seed, different N) so we can do paired comparisons. The benchmark must therefore (a) expose deterministic per-task ground truth, (b) be harness-agnostic — it must accept "here is the diff/patch Ark produced," not require its own scaffold.

Section 4 picks two primary benchmarks. Sections 1–3 are the landscape sweep. Section 5 is pitfalls. The final subsection lists concrete Ark follow-ups.

---

## 1. Candidate benchmarks (catalogue)

### 1.1 SWE-bench family (Princeton/Stanford, MIT licensed)

The single most relevant family. All variants share the same contract: given a repo snapshot + GitHub issue text, the agent must produce a unified-diff patch; the harness applies the patch in a Docker image identical to the upstream commit, runs the held-out test set, and reports pass/fail. Predictions are JSONL with `{instance_id, model_name_or_path, model_patch}`.

| Variant | Tasks | Languages | Year | Selection rationale | License |
| --- | --- | --- | --- | --- | --- |
| **SWE-bench (Full)** | 2,294 | Python only | Oct 2023 | All GitHub issues mined from 12 popular Python repos | MIT |
| **SWE-bench Lite** | 300 | Python | 2024 | Subset of Full, picked for "self-contained functional bug fixes," preserving difficulty distribution | MIT |
| **SWE-bench Verified** | 500 | Python | Aug 2024 | OpenAI-funded human review of solvability + test adequacy; current de facto standard | MIT |
| **SWE-bench Verified Mini** | 50 | Python (django+sphinx only) | 2024 | LP + k-means selection to mirror Verified's score distribution; 5 GB vs. 130 GB of Docker images | MIT (community fork) |
| **SWE-bench Multimodal** | unspecified | JS-heavy + screenshots | Oct 2024 | Visual / UI-bug subset; eval kept private (sb-cli only) | MIT (eval test-split private) |
| **SWE-bench Multilingual** | 300 | C, C++, Go, Java, JS/TS, PHP, Ruby, Rust (9) | 2025 | 42 repos; ~30–44 tasks/language; Princeton-curated | MIT |
| **SWE-bench Live** (Microsoft, NeurIPS 2025 D&B) | 1,319 initial + 50/month | Python (93 repos) | May 2025 | Rolling window starting 2024-01; contamination-resistant by construction | MIT |
| **SWE-bench Pro** (Scale AI, Sep 2025) | 1,865 (731 public / 858 held-out / 276 commercial) | Multi-lang, 41 repos including enterprise startups | Sep 2025 | Long-horizon, multi-file enterprise patches; top scores ~23% public, <20% commercial | Public split open; commercial private |
| **SWE-rebench** (Nebius, NeurIPS 2025) | 21,000+ tasks; benchmark slice rotates | Python (multi-repo mining) | May 2025 | Fully automated continuous pipeline; LLM-assigned task-quality labels | Open (HF dataset) |
| **Multi-SWE-bench** (ByteDance Seed) | 2,132 (+ mini = 400) | Python, Java, TS, JS, Go, Rust, C, C++ (8) | Apr 2025 | 68 expert annotators; separate from Princeton's Multilingual | Open |
| **SWE-MERA** (2025) | dynamic | Python | Jul 2025 | Yet another dynamic SWE benchmark; smaller community | Open |

**Evaluation harness for the whole family:**
- Docker-per-instance (full containerization since June 2024).
- `sb-cli` cloud submission: free, returns results in ~20 minutes for arbitrary batch sizes. Test split for Verified is kept private to limit overfitting; sb-cli is the official gate to the leaderboard.
- Modal cloud runner: available since Jan 2025.
- Local Docker: ~120 GB free disk needed for Verified; ~5 GB for Verified Mini.
- Per-instance wall-clock: ~3.5 min for Lite-style runs; 90–150 min for full Verified with 8 workers + 20-turn budget; an optimized image-registry path runs all 500 Verified in 62–73 min on one 32-core/128 GB machine (Epoch AI write-up).
- **Agent-agnostic:** the harness only cares about `{instance_id, model_patch}`. Ark can drive its own workflow internally and submit the final diff. This is the *key property* that makes SWE-bench a good fit.

### 1.2 SWE-Gym (UC Berkeley + CMU, ICML 2025)

- **Purpose:** training environment, not a leaderboard, but ships with a curated dev set.
- **Scale:** 2,438 Python tasks from 11 open-source repos.
- **Eval scaffolds shipped:** OpenHands and Moatless Tools (both can be swapped out — same SWE-bench-style patch contract).
- **Relevance for us:** the dataset itself is task-shape-compatible with SWE-bench; useful if we want a *bigger* training-style pool to draw from without paying for full Verified runs. The associated paper also shows the gain from inference-time scaling with verifiers (32% on Verified) — that's exactly the lever Ark's REVIEW loop is trying to pull, so it's a useful precedent.
- **License:** open.

### 1.3 LiveCodeBench (UC Berkeley + MIT, ICLR 2024 onward)

- **Task shape:** competitive-programming problems scraped monthly from LeetCode / AtCoder / Codeforces; eval is hidden-test pass/fail.
- **Scale:** release_v6 has 1,055 problems spanning May 2023 – Apr 2025.
- **Contamination story:** strong — you slice the dataset by problem-release date relative to model training cutoff and only score the post-cutoff window.
- **Fit for Ark:** **mediocre.** Tasks are single-file algorithmic problems. Ark's review loop is designed to catch *plan-level* errors on multi-file changes; a single-function algorithm problem barely gives the PLAN step anything to do. Useful as a control if you want a "low-headroom" comparison set.
- **License:** open.

### 1.4 BigCodeBench (BigCode @ HuggingFace, ICLR 2025)

- **Task shape:** 1,140 function-level tasks requiring multi-library tool use (139 libraries, 7 domains); two splits — Complete (autocomplete style) and Instruct (NL prompt). 5.6 tests/task, 99% branch coverage.
- **Fit for Ark:** **weak.** Function-level scope. Ark's value-add (PLAN articulation, REVIEW critique of structural choices) doesn't get exercised. Frontier models cap around 60%; saturation is a real concern.
- **License:** open.

### 1.5 RepoBench (NTU, ICLR 2024)

- **Task shape:** repo-level *code completion* — given cross-file context, complete the next line/block. Three sub-tasks: Retrieval, Code Completion, Pipeline. Python + Java.
- **Metrics:** Exact Match, Edit Similarity, CodeBLEU. **Not** pass/fail on tests.
- **Fit for Ark:** **bad fit.** Completion-style with fuzzy metrics — iteration impact will be ambiguous. Skip.

### 1.6 HumanEval / MBPP (OpenAI 2021, Google 2021)

- **Status:** **saturated** for any frontier model. HumanEval pass@1 ≈ 99%, MBPP pass@1 ≈ 94%. Both are function-level. The corpus is largely scraped into every training set.
- **Verdict:** **do not use** as our primary signal. Useful only as a no-op control to confirm Ark isn't *worse* than direct prompting at trivial problems.
- **Successors:** HumanEval Pro / MBPP Pro (self-invoking variants, ACL 2025 Findings) — still single-function but harder. Not what we want.

### 1.7 BIRD-SQL (NeurIPS 2023 → 2025 extensions)

- **Task shape:** 12,751 natural-language → SQL pairs over 95 databases (33 GB).
- **2025 finding:** BIRD's binary pass/fail only agrees with human experts 62% of the time; ~32% of training-set problems have annotation errors (CIDR 2026 paper).
- **2025 successors:** LiveSQLBench (contamination-free, dynamic) and BIRD-Interact (conversational + agentic modes).
- **Fit for Ark:** **off-domain.** SQL doesn't exercise the multi-file refactor patterns Ark is built around. Skip unless we explicitly want a database-tool test.

### 1.8 Aider's polyglot benchmark

- **Task shape:** 225 Exercism exercises in C++, Go, Java, JS, Python, Rust. Selected from a pool of 697 such that ≤3 of 7 baseline models solved each. Two-attempt protocol (re-prompt with test feedback after attempt 1).
- **Eval:** hidden tests.
- **Built into Aider's harness.** That's both a feature (it's well-instrumented) and a *blocker* (the two-attempt protocol is baked in — it would mask Ark's own N-shot loop unless we replace Aider's outer loop).
- **Fit:** **medium.** Could work if we run with N=1 attempt in Aider's harness and use Ark for whatever extra iteration we want.
- **License:** open.

### 1.9 METR RE-Bench (Nov 2024 / extended through 2025)

- **Task shape:** 7 open-ended ML research-engineering tasks (fit a scaling law, optimize a GPU kernel, etc.), each given an 8-hour budget per attempt. 71 human-expert attempts in the reference data.
- **Eval:** custom continuous score per task (not pass/fail).
- **Fit for Ark:** **wrong shape.** N=7 tasks is too few for paired statistical tests across N values. And the score is continuous + noisy. Skip.

### 1.10 MLE-bench (OpenAI, ICLR 2025 Oral)

- **Task shape:** 75 Kaggle competitions. Agent gets 24 hours per task, 36 cores, 440 GB RAM, 1× A10 GPU. Output is a submission file scored by Kaggle's metric (AUC/RMSE/etc.) and graded against the public leaderboard percentile (bronze/silver/gold).
- **Compute cost:** *enormous.* 24 hours per task × 75 tasks × N values × repeats. Not feasible for our experiment without a dedicated cluster.
- **Skip** for cost reasons.

### 1.11 SWE-Lancer (OpenAI + Princeton, Feb 2025)

- **Task shape:** 1,400+ Upwork freelance jobs, $50 bug fix → $32K feature. Two modes: independent coding tasks (graded by E2E tests, triple-verified) and managerial tasks (multiple-choice over proposals).
- **Open subset:** SWE-Lancer Diamond + unified Docker image.
- **Headroom:** Claude 3.5 Sonnet only solved 26.2% of independent tasks at launch — lots of room for iteration to move the needle.
- **Fit for Ark:** **strong as a second-tier benchmark.** The tasks are bigger and more open-ended than SWE-bench, so PLAN/REVIEW value-add should be more visible. The downside is fewer tasks per category and harder per-task cost forecasting.
- **License:** Diamond split is open; full set is not.

### 1.12 Terminal-Bench / TBench 2.0 (Stanford + Laude, 2025)

- **Task shape:** 89 hard terminal-tasks (compile code, train models, set up servers) — each has its own sandbox + tests. Frontier agents <65%.
- **Used by Cursor, Codex CLI, Claude Code, Gemini CLI** in their own evals.
- **Fit for Ark:** **interesting but different shape.** Tasks are operational rather than code-patching. The number of tasks (89) is on the small side for the statistical design we want, but doable.
- **License:** open.

### 1.13 SlopCodeBench (2025)

- **Task shape:** 20 problems × 93 checkpoints, where the agent extends its *own prior solution* across evolving specs. Explicitly designed to measure long-horizon degradation.
- **Headline finding:** quality degrades steadily — erosion in 80% of trajectories, verbosity in 89.8%.
- **Fit for Ark:** **conceptually adjacent** — they're testing iteration of *output*, we're testing iteration of *plan*. Not a direct fit (20 problems is too few), but very useful as prior art on "what iteration metrics to log."

### 1.14 SWE-CI (2025)

- **Task shape:** repository-level CI-loop benchmark — agent fixes failing CI builds. Long-horizon, measures maintainability not just correctness.
- **Fit:** interesting future direction; too new and small-community to anchor our first experiment on.

### 1.15 Holistic Agent Leaderboard (HAL, arXiv 2510.11977)

- **Not a benchmark itself** — a meta-leaderboard tracking model + scaffolding pairs across SWE-bench Verified, GAIA, Terminal-Bench, etc. Useful as a reference point for how others report agent-vs-harness effects. We should consider submitting our N-loop results here.

### 1.16 CommitPack / CommitPackFT / AgentPack

- **Task shape:** commit corpora, not benchmarks. CommitPack = 4 TB of permissive-licensed commits. CommitPackFT = 2 GB filtered. AgentPack (Sep 2025) = 1.8 M code edits co-authored by Claude Code, Codex, Cursor Agent.
- **Use:** *training/fine-tuning* data, not evaluation. Out of scope for our experiment but worth keeping in the back pocket if we ever want to train an Ark-specific reviewer model.

### 1.17 Anthropic / OpenAI public eval reports

- **Anthropic's SWE-bench writeup** (`anthropic.com/research/swe-bench-sonnet`, with updates through Sonnet 4.5/4.6 and Opus 4.5/4.7): documents that runs *take hundreds of turns* and frequently exceed 100K tokens. Confirms the harness is agent-agnostic and the patch contract.
- **OpenAI's "Why SWE-bench Verified no longer measures frontier capabilities"** (2025): they have shifted off Verified for top-line reporting, citing scaffolding-impact swings of 12+ points and contamination. They've moved to SWE-bench Pro. *This is the most important external signal in this whole document for picking a target.*

---

## 2. Fit to our experiment design — filter table

We need: (a) ground-truth pass/fail, (b) sample size big enough for N=1/2/3/5 paired comparisons, (c) deterministic same-task replay across N, (d) no opinionated outer loop the eval bakes in.

| Benchmark | Pass/fail ground truth | Per-task cost (~Claude Sonnet) | Re-runnable same task? | Harness opinionated? | Verdict for Ark experiment |
| --- | --- | --- | --- | --- | --- |
| SWE-bench Verified | Yes | $1–$3 typical, $5+ if no budget cap (100K+ tokens reported by Anthropic) | Yes (Docker) | No (just submit a diff) | **Primary candidate** |
| SWE-bench Verified Mini | Yes | Same per-task; 50 tasks total → much cheaper aggregate | Yes | No | **Primary for pilot runs** |
| SWE-bench Live | Yes | Same | Yes (rolling) | No | **Strong contamination-control control arm** |
| SWE-bench Lite | Yes | Lower per-task (smaller diffs) | Yes | No | Acceptable cheap alternative |
| SWE-bench Multilingual | Yes | Same | Yes | No | Useful for cross-language generalization later, not first cut |
| SWE-bench Pro | Yes | Higher (long-horizon, more files) | Yes | No | Aspirational; use after primary has answered the basic question |
| SWE-Lancer Diamond | Yes (E2E tests) | Variable, some tasks worth $32K of work | Yes (Docker) | No | Second-tier secondary benchmark |
| SWE-Gym | Yes | Same as SWE-bench | Yes | Ships OpenHands/Moatless but optional | Use as supplementary training-data pool only |
| LiveCodeBench | Yes (hidden tests) | Cheap (small problems) | Yes | No | Off-shape — too easy for plan-level work |
| BigCodeBench | Yes | Cheap | Yes | No | Off-shape — function-level |
| Aider polyglot | Yes | Cheap | Yes, but 2-attempt loop baked in | **Yes — outer retry loop in harness** | Replace its outer loop or skip |
| HumanEval / MBPP | Yes | Trivial | Yes | No | Saturated; control only |
| BIRD-SQL | Partial (62% expert agreement) | Cheap | Yes | No | Off-domain |
| RE-Bench | No (continuous score) | High (8 h/task) | Yes | Some | Wrong shape, too few tasks |
| MLE-bench | Yes (medal threshold) | Enormous (24 h × 75 tasks) | Yes | Some (24 h budget) | Compute-prohibitive |
| Terminal-Bench 2.0 | Yes | Medium | Yes | No | Small N (89), interesting secondary |
| SlopCodeBench | Yes per checkpoint | Medium | Yes | Some (iteration is the test) | Wrong shape but cite for metrics |
| SWE-CI | Yes | Medium | Yes | No | Too new |
| RepoBench | No (EM/ES/CodeBLEU) | Cheap | Yes | No | Wrong metric |

**Filter result:** the SWE-bench family is the only family that ticks all four boxes for a first experiment.

---

## 3. What "iteration impact" looks like per top candidate

Mapping the dependent variables to each remaining benchmark.

### 3.1 SWE-bench Verified / Verified Mini / Lite

| Metric | How to measure | Supported? |
| --- | --- | --- |
| pass@1 vs N | Resolve-rate (= patch applies + held-out tests pass) per N, computed over the *same* instance set | **Yes** — this is the canonical metric |
| Latency to commit | Wall-clock from PRD to COMMIT, logged in `task.toml` + workflow journal | **Yes**, Ark already records this in journals |
| Token cost | Sum of input+output tokens across PRD/PLAN/REVIEW×N/EXECUTE/VERIFY/COMMIT phases | **Yes**, model-side — Ark would need to wire prompt counters per phase |
| Regression-introduction rate | Tests that *previously passed* in the base image but now fail with Ark's patch. Distinct from "didn't solve the issue." | **Yes** — the harness already runs the full pre-existing test suite and reports per-test deltas. We log "previously-passing tests that now fail" as a derived metric. |
| Plan stability between iterations | Diff between PLAN.md at REVIEW round k and round k+1 — semantic similarity or churn-of-bullets. *Internal to Ark, not from benchmark.* | **Yes**, but the *measurement* lives in Ark, not SWE-bench |

### 3.2 SWE-Lancer Diamond (secondary)

- All five metrics still supported. Higher *headroom* because base resolve rates are in the 20–30% range — moving the needle by N is more visible.
- Caveat: per-task cost is variable and harder to forecast. Triple-verified E2E tests mean ground truth is solid.

### 3.3 SWE-bench Live (contamination-control arm)

- Same metric set as Verified. Use as a paired-cohort safety net: if Ark's iteration gain on Verified is much larger than on Live, suspect contamination is doing the work.

### 3.4 Aider polyglot (if we use it)

- pass@1 is the natural metric. But Aider's harness has its own 2-attempt loop. To use it cleanly we either (a) hack the harness to N=1 attempt and put Ark's iteration outside, or (b) accept that the comparison is "Ark's loop *on top of* Aider's loop."

### 3.5 Terminal-Bench 2.0 (secondary, smaller-N)

- pass/fail per task. N=89 → fine for descriptive stats, marginal for paired t-tests on small effects. Useful as a *non-SWE-bench* sanity check.

---

## 4. Recommended benchmarks + experiment shape

### 4.1 Primary recommendation

**Use SWE-bench Verified Mini (50 tasks) for pilot, then SWE-bench Verified (500 tasks) for the main experiment.** Reasons:

1. **Same task contract** — both produce a unified diff against the same Docker image. We can promote results from pilot → main without changing Ark code.
2. **Ground truth is binary, mechanical, and well-instrumented.** The pass/fail signal is deterministic given the patch. Reasonable variance still exists (1.5–2 pp standard deviation across replications at T=0 per AI21's 200K-run study), so we should still replicate.
3. **Harness is agent-agnostic.** Ark drives its full PRD → COMMIT lifecycle internally; we hand the final diff to `sb-cli` (cloud, free) or run Docker locally.
4. **Cost forecast for Verified is tractable.** Per-instance budgets in published runs are commonly capped at $1 (Claude 3.7, conservative) up to "hundreds of turns, 100K+ tokens" (Anthropic's own report). At Sonnet 4.5/4.6 list prices ($3 in / $15 out per Mtok) a single Ark task with 5 review rounds plausibly burns 500K–1M tokens → roughly $5–$15 per task per N-value. See §5 cost math.
5. **Verified Mini exists precisely for this:** 5 GB Docker footprint, ~50 tasks, distributionally faithful — perfect for early pilot before committing to the full 500-task run.

### 4.2 Secondary recommendation

**SWE-Lancer Diamond as a second-tier benchmark once Verified results are in hand.** Reasons:

1. Higher headroom (top models <30% resolve rate at launch).
2. Larger tasks → REVIEW loop has more to chew on.
3. Same patch-and-test contract as SWE-bench so we don't rebuild the harness.

Skip Terminal-Bench / Aider polyglot for v1 unless we specifically want to test outside-SWE-shape generalization.

### 4.3 Experiment sketch

```
Pilot:
  Benchmark:    SWE-bench Verified Mini (n = 50 tasks)
  Model:        single frontier model held constant (e.g., Sonnet 4.6 via API)
  Prompts:      Ark stock templates (PRD/PLAN/REVIEW/EXECUTE/VERIFY) — frozen
  Variables:    N ∈ {1, 2, 3, 5}
  Repeats:      3 replicates per (task, N) cell at temperature 0 (acknowledging AI21's
                T=0 variance finding — 1.5+ pp SD persists even at T=0)
  Total runs:   50 tasks × 4 N-values × 3 reps = 600 task-runs
  Budget cap:   $5/task/N to bound worst case; auto-abort tasks past cap
  Worst-case cost ≈ 600 × $5 = $3,000 for the pilot
  Expected cost (most tasks settle under cap): $1,000–$1,800

Main (after pilot reads positive):
  Benchmark:    SWE-bench Verified (n = 500)
  Same model and prompts.
  N ∈ {1, 2, 3, 5}, 1 replicate (we calibrate variance from pilot)
  500 × 4 × 1 = 2,000 task-runs.
  At $3 average → $6,000 budget; at $1 average → $2,000.
```

### 4.4 What to log (per task × N × replicate)

- `task_id`, `N`, `replicate_seed`
- Wall-clock duration per phase (`prd_ms`, `plan_ms`, `review_k_ms` for k ∈ [1..N], `execute_ms`, `verify_ms`)
- Tokens in/out per phase
- Dollar cost per phase (derived from token counts × price card)
- Final diff (the artifact submitted to sb-cli)
- `resolved` boolean from harness
- `failed_to_pass` from harness — count of previously-passing tests that the patch broke (regression metric)
- PLAN.md after each review round, hashed + diffed; also a "plan churn" metric = number of changed bullets between round k and k+1
- REVIEW verdict per round (open/landing/blocking)
- Any phase-level errors or VERIFY rollbacks
- Workflow journal blob

### 4.5 Statistical analysis

- **Primary test:** McNemar's paired test on resolved-bit-per-task. For each pair (N=1 vs N=2), (N=1 vs N=3), (N=1 vs N=5), tabulate "improved by extra loop" vs "regressed by extra loop" and apply the test. This is the textbook design for "same subject, different configuration." (Background: McNemar's test on paired binary data, e.g. Wikipedia / MachineLearningMastery / arXiv 1704.00045.)
- **Secondary:** logistic regression of `resolved ~ N + task_difficulty_decile`, fixed effects per task family — gives a coefficient on N.
- **Effect size:** report difference in resolve rate with 95% CI, not just p-values.
- **Variance budget:** AI21 reports 1.5+ pp SD at T=0. Any reported delta below ~3 pp should be treated as noise unless replicates and McNemar both align. With 500 tasks the McNemar power is enough to detect a 4–5 pp resolve-rate delta at α=0.05.
- **Sensitivity check:** rerun the analysis restricted to "low-leakage" tasks (e.g., SWE-bench Live or post-cutoff SWE-rebench slice) to confirm the iteration gain isn't memorization-mediated.

---

## 5. Pitfalls

### 5.1 Contamination / training-set leakage

Multiple 2025 studies show SWE-bench Verified is significantly contaminated:
- The original SWE-bench predates current model knowledge cutoffs by years. Over 94% of original SWE-bench instances predate today's training-data cutoffs.
- ~32.67% of "successful" patches in SWE-bench involve direct solution leakage (issue text or comments giving the fix), and another ~31.08% pass due to inadequate test cases — so reported scores may be inflated by ~3× relative to "true" capability.
- Repo-state loopholes exist: agents can sometimes inspect future commits via git reflog, branches, or origin remotes; the harness has been patched but the loophole class is open-ended.
- OpenAI explicitly stopped reporting SWE-bench Verified because of contamination + scaffolding-swing concerns ("Why SWE-bench Verified no longer measures frontier coding capabilities," 2025) and moved to SWE-bench Pro.

**Mitigation for our experiment:** the iteration-impact question is *within-task paired*, so contamination biases all N-values *equally* on the same task and largely cancels out for the McNemar test. But for the absolute resolve-rate numbers, expect inflation. Add the **SWE-bench Live monthly slice** (issues after model cutoff) as a sensitivity check.

### 5.2 Benchmark drift (Verified vs original)

- Verified is a *human-validated* subset of Full — easier on average because impossible/ambiguous instances were dropped.
- Verified Mini is sub-sampled to match Verified's score distribution; not a uniform random sample.
- Scores cannot be compared cross-variant without translation tables.

**Mitigation:** pick one variant and stay on it through the experiment; don't mix.

### 5.3 Per-task variance / harness non-determinism

- AI21's 200K-trajectory study: per-task pass@1 estimates vary by 2.2–6.0 pp across replications; SD >1.5 pp even at T=0. Inference-engine non-determinism + container clock drift + non-deterministic test ordering all contribute.
- Anthropic's report: runs commonly take hundreds of turns and 100K+ tokens — small differences in random tool-call ordering propagate.

**Mitigation:** replicate (3× in pilot). Treat <3 pp deltas as suspicious. Always report CIs.

### 5.4 Agent flakiness / harness-side scaffolding swings

- The same model can score 42% vs 78% on the same benchmark by changing only scaffolding (the Particula write-up, on SWE-bench).
- On SWE-bench Pro, scaffolding alone moves results 22+ points with the model held fixed (Quesma blog).
- Top-of-leaderboard divergence by harness is in the 10–20 pp range on Verified.

**Implication for us:** Ark *is* the scaffolding being tested. The whole point of the experiment is to isolate one knob (N) within Ark's scaffolding. Be very careful about freezing everything else. **Lock prompts, tool definitions, model parameters, and even the inference SDK version.** Record SDK + model version in every run.

### 5.5 Cost

Working numbers (Sonnet 4.6 list, $3/$15 per Mtok in/out, May 2026):

| Scenario | Tokens per task | $/task | Source |
| --- | --- | --- | --- |
| Aggressive cap (Claude 3.7 community runs) | ~100–300K | $1 | swe-agent docs |
| Anthropic-style un-capped agentic run | "hundreds of turns, 100K+ tokens" | $5–$15 | anthropic.com/research/swe-bench-sonnet |
| Ark deep-tier with N=5 reviews (estimate) | 500K–1M | $5–$15 | extrapolated from Anthropic numbers |
| Verified Mini pilot (50 tasks × 4 N × 3 reps, $5 cap) | — | — | ~$3,000 worst case |
| Verified main (500 tasks × 4 N × 1 rep, $3 avg) | — | — | ~$6,000 |

If the budget is tight, start with Verified Mini + N ∈ {1, 3} (skip the N=2 and N=5 cells in pilot) to halve the run count.

### 5.6 Choice of primary metric

The benchmark only gives `resolved ∈ {0, 1}`. Some of our most interesting dependent variables (plan churn, regression rate) are *not* on the benchmark — they're derived from Ark's own logs. That's fine but means **Ark needs to capture them deterministically**, ideally with a stable JSON schema, before we run the experiment. See the §6 Directions list.

### 5.7 Selection of tasks within Verified

- Verified Mini is engineered to be distributionally faithful. Other ad-hoc subsetting (e.g., "easy 50") would bias N-impact estimates.
- If we subset, do so by *repository* (e.g., "django only") to control for codebase complexity, not by difficulty score.

### 5.8 Sample-size pitfalls

- For a paired binary outcome with N=50 (Verified Mini) and a true 5 pp improvement, McNemar's power is ~50% — borderline. Need 500 tasks for adequate power at small effect sizes. Use Mini only for *pilot direction*, not for the final claim.

### 5.9 Multiple-comparison inflation

We're testing N=1 vs N=2, N=1 vs N=3, N=1 vs N=5, N=2 vs N=3, etc. — 6 pairwise tests. Apply Bonferroni or use a single ordinal trend test (Cochran-Armitage) to avoid p-hacking.

---

## Directions for Ark

Concrete follow-up tasks suggested by this research. Each is sized as a future task slug; the main session decides which to dispatch.

1. **`ark bench` subcommand (deep-tier task).** Wrap SWE-bench Verified / Verified Mini runs end-to-end: pull task list, drive Ark workflow per task, emit JSONL `{instance_id, model_patch}`, invoke `sb-cli` (or local Docker). Schema for run logs should match §4.4. License: MIT compatible (SWE-bench is MIT). Pilot first on Verified Mini.

2. **`ark journal --metrics` extension (standard-tier task).** Augment the existing workflow journal to emit per-phase tokens/cost/wall-clock as structured JSON. Required input for any iteration-impact analysis. Lives next to the existing journal write path (worktree feature).

3. **PLAN-diff metric (quick-tier task).** Compute "plan churn" = bullet-level diff between PLAN.md at review round k and round k+1. Persist alongside the journal. This is the *unique* metric Ark can offer that no benchmark gives us.

4. **Contamination-control arm (deep-tier task).** Wire SWE-bench Live's monthly slice into `ark bench` as a control cohort. Run pilot N-loop experiment on both Verified and Live; report deltas. If iteration gain on Live <<< gain on Verified, that's a contamination flag.

5. **Reviewer-config knob (deep-tier task, blocking the experiment).** Today N is set by human choice each task. To run the experiment we need `ark agent task review --max-rounds N` (or task.toml field) that bounds the loop. Required before any benchmark run; design now, even if we don't implement until the experiment is scheduled.

6. **Cost guardrails (standard-tier task).** A `$/task` cap with auto-abort. The Anthropic SWE-bench numbers (100K+ tokens per run) are unfriendly to a 2000-run experiment without this.

7. **SWE-Lancer Diamond as v2 (research-tier task).** Once we have Verified results, scope a follow-up against SWE-Lancer Diamond — higher headroom, larger tasks, same patch contract. Skipping to it directly is premature; do it after Verified answers the basic question.

8. **HAL leaderboard submission (chore).** If/when Ark + N>1 produces a notable result on Verified, consider submitting to the Holistic Agent Leaderboard (HAL, arXiv 2510.11977) where model + scaffolding pairs are tracked together — Ark is in scope.

9. **Workflow-time `ark research bench` slash command (quick-tier task).** Codify "when scoping a new benchmark, dispatch this researcher" so future bench picks follow the same intake.

10. **Decline path (not a task, a policy note).** If the experiment shows no significant gain from N > 1, Ark's *default* should drop to N=1 with manual escalation. Don't ship review-loop overhead users don't benefit from. The experiment design must support a *null finding* being actionable, not just a positive one.

## Caveats / Not found

- **No public per-task cost breakdown** for SWE-bench Verified from Anthropic or OpenAI — all numbers in §5.5 are extrapolated from list prices + reported per-run token volumes. A real cost forecast requires a small calibration run first (which is part of the pilot).
- **No paper specifically measuring "number of review rounds vs. resolve rate"** found in the corpus. The closest analogues are pass@k sample-efficiency work (Pass@ARC, ReVeal showing 36.9% → 42.4% over 19 turns) and SlopCodeBench (showing degradation, not improvement). Ark's experiment would fill a real gap.
- **SWE-bench Multimodal eval split is private** — sb-cli only; we can run it but won't get instance-level traces without sb-cli help. Probably not worth using.
- **SWE-bench Pro commercial set requires Scale AI contracting.** Public + held-out sets are open; the held-out leaderboard is sb-cli-only. The commercial set is not feasible for us.
- **Anthropic SWE-bench engineering doc** is paywalled behind anthropic.com login at higher detail levels; only the public blog version is cited above.
- Couldn't find a definitive 2026 update on whether **SWE-bench Live** has reached the original-Verified-sized 500-task milestone yet; the May 2025 paper says 1,319 initial + 50/month, and we'd need to recount at experiment time.

---

## References (informal)

SWE-bench family:
- swebench.com (Princeton/Stanford), MIT license, primary docs
- swebench.com/SWE-bench/reference/harness/ — agent-agnostic patch contract
- swebench.com/verified.html and verified-mini variant
- github.com/SWE-bench/SWE-bench
- github.com/SWE-bench/sb-cli — free cloud submission
- huggingface.co/datasets/SWE-bench/SWE-bench_Verified
- epoch.ai/blog/swebench-docker — runtime benchmarks (62–73 min for 500 tasks)
- ai21.com/blog/scaling-agentic-evaluation-swe-bench — 200K-run variance study (1.5+ pp SD at T=0)

2025 successors / contamination-aware variants:
- github.com/microsoft/SWE-bench-Live (NeurIPS 2025 D&B)
- nebius/SWE-rebench (NeurIPS 2025; 21K tasks, monthly refresh)
- scaleapi/SWE-bench_Pro-os (Sep 2025, enterprise codebases)
- multi-swe-bench/multi-swe-bench (ByteDance, 8 languages, 2025)

Other benchmarks evaluated:
- github.com/SWE-Gym/SWE-Gym (ICML 2025)
- livecodebench.github.io (contamination-aware, monthly)
- bigcode-project/bigcodebench (ICLR 2025)
- aider.chat/docs/leaderboards (polyglot, 225 Exercism tasks)
- metr.org/research — RE-Bench
- github.com/openai/mle-bench (ICLR 2025 Oral, 75 Kaggle)
- github.com/openai/SWELancer-Benchmark (Feb 2025, $1M of freelance work)
- tbench.ai — Terminal-Bench 2.0

Iteration-and-degradation prior art:
- SlopCodeBench (arXiv 2603.24755) — long-horizon degradation
- ReVeal (arXiv 2506.11442) — self-verification turn-by-turn
- Pass@ARC efficiency-penalized pass@k

Critique / contamination:
- openai.com/index/why-we-no-longer-evaluate-swe-bench-verified — OpenAI's deprecation note
- mindstudio.ai/blog/swe-rebench-benchmark-decontaminated-tests-model-inflation
- SWE-bench issue #465 — repo-state loopholes
- "SWE-ABS" (arXiv 2603.00520) — adversarial benchmark strengthening
- "On Randomness in Agentic Evals" (arXiv 2602.07150) — variance characterization

Stats methodology:
- en.wikipedia.org/wiki/McNemar's_test
- machinelearningmastery.com/mcnemars-test-for-machine-learning
- arXiv 1704.00045 — McNemar applied to system-vs-system on same tasks
