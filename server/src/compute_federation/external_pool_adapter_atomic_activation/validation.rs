use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use crate::compute_federation::{
    provider::{
        ComputeProvider, PROVIDER_KIND_EXTERNAL_POOL, PROVIDER_STATUS_ACTIVE,
        PROVIDER_STATUS_REGISTERING,
    },
    route_authority::{
        canonical_route_capability_set_digest, COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS, COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
        COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT, COMPUTE_ROUTE_CAPABILITY_PREPARE,
        COMPUTE_ROUTE_CAPABILITY_RECONCILE, COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT,
    },
};

use super::{
    canonical_atomic_activation_confirmation_json_and_digest,
    canonical_atomic_activation_idempotency_json_and_digest,
    canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest,
    canonical_projected_active_transition_proof_json_and_digest,
    canonical_task_protocol_active_carrier_json_and_digest,
    derive_external_pool_projected_v211_adapter_binding, derive_external_pool_stable_executor,
    ExternalPoolAdapterAtomicActivationConfirmationMaterial,
    ExternalPoolAdapterAtomicActivationIdempotencyMaterial,
    ExternalPoolAdapterAtomicActivationReceipt,
    ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial,
    ExternalPoolAdapterTaskProtocolActiveCarrierMaterial,
    ExternalPoolStableExecutorBindingMaterial, ExternalPoolStableExecutorIdMaterial,
    ATOMIC_ACTIVATION_ACTOR_KIND, ATOMIC_ACTIVATION_CANONICALIZATION,
    ATOMIC_ACTIVATION_CONFIRMATION, ATOMIC_ACTIVATION_DIGEST_ALGORITHM,
    ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE, ATOMIC_ACTIVATION_RECEIPT_SCHEMA,
};

pub(crate) fn validate_external_pool_adapter_atomic_activation_receipt(
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    identifier(&receipt.activation_receipt_id)?;
    digest(&receipt.activation_receipt_digest)?;
    if receipt.schema != ATOMIC_ACTIVATION_RECEIPT_SCHEMA
        || receipt.canonicalization != ATOMIC_ACTIVATION_CANONICALIZATION
        || receipt.digest_algorithm != ATOMIC_ACTIVATION_DIGEST_ALGORITHM
        || canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(receipt)?.1
            != receipt.activation_receipt_digest
    {
        bail!("V277 activation receipt envelope is not exact")
    }
    validate_provider_transition(receipt)?;
    validate_transition(receipt)?;
    validate_projected_binding(receipt)?;
    validate_executor(receipt)?;
    validate_route(receipt)?;
    validate_carrier(receipt)?;
    validate_audit(receipt)?;
    validate_times(receipt)
}

fn validate_provider_transition(
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let activation = &receipt.activation;
    let source = &activation.provider_transition.source_registering_provider;
    let target = &activation.provider_transition.target_active_provider;
    let source_provider: ComputeProvider = serde_json::from_str(&source.provider_json)?;
    let target_provider: ComputeProvider = serde_json::from_str(&target.provider_json)?;
    let mut expected_target = source_provider.clone();
    let expected_revision = source
        .provider_policy_revision
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V277 Provider revision overflow"))?;
    expected_target.status = PROVIDER_STATUS_ACTIVE.into();
    expected_target.policy_revision = expected_revision;
    expected_target.updated_at = activation.activation_target_updated_at.clone();
    expected_target
        .adapter
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("V277 registering Provider lacks logical Adapter"))?
        .adapter_id = activation.route_closure.route_adapter_projection_id.clone();
    if source.provider_id != target.provider_id
        || source.provider_id != source_provider.provider_id
        || target.provider_id != target_provider.provider_id
        || source.provider_policy_revision != source_provider.policy_revision
        || target.provider_policy_revision != target_provider.policy_revision
        || target.provider_policy_revision != expected_revision
        || source_provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || target_provider.provider_kind != PROVIDER_KIND_EXTERNAL_POOL
        || source_provider.status != PROVIDER_STATUS_REGISTERING
        || target_provider.status != PROVIDER_STATUS_ACTIVE
        || target_provider.updated_at != activation.activation_target_updated_at
        || target_provider != expected_target
        || serde_json::to_string(&source_provider)? != source.provider_json
        || serde_json::to_string(&target_provider)? != target.provider_json
        || sha256(&source.provider_json) != source.provider_digest
        || sha256(&target.provider_json) != target.provider_digest
    {
        bail!("V277 Provider transition is not exact adjacent projected-active")
    }
    Ok(())
}

fn validate_transition(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let input = &activation.v253_genesis_input;
    let material: ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial =
        serde_json::from_str(&input.projected_transition_proof_material_json)?;
    let (json, digest) = canonical_projected_active_transition_proof_json_and_digest(&material)?;
    let transition = &activation.provider_transition;
    let source = &transition.source_registering_provider;
    let target = &transition.target_active_provider;
    let source_provider: ComputeProvider = serde_json::from_str(&source.provider_json)?;
    let logical_adapter_id = &source_provider
        .adapter
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("V277 transition source lacks logical Adapter"))?
        .adapter_id;
    if json != input.projected_transition_proof_material_json
        || digest != input.projected_transition_proof_digest
        || material.provider_binding_id != activation.identity.provider_binding_id
        || material.provider_binding_digest != activation.identity.provider_binding_digest
        || material.activation_root_digest != activation.identity.activation_root_digest
        || material.registering_reattestation_receipt_id
            != input.registering_reattestation_receipt_id
        || material.registering_reattestation_receipt_digest
            != input.registering_reattestation_receipt_digest
        || material.source_registering_provider_id != source.provider_id
        || material.source_registering_provider_policy_revision != source.provider_policy_revision
        || material.source_registering_provider_json != source.provider_json
        || material.source_registering_provider_digest != source.provider_digest
        || material.target_active_provider_id != target.provider_id
        || material.target_active_provider_policy_revision != target.provider_policy_revision
        || material.target_active_provider_json != target.provider_json
        || material.target_active_provider_digest != target.provider_digest
        || material.logical_adapter_id != *logical_adapter_id
        || material.route_adapter_projection_id
            != activation.route_closure.route_adapter_projection_id
        || material.evidence_checked_at != activation.evidence_checked_at
    {
        bail!("V277 projected transition proof is not exact")
    }
    Ok(())
}

fn validate_projected_binding(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let target: ComputeProvider = serde_json::from_str(
        &activation
            .provider_transition
            .target_active_provider
            .provider_json,
    )?;
    let (_, expected) = derive_external_pool_projected_v211_adapter_binding(
        &target,
        &activation.route_closure.route_adapter_projection_id,
    )?;
    if expected != activation.projected_v211_binding {
        bail!("V277 projected v211 Adapter binding is not exact")
    }
    Ok(())
}

fn validate_executor(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let executor = &activation.stable_executor;
    let id_material: ExternalPoolStableExecutorIdMaterial =
        serde_json::from_str(&executor.executor_id_material_json)?;
    let binding_material: ExternalPoolStableExecutorBindingMaterial =
        serde_json::from_str(&executor.executor_binding_material_json)?;
    let expected = derive_external_pool_stable_executor(
        id_material.clone(),
        binding_material
            .logical_projection_compatibility_digest
            .clone(),
        activation
            .projected_v211_binding
            .projected_v211_adapter_binding_digest
            .clone(),
        binding_material.lane_subject_digest.clone(),
    )?;
    if expected != *executor
        || binding_material.executor_id != executor.executor_id
        || id_material.provider_binding_id != activation.identity.provider_binding_id
        || id_material.provider_binding_digest != activation.identity.provider_binding_digest
        || id_material.activation_root_digest != activation.identity.activation_root_digest
        || id_material.route_adapter_projection_id
            != activation.route_closure.route_adapter_projection_id
        || id_material.service_actor_id != activation.route_closure.service_actor_id
        || binding_material.provider_binding_id != activation.identity.provider_binding_id
        || binding_material.provider_binding_digest != activation.identity.provider_binding_digest
        || binding_material.activation_root_digest != activation.identity.activation_root_digest
        || binding_material.route_adapter_projection_id
            != activation.route_closure.route_adapter_projection_id
        || binding_material.service_actor_id != activation.route_closure.service_actor_id
        || binding_material.task_production_carrier_policy_digest
            != id_material.task_production_carrier_policy_digest
        || binding_material.projected_v211_adapter_binding_digest
            != activation
                .projected_v211_binding
                .projected_v211_adapter_binding_digest
    {
        bail!("V277 stable executor binding is not exact")
    }
    Ok(())
}

fn validate_route(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let route = &receipt.activation.route_closure;
    let target: ComputeProvider = serde_json::from_str(
        &receipt
            .activation
            .provider_transition
            .target_active_provider
            .provider_json,
    )?;
    let expected = [
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_ACK,
        COMPUTE_ROUTE_CAPABILITY_AUTHENTICATED_EVENTS,
        COMPUTE_ROUTE_CAPABILITY_CANCEL_NO_START,
        COMPUTE_ROUTE_CAPABILITY_IDEMPOTENT_COMMIT,
        COMPUTE_ROUTE_CAPABILITY_PREPARE,
        COMPUTE_ROUTE_CAPABILITY_RECONCILE,
    ];
    if route.route_capability_count != COMPUTE_ROUTE_REQUIRED_CAPABILITY_COUNT
        || route.capabilities.len() != route.route_capability_count as usize
        || route
            .capabilities
            .iter()
            .enumerate()
            .any(|(ordinal, capability)| {
                capability.ordinal != ordinal as i64
                    || capability.capability_id != expected[ordinal]
                    || capability.capability_revision <= 0
            })
        || canonical_route_capability_set_digest(&route.capabilities)?
            != route.route_capability_set_digest
        || target
            .adapter
            .as_ref()
            .is_none_or(|adapter| adapter.adapter_id != route.route_adapter_projection_id)
    {
        bail!("V277 route closure lacks the exact ordered six capabilities")
    }
    Ok(())
}

fn validate_carrier(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let evidence = &activation.renewable_evidence;
    let material: ExternalPoolAdapterTaskProtocolActiveCarrierMaterial =
        serde_json::from_str(&evidence.task_protocol_active_carrier_material_json)?;
    let (json, digest) = canonical_task_protocol_active_carrier_json_and_digest(&material)?;
    if json != evidence.task_protocol_active_carrier_material_json
        || digest != evidence.task_protocol_active_carrier_digest
        || material.provider_binding_id != activation.identity.provider_binding_id
        || material.provider_binding_digest != activation.identity.provider_binding_digest
        || material.activation_root_digest != activation.identity.activation_root_digest
        || material.target_active_provider_id
            != activation
                .provider_transition
                .target_active_provider
                .provider_id
        || material.target_active_provider_policy_revision
            != activation
                .provider_transition
                .target_active_provider
                .provider_policy_revision
        || material.target_active_provider_digest
            != activation
                .provider_transition
                .target_active_provider
                .provider_digest
        || material.route_adapter_projection_id
            != activation.route_closure.route_adapter_projection_id
        || material.task_protocol_conformance_run_receipt_id
            != evidence.task_protocol_conformance_run_receipt_id
        || material.task_protocol_conformance_run_receipt_digest
            != evidence.task_protocol_conformance_run_receipt_digest
    {
        bail!("V277 task-protocol active carrier is not exact")
    }
    Ok(())
}

fn validate_audit(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let audit = &activation.audit;
    let idempotency: ExternalPoolAdapterAtomicActivationIdempotencyMaterial =
        serde_json::from_str(&audit.idempotency_material_json)?;
    let confirmation: ExternalPoolAdapterAtomicActivationConfirmationMaterial =
        serde_json::from_str(&audit.confirmation_material_json)?;
    let (idempotency_json, idempotency_digest) =
        canonical_atomic_activation_idempotency_json_and_digest(&idempotency)?;
    let (confirmation_json, confirmation_digest) =
        canonical_atomic_activation_confirmation_json_and_digest(&confirmation)?;
    let target: ComputeProvider = serde_json::from_str(
        &activation
            .provider_transition
            .target_active_provider
            .provider_json,
    )?;
    if audit.activated_by_actor_kind != ATOMIC_ACTIVATION_ACTOR_KIND
        || audit.activated_by_actor_user_id != target.owner_account_id
        || audit.idempotency_scope != ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE
        || audit.idempotency_key != activation.identity.activation_root_digest
        || audit.confirmation != ATOMIC_ACTIVATION_CONFIRMATION
        || audit.idempotency_material_json != idempotency_json
        || audit.idempotency_digest != idempotency_digest
        || audit.confirmation_material_json != confirmation_json
        || audit.confirmation_digest != confirmation_digest
        || idempotency.actor_user_id != audit.activated_by_actor_user_id
        || idempotency.actor_kind != audit.activated_by_actor_kind
        || idempotency.provider_binding_id != activation.identity.provider_binding_id
        || idempotency.provider_binding_digest != activation.identity.provider_binding_digest
        || idempotency.activation_root_digest != activation.identity.activation_root_digest
        || idempotency.scope != audit.idempotency_scope
        || idempotency.key != audit.idempotency_key
        || confirmation.actor_user_id != audit.activated_by_actor_user_id
        || confirmation.actor_kind != audit.activated_by_actor_kind
        || confirmation.idempotency_digest != audit.idempotency_digest
        || confirmation.provider_binding_id != activation.identity.provider_binding_id
        || confirmation.provider_binding_digest != activation.identity.provider_binding_digest
        || confirmation.activation_root_digest != activation.identity.activation_root_digest
    {
        bail!("V277 audit/idempotency material is not Store-derived exact authority")
    }
    Ok(())
}

fn validate_times(receipt: &ExternalPoolAdapterAtomicActivationReceipt) -> Result<()> {
    let activation = &receipt.activation;
    let source: ComputeProvider = serde_json::from_str(
        &activation
            .provider_transition
            .source_registering_provider
            .provider_json,
    )?;
    let started = canonical_time(&activation.renewable_evidence.observation_started_at)?;
    let completed = canonical_time(&activation.renewable_evidence.observation_completed_at)?;
    let expires = canonical_time(&activation.renewable_evidence.observation_expires_at)?;
    let protocol_expires = canonical_time(
        &activation
            .renewable_evidence
            .task_protocol_conformance_expires_at,
    )?;
    let target = canonical_time(&activation.activation_target_updated_at)?;
    let evidence = canonical_time(&activation.evidence_checked_at)?;
    if canonical_time(&source.updated_at)? > target
        || target > started
        || started > completed
        || completed > evidence
        || evidence >= expires
        || expires > protocol_expires
        || activation.created_at != activation.evidence_checked_at
    {
        bail!("V277 dual-time freshness chain is invalid")
    }
    Ok(())
}

fn identifier(value: &str) -> Result<()> {
    if value.is_empty() || value != value.trim() || value.len() > 240 {
        bail!("V277 identifier is not exact")
    }
    Ok(())
}

fn digest(value: &str) -> Result<()> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        bail!("V277 digest is not lowercase SHA-256")
    }
    Ok(())
}

fn canonical_time(value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("V277 timestamp is not canonical UTC nanoseconds")
    }
    Ok(parsed)
}

fn sha256(value: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(value.as_bytes()))
}
