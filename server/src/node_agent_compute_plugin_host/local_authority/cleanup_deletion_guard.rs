use std::{
    fmt,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::{bail, Result};

use super::{
    cleanup_deletion_domain::{
        CandidateCleanupDeletionDomain, CandidateCleanupDeletionOperationLease,
    },
    ComputePluginAuthorityInstanceBinding, ComputePluginFetchProcessFence,
    HashedComputePluginCandidateCleanupAuthorizationReceipt,
};
use crate::node_agent_compute_plugin_host::{
    install_plan_admission_validation::is_identifier, manifest_validation::is_sha256,
};

#[must_use = "prepared cleanup deletion custody must be authorized or retained"]
pub(in crate::node_agent_compute_plugin_host) struct PreparedCandidateCleanupDeletionGuard {
    binding: CandidateCleanupDeletionBinding,
    domain: CandidateCleanupDeletionDomain,
    fence_liveness: Arc<AtomicBool>,
    generation: u64,
}

#[must_use = "authorized cleanup deletion custody must remain in the cleanup chain"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedCandidateCleanupDeletionGuard {
    binding: CandidateCleanupDeletionBinding,
    authorization_receipt_digest: String,
    domain: CandidateCleanupDeletionDomain,
    fence_liveness: Arc<AtomicBool>,
    generation: u64,
}

struct CandidateCleanupDeletionBinding {
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
    installation_id_digest: String,
    process_owner_epoch: i64,
    cleanup_id: String,
    candidate_token_digest: String,
    quarantine_id: String,
    quarantine_receipt_digest: String,
    staging_id: String,
    staging_run_digest: String,
    root_identity_digest: String,
}

impl ComputePluginFetchProcessFence {
    pub(in crate::node_agent_compute_plugin_host) fn prepare_candidate_cleanup_deletion_guard(
        &self,
        cleanup_id: String,
        candidate_token_digest: String,
        quarantine_id: String,
        quarantine_receipt_digest: String,
        staging_id: String,
        staging_run_digest: String,
        root_identity_digest: String,
    ) -> Result<PreparedCandidateCleanupDeletionGuard> {
        let authority_instance_binding = self.authority_instance_binding();
        let generation = authority_instance_binding.cleanup_deletion_domain.capture(
            self.process_owner_epoch(),
            &self.cleanup_deletion_fence_liveness,
        )?;
        if !is_identifier(&cleanup_id)
            || !is_sha256(&candidate_token_digest)
            || !is_identifier(&quarantine_id)
            || !is_sha256(&quarantine_receipt_digest)
            || !is_identifier(&staging_id)
            || !is_sha256(&staging_run_digest)
            || !is_sha256(&root_identity_digest)
        {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_PREPARED_BINDING_INVALID");
        }
        Ok(PreparedCandidateCleanupDeletionGuard {
            binding: CandidateCleanupDeletionBinding {
                authority_instance_binding: authority_instance_binding.clone(),
                installation_id_digest: self.installation_id_digest().to_string(),
                process_owner_epoch: self.process_owner_epoch(),
                cleanup_id,
                candidate_token_digest,
                quarantine_id,
                quarantine_receipt_digest,
                staging_id,
                staging_run_digest,
                root_identity_digest,
            },
            domain: authority_instance_binding.cleanup_deletion_domain.clone(),
            fence_liveness: Arc::clone(&self.cleanup_deletion_fence_liveness),
            generation,
        })
    }

    pub(super) fn cleanup_deletion_fence_liveness(&self) -> &Arc<AtomicBool> {
        &self.cleanup_deletion_fence_liveness
    }
}

impl PreparedCandidateCleanupDeletionGuard {
    pub(in crate::node_agent_compute_plugin_host) fn authorize(
        self,
        receipt: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
    ) -> std::result::Result<
        AuthorizedCandidateCleanupDeletionGuard,
        (anyhow::Error, PreparedCandidateCleanupDeletionGuard),
    > {
        if let Err(error) = self
            .ensure_current()
            .and_then(|()| validate_receipt_binding(&self.binding, receipt))
        {
            return Err((error, self));
        }
        Ok(AuthorizedCandidateCleanupDeletionGuard {
            binding: self.binding,
            authorization_receipt_digest: receipt.receipt_digest().to_string(),
            domain: self.domain,
            fence_liveness: self.fence_liveness,
            generation: self.generation,
        })
    }

    pub(in crate::node_agent_compute_plugin_host) fn enter_operation(
        &self,
    ) -> Result<CandidateCleanupDeletionOperationLease> {
        self.domain.enter_operation(
            &self.fence_liveness,
            self.binding.process_owner_epoch,
            self.generation,
        )
    }

    pub(in crate::node_agent_compute_plugin_host) fn ensure_current(&self) -> Result<()> {
        self.enter_operation().map(drop)
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_process_fence(
        &self,
        process_fence: &ComputePluginFetchProcessFence,
    ) -> Result<()> {
        validate_process_fence_binding(
            &self.binding,
            &self.domain,
            &self.fence_liveness,
            process_fence,
        )?;
        self.ensure_current()
    }
}

impl AuthorizedCandidateCleanupDeletionGuard {
    pub(in crate::node_agent_compute_plugin_host) fn validate_authorization(
        &self,
        receipt: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
    ) -> Result<()> {
        self.ensure_current()?;
        validate_receipt_binding(&self.binding, receipt)?;
        if self.authorization_receipt_digest != receipt.receipt_digest() {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_AUTHORIZATION_CHANGED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_root_identity(
        &self,
        root_identity_digest: &str,
    ) -> Result<()> {
        self.ensure_current()?;
        if self.binding.root_identity_digest != root_identity_digest {
            bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_ROOT_CHANGED");
        }
        Ok(())
    }

    pub(in crate::node_agent_compute_plugin_host) fn validate_process_fence(
        &self,
        process_fence: &ComputePluginFetchProcessFence,
    ) -> Result<()> {
        validate_process_fence_binding(
            &self.binding,
            &self.domain,
            &self.fence_liveness,
            process_fence,
        )?;
        self.ensure_current()
    }

    pub(in crate::node_agent_compute_plugin_host) fn enter_operation(
        &self,
    ) -> Result<CandidateCleanupDeletionOperationLease> {
        self.domain.enter_operation(
            &self.fence_liveness,
            self.binding.process_owner_epoch,
            self.generation,
        )
    }

    pub(in crate::node_agent_compute_plugin_host) fn ensure_current(&self) -> Result<()> {
        self.enter_operation().map(drop)
    }
}

fn validate_process_fence_binding(
    binding: &CandidateCleanupDeletionBinding,
    domain: &CandidateCleanupDeletionDomain,
    fence_liveness: &Arc<AtomicBool>,
    process_fence: &ComputePluginFetchProcessFence,
) -> Result<()> {
    if !binding
        .authority_instance_binding
        .matches(process_fence.authority_instance_binding())
        || binding.installation_id_digest != process_fence.installation_id_digest()
        || binding.process_owner_epoch != process_fence.process_owner_epoch()
        || !domain.matches(
            &process_fence
                .authority_instance_binding()
                .cleanup_deletion_domain,
        )
        || !Arc::ptr_eq(
            fence_liveness,
            process_fence.cleanup_deletion_fence_liveness(),
        )
    {
        bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_SOURCE_CHANGED");
    }
    Ok(())
}

fn validate_receipt_binding(
    binding: &CandidateCleanupDeletionBinding,
    hashed: &HashedComputePluginCandidateCleanupAuthorizationReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    if receipt.cleanup_id() != binding.cleanup_id
        || receipt.candidate_token_digest() != binding.candidate_token_digest
        || receipt.quarantine_id() != binding.quarantine_id
        || receipt.quarantine_receipt_digest() != binding.quarantine_receipt_digest
        || receipt.staging_id() != binding.staging_id
        || receipt.staging_run_digest() != binding.staging_run_digest
        || receipt.process_owner_epoch() != binding.process_owner_epoch
        || !is_sha256(hashed.receipt_digest())
    {
        bail!("COMPUTE_PLUGIN_CLEANUP_DELETION_RECEIPT_CHANGED");
    }
    Ok(())
}

impl fmt::Debug for PreparedCandidateCleanupDeletionGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedCandidateCleanupDeletionGuard")
            .field("binding", &"<redacted>")
            .finish()
    }
}

impl fmt::Debug for AuthorizedCandidateCleanupDeletionGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedCandidateCleanupDeletionGuard")
            .field("binding", &"<redacted>")
            .finish()
    }
}
