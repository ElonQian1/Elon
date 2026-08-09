use std::{fmt, path::Path};

use anyhow::{bail, Error, Result};
use elon_pc_dev_runtime::NodeDataPaths;

use super::recovery::{
    ComputePluginCursorDamageRecoveryCustody, ComputePluginPinnedFileRecovery,
    ComputePluginQuarantinedFileRecovery,
};

use crate::{
    node_agent_compute_plugin_host::fetch_contract::{
        AuthorizedComputePluginDownloadSegment, ComputePluginFetchClaimRecoveryKey,
    },
    node_agent_compute_plugin_host::root_lock::{
        ComputePluginRootLock, ComputePluginRootLockLease,
    },
    node_agent_managed_fs::{
        ManagedFileOpenFailure, PinnedManagedDirectory, PinnedManagedFile, PinnedManagedRoot,
        PinnedManagedSqliteNamespace, QuarantinedManagedFile,
    },
};

pub(in crate::node_agent_compute_plugin_host) type ComputePluginPartReconcileResult =
    std::result::Result<ComputePluginPartReconcileOutcome, ComputePluginPartReconcileFailure>;

pub(in crate::node_agent_compute_plugin_host) enum ComputePluginPartReconcileOutcome {
    Ready(ReconciledComputePluginPartFile),
    CursorDamaged(ComputePluginPartCursorDamage),
}

/// Long-lived bootstrap capability for one installation-owned data root. The Host must construct
/// it after the NodeAgent instance lock and retain it across claims; claim-time code never reopens
/// the root or marker by path.
pub(in crate::node_agent_compute_plugin_host) struct PinnedComputePluginRoot {
    pub(super) root: PinnedManagedRoot,
    pub(super) root_lock: ComputePluginRootLock,
    pub(super) installation_id_digest: String,
    pub(super) node_data_paths: NodeDataPaths,
}

/// Sealed namespace proof derived from one pinned compute-plugin root while its exact root-lock
/// lease remains retained. It intentionally exposes no SQLite file operations or inner namespace.
pub(in crate::node_agent_compute_plugin_host) struct PinnedComputePluginAuthoritySqliteNamespace {
    pub(super) _namespace: PinnedManagedSqliteNamespace,
    pub(super) _root_lock_lease: ComputePluginRootLockLease,
}

impl PinnedComputePluginRoot {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        self.root.root_identity_digest()
    }

    /// Exact configured path model consumed by the private pin constructor. This process-local
    /// binding proves that a root lock from another data root cannot authorize a path-based
    /// authority facade, even when both roots use the same installation identity.
    pub(in crate::node_agent_compute_plugin_host) fn node_data_paths(&self) -> &NodeDataPaths {
        &self.node_data_paths
    }

    pub(in crate::node_agent_compute_plugin_host) fn compute_plugin_root(
        &self,
    ) -> std::path::PathBuf {
        self.node_data_paths.compute_plugins()
    }

    /// Mints one linear lease for a capability that must keep the already-acquired root lock alive.
    /// It conveys no path-open or mutation authority by itself.
    pub(in crate::node_agent_compute_plugin_host) fn root_lock_lease(
        &self,
    ) -> ComputePluginRootLockLease {
        self.root_lock.lease()
    }
}

/// Existing-only capability for one candidate's downloads directory. It can open only direct
/// children whose original normalized relative path names this exact candidate directory.
pub(in crate::node_agent_compute_plugin_host) struct PinnedComputePluginCandidateDownloads {
    pub(super) directory: PinnedManagedDirectory,
    pub(super) root_lock_lease: ComputePluginRootLockLease,
    pub(super) relative_directory: String,
    pub(super) installation_id_digest: String,
    pub(super) root_identity_digest: String,
}

impl PinnedComputePluginCandidateDownloads {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn open_existing_artifact(
        &self,
        original_relative_path: &str,
    ) -> Result<PinnedManagedFile> {
        if original_relative_path.contains('\\') {
            bail!("COMPUTE_PLUGIN_VERIFICATION_PATH_NOT_CANONICAL");
        }
        let relative = Path::new(original_relative_path);
        let parent_matches = relative
            .parent()
            .is_some_and(|parent| parent == Path::new(&self.relative_directory));
        let file_name = relative
            .file_name()
            .filter(|_| parent_matches)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_VERIFICATION_FILE_PATH_CHANGED"))?;
        self.directory
            .open_existing_read_only_cleanup_child(file_name)
            .map_err(managed_open_error)
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_cleanup_parts(
        self,
    ) -> (PinnedManagedDirectory, ComputePluginRootLockLease) {
        (self.directory, self.root_lock_lease)
    }
}

impl fmt::Debug for PinnedComputePluginCandidateDownloads {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedComputePluginCandidateDownloads")
            .field("directory", &"<retained>")
            .field("binding", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for PinnedComputePluginRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedComputePluginRoot")
            .field("root", &"<retained>")
            .field("root_lock", &self.root_lock)
            .field("installation_id_digest", &"<redacted>")
            .finish()
    }
}

fn managed_open_error(failure: ManagedFileOpenFailure) -> Error {
    match failure {
        ManagedFileOpenFailure::NotOpened(error)
        | ManagedFileOpenFailure::FileNotOpened { error, .. } => Error::new(error),
        ManagedFileOpenFailure::Opened { error, .. } => error,
    }
}

impl fmt::Debug for ComputePluginPartReconcileOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ready(ready) => formatter.debug_tuple("Ready").field(ready).finish(),
            Self::CursorDamaged(damaged) => formatter
                .debug_tuple("CursorDamaged")
                .field(damaged)
                .finish(),
        }
    }
}

/// Linear capability proving the exact claim file is pinned and its same-handle length equals the
/// committed Store cursor. It deliberately exposes no raw File or write interface.
pub(in crate::node_agent_compute_plugin_host) struct ReconciledComputePluginPartFile {
    pub(super) authorized: AuthorizedComputePluginDownloadSegment,
    pub(super) file: PinnedManagedFile,
    pub(super) root_lock_lease: ComputePluginRootLockLease,
    pub(super) truncated_uncommitted_tail: bool,
}

impl ReconciledComputePluginPartFile {
    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        self.file.identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn committed_offset(&self) -> i64 {
        self.authorized.offset_bytes()
    }

    pub(in crate::node_agent_compute_plugin_host) fn truncated_uncommitted_tail(&self) -> bool {
        self.truncated_uncommitted_tail
    }

    pub(super) fn filesystem_mutated_before_write(&self) -> bool {
        self.authorized.offset_bytes() == 0
            || self.truncated_uncommitted_tail
            || self.file.directory_filesystem_mutated()
    }
}

impl fmt::Debug for ReconciledComputePluginPartFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReconciledComputePluginPartFile")
            .field("authorized", &"<retained>")
            .field("file", &"<retained>")
            .field(
                "truncated_uncommitted_tail",
                &self.truncated_uncommitted_tail,
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginPartCursorDamageKind {
    MissingCommittedFile,
    ShorterThanCommittedCursor,
}

/// Fail-closed evidence for a file that cannot satisfy the persisted cursor. This is not a writable
/// capability and must later be consumed by the Store damage transition; it never rolls the cursor
/// backward or fills the gap with zeroes.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPartCursorDamage {
    pub(super) kind: ComputePluginPartCursorDamageKind,
    pub(super) authorized: AuthorizedComputePluginDownloadSegment,
    pub(super) file: Option<PinnedManagedFile>,
    pub(super) root_lock_lease: ComputePluginRootLockLease,
    pub(super) observed_length_bytes: Option<i64>,
}

impl ComputePluginPartCursorDamage {
    pub(in crate::node_agent_compute_plugin_host) fn kind(
        &self,
    ) -> ComputePluginPartCursorDamageKind {
        self.kind
    }

    pub(in crate::node_agent_compute_plugin_host) fn observed_length_bytes(&self) -> Option<i64> {
        self.observed_length_bytes
    }

    pub(in crate::node_agent_compute_plugin_host) fn authorized(
        &self,
    ) -> &AuthorizedComputePluginDownloadSegment {
        &self.authorized
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_exact_evidence(
        &mut self,
    ) -> Result<()> {
        if self.authorized.offset_bytes() <= 0 {
            bail!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_OFFSET_INVALID");
        }
        match self.kind {
            ComputePluginPartCursorDamageKind::MissingCommittedFile => {
                if self.file.is_some() || self.observed_length_bytes.is_some() {
                    bail!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_MISSING_EVIDENCE_INVALID");
                }
            }
            ComputePluginPartCursorDamageKind::ShorterThanCommittedCursor => {
                let observed = self.observed_length_bytes.ok_or_else(|| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_LENGTH_MISSING")
                })?;
                if observed < 0 || observed >= self.authorized.offset_bytes() {
                    bail!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_LENGTH_INVALID");
                }
                let expected = u64::try_from(observed).map_err(|_| {
                    anyhow::anyhow!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_LENGTH_RANGE")
                })?;
                self.file
                    .as_mut()
                    .ok_or_else(|| {
                        anyhow::anyhow!("COMPUTE_PLUGIN_PART_CURSOR_DAMAGE_FILE_MISSING")
                    })?
                    .revalidate_exact_len(expected)?;
            }
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery_custody(
        self,
    ) -> (
        ComputePluginFetchClaimRecoveryKey,
        ComputePluginCursorDamageRecoveryCustody,
    ) {
        let recovery_key = self.authorized.into_recovery_key();
        let custody = match self.file {
            Some(file) => {
                ComputePluginCursorDamageRecoveryCustody::pinned(file, self.root_lock_lease)
            }
            None => ComputePluginCursorDamageRecoveryCustody::missing(self.root_lock_lease),
        };
        (recovery_key, custody)
    }
}

impl fmt::Debug for ComputePluginPartCursorDamage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPartCursorDamage")
            .field("kind", &self.kind)
            .field("authorized", &"<retained>")
            .field("file", &self.file.as_ref().map(|_| "<retained>"))
            .field("observed_length_bytes", &self.observed_length_bytes)
            .finish()
    }
}

/// Typed ownership boundary around all file-open/reconcile failures. Before mutation the exact
/// authorization is retained. Once create/truncate/sync may have changed the filesystem, only a
/// non-authorizing recovery key and the same open file handle survive.
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginPartReconcileFailure {
    BeforeFileMutation {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
    },
    OpenedFileRejected {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: ComputePluginQuarantinedFileRecovery,
    },
    UnreconciledFile {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: ComputePluginPinnedFileRecovery,
    },
    RecoveryRequiredWithoutFile {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        root_lock_lease: ComputePluginRootLockLease,
    },
    QuarantinedFileRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: ComputePluginQuarantinedFileRecovery,
    },
    UnexpectedExistingZeroCursorFile {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: ComputePluginPinnedFileRecovery,
    },
    FileRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: ComputePluginPinnedFileRecovery,
    },
}

impl fmt::Debug for ComputePluginPartReconcileFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (phase, retained) = match self {
            Self::BeforeFileMutation { .. } => ("before_file_mutation", "authorized"),
            Self::OpenedFileRejected { .. } => ("opened_file_rejected", "authorized+file"),
            Self::UnreconciledFile { .. } => ("unreconciled_file", "authorized+file"),
            Self::RecoveryRequiredWithoutFile { .. } => {
                ("recovery_required_without_file", "recovery_key")
            }
            Self::QuarantinedFileRecoveryRequired { .. } => {
                ("quarantined_file_recovery_required", "recovery_key+file")
            }
            Self::UnexpectedExistingZeroCursorFile { .. } => {
                ("unexpected_existing_zero_cursor_file", "recovery_key+file")
            }
            Self::FileRecoveryRequired { .. } => ("file_recovery_required", "recovery_key+file"),
        };
        formatter
            .debug_struct("ComputePluginPartReconcileFailure")
            .field("phase", &phase)
            .field("retained", &retained)
            .finish()
    }
}

impl ComputePluginPartReconcileFailure {
    pub(super) fn before(
        error: impl Into<Error>,
        authorized: AuthorizedComputePluginDownloadSegment,
    ) -> Self {
        Self::BeforeFileMutation {
            error: error.into(),
            authorized,
        }
    }

    pub(super) fn opened_rejected(
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: QuarantinedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::OpenedFileRejected {
            error,
            authorized,
            file: ComputePluginQuarantinedFileRecovery::new(file, root_lock_lease),
        }
    }

    pub(super) fn unreconciled(
        error: impl Into<Error>,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::UnreconciledFile {
            error: error.into(),
            authorized,
            file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
        }
    }

    pub(super) fn recovery_without_file(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::RecoveryRequiredWithoutFile {
            error: error.into(),
            recovery_key,
            root_lock_lease,
        }
    }

    pub(super) fn quarantined_recovery(
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: QuarantinedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::QuarantinedFileRecoveryRequired {
            error,
            recovery_key,
            file: ComputePluginQuarantinedFileRecovery::new(file, root_lock_lease),
        }
    }

    pub(super) fn unexpected_existing(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::UnexpectedExistingZeroCursorFile {
            error: error.into(),
            recovery_key,
            file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
        }
    }

    pub(super) fn file_recovery(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self::FileRecoveryRequired {
            error: error.into(),
            recovery_key,
            file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
        }
    }
}
