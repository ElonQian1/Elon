use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};

use super::{
    authorization::{AuthorizedCandidateHealthStore, ValidatedCandidateHealthStorePermit},
    recovery_key::ComputePluginCandidateHealthRecoveryKey,
    StagedComputePluginCandidateArchive, ValidatedCandidateHealthPublication,
};
use crate::node_agent_compute_plugin_host::{
    local_authority::HashedComputePluginCandidateHealthReceipt, manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

mod adoption;

pub(in crate::node_agent_compute_plugin_host) use adoption::{
    adopt_recovered_candidate_health, CandidateHealthRecoveryAdoptionFailure,
    CandidateHealthRecoveryAdoptionPhase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateHealthStorePhase {
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain candidate health must be inspected through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthOutcomeUncertainCustody<'root> {
    pub(super) publication: ValidatedCandidateHealthPublication<'root>,
    pub(super) recovery_key: ComputePluginCandidateHealthRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthStoreFailure<'root> {
    phase: CandidateHealthStorePhase,
    error: Error,
    recovery: CandidateHealthOutcomeUncertainCustody<'root>,
}

/// Staged files plus an exact durable healthy receipt. This still is not installed, promotable,
/// running, a ReadyCapability or commercially verified compute capacity.
#[must_use = "durable candidate health must be consumed by promotion or cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateHealthPublication<'root> {
    staged: StagedComputePluginCandidateArchive<'root>,
    receipt: HashedComputePluginCandidateHealthReceipt,
}

pub(in crate::node_agent_compute_plugin_host) fn store_authorized_candidate_health<'root>(
    authorized: AuthorizedCandidateHealthStore<'root, '_>,
) -> std::result::Result<DurableCandidateHealthPublication<'root>, CandidateHealthStoreFailure<'root>>
{
    let recovery_key = ComputePluginCandidateHealthRecoveryKey::from_authorized(&authorized);
    let result = {
        let permit = ValidatedCandidateHealthStorePermit::new(&authorized);
        authorized
            .authority_session
            .persist_candidate_health(permit)
    };
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(store_failure(
                CandidateHealthStorePhase::StoreOutcomeUncertain,
                error,
                authorized.publication,
                recovery_key,
            ))
        }
    };
    if let Err(error) = validate_store_receipt(&authorized.publication, &recovery_key, &receipt) {
        return Err(store_failure(
            CandidateHealthStorePhase::StoreReturnedPostconditionFailed,
            error,
            authorized.publication,
            recovery_key,
        ));
    }
    let (staged, _, _) = authorized.publication.into_parts();
    Ok(DurableCandidateHealthPublication { staged, receipt })
}

fn validate_store_receipt(
    publication: &ValidatedCandidateHealthPublication<'_>,
    key: &ComputePluginCandidateHealthRecoveryKey,
    hashed: &HashedComputePluginCandidateHealthReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    if receipt.health_id() != key.health_id()
        || receipt.evaluation_id() != expected.evaluation_id
        || receipt.candidate_token_digest() != staging.candidate_token_digest
        || receipt.staging_id() != staging.staging_id
        || receipt.staging_receipt_digest() != staging.staging_receipt_digest
        || receipt.staging_run_digest() != staging.staging_run_digest
        || receipt.health_observation_digest() != expected.health_observation_digest
        || receipt.authority_state_revision() != expected.authority_state_revision
        || receipt.inventory_revision() != expected.inventory_revision
        || receipt.inventory_digest() != expected.inventory_digest
        || receipt.authority_epoch() != expected.authority_epoch
        || receipt.process_owner_epoch() != expected.process_owner_epoch
        || receipt.recorded_at_ms() != expected.recorded_at_ms
        || receipt.expires_at_ms() != expected.expires_at_ms
        || hashed.observation() != publication.observation()
        || !is_sha256(hashed.receipt_digest())
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_HEALTH_RECEIPT_POSTCONDITION_CHANGED");
    }
    Ok(())
}

fn store_failure<'root>(
    phase: CandidateHealthStorePhase,
    error: Error,
    publication: ValidatedCandidateHealthPublication<'root>,
    recovery_key: ComputePluginCandidateHealthRecoveryKey,
) -> CandidateHealthStoreFailure<'root> {
    CandidateHealthStoreFailure {
        phase,
        error,
        recovery: CandidateHealthOutcomeUncertainCustody {
            publication,
            recovery_key,
        },
    }
}

impl CandidateHealthStoreFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(&self) -> CandidateHealthStorePhase {
        self.phase
    }
}

impl<'root> CandidateHealthStoreFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateHealthOutcomeUncertainCustody<'root>) {
        (self.error, self.recovery)
    }
}

impl CandidateHealthOutcomeUncertainCustody<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &ComputePluginCandidateHealthRecoveryKey {
        &self.recovery_key
    }
}

impl DurableCandidateHealthPublication<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn staged(
        &self,
    ) -> &StagedComputePluginCandidateArchive<'_> {
        &self.staged
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginCandidateHealthReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidate_retained_content(
        &mut self,
    ) -> Result<()> {
        self.staged.revalidate_retained_content()
    }
}

impl<'root> DurableCandidateHealthPublication<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        StagedComputePluginCandidateArchive<'root>,
        HashedComputePluginCandidateHealthReceipt,
    ) {
        (self.staged, self.receipt)
    }
}

impl fmt::Display for CandidateHealthStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateHealthStoreFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthStoreFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateHealthStoreFailure<'_> {}

impl fmt::Debug for CandidateHealthOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthOutcomeUncertainCustody")
            .field("recovery_key", &self.recovery_key)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for DurableCandidateHealthPublication<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableCandidateHealthPublication")
            .field("receipt", &self.receipt)
            .field("staged", &"<retained-handles>")
            .finish()
    }
}
