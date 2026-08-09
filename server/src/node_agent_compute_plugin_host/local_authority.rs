use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::local_authority_schema;

mod candidate_verification_revocation;
mod cleanup_completion_store;
mod cleanup_deletion_domain;
mod cleanup_deletion_guard;
mod cleanup_journal_store;
mod cleanup_store;
mod cleanup_topology_store;
mod fetch_claim_revocation;
mod fetch_store;
mod health_quarantine_store;
mod health_store;
mod initialization;
mod keyring_integrity;
mod keyring_snapshot;
mod keyring_store;
mod manifest_catalog_binding;
mod opened_authority;
mod plan_application;
mod plan_application_persistence;
mod plan_application_projection;
mod plan_application_replay_children;
mod plan_application_writes;
mod process_ownership;
mod rollback_checkpoint;
mod sharing_policy_binding;
#[allow(dead_code)]
mod sqlite_vfs_abi;
#[allow(dead_code, unused_imports)]
mod sqlite_vfs_policy;
mod staging_store;
mod verification_store;

pub(in crate::node_agent_compute_plugin_host) use cleanup_completion_store::{
    ComputePluginCandidateCleanupCompletionAuthorityFacts,
    ComputePluginCandidateCleanupCompletionAuthoritySession,
    ComputePluginCandidateCleanupCompletionRecoveryAuthoritySession,
    ComputePluginCandidateCleanupCompletionRecoveryOutcome,
    HashedComputePluginCandidateCleanupCompletionReceipt,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_CANONICALIZATION,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_DIGEST_ALGORITHM,
    CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
    HASHED_CANDIDATE_CLEANUP_COMPLETION_RECEIPT_SCHEMA,
};
pub(in crate::node_agent_compute_plugin_host) use cleanup_deletion_domain::CandidateCleanupDeletionOperationLease;
pub(in crate::node_agent_compute_plugin_host) use cleanup_deletion_guard::{
    AuthorizedCandidateCleanupDeletionGuard, PreparedCandidateCleanupDeletionGuard,
};
pub(in crate::node_agent_compute_plugin_host) use cleanup_journal_store::{
    ComputePluginCandidateCleanupDeleteIntentAuthoritySession,
    ComputePluginCandidateCleanupDeleteIntentRecoveryAuthoritySession,
    ComputePluginCandidateCleanupDeleteIntentRecoveryOutcome,
    ComputePluginCandidateCleanupDispositionAuthoritySession,
    ComputePluginCandidateCleanupDispositionRecoveryAuthoritySession,
    ComputePluginCandidateCleanupDispositionRecoveryOutcome,
    ComputePluginCandidateCleanupNamespaceDurabilityAuthoritySession,
    ComputePluginCandidateCleanupNamespaceDurabilityRecoveryAuthoritySession,
    ComputePluginCandidateCleanupNamespaceDurabilityRecoveryOutcome,
    ComputePluginCandidateCleanupParentAbsenceAuthoritySession,
    ComputePluginCandidateCleanupParentAbsenceRecoveryAuthoritySession,
    ComputePluginCandidateCleanupParentAbsenceRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use cleanup_store::{
    ComputePluginCandidateCleanupAuthorityFacts, ComputePluginCandidateCleanupAuthoritySession,
    ComputePluginCandidateCleanupRecoveryAuthoritySession,
    ComputePluginCandidateCleanupRecoveryOutcome,
    HashedComputePluginCandidateCleanupAuthorizationReceipt,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_CANONICALIZATION,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_DIGEST_ALGORITHM,
    CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
    HASHED_CANDIDATE_CLEANUP_AUTHORIZATION_RECEIPT_SCHEMA,
};
pub(in crate::node_agent_compute_plugin_host) use cleanup_topology_store::{
    ComputePluginCandidateCleanupTopologyAuthoritySession,
    ComputePluginCandidateCleanupTopologyRecoveryAuthoritySession,
    ComputePluginCandidateCleanupTopologyRecoveryOutcome,
};
pub(in crate::node_agent_compute_plugin_host) use fetch_store::{
    ComputePluginFetchAuthorityFacts, ComputePluginFetchAuthoritySession,
    ComputePluginPostSyncFetchAuthoritySession, ComputePluginPreparedFetchClaimFacts,
};
pub(in crate::node_agent_compute_plugin_host) use health_quarantine_store::{
    ComputePluginCandidateHealthQuarantineAuthorityFacts,
    ComputePluginCandidateHealthQuarantineAuthoritySession,
    ComputePluginCandidateHealthQuarantineRecoveryAuthoritySession,
    ComputePluginCandidateHealthQuarantineRecoveryOutcome,
    HashedComputePluginCandidateHealthQuarantineReceipt,
};
pub(in crate::node_agent_compute_plugin_host) use health_store::{
    ComputePluginCandidateHealthAuthorityFacts, ComputePluginCandidateHealthAuthoritySession,
    ComputePluginCandidateHealthRecoveryAuthoritySession,
    ComputePluginCandidateHealthRecoveryOutcome, HashedComputePluginCandidateHealthReceipt,
};
pub(crate) use initialization::{
    ComputePluginAuthorityInitialization, ComputePluginAuthorityInitializationOutcome,
};
pub(crate) use keyring_store::{
    ComputePluginKeyringInstallDisposition, ComputePluginKeyringInstallResult,
};
pub(in crate::node_agent_compute_plugin_host) use manifest_catalog_binding::{
    ComputePluginManifestCatalogBindingRecovery,
    ComputePluginManifestCatalogBindingRecoveryOutcome,
    ComputePluginManifestCatalogBindingStoreResult, DurableComputePluginManifestCatalogBinding,
    HashedComputePluginManifestCatalogBindingReceipt, RejectedComputePluginManifestCatalogBinding,
};
pub(in crate::node_agent_compute_plugin_host) use opened_authority::{
    ComputePluginHandleBoundAuthorityOpenIntent, OpenedComputePluginLocalAuthority,
};
pub(crate) use plan_application::{
    ComputePluginCandidateHandle, ComputePluginPlanApplicationDisposition,
    ComputePluginPlanApplicationReceipt, ComputePluginPlanApplicationResult,
};
pub(crate) use process_ownership::ComputePluginFetchProcessFence;
pub(in crate::node_agent_compute_plugin_host) use rollback_checkpoint::GuardedLocalComputePluginAuthorityRollbackCheckpointV2;
pub(crate) use rollback_checkpoint::{
    ComputePluginAuthorityRollbackCheckpoint, HashedComputePluginAuthorityRollbackCheckpoint,
    COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA,
    HASHED_COMPUTE_PLUGIN_AUTHORITY_ROLLBACK_CHECKPOINT_SCHEMA,
};
pub(in crate::node_agent_compute_plugin_host) use sharing_policy_binding::{
    ComputePluginSharingPolicyBindingRecovery, ComputePluginSharingPolicyBindingRecoveryOutcome,
    ComputePluginSharingPolicyBindingStoreResult, DurableComputePluginSharingPolicyBinding,
    HashedComputePluginSharingPolicyBindingReceipt,
    HashedComputePluginSharingPolicyCapabilityRevocationReceipt,
    RejectedComputePluginSharingPolicyBinding,
};
pub(in crate::node_agent_compute_plugin_host) use staging_store::{
    ComputePluginCandidateStagingAuthorityFacts,
    ComputePluginCandidateStagingRecoveryAuthoritySession,
    ComputePluginCandidateStagingRecoveryOutcome,
    ComputePluginPostRevalidationStagingAuthoritySession,
    HashedComputePluginCandidateStagingReceipt,
};
pub(in crate::node_agent_compute_plugin_host) use verification_store::{
    ComputePluginCandidateArtifactAuthorityFacts, ComputePluginCandidateVerificationAuthorityFacts,
    ComputePluginCandidateVerificationOutcomeReadFailure,
    ComputePluginCandidateVerificationRecoveryAuthoritySession,
    ComputePluginPostHashVerificationAuthoritySession,
    ComputePluginPostHashVerificationBindingFacts,
    ComputePluginPostPinVerificationAuthoritySession,
    ComputePluginPreparedCandidateVerificationFacts,
};

const COMPUTE_PLUGIN_STATE_FILE: &str = "compute-plugin-state.sqlite3";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Non-serializable identity shared only by capabilities descended from one in-process authority
/// facade. It prevents a process fence or recovery handle from being replayed against another
/// facade with matching scalar facts; it is not a database rollback anchor.
#[derive(Clone)]
pub(in crate::node_agent_compute_plugin_host) struct ComputePluginAuthorityInstanceBinding {
    identity: Arc<()>,
    cleanup_deletion_domain: cleanup_deletion_domain::CandidateCleanupDeletionDomain,
}

impl ComputePluginAuthorityInstanceBinding {
    fn new() -> Self {
        Self {
            identity: Arc::new(()),
            cleanup_deletion_domain: cleanup_deletion_domain::CandidateCleanupDeletionDomain::new(),
        }
    }

    pub(in crate::node_agent_compute_plugin_host) fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }
}

impl PartialEq for ComputePluginAuthorityInstanceBinding {
    fn eq(&self, other: &Self) -> bool {
        self.matches(other)
    }
}

impl Eq for ComputePluginAuthorityInstanceBinding {}

impl fmt::Debug for ComputePluginAuthorityInstanceBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginAuthorityInstanceBinding")
            .field("identity", &"<process-local>")
            .finish()
    }
}

/// Dormant path locator and legacy Store facade for the plugin control-state authority.
///
/// A value of this type is not a root capability and does not prove that SQLite was opened below a
/// pinned root. Its path-opening kernels remain intentionally disconnected from the Host. Future
/// planning and execution code must require `OpenedComputePluginLocalAuthority` instead.
#[derive(Debug)]
pub(crate) struct ComputePluginLocalAuthority {
    path: PathBuf,
    instance_binding: ComputePluginAuthorityInstanceBinding,
}

impl Default for ComputePluginLocalAuthority {
    fn default() -> Self {
        Self::new(crate::node_agent_config::state_path().with_file_name(COMPUTE_PLUGIN_STATE_FILE))
    }
}

impl ComputePluginLocalAuthority {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            instance_binding: ComputePluginAuthorityInstanceBinding::new(),
        }
    }

    /// Derives the dormant authority location below the configured compute-plugin root.
    /// Construction is path-only: the input is not promoted into a pinned-root capability, and
    /// this method does not create directories, open SQLite or install schema.
    pub(in crate::node_agent_compute_plugin_host) fn for_compute_plugin_root(root: &Path) -> Self {
        Self::new(root.join(COMPUTE_PLUGIN_STATE_FILE))
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub(in crate::node_agent_compute_plugin_host) fn instance_binding(
        &self,
    ) -> &ComputePluginAuthorityInstanceBinding {
        &self.instance_binding
    }

    pub(in crate::node_agent_compute_plugin_host) fn is_handle_bound_locator_for(
        &self,
        root: &Path,
    ) -> bool {
        self.path.parent() == Some(root)
            && self.path.file_name() == Some(std::ffi::OsStr::new(COMPUTE_PLUGIN_STATE_FILE))
    }

    /// Legacy path-open seam only. It may create the database or migrate schema and therefore must
    /// not be used by Bootstrap planning, even when the caller intends only to validate schema.
    pub(crate) fn ensure_schema(&self) -> Result<()> {
        self.connect().map(drop)
    }

    /// Only nested authority kernels may use the generic transaction seam. Host and downloader
    /// code receive purpose-specific operations so they cannot partially update authority tables.
    fn with_immediate<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("COMPUTE_PLUGIN_AUTHORITY_BEGIN_IMMEDIATE")?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .context("COMPUTE_PLUGIN_AUTHORITY_COMMIT")?;
        Ok(value)
    }

    /// Opens a stable deferred transaction for legacy nested readers. `connect()` may still create
    /// directories, switch journal mode or install/migrate schema before this transaction begins,
    /// so this is not a side-effect-free seam and must not back a planning snapshot producer.
    fn with_deferred<T>(&self, operation: impl FnOnce(&Transaction<'_>) -> Result<T>) -> Result<T> {
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .context("COMPUTE_PLUGIN_AUTHORITY_BEGIN_DEFERRED")?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .context("COMPUTE_PLUGIN_AUTHORITY_READ_COMMIT")?;
        Ok(value)
    }

    fn connect(&self) -> Result<Connection> {
        let parent = self.path.parent().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_PATH: database has no parent directory")
        })?;
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "COMPUTE_PLUGIN_AUTHORITY_DIRECTORY: cannot create {}",
                parent.display()
            )
        })?;
        let mut connection = Connection::open(&self.path).with_context(|| {
            format!(
                "COMPUTE_PLUGIN_AUTHORITY_OPEN: cannot open {}",
                self.path.display()
            )
        })?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .context("COMPUTE_PLUGIN_AUTHORITY_BUSY_TIMEOUT")?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .context("COMPUTE_PLUGIN_AUTHORITY_WAL")?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .context("COMPUTE_PLUGIN_AUTHORITY_SYNCHRONOUS")?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS")?;
        let foreign_keys = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS_READ")?;
        if foreign_keys != 1 {
            bail!("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS_DISABLED");
        }
        connection
            .pragma_update(None, "trusted_schema", "OFF")
            .context("COMPUTE_PLUGIN_AUTHORITY_TRUSTED_SCHEMA")?;
        local_authority_schema::ensure_schema(&mut connection)?;
        Ok(connection)
    }
}
