//! Domain-separated canonical digests for recursive policy authentication and currentness.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::super::digest::PlanDigest;
use super::{
    currentness::WindowsRecursivePolicyDispatchAuthorization,
    signature::{
        SignedWindowsRecursiveResolutionPolicyEnvelope,
        WindowsRecursivePolicySignatureVerificationReceipt,
    },
    AuthenticatedWindowsRecursiveResolutionPolicy,
};

pub(super) fn policy_scope_digest(
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
) -> String {
    let mut digest = PlanDigest::new(b"ELON_WINDOWS_RECURSIVE_POLICY_SCOPE_V1");
    digest.text(&policy.launch_context_intent_digest);
    digest.text(&policy.preliminary_request_plan_digest);
    digest.finish()
}

pub(super) fn policy_payload_digest(
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
) -> String {
    let mut digest = PlanDigest::new(b"ELON_WINDOWS_RECURSIVE_RESOLUTION_POLICY_PAYLOAD_V1");
    for value in [
        &policy.launch_context_intent_digest,
        &policy.preliminary_request_plan_digest,
        &policy.parser_policy_digest,
        &policy.authenticated_preloaded_module_set_digest,
    ] {
        digest.text(value);
    }
    for route in &policy.inherited_resolution_route_order {
        digest.text(route);
    }
    for limit in [
        policy.limits.max_wave_count,
        policy.limits.max_parsed_image_count,
        policy.limits.max_module_request_count,
        policy.limits.max_searched_name_count,
        policy.limits.max_system_image_request_count,
        policy.limits.max_forwarder_hop_count,
    ] {
        digest.u64(limit);
    }
    digest.boolean(policy.ambient_path_allowed);
    digest.boolean(policy.nested_api_set_host_redirection_allowed);
    digest.boolean(policy.positive_shadow_disposition_allowed);
    digest.text(&policy.dynamic_module_load_scope);
    digest.text(&policy.control_key_id);
    digest.u64(policy.control_keyring_generation);
    digest.finish()
}

const MAX_POLICY_AUTHORITY_JSON_BYTES: usize = 128 * 1024;

pub(super) fn signed_envelope_digest(
    envelope: &SignedWindowsRecursiveResolutionPolicyEnvelope,
) -> Result<String> {
    jcs_domain_digest(
        b"ELON_WINDOWS_RECURSIVE_POLICY_SIGNED_ENVELOPE_V1",
        json!({
            "schema": envelope.schema,
            "canonicalization": envelope.canonicalization,
            "digest_algorithm": envelope.digest_algorithm,
            "signature_algorithm": envelope.signature_algorithm,
            "signature_domain": envelope.signature_domain,
            "policy_scope_digest": &envelope.policy_scope_digest,
            "policy_generation": envelope.policy_generation,
            "not_before_ms": envelope.not_before_ms,
            "not_after_ms": envelope.not_after_ms,
            "policy_payload_digest": &envelope.policy_payload_digest,
            "signature_profile_digest": &envelope.signature_profile_digest,
            "control_key_id": &envelope.control_key_id,
            "control_key_record_digest": &envelope.control_key_record_digest,
            "control_public_key_spki_digest": &envelope.control_public_key_spki_digest,
            "signing_control_keyring_generation": envelope.signing_control_keyring_generation,
            "signature_material_digest": &envelope.signature_material_digest,
            "signature_bytes_digest": &envelope.signature_bytes_digest,
        }),
    )
}

/// Digest of the exact unsigned JCS envelope material. The Ed25519 message is formed by prefixing
/// the decoded digest with the fixed signature domain and a zero separator; signature bytes are
/// deliberately excluded so the signing contract is acyclic.
pub(super) fn signature_material_digest(
    envelope: &SignedWindowsRecursiveResolutionPolicyEnvelope,
) -> Result<String> {
    let (_, digest) = canonical_compute_plugin_ijson_and_sha256(
        &json!({
            "schema": envelope.schema,
            "canonicalization": envelope.canonicalization,
            "digest_algorithm": envelope.digest_algorithm,
            "signature_algorithm": envelope.signature_algorithm,
            "signature_domain": envelope.signature_domain,
            "policy_scope_digest": &envelope.policy_scope_digest,
            "policy_generation": envelope.policy_generation,
            "not_before_ms": envelope.not_before_ms,
            "not_after_ms": envelope.not_after_ms,
            "policy_payload_digest": &envelope.policy_payload_digest,
            "signature_profile_digest": &envelope.signature_profile_digest,
            "control_key_id": &envelope.control_key_id,
            "control_key_record_digest": &envelope.control_key_record_digest,
            "control_public_key_spki_digest": &envelope.control_public_key_spki_digest,
            "signing_control_keyring_generation": envelope.signing_control_keyring_generation,
        }),
        MAX_POLICY_AUTHORITY_JSON_BYTES,
    )?;
    Ok(digest)
}

pub(super) fn signature_bytes_digest(signature_bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_BYTES_V1");
    digest.update([0]);
    digest.update((signature_bytes.len() as u64).to_le_bytes());
    digest.update(signature_bytes);
    hex::encode(digest.finalize())
}

/// Exact bytes that a future Ed25519 verifier must consume. Keeping this assembly in one helper
/// prevents the verifier, envelope parser and source contract from drifting to different domains
/// or from signing the hexadecimal text instead of the decoded 32-byte digest.
pub(super) fn signature_verification_message(
    envelope: &SignedWindowsRecursiveResolutionPolicyEnvelope,
) -> Result<Vec<u8>> {
    let material_digest = hex::decode(signature_material_digest(envelope)?)?;
    if material_digest.len() != 32 {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_SIGNATURE_DIGEST_LENGTH_CHANGED");
    }
    let mut message = Vec::with_capacity(envelope.signature_domain.len() + 1 + 32);
    message.extend_from_slice(envelope.signature_domain.as_bytes());
    message.push(0);
    message.extend_from_slice(&material_digest);
    Ok(message)
}

pub(super) fn signature_message_digest(
    envelope: &SignedWindowsRecursiveResolutionPolicyEnvelope,
) -> Result<String> {
    let message = signature_verification_message(envelope)?;
    let mut digest = Sha256::new();
    digest.update(b"ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_MESSAGE_V1");
    digest.update([0]);
    digest.update((message.len() as u64).to_le_bytes());
    digest.update(message);
    Ok(hex::encode(digest.finalize()))
}

pub(super) fn signature_verification_receipt_digest(
    receipt: &WindowsRecursivePolicySignatureVerificationReceipt,
) -> Result<String> {
    jcs_domain_digest(
        b"ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_VERIFICATION_RECEIPT_V1",
        json!({
            "schema": receipt.schema,
            "canonicalization": receipt.canonicalization,
            "digest_algorithm": receipt.digest_algorithm,
            "signature_algorithm": receipt.signature_algorithm,
            "signature_domain": receipt.signature_domain,
            "policy_payload_digest": &receipt.policy_payload_digest,
            "signed_envelope_digest": &receipt.signed_envelope_digest,
            "signature_profile_digest": &receipt.signature_profile_digest,
            "signature_material_digest": &receipt.signature_material_digest,
            "signature_bytes_digest": &receipt.signature_bytes_digest,
            "signature_message_digest": &receipt.signature_message_digest,
            "control_key_id": &receipt.control_key_id,
            "control_key_record_digest": &receipt.control_key_record_digest,
            "control_public_key_spki_digest": &receipt.control_public_key_spki_digest,
            "signing_control_keyring_generation": receipt.signing_control_keyring_generation,
            "verified_policy_payload_digest": &receipt.verified_policy_payload_digest,
            "verifier_profile_digest": &receipt.verifier_profile_digest,
            "verified_at_ms": receipt.verified_at_ms,
            "trusted_time_receipt_digest": &receipt.trusted_time_receipt_digest,
            "verification_nonce_digest": &receipt.verification_nonce_digest,
        }),
    )
}

fn jcs_domain_digest(domain: &[u8], material: Value) -> Result<String> {
    let (canonical_json, _) =
        canonical_compute_plugin_ijson_and_sha256(&material, MAX_POLICY_AUTHORITY_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(canonical_json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}

pub(super) fn authenticated_policy_binding_digest(
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
) -> String {
    let mut digest = PlanDigest::new(b"ELON_WINDOWS_AUTHENTICATED_RECURSIVE_POLICY_BINDING_V2");
    digest.text(&policy.policy_payload_digest);
    digest.text(&policy.signed_envelope.signed_envelope_digest);
    digest.text(
        &policy
            .signature_verification
            .signature_verification_receipt_digest,
    );
    digest.finish()
}

pub(super) fn dispatch_authorization_digest(
    authorization: &WindowsRecursivePolicyDispatchAuthorization,
) -> String {
    let mut digest = PlanDigest::new(b"ELON_WINDOWS_RECURSIVE_POLICY_DISPATCH_AUTHORIZATION_V1");
    digest.text(authorization.schema);
    for value in [
        &authorization.authenticated_recursive_policy_digest,
        &authorization.policy_payload_digest,
        &authorization.signed_envelope_digest,
        &authorization.signature_verification_receipt_digest,
        &authorization.policy_scope_digest,
    ] {
        digest.text(value);
    }
    digest.u64(authorization.policy_generation);
    digest.text(&authorization.policy_not_before_ms.to_string());
    digest.text(&authorization.policy_not_after_ms.to_string());
    for value in [
        &authorization.control_key_id,
        &authorization.control_key_record_digest,
        &authorization.control_public_key_spki_digest,
    ] {
        digest.text(value);
    }
    digest.u64(authorization.signing_control_keyring_generation);
    digest.u64(authorization.observed_control_keyring_generation);
    for value in [
        &authorization.control_keyring_snapshot_digest,
        &authorization.active_control_key_record_digest,
        &authorization.active_control_public_key_spki_digest,
        &authorization.control_key_non_revocation_receipt_digest,
        &authorization.control_keyring_anti_rollback_receipt_digest,
    ] {
        digest.text(value);
    }
    digest.u64(authorization.policy_scope_current_generation);
    for value in [
        &authorization.policy_generation_currentness_receipt_digest,
        &authorization.currentness_backend_profile_digest,
    ] {
        digest.text(value);
    }
    digest.text(&authorization.trusted_now_ms.to_string());
    digest.text(&authorization.trusted_time_attestation_sequence.to_string());
    for value in [
        &authorization.trusted_time_receipt_digest,
        &authorization.currentness_observation_receipt_digest,
    ] {
        digest.text(value);
    }
    digest.usize(authorization.acquisition_receipt_ordinal);
    digest.usize(authorization.producer_wave_ordinal);
    for value in [
        &authorization.input_custody_digest,
        &authorization.pre_dispatch_plan_evidence_digest,
        &authorization.authorization_nonce_digest,
    ] {
        digest.text(value);
    }
    digest.finish()
}
