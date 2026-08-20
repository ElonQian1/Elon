use std::marker::PhantomData;

use anyhow::{bail, Result};
use rusqlite::Transaction;

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::{
            canonical_projected_active_transition_proof_json_and_digest,
            ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial,
            PROJECTED_ACTIVE_TRANSITION_PROOF_SCHEMA,
        },
        provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
    },
    store::compute_external_pool_adapter_runtime_bundle::{
        PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
        ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject,
    },
};

use super::types::CurrentExternalPoolAdapterCredentialReattestationAuthority;

/// Non-authorizing V253 proof for V277's future atomic registering-to-projection transition.
///
/// It borrows the opaque planned target and retains the exact current registering V253 authority.
/// It cannot be cloned, formatted, serialized, or converted into ordinary active currentness.
pub(in crate::store) struct PreparedExternalPoolAdapterCredentialProjectedActiveTransition<
    'proof,
    'tx,
    'conn,
> {
    credential: &'proof CurrentExternalPoolAdapterCredentialReattestationAuthority,
    planned: &'proof PlannedExternalPoolAdapterActiveNoWorkProbeSubject,
    proof_material_json: String,
    proof_digest: String,
    transaction: PhantomData<&'tx Transaction<'conn>>,
}

impl<'proof, 'tx, 'conn>
    PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'proof, 'tx, 'conn>
{
    pub(in crate::store) fn credential(
        &self,
    ) -> &CurrentExternalPoolAdapterCredentialReattestationAuthority {
        &self.credential
    }

    pub(in crate::store) fn planned(&self) -> &PlannedExternalPoolAdapterActiveNoWorkProbeSubject {
        self.planned
    }

    pub(in crate::store) fn proof_material_json(&self) -> &str {
        &self.proof_material_json
    }

    pub(in crate::store) fn proof_digest(&self) -> &str {
        &self.proof_digest
    }
}

pub(in crate::store) fn prepare_external_pool_adapter_credential_projected_active_transition_on<
    'proof,
    'tx,
    'conn,
>(
    transaction: &'tx Transaction<'conn>,
    no_work: &'proof ReprovedPlannedExternalPoolAdapterActiveNoWorkProbeSubject<'_, 'tx, 'conn>,
) -> Result<PreparedExternalPoolAdapterCredentialProjectedActiveTransition<'proof, 'tx, 'conn>> {
    let _same_transaction = transaction;
    let target = no_work.preflight();
    let evidence_checked_at = no_work.evidence_checked_at();
    if target.activation_target_updated_at() > evidence_checked_at {
        bail!("projected-active transition target uses a different evidence time anchor");
    }
    let activation = &target.activation_root().activation_root;
    let binding_id = &activation.provider_binding_id;
    let credential = no_work.observation().credential_authority();
    let receipt = credential.receipt();
    let observed = &receipt.reattestation.binding;
    let source = &target.source().provider;
    let planned = target.target();
    if credential.checked_at() != evidence_checked_at
        || observed.provider_binding_id != activation.provider_binding_id
        || observed.provider_binding_digest != activation.provider_binding_digest
        || observed.provider_id != activation.source_registering_provider_id
        || observed.observed_provider_policy_revision
            != activation.source_registering_provider_policy_revision
        || observed.observed_provider_digest != activation.source_registering_provider_digest
        || observed.observed_provider_status != PROVIDER_STATUS_REGISTERING
        || observed.adapter_id != activation.logical_adapter_id
        || observed.route_adapter_projection_id != activation.route_adapter_projection_id
        || source.status != PROVIDER_STATUS_REGISTERING
        || source.policy_revision != observed.observed_provider_policy_revision
        || planned.status != PROVIDER_STATUS_ACTIVE
        || planned.policy_revision != source.policy_revision.checked_add(1).unwrap_or(0)
        || planned
            .adapter
            .as_ref()
            .map(|adapter| adapter.adapter_id.as_str())
            != Some(activation.route_adapter_projection_id.as_str())
    {
        bail!(
            "projected-active transition does not bind exact logical source to planned projection"
        );
    }
    let proof = ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial {
        schema: PROJECTED_ACTIVE_TRANSITION_PROOF_SCHEMA.into(),
        provider_binding_id: activation.provider_binding_id.clone(),
        provider_binding_digest: activation.provider_binding_digest.clone(),
        activation_root_digest: target.activation_root().activation_root_digest.clone(),
        source_registering_provider_id: activation.source_registering_provider_id.clone(),
        source_registering_provider_policy_revision: activation
            .source_registering_provider_policy_revision,
        source_registering_provider_json: activation.source_registering_provider_json.clone(),
        source_registering_provider_digest: activation.source_registering_provider_digest.clone(),
        target_active_provider_id: activation.initial_active_provider_id.clone(),
        target_active_provider_policy_revision: activation.initial_active_provider_policy_revision,
        target_active_provider_json: activation.initial_active_provider_json.clone(),
        target_active_provider_digest: activation.initial_active_provider_digest.clone(),
        registering_reattestation_receipt_id: receipt.reattestation_receipt_id.clone(),
        registering_reattestation_receipt_digest: receipt.reattestation_receipt_digest.clone(),
        logical_adapter_id: activation.logical_adapter_id.clone(),
        route_adapter_projection_id: activation.route_adapter_projection_id.clone(),
        evidence_checked_at: evidence_checked_at.into(),
    };
    let (proof_material_json, proof_digest) =
        canonical_projected_active_transition_proof_json_and_digest(&proof)?;
    Ok(
        PreparedExternalPoolAdapterCredentialProjectedActiveTransition {
            credential,
            planned: target,
            proof_material_json,
            proof_digest,
            transaction: PhantomData,
        },
    )
}
