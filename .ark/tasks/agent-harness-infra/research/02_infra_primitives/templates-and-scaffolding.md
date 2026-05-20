# Templates and Scaffolding

## What the primitive means

A "template" is the *initial set of files* a tool drops into a project so
the user doesn't start from a blank directory. The interesting question
isn't *generating* the initial files (any tool can do that). It is the
**lifecycle**:

- What if the user edits a generated file?
- What if the upstream template changes?
- What if both happen?

This is the **template-update problem**, and it has been the dividing line
between successful and unsuccessful scaffolding tools for two decades.

| Tool | Update support | User-edit detection | Conflict resolution |
| ---- | -------------- | ------------------- | ------------------- |
| Cookiecutter | None | None | "Generate again, manually merge" |
| Yeoman | Weak | Per-file | Yeoman-specific 3-way merge |
| Copier | Yes | Via `.copier-answers.yml` + git | Diff-and-merge into project |
| `npm init`, `create-react-app` | None | None | "Eject" or run again |
| Ark | Yes | Hash-tracking via `.installed.json` | Per-file prompt; `--force` / `--skip-modified` / `--create-new` |

The pattern that *works* is Copier's: the project carries a small breadcrumb
(`.copier-answers.yml`) recording template version + answers; updates
diff-and-merge between the breadcrumb's template version and the new
one. Ark's manifest-hash approach is in the same family.

## Lineage: cookiecutter → yeoman → copier

### Cookiecutter (Python, 2013)

- Jinja-rendered file tree; user answers prompts; tool renders.
- "Cookiecutter has no template update support" (`copier.readthedocs.io/en/stable/comparisons/`).
- A whole secondary cottage industry (Cruft, Cookiecutter-update) tried
  to add this; Issue #784 / #785 are the canonical references.
- Workaround: a separate "template branch" tracking upstream; `git merge`
  into main.

### Yeoman (Node, 2012)

- Generator-driven; JS classes that emit files.
- "Yeoman has not-so-good support for template updates" — per-file diff
  prompts at re-run.
- Largely supplanted by framework-specific generators (`create-react-app`,
  `vue create`).

### Copier (Python, 2020)

- Born as Cookiecutter-with-updates.
- `.copier-answers.yml` in the generated project records the template
  source URL + commit + answers.
- `copier update` re-renders the template at HEAD, diffs against the old
  render, and 3-way merges into the project. User git-resolves conflicts.
- "Yeoman and Cookiecutter are dead; long live Copier" (RecallStack
  writeup, 2020). Strong claim, fair direction.

### Backstage Software Templates (Spotify, 2020)

- Enterprise platform-engineering generator; integrates with Spotify's
  internal devex.
- Like Cookiecutter, weak on updates — newer versions explore
  scaffolder-backend mutations.

### Verdict

Copier's design is the field standard. The breadcrumb-in-project (answer
file + template source + commit hash) is the right pattern. Ark's
manifest is conceptually close.

## How agent-harness templates work

### Claude Code plugins (`.claude-plugin/marketplace.json`)

- The marketplace is `.claude-plugin/marketplace.json` in a git repo.
- Each plugin is a directory containing `plugin.json` (optional when
  `strict: false`) + `commands/`, `agents/`, `skills/`, `hooks/`.
- Install: `/plugin marketplace add <git-url>`; `/plugin install <name>`.
- Update: presumably pull from the source git repo; no documented user-
  edit-detection — assumed to be read-only plugin install.
- "${CLAUDE_PLUGIN_ROOT}" resolves to the install dir; plugins reference
  their own files by environment variable.

### Codex skills

`.codex/skills/<name>/SKILL.md` + supporting files. The skill IS the
template — `init` extracts; user edits in place. Codex has no built-in
"update my skill from upstream" verb.

### OpenCode commands / plugins

`.opencode/commands/` for slash commands; `.opencode/plugins/*.ts` for
TypeScript context plugins. Bun-loaded. Ark ships `ark-context.ts`
verbatim via `extra_files` (`platforms.rs:372`).

### Cursor rules

`.cursor/rules/*.mdc` files. Not strictly "templates" — they ARE the user
configuration. Cursor doesn't ship template rules; users author them
fresh.

### Continue config

`config.yaml` at project root. Plus rules under `.continue/rules/`. Not
template-shaped; pure configuration.

### Aider

No templates whatsoever. `aider` is invoked in an existing repo and works
with what's there.

## Ark's template system in depth

Ark's templating is unusual: it **compiles template content into the
binary** rather than shipping it as a separate directory.

### `include_dir!` — compile-time embedding

`crates/ark-core/src/templates.rs` embeds template trees:

```rust
pub static CLAUDE_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/claude");
pub static CLAUDE_AGENT_TEMPLATES: Dir<'_> = ...;
pub static CODEX_TEMPLATES: Dir<'_> = ...;
pub static OPENCODE_TEMPLATES: Dir<'_> = ...;
```

The trees are walkable at runtime (`walk(tree)`) and each entry has a
`relative_path` + `contents`. AGENTS.md note: "Any change requires a
rebuild for it to take effect. The integration tests in
`commands/init.rs::tests` assert specific paths exist."

`include_dir` is the load-bearing crate (`docs.rs/include_dir`); the
`rust-embed` alternative has richer per-file metadata (hash, mtime) but
Ark computes hash separately via the manifest.

### Two extraction policies per platform

`crates/ark-core/src/platforms.rs:apply_managed_state`:

- **Templates tree** (`Platform.templates`) → extracted under `dest_dir`
  by `init::extract` with `WriteMode` honoring user edits (Skip /
  Override / CreateNew on conflict).
- **Agent tree** (`Platform.agents_templates`) → written
  **unconditionally** via `path.write_bytes`. "The unconditional write
  bypasses the upgrade conflict pipeline (agents are excluded from
  `collect_desired_templates`)." Quote from `platforms.rs:121`.

The split matters: slash commands can be user-modified safely; subagent
definitions are *reserved Ark stems* that must converge to canonical
shape. The "C-26 reserved-stem invariant" referenced in code comments is
the safety property: an Ark-shipped agent stem (e.g. `ark-researcher.md`)
must hold the canonical Ark contents post-init, regardless of prior
divergence.

### Manifest hash-tracking

`crates/ark-core/src/state/manifest.rs:hashes` — `BTreeMap<PathBuf, String>`
of content hashes:

```rust
pub fn record_file_with_hash(&mut self, path: impl Into<PathBuf>, contents: &[u8]) {
    // ...
    self.hashes.insert(path, hash_bytes(contents));
}
```

On `upgrade`, the planner reads each managed file, hashes current
content, compares to `manifest.hashes[path]`. Three outcomes:

- **Match** → user hasn't edited; safe to overwrite with new template.
- **Mismatch** → user has edited; conflict policy applies.
- **Absent from manifest** → user added a sibling; left alone.

Conflict policy flags from `commands/upgrade/plan.rs`:

- `--force` → overwrite all user edits.
- `--skip-modified` → leave user edits, skip the template update for
  that file.
- `--create-new` → write new template to `<file>.new`, preserve user
  version (Copier's "non-destructive" mode).
- Default (interactive) → prompt per file.

### Managed blocks — partial templates

For files Ark *co-authors* with the user (`CLAUDE.md`, `AGENTS.md`),
templates are partial: only the `ARK:START`/`ARK:END` block is
Ark-controlled. `io::fs::update_managed_block` performs the
insert/update/remove surgically (`crates/ark-core/src/io/fs/managed_block.rs`).
Re-applied unconditionally on every init/upgrade — Ark's analogue of
Copier's "always update this section."

### Hook entries — JSON-array surgery

`HookFileSpec` (`io/fs/hook.rs`) identifies Ark's entry in a shared JSON
array (`.claude/settings.json:hooks.SessionStart`) by command-string
match and surgically updates / removes / preserves it. Identity-based,
not hash-tracked. The canonical entry is regenerated by
`entry_builder()` every apply — so even a manually edited Ark hook entry
converges to canonical on upgrade.

### `extra_files` — verbatim writes, not hash-tracked

`platforms.rs:144` — `extra_files: &[(rel_path, body)]`. Used for
files whose canonical content lives as a `&'static str` rather than as
an `include_dir!` entry (notably `OPENCODE_ARK_CONTEXT_TS` and Codex's
`config.toml`). Written verbatim every apply; user edits are *not*
preserved.

### Round-trip preservation

The `commands/load.rs::tests` round-trip is: scaffold via templates →
user-edit some files → `unload` → `load` → assert byte-identical. This
asserts the *snapshot* path; the *upgrade* path is separately tested.
Two distinct invariants:

- **Snapshot/restore:** preserves every file Ark recorded in the
  manifest, byte-for-byte. Including the user's edits.
- **Upgrade:** brings template files toward the new shipped version;
  managed blocks + hooks re-converge; everything else honors the
  conflict policy.

## Comparison: Ark vs Copier on user-modified-file detection

Both use a manifest-shaped breadcrumb. Difference:

| Aspect | Copier | Ark |
| ------ | ------ | --- |
| Breadcrumb file | `.copier-answers.yml` | `.ark/.installed.json` |
| Records | template URL, commit, answers | files + hashes + managed blocks |
| Update strategy | Re-render at HEAD, 3-way merge into project, git resolves conflicts | Iterate manifest files; per-file conflict policy via flags |
| Resolution UX | git merge markers | CLI prompt (or flag-driven) |
| Granularity | File-level | File-level + managed-block level + hook-entry level |
| User answer storage | YAML in project | None (Ark has no answer file) |

Ark is more **fine-grained** because Ark co-authors *parts* of files
(managed blocks in CLAUDE.md, hook entries in settings.json) that
Copier-shape templates cannot easily handle. Ark is also *more
opinionated* because there are no template variables — every Ark project
gets the same shipped templates verbatim.

## Directions for Ark

1. **Adopt the marketplace.json plugin shape.** Ark already extracts a
   `.claude/commands/ark/` subtree + `.claude/agents/` + a hook + a
   managed block. Wrap all of this in a Claude-Code-style plugin
   manifest (`.claude-plugin/marketplace.json` at the top of
   `templates/claude/`) so Ark is installable via `/plugin install`. The
   `ark init` path remains; `/plugin install` becomes a secondary,
   richer onboarding path. Code site: add `templates/claude/.claude-plugin/`;
   no `crates/ark-core/src` change initially. Pairs with Direction 4 in
   `mcp-and-tool-registries.md`.
2. **Per-template `--var KEY=VALUE` answers.** Today Ark ships
   templates as static bytes; user fills `task.toml` slug+title at
   `task new`. A tiny variable-substitution layer (mustache, `{{slug}}`
   only, no general Jinja) would let templates carry per-project
   identity — e.g. the user's GitHub org name in CLAUDE.md. Pairs with
   the Copier breadcrumb story. Code site:
   `crates/ark-core/src/templates.rs` (extend `walk`).
3. **Migration scripts in templates.** Today `ark upgrade` blindly
   refreshes. Embed a `.ark/templates/migrations/<version>.md` per
   shipped version that documents user-visible changes; surface these
   in the upgrade plan output. Cheaper than fully automated migration,
   adequate for the Ark contract (binary + templates ship together).
   Code site: `crates/ark-core/src/commands/upgrade/plan.rs`.
4. **Promote `extra_files` to hash-tracked.** Today
   `OPENCODE_ARK_CONTEXT_TS` (a 60-line TS file) is rewritten every
   apply, blowing away user edits silently. Either (a) mark it
   explicitly Ark-reserved and document that fact, or (b) extend
   `extra_files` to participate in the hash conflict pipeline. Today's
   behaviour is a known data-loss vector for OpenCode users. Code site:
   `crates/ark-core/src/platforms.rs:144`.
5. **Snapshot/upgrade interaction test.** Round-trip test that
   `init -> edit-managed-file -> unload -> load -> upgrade` correctly
   preserves the user edit through both lifecycles. Today the two paths
   are tested independently (`commands/load.rs::tests` and
   `commands/upgrade/`); the composition is the realistic user flow.
   Code site: extend `crates/ark-core/src/commands/load.rs::tests` with
   a `load_then_upgrade_preserves_edits` case.

## Caveats / Not found

- The exact `plugin.json` schema for Claude Code plugins is partially
  community-defined; see `hesreallyhim/claude-code-json-schema` for an
  unofficial JSON Schema.
- I did not survey `npm init` / `cargo new` / Maven archetypes; treat
  them as cookiecutter-shaped (generate once, no update).
- Cursor's rules format and Continue's config format do not exercise the
  template-update problem because they are user-authored from scratch.
- Whether Anthropic's "Skills" open standard (Dec 2025) defines a
  template-update model is unclear — it specifies install but not
  refresh.

## Sources

- [include_dir crate docs](https://docs.rs/include_dir/latest/include_dir/)
- [rust-embed crate](https://crates.io/crates/rust-embed)
- [Cookiecutter vs Yeoman (CookieCutter.io)](https://www.cookiecutter.io/article-post/compare-cookiecutter-to-yeoman)
- [Cookiecutter Issue #784 — template updates](https://github.com/cookiecutter/cookiecutter/issues/784)
- [Copier comparisons](https://copier.readthedocs.io/en/stable/comparisons/)
- [Copier updating docs](https://copier.readthedocs.io/en/stable/updating/)
- [Cookiecutter alternatives](https://safjan.com/cookiecutter-alternatives/)
- [Yeoman and Cookiecutter are dead; long live Copier](https://www.recallstack.icu/en/2020/04/18/yeoman-and-cookiecutter-are-dead-long-live-copier/)
- [Claude Code Plugin Marketplaces](https://code.claude.com/docs/en/plugin-marketplaces)
- [Official Anthropic marketplace.json](https://github.com/anthropics/claude-plugins-official/blob/main/.claude-plugin/marketplace.json)
- [Claude Code JSON Schemas (unofficial)](https://github.com/hesreallyhim/claude-code-json-schema)
- [AGENTS.md spec](https://agents.md/)
