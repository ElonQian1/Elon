use std::fmt;

use super::super::root_lock::ComputePluginRootLockLease;
use crate::node_agent_managed_fs::{PinnedManagedFile, QuarantinedManagedFile};

/// Non-writable recovery custody for an exact pinned file. It intentionally exposes neither the
/// raw handle nor an unwrap operation; future recovery transitions must be implemented beside this
/// type and consume it through a purpose-specific API.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginPinnedFileRecovery {
    file: PinnedManagedFile,
    _root_lock: ComputePluginRootLockLease,
}

impl ComputePluginPinnedFileRecovery {
    pub(in crate::node_agent_compute_plugin_host) fn from_pinned(
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self {
            file,
            _root_lock: root_lock_lease,
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn file_identity_digest(&self) -> &str {
        self.file.identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn len_bytes(&self) -> u64 {
        self.file.len_bytes()
    }
}

/// Recovery-only custody for an opened file that failed managed-file validation. Keeping the root
/// lease inside the same opaque value prevents a caller from moving the quarantined handle away
/// from cross-process exclusion.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginQuarantinedFileRecovery {
    _file: QuarantinedManagedFile,
    _root_lock: ComputePluginRootLockLease,
}

impl ComputePluginQuarantinedFileRecovery {
    pub(super) fn new(
        file: QuarantinedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self {
            _file: file,
            _root_lock: root_lock_lease,
        }
    }
}

/// Opaque XOR custody for an uncertain cursor-damage Store outcome. A shorter file retains the
/// exact pinned handle and its root lock together; a missing file retains the root lock by itself.
/// No host sibling can construct a state with neither form of exclusion.
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginCursorDamageRecoveryCustody {
    inner: ComputePluginCursorDamageRecoveryCustodyInner,
}

enum ComputePluginCursorDamageRecoveryCustodyInner {
    Pinned {
        _file: ComputePluginPinnedFileRecovery,
    },
    Missing {
        _root_lock: ComputePluginRootLockLease,
    },
}

impl ComputePluginCursorDamageRecoveryCustody {
    pub(super) fn pinned(
        file: PinnedManagedFile,
        root_lock_lease: ComputePluginRootLockLease,
    ) -> Self {
        Self {
            inner: ComputePluginCursorDamageRecoveryCustodyInner::Pinned {
                _file: ComputePluginPinnedFileRecovery::from_pinned(file, root_lock_lease),
            },
        }
    }

    pub(super) fn missing(root_lock_lease: ComputePluginRootLockLease) -> Self {
        Self {
            inner: ComputePluginCursorDamageRecoveryCustodyInner::Missing {
                _root_lock: root_lock_lease,
            },
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn has_pinned_file(&self) -> bool {
        matches!(
            &self.inner,
            ComputePluginCursorDamageRecoveryCustodyInner::Pinned { .. }
        )
    }
}

impl fmt::Debug for ComputePluginCursorDamageRecoveryCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginCursorDamageRecoveryCustody")
            .field("pinned_file", &self.has_pinned_file())
            .finish()
    }
}

impl fmt::Debug for ComputePluginQuarantinedFileRecovery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginQuarantinedFileRecovery")
            .field("file", &"<retained-quarantined>")
            .finish()
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
