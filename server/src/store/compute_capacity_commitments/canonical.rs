use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::capacity_commitment::{
        ComputeCapacityCommitment, ComputeCapacityCommitmentQuantity,
        ComputeCapacityCommitmentTerminalReceipt,
    },
    compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256,
};

use super::types::{CancelComputeCapacityCommitment, CreateComputeCapacityCommitment};

const MAX_COMMITMENT_JSON_BYTES: usize = 1024 * 1024;
const COMMITMENT_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-V1";
const TERMINAL_DIGEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-TERMINAL-V1";
const CREATE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-CREATE-REQUEST-V1";
const CANCEL_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-CANCEL-REQUEST-V1";
const EXPIRE_REQUEST_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-EXPIRE-REQUEST-V1";
const EXPIRE_KEY_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-COMMITMENT-EXPIRE-KEY-V1";

pub(super) fn canonical_commitment_json_and_digest(
    commitment: &ComputeCapacityCommitment,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        COMMITMENT_DIGEST_DOMAIN,
        commitment,
        "commitment_digest",
        &commitment.commitment_digest,
    )
}

pub(super) fn canonical_terminal_json_and_digest(
    receipt: &ComputeCapacityCommitmentTerminalReceipt,
) -> Result<(String, String)> {
    envelope_json_and_digest(
        TERMINAL_DIGEST_DOMAIN,
        receipt,
        "terminal_receipt_digest",
        &receipt.terminal_receipt_digest,
    )
}

pub(super) fn create_request_digest(input: &CreateComputeCapacityCommitment) -> Result<String> {
    let quantities = sorted_quantities(&input.quantities);
    domain_digest(
        CREATE_REQUEST_DOMAIN,
        &serde_json::json!({
            "owner_account_id": input.owner_account_id,
            "provider_id": input.provider_id,
            "provider_policy_revision": input.provider_policy_revision,
            "provider_digest": input.provider_digest,
            "offer_id": input.offer_id,
            "offer_version": input.offer_version,
            "offer_digest": input.offer_digest,
            "pool": input.pool,
            "delivery_window": input.delivery_window,
            "price_snapshot_id": input.price_snapshot_id,
            "price_snapshot_digest": input.price_snapshot_digest,
            "reference_binding_id": input.reference_binding_id,
            "reference_binding_digest": input.reference_binding_digest,
            "instrument_id": input.instrument_id,
            "quantities": quantities,
            "idempotency_scope": input.idempotency_scope,
            "idempotency_key": input.idempotency_key,
            "confirmation": input.confirmation,
        }),
    )
}

pub(super) fn cancel_request_digest(input: &CancelComputeCapacityCommitment) -> Result<String> {
    domain_digest(
        CANCEL_REQUEST_DOMAIN,
        &serde_json::json!({
            "owner_account_id": input.owner_account_id,
            "provider_id": input.provider_id,
            "pool_id": input.pool_id,
            "commitment_id": input.commitment_id,
            "expected_commitment_revision": input.expected_commitment_revision,
            "expected_commitment_digest": input.expected_commitment_digest,
            "reason": normalized_reason(&input.reason),
            "idempotency_scope": input.idempotency_scope,
            "idempotency_key": input.idempotency_key,
            "confirmation": input.confirmation,
        }),
    )
}

pub(super) fn expire_request_digest(commitment: &ComputeCapacityCommitment) -> Result<String> {
    domain_digest(
        EXPIRE_REQUEST_DOMAIN,
        &serde_json::json!({
            "commitment_id": commitment.commitment_id,
            "commitment_revision": commitment.commitment_revision,
            "commitment_digest": commitment.commitment_digest,
            "claim": commitment.claim,
            "expires_at": commitment.expires_at,
        }),
    )
}

pub(super) fn expire_idempotency_key(commitment: &ComputeCapacityCommitment) -> Result<String> {
    Ok(format!(
        "expire:{}",
        domain_digest(
            EXPIRE_KEY_DOMAIN,
            &serde_json::json!({
                "commitment_id": commitment.commitment_id,
                "commitment_digest": commitment.commitment_digest,
                "expires_at": commitment.expires_at,
            }),
        )?
    ))
}

pub(super) fn normalized_reason(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn sorted_quantities(
    quantities: &[ComputeCapacityCommitmentQuantity],
) -> Vec<ComputeCapacityCommitmentQuantity> {
    let mut values = quantities.to_vec();
    values.sort_by(|left, right| left.meter.cmp(&right.meter));
    values
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
        .ok_or_else(|| anyhow::anyhow!("容量承诺信封不是 JSON object"))?;
    let mut projection = object.clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("容量承诺信封缺少摘要字段 {digest_field}");
    }
    let digest = domain_digest(domain, &projection)?;
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(envelope, MAX_COMMITMENT_JSON_BYTES)?;
    if !stored_digest.is_empty() && stored_digest != digest {
        bail!("容量承诺信封摘要不一致");
    }
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_COMMITMENT_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
