use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::local_authority::ComputePluginWorkAdmissionRecoveryAuthoritySession;

use super::{
    ComputePluginWorkAdmissionReceiptPair, ComputePluginWorkAdmissionRecoveryOutcome,
    DurableWorkAdmittedPluginSlot, InstalledWorkAdmissionOutcomeUncertainCustody,
    InstalledWorkAdmissionRecoveryAdoption, InstalledWorkAdmissionRecoveryAdoptionFailure,
    InstalledWorkAdmissionRecoveryAdoptionPhase, InstalledWorkAdmissionRecoveryRevalidationFailure,
    PendingInstalledWorkAdmissionRecoveryAdoption,
};

/// Performs a fresh full retained-handle rehash without treating the superseded candidate guard as
/// current authority. The caller must mint the recovery session strictly after this barrier.
pub(in crate::node_agent_compute_plugin_host) fn begin_work_admission_recovery_revalidation<
    'root,
>(
    recovery: InstalledWorkAdmissionOutcomeUncertainCustody<'root>,
) -> Result<
    PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
    InstalledWorkAdmissionRecoveryRevalidationFailure<'root>,
> {
    let (mut revalidated, key) = recovery.into_parts();
    match revalidated.fresh_revalidate_for_recovery() {
        Ok(at) => Ok(PendingInstalledWorkAdmissionRecoveryAdoption::new(
            InstalledWorkAdmissionOutcomeUncertainCustody::new(revalidated, key),
            at,
        )),
        Err(error) => Err(InstalledWorkAdmissionRecoveryRevalidationFailure::new(
            error,
            InstalledWorkAdmissionOutcomeUncertainCustody::new(revalidated, key),
        )),
    }
}

pub(in crate::node_agent_compute_plugin_host) fn adopt_recovered_work_admission<'root>(
    pending: PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
    authority_session: ComputePluginWorkAdmissionRecoveryAuthoritySession<'_>,
) -> Result<
    InstalledWorkAdmissionRecoveryAdoption<'root>,
    InstalledWorkAdmissionRecoveryAdoptionFailure<'root>,
> {
    let key = pending.recovery_key();
    if !authority_session.was_observed_strictly_after(pending.revalidated_at())
        || !authority_session
            .authority_instance_binding()
            .matches(key.authority_instance_binding())
        || authority_session.installation_id_digest() != key.installation_id_digest()
        || authority_session.clock_epoch_digest() != key.clock_epoch_digest()
    {
        return Err(InstalledWorkAdmissionRecoveryAdoptionFailure::new(
            InstalledWorkAdmissionRecoveryAdoptionPhase::RecoveryAuthorityNotPostRevalidation,
            anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERY_AUTHORITY_INVALID"),
            pending,
            None,
        ));
    }
    let outcome = match authority_session.read_work_admission_outcome(key) {
        Ok(value) => value,
        Err(error) => {
            return Err(InstalledWorkAdmissionRecoveryAdoptionFailure::new(
                InstalledWorkAdmissionRecoveryAdoptionPhase::RecoveryReadOutcomeUncertain,
                error,
                pending,
                None,
            ))
        }
    };
    if let Some(pair) = outcome_pair(&outcome) {
        if let Err(error) = validate_exact_pair(&pending, pair) {
            return Err(InstalledWorkAdmissionRecoveryAdoptionFailure::new(
                InstalledWorkAdmissionRecoveryAdoptionPhase::RecoveredOutcomePostconditionFailed,
                error,
                pending,
                Some(outcome),
            ));
        }
    }
    adopt_exact_outcome(pending, outcome)
}

fn outcome_pair(
    outcome: &ComputePluginWorkAdmissionRecoveryOutcome,
) -> Option<&ComputePluginWorkAdmissionReceiptPair> {
    match outcome {
        ComputePluginWorkAdmissionRecoveryOutcome::AdmittedCurrent(value)
        | ComputePluginWorkAdmissionRecoveryOutcome::CommittedHistorical(value) => Some(value),
        ComputePluginWorkAdmissionRecoveryOutcome::NotCreated
        | ComputePluginWorkAdmissionRecoveryOutcome::NotCreatedSuperseded => None,
    }
}

fn validate_exact_pair(
    pending: &PendingInstalledWorkAdmissionRecoveryAdoption<'_>,
    pair: &ComputePluginWorkAdmissionReceiptPair,
) -> Result<()> {
    pair.validate()?;
    let key = pending.recovery_key();
    let expected = key.expectation();
    let receipt = pair.receipt().receipt();
    if pair.source().source_digest() != expected.source_digest()
        || pair.receipt().receipt_digest() != expected.expected_receipt_digest()
        || receipt.work_admission_id() != key.work_admission_id()
        || receipt.installation_id_digest() != key.installation_id_digest()
        || receipt.plugin_id() != key.plugin_id()
        || receipt.slot_ref() != key.slot_ref()
        || receipt.release() != key.release()
    {
        bail!("COMPUTE_PLUGIN_WORK_ADMISSION_RECOVERED_RECEIPT_CHANGED");
    }
    Ok(())
}

fn adopt_exact_outcome<'root>(
    pending: PendingInstalledWorkAdmissionRecoveryAdoption<'root>,
    outcome: ComputePluginWorkAdmissionRecoveryOutcome,
) -> Result<
    InstalledWorkAdmissionRecoveryAdoption<'root>,
    InstalledWorkAdmissionRecoveryAdoptionFailure<'root>,
> {
    let (recovery, _) = pending.into_parts();
    let (revalidated, _) = recovery.into_parts();
    Ok(match outcome {
        ComputePluginWorkAdmissionRecoveryOutcome::NotCreated => {
            InstalledWorkAdmissionRecoveryAdoption::NotCreated(revalidated.into_installed())
        }
        ComputePluginWorkAdmissionRecoveryOutcome::AdmittedCurrent(receipts) => {
            InstalledWorkAdmissionRecoveryAdoption::AdmittedCurrent(
                DurableWorkAdmittedPluginSlot::new(revalidated, receipts),
            )
        }
        ComputePluginWorkAdmissionRecoveryOutcome::CommittedHistorical(receipts) => {
            InstalledWorkAdmissionRecoveryAdoption::CommittedHistorical {
                installed: revalidated.into_installed(),
                receipts,
            }
        }
        ComputePluginWorkAdmissionRecoveryOutcome::NotCreatedSuperseded => {
            InstalledWorkAdmissionRecoveryAdoption::NotCreatedSuperseded(
                revalidated.into_installed(),
            )
        }
    })
}
