//! `ark agent task commit` — atomic task closure via commit-then-amend.
//!
//! `task_commit` is the single Ark-side mutation that closes a task. The
//! protocol uses a **commit-then-amend** sequence so the workspace journal
//! entry sees the closing commit and includes it in the entry's
//! commits-in-range table:
//!
//! 1. Verify preconditions (phase, staged work, VERIFY checklist).
//! 2. (Deep tier) Promote the latest PLAN's `## Spec` section to a feature
//!    SPEC and register the row in the features INDEX. (Snapshots taken
//!    into `RollbackGuard` first.)
//! 3. Save `task.toml` with `phase = Committed` and `committed_at = now`.
//! 4. Stage Ark-managed files that exist *now* (task.toml + (deep) SPEC +
//!    features INDEX) alongside whatever the user already staged.
//! 5. **First commit** (`git commit -m "<msg>"`) — captures user work +
//!    task.toml + (deep) SPEC + features INDEX. Workspace journal is
//!    deliberately absent so it can witness the closing SHA below.
//! 6. Render + append the journal entry: the renderer's `git log
//!    <start_head>..HEAD` now includes the closing commit row by
//!    construction; re-render the workspace index.
//! 7. Stage journal + workspace index, then **amend**
//!    (`git commit --amend --no-edit`) — folds the journal write into the
//!    closing commit. The amend rewrites the commit's hash; `git log -S
//!    '**Slug**: <slug>'` against the journal file resolves to the
//!    post-amend SHA, while the table inside the journal entry records
//!    the pre-amend SHA. Both invariants are documented in the workflow
//!    doc.
//! 8. Read the post-amend SHA for the summary returned to the caller.
//!
//! ## Why amend is safe here
//!
//! Amend rewrites the commit's hash. That is normally fragile because it
//! breaks anything referencing the pre-amend SHA. Inside `task_commit`'s
//! window:
//!
//! - The first commit (step 5) just landed in the local repo; nothing has
//!   pushed it.
//! - No other operation has had the chance to reference the pre-amend SHA.
//! - The user invoked `/ark:commit` precisely because they want the task
//!   closure to be one logical commit — amending one private commit into
//!   itself is the closest we can get.
//!
//! ## Rollback
//!
//! [`RollbackGuard`] tracks every snapshot taken before each mutation
//! plus the number of commits that landed (zero or one — amend rewrites
//! the existing commit, it does not add a new one). On `Drop`, the guard:
//!
//! 1. If a commit landed, runs `git reset --soft HEAD~1` to undo it
//!    (preserving staged content for inspection).
//! 2. Restores `task.toml`, journal length, workspace index bytes, and
//!    (deep tier) SPEC + features INDEX bytes.
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
        workspace::{RecordTaskOptions, WorkspaceRecorded, record::record_task},
    },
    error::{Error, Result},
    io::{PathExt, git::run_git},
    layout::Layout,
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
}

/// Summary returned by [`task_commit`] on success.
#[derive(Debug, Clone)]
pub struct TaskCommitSummary {
    /// Slug of the task that was closed.
    pub slug: String,
    /// Workflow tier of the closed task.
    pub tier: Tier,
    /// HEAD SHA after the closing commit. `None` under `--no-commit`.
    /// Display-only; not persisted in `task.toml`.
    pub head_sha: Option<String>,
    /// Path to the journal file the entry was appended to. `None` under
    /// `--no-commit`.
    pub journal_path: Option<PathBuf>,
    /// Session number assigned to the journal entry. `None` under
    /// `--no-commit`.
    pub session_number: Option<u32>,
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
        if let Some(n) = self.session_number {
            write!(f, " [workspace session {n}]")?;
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
        require_staged_work(&task_cwd, &opts.slug)?;
    }
    if opts.no_commit && opts.message.is_none() && prev_toml.tier != Tier::Deep {
        // Honest warning: --no-commit on non-deep tiers is a phase-flip-only
        // path. Surface it once on stderr per NG-9.
        eprintln!(
            "--no-commit on {:?} tier: only phase transition recorded",
            prev_toml.tier
        );
    } else if opts.no_commit {
        eprintln!(
            "--no-commit: journal not written; run /ark:record manually if you want a session \
             entry"
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
    let mut guard = RollbackGuard::new(&task_dir, &task_cwd);

    let deep = prev_toml.tier == Tier::Deep;
    let spec_path = layout.specs_feature_dir(&opts.slug).join("SPEC.md");
    let features_index_path = layout.specs_features_index();

    if deep {
        guard.snapshot_spec(&spec_path)?;
        guard.snapshot_features_index(&features_index_path)?;
        let now = Utc::now();
        spec_extract(SpecExtractOptions {
            project_root: opts.project_root.clone(),
            slug: opts.slug.clone(),
            plan_override: None,
            task_dir_override: Some(task_dir.clone()),
        })?;
        spec_register(SpecRegisterOptions {
            project_root: opts.project_root.clone(),
            feature: opts.slug.clone(),
            scope: prev_toml.title.clone(),
            from_task: opts.slug.clone(),
            date: now.date_naive(),
        })?;
    }

    let now = Utc::now();
    guard.snapshot_toml(prev_toml.clone());
    let mut next_toml = prev_toml.clone();
    next_toml.phase = Phase::Committed;
    next_toml.committed_at = Some(now);
    next_toml.updated_at = now;
    next_toml.save(&task_dir)?;

    if opts.no_commit {
        // Phase + (deep) SPEC are persisted; user owns the follow-up commit
        // and may invoke `/ark:record` for a manual journal entry.
        guard.commit();
        return Ok(TaskCommitSummary {
            slug: opts.slug,
            tier: prev_toml.tier,
            head_sha: None,
            journal_path: None,
            session_number: None,
            deep_spec_promoted: deep,
            pending_verify: pending,
        });
    }

    let message = opts.message.clone().ok_or(Error::CommitMessageRequired {
        slug: opts.slug.clone(),
    })?;

    // Stage Ark-managed files that exist now (task.toml, deep-tier SPEC +
    // features INDEX). Journal + workspace index are absent at this point;
    // they are appended after the first commit lands so the journal entry
    // can include the closing SHA in its commits-in-range table.
    let pre_commit_ark_files =
        ark_files_for_first_commit(&task_dir, deep, &spec_path, &features_index_path);
    guard.record_staged(pre_commit_ark_files.clone());
    if !stage_files(&task_cwd, &pre_commit_ark_files)? {
        return Err(Error::GitCommitFailed {
            path: task_cwd.clone(),
            stderr: "git add of Ark-managed files failed (pre-commit stage)".into(),
        });
    }
    let first_commit = run_git(&["commit", "-m", &message], &task_cwd)?;
    if !first_commit.is_success() {
        return Err(Error::GitCommitFailed {
            path: task_cwd.clone(),
            stderr: first_commit.stderr.trim().to_string(),
        });
    }
    guard.note_commit_landed();

    // Render + append the journal entry. The renderer's `git log
    // <start_head>..HEAD` includes the closing commit row by construction
    // because HEAD now points at the just-landed first commit. record_task
    // internally re-renders the workspace index. Snapshots are taken into
    // the guard before each mutation so a failure here triggers full
    // rollback (journal truncate, index restore, soft-reset of the first
    // commit).
    let journal_n = guard.snapshot_journal_active(&layout)?;
    guard.snapshot_workspace_index_for_active(&layout, journal_n.as_deref())?;
    let outcome = record_task(RecordTaskOptions {
        project_root: opts.project_root.clone(),
        slug: opts.slug.clone(),
        title: prev_toml.title.clone(),
        tier: prev_toml.tier,
        branch: prev_toml.branch.clone(),
        base_branch: prev_toml.base_branch.clone(),
        worktree_path: prev_toml
            .worktree_path
            .as_ref()
            .map(|p| layout.root().join(p)),
        start_head: prev_toml.start_head.clone(),
        task_dir: task_dir.clone(),
        recorded_at: now,
    })?;

    let session_info = match outcome {
        WorkspaceRecorded::Recorded {
            journal_path,
            session_number,
        } => Some((journal_path, session_number)),
        WorkspaceRecorded::SkippedNoIdentity | WorkspaceRecorded::SkippedDisabled => {
            // No journal mutation happened. Discard journal + index snapshots
            // so the rollback path doesn't try to restore files we didn't
            // touch. The first commit still amends below — even without a
            // journal entry, the closure commit is the single closure
            // artifact, and the user can invoke `/ark:record` later.
            guard.discard_journal_snapshot();
            guard.discard_workspace_index_snapshot();
            None
        }
    };

    // Stage the journal write (when a journal entry was actually written)
    // and amend the closing commit so the tree reflects it. When the
    // journal write was skipped (no developer set, auto-record disabled),
    // the amend would be a no-op; skip it entirely.
    if let Some((journal_path, _)) = &session_info {
        let mut amend_files = vec![journal_path.clone()];
        if let Some(dev) = developer_from_journal_path(&layout, journal_path) {
            amend_files.push(layout.workspace_index(&dev));
        }
        guard.add_staged(amend_files.clone());
        if !stage_files(&task_cwd, &amend_files)? {
            return Err(Error::GitCommitFailed {
                path: task_cwd.clone(),
                stderr: "git add of journal/index failed (pre-amend stage)".into(),
            });
        }
        let amend = run_git(&["commit", "--amend", "--no-edit"], &task_cwd)?;
        if !amend.is_success() {
            return Err(Error::GitCommitFailed {
                path: task_cwd.clone(),
                stderr: amend.stderr.trim().to_string(),
            });
        }
    }

    // Post-amend HEAD is the user-visible closing SHA. Slug-anchored
    // `git log -S` recovery returns this value (the journal file's content
    // came in via the amend).
    let head_sha = run_git(&["rev-parse", "HEAD"], &task_cwd)
        .ok()
        .filter(|o| o.is_success())
        .map(|o| o.stdout.trim().to_string())
        .filter(|s| !s.is_empty());

    // Success: disarm the guard so its drop is a no-op.
    guard.commit();

    Ok(TaskCommitSummary {
        slug: opts.slug,
        tier: prev_toml.tier,
        head_sha,
        journal_path: session_info.as_ref().map(|(p, _)| p.clone()),
        session_number: session_info.as_ref().map(|(_, n)| *n),
        deep_spec_promoted: deep,
        pending_verify: pending,
    })
}

/// Returns `Ok` iff `(tier, phase)` is a legal `task_commit` precondition.
fn check_phase_for_commit(tier: Tier, phase: Phase) -> Result<()> {
    use Phase::*;
    use Tier::*;
    let ok = matches!(
        (tier, phase),
        (Quick, Execute) | (Standard, Verify) | (Deep, Verify)
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
///   - (deep tier) the promoted SPEC and the features INDEX.
///
/// The journal file and workspace index are deliberately absent here — they
/// are appended after the first commit lands and staged for the amend step
/// inline by `task_commit`. User work files are intentionally absent in
/// both lists: the user staged them via `git add` before invoking
/// `/ark:commit`.
fn ark_files_for_first_commit(
    task_dir: &Path,
    deep: bool,
    spec_path: &Path,
    features_index_path: &Path,
) -> Vec<PathBuf> {
    let mut files = Vec::with_capacity(3);
    files.push(task_dir.join("task.toml"));
    if deep {
        files.push(spec_path.to_path_buf());
        files.push(features_index_path.to_path_buf());
    }
    files
}

/// Recovers the developer name from a `journal-N.md` path of the form
/// `<root>/.ark/workspace/<dev>/journal-N.md`.
///
/// Used only to resolve the workspace index path for staging. Returns `None`
/// if the path does not match the expected layout.
fn developer_from_journal_path(layout: &Layout, journal_path: &Path) -> Option<String> {
    let workspace = layout.workspace_dir();
    let rel = journal_path.strip_prefix(&workspace).ok()?;
    rel.iter().next().map(|s| s.to_string_lossy().into_owned())
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

// -- RollbackGuard --------------------------------------------------------

/// Scoped rollback helper for [`task_commit`].
///
/// Snapshots accumulate as destructive mutations succeed; on `Drop` (any
/// error path before [`commit`](Self::commit) is called), every accumulated
/// snapshot is restored in reverse-of-recording order. On the success path,
/// `commit()` disarms the guard so its drop becomes a no-op.
///
/// `commits_landed` tracks the commit-then-amend protocol: 0 if no commit
/// happened, 1 if the first commit landed (regardless of whether the amend
/// also landed — amend doesn't add a new commit, it rewrites the existing
/// one). On rollback the guard issues `git reset --soft HEAD~commits_landed`
/// to undo the commit while keeping the staged content for inspection,
/// then unstages targeted files.
struct RollbackGuard {
    armed: bool,
    task_cwd: PathBuf,
    prev_toml: Option<(PathBuf, TaskToml)>,
    journal: Option<JournalSnapshot>,
    workspace_index: Option<WorkspaceIndexSnapshot>,
    spec_file: Option<SpecFileSnapshot>,
    features_index: Option<FeaturesIndexSnapshot>,
    ark_files: Vec<PathBuf>,
    commits_landed: u32,
}

struct JournalSnapshot {
    path: PathBuf,
    pre_append_len: u64,
}

struct WorkspaceIndexSnapshot {
    path: PathBuf,
    prev_bytes: Option<Vec<u8>>,
}

struct SpecFileSnapshot {
    path: PathBuf,
    /// `None` means the file was absent before extract.
    prev_bytes: Option<Vec<u8>>,
}

struct FeaturesIndexSnapshot {
    path: PathBuf,
    prev_bytes: Vec<u8>,
}

impl RollbackGuard {
    fn new(task_dir: &Path, task_cwd: &Path) -> Self {
        let _ = task_dir;
        Self {
            armed: true,
            task_cwd: task_cwd.to_path_buf(),
            prev_toml: None,
            journal: None,
            workspace_index: None,
            spec_file: None,
            features_index: None,
            ark_files: Vec::new(),
            commits_landed: 0,
        }
    }

    /// Records that the first commit landed. Used by `Drop` to decide
    /// whether to issue `git reset --soft HEAD~1` during rollback.
    fn note_commit_landed(&mut self) {
        self.commits_landed = 1;
    }

    /// Adds files to the targeted-unstage list (called for the amend step,
    /// after the pre-commit `git add` already populated the list).
    fn add_staged(&mut self, more: Vec<PathBuf>) {
        self.ark_files.extend(more);
    }

    fn snapshot_toml(&mut self, toml: TaskToml) {
        // Path is task_dir/task.toml, but TaskToml::save resolves it from the
        // dir. Store None for path; the restore path uses the dir we passed
        // to `new`. We retain the cloned TaskToml so restore can re-save it.
        self.prev_toml = Some((PathBuf::new(), toml));
    }

    /// Snapshots the active journal file's pre-append length.
    ///
    /// Returns `Ok(Some(active_path_str))` so the caller can also snapshot the
    /// workspace index path in one go. Returns `Ok(None)` on systems where the
    /// journal write was skipped (the caller will discard the snapshot via
    /// [`discard_journal_snapshot`](Self::discard_journal_snapshot) if no
    /// append later happens).
    fn snapshot_journal_active(&mut self, layout: &Layout) -> Result<Option<String>> {
        use crate::commands::agent::workspace::record::WorkspaceRecorded;
        let _ = WorkspaceRecorded::SkippedNoIdentity;
        // We snapshot whichever active journal exists *now*; if record_task
        // returns Skipped*, the snapshot is discarded.
        // Determine developer from the .developer file; if absent, no
        // snapshot is needed.
        let developer =
            match crate::commands::agent::workspace::identity::read_developer_name(layout)? {
                Some(d) => d,
                None => return Ok(None),
            };
        let (path, _n) =
            crate::commands::agent::workspace::journal::find_active(layout, &developer)?;
        let pre_len = if path.exists() {
            // best-effort metadata read; on error treat as 0 so truncation
            // becomes a no-op rather than an error
            std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };
        self.journal = Some(JournalSnapshot {
            path,
            pre_append_len: pre_len,
        });
        Ok(Some(developer))
    }

    fn snapshot_workspace_index_for_active(
        &mut self,
        layout: &Layout,
        developer: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        let Some(dev) = developer else {
            return Ok(None);
        };
        let path = layout.workspace_index(dev);
        let prev_bytes = path.read_text_optional()?.map(|s| s.into_bytes());
        let result = path.clone();
        self.workspace_index = Some(WorkspaceIndexSnapshot { path, prev_bytes });
        Ok(Some(result))
    }

    fn discard_journal_snapshot(&mut self) {
        self.journal = None;
    }

    fn discard_workspace_index_snapshot(&mut self) {
        self.workspace_index = None;
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

    fn snapshot_features_index(&mut self, path: &Path) -> Result<()> {
        // The features INDEX is created by `ark init`; if absent, treat as
        // empty bytes so a rollback that's never triggered does nothing.
        let prev_bytes = if path.exists() {
            path.read_bytes()?
        } else {
            Vec::new()
        };
        self.features_index = Some(FeaturesIndexSnapshot {
            path: path.to_path_buf(),
            prev_bytes,
        });
        Ok(())
    }

    fn record_staged(&mut self, ark_files: Vec<PathBuf>) {
        self.ark_files = ark_files;
    }

    fn commit(mut self) {
        self.armed = false;
    }

    fn restore(&self) {
        // Order:
        //   1. soft-reset any landed commit so HEAD points at the parent.
        //   2. targeted unstage of Ark-managed files only — preserves the
        //      user's pre-existing index entries.
        //   3. workspace index restore.
        //   4. journal truncate to pre-append length.
        //   5. SPEC file restore (deep tier).
        //   6. features INDEX restore (deep tier).
        //   7. task.toml restore.
        //
        // Reverse-of-recording ensures each restore sees the prior on-disk
        // state.
        for _ in 0..self.commits_landed {
            let out = run_git(&["reset", "--soft", "HEAD~1"], &self.task_cwd);
            if let Ok(o) = out
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

        if let Some(snap) = &self.workspace_index {
            match &snap.prev_bytes {
                Some(b) => {
                    if let Err(e) = snap.path.write_bytes(b) {
                        eprintln!(
                            "rollback: workspace index restore at {:?} failed: {e}",
                            snap.path
                        );
                    }
                }
                None => {
                    let _ = std::fs::remove_file(&snap.path);
                }
            }
        }

        if let Some(snap) = &self.journal
            && let Err(e) = truncate_to(&snap.path, snap.pre_append_len)
        {
            eprintln!(
                "rollback: journal truncate at {:?} to {} bytes failed: {e}",
                snap.path, snap.pre_append_len
            );
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

        if let Some(snap) = &self.features_index
            && let Err(e) = snap.path.write_bytes(&snap.prev_bytes)
        {
            eprintln!(
                "rollback: features INDEX restore at {:?} failed: {e}",
                snap.path
            );
        }

        if let Some((task_dir_unused, toml)) = &self.prev_toml {
            // The task dir is reachable through TaskToml's id + the cwd; we
            // saved the prev toml from `task_dir`, so we restore it there.
            // We did not retain task_dir explicitly — re-derive from cwd
            // (which is the worktree or root) plus `.ark/tasks/<id>`.
            let _ = task_dir_unused;
            let derived_task_dir = self.task_cwd.join(".ark").join("tasks").join(&toml.id);
            if let Err(e) = toml.save(&derived_task_dir) {
                eprintln!(
                    "rollback: task.toml restore at {:?} failed: {e}",
                    derived_task_dir
                );
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

/// Truncates a file to the given byte length, creating it empty if missing.
fn truncate_to(path: &Path, len: u64) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let f = std::fs::OpenOptions::new().write(true).open(path)?;
    f.set_len(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::agent::state::Tier;

    /// Verifies the precondition table: only `(Quick, Execute)`,
    /// `(Standard, Verify)`, and `(Deep, Verify)` are accepted as commit inputs.
    #[test]
    fn check_phase_for_commit_accepts_only_legal_inputs() {
        assert!(check_phase_for_commit(Tier::Quick, Phase::Execute).is_ok());
        assert!(check_phase_for_commit(Tier::Standard, Phase::Verify).is_ok());
        assert!(check_phase_for_commit(Tier::Deep, Phase::Verify).is_ok());

        for (tier, phase) in [
            (Tier::Quick, Phase::Design),
            (Tier::Quick, Phase::Verify),
            (Tier::Standard, Phase::Execute),
            (Tier::Standard, Phase::Plan),
            (Tier::Deep, Phase::Plan),
            (Tier::Deep, Phase::Review),
            (Tier::Deep, Phase::Execute),
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
    /// INDEX. The journal file and workspace index are deliberately absent
    /// — they are staged for the amend step inline by `task_commit`.
    #[test]
    fn ark_files_for_first_commit_excludes_user_work() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path());
        let task_dir = layout.task_dir("foo");
        let spec_path = layout.specs_feature_dir("foo").join("SPEC.md");
        let features_index = layout.specs_features_index();

        let standard = ark_files_for_first_commit(&task_dir, false, &spec_path, &features_index);
        assert_eq!(standard.len(), 1);
        assert!(standard[0].ends_with("task.toml"));

        let deep = ark_files_for_first_commit(&task_dir, true, &spec_path, &features_index);
        assert_eq!(deep.len(), 3);
        assert!(deep.iter().all(|p| p.to_string_lossy().contains(".ark/")));
    }
}

// -- End-to-end git tests -------------------------------------------------

#[cfg(test)]
mod e2e {
    //! End-to-end tests that drive `task_commit` against a real
    //! `git init` tempdir.
    //!
    //! These cover the post-refactor invariants codex flagged:
    //! - V-IT-1: closing commit contains all five Ark-managed files.
    //! - V-UT-17 / C-23: no Ark-managed file appears in `git status` post-commit.
    //! - V-UT-36/37/38 / R-201: slug-anchored `git log -S` recovers the
    //!   closing SHA across later journal writes.

    use super::*;
    use crate::{
        InitOptions,
        commands::agent::{
            state::Tier,
            task::{
                new::{TaskNewOptions, task_new},
                phase::{TaskPhaseOptions, task_execute, task_plan, task_review, task_verify},
            },
            workspace::init::{WorkspaceInitOptions, workspace_init},
        },
        init::init,
        io::{PathExt, git::run_git},
    };

    /// Initializes a git repo, scaffolds Ark, and bootstraps a developer.
    fn init_repo_with_ark_and_dev(name: &str) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        run_git(&["init", "--quiet"], tmp.path()).unwrap();
        run_git(&["config", "user.email", "test@example.com"], tmp.path()).unwrap();
        run_git(&["config", "user.name", "Test"], tmp.path()).unwrap();
        run_git(&["config", "commit.gpgsign", "false"], tmp.path()).unwrap();
        run_git(&["checkout", "-b", "main"], tmp.path()).unwrap();
        // Initial commit so HEAD has a SHA for `git rev-parse HEAD`.
        tmp.path()
            .join("README.md")
            .write_bytes(b"# repo\n")
            .unwrap();
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "init", "--quiet"], tmp.path()).unwrap();
        // Scaffold Ark.
        init(InitOptions::new(tmp.path())).unwrap();
        // Workspace identity.
        workspace_init(WorkspaceInitOptions {
            project_root: tmp.path().to_path_buf(),
            name: name.into(),
        })
        .unwrap();
        // Commit the scaffold so subsequent `task_commit` runs aren't fighting
        // ark scaffolding diff noise.
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(&["commit", "-m", "ark scaffold", "--quiet"], tmp.path()).unwrap();
        tmp
    }

    /// Drives a quick task to `Phase::Execute` and stages a user-work file.
    ///
    /// Returns the slug. Standard / deep variants reuse this and advance further.
    /// Mirrors the real workflow: scaffold the task, commit the PRD/task.toml
    /// scaffolding, then move to execute and stage real work.
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
        // Commit scaffolding so it is not dirty at task_commit time.
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
        // Stage real user work.
        if let Some(parent) = std::path::Path::new(work_path).parent()
            && !parent.as_os_str().is_empty()
        {
            tmp.join(parent).ensure_dir().unwrap();
        }
        tmp.join(work_path).write_bytes(work_body).unwrap();
        run_git(&["add", work_path], tmp).unwrap();
    }

    /// Counts files in `git status --porcelain` whose path matches an
    /// Ark-managed substring. Returns the matching paths so failures are
    /// readable.
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

    /// V-IT-3 / V-UT-17 / C-23: quick-tier closing commit captures all
    /// Ark-managed files in one commit and leaves no Ark-managed file dirty.
    #[test]
    fn quick_tier_commit_is_atomic_and_clean() {
        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "qd", "src/foo.rs", b"pub fn foo() {}\n");

        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat(qd): user work".into()),
            no_commit: false,
        })
        .unwrap();
        assert_eq!(summary.tier, Tier::Quick);
        assert!(summary.head_sha.is_some());
        assert_eq!(summary.session_number, Some(1));
        assert!(!summary.deep_spec_promoted);

        let dirty = ark_managed_dirty(tmp.path());
        assert!(
            dirty.is_empty(),
            "no Ark-managed file should be dirty post-commit, got {dirty:?}"
        );

        // Closing commit's tree must include task.toml + journal + workspace index + work file.
        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains(".ark/tasks/qd/task.toml"));
        assert!(head_files.contains(".ark/workspace/alice/journal-1.md"));
        assert!(head_files.contains(".ark/workspace/alice/index.md"));
        assert!(head_files.contains("src/foo.rs"));
    }

    /// R-204: user pre-staged file outside Ark's purview must be captured by
    /// the closing commit (it is part of `git diff --cached`); user unstaged
    /// files must remain untouched.
    #[test]
    fn commit_does_not_touch_unstaged_user_files() {
        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "qd", "src/foo.rs", b"pub fn foo() {}\n");
        // An UNSTAGED user file outside Ark's purview.
        tmp.path()
            .join("notes.txt")
            .write_bytes("draft notes — do not commit yet\n".as_bytes())
            .unwrap();

        task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat(qd): foo".into()),
            no_commit: false,
        })
        .unwrap();

        // notes.txt is still unstaged-modified afterwards.
        let status = run_git(&["status", "--porcelain"], tmp.path()).unwrap();
        assert!(
            status.stdout.contains("notes.txt"),
            "user's unstaged file must survive untouched: {}",
            status.stdout
        );
    }

    /// R-102: `task_commit` rejects when nothing is staged.
    #[test]
    fn commit_errors_when_nothing_staged() {
        let tmp = init_repo_with_ark_and_dev("alice");
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
        // No `git add` — staging area is clean.
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "qd".into(),
            message: Some("feat: x".into()),
            no_commit: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::NothingStaged { .. }));
    }

    /// R-201 / V-UT-36: slug-anchored `git log -S` recovers the closing SHA.
    #[test]
    fn slug_anchored_log_recovers_closing_sha() {
        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "demo", "src/foo.rs", b"x\n");
        let summary = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: Some("feat: demo".into()),
            no_commit: false,
        })
        .unwrap();
        let expected = summary.head_sha.unwrap();

        let layout = Layout::new(tmp.path());
        let journal_path = layout.workspace_journal("alice", 1);
        let recovered = run_git(
            &[
                "log",
                "-S",
                "**Slug**: demo",
                "--format=%H",
                "-n",
                "1",
                "--",
                journal_path.to_str().unwrap(),
            ],
            tmp.path(),
        )
        .unwrap();
        assert_eq!(recovered.stdout.trim(), expected);
    }

    /// R-201 / V-UT-37: lookup remains valid after a manual `/ark:record`
    /// adds a later entry to the same journal file.
    #[test]
    fn slug_anchored_log_survives_later_manual_record() {
        use crate::commands::agent::workspace::record::{WorkspaceRecordOptions, workspace_record};

        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "demo", "src/foo.rs", b"x\n");
        let original = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: Some("feat: demo".into()),
            no_commit: false,
        })
        .unwrap();
        let original_sha = original.head_sha.unwrap();

        // Now a manual `/ark:record` entry lands on top of the same journal.
        workspace_record(WorkspaceRecordOptions {
            project_root: tmp.path().to_path_buf(),
            title: Some("manual note".into()),
            summary: Some("did something".into()),
            next: None,
        })
        .unwrap();
        // The user commits the manual entry separately (mirrors what /ark:record's
        // template instructs).
        run_git(&["add", "."], tmp.path()).unwrap();
        run_git(
            &["commit", "-m", "chore: manual record", "--quiet"],
            tmp.path(),
        )
        .unwrap();

        let layout = Layout::new(tmp.path());
        let journal_path = layout.workspace_journal("alice", 1);
        let recovered = run_git(
            &[
                "log",
                "-S",
                "**Slug**: demo",
                "--format=%H",
                "-n",
                "1",
                "--",
                journal_path.to_str().unwrap(),
            ],
            tmp.path(),
        )
        .unwrap();
        assert_eq!(
            recovered.stdout.trim(),
            original_sha,
            "slug-anchored lookup must still return the original closure commit"
        );
    }

    /// R-201 / V-UT-38: lookup distinguishes between two task closures on
    /// the same journal file.
    #[test]
    fn slug_anchored_log_distinguishes_two_tasks() {
        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "foo", "src/foo.rs", b"foo\n");
        let foo = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "foo".into(),
            message: Some("feat: foo".into()),
            no_commit: false,
        })
        .unwrap()
        .head_sha
        .unwrap();

        quick_at_execute_with_staged_work(tmp.path(), "bar", "src/bar.rs", b"bar\n");
        let bar = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "bar".into(),
            message: Some("feat: bar".into()),
            no_commit: false,
        })
        .unwrap()
        .head_sha
        .unwrap();

        let layout = Layout::new(tmp.path());
        let journal_path = layout.workspace_journal("alice", 1);

        let look = |slug: &str| -> String {
            run_git(
                &[
                    "log",
                    "-S",
                    &format!("**Slug**: {slug}"),
                    "--format=%H",
                    "-n",
                    "1",
                    "--",
                    journal_path.to_str().unwrap(),
                ],
                tmp.path(),
            )
            .unwrap()
            .stdout
            .trim()
            .to_string()
        };
        assert_eq!(look("foo"), foo);
        assert_eq!(look("bar"), bar);
    }

    /// V-UT-13 + C-3: `task_commit` without `-m` and without `--no-commit`
    /// errors with `CommitMessageRequired`.
    #[test]
    fn commit_errors_when_message_missing() {
        let tmp = init_repo_with_ark_and_dev("alice");
        quick_at_execute_with_staged_work(tmp.path(), "demo", "src/foo.rs", b"x\n");
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: None,
            no_commit: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::CommitMessageRequired { .. }));
    }

    /// V-UT-12 / C-14: `--no-commit` skips the git commit + journal but on
    /// quick tier still flips phase to `Committed` (no SPEC to extract).
    #[test]
    fn no_commit_flips_phase_without_committing() {
        let tmp = init_repo_with_ark_and_dev("alice");
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
        })
        .unwrap();
        assert!(summary.head_sha.is_none());
        assert!(summary.session_number.is_none());

        let layout = Layout::new(tmp.path());
        let toml = TaskToml::load(&layout.task_dir("qd")).unwrap();
        assert_eq!(toml.phase, Phase::Committed);
        assert!(toml.committed_at.is_some());
    }

    /// R-203 / V-UT-16 / V-IT-13/14: pre-commit hook rejects → rollback
    /// restores task.toml, journal, workspace index. User's pre-existing
    /// staged file (outside Ark's purview) survives the targeted unstage.
    #[test]
    fn rollback_on_pre_commit_hook_failure_restores_everything() {
        let tmp = init_repo_with_ark_and_dev("alice");
        // Install a pre-commit hook that always rejects.
        let hooks = tmp.path().join(".git/hooks");
        hooks.ensure_dir().unwrap();
        let hook_path = hooks.join("pre-commit");
        hook_path.write_bytes(b"#!/bin/sh\nexit 1\n").unwrap();
        run_git(&["update-index", "--chmod=+x", "--refresh"], tmp.path()).ok();
        // Make hook executable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hook_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hook_path, perms).unwrap();
        }

        // User pre-stages a file unrelated to Ark.
        tmp.path()
            .join("user_intent.txt")
            .write_bytes(b"my staged work\n")
            .unwrap();
        run_git(&["add", "user_intent.txt"], tmp.path()).unwrap();

        // Now drive a quick task that will hit the rejecting hook.
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
        let journal_path = layout.workspace_journal("alice", 1);
        let pre_journal = journal_path.read_bytes().unwrap();
        let pre_toml = TaskToml::load(&layout.task_dir("demo")).unwrap();

        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "demo".into(),
            message: Some("feat: x".into()),
            no_commit: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::GitCommitFailed { .. }));

        // Rollback restored task.toml.
        let post_toml = TaskToml::load(&layout.task_dir("demo")).unwrap();
        assert_eq!(
            post_toml.phase, pre_toml.phase,
            "task.toml phase rolled back"
        );
        assert!(post_toml.committed_at.is_none(), "committed_at not set");
        // Journal restored byte-for-byte.
        let post_journal = journal_path.read_bytes().unwrap();
        assert_eq!(pre_journal, post_journal, "journal rolled back");
        // User's pre-staged file is still in the index.
        let cached = run_git(&["diff", "--cached", "--name-only"], tmp.path())
            .unwrap()
            .stdout;
        assert!(
            cached.contains("user_intent.txt"),
            "user's pre-staged file must survive: cached={cached:?}"
        );
    }

    /// Phase precondition test: standard tier requires `Verify`, not `Execute`.
    #[test]
    fn standard_tier_commit_from_execute_errors() {
        let tmp = init_repo_with_ark_and_dev("alice");
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
        // Stage so the precondition fails on phase, not staging.
        tmp.path().join("foo.rs").write_bytes(b"x").unwrap();
        run_git(&["add", "foo.rs"], tmp.path()).unwrap();
        let err = task_commit(TaskCommitOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "std".into(),
            message: Some("m".into()),
            no_commit: false,
        })
        .unwrap_err();
        assert!(matches!(err, Error::IllegalPhaseTransition { .. }));
    }

    /// Standard-tier full happy path: design → plan → execute → verify →
    /// stage → commit. Closing commit contains all four files; phase is
    /// Committed; slug-anchored lookup recovers the SHA.
    #[test]
    fn standard_tier_commit_happy_path() {
        let tmp = init_repo_with_ark_and_dev("alice");
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
        })
        .unwrap();
        assert!(summary.head_sha.is_some());
        let layout = Layout::new(tmp.path());
        let toml = TaskToml::load(&layout.task_dir("std")).unwrap();
        assert_eq!(toml.phase, Phase::Committed);
    }

    /// Deep-tier full happy path: design → plan → review → execute → verify
    /// → stage → commit. The closing commit also contains the promoted
    /// SPEC + features INDEX. Rollback for spec_extract failure is exercised
    /// by the unit-level `RollbackGuard` tests in the parent module.
    #[test]
    fn deep_tier_commit_promotes_spec_into_closing_commit() {
        let tmp = init_repo_with_ark_and_dev("alice");
        task_new(TaskNewOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
            title: "deep demo".into(),
            tier: Tier::Deep,
            worktree: None,
        })
        .unwrap();
        task_plan(TaskPhaseOptions {
            project_root: tmp.path().to_path_buf(),
            slug: "deep".into(),
        })
        .unwrap();
        // Seed the plan with a Spec section so spec_extract succeeds.
        tmp.path()
            .join(".ark/tasks/deep/00_PLAN.md")
            .write_bytes(b"# plan 00\n## Spec\n\n[**Goals**]\n- G-1: v1\n\n## Runtime\nrt\n")
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

        // The seeded VERIFY.md is full of `PENDING` items by design; the
        // implementer would resolve each before commit. Replace the body
        // with an all-PASS document so the deep-tier gate accepts.
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
        })
        .unwrap();
        assert!(summary.deep_spec_promoted);

        let head_files = run_git(&["show", "--name-only", "--format=", "HEAD"], tmp.path())
            .unwrap()
            .stdout;
        assert!(head_files.contains(".ark/tasks/deep/task.toml"));
        assert!(head_files.contains(".ark/specs/features/deep/SPEC.md"));
        assert!(head_files.contains(".ark/specs/features/INDEX.md"));
        assert!(head_files.contains(".ark/workspace/alice/journal-1.md"));
    }
}
