use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::delivery_allocation::{
        ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::types::{
    CreateComputeDeliveryAllocationGrant, DeclineComputeDeliveryAllocationGrant,
    ExerciseComputeDeliveryAllocationGrant,
};

const MAX_ALLOCATION_JSON_BYTES: usize = 1024 * 1024;
const GRANT_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-GRANT-V1";
const TERMINAL_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-TERMINAL-V1";
const CREATE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-GRANT-REQUEST-V1";
const EXERCISE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-EXERCISE-REQUEST-V1";
const DECLINE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-DECLINE-REQUEST-V1";
const EXPIRE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-EXPIRE-REQUEST-V1";
const EXPIRE_KEY_DOMAIN: &[u8] = b"ELON-COMPUTE-DELIVERY-ALLOCATION-EXPIRE-KEY-V1";

pub(super) fn canonical_grant_json_and_digest(
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        GRANT_DIGEST_DOMAIN,
        grant,
        "grant_digest",
        &grant.grant_digest,
    )
}

pub(super) fn canonical_terminal_json_and_digest(
    terminal: &ComputeDeliveryAllocationTerminalReceipt,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        TERMINAL_DIGEST_DOMAIN,
        terminal,
        "terminal_receipt_digest",
        &terminal.terminal_receipt_digest,
    )
}

pub(super) fn create_request_digest(
    input: &CreateComputeDeliveryAllocationGrant,
) -> Result<String> {
    domain_digest(
        CREATE_REQUEST_DOMAIN,
        &serde_json::json!({
            "provider_owner_account_id": input.provider_owner_account_id,
            "provider_id": input.provider_id,
            "pool_id": input.pool_id,
            "commitment_id": input.commitment_id,
            "expected_commitment_revision": input.expected_commitment_revision,
            "expected_commitment_digest": input.expected_commitment_digest,
            "consumer_account_id": input.consumer_account_id,
            "job_id": input.job_id,
            "expected_job_revision": input.expected_job_revision,
            "expected_job_digest": input.expected_job_digest,
            "idempotency_scope": input.idempotency_scope,
            "idempotency_key": input.idempotency_key,
            "confirmation": input.confirmation,
        }),
    )
}

pub(super) fn exercise_request_digest(
    input: &ExerciseComputeDeliveryAllocationGrant,
) -> Result<String> {
    domain_digest(
        EXERCISE_REQUEST_DOMAIN,
        &serde_json::json!({
            "consumer_account_id": input.consumer_account_id,
            "grant_id": input.grant_id,
            "reservation_id": input.reservation_id,
            "expected_grant_revision": input.expected_grant_revision,
            "expected_grant_digest": input.expected_grant_digest,
            "idempotency_scope": input.idempotency_scope,
            "idempotency_key": input.idempotency_key,
            "confirmation": input.confirmation,
        }),
    )
}

pub(super) fn decline_request_digest(
    input: &DeclineComputeDeliveryAllocationGrant,
) -> Result<String> {
    domain_digest(
        DECLINE_REQUEST_DOMAIN,
        &serde_json::json!({
            "consumer_account_id": input.consumer_account_id,
            "grant_id": input.grant_id,
            "expected_grant_revision": input.expected_grant_revision,
            "expected_grant_digest": input.expected_grant_digest,
            "idempotency_scope": input.idempotency_scope,
            "idempotency_key": input.idempotency_key,
            "confirmation": input.confirmation,
        }),
    )
}

pub(super) fn expire_request_digest(grant: &ComputeDeliveryAllocationGrant) -> Result<String> {
    domain_digest(
        EXPIRE_REQUEST_DOMAIN,
        &serde_json::json!({
            "grant_id": grant.grant_id,
            "grant_revision": grant.grant_revision,
            "grant_digest": grant.grant_digest,
            "commitment": grant.commitment,
            "exercise_expires_at": grant.exercise_expires_at,
        }),
    )
}

pub(super) fn expire_idempotency_key(grant: &ComputeDeliveryAllocationGrant) -> Result<String> {
    Ok(format!(
        "expire:{}",
        domain_digest(
            EXPIRE_KEY_DOMAIN,
            &serde_json::json!({
                "grant_id": grant.grant_id,
                "grant_digest": grant.grant_digest,
                "exercise_expires_at": grant.exercise_expires_at,
            }),
        )?
    ))
}

fn envelope_json_and_digest<E: Serialize>(
    domain: &[u8],
    envelope: &E,
    digest_field: &str,
    stored_digest: &str,
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("DeliveryAllocation 信封不是 JSON object"))?;
    let mut projection = object.clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("DeliveryAllocation 信封缺少摘要字段 {digest_field}");
    }
    let digest = domain_digest(domain, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(envelope, MAX_ALLOCATION_JSON_BYTES)?;
    if !stored_digest.is_empty() && stored_digest != digest {
        bail!("DeliveryAllocation 信封摘要不一致");
    }
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_ALLOCATION_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
