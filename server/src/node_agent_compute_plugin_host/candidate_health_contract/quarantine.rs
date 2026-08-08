use std::{error::Error as StdError, fmt};

use anyhow::{bail, Error, Result};
use uuid::Uuid;

use super::ValidatedCandidateHealthFailurePublication;
use crate::node_agent_compute_plugin_host::{
    candidate_staging_contract::StagedComputePluginCandidateArchive,
    install_plan_admission_validation::is_identifier,
    local_authority::{
        ComputePluginCandidateHealthQuarantineAuthorityFacts,
        ComputePluginCandidateHealthQuarantineAuthoritySession, ComputePluginFetchProcessFence,
        ComputePluginLocalAuthority, HashedComputePluginCandidateHealthQuarantineReceipt,
    },
    manifest_validation::is_sha256,
    signed_artifact_verification::jcs_sha256_hex,
};

mod adoption;
mod recovery_key;

pub(in crate::node_agent_compute_plugin_host) use adoption::{
    adopt_recovered_candidate_health_quarantine, CandidateHealthQuarantineRecoveryAdoptionFailure,
    CandidateHealthQuarantineRecoveryAdoptionPhase,
};
pub(in crate::node_agent_compute_plugin_host) use recovery_key::{
    CandidateHealthQuarantineReceiptExpectation, CandidateHealthQuarantineRecoveryKey,
};

#[must_use = "authorized candidate failure must be quarantined or returned for cleanup"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedCandidateHealthQuarantine<
    'root,
    'authority,
> {
    pub(super) publication: ValidatedCandidateHealthFailurePublication<'root>,
    pub(super) authority_session:
        ComputePluginCandidateHealthQuarantineAuthoritySession<'authority>,
    pub(super) facts: ComputePluginCandidateHealthQuarantineAuthorityFacts,
    pub(super) quarantine_id: String,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineAuthorizationFailure<
    'root,
> {
    error: Error,
    publication: ValidatedCandidateHealthFailurePublication<'root>,
}

pub(in crate::node_agent_compute_plugin_host) struct ValidatedCandidateHealthQuarantinePermit<
    'permit,
    'root,
> {
    authorized: &'permit AuthorizedCandidateHealthQuarantine<'root, 'permit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateHealthQuarantineStorePhase {
    StoreOutcomeUncertain,
    StoreReturnedPostconditionFailed,
}

#[must_use = "uncertain quarantine must be inspected through recovery authority"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineOutcomeUncertainCustody<
    'root,
> {
    pub(super) publication: ValidatedCandidateHealthFailurePublication<'root>,
    pub(super) recovery_key: CandidateHealthQuarantineRecoveryKey,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateHealthQuarantineStoreFailure<'root> {
    phase: CandidateHealthQuarantineStorePhase,
    error: Error,
    recovery: CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
}

/// Retained staged file handles plus an exact durable `staged -> failed` receipt. This is not a
/// cleanup permit, deletion receipt, retry permit, install result or promotion capability.
#[must_use = "quarantined candidate must be consumed by a future cleanup authorization"]
pub(in crate::node_agent_compute_plugin_host) struct DurableCandidateHealthQuarantine<'root> {
    pub(super) staged: StagedComputePluginCandidateArchive<'root>,
    pub(super) receipt: HashedComputePluginCandidateHealthQuarantineReceipt,
}

pub(in crate::node_agent_compute_plugin_host) fn authorize_candidate_health_quarantine<
    'root,
    'authority,
>(
    publication: ValidatedCandidateHealthFailurePublication<'root>,
    authority: &'authority ComputePluginLocalAuthority,
    process_fence: &'authority ComputePluginFetchProcessFence,
) -> std::result::Result<
    AuthorizedCandidateHealthQuarantine<'root, 'authority>,
    CandidateHealthQuarantineAuthorizationFailure<'root>,
> {
    let authority_session = match authority.bind_candidate_health_quarantine_authority_session(
        process_fence,
        publication.trusted_time(),
    ) {
        Ok(session) => session,
        Err(error) => {
            return Err(CandidateHealthQuarantineAuthorizationFailure { error, publication })
        }
    };
    let facts = match authority_session.read_candidate_health_quarantine_binding(&publication) {
        Ok(facts) => facts,
        Err(error) => {
            return Err(CandidateHealthQuarantineAuthorizationFailure { error, publication })
        }
    };
    let quarantine_id = format!("chq_{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    if !is_identifier(&quarantine_id) {
        return Err(CandidateHealthQuarantineAuthorizationFailure {
            error: anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_ID_INVALID"),
            publication,
        });
    }
    Ok(AuthorizedCandidateHealthQuarantine {
        publication,
        authority_session,
        facts,
        quarantine_id,
    })
}

pub(in crate::node_agent_compute_plugin_host) fn quarantine_authorized_candidate_health_failure<
    'root,
>(
    authorized: AuthorizedCandidateHealthQuarantine<'root, '_>,
) -> std::result::Result<
    DurableCandidateHealthQuarantine<'root>,
    CandidateHealthQuarantineStoreFailure<'root>,
> {
    let recovery_key = CandidateHealthQuarantineRecoveryKey::from_authorized(&authorized);
    let result = {
        let permit = ValidatedCandidateHealthQuarantinePermit::new(&authorized);
        authorized
            .authority_session
            .persist_candidate_health_quarantine(permit)
    };
    let receipt = match result {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(store_failure(
                CandidateHealthQuarantineStorePhase::StoreOutcomeUncertain,
                error,
                authorized.publication,
                recovery_key,
            ))
        }
    };
    if let Err(error) =
        validate_quarantine_receipt(&authorized.publication, &recovery_key, &receipt)
    {
        return Err(store_failure(
            CandidateHealthQuarantineStorePhase::StoreReturnedPostconditionFailed,
            error,
            authorized.publication,
            recovery_key,
        ));
    }
    let (staged, _, _) = authorized.publication.into_parts();
    Ok(DurableCandidateHealthQuarantine { staged, receipt })
}

pub(super) fn validate_quarantine_receipt(
    publication: &ValidatedCandidateHealthFailurePublication<'_>,
    key: &CandidateHealthQuarantineRecoveryKey,
    hashed: &HashedComputePluginCandidateHealthQuarantineReceipt,
) -> Result<()> {
    let receipt = hashed.receipt();
    let expected = key.receipt_expectation();
    let staging = key.staging_expectation();
    if receipt.quarantine_id() != key.quarantine_id()
        || receipt.evaluation_id() != expected.evaluation_id
        || receipt.candidate_token_digest() != staging.candidate_token_digest
        || receipt.staging_id() != staging.staging_id
        || receipt.staging_receipt_digest() != staging.staging_receipt_digest
        || receipt.staging_run_digest() != staging.staging_run_digest
        || receipt.failure_observation_digest() != expected.failure_observation_digest
        || receipt.authority_state_revision_before() != expected.authority_state_revision_before
        || receipt.authority_state_revision_after() != expected.authority_state_revision_after
        || receipt.inventory_revision_before() != expected.inventory_revision_before
        || receipt.inventory_revision_after() != expected.inventory_revision_after
        || receipt.inventory_digest_before() != expected.inventory_digest_before
        || receipt.inventory_digest_after() != expected.inventory_digest_after
        || receipt.authority_epoch_before() != expected.authority_epoch_before
        || receipt.authority_epoch_after() != expected.authority_epoch_after
        || receipt.process_owner_epoch() != expected.process_owner_epoch
        || receipt.trusted_time_high_water_ms_before() != expected.trusted_time_high_water_ms_before
        || receipt.failed_at_ms() != expected.failed_at_ms
        || receipt.slot_phase_after() != "failed"
        || hashed.observation() != publication.observation()
        || !is_sha256(hashed.receipt_digest())
        || jcs_sha256_hex(receipt)? != hashed.receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_QUARANTINE_RECEIPT_POSTCONDITION_CHANGED");
    }
    Ok(())
}

fn store_failure<'root>(
    phase: CandidateHealthQuarantineStorePhase,
    error: Error,
    publication: ValidatedCandidateHealthFailurePublication<'root>,
    recovery_key: CandidateHealthQuarantineRecoveryKey,
) -> CandidateHealthQuarantineStoreFailure<'root> {
    CandidateHealthQuarantineStoreFailure {
        phase,
        error,
        recovery: CandidateHealthQuarantineOutcomeUncertainCustody {
            publication,
            recovery_key,
        },
    }
}

impl<'permit, 'root> ValidatedCandidateHealthQuarantinePermit<'permit, 'root> {
    pub(super) fn new(
        authorized: &'permit AuthorizedCandidateHealthQuarantine<'root, 'permit>,
    ) -> Self {
        Self { authorized }
    }

    pub(in crate::node_agent_compute_plugin_host) fn publication(
        &self,
    ) -> &ValidatedCandidateHealthFailurePublication<'root> {
        &self.authorized.publication
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginCandidateHealthQuarantineAuthorityFacts {
        &self.authorized.facts
    }

    pub(in crate::node_agent_compute_plugin_host) fn quarantine_id(&self) -> &str {
        &self.authorized.quarantine_id
    }
}

impl CandidateHealthQuarantineStoreFailure<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateHealthQuarantineStorePhase {
        self.phase
    }
}

impl<'root> CandidateHealthQuarantineStoreFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        Error,
        CandidateHealthQuarantineOutcomeUncertainCustody<'root>,
    ) {
        (self.error, self.recovery)
    }
}

impl CandidateHealthQuarantineOutcomeUncertainCustody<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn recovery_key(
        &self,
    ) -> &CandidateHealthQuarantineRecoveryKey {
        &self.recovery_key
    }
}

impl DurableCandidateHealthQuarantine<'_> {
    pub(in crate::node_agent_compute_plugin_host) fn staged(
        &self,
    ) -> &StagedComputePluginCandidateArchive<'_> {
        &self.staged
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipt(
        &self,
    ) -> &HashedComputePluginCandidateHealthQuarantineReceipt {
        &self.receipt
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidate_retained_content(
        &mut self,
    ) -> Result<()> {
        self.staged.revalidate_retained_content()
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidate_for_prepared_candidate_cleanup(
        &mut self,
        guard: &crate::node_agent_compute_plugin_host::local_authority::PreparedCandidateCleanupDeletionGuard,
    ) -> Result<()> {
        self.staged.revalidate_for_prepared_candidate_cleanup(guard)
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidate_for_authorized_candidate_cleanup(
        &mut self,
        guard: &crate::node_agent_compute_plugin_host::local_authority::AuthorizedCandidateCleanupDeletionGuard,
    ) -> Result<()> {
        self.staged
            .revalidate_for_authorized_candidate_cleanup(guard)
    }
}

impl<'root> DurableCandidateHealthQuarantine<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (
        StagedComputePluginCandidateArchive<'root>,
        HashedComputePluginCandidateHealthQuarantineReceipt,
    ) {
        (self.staged, self.receipt)
    }
}

impl<'root> CandidateHealthQuarantineAuthorizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, ValidatedCandidateHealthFailurePublication<'root>) {
        (self.error, self.publication)
    }
}

macro_rules! impl_failure {
    ($failure:ident) => {
        impl fmt::Display for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:#}", self.error)
            }
        }

        impl fmt::Debug for $failure<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($failure))
                    .field("error", &self.error)
                    .finish_non_exhaustive()
            }
        }

        impl StdError for $failure<'_> {}
    };
}

impl_failure!(CandidateHealthQuarantineAuthorizationFailure);
impl_failure!(CandidateHealthQuarantineStoreFailure);

impl fmt::Debug for CandidateHealthQuarantineOutcomeUncertainCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateHealthQuarantineOutcomeUncertainCustody")
            .field("recovery_key", &self.recovery_key)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for DurableCandidateHealthQuarantine<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableCandidateHealthQuarantine")
            .field("receipt", &self.receipt)
            .field("staged", &"<retained-handles>")
            .finish()
    }
}
