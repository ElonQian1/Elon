use std::fmt;

use anyhow::Error;

use crate::{
    node_agent_compute_plugin_host::{
        fetch_contract::ComputePluginFetchClaimRecoveryKey,
        fetch_file::ComputePluginPinnedFileRecovery, root_lock::ComputePluginRootLockLease,
    },
    node_agent_managed_fs::PinnedManagedFile,
};

/// One-shot seal minted only by the durable binder. It lets the synced-file module release its
/// private parts to that binder without exposing a general-purpose unwrap operation to the Host.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginDurableBindPermit {
    _private: (),
}

impl ComputePluginDurableBindPermit {
    pub(super) fn new() -> Self {
        Self { _private: () }
    }
}

/// Fsync already happened before this boundary, so every binding failure consumes the mutation
/// authorization and retains only stable recovery identity plus the same pinned file handle.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPostSyncBindingFailure {
    pub error: Error,
    pub recovery_key: ComputePluginFetchClaimRecoveryKey,
    pub file: ComputePluginPinnedFileRecovery,
}

impl fmt::Debug for ComputePluginPostSyncBindingFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginPostSyncBindingFailure")
            .field("error", &"<redacted>")
            .field("recovery_key", &"<redacted>")
            .field("file", &"<retained>")
            .finish()
    }
}

impl ComputePluginPostSyncBindingFailure {
    pub(super) fn new(
        error: impl Into<Error>,
        recovery_key: ComputePluginFetchClaimRecoveryKey,
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self {
            error: error.into(),
            recovery_key,
            file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
        }
    }
}
