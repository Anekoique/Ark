# Research: Install / Upgrade / Uninstall Lifecycle

- Query: Ark's `load`/`unload`/`remove`/`upgrade` verbs as a UX case study; why most CLIs don't have these verbs; snapshot patterns; reversibility as a primary design property.
- Scope: mixed
- Date: 2026-05-20

## Findings

### Files (internal)

| Path | Description |
| ---- | ----------- |
| `crates/ark-core/src/commands/init.rs` | The "create from templates" path. Used by `load` when no snapshot exists, callable directly by users. |
| `crates/ark-core/src/commands/load.rs` | If `.ark.db` exists → restore from snapshot. Otherwise → behave like `init`. The bring-it-back verb. |
| `crates/ark-core/src/commands/unload.rs` | Capture every owned file + every managed block + every hook entry into `.ark.db`, then wipe the live footprint. The freeze verb. |
| `crates/ark-core/src/commands/remove.rs` | Unconditional wipe of everything (`.ark/`, platform dirs, managed blocks, `.ark.db`, hook entries). The eject verb. |
| `crates/ark-core/src/commands/upgrade/mod.rs` | Refresh embedded templates to the current CLI version, with hash-based modification detection. The migrate verb. |
| `crates/ark-core/src/state/snapshot.rs` | The `.ark.db` file format — JSON with files (base64-encoded bytes), managed blocks, hook entries. |
| `crates/ark-core/src/state/manifest.rs` | `.ark/.installed.json` — sha256 hashes for every Ark-written file. Drives upgrade conflict detection. |
| `AGENTS.md` (l. 114-126) | Lifecycle table — the canonical user-facing description. |
| `crates/ark-cli/src/main.rs` (l. 290-330) | `UpgradeArgs` and the `policy()` method mapping `--force` / `--skip-modified` / `--create-new` to `ConflictPolicy`. |

### Code patterns

**The `load` decision tree** (`load.rs:73-93`):

```rust
pub fn load(opts: LoadOptions) -> Result<LoadSummary> {
    let layout = Layout::new(&opts.project_root);
    let ark_dir = layout.ark_dir();

    if ark_dir.exists() {
        if !opts.force {
            return Err(Error::AlreadyLoaded { path: ark_dir });
        }
        // --force: wipe the live footprint so either path below writes cleanly.
        layout.owned_dirs().iter().try_for_each(|d| d.remove_dir_all().map(|_| ()))?;
    }

    match Snapshot::read(layout.root())? {
        Some(snapshot) => restore(&layout, snapshot),
        None => fresh(&layout),
    }
}
```

Three states: already-loaded (error unless `--force`), snapshot-present (restore), snapshot-absent (scaffold fresh via `init`). The error message names the recovery flag: `"ark is already loaded at {path}; pass --force to replace it"` (`error.rs:43-47`).

**The `unload` capture+wipe sequence** (`unload.rs:68-142`):

```rust
pub fn unload(opts: UnloadOptions) -> Result<UnloadSummary> {
    ...
    // 1. Capture every file under Ark-owned directories.
    // 2. Capture + remove managed blocks.
    // 3. Capture Ark-owned hook entries.
    // 4. Persist the snapshot before destroying anything else.
    snapshot.write(layout.root())?;
    // 5. Delete the live Ark footprint.
    layout.owned_dirs().iter().try_for_each(|d| d.remove_dir_all().map(|_| ()))?;
    ...
}
```

The capture-before-delete sequence is explicit. Five numbered steps. The `.ark.db` write at step 4 is the durable barrier — if step 5 fails halfway, the snapshot still exists and `load` can re-create the live state.

**Hash recording at write time** (`io/fs.rs` via `manifest.record_file_with_hash`):

When `init` or `upgrade` writes a file, it stores `sha256(contents)` in `.ark/.installed.json`'s `hashes` map. On the next `upgrade`, the planner compares:

```
on_disk_hash == recorded_hash  ?  not user-modified  :  user-modified
```

This is the conflict-detection primitive. The `Manifest::record_file_with_hash` is called at exactly two sites in `init.rs` — the conflict pipeline and the post-write update. There's no third site that could write a file without recording a hash, which is why the source-scan invariant `init_source_no_bare_std_fs_or_dot_path_literals` is enforced as a test.

**The `upgrade` conflict pipeline** (`upgrade/mod.rs:259-385`):

```rust
pub fn upgrade(opts: UpgradeOptions, prompter: &mut dyn Prompter) -> Result<UpgradeSummary> {
    ...
    let plan = plan_actions(&layout, &manifest, &desired, opts.conflict_policy, prompter)?;
    ...
    for action in plan.actions {
        match action {
            PlannedAction::Write { relative, contents, kind } => { ... }
            PlannedAction::RefreshHashOnly { relative, contents } => { ... }
            PlannedAction::CreateNew { relative, contents } => {
                // write to <path>.new sidecar
            }
            PlannedAction::Preserve { .. } => {
                summary.modified_preserved += 1;
            }
            ...
        }
    }
    ...
}
```

Six action types: `Add`, `AutoUpdate`, `Overwrite`, `CreateNew`, `RefreshHashOnly`, `Preserve`. The `Delete` and `DropManifestEntry` actions are deferred until after the manifest is flushed — a careful ordering invariant so a failed delete can't leave the manifest inconsistent. Comment at l. 297-298: "apply_writes phase: Add, AutoUpdate, Overwrite, CreateNew, RefreshHashOnly, Preserve. Deletions are deferred until after the manifest is flushed."

**The `remove` per-platform cleanup** (`remove.rs:91-104`):

```rust
let mut per_platform = BTreeMap::new();
for platform in PLATFORMS {
    let hook_entry = platform.remove_hook(&layout)?;
    let dest_dir = platform.remove_dir(&layout)?;
    per_platform.insert(
        platform.id,
        RemovedPlatform { dest_dir, hook_entry },
    );
}
```

Hook removal precedes directory removal. The comment explains why (l. 88-92): "For platforms (e.g. Codex) whose hook file lives inside `removal_root`, running `remove_dir` first would delete the file the surgical `remove_hook` needs to act on — leaving any sibling user entries lost." This is a real correctness bug avoided by ordering.

**Round-trip preservation** is tested in `roundtrip_preserves_edited_and_added_claude_commands` (`load.rs:248-262`):

```rust
let custom = tmp.path().join(".claude/commands/ark/plan.md");
std::fs::write(&custom, "# user plan\n").unwrap();

unload(UnloadOptions::new(tmp.path())).unwrap();
load(LoadOptions::new(tmp.path())).unwrap();

assert_eq!(std::fs::read_to_string(&custom).unwrap(), "# user plan\n");
```

A user-authored file at a non-reserved stem inside an Ark-owned directory survives an unload/load round-trip byte-identical. This is the load-bearing guarantee — without it, Ark would be hostile to anyone who personalizes their slash commands.

**Reserved stems are NOT preserved** (`load.rs:510-528` for the agents case): A user-edited `ark-researcher.md` gets overwritten with the canonical body on load. The contract: stems beginning with `ark-` (the reserved prefix) are Ark-owned end-to-end; user-authored agents must use other names. This is the C-26 invariant called out in the source.

### External references

#### Why most CLIs don't have install/upgrade/uninstall verbs

The standard pattern in CLI design is to delegate lifecycle to the package manager. `cargo install ripgrep` works because Cargo is a package manager. `npm install -g some-tool` works because npm is. `pip install --user` works because pip is. The tool itself doesn't need to know how to uninstall — `cargo uninstall` does that.

Ark inherits this for the `ark` binary (`npm install -g @anekoique/ark`, `cargo install --git`). But the *project-side footprint* — the `.ark/` directory, the slash commands, the managed blocks — has no package manager. No tool except Ark itself knows what files were Ark-written. Hence the in-tool lifecycle.

The closest precedent in CLI history is the dotfile-management category: chezmoi, GNU Stow, yadm. Each ships its own lifecycle verbs because there's no upstream package manager for "the contents of your home directory."

#### Chezmoi's `apply` / `add` / `forget` / `remove`

Chezmoi distinguishes four kinds of lifecycle operation ([chezmoi *Daily operations*](https://www.chezmoi.io/user-guide/daily-operations/)):

| Verb | Effect |
| --- | --- |
| `chezmoi add ~/.zshrc` | Begin managing an existing file — copies destination state into source state. |
| `chezmoi apply` | Apply source state to destination, computing the diff. |
| `chezmoi diff` | Show what `apply` would do without doing it. |
| `chezmoi merge ~/.zshrc` | Three-way merge when destination drifted since last apply. |
| `chezmoi forget ~/.zshrc` | Stop managing without deleting the destination file. |
| `chezmoi remove ~/.zshrc` | Stop managing AND delete from destination. |

The `forget` vs `remove` distinction is interesting: chezmoi separates "leave the file on disk but I'm no longer managing it" from "delete the file." Ark conflates these — `ark remove` always deletes. The closest Ark analog to `forget` is hand-deleting `.ark/.installed.json` and the `.ark/` directory while keeping the slash commands as user-owned files. This is doable but not blessed.

#### GNU Stow's `stow` / `unstow` symmetry

Stow has two verbs:

| Verb | Effect |
| --- | --- |
| `stow zsh` | Create symlinks for every file in `~/dotfiles/zsh/` into `~`. |
| `stow -D zsh` | Remove all those symlinks; original files in `~/dotfiles/zsh/` untouched. |

Reversibility is the entire point of Stow. "The beauty of Stow's system for uninstallation lies in its reversibility. To uninstall, you simply need to remove the symbolic links that Stow created. The original files remain untouched in their dedicated Stow directory, ready to be re-linked if needed, or safely deleted later" ([Linux Vault, *How to Use GNU Stow*](https://www.thelinuxvault.net/blog/how-to-use-gnu-stow-to-manage-programs-installed-from-source-and-dotfiles/)).

The symlink mechanism makes this trivial. Ark can't use symlinks (agent platforms read literal files, not symlinks at known paths). The functional substitute is the snapshot — `.ark.db` plays the role Stow's source tree plays at rest.

#### Ansible's idempotent state management

Ansible's `apt`, `file`, `template`, `blockinfile` modules each define a *target state* and converge the system to it on every run. No explicit `unload` verb; Ansible playbooks are run-once-converge-state and you remove things by changing the desired state and re-running.

"Traditional Ansible patterns do not work directly on NixOS because modifying config files directly gets overwritten on rebuild and all system state is defined in /etc/nixos/configuration.nix" ([OneUptime, *How to Use Ansible to Configure NixOS*](https://oneuptime.com/blog/post/2026-02-21-ansible-configure-nixos/view)). The point: declarative tools (Nix, Ansible) avoid the install/upgrade/uninstall verb tax by making *every* invocation a converge-to-state operation. The cost is configuration that lives outside the project tree (in Ansible's case, in the playbook).

Ark's `init` is *not* fully declarative — it's an imperative scaffold. The closest declarative gesture is the idempotency test (`init_is_idempotent`): running `init` twice produces the same state, but only because Skip mode preserves on-disk content rather than re-converging to template content.

#### Nix home-manager

Home-manager uses a generation model: each `home-manager switch` creates a new "generation" of the user environment. Generations are versioned; you can roll back to a previous generation if a switch broke something.

"Note that in some cases Home Manager cannot detect whether it will overwrite previous manual configuration, for example, the Gnome Terminal module will write to dconf and cannot tell whether a configuration that is about to be overwritten was from a previous Home Manager generation or from manual configuration" ([Home Manager Manual](https://nix-community.github.io/home-manager/)). This is the same problem Ark's hash tracking solves — except home-manager doesn't fully solve it for opaque downstream tools.

Ark could conceptually adopt a generation model for `.ark.db` (multiple snapshots, rollback). Today it's one snapshot at a time.

#### `npm install` / `npm uninstall` / `npm update`

The package-manager pattern: npm tracks installed packages in `node_modules/` + `package.json` + `package-lock.json`. `npm install <pkg>` writes; `npm uninstall <pkg>` removes; `npm update <pkg>` upgrades. Conflict resolution lives in the lock file resolver, not in user prompts.

`npm` doesn't have an analog to Ark's "I have hand-modified the installed file" situation because user content doesn't live inside `node_modules/`. The package manager owns those directories outright. Ark cannot follow this model because Ark's owned directories (`.ark/`, `.claude/commands/ark/`) intentionally allow user-authored files alongside (the round-trip tests demand it).

#### Backup tools (restic, Time Machine) — the snapshot precedent

The closest precedent for `.ark.db` as a single-blob freeze is backup tooling: restic, borg, Time Machine. Each creates a snapshot at a point in time, lets you restore to that point. None of them deletes the live state after taking the backup — that's Ark's distinctive twist.

"Snapshot + delete" as a single operation is rare. The motivating use case is "I'm done with Ark for now but want to be able to come back to my tasks later" — chezmoi's `forget` and stow's `unstow` are the closest peers, but neither preserves user data the way `unload` does.

#### Reversibility as a UX principle — Don Norman territory

The classic *Design of Everyday Things* argument: "make actions reversible" is one of seven design principles. Software systems that get this right (Photoshop's history palette, Time Machine, undo logs in databases) feel safe to experiment with. Systems that don't (`rm -rf`, force-pushed git history, deleted Slack messages) breed paranoia.

Ark's lifecycle is unusually reversibility-forward for a CLI tool:

- `init` writes manifest hashes so `remove` can be precise.
- `unload` captures everything before deleting anything.
- `load --force` preserves the existing snapshot first.
- `upgrade --create-new` writes `.new` sidecars instead of overwriting.
- `remove` is the only irreversible verb, and even then it does not touch user files outside the manifest.

Compare: `npm uninstall` is reversible (just `npm install` again). `pip uninstall` is reversible (cached wheel). `cargo install --force` is non-destructive (replaces the binary at `~/.cargo/bin`). `aider`, `claude code`, `cline`, `continue.dev` — none of these have analogous reversibility surfaces because none of them write substantial on-disk footprints.

The Ark lifecycle is the *right* UX for an opinionated, structural-scaffold harness. The same shape would be over-engineering for a runtime-only tool.

### The four-verb model, mapped

```
       fresh project                  loaded project                  snapshot only
            |                              |                              |
            |  ark init                    |  ark unload                  |  ark load
            |  ark load                    |─────────────────────────────>|
            |─────────────────────────────>|                              |
            |                              |                              |
            |                              |  ark upgrade                 |
            |                              |  (refresh templates)         |
            |                              |◀────┐                        |
            |                              |─────┘                        |
            |                              |                              |
            |  ark remove                  |  ark remove                  |  ark remove
            |  (no-op)                     |  (wipes both)                |  (wipes db only)
            v                              v                              v
       fresh project                  fresh project                  fresh project
```

Three "interesting" states, four transitions, plus the universal eject. Every state is reachable from every other. The graph is small enough to fit in the user's head; large enough to support genuinely different workflows (e.g. unload-before-running-CI, load-when-resuming).

### What Ark gets uniquely right

- **Snapshot is a single file.** `.ark.db` is one JSON blob. Easy to inspect (it's JSON), easy to delete (it's one file), easy to ignore in git (one entry in `.gitignore`).
- **The capture-before-delete invariant is enforced by code structure** (`unload.rs:130`: `snapshot.write(layout.root())?;` precedes the destructive loop). No way to delete first by mistake.
- **The manifest is per-platform-additive.** `drop_manifest_entries_under` only drops the subset being re-written. `ark init --codex` after a Claude-only install doesn't forget Claude.
- **The upgrade conflict pipeline produces a structured `PlannedAction` list.** Tested independently of side effects. Failure of one action doesn't corrupt the manifest.
- **The Prompter trait separates the library from stdio.** `crates/ark-cli/src/main.rs:667-687` implements `StdioPrompter`; tests can inject `PanicPrompter` or `StubPrompter`. Clean.
- **Reserved stems are explicitly callout.** `crates/ark-core/src/commands/load.rs:118-124` re-overwrites canonical agent bodies even when the snapshot captured user-edited bytes. The C-26 invariant is documented inline.

### What's awkward or under-served

- **No `forget` verb.** A user who wants to keep their `.ark/tasks/` directory but stop having Ark manage `CLAUDE.md` has no clean path. They'd have to hand-edit `.ark/.installed.json` to remove the managed-block entry — undocumented surgery.
- **No generation history.** `.ark.db` is one snapshot. Re-running `unload` overwrites the previous snapshot. Users who want "let me roll back to last Tuesday's task list" must use git, not Ark.
- **`ark upgrade` is silent about what the new template version actually changed.** The summary line shows `0.1.0 -> 0.2.0` and counters, but doesn't surface "the workflow file now mentions research tier" or "the PRD template added a SPEC Path block." Release notes live in the GitHub release; Ark doesn't pull them in.
- **No `ark status` to show "what state is Ark in right now."** A user discovers state by `ls .ark` and `cat .ark/.installed.json`. `ark context` is the closest verb, but it's about workflow state (tasks, phases) not lifecycle state (loaded? unloaded? upgrade pending?).
- **The lifecycle verbs are not very discoverable from `ark --help`.** They're listed, but a first-time user has no idea why they'd ever run `unload`. The motivating story ("freeze your tasks during a vacation, restore later") lives nowhere in the doc surface beyond a sentence in `AGENTS.md`.
- **`--allow-downgrade` is the only safety hatch for version mismatch.** The current logic refuses an older CLI on a newer project. There's no "the templates would change in incompatible ways" warning before the upgrade — only after.

### Caveats / Not found

- No data on `unload` adoption — whether users actually freeze their state or just leave Ark loaded indefinitely. Field telemetry would inform whether the verb earns its complexity.
- The exact `restic` / `borg` snapshot patterns were not surveyed in detail; the comparison rests on conceptual similarity rather than implementation parallel.
- Whether the `forget` / `remove` split chezmoi uses would be valuable for Ark users is speculative — depends on whether users actually want to "stop managing without deleting."
- No external sources observed for an "agent harness lifecycle" pattern explicitly; this is unusual territory.

## Directions for Ark

1. **Add `ark status` as a lifecycle introspection verb.** Output:
   ```
   ark 0.3.2 (project: 0.3.1; upgrade available)
   state: loaded
   manifest: .ark/.installed.json (32 files, 28 hashed, 2 managed blocks)
   snapshot: none (last unload: never)
   focus: agent-harness-infra (research / research)
   ```
   Mirrors `git status` in shape. Separates lifecycle introspection from `ark context`'s workflow introspection — both are useful, both should be cheap to invoke.

2. **Surface upgrade changelog inline.** Ship the release notes as a `templates/upgrade-notes/<version>.md` set; `ark upgrade` prints the diff between `version_from` and `version_to` notes. Today the summary counts files but tells the user nothing about what they're upgrading *into*. mdBook's edit-url-template implies the release notes already exist in `docs/`; pulling them in to the CLI output closes the gap.

3. **Add `ark unload --dry-run` and `ark remove --dry-run`.** Both verbs are destructive (one reversibly, one not). A preview mode that shows exactly what would land in `.ark.db` (or be deleted) helps cautious operators. Implementation cost is low — both already iterate the owned-dirs walk and the manifest.

4. **Distinguish `ark remove` from `ark forget`.** `ark forget --platform claude` would delete just the platform-specific files, leave `.ark/` intact. `ark forget --keep-tasks` would remove templates / workflow / specs but keep `tasks/`. The chezmoi `forget` vs `remove` split documents the user-intent gap that exists today only as "edit `.installed.json` and good luck."

5. **Track multiple unload generations.** `.ark.db.YYYYMMDD-HHMM` files instead of a single `.ark.db`. `ark load` picks the latest; `ark load --from <timestamp>` picks an earlier snapshot. The snapshot file is small (JSON, base64'd content); ten generations is ~tens of KB worst case. Single-file lifecycle becomes generational; rollback joins the existing reversibility story.
