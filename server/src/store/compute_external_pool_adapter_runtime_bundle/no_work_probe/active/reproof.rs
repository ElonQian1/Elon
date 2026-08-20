//! Final typed subject reproof and same-connection postcommit callback boundary.

use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::store::{
    compute_external_pool_adapter_provider_active_successor::reprove_external_pool_adapter_provider_active_successor_target_on,
    compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
    compute_provider_registry::current_registered_provider_on,
};

use super::types::PlannedExternalPoolAdapterActiveNoWorkProbeSubject;

/// Final transaction proof for the planned genesis subject. The final callback receives this type,
/// never an untyped I/O result or caller-constructed boolean.
pub(in crate::store) struct ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<
    'authority,
    'tx,
    'conn,
> {
    preflight: &'authority PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
    observation: &'authority CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<
        'authority,
        'tx,
        'conn,
    >,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'authority, 'tx, 'conn>
    ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'authority, 'tx, 'conn>
{
    pub(in crate::store) fn preflight(
        &self,
    ) -> &PlannedExternalPoolAdapterActiveNoWorkProbeSubject {
        self.preflight
    }

    pub(in crate::store) fn observation(
        &self,
    ) -> &CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<'authority, 'tx, 'conn> {
        self.observation
    }

    pub(in crate::store) fn evidence_checked_at(&self) -> &str {
        self.observation.checked_at()
    }
}

pub(in crate::store) fn with_reproved_planned_external_pool_adapter_active_no_work_subject<
    'authority,
    'tx,
    'conn,
    Output,
>(
    transaction: &'tx Transaction<'conn>,
    preflight: &'authority PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
    observation: &'authority CurrentExternalPoolAdapterNoWorkProbeObservationAuthority<
        'authority,
        'tx,
        'conn,
    >,
    final_callback: impl FnOnce(
        &'tx Transaction<'conn>,
        &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'authority, 'tx, 'conn>,
    ) -> Result<Output>,
) -> Result<Output> {
    let evidence_checked_at = observation.checked_at();
    let compatibility = observation.runtime_compatibility().verification();
    let credential = &observation.credential().reattestation.binding;
    reprove_external_pool_adapter_provider_active_successor_target_on(
        transaction,
        preflight.source(),
        preflight.target(),
        preflight.activation_root(),
        observation.companion(),
        observation.runtime_compatibility(),
        preflight.activation_target_updated_at(),
        evidence_checked_at,
    )?;
    let current =
        current_registered_provider_on(transaction, &preflight.source().provider.provider_id)?
            .ok_or_else(|| anyhow::anyhow!("planned active no-work source Provider disappeared"))?;
    if !observation.no_work_observed()
        || !observation.authenticated_shutdown_completed()
        || !observation.pidfd_reaped()
        || !observation.cgroup_cleaned()
        || !observation.scratch_cleaned()
        || observation.probe_checked_at() < preflight.activation_target_updated_at()
        || evidence_checked_at < preflight.activation_target_updated_at()
        || current.provider != preflight.source().provider
        || current.provider_digest != preflight.source().provider_digest
        || observation.upstream_target() != preflight.transport_target()
        || observation.companion().companion() != preflight.companion()
        || compatibility.verification_receipt_id
            != preflight.runtime_compatibility_verification_receipt_id()
        || compatibility.verification_receipt_digest
            != preflight.runtime_compatibility_verification_receipt_digest()
        || credential.provider_id != preflight.source().provider.provider_id
        || credential.observed_provider_policy_revision
            != preflight.source().provider.policy_revision
        || credential.observed_provider_digest != preflight.source().provider_digest
        || credential.observed_provider_status != preflight.source().provider.status
    {
        bail!("planned active no-work final subject differs from its pre-I/O target");
    }
    let authority = ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject {
        preflight,
        observation,
        transaction: PhantomData,
    };
    final_callback(transaction, &authority)
}
