//! `RecordTransaction` — atomic snapshot/rollback for journal appends + index updates.
//!
//! Owned by `workspace_record`. Snapshots are taken **before** any mutation.
//! On internal failure the primitive rolls itself back and returns `Err`;
//! on success it returns a `RecordSnapshot` higher layers (`RollbackGuard`)
//! can adopt for outer-transaction rollback.
//!
//! Two rollback flavors:
//!
//! - **Index restore** (full-file): use `io::fs::write_atomic` (temp+rename)
//!   so concurrent readers never see a partial write.
//! - **Journal rollback** (suffix-checked): keep the appended bytes in
//!   memory; on rollback verify the journal still ends with those bytes
//!   before truncating to the snapshotted length. If the suffix has drifted
//!   (concurrent appender wrote bytes after this transaction), leave the
//!   file untouched and surface `Error::JournalDriftDetected`. The
//!   primitive does no extra coordination beyond `O_APPEND`.

use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    io::{PathExt, write_atomic},
};

/// Lifecycle states for a record transaction.
///
/// A `bool` would be ambiguous: the transaction has more than two outcomes
/// (begin → mutating → committed | rolled back), and rollback decisions
/// depend on which mutation step succeeded last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxState {
    /// Begin called, no mutations yet.
    Open,
    /// Journal appended; personal index not yet updated.
    JournalAppended,
    /// Personal index updated; top-level not yet updated.
    PersonalUpdated,
    /// Top-level updated; ready to commit.
    TopLevelUpdated,
    /// Successfully committed. No further operations allowed.
    Committed,
    /// Rolled back. No further operations allowed.
    RolledBack,
}

/// Bookkeeping for a single `workspace_record` invocation.
///
/// `commit()` returns a `RecordSnapshot` that exposes only the fields
/// outer transactions need.
#[derive(Debug)]
pub struct RecordTransaction {
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    /// Bytes this transaction appended to the journal.
    ///
    /// Used by suffix-checked rollback: we only truncate if the file still
    /// ends with these exact bytes.
    appended_bytes: Vec<u8>,
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
    state: TxState,
}

/// Adoptable snapshot returned by a successful `commit()`.
///
/// Outer transactions (`CommitGuard`) call `rollback()` on this when they
/// themselves need to undo the workspace mutations after the inner
/// transaction had already succeeded.
#[derive(Debug)]
pub struct RecordSnapshot {
    journal_path: PathBuf,
    journal_byte_length_before: u64,
    appended_bytes: Vec<u8>,
    personal_index_path: PathBuf,
    personal_index_bytes_before: Vec<u8>,
    top_level_index_path: PathBuf,
    top_level_index_bytes_before: Vec<u8>,
}

impl RecordTransaction {
    /// Snapshots the three target files before any mutation.
    ///
    /// `journal_path` may not yet exist (a new journal-N.md before its first
    /// append); in that case the byte length is `0`.
    /// `personal_index_path` and `top_level_index_path` may also be absent;
    /// their snapshot bytes are empty `Vec<u8>` and the rollback is a
    /// best-effort delete (so a fresh personal index left after a failed
    /// transaction is removed cleanly).
    pub fn begin(
        journal_path: PathBuf,
        personal_index_path: PathBuf,
        top_level_index_path: PathBuf,
    ) -> Result<Self> {
        let journal_byte_length_before = match std::fs::metadata(&journal_path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
            Err(source) => {
                return Err(Error::Io {
                    path: journal_path,
                    source,
                });
            }
        };
        let personal_index_bytes_before = personal_index_path.read_optional()?.unwrap_or_default();
        let top_level_index_bytes_before =
            top_level_index_path.read_optional()?.unwrap_or_default();

        Ok(Self {
            journal_path,
            journal_byte_length_before,
            appended_bytes: Vec::new(),
            personal_index_path,
            personal_index_bytes_before,
            top_level_index_path,
            top_level_index_bytes_before,
            state: TxState::Open,
        })
    }

    /// Records the bytes that were appended to the journal.
    ///
    /// Caller passes the exact bytes they wrote; rollback compares the
    /// file's tail against these.
    pub fn record_appended_bytes(&mut self, bytes: Vec<u8>) {
        self.appended_bytes = bytes;
    }

    /// Returns the journal path the transaction is bound to.
    ///
    /// Used by `record.rs`'s stamp-suffix-capture helper.
    pub(crate) fn journal_path(&self) -> &std::path::Path {
        &self.journal_path
    }

    /// Returns the journal byte length captured at `begin()`.
    ///
    /// Used by `record.rs`'s stamp-suffix-capture helper.
    pub(crate) fn journal_byte_length_before(&self) -> u64 {
        self.journal_byte_length_before
    }

    /// Marks the journal-append step as complete.
    pub fn mark_journal_appended(&mut self) {
        self.state = TxState::JournalAppended;
    }

    /// Marks the personal-index update as complete.
    pub fn mark_personal_updated(&mut self) {
        self.state = TxState::PersonalUpdated;
    }

    /// Marks the top-level-index update as complete.
    pub fn mark_top_level_updated(&mut self) {
        self.state = TxState::TopLevelUpdated;
    }

    /// Rolls back in reverse order of completed mutations.
    ///
    /// Top-level index → personal index → journal (suffix-checked). Drift
    /// detection on the journal returns `JournalDriftDetected` while still
    /// running the other restores, so a concurrent appender's data is
    /// preserved without abandoning the index restores.
    pub fn rollback(mut self) -> Result<()> {
        if matches!(self.state, TxState::Committed | TxState::RolledBack) {
            return Ok(());
        }
        let result = restore_for_state(
            self.state,
            &self.top_level_index_path,
            &self.top_level_index_bytes_before,
            &self.personal_index_path,
            &self.personal_index_bytes_before,
            &self.journal_path,
            &self.appended_bytes,
            self.journal_byte_length_before,
        );
        self.state = TxState::RolledBack;
        result
    }

    /// Marks success and yields the adoptable snapshot.
    pub fn commit(mut self) -> RecordSnapshot {
        self.state = TxState::Committed;
        RecordSnapshot {
            journal_path: self.journal_path,
            journal_byte_length_before: self.journal_byte_length_before,
            appended_bytes: self.appended_bytes,
            personal_index_path: self.personal_index_path,
            personal_index_bytes_before: self.personal_index_bytes_before,
            top_level_index_path: self.top_level_index_path,
            top_level_index_bytes_before: self.top_level_index_bytes_before,
        }
    }
}

impl RecordSnapshot {
    /// Rolls back the workspace mutations represented by this snapshot.
    ///
    /// Always restores all three targets (an adopted snapshot is, by
    /// definition, post-success — the inner transaction had reached
    /// `TopLevelUpdated`). Same suffix-checked semantics as
    /// [`RecordTransaction::rollback`].
    pub fn rollback(self) -> Result<()> {
        restore_for_state(
            TxState::TopLevelUpdated,
            &self.top_level_index_path,
            &self.top_level_index_bytes_before,
            &self.personal_index_path,
            &self.personal_index_bytes_before,
            &self.journal_path,
            &self.appended_bytes,
            self.journal_byte_length_before,
        )
    }
}

/// Replays the rollback steps that `state` says are needed.
///
/// Reverse-order: top-level index, personal index, journal. The journal
/// truncate runs even when an earlier index restore fails, so a concurrent
/// appender's data is detected (drift error returned), but the first error
/// encountered is still propagated to the caller.
#[allow(clippy::too_many_arguments)]
fn restore_for_state(
    state: TxState,
    top_path: &Path,
    top_bytes: &[u8],
    personal_path: &Path,
    personal_bytes: &[u8],
    journal_path: &Path,
    appended_bytes: &[u8],
    journal_len_before: u64,
) -> Result<()> {
    let mut first_error: Option<Error> = None;
    let mut record = |result: Result<()>| {
        if let Err(e) = result
            && first_error.is_none()
        {
            first_error = Some(e);
        }
    };

    if matches!(state, TxState::TopLevelUpdated) {
        record(restore_index(top_path, top_bytes));
    }
    if matches!(state, TxState::TopLevelUpdated | TxState::PersonalUpdated) {
        record(restore_index(personal_path, personal_bytes));
    }
    if matches!(
        state,
        TxState::TopLevelUpdated | TxState::PersonalUpdated | TxState::JournalAppended
    ) {
        record(suffix_check_truncate(
            journal_path,
            appended_bytes,
            journal_len_before,
        ));
    }
    first_error.map_or(Ok(()), Err)
}

/// Restores `bytes` to `path`, deleting the file when bytes are empty and the
/// snapshot recorded "did not exist" (empty Vec).
fn restore_index(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() {
        // Empty snapshot means either the file did not exist, or it was
        // genuinely empty. We cannot distinguish reliably; opt for "delete if
        // present" since a present-but-empty index is invalid in our schema
        // (it must at least contain the managed-block markers).
        if path.exists() {
            std::fs::remove_file(path).map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        parent.ensure_dir()?;
    }
    write_atomic(path, bytes)
}

/// Truncates `journal_path` back to `snapshot_len` iff its tail equals
/// `appended_bytes`.
fn suffix_check_truncate(
    journal_path: &Path,
    appended_bytes: &[u8],
    snapshot_len: u64,
) -> Result<()> {
    if appended_bytes.is_empty() {
        return Ok(());
    }
    let metadata = match std::fs::metadata(journal_path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Journal vanished entirely (user deleted the file). Nothing to
            // truncate; not a drift situation either.
            return Ok(());
        }
        Err(source) => {
            return Err(Error::Io {
                path: journal_path.to_path_buf(),
                source,
            });
        }
    };
    let actual_len = metadata.len();
    let expected_suffix_len = appended_bytes.len() as u64;

    if actual_len < snapshot_len + expected_suffix_len {
        return Err(Error::JournalDriftDetected {
            path: journal_path.to_path_buf(),
            expected_suffix_len,
            actual_len,
        });
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(journal_path)
        .map_err(|source| Error::Io {
            path: journal_path.to_path_buf(),
            source,
        })?;
    file.seek(SeekFrom::End(-(expected_suffix_len as i64)))
        .map_err(|source| Error::Io {
            path: journal_path.to_path_buf(),
            source,
        })?;
    let mut tail = vec![0u8; appended_bytes.len()];
    file.read_exact(&mut tail).map_err(|source| Error::Io {
        path: journal_path.to_path_buf(),
        source,
    })?;
    if tail != appended_bytes {
        return Err(Error::JournalDriftDetected {
            path: journal_path.to_path_buf(),
            expected_suffix_len,
            actual_len,
        });
    }

    file.set_len(snapshot_len).map_err(|source| Error::Io {
        path: journal_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("journal.md");
        let personal = tmp.path().join("personal.md");
        let top = tmp.path().join("top.md");
        std::fs::write(&journal, b"old journal\n").unwrap();
        std::fs::write(&personal, b"old personal\n").unwrap();
        std::fs::write(&top, b"old top\n").unwrap();
        (tmp, journal, personal, top)
    }

    /// Suffix-check rollback restores the file byte-for-byte when no drift.
    #[test]
    fn rollback_truncates_journal_when_tail_matches() {
        let (_tmp, journal, personal, top) = setup();
        let mut tx = RecordTransaction::begin(journal.clone(), personal, top).unwrap();
        let appended = b"NEW ENTRY\n".to_vec();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap();
        std::io::Write::write_all(&mut f, &appended).unwrap();
        tx.record_appended_bytes(appended);
        tx.mark_journal_appended();

        tx.rollback().unwrap();
        assert_eq!(std::fs::read(&journal).unwrap(), b"old journal\n");
    }

    /// Drift detection: another writer appended after this transaction.
    /// Rollback returns `JournalDriftDetected` and leaves file intact.
    #[test]
    fn rollback_detects_drift_and_preserves_concurrent_data() {
        let (_tmp, journal, personal, top) = setup();
        let mut tx = RecordTransaction::begin(journal.clone(), personal, top).unwrap();
        let appended = b"OUR ENTRY\n".to_vec();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap();
        std::io::Write::write_all(&mut f, &appended).unwrap();
        // Simulate concurrent appender:
        std::io::Write::write_all(&mut f, b"CONCURRENT\n").unwrap();
        tx.record_appended_bytes(appended);
        tx.mark_journal_appended();

        let err = tx.rollback().unwrap_err();
        assert!(matches!(err, Error::JournalDriftDetected { .. }));
        // Concurrent data preserved.
        let now = std::fs::read_to_string(&journal).unwrap();
        assert!(now.contains("CONCURRENT"));
        assert!(now.contains("OUR ENTRY"));
    }

    /// Rollback restores personal + top-level indices (TopLevelUpdated state).
    #[test]
    fn rollback_restores_indices_in_reverse() {
        let (_tmp, journal, personal, top) = setup();
        let mut tx =
            RecordTransaction::begin(journal.clone(), personal.clone(), top.clone()).unwrap();
        // Mutate all three.
        let appended = b"X\n".to_vec();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap();
        std::io::Write::write_all(&mut f, &appended).unwrap();
        tx.record_appended_bytes(appended);
        tx.mark_journal_appended();
        std::fs::write(&personal, b"new personal\n").unwrap();
        tx.mark_personal_updated();
        std::fs::write(&top, b"new top\n").unwrap();
        tx.mark_top_level_updated();

        tx.rollback().unwrap();
        assert_eq!(std::fs::read(&journal).unwrap(), b"old journal\n");
        assert_eq!(std::fs::read(&personal).unwrap(), b"old personal\n");
        assert_eq!(std::fs::read(&top).unwrap(), b"old top\n");
    }

    /// `commit` produces a `RecordSnapshot` that can roll back later.
    #[test]
    fn commit_yields_snapshot_with_working_rollback() {
        let (_tmp, journal, personal, top) = setup();
        let mut tx =
            RecordTransaction::begin(journal.clone(), personal.clone(), top.clone()).unwrap();
        let appended = b"S\n".to_vec();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&journal)
            .unwrap();
        std::io::Write::write_all(&mut f, &appended).unwrap();
        tx.record_appended_bytes(appended);
        tx.mark_journal_appended();
        std::fs::write(&personal, b"updated\n").unwrap();
        tx.mark_personal_updated();
        std::fs::write(&top, b"updated top\n").unwrap();
        tx.mark_top_level_updated();
        let snapshot = tx.commit();

        snapshot.rollback().unwrap();
        assert_eq!(std::fs::read(&journal).unwrap(), b"old journal\n");
        assert_eq!(std::fs::read(&personal).unwrap(), b"old personal\n");
        assert_eq!(std::fs::read(&top).unwrap(), b"old top\n");
    }

    /// Rollback at the Open state (no mutations) is a no-op.
    #[test]
    fn rollback_at_open_state_is_noop() {
        let (_tmp, journal, personal, top) = setup();
        let tx = RecordTransaction::begin(journal.clone(), personal.clone(), top.clone()).unwrap();
        tx.rollback().unwrap();
        assert_eq!(std::fs::read(&journal).unwrap(), b"old journal\n");
        assert_eq!(std::fs::read(&personal).unwrap(), b"old personal\n");
        assert_eq!(std::fs::read(&top).unwrap(), b"old top\n");
    }
}
