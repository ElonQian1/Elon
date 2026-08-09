use std::fmt;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::ComputePluginAuthorityInstanceBinding;
use crate::{
    node_agent_compute_plugin_host::root_lock::ComputePluginRootLockLease,
    node_agent_instance_lock::NodeAgentInstanceLockLease,
};

const HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE: &str =
    "COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE";

/// Linear prerequisites for one future handle-bound authority open.
///
/// This value retains the exact root-lock and NodeAgent instance-lock handles while binding the
/// authority filename to one pinned installation/root identity. It deliberately contains neither
/// a SQLite connection nor an operating-system handle for the database file, so possessing it is
/// not proof that the authority is open. Only a verified handle-bound SQLite VFS may turn it into
/// [`OpenedComputePluginLocalAuthority`]. There is deliberately no constructor in this batch: a
/// future Bootstrap controller must mint this value atomically from one exact NodeDataPaths/root,
/// authority-instance and NodeAgent-instance-lock witness instead of accepting an arbitrary lease.
#[must_use = "dropping the open intent releases its retained authority locks"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginHandleBoundAuthorityOpenIntent {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    root_identity_digest: String,
    authority_file_name: &'static str,
    root_lock_lease: ComputePluginRootLockLease,
    instance_lock_lease: NodeAgentInstanceLockLease,
}

impl ComputePluginHandleBoundAuthorityOpenIntent {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_file_name(&self) -> &str {
        self.authority_file_name
    }

    /// Production remains fail-closed until a registered SQLite VFS can consume the already
    /// pinned parent capability and own every main/journal/WAL/SHM file handle. This method must
    /// never fall back to a rusqlite path opener, a canonical path, or a post-open FileId check.
    pub(in crate::node_agent_compute_plugin_host) fn open(
        self,
    ) -> Result<OpenedComputePluginLocalAuthority> {
        bail!(HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE)
    }
}

impl fmt::Debug for ComputePluginHandleBoundAuthorityOpenIntent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginHandleBoundAuthorityOpenIntent")
            .field("authority_instance", &"<process-local>")
            .field("installation_id_digest", &"<redacted>")
            .field("root_identity_digest", &"<redacted>")
            .field("authority_file_name", &self.authority_file_name)
            .field("root_lock", &"<retained>")
            .field("instance_lock", &"<retained>")
            .finish()
    }
}

/// An authority connection whose SQLite file lifecycle has already been proven handle-bound.
///
/// There is intentionally no production constructor while the repository lacks the required VFS.
/// The private backend prevents a path-opened or in-memory `Connection` from being wrapped and
/// misrepresented as durable authority. The retained locks outlive the connection; field order is
/// deliberate so the SQLite connection is dropped before either lock lease.
#[must_use = "dropping the opened authority closes SQLite before releasing its retained locks"]
pub(in crate::node_agent_compute_plugin_host) struct OpenedComputePluginLocalAuthority {
    backend: SealedHandleBoundSqliteBackend,
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    root_identity_digest: String,
    authority_file_name: &'static str,
    _root_lock_lease: ComputePluginRootLockLease,
    _instance_lock_lease: NodeAgentInstanceLockLease,
}

impl OpenedComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        &self.installation_id_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        &self.root_identity_digest
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_file_name(&self) -> &str {
        self.authority_file_name
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }

    /// Runs one purpose-specific write kernel on the already-open handle-bound connection. This
    /// method never opens a filename or installs schema; only the private VFS lifecycle may create
    /// an `OpenedComputePluginLocalAuthority` in the first place.
    pub(super) fn with_immediate<T>(
        &mut self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let transaction = self
            .backend
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("COMPUTE_PLUGIN_OPENED_AUTHORITY_BEGIN_IMMEDIATE")?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .context("COMPUTE_PLUGIN_OPENED_AUTHORITY_COMMIT")?;
        Ok(value)
    }

    /// Reserved for the future VFS implementation in this private module. Requiring both the
    /// sealed backend and the linear intent prevents other modules from blessing an arbitrary
    /// `Connection` as handle-bound.
    #[allow(dead_code)]
    fn from_verified_backend(
        backend: SealedHandleBoundSqliteBackend,
        intent: ComputePluginHandleBoundAuthorityOpenIntent,
    ) -> Self {
        Self {
            backend,
            authority_instance_binding: intent.authority_instance_binding,
            installation_id_digest: intent.installation_id_digest,
            root_identity_digest: intent.root_identity_digest,
            authority_file_name: intent.authority_file_name,
            _root_lock_lease: intent.root_lock_lease,
            _instance_lock_lease: intent.instance_lock_lease,
        }
    }
}

impl fmt::Debug for OpenedComputePluginLocalAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedComputePluginLocalAuthority")
            .field("backend", &"<handle-bound-sqlite>")
            .field("authority_instance", &"<process-local>")
            .field("installation_id_digest", &"<redacted>")
            .field("root_identity_digest", &"<redacted>")
            .field("authority_file_name", &self.authority_file_name)
            .field("root_lock", &"<retained>")
            .field("instance_lock", &"<retained>")
            .finish()
    }
}

/// Private on purpose: rusqlite's raw `Connection` type does not prove how SQLite opened its main
/// database or sidecars. A future VFS implementation in this module must construct this wrapper
/// only after consuming an opaque, one-shot handle registry entry and validating the VFS result.
#[allow(dead_code)]
struct SealedHandleBoundSqliteBackend {
    connection: Connection,
}
