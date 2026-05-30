//! Error model shared by Ark command implementations.

use std::{io, path::PathBuf};

use thiserror::Error;

use crate::commands::agent::state::{Phase, Tier};

/// Convenient result alias using Ark's [`enum@Error`] type.
pub type Result<T> = std::result::Result<T, Error>;

/// Error cases produced by Ark command and I/O operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Filesystem or path-specific I/O failed.
    #[error("io error at {path}: {source}")]
    Io {
        /// Path associated with the failed operation.
        path: PathBuf,
        /// Original I/O error.
        #[source]
        source: io::Error,
    },

    /// Installation manifest JSON could not be parsed.
    #[error("manifest corrupt at {path}: {source}")]
    ManifestCorrupt {
        /// Manifest path.
        path: PathBuf,
        /// JSON parse error.
        #[source]
        source: serde_json::Error,
    },

    /// Snapshot JSON could not be parsed or decoded.
    #[error("snapshot corrupt: {reason}")]
    SnapshotCorrupt {
        /// Human-readable corruption reason.
        reason: String,
    },

    /// Ark is already loaded in the target project.
    #[error("ark is already loaded at {path}; pass --force to replace it")]
    AlreadyLoaded {
        /// Existing Ark directory path.
        path: PathBuf,
    },

    /// No Ark installation exists at the requested path.
    #[error("no ark installation found at {path}")]
    NotLoaded {
        /// Path that failed discovery.
        path: PathBuf,
    },

    /// Snapshot path validation rejected an unsafe path.
    #[error("refusing unsafe snapshot path {path:?}: {reason}")]
    UnsafeSnapshotPath {
        /// Unsafe path from the snapshot.
        path: PathBuf,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// Requested phase transition is illegal for the task tier.
    #[error("illegal phase transition under tier {tier:?}: {from:?} -> {to:?}")]
    IllegalPhaseTransition {
        /// Workflow tier whose state machine was checked.
        tier: Tier,
        /// Current phase.
        from: Phase,
        /// Requested phase.
        to: Phase,
    },

    /// Command was invoked for the wrong task tier.
    #[error("wrong tier: expected {expected:?}, got {actual:?}")]
    WrongTier {
        /// Required tier.
        expected: Tier,
        /// Actual task tier.
        actual: Tier,
    },

    /// Task slug did not resolve to an existing task.
    #[error("task not found: {slug}")]
    TaskNotFound {
        /// Missing task slug.
        slug: String,
    },

    /// Task slug already exists at the destination.
    #[error("task already exists: {slug}")]
    TaskAlreadyExists {
        /// Existing task slug or archive slug.
        slug: String,
    },

    /// This checkout has no focused task. Run `task new` or `task resume` to bind.
    #[error(
        "no focus set in `{}`; run `ark agent task new` or `task resume --slug <one-of>` to bind \
         this checkout (active: {})",
        project_root.display(),
        if candidates.is_empty() { "<none>".to_string() } else { candidates.join(", ") },
    )]
    NoFocus {
        /// Checkout root that was probed.
        project_root: PathBuf,
        /// Active task slugs available for `task resume`.
        candidates: Vec<String>,
    },

    /// Requested embedded template is unknown.
    #[error("unknown template: {name}")]
    UnknownTemplate {
        /// Template name.
        name: String,
    },

    /// PLAN file has no extractable SPEC section.
    #[error("PLAN at {plan_path} has no `## Spec` section")]
    SpecSectionMissing {
        /// PLAN path that was inspected.
        plan_path: PathBuf,
    },

    /// Feature SPEC already exists at the target path.
    #[error("feature SPEC already exists for `{feature}` at {path}")]
    SpecAlreadyExists {
        /// Feature slug.
        feature: String,
        /// Existing SPEC path.
        path: PathBuf,
    },

    /// Task directory has no `NN_PLAN.md` files.
    #[error("no `NN_PLAN.md` found in {task_dir}")]
    NoPlanFound {
        /// Task directory that was inspected.
        task_dir: PathBuf,
    },

    /// `task.toml` could not be parsed.
    #[error("task.toml corrupt at {path}: {source}")]
    TaskTomlCorrupt {
        /// Corrupt TOML path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Feature SPEC registration field failed validation.
    #[error("invalid spec field `{field}`: {reason}")]
    InvalidSpecField {
        /// Field name.
        field: String,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// Task metadata field failed validation.
    #[error("invalid task field `{field}`: {reason}")]
    InvalidTaskField {
        /// Field name.
        field: String,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// Managed-block delimiters are unbalanced.
    #[error("managed block corrupt in {path}: marker `{marker}` has START without END")]
    ManagedBlockCorrupt {
        /// File containing the corrupt block.
        path: PathBuf,
        /// Marker whose start delimiter has no end delimiter.
        marker: String,
    },

    /// CLI version is older than the recorded install version.
    #[error(
        "refusing to downgrade: project is at {project_version}, CLI is {cli_version}; pass \
         --allow-downgrade to proceed"
    )]
    DowngradeRefused {
        /// Version recorded in the project manifest.
        project_version: String,
        /// Running CLI version.
        cli_version: String,
    },

    /// Installation manifest contains an unsafe path.
    #[error("unsafe path in installation manifest {path:?}: {reason}")]
    UnsafeManifestPath {
        /// Unsafe path from the manifest.
        path: PathBuf,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// Spawning `git` failed.
    #[error("failed to spawn git: {source}")]
    GitSpawn {
        /// Original process-spawn error.
        #[source]
        source: io::Error,
    },

    // Worktree feature errors.
    /// Worktree destination already exists.
    #[error("worktree directory already exists at {path:?}")]
    WorktreeDirExists {
        /// Existing worktree path.
        path: PathBuf,
    },

    /// No worktree is bound to the requested task slug.
    #[error("no worktree found for slug `{slug}`")]
    WorktreeNotFound {
        /// Task slug.
        slug: String,
    },

    /// Worktree has uncommitted changes.
    #[error("worktree at {path:?} has uncommitted changes; pass --force to override")]
    WorktreeDirty {
        /// Dirty worktree path.
        path: PathBuf,
    },

    /// Branch is already checked out somewhere else.
    #[error("branch `{branch}` is already checked out at {where_at:?}")]
    BranchInUse {
        /// Branch name.
        branch: String,
        /// Existing checkout path.
        where_at: PathBuf,
    },

    /// Branch name failed validation.
    #[error("invalid branch name `{branch}`: {reason}")]
    InvalidBranchName {
        /// Invalid branch name.
        branch: String,
        /// Validation failure reason.
        reason: String,
    },

    /// Branch type is not one of Ark's allowed prefixes.
    #[error("invalid branch type `{value}`; expected one of feat, fix, refactor, chore, ci, docs")]
    InvalidBranchType {
        /// Invalid branch type value.
        value: String,
    },

    /// Worktree config TOML could not be parsed.
    #[error("config.toml corrupt at {path:?}: {source}")]
    WorktreeConfigCorrupt {
        /// Config file path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Worktree post-create hook exited non-zero.
    #[error("post_create hook `{command}` failed with exit code {exit_code}")]
    PostCreateHookFailed {
        /// Hook command string.
        command: String,
        /// Process exit code.
        exit_code: i32,
    },

    /// Configured worktree copy source is missing.
    #[error("config.toml [worktree] `copy` source missing: {path:?}")]
    WorktreeCopySourceMissing {
        /// Missing source path.
        path: PathBuf,
    },

    /// Task slug already exists in the parent checkout.
    #[error(
        "task `{slug}` already exists on the parent at {path:?}; archive it or run without \
         --worktree"
    )]
    TaskExistsOnParent {
        /// Existing task slug.
        slug: String,
        /// Existing parent task path.
        path: PathBuf,
    },

    /// Worktree creation was invoked from inside a worktree.
    #[error(
        "`task new --worktree` cannot be invoked from inside an existing worktree (root is \
         {current_root:?})"
    )]
    NestedWorktreeForbidden {
        /// Current worktree root.
        current_root: PathBuf,
    },

    /// Config field failed validation.
    #[error("invalid config.toml field `{field}`: {reason}")]
    InvalidConfigField {
        /// Field name.
        field: &'static str,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// No developer identity available for a workspace operation.
    #[error(
        "no developer identity set; run `ark init --developer <name>` or set [workspace] \
         developer in .ark/config.toml"
    )]
    MissingIdentity,

    /// Writing the `.ark/.developer` file failed.
    #[error("failed to write developer file at {path:?}: {source}")]
    DeveloperWriteFailed {
        /// Identity file path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Workspace config TOML could not be parsed.
    #[error("workspace config invalid at {path:?}: {source}")]
    WorkspaceConfigInvalid {
        /// Config file path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Agent-edited entry file lacks one of the required sections.
    #[error("entry file malformed: {reason}")]
    EntryFileMalformed {
        /// Specific structural failure.
        reason: &'static str,
    },

    /// Active journal's last `## Session N:` heading already carries stamped
    /// auto-fields — the agent did not append a fresh heading before invoking
    /// `task commit`.
    #[error(
        "journal `{}` last `## Session` heading is already stamped; append a fresh `## Session N: \
         <title>` block (with `### Summary` and `### Main Changes`) for slug `{slug}` before \
         re-running `ark agent task commit`",
        journal_path.display()
    )]
    JournalSessionHeadingMissing {
        /// Active journal file path.
        journal_path: PathBuf,
        /// Slug of the task being committed (`"-"` for manual entries).
        slug: String,
    },

    /// `ark archive` refuses to run when the git staging area is dirty.
    #[error(
        "ark archive requires a clean staging area; run `git stash` or `git commit` first ({} staged path(s))",
        .staged_paths.len()
    )]
    ArchiveIndexNotEmpty {
        /// Paths currently staged in the index.
        staged_paths: Vec<String>,
    },

    /// Pickaxe lookup found zero commits matching `**Slug**: <slug>`.
    #[error(
        "could not resolve closing commit for slug `{slug}` in {journal:?} (pickaxe match count: \
         0)"
    )]
    SlotResolveNoMatch {
        /// Slug whose sentinel we tried to resolve.
        slug: String,
        /// Journal file path.
        journal: PathBuf,
    },

    /// Pickaxe lookup found more than one matching commit.
    #[error(
        "ambiguous closing commit for slug `{slug}`; pickaxe matched {} commits: {candidates:?}",
        .candidates.len()
    )]
    SlotResolveAmbiguous {
        /// Slug whose sentinel we tried to resolve.
        slug: String,
        /// All candidate full SHAs.
        candidates: Vec<String>,
    },

    /// Journal file recorded in `task.toml.journal_path` is missing on disk.
    #[error("workspace journal missing at {path:?}")]
    JournalMissing {
        /// Recorded (and missing) journal path.
        path: PathBuf,
    },

    /// Journal file's tail no longer matches the bytes we appended; rollback
    /// would delete a concurrent appender's data, so we left the file intact.
    #[error(
        "journal at {path:?} drifted (expected suffix {expected_suffix_len} bytes, current length \
         {actual_len}); leaving file untouched"
    )]
    JournalDriftDetected {
        /// Journal file path.
        path: PathBuf,
        /// Bytes this transaction appended.
        expected_suffix_len: u64,
        /// Current file length on disk.
        actual_len: u64,
    },

    /// State file `.state.toml` failed to parse.
    #[error("state.toml corrupt at {path:?}: {source}")]
    StateTomlCorrupt {
        /// State file path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Exclusive state-file lock could not be acquired within the backoff window.
    #[error("state.toml lock contended at {path:?}; another process is mutating state")]
    StateLockContended {
        /// Lock file path.
        path: PathBuf,
    },

    /// Task discard refused because a seeded artifact has user content.
    #[error("task `{slug}` has user content in {file}; pass --force to discard anyway")]
    TaskStillActive {
        /// Slug of the task being discarded.
        slug: String,
        /// First seeded file whose contents diverged from its template.
        file: String,
    },

    // Commit closure errors.
    /// `task_commit` invoked with no staged work.
    #[error("task `{slug}` cannot be committed without staged work; run `git add <files>` first")]
    NothingStaged {
        /// Slug of the task being committed.
        slug: String,
    },

    /// VERIFY.md has unresolved checklist items or findings.
    #[error(
        "VERIFY.md at {path:?} has {items} pending item(s) and {findings} pending finding(s); \
         resolve before commit"
    )]
    VerifyIncomplete {
        /// Path to the VERIFY.md being parsed.
        path: PathBuf,
        /// Count of `- [ ] ...: PENDING` checklist items.
        items: u32,
        /// Count of findings with `Resolution: PENDING`.
        findings: u32,
    },

    /// `git commit` exited non-zero during `task_commit`.
    #[error("git commit failed at {path:?}: {stderr}")]
    GitCommitFailed {
        /// Working directory where `git commit` ran.
        path: PathBuf,
        /// Trimmed stderr from the failed invocation.
        stderr: String,
    },

    /// `task_commit` invoked without `-m` and without `--no-commit`.
    #[error(
        "commit message is required for task `{slug}`: pass `-m` or generate one before invoking \
         `task commit`"
    )]
    CommitMessageRequired {
        /// Slug of the task being committed.
        slug: String,
    },

    /// `ark archive` saw a `phase = Committed` task with no `committed_at`.
    #[error(
        "task `{slug}` has phase Committed but committed_at is missing; cannot derive archive \
         month"
    )]
    CommittedAtMissing {
        /// Slug of the inconsistent task.
        slug: String,
    },

    /// VERIFY template is missing a required substitution marker.
    #[error("template marker `{marker}` is missing in {path:?}")]
    TemplateMarkerMissing {
        /// Marker name that was not found.
        marker: &'static str,
        /// Template file path.
        path: PathBuf,
    },

    /// PRD has no `[**SPEC Path**]` block; required on deep tier.
    #[error("PRD at `{source_path}` has no `[**SPEC Path**]` block")]
    FeaturePathMissing {
        /// File whose `[**SPEC Path**]` block was expected (PRD or caller artifact).
        source_path: PathBuf,
    },

    /// Parsed SPEC-path body failed validation.
    #[error("invalid SPEC path `{value}`: {reason}")]
    InvalidFeaturePath {
        /// File the offending value came from (PRD path, or a non-PRD caller's artifact).
        source_path: PathBuf,
        /// Offending value verbatim.
        value: String,
        /// Validation failure reason.
        reason: &'static str,
    },

    // `[upgrade]` strategy + backup errors.
    /// The `[upgrade]` section of `.ark/config.toml` could not be parsed.
    #[error("config.toml `[upgrade]` corrupt at {path:?}: {source}")]
    UpgradeConfigCorrupt {
        /// Config file path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// An `[upgrade]` strategy entry failed semantic validation.
    #[error("invalid `[upgrade]` config: {reason} ({path})")]
    UpgradeConfigInvalid {
        /// The offending strategy path.
        path: PathBuf,
        /// Validation failure reason.
        reason: &'static str,
    },

    /// `ark upgrade --restore` found no backup to restore.
    #[error("no upgrade backup to restore at {path:?}; nothing to undo")]
    NoBackupToRestore {
        /// Backup directory that was probed.
        path: PathBuf,
    },

    // `ark sandbox` feature errors.
    /// Spawning `docker` failed (binary missing, permissions).
    #[error("docker {op} failed to spawn: {source}")]
    DockerSpawn {
        /// The docker subcommand being run (e.g. `"info"`, `"run"`).
        op: &'static str,
        /// Original process-spawn error.
        #[source]
        source: io::Error,
    },

    /// The selected sandbox backend is not reachable.
    #[error("sandbox backend `{engine}` is unavailable")]
    SandboxBackendUnavailable {
        /// Backend id that failed its availability probe.
        engine: String,
    },

    /// `[sandbox] engine` named a backend Ark does not implement.
    #[error("unknown sandbox engine `{value}`")]
    UnknownSandboxEngine {
        /// Offending engine value verbatim.
        value: String,
    },

    /// The `[sandbox]` section of `.ark/config.toml` could not be parsed.
    #[error("config.toml `[sandbox]` corrupt at {path:?}: {source}")]
    SandboxConfigCorrupt {
        /// Config file path.
        path: PathBuf,
        /// TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// A `[sandbox]` config value failed semantic validation.
    #[error("invalid sandbox config: {reason}")]
    SandboxConfigInvalid {
        /// Validation failure reason.
        reason: &'static str,
    },

    /// A sandbox already exists for the requested slug.
    #[error("sandbox for `{slug}` already exists ({container})")]
    SandboxExists {
        /// Task slug.
        slug: String,
        /// Existing container name.
        container: String,
    },

    /// No sandbox container was found for the requested slug.
    #[error("no sandbox found for `{slug}`")]
    SandboxNotFound {
        /// Task slug.
        slug: String,
    },

    /// `docker pull` of the configured image failed.
    #[error("failed to pull image `{image}` (exit {exit_code})")]
    ImagePullFailed {
        /// Image reference that failed to pull.
        image: String,
        /// Process exit code.
        exit_code: i32,
    },

    /// `docker run` failed to start the container.
    #[error("failed to start container `{container}` (exit {exit_code})")]
    ContainerStartFailed {
        /// Container name that failed to start.
        container: String,
        /// Process exit code.
        exit_code: i32,
    },

    /// `enter --agent` found no installed platform to launch.
    #[error("no agent platform installed for `--agent`")]
    NoAgentPlatform {
        /// Checkout root that was probed.
        project_root: PathBuf,
    },

    /// The selected platform has no yolo argv defined for `--agent`.
    #[error("platform `{platform}` has no supported yolo mode for `--agent`")]
    AgentYoloUnsupported {
        /// Platform id with no yolo argv.
        platform: String,
    },
}

impl Error {
    /// Wraps an I/O error with the path being operated on.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
