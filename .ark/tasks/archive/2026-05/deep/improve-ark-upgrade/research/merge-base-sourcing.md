# Merge Base Sourcing — Research

- Query: For the new `[upgrade] merged` diff3 strategy, where does `ark upgrade` get the **base** bytes (the original template the user's file started from)? The manifest stores only a SHA-256 hash, not the bytes.
- Scope: mixed (internal codebase + external prior art / crates)
- Date: 2026-05-27

## Question

A diff3 3-way merge needs three inputs:
- **base** = the original template bytes the user's file started from (the version Ark last wrote)
- **ours** = the user's current on-disk file
- **theirs** = the new embedded template from the current CLI

`ours` and `theirs` are both available at upgrade time. `theirs` is the embedded
template (`collect_desired_templates`, `crates/ark-core/src/commands/upgrade/mod.rs:177`).
`ours` is read from disk (`absolute.read_optional()`, `plan.rs:199`). The gap is
**base**: the manifest records only a hash, never the bytes.

Confirmed: `Manifest.hashes: BTreeMap<PathBuf, String>` stores `hash_bytes(contents)`
only (`crates/ark-core/src/state/manifest.rs:30-32, 66-72`). `hash_bytes` is SHA-256
hex (per SPEC C-1, `.ark/specs/features/ark-upgrade/SPEC.md:195`). There is no
byte-level record of what Ark last wrote. So at upgrade time we can hash-detect
"user modified this" (`classify`, `plan.rs:134-153`) but cannot reconstruct base.

## Options

### Option 1 — Store original bytes at write time

**Mechanism.** Every place Ark writes a tracked template, persist the exact bytes
so a future upgrade has the base. There are two write sites that already call
`record_file_with_hash`:
- init: `extract_filtered` → `manifest.record_file_with_hash(relative, &contents)`
  (`crates/ark-core/src/commands/init.rs:246`)
- upgrade: the apply loop → `manifest.record_file_with_hash(&relative, &contents)`
  for `Write` and `RefreshHashOnly` (`upgrade/mod.rs:309, 317`)

Both already hold the post-splice `contents: Vec<u8>` in hand, so capturing bytes is
a one-line extension at each site. The base for a future merge is exactly these
bytes (already post-managed-block-splice, which is correct — the user started from
the spliced file on disk).

Two sub-options for *where* the bytes live:

**1a. Extend the manifest to carry bytes (base64).** Add a parallel map, e.g.
`bases: BTreeMap<PathBuf, String>` (base64), mirroring how `snapshot.rs` already
base64-encodes full file bytes (`SnapshotFile.content_b64`,
`crates/ark-core/src/state/snapshot.rs:46-51`, via `B64.encode` at line 117). This
is established precedent in the same codebase — `.ark.db` round-trips arbitrary bytes
losslessly through base64 + serde_json today.
- Cost: manifest size grows from ~hashes to ~full corpus. The tracked template set
  is dozens of small markdown/toml files (the `init` test enumerates ~22 paths,
  `init.rs:274-297`); base64 inflates by ~33%. Likely tens to low-hundreds of KB.
- VCS impact: **`.ark/.installed.json` IS git-tracked in a self-hosting repo** —
  `git ls-files` lists `.ark/.installed.json`, and the shipped `.ark/.gitignore`
  (`templates/ark/.gitignore`) ignores only `worktrees/`, `.state.toml*`,
  `.developer` — NOT `.installed.json`. So embedding bodies would commit the entire
  template corpus (base64) into every host project's VCS and churn the manifest on
  every upgrade. That is a meaningful diff-noise / repo-bloat regression for a file
  users currently see as a small hash ledger.

**1b. Separate sidecar store under `.ark/`.** Keep the manifest as-is; write base
bytes to a dedicated location, e.g. `.ark/.upgrade-base/<path>` (mirroring tree) or
a single `.ark/.upgrade-base.json` blob. To avoid the VCS churn of 1a, ship a
`.gitignore` rule for it (the shipped `.ark/.gitignore` is the natural home — add a
line like `.upgrade-base/`). This keeps base bytes local-only, matching how `.ark.db`
and `.developer` are treated as non-committed local state.
- Cost: same byte volume as 1a but off the VCS path; one extra write per tracked
  file per init/upgrade. Needs its own lifecycle hooks in `unload`/`remove` (the
  sidecar must be captured/wiped alongside the manifest) — `remove.rs` and
  `unload.rs` would need to learn about it.
- Note: a sidecar is only meaningful for files that are NOT git-committed. If the
  host commits `.ark/`, the user could in principle recover base from git history,
  but Ark must not shell to git (CLAUDE.md "What Not to Do"), so an in-tree sidecar
  is the self-contained option.

**Migration impact (both sub-options).** Existing installs have hash-only manifests
and no stored bytes. The very first post-feature upgrade cannot have a base for any
pre-existing file (the bytes were never recorded). Base becomes available only from
the *next* write onward. So Option 1 must be paired with Option 3 (no-base fallback)
for the transition window regardless. `serde(default)` on the new field keeps old
manifests deserializable — same pattern as the existing `hashes` field
(`#[serde(default, skip_serializing_if = ...)]`, `manifest.rs:31`) and the
`legacy_manifest_without_hashes_field_deserializes` test (`manifest.rs:172-182`).

**Verdict.** Viable and the only option that actually yields a base without network.
**1b (gitignored sidecar)** avoids the committed-manifest bloat of 1a, at the cost of
extra lifecycle plumbing. **1a (manifest base64)** is simpler to implement and reuses
the exact snapshot precedent, but bloats a git-tracked file. The PLAN must decide
manifest-bloat-vs-plumbing; either way it only stores bases for files actually listed
under `[upgrade] merged` (no need to store bytes for the whole corpus — scope storage
to merged paths to bound the cost).

### Option 2 — Reconstruct base from a prior release's embedded templates

**Mechanism.** The base equals the embedded template of the version recorded in
`manifest.version`. In principle, fetch the templates that shipped with that version.

**Why it is a dead end.** Ark embeds only the **current** CLI's templates via
`include_dir!` (`crates/ark-core/src/templates.rs:34-42`; the macro reads
`$CARGO_MANIFEST_DIR/../../templates/...` at build time). Past versions' templates
are not embedded in the binary. Recovering them would require fetching an old release
artifact or git ref — i.e. **network or git I/O**.

Confirmed against the SPEC: **NG-1 "No network I/O; no migration manifest system"**
(`.ark/specs/features/ark-upgrade/SPEC.md:11`) and **C-12 "All filesystem access in
upgrade.rs routes through `io::PathExt`/`io::fs`"** (SPEC:206). CLAUDE.md also bars
shelling out to git from Rust ("Don't shell out to git… use `PathExt`"). Fetching old
templates violates NG-1 directly.

**Cost / Migration impact.** N/A — infeasible under the existing non-goals.

**Verdict.** **Rejected.** Confirmed dead end: the data does not exist in the binary,
and acquiring it requires network/git access that NG-1 forbids. Document as
considered-and-rejected so the PLAN does not relitigate it.

### Option 3 — No-base fallback

**Mechanism.** When base bytes are unavailable for a `merged` path, do not attempt
diff3 (a 2-way "merge" with no base produces garbage). Instead fall back to the
existing conflict pipeline: classify as `UserModified` / `AmbiguousNoHash` and route
through `resolve_conflict` → overwrite / skip / `.new` (`plan.rs:163-174, 229-245`).

**Detection** ("do we have base bytes for this path?"): a single lookup against
whichever store Option 1 picks — e.g. `manifest.base_for(path).is_some()` (1a) or
sidecar file exists (1b). No base → fallback branch. This is cheap and local.

The PRD already mandates this behavior: *"a `merged` file with no recoverable base
falls back to the existing overwrite/skip/.new conflict path rather than producing a
bogus merge"* (`PRD.md:24`) and *"The manifest records only a hash… so this task must
establish where the base comes from"* (`PRD.md:24`).

**UX.** On fallback, the upgrade summary / `--dry-run` preview should distinguish
"merged" outcomes from "fell back to conflict (no base)" so the user understands why
a `merged` file prompted instead of auto-merging. After one post-feature upgrade
writes the file (Option 1 records its bytes), subsequent upgrades have a base and the
merge path activates — so the fallback is self-healing for files Ark re-writes, and
permanent only for files the user has diverged and never let Ark overwrite.

**Cost.** Near-zero — reuses the entire existing conflict machinery.

**Migration impact.** This IS the migration strategy for the hash-only-manifest
transition window. Required regardless of which Option 1 sub-option is chosen.

**Verdict.** **Required, complementary to Option 1.** Not an alternative — Options 1+3
together are the design. Option 3 alone (no base storage) means `merged` never
actually merges, defeating the feature.

## Prior Art

How real tools source the merge base for "re-apply template, but the user edited it":

- **git merge-file / git's 3-way merge.** Git computes the base as the *merge-base
  commit* (nearest common ancestor) and reads the base file content from that commit's
  tree. `git merge-file <current> <base> <other>` takes the base as an explicit file
  argument; `--diff3` renders the base region inside conflict markers. Key point: git
  *has* the base because every prior state is in the object store. Ark has no such
  store for templates — which is exactly why Option 2 fails and Option 1 (store it
  ourselves) is necessary.
  Source: <https://git-scm.com/docs/git-merge-file>
- **cruft (cookiecutter updater) — the closest analogue.** cruft stores the **template
  git commit hash** it generated from in `.cruft.json`. On `cruft update` it
  regenerates the template at *that stored commit* (= base), regenerates at the latest
  commit (= theirs), diffs the two, and applies that patch to the project (= ours).
  It reconstructs the base by re-rendering the old template version. This is Option 2
  in spirit — but cruft can do it because it has the template *repo* on hand to check
  out the old commit. Ark embeds only one version and forbids network/git, so Ark
  cannot replay this; it must instead *persist* the base (Option 1).
  Sources: <https://cruft.github.io/cruft/>,
  <https://github.com/cruft/cruft>
- **chezmoi.** `chezmoi merge` runs a 3-way merge between destination state (ours),
  source state, and target state (rendered template). Notably, chezmoi's docs state
  that for non-template files the merge does **not** supply a true git-style base —
  the "third" input is the rendered target, not a recorded ancestor. It largely defers
  to an interactive merge tool (vimdiff default) rather than a deterministic
  ancestor-based diff3. Takeaway: a tool without a recorded ancestor degrades to
  interactive/no-base merging — reinforcing that Ark needs Option 1 to get a *true*
  diff3 rather than a 2-way approximation.
  Source: <https://www.chezmoi.io/reference/commands/merge/>
- **agent-infra (`reference/agent-infra/`, the PRD's cited inspiration).** Its
  `files.merged` strategy is documented as *"AI-assisted merge preserves local
  additions where possible"* (README "File Management Strategies" table). It is **not**
  a deterministic diff3 with a stored ancestor — it hands the reconciliation to an AI
  workflow. So agent-infra inspires Ark's **declared-strategy config layer**
  (`managed`/`merged`/`ejected`), NOT the base-sourcing mechanism. Ark is choosing a
  stricter, deterministic diff3 where agent-infra punts to an LLM; that is the gap this
  research closes. Config shape it uses (for the PLAN's config design): a `files`
  object with `managed` / `merged` / `ejected` arrays of path/prefix strings
  (`reference/agent-infra/README.md` ~L808–889).

Synthesis: every deterministic 3-way merger needs a real ancestor. Tools that have a
version store (git) or a template repo (cruft) recover it; tools that lack one
(chezmoi non-template, agent-infra) degrade to interactive or AI merge. Ark has
neither store and forbids acquiring one (NG-1), so it must **record the base itself at
write time** — Option 1.

## diff3 Crate Options

| Crate | Version | Conflict-marker support | `&[u8]` API | Maintenance | Verdict |
|-------|---------|-------------------------|-------------|-------------|---------|
| `diffy` | 0.5.0 (released 2026-04-27, Rust 2024) | Yes — git-style `<<<<<<< ours` / `======= ` / `>>>>>>> theirs`; `ConflictStyle::{Merge, Diff3}` (Diff3 adds `\|\|\|\|\|\|\| original` base region) via `MergeOptions` | Yes — `merge_bytes(ancestor, ours, theirs)` and `MergeOptions::merge_bytes`, alongside utf8 `merge()`; returns `Result<_, _>` where `Err` carries the conflict-marked output | Active: ~834k downloads/mo, used by 188 crates; author Brandon Williams; current deps | **Recommended.** Purpose-built diff3, git-style markers configurable, byte API matches Ark's `Vec<u8>` template handling |
| `threeway_merge` | (lib.rs entry) | Yes — `MergeStyle::{Normal, Diff3, ZealousDiff3}`, customizable base/ours/theirs labels | str-oriented | Smaller, less-used than diffy | Fallback only if more marker-label control needed; less battle-tested |
| `similar` | 2.x | Has diffing + `TextMerge`/3-way merge helpers | str-oriented (utf8) | Very active (mitsuhiko), widely used | Heavier (general diffing lib); usable but diffy is leaner for the single merge need |
| hand-rolled | — | Would reimplement diff3 + marker rendering | — | Ark would own it | **Avoid** — diff3 is subtle (false-conflict minimization); reinventing risks bugs the existing crates already handle |

Dependency-fit notes:
- Ark's Cargo tree (`crates/ark-core/Cargo.toml`) has **no** diff/merge crate today.
  `base64`, `serde_json`, `sha2` are present (relevant for Option 1 byte storage).
- `diffy` 0.5 is `no_std` by default; base merge functionality needs **no optional
  features**, only `hashbrown 0.17` as a mandatory dep — a minimal addition. License
  is MIT OR Apache-2.0 (compatible with Ark's MIT).
- The PRD requires *"Git-style conflict markers"* (`PRD.md:22`); `diffy`'s default
  `ConflictStyle::Merge` produces exactly `<<<<<<<`/`=======`/`>>>>>>>`, so it
  satisfies the contract out of the box. `merge_bytes` returning the marked-up bytes
  in the `Err` arm maps cleanly to the PRD's "merge-conflict → write file with markers,
  report as conflict" outcome (`PRD.md:22, 25`).

## Recommendation

**Adopt Option 1 + Option 3 together, using `diffy::merge_bytes`.**

1. **Source the base by storing it at write time (Option 1), scoped to `merged`
   paths.** Only files the user lists under `[upgrade] merged` need a recorded base, so
   bound storage to that set rather than the whole corpus. Reuse the
   `record_file_with_hash` call sites (`init.rs:246`, `upgrade/mod.rs:309,317`) which
   already hold the post-splice bytes.
2. **Fall back to the existing conflict pipeline whenever no base is recorded
   (Option 3)** — covers pre-feature installs and any merged path Ark has not yet
   re-written. This is mandatory and the PRD already specifies it.
3. **Use `diffy` 0.5 `merge_bytes`** with default (git-style) conflict markers. Clean
   merge (`Ok`) applies silently and counts as `merge-clean`; conflict (`Err`) writes
   the marker-laden bytes and counts as `merge-conflict`.
4. **Reject Option 2** explicitly in the PLAN (NG-1 forbids the network/git access it
   needs; past templates are not embedded).

**The 2–3 key trade-offs the PLAN must decide:**

- **Where base bytes live — manifest base64 (Option 1a) vs. gitignored sidecar
  (Option 1b).** 1a is simplest and reuses `snapshot.rs` precedent but bloats the
  **git-tracked** `.ark/.installed.json` (confirmed tracked; shipped `.ark/.gitignore`
  does not exclude it) and churns it on every upgrade. 1b keeps base bytes local/
  untracked (add a rule to the shipped `.ark/.gitignore`) but needs new lifecycle
  plumbing in `unload`/`remove`. Decide manifest-bloat vs. plumbing-cost — and decide
  whether base storage is scoped to `merged` paths only (recommended) or all tracked
  files.
- **Migration UX for the transition window.** Every existing install upgrades once with
  no base for any file, so the first upgrade silently uses the Option 3 fallback for
  merged files; merging only activates on the *next* upgrade after Ark re-records the
  bytes. The PLAN must decide how `--dry-run`/summary label this ("no base — fell back
  to conflict") so it is not mistaken for a bug.
- **New dependency acceptance.** Adding `diffy` (MIT/Apache-2.0, ~834k dl/mo, +`hashbrown`)
  vs. hand-rolling diff3. Recommendation is to take the crate; the PLAN should confirm
  the dependency-addition policy and that `merge_bytes` (not just utf8 `merge`) is used
  so non-utf8 template bytes round-trip safely.

## Caveats / Not found

- I did not find the `reference/agent-infra/` tree inside the worktree — `reference/`
  is gitignored out of worktrees and lives only in the main checkout
  (`/Users/anekoique/Agent/Ark/reference/agent-infra/`). I read it there. agent-infra's
  `merged` is AI-assisted, not deterministic diff3, so it informs the config layer, not
  base sourcing.
- I did not inspect diffy's source line-by-line for exact `MergeOptions` builder method
  names; the public surface (`merge`, `merge_bytes`, `ConflictStyle::{Merge, Diff3}`,
  `MergeOptions`) is confirmed via docs.rs but the exact builder ergonomics (e.g.
  `set_conflict_style`) should be verified against `docs.rs/diffy/0.5.0` during PLAN/
  implementation.
- Exact byte volume of storing the full corpus base64 was estimated (tens to
  low-hundreds of KB) from the ~22-path init enumeration, not measured. If the PLAN
  scopes base storage to `merged` paths only (recommended), this is moot.
- No existing Ark code performs any merge beyond managed-block splicing
  (`merge_managed_blocks`, `managed_block.rs:92`); diff3 is genuinely new surface. The
  managed-block path is explicitly out of scope for `merged` per PRD (`PRD.md:23`).
