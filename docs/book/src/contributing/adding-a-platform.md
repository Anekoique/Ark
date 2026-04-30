# Adding a Platform

Adding a fourth platform (e.g. Cursor, Gemini Code) is mostly a registry entry plus a template tree. The command bodies in `init.rs` / `unload.rs` / `load.rs` / `remove.rs` / `upgrade.rs` iterate the same `&[&Platform]` slice; no command-body refactoring required.

## The Platform struct

Defined in `crates/ark-core/src/platforms.rs`:

```rust
pub struct Platform {
    pub id: &'static str,                        // canonical id, e.g. "cursor"
    pub cli_flag: &'static str,                  // the --<flag> name, e.g. "cursor"
    pub templates: &'static include_dir::Dir<'static>,
    pub dest_dir: &'static str,                  // e.g. ".cursor/commands/ark"
    pub removal_root: &'static str,              // e.g. ".cursor"
    pub managed_block_target: Option<&'static str>, // e.g. Some("CURSOR.md") or None
    pub hook_file: Option<HookFileSpec>,         // None if the platform has no hook contract
    pub extra_files: &'static [(&'static str, &'static str)], // (relative_path, contents)
}
```

The template tree at `templates/cursor/...` is captured at compile time via `include_dir!`. The `dest_dir` and `removal_root` should be cousin paths: `dest_dir` is where templates are extracted (a leaf), `removal_root` is where `ark remove` stops (a parent — usually the platform's top-level dir).

## Steps

1. **Add a template tree.** Create `templates/cursor/commands/ark/{quick,design,archive,record}.md`. Each one carries Cursor's frontmatter shape (whatever that platform expects) and a body translated from the Claude originals.
2. **Add an `include_dir!` constant.** In `crates/ark-core/src/templates.rs`, add `pub static CURSOR_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../templates/cursor/commands");`.
3. **Add a `Platform` const.** In `crates/ark-core/src/platforms.rs`:

   ```rust
   pub const CURSOR_PLATFORM: Platform = Platform {
       id: "cursor",
       cli_flag: "cursor",
       templates: &crate::templates::CURSOR_TEMPLATES,
       dest_dir: ".cursor/commands/ark",
       removal_root: ".cursor",
       managed_block_target: Some(".cursor/RULES.md"), // or whatever Cursor reads
       hook_file: None, // or Some(HookFileSpec { ... }) if Cursor has hooks
       extra_files: &[],
   };
   ```

4. **Register it.** Add `&CURSOR_PLATFORM` to the `PLATFORMS` slice. Order matters — the canonical iteration order is preserved across `init` / `upgrade` / `unload` / `load` / `remove`. New platforms append.
5. **Add CLI flags.** In `crates/ark-cli/src/main.rs`'s `InitArgs`, add `--cursor` / `--no-cursor` flags. Update the `flags()` mapping to route `"cursor"` to the `PlatformFlag` state.
6. **Update parity tests.** If `templates/cursor/commands/ark/<name>.md` should track Claude commands 1:1, add a parity test in `crates/ark-core/src/templates.rs` matching the existing `every_claude_command_has_a_codex_skill_sibling` shape.

## Hook file specs

If your platform has a hook contract (a JSON / YAML file Ark needs to install entries into):

```rust
pub const CURSOR_HOOK_SPEC: HookFileSpec = HookFileSpec {
    path: ".cursor/hooks.json",
    hooks_array_key: "SessionStart",   // the array under root.hooks that carries entries
    identity_key: "command",           // the field on each entry that identifies it
    entry_builder: ark_cursor_hook_entry,  // fn() -> serde_json::Value
};
```

`update_hook_file` and `remove_hook_file` are parameterized over `(path, hooks_array_key, identity_key)`; they handle the JSON surgery without you touching `serde_json` directly.

The hook file is **not** hash-tracked — it's re-applied unconditionally on every `ark init` / `ark load` / `ark upgrade`, like Claude's `settings.json` and Codex's `hooks.json`.

## Managed-block file

If your platform reads a markdown file that Ark needs to inject content into (parallel to Claude's `CLAUDE.md` and Codex/OpenCode's `AGENTS.md`):

```rust
managed_block_target: Some(".cursor/RULES.md"),
```

`init` writes a `<!-- ARK -->` managed block to that file (or merges into existing content). `unload` captures the block; `load` restores it; `remove` deletes it surgically (delimiters and all, surrounding content preserved).

Multiple platforms can share a managed-block target. The manifest dedupes on `(file, marker)`, so e.g. Codex and OpenCode both writing to `AGENTS.md` results in one block on disk and one entry in the manifest.

## Extra files

Files that need to ship but don't fit the template-tree shape (e.g. OpenCode's TypeScript plugin):

```rust
extra_files: &[(".cursor/plugins/ark.ts", CURSOR_ARK_PLUGIN_TS)],
```

Each tuple is `(relative_path, contents)`. The contents are baked into the binary at compile time. `init` writes them; `upgrade` re-applies them unconditionally (not hash-tracked).

## Testing

The source-scan invariant tests in `commands/init.rs`, `commands/upgrade.rs`, etc. enforce that:

- All filesystem ops route through `Layout` and `PathExt`.
- No bare `std::fs::*` calls.
- No hand-joined `.cursor/` (or `.ark/`, etc.) literal paths in command bodies.

If you add a new platform, you probably don't need to touch these tests — they scan command bodies, not platform definitions.

Add unit tests for your platform's shape (shape, `apply_managed_state` round-trip, `capture_hook` / `remove_hook` semantics) in `platforms.rs`'s test module. Pattern-match the existing `opencode_*` tests.

## What you don't have to touch

- The command bodies in `init.rs`, `unload.rs`, etc. iterate `PLATFORMS` and call methods on each `&Platform`. They never name a platform directly.
- `ark context` and the slash-command flow are platform-neutral once your hook is installed.
- The book's [Platforms](../platforms/claude-code.md) section gets a new chapter (per platform), but no other docs need surgery.

The whole point of the registry is that adding a platform is a *registry entry*, not a refactor.
