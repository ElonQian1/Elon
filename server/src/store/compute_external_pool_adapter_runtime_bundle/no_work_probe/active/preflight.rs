//! Typed subject projection before any transaction-free no-work execution.

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::store::{
    compute_external_pool_adapter_provider_active_successor::PreparedExternalPoolAdapterProviderActiveSuccessorTarget,
    compute_external_pool_adapter_runtime_bundle::{
        CurrentExternalPoolAdapterNoWorkProbeObservationAuthority,
        ExternalPoolAdapterProviderRuntimeReadinessRuntime,
    },
    compute_external_pool_adapter_task_protocol_conformance::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
    compute_external_pool_adapter_upstream_transport_target::ExternalPoolAdapterInstallationReopener,
    Store,
};

use super::reproof::{
    with_reproved_planned_external_pool_adapter_active_no_work_subject,
    ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
};
use super::types::{
    DurableExternalPoolAdapterActiveNoWorkProbeSubject,
    PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
};

pub(in crate::store) fn planned_external_pool_adapter_active_no_work_probe_subject_on(
    prepared: &PreparedExternalPoolAdapterProviderActiveSuccessorTarget<'_, '_>,
) -> Result<PlannedExternalPoolAdapterActiveNoWorkProbeSubject> {
    let activation = prepared.activation_root();
    let root = &activation.activation_root;
    let transport_target = prepared.companion().target().target();
    let companion = prepared.companion().companion();
    let compatibility = prepared.runtime_compatibility().verification();
    if prepared.target().updated_at != prepared.activation_target_updated_at()
        || transport_target.target_id != root.target_id
        || transport_target.target_digest != root.target_digest
        || companion.companion_id != root.companion_id
        || companion.companion_digest != root.companion_digest
        || prepared.authority_checked_at() < prepared.activation_target_updated_at()
    {
        bail!("planned active no-work subject does not retain the exact V274 target roots");
    }
    Ok(PlannedExternalPoolAdapterActiveNoWorkProbeSubject::new(
        prepared.source().clone(),
        prepared.target().clone(),
        activation.clone(),
        transport_target.clone(),
        companion.clone(),
        compatibility.verification_receipt_id.clone(),
        compatibility.verification_receipt_digest.clone(),
        prepared.activation_target_updated_at().into(),
    ))
}

impl Store {
    /// Executes planned genesis evidence through the existing registering-only producer.
    /// The frozen target is checked before I/O and again in the producer's final transaction;
    /// callers must still rebuild the typed V274 target before invoking the final append callback.
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(in crate::store) async fn with_planned_external_pool_adapter_active_no_work_probe_observation<
        Pending,
        Output,
    >(
        &self,
        planned: &PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        reopen_prepared: &mut ExternalPoolAdapterInstallationReopener<'_>,
        runtime: &ExternalPoolAdapterProviderRuntimeReadinessRuntime,
        consume: impl FnOnce(
                &Transaction<'_>,
                &ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, '_, '_>,
            ) -> Result<Pending>
            + Send,
        postcommit: impl FnOnce(&rusqlite::Connection, Pending) -> Result<Output> + Send,
    ) -> Result<Option<Output>> {
        let root = &planned.activation_root().activation_root;
        let preflight_planned = planned;
        let final_planned = planned;
        self.with_current_external_pool_adapter_no_work_probe_observation(
            &root.profile_id,
            &root.companion_id,
            &root.companion_digest,
            &root.target_id,
            &root.target_digest,
            planned.runtime_compatibility_verification_receipt_id(),
            planned.runtime_compatibility_verification_receipt_digest(),
            reopen_prepared,
            runtime,
            move |_transaction, target, checked_at| {
                if target.target() != preflight_planned.transport_target()
                    || checked_at < preflight_planned.activation_target_updated_at()
                {
                    bail!("planned active no-work preflight differs from its frozen target");
                }
                Ok(())
            },
            move |transaction, observation| {
                let credential = &observation.credential().reattestation.binding;
                let source = &final_planned.source().provider;
                let compatibility = observation.runtime_compatibility().verification();
                if observation.upstream_target() != final_planned.transport_target()
                    || observation.companion().companion() != final_planned.companion()
                    || observation.checked_at() < final_planned.activation_target_updated_at()
                    || observation.probe_checked_at() < final_planned.activation_target_updated_at()
                    || compatibility.verification_receipt_id
                        != final_planned.runtime_compatibility_verification_receipt_id()
                    || compatibility.verification_receipt_digest
                        != final_planned.runtime_compatibility_verification_receipt_digest()
                    || credential.provider_id != source.provider_id
                    || credential.observed_provider_policy_revision != source.policy_revision
                    || credential.observed_provider_digest != final_planned.source().provider_digest
                    || credential.observed_provider_status != source.status
                {
                    bail!("planned active no-work final observation differs from its target");
                }
                with_reproved_planned_external_pool_adapter_active_no_work_subject(
                    transaction,
                    final_planned,
                    observation,
                    consume,
                )
            },
            postcommit,
        )
        .await
    }
}

pub(super) fn durable_external_pool_adapter_active_no_work_probe_subject_on(
    task_protocol: &CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority<'_, '_>,
) -> Result<DurableExternalPoolAdapterActiveNoWorkProbeSubject> {
    let carrier = task_protocol.carrier();
    let historical = carrier.historical_activation();
    let receipt = historical.receipt();
    let activation = historical.activation_root();
    let root = &activation.activation_root;
    let target = carrier.target();
    let companion = carrier.companion();
    let renewed_route = carrier.renewed_route();
    let credential = carrier.credential().receipt();
    let compatibility = carrier.runtime_compatibility().verification();
    if receipt.activation.identity.provider_binding_id != root.provider_binding_id
        || receipt.activation.identity.provider_binding_digest != root.provider_binding_digest
        || receipt.activation.identity.activation_root_digest != activation.activation_root_digest
        || historical.active_provider().provider_id != root.provider_id
        || target.target_id != root.target_id
        || target.target_digest != root.target_digest
        || companion.companion_id != root.companion_id
        || companion.companion_digest != root.companion_digest
    {
        bail!("durable active no-work subject does not retain the exact S1 carrier roots");
    }
    Ok(DurableExternalPoolAdapterActiveNoWorkProbeSubject::new(
        receipt.clone(),
        activation.clone(),
        historical.active_provider().clone(),
        target.clone(),
        companion.clone(),
        renewed_route.receipt().route_renewal_receipt_id.clone(),
        renewed_route.receipt().route_renewal_receipt_digest.clone(),
        renewed_route.effective_expires_at().into(),
        credential.reattestation_receipt_id.clone(),
        credential.reattestation_receipt_digest.clone(),
        compatibility.verification_receipt_id.clone(),
        compatibility.verification_receipt_digest.clone(),
        task_protocol.receipt().run_receipt_id.clone(),
        task_protocol.receipt().run_receipt_digest.clone(),
        carrier.checked_at().into(),
    ))
}
