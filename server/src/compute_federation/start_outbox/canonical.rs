use anyhow::{ensure, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeAttemptDispatchActorReceiptEnvelope, ComputeLeaseAuthorityBindingEnvelope,
    ComputeStartNoStartProofEnvelope, ComputeStartOutboxClaimReceiptEnvelope,
    ComputeStartOutboxOperationEnvelope, ComputeStartOutboxRemoteObservationEnvelope,
    ComputeStartOutboxSendAttemptEnvelope,
};

const MAX_START_OUTBOX_JSON_BYTES: usize = 2 * 1024 * 1024;
const OPERATION_DOMAIN: &[u8] = b"ELON-COMPUTE-START-OUTBOX-OPERATION-V1";
const CLAIM_RECEIPT_DOMAIN: &[u8] = b"ELON-COMPUTE-START-OUTBOX-CLAIM-RECEIPT-V1";
const SEND_ATTEMPT_DOMAIN: &[u8] = b"ELON-COMPUTE-START-OUTBOX-SEND-ATTEMPT-V1";
const REMOTE_OBSERVATION_DOMAIN: &[u8] = b"ELON-COMPUTE-START-OUTBOX-REMOTE-OBSERVATION-V1";
const NO_START_PROOF_DOMAIN: &[u8] = b"ELON-COMPUTE-START-NO-START-PROOF-V1";
const LEASE_AUTHORITY_DOMAIN: &[u8] = b"ELON-COMPUTE-LEASE-AUTHORITY-BINDING-V1";
const LEASE_AUTHORITY_SCOPES_DOMAIN: &[u8] = b"ELON-COMPUTE-LEASE-AUTHORITY-SCOPES-V1";
const ACTOR_RECEIPT_DOMAIN: &[u8] = b"ELON-COMPUTE-ATTEMPT-DISPATCH-ACTOR-RECEIPT-V1";

pub(crate) fn canonical_start_outbox_operation_json_and_digest(
    envelope: &ComputeStartOutboxOperationEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(OPERATION_DOMAIN, "outbox_digest", envelope)
}

pub(crate) fn canonical_start_outbox_claim_receipt_json_and_digest(
    envelope: &ComputeStartOutboxClaimReceiptEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(CLAIM_RECEIPT_DOMAIN, "claim_receipt_digest", envelope)
}

pub(crate) fn canonical_start_outbox_send_attempt_json_and_digest(
    envelope: &ComputeStartOutboxSendAttemptEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(SEND_ATTEMPT_DOMAIN, "send_attempt_digest", envelope)
}

pub(crate) fn canonical_start_outbox_remote_observation_json_and_digest(
    envelope: &ComputeStartOutboxRemoteObservationEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(REMOTE_OBSERVATION_DOMAIN, "observation_digest", envelope)
}

pub(crate) fn canonical_start_no_start_proof_json_and_digest(
    envelope: &ComputeStartNoStartProofEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(NO_START_PROOF_DOMAIN, "proof_digest", envelope)
}

pub(crate) fn canonical_lease_authority_binding_json_and_digest(
    envelope: &ComputeLeaseAuthorityBindingEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(LEASE_AUTHORITY_DOMAIN, "lease_authority_digest", envelope)
}

pub(crate) fn canonical_lease_authority_scopes_digest(scopes: &[String]) -> Result<String> {
    domain_digest(LEASE_AUTHORITY_SCOPES_DOMAIN, scopes)
}

pub(crate) fn canonical_attempt_dispatch_actor_receipt_json_and_digest(
    envelope: &ComputeAttemptDispatchActorReceiptEnvelope,
) -> Result<(String, String)> {
    envelope_json_and_digest(ACTOR_RECEIPT_DOMAIN, "actor_receipt_digest", envelope)
}

fn envelope_json_and_digest<E: Serialize>(
    domain: &[u8],
    digest_field: &str,
    envelope: &E,
) -> Result<(String, String)> {
    let mut projection = serde_json::to_value(envelope)?;
    let object = projection.as_object_mut();
    ensure!(
        object.is_some(),
        "authority envelope must serialize as an object"
    );
    let removed = object.and_then(|value| value.remove(digest_field));
    ensure!(
        removed.is_some(),
        "authority envelope is missing digest field {digest_field}"
    );
    let digest = domain_digest(domain, &projection)?;
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(envelope, MAX_START_OUTBOX_JSON_BYTES)?;
    Ok((json, digest))
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) = canonical_compute_plugin_ijson_and_sha256(value, MAX_START_OUTBOX_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
