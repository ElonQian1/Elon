use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};
use uuid::Uuid;

use super::{
    capability::{
        AuthorizedComputePluginCandidateStaging, RevalidatedComputePluginCandidateStaging,
        ValidatedCandidateStagingStorePermit,
    },
    recovery_key::ComputePluginCandidateStagingRecoveryKey,
};
use crate::node_agent_compute_plugin_host::{
    candidate_extraction::ExtractedComputePluginCandidateArchive,
    install_plan_admission_validation::is_identifier, lifecycle::SLOT_STAGED,
    local_authority::HashedComputePluginCandidateStagingReceipt, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateStagingStorePhase {
    PreStorePreparation,
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain candidate staging must be inspected through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingOutcomeUncertainCustody<'root>
{
    pub(super) revalidated: RevalidatedComputePluginCandidateStaging<'root>,
    pub(super) recovery_key: ComputePluginCandidateStagingRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateStagingStoreFailure<'root> {
    phase: CandidateStagingStorePhase,
    error: Error,
    recovery: CandidateStagingOutcomeUncertainCustody<'root>,
}

/// Extracted files whose exact Store transaction durably reached `staged`. This is not installed,
/// healthy, promotable, running, or commercially verified compute capacity.
#[must_use = "staged candidate custody must be consumed by health validation or cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct StagedComputePluginCandidateArchive<'root> {
    pub(super) archive: ExtractedComputePluginCandidateArchive<'root>,
    receipt: HashedComputePluginCandidateStagingReceipt,
    recovery_key: ComputePluginCandidateStagingRecoveryKey,
}

impl<'root> CandidateStagingStoreFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(&self) -> CandidateStagingStorePhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_recovery(
        self,
    ) -> CandidateStagingOutcomeUncertainCustody<'root> {
        self.recovery
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateStagingOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl CandidateStagingOutcomeUncertainCustody<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginCandidateStagingRecoveryKey {
        &self.recovery_key
    }
}

impl StagedComputePluginCandidateArchive<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginCandidateStagingReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginCandidateStagingRecoveryKey {
        &self.recovery_key
    }
}

impl<'root> StagedComputePluginCandidateArchive<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        ExtractedComputePluginCandidateArchive<'root>,
        HashedComputePluginCandidateStagingReceipt,
        ComputePluginCandidateStagingRecoveryKey,
    ) {
        (self.archive, self.receipt, self.recovery_key)
    }
}

impl fmt::Debug for CandidateStagingOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateStagingOutcomeUncertainCustody")
            .field("revalidated", &self.revalidated)
            .field("recovery_key", &self.recovery_key)
            .finish()
    }
}

impl fmt::Debug for CandidateStagingStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateStagingStoreFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("recovery", &self.recovery)
            .finish()
    }
}

impl fmt::Display for CandidateStagingStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for CandidateStagingStoreFailure<'_> {}

impl fmt::Debug for StagedComputePluginCandidateArchive<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StagedComputePluginCandidateArchive")
            .field("receipt", &self.receipt)
            .field("recovery_key", &self.recovery_key)
            .field("archive", &self.archive)
            .finish()
    }
}

pub(in crate::node_agent_compute_plugin_host) fn store_authorized_candidate_staging<'root>(
    authorized: AuthorizedComputePluginCandidateStaging<'root, '_>,
) -> std::result::Result<
    StagedComputePluginCandidateArchive<'root>,
    CandidateStagingStoreFailure<'root>,
> {
    let staging_id = format!("cst_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let recovery_key =
        ComputePluginCandidateStagingRecoveryKey::from_authorized(&authorized, staging_id.clone());
    if !is_identifier(&staging_id) {
        return Err(store_failure(
            CandidateStagingStorePhase::PreStorePreparation,
            anyhow::anyhow!("COMPUTE_PLUGIN_STAGING_ID_GENERATION_FAILED"),
            authorized,
            recovery_key,
        ));
    }

    let store_result = {
        let permit = ValidatedCandidateStagingStorePermit::new(&authorized, &staging_id);
        authorized
            .authority_session
            .persist_candidate_staging(permit)
    };
    let receipt = match store_result {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(store_failure(
                CandidateStagingStorePhase::StoreOutcomeUncertain,
                error,
                authorized,
                recovery_key,
            ))
        }
    };
    if let Err(error) = validate_store_receipt(&authorized, &recovery_key, &receipt) {
        return Err(store_failure(
            CandidateStagingStorePhase::StoreReturnedPostconditionFailed,
            error,
            authorized,
            recovery_key,
        ));
    }
    Ok(StagedComputePluginCandidateArchive {
        archive: authorized.revalidated.archive,
        receipt,
        recovery_key,
    })
}

fn validate_store_receipt(
    authorized: &AuthorizedComputePluginCandidateStaging<'_, '_>,
    recovery_key: &ComputePluginCandidateStagingRecoveryKey,
    receipt: &HashedComputePluginCandidateStagingReceipt,
) -> Result<()> {
    let body = receipt.receipt();
    let binding = &authorized.binding;
    if body.staging_id() != recovery_key.staging_id()
        || body.candidate_token_digest() != recovery_key.candidate_token_digest()
        || body.verification_id() != recovery_key.verification_id()
        || body.staging_run_digest() != recovery_key.staging_run_digest()
        || body.authority_state_revision_after()
            != binding
                .authority_state_revision()
                .checked_add(1)
                .unwrap_or(-1)
        || body.inventory_revision_after()
            != binding.inventory_revision().checked_add(1).unwrap_or(-1)
        || !is_sha256(body.inventory_digest_after())
        || body.inventory_digest_after() == binding.inventory_digest()
        || body.authority_epoch_after() != binding.authority_epoch().checked_add(1).unwrap_or(-1)
        || body.staged_at_ms() != authorized.authority_session.trusted_now_ms()
        || body.slot_phase_after() != SLOT_STAGED
        || !is_sha256(receipt.receipt_digest())
        || jcs_sha256_hex(body)? != receipt.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_STAGING_RECEIPT_POSTCONDITION_CHANGED");
    }
    Ok(())
}

fn store_failure<'root>(
    phase: CandidateStagingStorePhase,
    error: Error,
    authorized: AuthorizedComputePluginCandidateStaging<'root, '_>,
    recovery_key: ComputePluginCandidateStagingRecoveryKey,
) -> CandidateStagingStoreFailure<'root> {
    CandidateStagingStoreFailure {
        phase,
        error,
        recovery: CandidateStagingOutcomeUncertainCustody {
            revalidated: authorized.revalidated,
            recovery_key,
        },
    }
}
