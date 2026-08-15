use anyhow::{ensure, Result};
use rusqlite::types::Value;

use crate::compute_federation::external_pool_adapter_task_protocol_production::{
    canonical_task_production_exchange_attempt_json_and_digest,
    canonical_task_production_exchange_receipt_json_and_digest,
    ExternalPoolAdapterTaskExchangeAttemptEnvelope, ExternalPoolAdapterTaskExchangeIdentity,
    ExternalPoolAdapterTaskExchangeReceiptEnvelope,
};

use super::{canonical_value, integer, text};

pub(in crate::store::compute_external_pool_adapter_task_delivery) fn exchange_attempt_values(
    envelope: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_exchange_attempt_json_and_digest(envelope)?.0;
    let mut values = Vec::with_capacity(52);
    values.extend([
        text(&envelope.exchange_attempt_id),
        text(&envelope.schema),
        text(&envelope.exchange_attempt_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
    ]);
    values.extend(identity_values(&envelope.attempt.identity)?);
    values.extend([
        text(&envelope.attempt.started_at),
        text(&envelope.attempt.boundary.authority_status),
        canonical_value(&envelope.attempt.boundary.effects)?,
        canonical_value(&envelope.attempt.boundary.readiness)?,
    ]);
    ensure!(
        values.len() == 52,
        "V273 exchange attempt mapping is not 52 columns"
    );
    Ok(values)
}

pub(in crate::store::compute_external_pool_adapter_task_delivery) fn exchange_receipt_values(
    envelope: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<Vec<Value>> {
    let canonical = canonical_task_production_exchange_receipt_json_and_digest(envelope)?.0;
    let receipt = &envelope.receipt;
    let mut values = Vec::with_capacity(65);
    values.extend([
        text(&envelope.exchange_receipt_id),
        text(&envelope.schema),
        text(&envelope.exchange_receipt_digest),
        Value::Text(canonical),
        text(&envelope.canonicalization),
        text(&envelope.digest_algorithm),
        text(&receipt.exchange_attempt_id),
        text(&receipt.exchange_attempt_digest),
    ]);
    values.extend(identity_values(&receipt.identity)?);
    values.extend([
        integer(receipt.exchange_ordinal)?,
        text(&receipt.exchange_nonce_digest),
        integer(receipt.upstream_request_bytes)?,
        text(&receipt.upstream_request_sha256),
        integer(receipt.upstream_response_bytes)?,
        text(&receipt.upstream_response_sha256),
        integer(receipt.semantic_observation_bytes)?,
        text(&receipt.semantic_observation_sha256),
        text(&receipt.exchange_root),
        text(&receipt.authenticated_at),
        text(&receipt.received_at),
        text(&receipt.recorded_at),
        text(&receipt.boundary.authority_status),
        canonical_value(&receipt.boundary.effects)?,
        canonical_value(&receipt.boundary.readiness)?,
    ]);
    ensure!(
        values.len() == 65,
        "V273 exchange receipt mapping is not 65 columns"
    );
    Ok(values)
}

fn identity_values(identity: &ExternalPoolAdapterTaskExchangeIdentity) -> Result<Vec<Value>> {
    let route = &identity.route;
    let roots = &identity.session.roots;
    Ok(vec![
        text(&identity.operation_kind),
        text(&identity.source.source_kind),
        text(&identity.source.source_id),
        text(&identity.source.source_digest),
        text(&identity.adapter.provider_id),
        text(&identity.adapter.adapter_id),
        integer(identity.adapter.adapter_revision)?,
        text(&identity.adapter.adapter_registry_digest),
        text(&identity.adapter.adapter_implementation_digest),
        text(&identity.command.command_id),
        text(&identity.command.command_digest),
        text(&identity.command.outbox_id),
        text(&identity.command.outbox_digest),
        text(&identity.command.send_attempt_id),
        text(&identity.command.send_attempt_digest),
        text(&route.route_authorization_id),
        text(&route.route_authorization_digest),
        text(&route.route_credential_id),
        integer(route.route_credential_revision)?,
        text(&route.route_credential_digest),
        text(&route.credential_verification_receipt_id),
        text(&route.credential_verification_receipt_digest),
        text(&route.credential_verifier_id),
        integer(route.credential_verifier_revision)?,
        text(&route.credential_verifier_digest),
        text(&identity.executor_binding_digest),
        integer(identity.fencing_generation)?,
        text(&identity.fence_digest),
        text(&roots.supervisor_session_policy_digest),
        text(&roots.runtime_launch_profile_digest),
        text(&roots.task_protocol_profile_digest),
        text(&identity.session.upstream_transport_target_id),
        text(&roots.upstream_transport_target_digest),
        text(&roots.supervisor_session_policy_companion_digest),
        text(&roots.launch_image_sha256),
        text(&roots.ephemeral_task_secret_delivery_root),
        text(&identity.session.task_protocol_conformance_run_receipt_id),
        text(&roots.task_protocol_conformance_run_receipt_digest),
        text(&identity.session.session_roots_digest),
        text(&identity.session.session_transcript_digest),
        text(&identity.request_digest),
        text(&identity.delivery_attempt_digest),
    ])
}
