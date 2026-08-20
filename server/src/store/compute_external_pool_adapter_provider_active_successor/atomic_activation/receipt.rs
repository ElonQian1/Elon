use anyhow::{bail, ensure, Result};
use rusqlite::{params_from_iter, types::Value, Connection};

use crate::compute_federation::external_pool_adapter_atomic_activation::{
    canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest,
    canonical_external_pool_adapter_atomic_activation_route_capabilities_json,
    validate_external_pool_adapter_atomic_activation_receipt,
    ExternalPoolAdapterAtomicActivationReceipt,
};

use super::types::StoredExternalPoolAdapterAtomicActivation;

pub(super) const RECEIPT_COLUMNS: &str = "activation_receipt_id,activation_receipt_schema,activation_receipt_digest,activation_receipt_json,canonicalization,digest_algorithm,provider_binding_id,provider_binding_digest,activation_root_digest,source_registering_provider_id,source_registering_provider_policy_revision,source_registering_provider_json,source_registering_provider_digest,target_active_provider_id,target_active_provider_policy_revision,target_active_provider_json,target_active_provider_digest,registering_reattestation_receipt_id,registering_reattestation_receipt_digest,projected_transition_proof_material_json,projected_transition_proof_digest,executor_id,executor_id_hash,executor_id_material_json,executor_binding_material_json,stable_executor_binding_digest,projected_v211_adapter_binding_json,projected_v211_adapter_binding_digest,route_adapter_projection_id,route_adapter_revision,route_adapter_digest,service_actor_id,service_actor_authorization_id,service_actor_authorization_digest,route_credential_id,route_credential_revision,route_credential_digest,route_authorization_id,route_authorization_revision,route_authorization_digest,route_capabilities_json,route_capability_count,route_capability_set_digest,route_capability_0_id,route_capability_0_revision,route_capability_1_id,route_capability_1_revision,route_capability_2_id,route_capability_2_revision,route_capability_3_id,route_capability_3_revision,route_capability_4_id,route_capability_4_revision,route_capability_5_id,route_capability_5_revision,route_seal_id,route_seal_digest,active_runtime_observation_id,active_runtime_observation_digest,observation_started_at,observation_completed_at,observation_expires_at,task_protocol_conformance_run_receipt_id,task_protocol_conformance_run_receipt_digest,task_protocol_conformance_expires_at,task_protocol_active_carrier_material_json,task_protocol_active_carrier_digest,activated_by_actor_kind,activated_by_actor_user_id,idempotency_scope,idempotency_key,idempotency_material_json,idempotency_digest,confirmation,confirmation_material_json,confirmation_digest,activation_target_updated_at,evidence_checked_at,created_at";

pub(in crate::store) fn persist_external_pool_adapter_atomic_activation_receipt_on(
    connection: &Connection,
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<()> {
    let values = receipt_scalar_values(receipt)?;
    let placeholders = (1..=values.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    connection.execute(
        &format!(
            "INSERT INTO compute_external_pool_adapter_atomic_activation_receipts ({RECEIPT_COLUMNS}) VALUES ({placeholders})"
        ),
        params_from_iter(values.iter()),
    )?;
    let stored = super::read::receipt_by_id_on(connection, &receipt.activation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("V277 receipt disappeared after insert"))?;
    audit_stored_receipt(stored, Some(receipt))?;
    Ok(())
}

pub(super) fn audit_stored_receipt(
    stored: StoredExternalPoolAdapterAtomicActivation,
    expected: Option<&ExternalPoolAdapterAtomicActivationReceipt>,
) -> Result<ExternalPoolAdapterAtomicActivationReceipt> {
    validate_external_pool_adapter_atomic_activation_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(&stored.receipt)?;
    ensure!(
        digest == stored.receipt.activation_receipt_digest,
        "V277 receipt digest failed canonical replay"
    );
    let expected_values = receipt_scalar_values_with_json(&stored.receipt, json)?;
    if expected_values != stored.scalar_values {
        bail!("V277 receipt scalar/canonical readback is not exact");
    }
    if expected.is_some_and(|expected| expected != &stored.receipt) {
        bail!("V277 receipt replay conflicts with the expected immutable receipt");
    }
    Ok(stored.receipt)
}

pub(super) fn receipt_scalar_values(
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
) -> Result<Vec<Value>> {
    validate_external_pool_adapter_atomic_activation_receipt(receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(receipt)?;
    ensure!(
        digest == receipt.activation_receipt_digest,
        "V277 receipt digest is not exact"
    );
    receipt_scalar_values_with_json(receipt, json)
}

fn receipt_scalar_values_with_json(
    receipt: &ExternalPoolAdapterAtomicActivationReceipt,
    json: String,
) -> Result<Vec<Value>> {
    let activation = &receipt.activation;
    let identity = &activation.identity;
    let transition = &activation.provider_transition;
    let source = &transition.source_registering_provider;
    let target = &transition.target_active_provider;
    let genesis = &activation.v253_genesis_input;
    let executor = &activation.stable_executor;
    let projected = &activation.projected_v211_binding;
    let route = &activation.route_closure;
    ensure!(
        route.capabilities.len() == 6,
        "V277 route closure is not six-wide"
    );
    let caps_json = canonical_external_pool_adapter_atomic_activation_route_capabilities_json(
        &route.capabilities,
    )?;
    let evidence = &activation.renewable_evidence;
    let audit = &activation.audit;
    let mut values = vec![
        text(&receipt.activation_receipt_id),
        text(&receipt.schema),
        text(&receipt.activation_receipt_digest),
        Value::Text(json),
        text(&receipt.canonicalization),
        text(&receipt.digest_algorithm),
        text(&identity.provider_binding_id),
        text(&identity.provider_binding_digest),
        text(&identity.activation_root_digest),
        text(&source.provider_id),
        integer(source.provider_policy_revision),
        text(&source.provider_json),
        text(&source.provider_digest),
        text(&target.provider_id),
        integer(target.provider_policy_revision),
        text(&target.provider_json),
        text(&target.provider_digest),
        text(&genesis.registering_reattestation_receipt_id),
        text(&genesis.registering_reattestation_receipt_digest),
        text(&genesis.projected_transition_proof_material_json),
        text(&genesis.projected_transition_proof_digest),
        text(&executor.executor_id),
        text(&executor.executor_id_hash),
        text(&executor.executor_id_material_json),
        text(&executor.executor_binding_material_json),
        text(&executor.stable_executor_binding_digest),
        text(&projected.projected_v211_adapter_binding_json),
        text(&projected.projected_v211_adapter_binding_digest),
        text(&route.route_adapter_projection_id),
        integer(route.route_adapter_revision),
        text(&route.route_adapter_digest),
        text(&route.service_actor_id),
        text(&route.service_actor_authorization_id),
        text(&route.service_actor_authorization_digest),
        text(&route.route_credential_id),
        integer(route.route_credential_revision),
        text(&route.route_credential_digest),
        text(&route.route_authorization_id),
        integer(route.route_authorization_revision),
        text(&route.route_authorization_digest),
        Value::Text(caps_json),
        integer(route.capabilities.len() as i64),
        text(&route.route_capability_set_digest),
    ];
    for capability in &route.capabilities {
        values.push(text(&capability.capability_id));
        values.push(integer(capability.capability_revision));
    }
    values.extend([
        text(&route.route_seal_id),
        text(&route.route_seal_digest),
        text(&evidence.active_runtime_observation_id),
        text(&evidence.active_runtime_observation_digest),
        text(&evidence.observation_started_at),
        text(&evidence.observation_completed_at),
        text(&evidence.observation_expires_at),
        text(&evidence.task_protocol_conformance_run_receipt_id),
        text(&evidence.task_protocol_conformance_run_receipt_digest),
        text(&evidence.task_protocol_conformance_expires_at),
        text(&evidence.task_protocol_active_carrier_material_json),
        text(&evidence.task_protocol_active_carrier_digest),
        text(&audit.activated_by_actor_kind),
        text(&audit.activated_by_actor_user_id),
        text(&audit.idempotency_scope),
        text(&audit.idempotency_key),
        text(&audit.idempotency_material_json),
        text(&audit.idempotency_digest),
        text(&audit.confirmation),
        text(&audit.confirmation_material_json),
        text(&audit.confirmation_digest),
        text(&activation.activation_target_updated_at),
        text(&activation.evidence_checked_at),
        text(&activation.created_at),
    ]);
    ensure!(values.len() == 79, "V277 receipt projection is not 79-wide");
    Ok(values)
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn integer(value: i64) -> Value {
    Value::Integer(value)
}
