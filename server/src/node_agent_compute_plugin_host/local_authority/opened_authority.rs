use std::fmt;

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::sqlite_vfs_policy::ManagedSqliteRegistryCustody;
use super::{ComputePluginAuthorityInstanceBinding, COMPUTE_PLUGIN_STATE_FILE};
use crate::node_agent_compute_plugin_host::bootstrap::PinnedAuthorityOpenCustody;

const HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE: &str =
    "COMPUTE_PLUGIN_HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE";

/// Linear prerequisites for one future handle-bound authority open.
///
/// This value retains the complete, unforgeable Bootstrap controller together with the exact
/// four-name namespace derived from its pinned root. It deliberately contains no SQLite
/// connection, so possessing it is not proof that the authority is open. Only a verified
/// handle-bound SQLite VFS may turn it into [`OpenedComputePluginLocalAuthority`].
#[must_use = "dropping the open intent releases its retained authority locks"]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginHandleBoundAuthorityOpenIntent {
    custody: PinnedAuthorityOpenCustody,
    authority_file_name: &'static str,
}

impl ComputePluginHandleBoundAuthorityOpenIntent {
    /// The constructor accepts only sealed custody minted by the linear controller. It never
    /// accepts a caller-supplied directory, namespace, lock lease, witness or scalar identity.
    pub(in crate::node_agent_compute_plugin_host) fn from_controller_custody(
        custody: PinnedAuthorityOpenCustody,
    ) -> Result<Self> {
        custody.ensure_current()?;
        Ok(Self {
            custody,
            authority_file_name: COMPUTE_PLUGIN_STATE_FILE,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.custody.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        self.custody.root_identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_file_name(&self) -> &str {
        self.authority_file_name
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.custody.authority_instance_binding()
    }

    fn ensure_current(&self) -> Result<()> {
        self.custody.ensure_current()
    }

    /// Production remains fail-closed until a registered SQLite VFS can consume the already
    /// pinned parent capability and own every main/journal/WAL/SHM file handle. This method must
    /// never fall back to a rusqlite path opener, a canonical path, or a post-open FileId check.
    pub(in crate::node_agent_compute_plugin_host) fn open(
        self,
    ) -> Result<OpenedComputePluginLocalAuthority> {
        self.ensure_current()?;
        bail!(HANDLE_BOUND_SQLITE_VFS_UNAVAILABLE)
    }
}

impl Drop for ComputePluginHandleBoundAuthorityOpenIntent {
    fn drop(&mut self) {
        // Publish terminal revocation before automatic field destruction releases the namespace,
        // root lock or configured NodeAgent instance-lock lease.
        self.custody.retire();
    }
}

impl ManagedSqliteRegistryCustody for ComputePluginHandleBoundAuthorityOpenIntent {
    fn ensure_registry_current(&self) -> Result<()> {
        self.ensure_current()
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
    intent: ComputePluginHandleBoundAuthorityOpenIntent,
}

impl OpenedComputePluginLocalAuthority {
    pub(in crate::node_agent_compute_plugin_host) fn installation_id_digest(&self) -> &str {
        self.intent.installation_id_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn root_identity_digest(&self) -> &str {
        self.intent.root_identity_digest()
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_file_name(&self) -> &str {
        self.intent.authority_file_name()
    }

    pub(in crate::node_agent_compute_plugin_host) fn authority_instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        self.intent.authority_instance_binding()
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
        Self { backend, intent }
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
            .field("authority_file_name", &self.intent.authority_file_name())
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
