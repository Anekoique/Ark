//! `ark agent task commit` — atomic task closure.
//!
//! `task_commit` is the single Ark-side mutation that closes a task. It
//! lands user-staged work + Ark-managed closure artifacts in **one** git
//! commit:
//!
//! 1. Verify preconditions (phase, staged work, VERIFY checklist).
//! 2. (Deep tier) Promote the latest PLAN's `## Spec` section to a feature
//!    SPEC and register the row in the features INDEX. (Snapshots taken
//!    into `RollbackGuard` first.)
//! 3. Save `task.toml` with `phase = Committed` and `committed_at = now`.
//! 4. Stage Ark-managed files (task.toml + (deep) SPEC + features INDEX)
//!    alongside whatever the user already staged.
//! 5. `git commit -m "<msg>"` — the closing commit captures user work +
//!    task.toml + (deep) SPEC + features INDEX.
//! 6. Read the closing commit's SHA for the summary returned to the caller.
//!
//! ## Rollback
//!
//! [`RollbackGuard`] tracks every snapshot taken before each mutation
//! plus the number of commits that landed (zero or one). On `Drop`, the
//! guard:
//!
//! 1. If a commit landed, runs `git reset --soft HEAD~1` to undo it
//!    (preserving staged content for inspection).
//! 2. Restores `task.toml` and (deep tier) SPEC + features INDEX bytes.
//! 3. Runs `git reset HEAD <ark_files>` to unstage only the files Ark
//!    added — the user's pre-existing index entries are preserved.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use chrono::Utc;

use crate::{
    commands::agent::{
        spec::{
            extract::{SpecExtractOptions, spec_extract},
            register::{SpecRegisterOptions, spec_register},
        },
        state::{Phase, TaskToml, Tier, validate_slug},
    },
    error::{Error, Result},
    io::{PathExt, git::run_git},
    layout::Layout,
    state::state_mutate,
};

/// Options accepted by [`task_commit`].
#[derive(Debug, Clone)]
pub struct TaskCommitOptions {
    /// Project root containing the Ark installation.
    pub project_root: PathBuf,
    /// Slug of the task being closed.
    pub slug: String,
    /// Commit message. Required when `no_commit == false`.
    pub message: Option<String>,
    /// Skip `git commit` and journal write; deep tier still extracts SPEC
    /// and the phase still flips to `Committed`. The user takes
    /// responsibility for any follow-up commit + journal record.
    pub no_commit: bool,
    /// Stage every tracked + untracked change (`git add -A`) before the
    /// staged-work gate. Folds the user's manual `git add` into the closure.
    /// Mutually exclusive with `no_commit` (nothing to stage for).
    pub all: bool,
}

/// Summary returned by [`task_commit`] on success.
#[derive(Debug, Clone)]
pub struct TaskCommitSummary {
    /// Slug of the task that was closed.
    pub slug: String,
    /// Workflow tier of the closed task.
    pub tier: Tier,
    /// HEAD SHA after the closing commit, or `None` under `--no-commit`.
    /// Display-only; not persisted in `task.toml`.
    pub head_sha: Option<String>,
    /// True when deep-tier SPEC extraction ran.
    pub deep_spec_promoted: bool,
    /// VERIFY pending counts surfaced as a warning on standard tier.
    pub pending_verify: VerifyPendingCounts,
}

/// VERIFY checklist + findings residue counted by `parse_verify_md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VerifyPendingCounts {
    /// Count of `- [ ] ...: PENDING` checklist items.
    pub items: u32,
    /// Count of findings whose `Resolution: PENDING` line is present.
    pub findings: u32,
}

impl VerifyPendingCounts {
    /// Returns `true` when neither items nor findings remain pending.
    fn is_clean(self) -> bool {
        self.items == 0 && self.findings == 0
    }
}

impl fmt::Display for TaskCommitSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "committed `{}` ({:?})", self.slug, self.tier)?;
        if let Some(sha) = &self.head_sha {
            write!(f, " at {sha}")?;
        }
        if self.deep_spec_promoted {
            write!(f, " [SPEC promoted]")?;
        }
        if !self.pending_verify.is_clean() {
            write!(
                f,
                " [VERIFY warnings: {} item(s), {} finding(s)]",
                self.pending_verify.items, self.pending_verify.findings
            )?;
        }
        Ok(())
    }
}

/// Closes the task: VERIFY gate, deep-tier SPEC promotion, atomic commit of
/// work + journal + task.toml + (deep) SPEC + features INDEX.
pub fn task_commit(opts: TaskCommitOptions) -> Result<TaskCommitSummary> {
    validate_slug(&opts.slug)?;

    let layout = Layout::new(&opts.project_root);
    let task_dir = layout.task_dir(&opts.slug);
    if !task_dir.exists() {
        return Err(Error::TaskNotFound { slug: opts.slug });
    }

    let prev_toml = TaskToml::load(&task_dir)?;
    check_phase_for_commit(prev_toml.tier, prev_toml.phase)?;

    let task_cwd = task_cwd_for(&prev_toml, &layout);

    if !opts.no_commit {
        if opts.all {
            stage_all(&task_cwd)?;
        }
        require_staged_work(&task_cwd, &opts.slug)?;
    }
    if opts.no_commit {
        // Non-deep tiers under `--no-commit` only record the phase
        // transition; deep tiers additionally extract the SPEC.
        eprintln!(
            "--no-commit on {:?} tier: phase transition recorded; user owns the follow-up commit",
            prev_toml.tier
        );
    }

    let pending = if matches!(prev_toml.tier, Tier::Standard | Tier::Deep) {
        let verify_path = task_dir.join("VERIFY.md");
        let counts = parse_verify_md(&verify_path)?;
        if prev_toml.tier == Tier::Deep && !counts.is_clean() {
            return Err(Error::VerifyIncomplete {
                path: verify_path,
                items: counts.items,
                findings: counts.findings,
            });
        }
        if prev_toml.tier == Tier::Standard && !counts.is_clean() {
            eprintln!(
                "warn: VERIFY.md has {} pending item(s) and {} pending finding(s); proceeding on \
                 standard tier",
                counts.items, counts.findings
            );
        }
        counts
    } else {
        VerifyPendingCounts::default()
    };

    // RollbackGuard arms here. Every subsequent destructive step records a
    // snapshot before mutating; an `Err` between this point and
    // `guard.commit()` triggers the guard's `Drop` which restores everything,
    // including soft-resetting any landed commit.
    let mut guard = RollbackGuard::new(&task_cwd);

    let deep = prev_toml.tier == Tier::Deep;
    // Deep tier requires a `[**SPEC Path**]` block in the latest PRD; quick
    // and standard tiers default to the legacy single-segment `[slug]` path.
    let prd_path = task_dir.join("PRD.md");
    let feature_segments: Vec<String> = if deep {
        let prd_text = prd_path.read_text()?;
        crate::commands::agent::task::prd::parse_spec_path(
            &prd_text, &opts.slug, &prd_path, &layout,
        )?
    } else {
        vec![opts.slug.clone()]
    };
    let seg_refs: Vec<&str> = feature_segments.iter().map(String::as_str).collect();
    let spec_dir = layout.specs_feature_dir(&seg_refs, &prd_path)?;
    let spec_path = spec_dir.join("SPEC.md");
    let intermediate_indexes = intermediate_index_paths(&layout, &feature_segments);

    if deep {
        guard.snapshot_spec(&spec_path)?;
        guard.snapshot_features_indexes(&intermediate_indexes)?;
        let now = Utc::now();
        spec_extract(SpecExtractOptions {
            project_root: opts.project_root.clone(),
            slug: opts.slug.clone(),
            feature_path: feature_segments.clone(),
            plan_override: None,
            task_dir_override: Some(task_dir.clone()),
        })?;
        spec_register(SpecRegisterOptions {
            project_root: opts.project_root.clone(),
            feature_path: feature_segments.clone(),
            scope: prev_toml.title.clone(),
            from_task: opts.slug.clone(),
            date: now.date_naive(),
        })?;
    }

    // Workspace journal write. Skipped under `--no-commit` and when no
    // identity is set (the agent should have appended its session block to
    // the active journal already; `workspace_record` finds it there and
    // stamps the auto-fields).
    let workspace_journal_path: Option<String> = if !opts.no_commit {
        record_workspace_journal(&opts, &prev_toml, &task_cwd, &mut guard)?
    } else {
        None
    };

    let now = Utc::now();
    guard.snapshot_toml(prev_toml.clone());
    let mut next_toml = prev_toml.clone();
    next_toml.phase = Phase::Committed;
    next_toml.committed_at = Some(now);
    next_toml.updated_at = now;
    next_toml.journal_path = workspace_journal_path;
    next_toml.save(&task_dir)?;

    if opts.no_commit {
        // Phase + (deep) SPEC are persisted; user owns the follow-up commit.
        guard.commit();
        clear_focus_if_matches(&layout, &opts.slug)?;
        return Ok(TaskCommitSummary {
            slug: opts.slug,
            tier: prev_toml.tier,
            head_sha: None,
            deep_spec_promoted: deep,
            pending_verify: pending,
        });
    }

    let message = opts.message.clone().ok_or(Error::CommitMessageRequired {
        slug: opts.slug.clone(),
    })?;

    // Stage Ark-managed files: task.toml; plus (deep tier) the promoted
    // SPEC and every INDEX along the leaf-to-root path; plus (research
    // tier) every file under `<task_dir>/research/`; plus any workspace
    // paths that `record_workspace_journal` populated. The user's work is
    // already staged.
    let ark_files = ark_files_for_first_commit(
        &task_dir,
        prev_toml.tier,
        &spec_path,
        &intermediate_indexes,
        &guard.workspace_paths.clone(),
    );
    guard.record_staged(ark_files.clone());
    if !stage_files(&task_cwd, &ark_files)? {
        return Err(Error::GitCommitFailed {
            path: task_cwd.clone(),
            stderr: "git add of Ark-managed files failed".into(),
        });
    }
    let commit_out = run_git(&["commit", "-m", &message], &task_cwd)?;
    if !commit_out.is_success() {
        return Err(Error::GitCommitFailed {
            path: task_cwd.clone(),
            stderr: commit_out.stderr.trim().to_string(),
        });
    }
    guard.note_commit_landed();

    let head_sha = run_git(&["rev-parse", "HEAD"], &task_cwd)
        .ok()
        .filter(|o| o.is_success())
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());

    guard.commit();
    clear_focus_if_matches(&layout, &opts.slug)?;

    Ok(TaskCommitSummary {
        slug: opts.slug,
        tier: prev_toml.tier,
        head_sha,
        deep_spec_promoted: deep,
        pending_verify: pending,
    })
}

/// Clears `state.focus` iff it points at `slug`. Leaves `tasks.active` alone:
/// commit promotes the task to `Committed` but it remains active until
/// `ark archive` runs.
fn clear_focus_if_matches(layout: &Layout, slug: &str) -> Result<()> {
    state_mutate(layout, |state| {
        if state.focus.as_deref() == Some(slug) {
            state.focus = None;
        }
        Ok(())
    })
}

/// Returns `Ok` iff `(tier, phase)` is a legal `task_commit` precondition.
fn check_phase_for_commit(tier: Tier, phase: Phase) -> Result<()> {
    // `Tier::Research` and `Phase::Research` share the bare name `Research`;
    // glob imports would shadow one with the other, so qualify both here.
    let ok = matches!(
        (tier, phase),
        (Tier::Quick, Phase::Execute)
            | (Tier::Standard, Phase::Verify)
            | (Tier::Deep, Phase::Verify)
            | (Tier::Research, Phase::Research)
    );
    if ok {
        Ok(())
    } else {
        Err(Error::IllegalPhaseTransition {
            tier,
            from: phase,
            to: Phase::Committed,
        })
    }
}

/// Resolves the working directory `task_commit` operates against.
///
/// Three cases:
///
/// - No worktree binding: run from the project root.
/// - Worktree binding, joined path exists: run from the worktree (the user
///   invoked ark from the parent checkout).
/// - Worktree binding, joined path missing: the user invoked ark *inside*
///   the worktree, where `task.toml.worktree_path` (project-relative to the
///   parent) does not resolve. Fall back to `layout.root()` — that is
///   already the worktree's root.
fn task_cwd_for(toml: &TaskToml, layout: &Layout) -> PathBuf {
    let Some(rel) = toml.worktree_path.as_ref() else {
        return layout.root().to_path_buf();
    };
    let joined = layout.root().join(rel);
    if joined.is_dir() {
        joined
    } else {
        layout.root().to_path_buf()
    }
}

/// Stages every tracked + untracked change in `cwd` via `git add -A`.
///
/// Backs the `--all` flag: folds the user's manual `git add` into the
/// closure. The subsequent `require_staged_work` gate still applies, so a
/// genuinely empty working tree continues to surface `NothingStaged`.
fn stage_all(cwd: &Path) -> Result<()> {
    let out = run_git(&["add", "-A"], cwd)?;
    if out.is_success() {
        Ok(())
    } else {
        Err(Error::GitCommitFailed {
            path: cwd.to_path_buf(),
            stderr: out.stderr.trim().to_string(),
        })
    }
}

/// Returns `Err(NothingStaged)` when `git diff --cached --quiet` reports a
/// clean stage.
fn require_staged_work(cwd: &Path, slug: &str) -> Result<()> {
    let out = run_git(&["diff", "--cached", "--quiet"], cwd)?;
    if out.is_success() {
        return Err(Error::NothingStaged {
            slug: slug.to_string(),
        });
    }
    Ok(())
}

/// Stages the listed Ark-managed files via `git add`.
///
/// Returns `Ok(false)` when `git add` reports a non-zero exit so the caller
/// can wrap the failure into a `GitCommitFailed` carrying useful context.
fn stage_files(cwd: &Path, files: &[PathBuf]) -> Result<bool> {
    let mut args: Vec<&str> = vec!["add", "--"];
    let strings: Vec<String> = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    for s in &strings {
        args.push(s.as_str());
    }
    let out = run_git(&args, cwd)?;
    Ok(out.is_success())
}

/// Computes the Ark-managed files staged for the **first** commit of the
/// commit-then-amend protocol.
///
/// The list is exactly:
///   - this task's `task.toml`;
///   - (deep tier) the promoted SPEC and the features INDEX;
///   - (research tier) every file under `<task_dir>/research/` recursively.
///
/// The journal file and workspace index are deliberately absent here — they
/// are appended after the first commit lands and staged for the amend step
/// inline by `task_commit`. User work files are intentionally absent in
/// both lists: the user staged them via `git add` before invoking
/// `/ark:commit`.
fn ark_files_for_first_commit(
    task_dir: &Path,
    tier: Tier,
    spec_path: &Path,
    intermediate_indexes: &[PathBuf],
    workspace_files: &[PathBuf],
) -> Vec<PathBuf> {
    let mut files = Vec::with_capacity(2 + intermediate_indexes.len() + workspace_files.len());
    files.push(task_dir.join("task.toml"));
    if matches!(tier, Tier::Deep) {
        files.push(spec_path.to_path_buf());
        for p in intermediate_indexes {
            files.push(p.clone());
        }
    }
    if matches!(tier, Tier::Research) {
        files.extend(research_corpus_files(task_dir));
    }
    files.extend(workspace_files.iter().cloned());
    files
}

/// Lists every file under `<task_dir>/research/` recursively.
///
/// Empty vector when the directory does not exist (research tier may legally
/// commit with no corpus — the PRD alone is the artifact). Walks via
/// [`crate::io::fs::walk_files`], so order is unspecified; sort callers if
/// they need determinism.
fn research_corpus_files(task_dir: &Path) -> Vec<PathBuf> {
    let research_dir = task_dir.join("research");
    crate::io::fs::walk_files(&research_dir).unwrap_or_default()
}

/// Returns every INDEX path that the leaf-to-root upsert may touch, ordered
/// leaf-to-root.
///
/// For segments `[a, b, c]`:
///   - leaf level: `.ark/specs/features/a/b/INDEX.md` (carries the `c` leaf row)
///   - mid level:  `.ark/specs/features/a/INDEX.md`   (carries the `b/INDEX.md` row)
///   - root level: `.ark/specs/features/INDEX.md`     (carries the `a/INDEX.md` row)
///
/// For a single-segment path `[c]`, only the root `.ark/specs/features/INDEX.md`
/// is returned so the legacy flat layout is preserved bit-for-bit.
pub fn intermediate_index_paths(layout: &Layout, segments: &[String]) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(segments.len());
    for i in (0..segments.len()).rev() {
        let mut p = layout.specs_features_dir();
        for seg in &segments[..i] {
            p.push(seg);
        }
        p.push("INDEX.md");
        paths.push(p);
    }
    paths
}

/// Parses a VERIFY.md document and counts unresolved checklist items + findings.
///
/// A line is a *pending checklist item* when it matches `^- \[ \] .*: PENDING\s*$`.
/// A *pending finding* is any `Resolution: PENDING` line (case-sensitive,
/// trimmed). A missing VERIFY.md file is treated as zero pending — the
/// caller's phase-precondition check is responsible for catching that case
/// before this function runs.
fn parse_verify_md(path: &Path) -> Result<VerifyPendingCounts> {
    let text = match path.read_text_optional()? {
        Some(t) => t,
        None => return Ok(VerifyPendingCounts::default()),
    };
    let mut counts = VerifyPendingCounts::default();
    for raw in text.lines() {
        let line = raw.trim_end();
        if is_pending_checklist_item(line) {
            counts.items += 1;
        } else if is_pending_finding_resolution(line) {
            counts.findings += 1;
        }
    }
    Ok(counts)
}

/// Predicate for pending checklist items: `- [ ] ... : PENDING`.
fn is_pending_checklist_item(line: &str) -> bool {
    let line = line.trim_start();
    let Some(rest) = line.strip_prefix("- [ ]") else {
        return false;
    };
    rest.trim_end().ends_with(": PENDING") || rest.trim_end().ends_with(":PENDING")
}

/// Predicate for pending finding resolutions: `Resolution: PENDING`.
fn is_pending_finding_resolution(line: &str) -> bool {
    let line = line.trim_start();
    line.strip_prefix("- ")
        .or(Some(line))
        .map(|stripped| {
            stripped.starts_with("**Resolution:** PENDING")
                || stripped.starts_with("Resolution: PENDING")
                || stripped.starts_with("- Resolution: PENDING")
        })
        .unwrap_or(false)
}

/// Calls `workspace_record` to stamp auto-fields on the agent's
/// pre-written journal entry, then tracks the touched files in the guard.
///
/// Returns `Ok(None)` when identity is unset (silent skip — fresh installs
/// and CI environments without a `.ark/.developer`). Returns `Err` when
/// the agent failed to append a session heading to the journal.
fn record_workspace_journal(
    opts: &TaskCommitOptions,
    prev_toml: &TaskToml,
    task_cwd: &Path,
    guard: &mut RollbackGuard,
) -> Result<Option<String>> {
    use crate::commands::agent::workspace::{
        RecordMode, RecordOptions, identity::ResolveOptions, identity_resolve, workspace_record,
    };

    if identity_resolve(ResolveOptions::new(&opts.project_root)).is_err() {
        return Ok(None);
    }

    // Non-worktree tasks have `branch = None`; fall back to the current
    // checkout's branch so the journal renders the real name, not `HEAD`.
    let branch_owned = prev_toml
        .branch
        .clone()
        .or_else(|| current_branch(task_cwd));
    let branch = branch_owned.as_deref().unwrap_or("HEAD");
    let base_branch = prev_toml.base_branch.as_deref().unwrap_or("main");
    let start_head_short = prev_toml
        .start_head
        .as_deref()
        .map(|s| s.chars().take(7).collect::<String>())
        .unwrap_or_else(|| "<unknown>".into());
    let commits_in_range = collect_commits_in_range(task_cwd, prev_toml.start_head.as_deref());

    let mode = RecordMode::Task {
        slug: &opts.slug,
        branch,
        base_branch,
        start_head_short: &start_head_short,
        commits_in_range: &commits_in_range,
    };
    let summary = workspace_record(RecordOptions::new(&opts.project_root, mode))?;

    let journal_relative = summary.journal_path_relative().to_string();
    let journal_path = summary.journal_path().to_path_buf();
    let dev_dir_index = journal_path.parent().map(|p| p.join("index.md"));
    let workspace_index = Layout::new(&opts.project_root).workspace_index();

    guard.adopt_record_snapshot(summary.into_snapshot());
    if let Some(idx) = dev_dir_index {
        guard.add_workspace_path(idx);
    }
    guard.add_workspace_path(workspace_index);
    guard.add_workspace_path(journal_path);

    Ok(Some(journal_relative))
}

/// Collects `git log <start_head>..HEAD --oneline` as `(short_sha, subject)`
/// pairs.
///
/// Falls back to `git log -n 20` for pre-refactor tasks (no `start_head`).
fn collect_commits_in_range(task_cwd: &Path, start_head: Option<&str>) -> Vec<(String, String)> {
    let args = match start_head {
        Some(sh) => vec![
            "log".to_string(),
            format!("{sh}..HEAD"),
            "--format=%h %s".to_string(),
        ],
        None => vec![
            "log".to_string(),
            "-n".to_string(),
            "20".to_string(),
            "--format=%h %s".to_string(),
        ],
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = match run_git(&arg_refs, task_cwd) {
        Ok(o) if o.is_success() => o,
        _ => return Vec::new(),
    };
    out.stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let sha = parts.next()?.to_string();
            let subject = parts.next().unwrap_or("").to_string();
            Some((sha, subject))
        })
        .collect()
}

/// Returns the current branch name in `cwd`, or `None` on detached HEAD or
/// non-git directories.
fn current_branch(cwd: &Path) -> Option<String> {
    let out = run_git(&["rev-parse", "--abbrev-ref", "HEAD"], cwd).ok()?;
    if !out.is_success() {
        return None;
    }
    let name = out.stdout.trim().to_string();
    if name.is_empty() || name == "HEAD" {
        None
    } else {
        Some(name)
    }
}

// -- RollbackGuard --------------------------------------------------------

/// Scoped rollback helper for [`task_commit`].
///
/// Snapshots accumulate as destructive mutations succeed; on `Drop` (any
/// error path before [`commit`](Self::commit) is called), every accumulated
/// snapshot is restored. On the success path, `commit()` disarms the guard
/// so its drop becomes a no-op.
///
/// `commits_landed` is 0 before `git commit` runs and 1 after. On rollback
/// the guard issues `git reset --soft HEAD~1` to undo the commit while
/// keeping the staged content for inspection, then unstages the
/// Ark-managed files (preserving the user's pre-existing index entries).
struct RollbackGuard {
    armed: bool,
    task_cwd: PathBuf,
    prev_toml: Option<TaskToml>,
    spec_file: Option<SpecFileSnapshot>,
    /// Snapshots for every INDEX that may be mutated during the leaf-to-root
    /// upsert. `prev_bytes: None` means the INDEX did not exist pre-mutation,
    /// so rollback unlinks it. Restore iterates in reverse insertion order.
    features_indexes: Vec<FeaturesIndexSnapshot>,
    /// Adopted workspace snapshots, replayed in reverse adoption order on
    /// rollback. Each one's `rollback` does suffix-check truncate +
    /// `write_atomic` restore of the personal and top-level indices.
    record_snapshots: Vec<crate::commands::agent::workspace::RecordSnapshot>,
    /// Workspace paths to stage alongside `ark_files`.
    workspace_paths: Vec<PathBuf>,
    ark_files: Vec<PathBuf>,
    commits_landed: u32,
}

struct SpecFileSnapshot {
    path: PathBuf,
    /// `None` means the file was absent before extract.
    prev_bytes: Option<Vec<u8>>,
}

struct FeaturesIndexSnapshot {
    path: PathBuf,
    /// `None` means the INDEX did not exist pre-mutation; rollback unlinks it.
    prev_bytes: Option<Vec<u8>>,
}

impl RollbackGuard {
    fn new(task_cwd: &Path) -> Self {
        Self {
            armed: true,
            task_cwd: task_cwd.to_path_buf(),
            prev_toml: None,
            spec_file: None,
            features_indexes: Vec::new(),
            record_snapshots: Vec::new(),
            workspace_paths: Vec::new(),
            ark_files: Vec::new(),
            commits_landed: 0,
        }
    }

    /// Adopts a successful `workspace_record`'s snapshot. Snapshots are
    /// drained in reverse order on rollback, restoring journals and
    /// indices in the inverse order they were written.
    fn adopt_record_snapshot(
        &mut self,
        snapshot: crate::commands::agent::workspace::RecordSnapshot,
    ) {
        self.record_snapshots.push(snapshot);
    }

    /// Tracks a workspace path that should be staged alongside the
    /// task.toml + (deep) SPEC + features-INDEX in the closing commit, and
    /// unstaged on rollback.
    fn add_workspace_path(&mut self, path: PathBuf) {
        self.workspace_paths.push(path);
    }

    /// Records that the closing commit landed. Used by `Drop` to decide
    /// whether to issue `git reset --soft HEAD~1` during rollback.
    fn note_commit_landed(&mut self) {
        self.commits_landed = 1;
    }

    fn snapshot_toml(&mut self, toml: TaskToml) {
        self.prev_toml = Some(toml);
    }

    fn snapshot_spec(&mut self, spec_path: &Path) -> Result<()> {
        let prev_bytes = if spec_path.exists() {
            Some(spec_path.read_bytes()?)
        } else {
            None
        };
        self.spec_file = Some(SpecFileSnapshot {
            path: spec_path.to_path_buf(),
            prev_bytes,
        });
        Ok(())
    }

    /// Snapshots one INDEX path; pushes onto `features_indexes` (multiple
    /// snapshots accumulate; restore is reverse insertion order).
    fn snapshot_features_index(&mut self, path: &Path) -> Result<()> {
        let prev_bytes = if path.exists() {
            Some(path.read_bytes()?)
        } else {
            None
        };
        self.features_indexes.push(FeaturesIndexSnapshot {
            path: path.to_path_buf(),
            prev_bytes,
        });
        Ok(())
    }

    /// Snapshots every INDEX along the leaf-to-root path before any
    /// mutation. Called once per `task_commit` after `parse_spec_path`
    /// produces the segments.
    fn snapshot_features_indexes(&mut self, paths: &[PathBuf]) -> Result<()> {
        for p in paths {
            self.snapshot_features_index(p)?;
        }
        Ok(())
    }

    fn record_staged(&mut self, ark_files: Vec<PathBuf>) {
        self.ark_files = ark_files;
    }

    fn commit(mut self) {
        self.armed = false;
    }

    fn restore(&mut self) {
        // Order:
        //   1. soft-reset any landed commit so HEAD points at the parent.
        //   2. targeted unstage of Ark-managed files only — preserves the
        //      user's pre-existing index entries.
        //   3. workspace snapshot restore (suffix-check truncate journal,
        //      restore indices in reverse adoption order).
        //   4. SPEC file restore (deep tier).
        //   5. features INDEX restore (deep tier).
        //   6. task.toml restore.
        for _ in 0..self.commits_landed {
            if let Ok(o) = run_git(&["reset", "--soft", "HEAD~1"], &self.task_cwd)
                && !o.is_success()
            {
                eprintln!(
                    "rollback: git reset --soft HEAD~1 failed: {}",
                    o.stderr.trim()
                );
            }
        }

        if !self.ark_files.is_empty() {
            let mut args: Vec<&str> = vec!["reset", "HEAD", "--"];
            let strings: Vec<String> = self
                .ark_files
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            for s in &strings {
                args.push(s.as_str());
            }
            let _ = run_git(&args, &self.task_cwd);
        }

        // Workspace rollback runs in reverse adoption order. Drain the Vec;
        // each snapshot's rollback consumes the snapshot.
        for snapshot in std::mem::take(&mut self.record_snapshots).into_iter().rev() {
            if let Err(e) = snapshot.rollback() {
                eprintln!("rollback: workspace snapshot restore failed: {e}");
            }
        }

        if let Some(snap) = &self.spec_file {
            match &snap.prev_bytes {
                Some(b) => {
                    if let Err(e) = snap.path.write_bytes(b) {
                        eprintln!("rollback: SPEC restore at {:?} failed: {e}", snap.path);
                    }
                }
                None => {
                    let _ = std::fs::remove_file(&snap.path);
                }
            }
        }

        for snap in self.features_indexes.iter().rev() {
            match &snap.prev_bytes {
                Some(bytes) => {
                    if let Err(e) = snap.path.write_bytes(bytes) {
                        eprintln!(
                            "rollback: features INDEX restore at {:?} failed: {e}",
                            snap.path
                        );
                    }
                }
                None => {
                    let _ = std::fs::remove_file(&snap.path);
                }
            }
        }

        if let Some(toml) = &self.prev_toml {
            let task_dir = self.task_cwd.join(".ark").join("tasks").join(&toml.id);
            if let Err(e) = toml.save(&task_dir) {
                eprintln!("rollback: task.toml restore at {:?} failed: {e}", task_dir);
            }
        }
    }
}

impl Drop for RollbackGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::state::Tier;

    /// Verifies the precondition table: only `(Quick, Execute)`,
    /// `(Standard, Verify)`, `(Deep, Verify)`, and `(Research, Research)`
    /// are accepted as commit inputs.
    #[test]
    fn check_phase_for_commit_accepts_only_legal_inputs() {
        assert!(check_phase_for_commit(Tier::Quick, Phase::Execute).is_ok());
        assert!(check_phase_for_commit(Tier::Standard, Phase::Verify).is_ok());
        assert!(check_phase_for_commit(Tier::Deep, Phase::Verify).is_ok());
        assert!(check_phase_for_commit(Tier::Research, Phase::Research).is_ok());

        for (tier, phase) in [
            (Tier::Quick, Phase::Design),
            (Tier::Quick, Phase::Verify),
            (Tier::Standard, Phase::Execute),
            (Tier::Standard, Phase::Plan),
            (Tier::Deep, Phase::Plan),
            (Tier::Deep, Phase::Review),
            (Tier::Deep, Phase::Execute),
            // Research tier rejects every phase except Phase::Research.
            (Tier::Research, Phase::Design),
            (Tier::Research, Phase::Plan),
            (Tier::Research, Phase::Verify),
            (Tier::Research, Phase::Execute),
        ] {
            assert!(
                check_phase_for_commit(tier, phase).is_err(),
                "{tier:?} {phase:?} must be rejected as a commit input"
            );
        }
    }

    /// Verifies that the parser counts `- [ ] ...: PENDING` checklist items.
    #[test]
    fn parse_verify_counts_pending_checklist_items() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("VERIFY.md");
        p.write_bytes(
            b"# VERIFY\n## Project Spec Compliance\n- [ ] LAYOUT.md: PENDING\n- [x] STYLE.md: \
              PASS\n- [ ] ERRORS.md: PENDING\n",
        )
        .unwrap();
        let counts = parse_verify_md(&p).unwrap();
        assert_eq!(counts.items, 2);
        assert_eq!(counts.findings, 0);
    }

    /// Verifies that `Resolution: PENDING` lines under findings are counted.
    #[test]
    fn parse_verify_counts_pending_findings() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("VERIFY.md");
        p.write_bytes(
            "# VERIFY\n## Findings\n### V-001 — title\n- Severity: HIGH\n- Resolution: \
             PENDING\n### V-002 — title2\n- Resolution: FIXED in abc123\n"
                .as_bytes(),
        )
        .unwrap();
        let counts = parse_verify_md(&p).unwrap();
        assert_eq!(counts.items, 0);
        assert_eq!(counts.findings, 1);
    }

    /// Verifies that an absent VERIFY.md returns zero pending (callers gate
    /// the existence check upstream via the phase precondition).
    #[test]
    fn parse_verify_missing_file_is_zero_pending() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("does-not-exist.md");
        let counts = parse_verify_md(&p).unwrap();
        assert_eq!(counts, VerifyPendingCounts::default());
    }

    /// Verifies that `is_pending_checklist_item` rejects an `[x]` line.
    #[test]
    fn pending_predicate_rejects_resolved_items() {
        assert!(!is_pending_checklist_item("- [x] LAYOUT.md: PASS"));
        assert!(!is_pending_checklist_item("- [ ] LAYOUT.md: PASS"));
        assert!(is_pending_checklist_item("- [ ] LAYOUT.md: PENDING"));
    }

    /// Verifies that `ark_files_for_first_commit` excludes user-work files.
    ///
    /// The list must be exactly the Ark-managed files: never the working
    /// tree's `src/foo.rs` etc. Quick/standard tier returns just `task.toml`;
    /// deep tier additionally includes the promoted SPEC and the features
    /// INDEX. Research tier includes the corpus walk (covered by a separate
    /// test below). Workspace paths (when present) append last.
    #[test]
    fn ark_files_for_first_commit_excludes_user_work() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let task_dir = layout.task_dir("foo");
        let prd = task_dir.join("PRD.md");
        let spec_path = layout
            .specs_feature_dir(&["foo"], &prd)
            .unwrap()
            .join("SPEC.md");
        let features_index = vec![layout.specs_features_index()];
        let no_workspace: &[PathBuf] = &[];

        let standard = ark_files_for_first_commit(
            &task_dir,
            Tier::Standard,
            &spec_path,
            &features_index,
            no_workspace,
        );
        assert_eq!(standard.len(), 1);
        assert!(standard[0].ends_with("task.toml"));

        let deep = ark_files_for_first_commit(
            &task_dir,
            Tier::Deep,
            &spec_path,
            &features_index,
            no_workspace,
        );
        assert_eq!(deep.len(), 3);
        assert!(deep.iter().all(|p| p.to_string_lossy().contains(".ark/")));

        let with_ws = ark_files_for_first_commit(
            &task_dir,
            Tier::Deep,
            &spec_path,
            &features_index,
            &[PathBuf::from(".ark/workspace/alice/journal-1.md")],
        );
        assert_eq!(with_ws.len(), 4);
    }

    /// Walks `<task_dir>/research/` recursively (including nested subdirs)
    /// and returns empty when the directory is absent. Confirms
    /// `ark_files_for_first_commit` on the research tier includes
    /// `task.toml` plus every corpus file and excludes SPEC artifacts.
    #[test]
    fn research_corpus_files_walks_recursively() {
        use crate::io::PathExt;

        let tmp = tempfile::tempdir().unwrap();
        let task_dir = tmp.path().join("task");
        task_dir.ensure_dir().unwrap();
        let layout = Layout::new(tmp.path());
        let prd = task_dir.join("PRD.md");
        let spec_path = layout
            .specs_feature_dir(&["foo"], &prd)
            .unwrap()
            .join("SPEC.md");
        let features_index = vec![layout.specs_features_index()];

        // Absent research/ → empty corpus, ark_files = [task.toml] only.
        assert!(research_corpus_files(&task_dir).is_empty());
        let research_no_corpus =
            ark_files_for_first_commit(&task_dir, Tier::Research, &spec_path, &features_index, &[]);
        assert_eq!(research_no_corpus.len(), 1);
        assert!(research_no_corpus[0].ends_with("task.toml"));

        // Populate research/ with a top-level + nested file.
        let research = task_dir.join("research");
        research.ensure_dir().unwrap();
        research.join("topic-a.md").write_bytes(b"a").unwrap();
        let nested = research.join("subdir");
        nested.ensure_dir().unwrap();
        nested.join("topic-b.md").write_bytes(b"b").unwrap();

        let corpus = research_corpus_files(&task_dir);
        assert_eq!(corpus.len(), 2);
        assert!(
            corpus.iter().any(|p| p.ends_with("topic-a.md")),
            "{corpus:?}"
        );
        assert!(
            corpus.iter().any(|p| p.ends_with("topic-b.md")),
            "{corpus:?}"
        );

        // ark_files_for_first_commit threads the corpus through.
        let research_files =
            ark_files_for_first_commit(&task_dir, Tier::Research, &spec_path, &features_index, &[]);
        assert_eq!(research_files.len(), 3);
        assert!(research_files[0].ends_with("task.toml"));
        assert!(
            research_files
                .iter()
                .filter(|p| p.to_string_lossy().contains("/research/"))
                .count()
                == 2,
            "{research_files:?}"
        );

        // SPEC and features INDEX must NOT be staged on research tier.
        assert!(
            !research_files
                .iter()
                .any(|p| p.to_string_lossy().contains("specs/features/"))
        );
    }

    /// `intermediate_index_paths` returns leaf-to-root order.
    #[test]
    fn intermediate_index_paths_leaf_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let segs = vec!["xemu".to_string(), "csr".to_string()];
        let paths = intermediate_index_paths(&layout, &segs);
        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("specs/features/xemu/INDEX.md"));
        assert!(paths[1].ends_with("specs/features/INDEX.md"));
    }

    /// Single-segment path → only root INDEX (preserves the legacy flat layout).
    #[test]
    fn intermediate_index_paths_single_segment_is_root_only() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let segs = vec!["klib".to_string()];
        let paths = intermediate_index_paths(&layout, &segs);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("specs/features/INDEX.md"));
    }

    /// Three-segment path → three INDEXes, leaf to root.
    #[test]
    fn intermediate_index_paths_three_segments() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let segs = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let paths = intermediate_index_paths(&layout, &segs);
        assert_eq!(paths.len(), 3);
        assert!(paths[0].ends_with("specs/features/a/b/INDEX.md"));
        assert!(paths[1].ends_with("specs/features/a/INDEX.md"));
        assert!(paths[2].ends_with("specs/features/INDEX.md"));
    }
}

// -- End-to-end git tests -------------------------------------------------

#[cfg(test)]
mod e2e {
    //! End-to-end tests that drive `task_commit` against a real
    //! `git init` tempdir.
    //!
    //! These cover the post-refactor invariants:
    //! - Closing commit contains all Ark-managed files (task.toml +
    //!   (deep) SPEC + features INDEX) + the user's staged work.
    //! - No Ark-managed file appears in `git status` post-commit.
    //! - User's pre-existing index entries survive a rollback.

    use super::*;
    use crate::{
        InitOptions,
        commands::agent::{
            state::Tier,
            task::{
                new::{TaskNewOptions, task_new},
                phase::{TaskPhaseOptions, task_execute, task_plan, task_review, task_verify},
            },
        },
        init::init,
        io::{PathExt, git::run_git},
    };

    /// Initializes a git repo and scaffolds Ark.
    fn init_repo_with_ark() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();
        tmp.path()
            .join("README.md")
            .write_bytes(b"# repo\n")
            .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "init", "--quiet"], tmp.path()).unwrap();
        init(InitOptions::new(tmp.path())).unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "ark scaffold", "--quiet"], tmp.path()).unwrap();
        tmp
    }

    /// Drives a quick task to `Phase::Execute` with task scaffolding committed
    /// and a user-work file staged for the closing commit.
    fn quick_at_execute_with_staged_work(
        tmp: &std::path::Path,
        slug: &str,
        work_path: &str,
        work_body: &[u8],
    ) {
        task_new(TaskNewOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
            title: format!("{slug} task"),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        run_git(&["add", "."], tmp).unwrap();
        run_git(
            &[
                "commit",
                "-m",
                &format!("chore({slug}): scaffold"),
                "--quiet",
            ],
            tmp,
        )
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
        })
        .unwrap();
        if let Some(parent) = std::path::Path::new(work_path).parent()
            && !parent.as_os_str().is_empty()
        {
            tmp.join(parent).ensure_dir().unwrap();
        }
        tmp.join(work_path).write_bytes(work_body).unwrap();
        run_git(&["add", work_path], tmp).unwrap();
    }

    /// Returns Ark-managed paths that show in `git status --porcelain`.
    fn ark_managed_dirty(tmp: &std::path::Path) -> Vec<String> {
        let out = run_git(&["status", "--porcelain"], tmp).unwrap();
        out.stdout
            .lines()
            .filter_map(|l| {
                let path = l.get(3..).unwrap_or("").trim();
                let is_ark = path.starts_with(".ark/")
                    || path.contains("/.ark/")
                    || path.contains("specs/features/");
                is_ark.then(|| path.to_string())
            })
            .collect()
    }

    /// Quick-tier closure: closing commit captures task.toml + user work;
    /// nothing Ark-managed is dirty afterwards.
    #[test]
    fn quick_tier_commit_is_atomic_and_clean() {
        let tmp = init_repo_with_ark();
        quick_at_execute_with_staged_work(tmp.path(), "qd", "src/foo.rs", b"pub fn foo() {}\n");

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat(qd): user work".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
        assert_eq!(summary.tier, Tier::Quick);
        assert!(summary.head_sha.is_some());
        assert!(!summary.deep_spec_promoted);

        let dirty = ark_managed_dirty(tmp.path());
        assert!(
            dirty.is_empty(),
            "no Ark-managed file should be dirty post-commit, got {dirty:?}"
        );

        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains(".ark/tasks/qd/task.toml"));
        assert!(head_files.contains("src/foo.rs"));

        // Focus is released on commit so the next non-targeted verb errors
        // cleanly instead of resolving to a closed task.
        let state = crate::state::load_state(&Layout::new(tmp.path())).unwrap();
        assert!(state.focus.is_none(), "task_commit must clear focus");
        // Slug stays in active until `ark archive` runs.
        assert!(state.tasks.active.iter().any(|s| s == "qd"));
    }

    /// Unstaged user files outside Ark's purview survive the closing commit.
    #[test]
    fn commit_does_not_touch_unstaged_user_files() {
        let tmp = init_repo_with_ark();
        quick_at_execute_with_staged_work(tmp.path(), "qd", "src/foo.rs", b"pub fn foo() {}\n");
        tmp.path()
            .join("notes.txt")
            .write_bytes("draft notes - do not commit yet\n".as_bytes())
            .unwrap();

        task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat(qd): foo".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();

        let status = run_git(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(
            status.stdout.contains("notes.txt"),
            "user's unstaged file must survive untouched: {}",
            status.stdout
        );
    }

    /// `--all` stages unstaged work before the gate: a quick task with an
    /// unstaged work file commits cleanly and the file lands in HEAD without a
    /// manual `git add`.
    #[test]
    fn all_flag_stages_unstaged_work() {
        let tmp = init_repo_with_ark();
        // Drive to Execute and scaffold a task without staging any work.
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            title: "qd task".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(
            &["commit", "-m", "chore(qd): scaffold", "--quiet"],
            tmp.path(),
        )
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();

        // Unstaged user work — no `git add` here.
        tmp.path().join("src").ensure_dir().unwrap();
        tmp.path()
            .join("src/foo.rs")
            .write_bytes(b"pub fn foo() {}\n")
            .unwrap();

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat(qd): user work".into()),
            no_commit: false,
            all: true,
        })
        .unwrap();
        assert!(summary.head_sha.is_some());

        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains("src/foo.rs"), "all=true must stage it");
        assert!(head_files.contains(".ark/tasks/qd/task.toml"));

        let dirty = ark_managed_dirty(tmp.path());
        assert!(
            dirty.is_empty(),
            "nothing Ark-managed dirty post-commit: {dirty:?}"
        );
    }

    /// `--all` on a genuinely empty working tree still surfaces `NothingStaged`
    /// (the gate runs after `git add -A`, which stages nothing).
    #[test]
    fn all_flag_on_clean_tree_still_errors_nothing_staged() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(
            &["commit", "-m", "chore(qd): scaffold", "--quiet"],
            tmp.path(),
        )
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        // task_execute rewrote task.toml (Design -> Execute); commit it so the
        // working tree is genuinely clean before the stage-all gate.
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(
            &["commit", "-m", "chore(qd): execute", "--quiet"],
            tmp.path(),
        )
        .unwrap();
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat: x".into()),
            no_commit: false,
            all: true,
        })
        .unwrap_err();
        assert!(matches!(err, Error::NothingStaged { .. }));
    }

    /// `task_commit` rejects when nothing is staged.
    #[test]
    fn commit_errors_when_nothing_staged() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat: x".into()),
            no_commit: false,
            all: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::NothingStaged { .. }));
    }

    /// `task_commit` without `-m` and without `--no-commit` errors with
    /// `CommitMessageRequired`.
    #[test]
    fn commit_errors_when_message_missing() {
        let tmp = init_repo_with_ark();
        quick_at_execute_with_staged_work(tmp.path(), "demo", "src/foo.rs", b"x\n");
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: None,
            no_commit: false,
            all: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::CommitMessageRequired { .. }));
    }

    /// `--no-commit` skips the git commit but still flips phase to
    /// `Committed`; head_sha is `None`.
    #[test]
    fn no_commit_flips_phase_without_committing() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            title: "qd".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
        })
        .unwrap();
        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: None,
            no_commit: true,
            all: false,
        })
        .unwrap();
        assert!(summary.head_sha.is_none());

        let layout = Layout::new(tmp.path());
        let toml = TaskToml::load(&layout.task_dir("qd")).unwrap();
        assert_eq!(toml.phase, Phase::Committed);
        assert!(toml.committed_at.is_some());

        // --no-commit also releases focus: the phase transition is what
        // matters; the user owns the follow-up commit but no longer needs
        // focus bound.
        let state = crate::state::load_state(&layout).unwrap();
        assert!(state.focus.is_none(), "--no-commit must also clear focus");
    }

    /// Pre-commit hook rejects → rollback restores `task.toml`; the user's
    /// pre-existing staged file (outside Ark's purview) survives the targeted
    /// unstage.
    #[test]
    fn rollback_on_pre_commit_hook_failure_restores_everything() {
        let tmp = init_repo_with_ark();
        let hooks = tmp.path().join(".git/hooks");
        hooks.ensure_dir().unwrap();
        let hook_path = hooks.join("pre-commit");
        hook_path.write_bytes(b"#!/bin/sh\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hook_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_path, perms).unwrap();
        }

        tmp.path()
            .join("user_intent.txt")
            .write_bytes(b"my staged work\n")
            .unwrap();
        run_git(&["add", "user_intent.txt"], tmp.path()).unwrap();

        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            title: "demo".into(),
            tier: Tier::Quick,
            worktree: None,
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
        })
        .unwrap();

        let layout = Layout::new(tmp.path());
        let pre_toml = TaskToml::load(&layout.task_dir("demo")).unwrap();

        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: Some("feat: x".into()),
            no_commit: false,
            all: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::GitCommitFailed { .. }));

        let post_toml = TaskToml::load(&layout.task_dir("demo")).unwrap();
        assert_eq!(post_toml.phase, pre_toml.phase, "phase rolled back");
        assert!(post_toml.committed_at.is_none(), "committed_at not set");
        let cached = run_git(&["diff", "--cached", "--name-only"], tmp.path())
            .unwrap()
            .stdout;
        assert!(
            cached.contains("user_intent.txt"),
            "user's pre-staged file must survive: cached={cached:?}"
        );
    }

    /// Standard tier requires `Verify`, not `Execute`.
    #[test]
    fn standard_tier_commit_from_execute_errors() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
            title: "s".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
        })
        .unwrap();
        tmp.path().join("foo.rs").write_bytes(b"x").unwrap();
        run_git(&["add", "foo.rs"], tmp.path()).unwrap();
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
            message: Some("m".into()),
            no_commit: false,
            all: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    /// Standard-tier full happy path: design → plan → execute → verify →
    /// stage → commit. Phase ends at `Committed`; user work is in HEAD.
    #[test]
    fn standard_tier_commit_happy_path() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
            title: "standard demo".into(),
            tier: Tier::Standard,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
        })
        .unwrap();

        tmp.path()
            .join("src/foo.rs")
            .write_bytes(b"pub fn foo() {}\n")
            .unwrap();
        run_git(&["add", "src/foo.rs"], tmp.path()).unwrap();

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
            message: Some("feat(std): work".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
        assert!(summary.head_sha.is_some());
        let layout = Layout::new(tmp.path());
        let toml = TaskToml::load(&layout.task_dir("std")).unwrap();
        assert_eq!(toml.phase, Phase::Committed);
    }

    /// Deep-tier full happy path: design → plan → review → execute → verify
    /// → stage → commit. Closing commit contains the promoted SPEC + features
    /// INDEX + task.toml + user work.
    #[test]
    fn deep_tier_commit_promotes_spec_into_closing_commit() {
        let tmp = init_repo_with_ark();
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
            title: "deep demo".into(),
            tier: Tier::Deep,
            worktree: None,
        })
        .unwrap();
        // Add a `[**SPEC Path**]` block to the seeded PRD; required on deep
        // tier for `task_commit` to know where to land the promoted SPEC.
        tmp.path()
            .join(".ark/tasks/deep/PRD.md")
            .write_bytes(b"[**SPEC Path**]\n\ndeep\n")
            .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
        })
        .unwrap();
        tmp.path()
            .join(".ark/tasks/deep/PLAN.md")
            .write_bytes(b"# plan\n## Spec\n\n[**Goals**]\n- G-1: v1\n\n## Runtime\nrt\n")
            .unwrap();
        task_review(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
        })
        .unwrap();
        task_execute(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
        })
        .unwrap();
        task_verify(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
        })
        .unwrap();

        // The seeded VERIFY.md has PENDING items by design; the implementer
        // would resolve each before commit. Replace with an all-PASS body so
        // the deep gate accepts.
        let layout = Layout::new(tmp.path());
        layout
            .task_dir("deep")
            .join("VERIFY.md")
            .write_bytes(b"# VERIFY\n## resolved\n- [x] all: PASS\n")
            .unwrap();

        tmp.path().join("src").ensure_dir().unwrap();
        tmp.path()
            .join("src/deep.rs")
            .write_bytes(b"pub fn deep() {}\n")
            .unwrap();
        run_git(&["add", "src/deep.rs"], tmp.path()).unwrap();

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
            message: Some("feat(deep): land it".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
        assert!(summary.deep_spec_promoted);

        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains(".ark/tasks/deep/task.toml"));
        assert!(head_files.contains(".ark/specs/features/deep/SPEC.md"));
        assert!(head_files.contains(".ark/specs/features/INDEX.md"));
    }

    /// Drives a research task to a committable state with optional corpus
    /// files staged.
    ///
    /// Creates the task at `(Tier::Research, Phase::Research)`, commits the
    /// scaffold so subsequent operations have a clean parent, then writes
    /// the supplied `corpus_files` under `<task>/research/` and updates the
    /// PRD body (so an empty-corpus case still has *something* staged for
    /// `task_commit`'s `NothingStaged` precondition).
    fn research_at_research_with_staged_corpus(
        tmp: &std::path::Path,
        slug: &str,
        corpus_files: &[(&str, &[u8])],
    ) {
        task_new(TaskNewOptions {
            project_root: tmp.to_path_buf(),
            slug: slug.into(),
            title: format!("{slug} research"),
            tier: Tier::Research,
            worktree: None,
        })
        .unwrap();
        run_git(&["add", "."], tmp).unwrap();
        run_git(
            &[
                "commit",
                "-m",
                &format!("chore({slug}): scaffold"),
                "--quiet",
            ],
            tmp,
        )
        .unwrap();

        let task_dir = tmp.join(format!(".ark/tasks/{slug}"));

        // Mutate the PRD body so the user-visible "PRD got filled" step shows
        // up as a staged change. Otherwise the empty-corpus case has nothing
        // dirty and `task_commit` correctly errors with NothingStaged.
        let prd_path = task_dir.join("PRD.md");
        let prd = prd_path.read_text().unwrap_or_default();
        prd_path
            .write_bytes(format!("{prd}\n[Test PRD body for {slug}]\n").as_bytes())
            .unwrap();
        run_git(&["add", &format!(".ark/tasks/{slug}/PRD.md")], tmp).unwrap();

        if !corpus_files.is_empty() {
            let research_dir = task_dir.join("research");
            research_dir.ensure_dir().unwrap();
            for (rel, body) in corpus_files {
                let full = research_dir.join(rel);
                if let Some(parent) = full.parent() {
                    parent.ensure_dir().unwrap();
                }
                full.write_bytes(body).unwrap();
            }
            run_git(&["add", &format!(".ark/tasks/{slug}/research")], tmp).unwrap();
        }
    }

    /// Research-tier closure: closing commit captures task.toml + corpus;
    /// focus is released; no SPEC promoted.
    #[test]
    fn research_tier_commit_stages_corpus_and_clears_focus() {
        let tmp = init_repo_with_ark();
        research_at_research_with_staged_corpus(
            tmp.path(),
            "rs",
            &[("topic-a.md", b"alpha\n"), ("subdir/topic-b.md", b"beta\n")],
        );

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "rs".into(),
            message: Some("research(rs): corpus".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
        assert_eq!(summary.tier, Tier::Research);
        assert!(summary.head_sha.is_some());
        assert!(!summary.deep_spec_promoted, "research never promotes SPEC");

        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains(".ark/tasks/rs/task.toml"));
        assert!(head_files.contains(".ark/tasks/rs/research/topic-a.md"));
        assert!(head_files.contains(".ark/tasks/rs/research/subdir/topic-b.md"));
        // No SPEC promotion: features/<slug>/SPEC.md must not appear.
        assert!(
            !head_files.contains(".ark/specs/features/rs/SPEC.md"),
            "research tier must not promote a SPEC: {head_files}"
        );

        let state = crate::state::load_state(&Layout::new(tmp.path())).unwrap();
        assert!(state.focus.is_none(), "task_commit must clear focus");
        assert!(state.tasks.active.iter().any(|s| s == "rs"));

        let toml = TaskToml::load(&Layout::new(tmp.path()).task_dir("rs")).unwrap();
        assert_eq!(toml.tier, Tier::Research);
        assert_eq!(toml.phase, Phase::Committed);
        assert!(toml.committed_at.is_some());
    }

    /// Research tier never reads VERIFY.md and therefore commits cleanly
    /// even when no VERIFY.md is on disk.
    #[test]
    fn research_tier_skips_verify_gate() {
        let tmp = init_repo_with_ark();
        research_at_research_with_staged_corpus(tmp.path(), "rs", &[("only.md", b"x\n")]);

        // Explicit assertion: there is no VERIFY.md anywhere in the task dir.
        let task_dir = tmp.path().join(".ark/tasks/rs");
        assert!(!task_dir.join("VERIFY.md").exists());

        task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "rs".into(),
            message: Some("research(rs): commit without VERIFY".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
    }

    /// Research tier with no `research/` directory at all commits cleanly
    /// when only PRD is staged: the corpus is optional.
    #[test]
    fn research_tier_commit_succeeds_with_empty_corpus() {
        let tmp = init_repo_with_ark();
        research_at_research_with_staged_corpus(tmp.path(), "rs", &[]);
        // PRD is staged by the helper; no research/ subdir exists.
        assert!(!tmp.path().join(".ark/tasks/rs/research").exists());

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "rs".into(),
            message: Some("research(rs): empty corpus".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();
        assert!(summary.head_sha.is_some());
    }

    /// Research-tier task never produces a feature SPEC. After commit, the
    /// SPEC path is empty AND the features INDEX has no row for the slug.
    #[test]
    fn research_tier_commit_does_not_promote_spec() {
        let tmp = init_repo_with_ark();
        research_at_research_with_staged_corpus(tmp.path(), "rs", &[("notes.md", b"corpus\n")]);

        task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "rs".into(),
            message: Some("research(rs): no spec".into()),
            no_commit: false,
            all: false,
        })
        .unwrap();

        // No SPEC file landed.
        let spec_path = tmp.path().join(".ark/specs/features/rs/SPEC.md");
        assert!(!spec_path.exists(), "research must not promote a SPEC");

        // Features INDEX should not gain a row for the research slug.
        let features_index = tmp
            .path()
            .join(".ark/specs/features/INDEX.md")
            .read_text()
            .unwrap_or_default();
        assert!(
            !features_index.contains("rs/SPEC.md") && !features_index.contains("rs/INDEX.md"),
            "research slug must not appear in features INDEX: {features_index}",
        );
    }
}
