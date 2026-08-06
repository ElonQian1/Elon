use std::{fmt, time::Instant};

use anyhow::Error;

use crate::{
    node_agent_compute_plugin_host::{
        fetch_contract::{
            AuthorizedComputePluginDownloadSegment, ComputePluginDurableBindPermit,
            ComputePluginFetchClaimRecoveryKey,
        },
        fetch_file::{ComputePluginPinnedFileRecovery, ReconciledComputePluginPartFile},
        root_lock::ComputePluginRootLockLease,
    },
    node_agent_managed_fs::PinnedManagedFile,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginSegmentWritePhase {
    Payload,
    Canceled,
    PrewriteRevalidate,
    Seek,
    Write,
    Flush,
    Sync,
    PostSyncRevalidate,
}

/// Same-handle evidence that bytes reached the claimed end offset and survived fsync plus identity
/// revalidation. It is not commit evidence until a later trusted-time observation is bound.
pub(in crate::node_agent_compute_plugin_host) struct SyncedComputePluginPartFile {
    pub(super) authorized: AuthorizedComputePluginDownloadSegment,
    pub(super) file: PinnedManagedFile,
    pub(super) root_lock_lease: ComputePluginRootLockLease,
    pub(super) sync_completed_at: Instant,
}

impl SyncedComputePluginPartFile {
    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        self.file.identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
        _permit: ComputePluginDurableBindPermit,
    ) -> (
        AuthorizedComputePluginDownloadSegment,
        PinnedManagedFile,
        ComputePluginRootLockLease,
        Instant,
    ) {
        (
            self.authorized,
            self.file,
            self.root_lock_lease,
            self.sync_completed_at,
        )
    }
}

impl fmt::Debug for SyncedComputePluginPartFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SyncedComputePluginPartFile")
            .field("authorized", &"<retained>")
            .field("file", &"<retained>")
            .field("sync_completed_at", &"<monotonic>")
            .finish()
    }
}

/// Before any filesystem mutation the full reconciled capability may be retried. Once directory
/// creation, create-new, truncate or a write syscall may have changed state, only the non-
/// authorizing recovery identity and the exact pinned file survive.
pub(in crate::node_agent_compute_plugin_host) enum ComputePluginSegmentWriteFailure {
    BeforeAnyFilesystemMutation {
        phase: ComputePluginSegmentWritePhase,
        error: Error,
        reconciled: ReconciledComputePluginPartFile,
    },
    RecoveryRequired {
        phase: ComputePluginSegmentWritePhase,
        error: Error,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: ComputePluginPinnedFileRecovery,
    },
}

impl fmt::Debug for ComputePluginSegmentWriteFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BeforeAnyFilesystemMutation { phase, .. } => formatter
                .debug_struct("ComputePluginSegmentWriteFailure")
                .field("phase", phase)
                .field("retained", &"reconciled")
                .finish(),
            Self::RecoveryRequired { phase, .. } => formatter
                .debug_struct("ComputePluginSegmentWriteFailure")
                .field("phase", phase)
                .field("retained", &"recovery_key+file")
                .finish(),
        }
    }
}

impl ComputePluginSegmentWriteFailure {
    pub(super) fn from_reconciled(
        phase: ComputePluginSegmentWritePhase,
        error: impl Into<Error>,
        reconciled: ReconciledComputePluginPartFile,
        current_write_mutation_was_attempted: bool,
    ) -> Self {
        if !reconciled.filesystem_mutated_before_write() && !current_write_mutation_was_attempted {
            return Self::BeforeAnyFilesystemMutation {
                phase,
                error: error.into(),
                reconciled,
            };
        }
        let ReconciledComputePluginPartFile {
            authorized,
            file,
            root_lock_lease,
            ..
        } = reconciled;
        Self::RecoveryRequired {
            phase,
            error: error.into(),
            recovery_key: authorized.into_recovery_key(),
            file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
        }
    }
}
