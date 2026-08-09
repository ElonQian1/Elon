use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginCandidatePromotionRecoveryAuthoritySession,
    ComputePluginCandidatePromotionRecoveryOutcome,
};

use super::{
    CandidatePromotionOutcomeUncertainCustody, CandidatePromotionReceiptPair,
    CandidatePromotionRecoveryAdoption, CandidatePromotionRecoveryAdoptionFailure,
    CandidatePromotionRecoveryAdoptionPhase, DurableInstalledPluginSlot,
};

/// Consumes uncertain custody through authenticated, read-only recovery. NotCreated demotes the
/// retained handles to durable-health custody and returns no revalidated promotion or Store
/// permit. Either outcome is adopted only after the same pinned staged content and seal pass a
/// fresh full revalidation.
pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_candidate_promotion<'root>(
    recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
    authority_session: ComputePluginCandidatePromotionRecoveryAuthoritySession<'_>,
) -> std::result::Result<
    CandidatePromotionRecoveryAdoption<'root>,
    CandidatePromotionRecoveryAdoptionFailure<'root>,
> {
    let outcome = match authority_session.read_candidate_promotion_outcome(recovery.recovery_key())
    {
        Ok(outcome) => outcome,
        Err(error) => {
            return Err(CandidatePromotionRecoveryAdoptionFailure::new(
                CandidatePromotionRecoveryAdoptionPhase::RecoveryReadOutcomeUncertain,
                error,
                recovery,
                None,
            ))
        }
    };
    match outcome {
        ComputePluginCandidatePromotionRecoveryOutcome::NotCreated => adopt_not_created(recovery),
        ComputePluginCandidatePromotionRecoveryOutcome::Installed(receipts) => {
            adopt_installed(recovery, receipts)
        }
    }
}

fn adopt_not_created<'root>(
    recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
) -> std::result::Result<
    CandidatePromotionRecoveryAdoption<'root>,
    CandidatePromotionRecoveryAdoptionFailure<'root>,
> {
    let (mut revalidated, recovery_key) = recovery.into_parts();
    if let Err(error) = revalidated.fresh_revalidate_retained_content() {
        return Err(CandidatePromotionRecoveryAdoptionFailure::new(
            CandidatePromotionRecoveryAdoptionPhase::RetainedContentRevalidationFailed,
            error,
            CandidatePromotionOutcomeUncertainCustody::new(revalidated, recovery_key),
            Some(ComputePluginCandidatePromotionRecoveryOutcome::NotCreated),
        ));
    }
    Ok(CandidatePromotionRecoveryAdoption::NotCreated(
        revalidated.into_publication(),
    ))
}

fn adopt_installed<'root>(
    recovery: CandidatePromotionOutcomeUncertainCustody<'root>,
    receipts: CandidatePromotionReceiptPair,
) -> std::result::Result<
    CandidatePromotionRecoveryAdoption<'root>,
    CandidatePromotionRecoveryAdoptionFailure<'root>,
> {
    if let Err(error) = validate_exact_receipts(&recovery, &receipts) {
        return Err(CandidatePromotionRecoveryAdoptionFailure::new(
            CandidatePromotionRecoveryAdoptionPhase::RecoveredOutcomePostconditionFailed,
            error,
            recovery,
            Some(ComputePluginCandidatePromotionRecoveryOutcome::Installed(
                receipts,
            )),
        ));
    }
    let (mut revalidated, recovery_key) = recovery.into_parts();
    if let Err(error) = revalidated.fresh_revalidate_retained_content() {
        return Err(CandidatePromotionRecoveryAdoptionFailure::new(
            CandidatePromotionRecoveryAdoptionPhase::RetainedContentRevalidationFailed,
            error,
            CandidatePromotionOutcomeUncertainCustody::new(revalidated, recovery_key),
            Some(ComputePluginCandidatePromotionRecoveryOutcome::Installed(
                receipts,
            )),
        ));
    }
    Ok(CandidatePromotionRecoveryAdoption::Installed(
        DurableInstalledPluginSlot::new(revalidated, receipts),
    ))
}

fn validate_exact_receipts(
    recovery: &CandidatePromotionOutcomeUncertainCustody<'_>,
    receipts: &CandidatePromotionReceiptPair,
) -> Result<()> {
    receipts.validate()?;
    let expected = recovery.recovery_key().expectation();
    if receipts.install().receipt_digest() != expected.expected_install_receipt_digest()
        || receipts.promotion().receipt_digest() != expected.expected_promotion_receipt_digest()
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_PROMOTION_ADOPTION_RECEIPT_CHANGED");
    }
    Ok(())
}
