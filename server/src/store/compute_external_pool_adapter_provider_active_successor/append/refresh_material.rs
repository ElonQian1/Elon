//! Canonical V274 refresh material derived only from final active no-work authority.

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_provider_active_successor::{
            provider_active_successor_effects_none, provider_active_successor_readiness_none,
            provider_active_successor_runtime_observation_digest,
            ExternalPoolAdapterProviderActiveSuccessorActivationWitness,
            ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence,
            ExternalPoolAdapterProviderActiveSuccessorLineage,
            ExternalPoolAdapterProviderActiveSuccessorMaterial,
            ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
            ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation,
            ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence,
        },
        provider::ComputeProvider,
    },
    store::compute_external_pool_adapter_runtime_bundle::CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority,
};

use super::super::read::head_by_binding_and_root_on;

pub(in crate::store) fn build_external_pool_adapter_provider_active_successor_refresh_material_on(
    transaction: &Transaction<'_>,
    observation: &CurrentExternalPoolAdapterProjectedActiveNoWorkObservationAuthority<'_, '_, '_>,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorMaterial> {
    if !observation.no_work_observed() {
        bail!("V274 refresh requires authenticated terminal no-work");
    }
    let task_protocol = observation.task_protocol();
    let carrier = task_protocol.carrier();
    let historical = carrier.historical_activation();
    let activation = historical.activation_root();
    let root = &activation.activation_root;
    let head = head_by_binding_and_root_on(
        transaction,
        &root.provider_binding_id,
        &activation.activation_root_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("V274 refresh lacks historical predecessor"))?;
    let sequence = head
        .receipt
        .successor
        .lineage
        .successor_sequence
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V274 refresh sequence overflow"))?;
    let provider = historical.active_provider();
    let provider_evidence = provider_evidence(provider)?;
    let credential = carrier.credential().receipt();
    let mut runtime_observation = ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation {
        runtime_observation_id: observation.post_cleanup_observation_commitment().into(),
        runtime_observation_digest: String::new(),
        observed_provider: provider_evidence.clone(),
        observation_started_at: observation.probe_checked_at().into(),
        observation_completed_at: observation.checked_at().into(),
        observation_expires_at: observation.expires_at().into(),
    };
    runtime_observation.runtime_observation_digest =
        provider_active_successor_runtime_observation_digest(&runtime_observation)?;
    Ok(ExternalPoolAdapterProviderActiveSuccessorMaterial {
        activation: activation.clone(),
        lineage: ExternalPoolAdapterProviderActiveSuccessorLineage {
            successor_sequence: sequence,
            predecessor_active_successor_receipt_id: Some(
                head.receipt.active_successor_receipt_id.clone(),
            ),
            predecessor_active_successor_receipt_digest: Some(head.receipt.receipt_digest.clone()),
        },
        evidence_provider: provider_evidence.clone(),
        credential_evidence: ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence {
            reattestation_receipt_id: credential.reattestation_receipt_id.clone(),
            reattestation_receipt_digest: credential.reattestation_receipt_digest.clone(),
            observed_provider: provider_evidence.clone(),
        },
        runtime_observation,
        task_protocol_evidence: ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence {
            task_protocol_conformance_run_receipt_id: task_protocol
                .receipt()
                .run_receipt_id
                .clone(),
            task_protocol_conformance_run_receipt_digest: task_protocol
                .receipt()
                .run_receipt_digest
                .clone(),
            task_protocol_conformance_expires_at: task_protocol.receipt().run.expires_at.clone(),
        },
        activation_witness: ExternalPoolAdapterProviderActiveSuccessorActivationWitness {
            activation_witness_id: historical.receipt().activation_receipt_id.clone(),
            activation_witness_digest: historical.receipt().activation_receipt_digest.clone(),
        },
        activation_target_updated_at: historical
            .receipt()
            .activation
            .activation_target_updated_at
            .clone(),
        evidence_checked_at: observation.checked_at().into(),
        created_at: observation.checked_at().into(),
        effects: provider_active_successor_effects_none(),
        readiness: provider_active_successor_readiness_none(),
    })
}

fn provider_evidence(
    provider: &ComputeProvider,
) -> Result<ExternalPoolAdapterProviderActiveSuccessorProviderEvidence> {
    let provider_json = serde_json::to_string(provider)?;
    let provider_digest = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(provider_json.as_bytes()))
    };
    Ok(ExternalPoolAdapterProviderActiveSuccessorProviderEvidence {
        provider_id: provider.provider_id.clone(),
        provider_policy_revision: provider.policy_revision,
        provider_json,
        provider_digest,
    })
}
