# Workspace Layout

This chapter is for contributors extending Ark itself. It walks through the Cargo workspace and the `ark-core` module map.

## Cargo workspace

```
.
├── crates/
│   ├── ark-cli/        # thin clap adapter — keep it boring
│   └── ark-core/       # all logic lives here
├── templates/
│   ├── ark/            # files extracted into the host's .ark/
│   ├── claude/         # files extracted into the host's .claude/
│   ├── codex/          # files extracted into the host's .codex/
│   └── opencode/       # files extracted into the host's .opencode/
├── docs/
│   ├── book/           # this book
│   └── ...             # roadmap, design notes
├── reference/          # third-party projects we draw from (READ-ONLY)
└── README.md           # user-facing overview
```

Two crates: `ark-cli` is a thin clap adapter, `ark-core` is the library. The split exists so the library can be exercised without going through clap parsing — every test uses the library directly, not the binary.

## `ark-core` module map

```
ark-core/src/
├── lib.rs              # public re-exports
├── error.rs            # Error enum, Result alias
├── layout.rs           # Layout + project-root constants + discover_from
├── platforms.rs        # Platform registry (Claude / Codex / OpenCode)
├── templates.rs        # include_dir!() trees + walker
├── io/
│   ├── mod.rs          # public surface
│   ├── path_ext.rs     # PathExt trait wrapping std::fs
│   ├── fs.rs           # write_file, walk_files, managed-block ops, hook-file editor
│   └── git.rs          # the only sanctioned Command::new("git") site
├── state/
│   ├── manifest.rs     # .ark/.installed.json
│   └── snapshot.rs     # .ark.db capture/restore (incl. SnapshotHookBody)
└── commands/
    ├── init.rs         # scaffold from templates
    ├── load.rs         # restore from .ark.db OR scaffold
    ├── unload.rs       # capture into .ark.db, remove live files
    ├── remove.rs       # unconditional wipe
    ├── upgrade.rs      # refresh embedded templates to current CLI version
    ├── archive.rs      # bulk-archive committed tasks (top-level `ark archive`)
    ├── cleanup.rs      # sweep stale worktrees (top-level `ark cleanup`)
    ├── context/        # `ark context` — read-only state snapshot
    │   ├── model.rs    #   Context + sub-structs, schema=1
    │   ├── gather.rs   #   one-pass collection (git + tasks + specs)
    │   ├── projection.rs # Scope / PhaseFilter / project()
    │   ├── checkout.rs #   checkout-kind + focus resolution
    │   ├── spec_tree.rs #  recursive features-INDEX tree
    │   ├── subagents.rs #  installed-subagent enumeration
    │   ├── render.rs   #   text-mode Display
    │   └── related_specs.rs # PRD [**Related Specs**] parser
    └── agent/          # `ark agent` namespace (hidden CLI, not semver)
        ├── state.rs    #   TaskToml + legal-transition table
        ├── task/       #   lifecycle (new/plan/review/execute/verify/commit/archive/
        │               #   resume/discard/promote) + worktree/ (list/cleanup)
        ├── spec/       #   feature SPEC extract / import / register (recursive paths)
        ├── workspace/  #   identity + per-developer journal (record/stamp/transaction)
        └── template.rs #   internal helper: extract embedded templates
```

## The `ark agent` namespace

`ark agent` is a **hidden** top-level subcommand (`#[command(hide = true)]`) that packages the structural workflow mutations as typed Rust commands. Callers are the shipped slash commands and the workflow doc; end users prefer those.

**Stability policy.** The CLI surface under `ark agent` is **not covered by semver**. The contract is internal — the binary and its shipped templates version together, not against external callers. `ark --help` does not list `agent`; `ark agent --help` renders its children with a stability banner.

**Responsibilities of this layer:**

- Create task directories (`task new`) and move them on archive (`task archive`) — whenever the operation touches filesystem structure that has to be correct.
- Transition phases (`task plan` / `review` / `execute` / `verify` / `commit` / `archive`) — the state machine enforces legality per tier (research tier runs `research → committed → archived`).
- Extract SPEC bodies from PLANs (recursive `specs/features/<path>/`) and upsert rows in each `INDEX.md`'s managed block from leaf to root.
- Append per-developer workspace journal entries inside the closing commit (`task commit`) or on demand (`workspace record`).

**Not** this layer's responsibility:

- Rare lifecycle operations — task reopening — the workflow doc tells the agent to hand-edit these. Tier promotion is supported mid-flight via `ark agent task promote`; other ad hoc lifecycle edits remain manual.
- Artifact content (PRD prose, PLAN sections, REVIEW verdicts) — agent's judgment.
- Git / GH operations — agent uses them directly.
- Consistency checks / doctoring — reviewer judgment.

## Build, test, lint

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs all four; PRs that fail any won't merge. Run them before requesting review.

## End-to-end smoke test

```bash
cargo build --release
TMP=$(mktemp -d)
./target/release/ark load --dir "$TMP"
./target/release/ark unload --dir "$TMP"
./target/release/ark load --dir "$TMP"
./target/release/ark remove --dir "$TMP"
rm -rf "$TMP"
```

Round-trip must preserve user-edited and user-added files under `.ark/` and `.claude/commands/ark/`.

## Code conventions

- **Errors.** Every fallible op returns `crate::error::Result<T>`. Wrap `std::io::Error` via `Error::io(path, source)`. Never `unwrap()` outside tests; reserve `expect("…")` for documented invariants only.
- **Filesystem.** Prefer the methods on `io::PathExt` over `std::fs::*` so error paths stay structured. The trait is implemented for any `T: AsRef<Path>`. Source-scan invariant tests in `commands/init.rs`, `commands/upgrade.rs`, etc. enforce this at compile-test time.
- **Managed blocks** in text files are owned by `io::fs::{read,update,remove}_managed_block` — don't hand-write `ARK:START` / `ARK:END` HTML-comment delimiters elsewhere.
- **Project paths.** Route through `layout::Layout` (`ark_dir()`, `claude_md()`, `owned_dirs()`, etc.). Don't `root.join(".ark")` ad-hoc. The same source-scan tests catch hand-joined `.ark/` literals.
- **Commands return summaries that `impl Display`.** The CLI calls one `render(summary)` per dispatch — don't add ad-hoc print logic.
- **Style.** Functional combinators (`try_for_each`, `and_then`, `map_or`) where they read more clearly; explicit imperative form where they don't. `cargo fmt` settles all formatting debates.
- **Tests.** Every module that does I/O has unit tests using `tempfile::tempdir()`. Round-trip coverage lives in `commands/load.rs::tests`.

## Reference material

- `reference/` mirrors third-party projects we study (trellis, openspec, spec-kit, superpowers, agents-cli, etc.). **Treat it as read-only**; don't edit anything under `reference/`.
- Design history and roadmap notes live in `docs/`.

## What not to do

- Don't add files just to host one function. Single-responsibility helpers belong in the module that owns the responsibility.
- Don't introduce `crate::*::*` paths that bypass `layout::Layout` for path computation.
- Don't shell out to git, `mv`, or `rm` from Rust code — use `PathExt`. The single sanctioned `Command::new("git")` site is `io/git.rs`; everything else routes through it.
- Don't mutate `reference/` or commit anything from `target/`.
