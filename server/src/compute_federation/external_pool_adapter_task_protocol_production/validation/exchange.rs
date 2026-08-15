use anyhow::{bail, Result};

use super::{super::*, support};

pub(crate) fn validate_task_production_exchange_attempt(
    value: &ExternalPoolAdapterTaskExchangeAttemptEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_EXCHANGE_ATTEMPT_SCHEMA,
        &value.exchange_attempt_id,
        &value.exchange_attempt_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    exchange_identity(&value.attempt.identity)?;
    support::canonical_nanos(&value.attempt.started_at)?;
    support::boundary(&value.attempt.boundary)?;
    if canonical_task_production_exchange_attempt_json_and_digest(value)?.1
        != value.exchange_attempt_digest
    {
        bail!("task production exchange attempt digest is not exact")
    }
    Ok(())
}

pub(crate) fn validate_task_production_exchange_receipt(
    value: &ExternalPoolAdapterTaskExchangeReceiptEnvelope,
) -> Result<()> {
    support::metadata(
        &value.schema,
        TASK_PRODUCTION_EXCHANGE_RECEIPT_SCHEMA,
        &value.exchange_receipt_id,
        &value.exchange_receipt_digest,
        &value.canonicalization,
        &value.digest_algorithm,
    )?;
    let receipt = &value.receipt;
    support::identifier(&receipt.exchange_attempt_id)?;
    support::digest(&receipt.exchange_attempt_digest)?;
    exchange_identity(&receipt.identity)?;
    if receipt.exchange_ordinal == 0
        || receipt.exchange_ordinal > TASK_PRODUCTION_MAX_EXCHANGE_ORDINAL
    {
        bail!("task production exchange ordinal is invalid")
    }
    support::digest(&receipt.exchange_nonce_digest)?;
    bounded_size(
        receipt.upstream_request_bytes,
        TASK_PRODUCTION_MAX_UPSTREAM_REQUEST_BYTES,
        "upstream request",
    )?;
    support::digest(&receipt.upstream_request_sha256)?;
    bounded_size(
        receipt.upstream_response_bytes,
        TASK_PRODUCTION_MAX_RESPONSE_BYTES,
        "upstream response",
    )?;
    support::digest(&receipt.upstream_response_sha256)?;
    bounded_size(
        receipt.semantic_observation_bytes,
        TASK_PRODUCTION_MAX_OBSERVATION_BYTES,
        "semantic observation",
    )?;
    support::digest(&receipt.semantic_observation_sha256)?;
    support::digest(&receipt.session_transcript_digest)?;
    support::digest(&receipt.exchange_root)?;
    if receipt.session_transcript_digest != receipt.identity.session.session_transcript_digest {
        bail!("task production receipt session transcript was substituted")
    }
    let authenticated_at = support::canonical_nanos(&receipt.authenticated_at)?;
    let received_at = support::canonical_nanos(&receipt.received_at)?;
    let recorded_at = support::canonical_nanos(&receipt.recorded_at)?;
    if authenticated_at > received_at || received_at > recorded_at {
        bail!("task production receipt timestamps are out of order")
    }
    support::boundary(&receipt.boundary)?;
    if canonical_task_production_exchange_receipt_json_and_digest(value)?.1
        != value.exchange_receipt_digest
    {
        bail!("task production exchange receipt digest is not exact")
    }
    Ok(())
}

fn exchange_identity(value: &ExternalPoolAdapterTaskExchangeIdentity) -> Result<()> {
    operation_source(value)?;
    support::identifier(&value.source.source_id)?;
    support::digest(&value.source.source_digest)?;
    support::identifier(&value.adapter.provider_id)?;
    support::identifier(&value.adapter.adapter_id)?;
    positive_revision(value.adapter.adapter_revision, "adapter")?;
    support::digest(&value.adapter.adapter_registry_digest)?;
    support::digest(&value.adapter.adapter_implementation_digest)?;
    command(&value.command)?;
    route(&value.route)?;
    support::digest(&value.executor_binding_digest)?;
    positive_revision(value.fencing_generation, "fencing generation")?;
    support::digest(&value.fence_digest)?;
    session(&value.session)?;
    support::digest(&value.request_digest)?;
    support::digest(&value.delivery_attempt_digest)?;
    Ok(())
}

fn operation_source(value: &ExternalPoolAdapterTaskExchangeIdentity) -> Result<()> {
    let expected_source = match value.operation_kind.as_str() {
        "prepare" | "idempotent_commit" | "cancel_no_start" => TASK_PRODUCTION_SOURCE_START_SEND,
        "reconcile" => TASK_PRODUCTION_SOURCE_RECONCILE_POLL,
        "authenticated_events" => TASK_PRODUCTION_SOURCE_EVENT_POLL,
        _ => bail!("task production exchange operation is unknown"),
    };
    if value.source.source_kind != expected_source {
        bail!("task production exchange source does not match operation")
    }
    if expected_source == TASK_PRODUCTION_SOURCE_START_SEND
        && (value.source.source_id != value.command.send_attempt_id
            || value.source.source_digest != value.command.send_attempt_digest)
    {
        bail!("task production start source is not the exact send attempt")
    }
    Ok(())
}

fn command(value: &ExternalPoolAdapterTaskCommandBinding) -> Result<()> {
    for id in [&value.command_id, &value.outbox_id, &value.send_attempt_id] {
        support::identifier(id)?;
    }
    for digest_value in [
        &value.command_digest,
        &value.outbox_digest,
        &value.send_attempt_digest,
    ] {
        support::digest(digest_value)?;
    }
    Ok(())
}

fn route(value: &ExternalPoolAdapterTaskRouteBinding) -> Result<()> {
    for id in [
        &value.route_authorization_id,
        &value.route_credential_id,
        &value.credential_verification_receipt_id,
        &value.credential_verifier_id,
    ] {
        support::identifier(id)?;
    }
    for digest_value in [
        &value.route_authorization_digest,
        &value.route_credential_digest,
        &value.credential_verification_receipt_digest,
        &value.credential_verifier_digest,
    ] {
        support::digest(digest_value)?;
    }
    positive_revision(value.route_credential_revision, "route credential")?;
    positive_revision(value.credential_verifier_revision, "credential verifier")
}

fn session(value: &ExternalPoolAdapterTaskSessionBinding) -> Result<()> {
    support::digest(&value.session_roots_digest)?;
    support::digest(&value.session_transcript_digest)?;
    if value.session_transcript_digest != value.session_roots_digest {
        bail!("task production authenticated session transcript is not exact")
    }
    support::identifier(&value.upstream_transport_target_id)?;
    support::identifier(&value.task_protocol_conformance_run_receipt_id)?;
    for root in [
        &value.roots.supervisor_session_policy_digest,
        &value.roots.runtime_launch_profile_digest,
        &value.roots.task_protocol_profile_digest,
        &value.roots.upstream_transport_target_digest,
        &value.roots.supervisor_session_policy_companion_digest,
        &value.roots.launch_image_sha256,
        &value.roots.ephemeral_task_secret_delivery_root,
        &value.roots.task_protocol_conformance_run_receipt_digest,
    ] {
        support::digest(root)?;
    }
    if task_production_session_roots_digest(&value.roots)? != value.session_roots_digest {
        bail!("task production session root transcript is not exact")
    }
    Ok(())
}

fn positive_revision(value: u64, label: &str) -> Result<()> {
    if value == 0 || value > TASK_PRODUCTION_MAX_SAFE_INTEGER {
        bail!("task production {label} revision is invalid")
    }
    Ok(())
}

fn bounded_size(value: u64, maximum: u64, label: &str) -> Result<()> {
    if value == 0 || value > maximum {
        bail!("task production {label} size is invalid")
    }
    Ok(())
}
