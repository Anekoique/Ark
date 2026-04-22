# Ark — Agent Guide

A simple CLI agent harness and development workflow for orchestrating AI-driven programming tasks. This file is read by AI coding agents (Codex, Claude Code, etc.) working in **this repository**. Users of the published `ark` CLI should read [README.md](README.md) instead.

## Project at a Glance

- **Crate type:** Rust workspace shipping the `ark` binary.
- **MSRV / toolchain:** pinned in `rust-toolchain.toml` (nightly — required for `rustfmt` unstable options).
- **Releases:** automated via `dist` (see `[workspace.metadata.dist]` in `Cargo.toml`); binaries land on GitHub Releases and `@anekoique/ark` on npm.
- **Status:** experimental Phase 0; only Claude Code is targeted as an integration.

## Repository Layout

```
.
├── crates/
│   ├── ark-cli/        # thin clap adapter — keep it boring
│   └── ark-core/       # all logic lives here
├── templates/
│   ├── ark/            # files extracted into the host's .ark/
│   └── claude/         # files extracted into the host's .claude/
├── docs/               # roadmap, design notes
├── reference/          # third-party projects we draw from (READ-ONLY)
└── README.md           # user-facing overview
```

### `ark-core` module map

```
ark-core/src/
├── lib.rs              # public re-exports
├── error.rs            # Error enum, Result alias
├── layout.rs           # Layout + project-root constants
├── templates.rs        # include_dir!() trees + walker
├── io/
│   ├── path_ext.rs     # PathExt trait wrapping std::fs
│   └── fs.rs           # write_file, walk_files, managed-block ops
├── state/
│   ├── manifest.rs     # .ark/.installed.json
│   └── snapshot.rs     # .ark.db + gitignore protocol
└── commands/
    ├── init.rs         # scaffold from templates
    ├── load.rs         # restore from .ark.db OR scaffold
    ├── unload.rs       # capture into .ark.db, remove live files
    └── remove.rs       # unconditional wipe
```

## Build, Test, Lint

```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

CI runs all four; PRs that fail any won't merge. Run them before requesting review.

## End-to-End Smoke Test

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

## Code Conventions

- **Errors:** every fallible op returns `crate::error::Result<T>`. Wrap `std::io::Error` via `Error::io(path, source)`. Never `unwrap()` outside tests; reserve `expect("…")` for documented invariants only.
- **Filesystem:** prefer the methods on `io::PathExt` over `std::fs::*` so error paths stay structured. The trait is implemented for any `T: AsRef<Path>`.
- **Managed blocks** in text files are owned by `io::fs::{read,update,remove}_managed_block` — don't hand-write `<!-- ARK:START -->` strings elsewhere. The `Marker` helper inside `fs.rs` is private on purpose.
- **Project paths:** route through `layout::Layout` (`ark_dir()`, `claude_md()`, `owned_dirs()`, etc.). Don't `root.join(".ark")` ad-hoc.
- **Commands return summaries that `impl Display`.** The CLI calls one `render(summary)` per dispatch — don't add ad-hoc print logic.
- **Style:** functional combinators (`try_for_each`, `and_then`, `map_or`) where they read more clearly; explicit imperative form where they don't. `cargo fmt` settles all formatting debates.
- **Tests:** every module that does I/O has unit tests using `tempfile::tempdir()`. Round-trip coverage lives in `commands/load.rs::tests`.

## Lifecycle Model (what `ark` does)

| Command | Effect |
|---|---|
| `ark init` | Scaffold `.ark/` + `.claude/commands/ark/` from embedded templates; insert `<!-- ARK -->` block in `CLAUDE.md`; record artifacts in `.ark/.installed.json`. |
| `ark load` | If `.ark.db` exists → restore from snapshot (and remove the snapshot + its `.gitignore` line). Otherwise → behave like `init`. Refuses if `.ark/` exists; `--force` wipes first. |
| `ark unload` | Capture every file under owned dirs and every recorded managed block into `.ark.db`; remove the live footprint; add `.ark.db` to `.gitignore`. |
| `ark remove` | Unconditional wipe of `.ark/`, `.claude/commands/ark/`, managed blocks, `.ark.db`, and the `.gitignore` entry. |

User-authored files inside owned dirs (`.ark/tasks/...`, custom slash commands) survive an `unload` → `load` round-trip losslessly.

## When Editing Templates

`templates/` is embedded into the binary at build time via `include_dir!`. Any change requires a rebuild for it to take effect. The integration tests in `commands/init.rs::tests` assert specific paths exist — update them when you add or remove template files.

## Reference Material

- `reference/` mirrors third-party projects we study (trellis, openspec, spec-kit, superpowers, agents-cli, etc.). **Treat it as read-only**; don't edit anything under `reference/`.
- Design history and roadmap notes live in `docs/`.

## What Not to Do

- Don't add files just to host one function. Single-responsibility helpers belong in the module that owns the responsibility (see `Snapshot`'s gitignore protocol, `Marker` private to `io/fs.rs`).
- Don't introduce `crate::*::*` paths that bypass `layout::Layout` for path computation.
- Don't shell out to git, `mv`, or `rm` from Rust code — use `PathExt`.
- Don't mutate `reference/` or commit anything from `target/`.
