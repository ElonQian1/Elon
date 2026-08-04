use std::fmt;

use anyhow::Error;

use crate::{
    node_agent_compute_plugin_host::fetch_contract::{
        AuthorizedComputePluginDownloadSegment, ComputePluginFetchClaimRecoveryKey,
    },
    node_agent_managed_fs::{PinnedManagedFile, PinnedManagedRoot, QuarantinedManagedFile},
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
    pub(super) installation_id_digest: String,
}

/// Non-writable recovery custody for an exact pinned file. It intentionally exposes neither the
/// raw handle nor an unwrap operation; future recovery transitions must be implemented beside this
/// type and consume it through a purpose-specific API.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPinnedFileRecovery {
    file: PinnedManagedFile,
}

impl ComputePluginPinnedFileRecovery {
    pub(in crate::node_agent_compute_plugin_host) fn from_pinned(file: PinnedManagedFile) -> Self {
        Self { file }
    }

    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        self.file.identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn len_bytes(&self) -> u64 {
        self.file.len_bytes()
    }
}

impl fmt::Debug for ComputePluginPinnedFileRecovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPinnedFileRecovery")
            .field("file", &"<retained-non-writable>")
            .finish()
    }
}

impl fmt::Debug for PinnedComputePluginRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedComputePluginRoot")
            .field("root", &"<retained>")
            .field("installation_id_digest", &"<redacted>")
            .finish()
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
        file: QuarantinedManagedFile,
    },
    UnreconciledFile {
        error: Error,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: ComputePluginPinnedFileRecovery,
    },
    RecoveryRequiredWithoutFile {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    },
    QuarantinedFileRecoveryRequired {
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: QuarantinedManagedFile,
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
    ) -> Self {
        Self::OpenedFileRejected {
            error,
            authorized,
            file,
        }
    }

    pub(super) fn unreconciled(
        error: impl Into<Error>,
        authorized: AuthorizedComputePluginDownloadSegment,
        file: PinnedManagedFile,
    ) -> Self {
        Self::UnreconciledFile {
            error: error.into(),
            authorized,
            file: ComputePluginPinnedFileRecovery::from_pinned(file),
        }
    }

    pub(super) fn recovery_without_file(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
    ) -> Self {
        Self::RecoveryRequiredWithoutFile {
            error: error.into(),
            recovery_key,
        }
    }

    pub(super) fn quarantined_recovery(
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: QuarantinedManagedFile,
    ) -> Self {
        Self::QuarantinedFileRecoveryRequired {
            error,
            recovery_key,
            file,
        }
    }

    pub(super) fn unexpected_existing(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: PinnedManagedFile,
    ) -> Self {
        Self::UnexpectedExistingZeroCursorFile {
            error: error.into(),
            recovery_key,
            file: ComputePluginPinnedFileRecovery::from_pinned(file),
        }
    }

    pub(super) fn file_recovery(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: PinnedManagedFile,
    ) -> Self {
        Self::FileRecoveryRequired {
            error: error.into(),
            recovery_key,
            file: ComputePluginPinnedFileRecovery::from_pinned(file),
        }
    }
}
