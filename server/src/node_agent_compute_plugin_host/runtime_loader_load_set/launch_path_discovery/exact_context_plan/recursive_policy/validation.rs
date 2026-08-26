//! Borrow-only validation for signed policy and point-of-use dispatch currentness.

use anyhow::{bail, Result};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

use super::{
    currentness::{
        WindowsRecursivePolicyDispatchAuthorization, POLICY_DISPATCH_AUTHORIZATION_SCHEMA,
    },
    digest,
    signature::{
        POLICY_CANONICALIZATION, POLICY_DIGEST_ALGORITHM, POLICY_SIGNATURE_ALGORITHM,
        POLICY_SIGNATURE_DOMAIN, SIGNATURE_VERIFICATION_RECEIPT_SCHEMA,
        SIGNED_POLICY_ENVELOPE_SCHEMA,
    },
    AuthenticatedWindowsRecursiveResolutionPolicy, RECURSIVE_DYNAMIC_LOAD_SCOPE,
};

pub(super) fn validate_policy_against(
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
    expected_launch_context_intent_digest: &str,
    expected_preliminary_request_plan_digest: &str,
    expected_parser_policy_digest: &str,
    expected_authenticated_preloaded_module_set_digest: &str,
    expected_resolution_route_order: &[String],
) -> Result<()> {
    if policy.launch_context_intent_digest != expected_launch_context_intent_digest
        || policy.preliminary_request_plan_digest != expected_preliminary_request_plan_digest
        || policy.parser_policy_digest != expected_parser_policy_digest
        || policy.authenticated_preloaded_module_set_digest
            != expected_authenticated_preloaded_module_set_digest
        || policy.inherited_resolution_route_order != expected_resolution_route_order
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_SOURCE_CHANGED");
    }
    validate_policy_source(policy)
}

pub(super) fn validate_policy_source(
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
) -> Result<()> {
    let envelope = &policy.signed_envelope;
    let verification = &policy.signature_verification;
    let expected_signature_material_digest = digest::signature_material_digest(envelope)?;
    let expected_signature_bytes_digest = digest::signature_bytes_digest(&envelope.signature_bytes);
    let expected_signature_message_digest = digest::signature_message_digest(envelope)?;
    let expected_signed_envelope_digest = digest::signed_envelope_digest(envelope)?;
    let expected_signature_verification_receipt_digest =
        digest::signature_verification_receipt_digest(verification)?;
    if policy.ambient_path_allowed
        || policy.nested_api_set_host_redirection_allowed
        || policy.positive_shadow_disposition_allowed
        || policy.dynamic_module_load_scope != RECURSIVE_DYNAMIC_LOAD_SCOPE
        || !valid_control_key_id(&policy.control_key_id)
        || policy.control_keyring_generation == 0
        || envelope.schema != SIGNED_POLICY_ENVELOPE_SCHEMA
        || envelope.canonicalization != POLICY_CANONICALIZATION
        || envelope.digest_algorithm != POLICY_DIGEST_ALGORITHM
        || envelope.signature_algorithm != POLICY_SIGNATURE_ALGORITHM
        || envelope.signature_domain != POLICY_SIGNATURE_DOMAIN
        || envelope.policy_generation == 0
        || envelope.not_before_ms < 0
        || envelope.not_before_ms >= envelope.not_after_ms
        || envelope.signing_control_keyring_generation == 0
        || envelope.signature_bytes.len() != 64
        || !valid_control_key_id(&envelope.control_key_id)
        || envelope.control_key_id != policy.control_key_id
        || envelope.signing_control_keyring_generation != policy.control_keyring_generation
        || digest::policy_scope_digest(policy) != envelope.policy_scope_digest
        || digest::policy_payload_digest(policy) != policy.policy_payload_digest
        || envelope.policy_payload_digest != policy.policy_payload_digest
        || expected_signature_material_digest != envelope.signature_material_digest
        || expected_signature_bytes_digest != envelope.signature_bytes_digest
        || expected_signed_envelope_digest != envelope.signed_envelope_digest
        || verification.schema != SIGNATURE_VERIFICATION_RECEIPT_SCHEMA
        || verification.canonicalization != envelope.canonicalization
        || verification.digest_algorithm != envelope.digest_algorithm
        || verification.signature_algorithm != envelope.signature_algorithm
        || verification.signature_domain != envelope.signature_domain
        || verification.policy_payload_digest != policy.policy_payload_digest
        || verification.signed_envelope_digest != envelope.signed_envelope_digest
        || verification.signature_profile_digest != envelope.signature_profile_digest
        || verification.signature_material_digest != envelope.signature_material_digest
        || verification.signature_bytes_digest != envelope.signature_bytes_digest
        || verification.signature_message_digest != expected_signature_message_digest
        || verification.control_key_id != envelope.control_key_id
        || verification.control_key_record_digest != envelope.control_key_record_digest
        || verification.control_public_key_spki_digest != envelope.control_public_key_spki_digest
        || verification.signing_control_keyring_generation
            != envelope.signing_control_keyring_generation
        || verification.verified_policy_payload_digest != policy.policy_payload_digest
        || verification.verified_at_ms < 0
        || expected_signature_verification_receipt_digest
            != verification.signature_verification_receipt_digest
        || digest::authenticated_policy_binding_digest(policy)
            != policy.authenticated_recursive_policy_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_AUTHENTICATION_CHANGED");
    }

    if [
        &policy.launch_context_intent_digest,
        &policy.preliminary_request_plan_digest,
        &policy.parser_policy_digest,
        &policy.authenticated_preloaded_module_set_digest,
        &policy.policy_payload_digest,
        &policy.authenticated_recursive_policy_digest,
        &envelope.policy_scope_digest,
        &envelope.signature_profile_digest,
        &envelope.control_key_record_digest,
        &envelope.control_public_key_spki_digest,
        &envelope.signature_material_digest,
        &envelope.signature_bytes_digest,
        &envelope.signed_envelope_digest,
        &verification.signature_message_digest,
        &verification.verifier_profile_digest,
        &verification.trusted_time_receipt_digest,
        &verification.verification_nonce_digest,
        &verification.signature_verification_receipt_digest,
    ]
    .into_iter()
    .any(|value| !is_sha256(value))
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_DIGEST_CHANGED");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_dispatch_authorization(
    authorization: &WindowsRecursivePolicyDispatchAuthorization,
    policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
    acquisition_receipt_ordinal: usize,
    producer_wave_ordinal: usize,
    input_custody_digest: &str,
    pre_dispatch_plan_evidence_digest: &str,
) -> Result<()> {
    validate_policy_source(policy)?;
    let envelope = &policy.signed_envelope;
    let verification = &policy.signature_verification;
    if authorization.schema != POLICY_DISPATCH_AUTHORIZATION_SCHEMA
        || authorization.authenticated_recursive_policy_digest != policy.digest()
        || authorization.policy_payload_digest != policy.policy_payload_digest
        || authorization.signed_envelope_digest != envelope.signed_envelope_digest
        || authorization.signature_verification_receipt_digest
            != verification.signature_verification_receipt_digest
        || authorization.policy_scope_digest != envelope.policy_scope_digest
        || authorization.policy_generation != envelope.policy_generation
        || authorization.policy_not_before_ms != envelope.not_before_ms
        || authorization.policy_not_after_ms != envelope.not_after_ms
        || authorization.control_key_id != envelope.control_key_id
        || authorization.control_key_record_digest != envelope.control_key_record_digest
        || authorization.control_public_key_spki_digest != envelope.control_public_key_spki_digest
        || authorization.signing_control_keyring_generation
            != envelope.signing_control_keyring_generation
        || authorization.observed_control_keyring_generation
            < authorization.signing_control_keyring_generation
        || authorization.active_control_key_record_digest != authorization.control_key_record_digest
        || authorization.active_control_public_key_spki_digest
            != authorization.control_public_key_spki_digest
        || authorization.policy_scope_current_generation != authorization.policy_generation
        || authorization.trusted_now_ms < verification.verified_at_ms
        || authorization.trusted_now_ms < authorization.policy_not_before_ms
        || authorization.trusted_now_ms >= authorization.policy_not_after_ms
        || authorization.trusted_time_attestation_sequence <= 0
        || authorization.acquisition_receipt_ordinal != acquisition_receipt_ordinal
        || authorization.producer_wave_ordinal != producer_wave_ordinal
        || authorization.input_custody_digest != input_custody_digest
        || authorization.pre_dispatch_plan_evidence_digest != pre_dispatch_plan_evidence_digest
        || !valid_control_key_id(&authorization.control_key_id)
        || digest::dispatch_authorization_digest(authorization)
            != authorization.authorization_digest
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_CURRENTNESS_CHANGED");
    }

    if [
        &authorization.authenticated_recursive_policy_digest,
        &authorization.policy_payload_digest,
        &authorization.signed_envelope_digest,
        &authorization.signature_verification_receipt_digest,
        &authorization.policy_scope_digest,
        &authorization.control_key_record_digest,
        &authorization.control_public_key_spki_digest,
        &authorization.control_keyring_snapshot_digest,
        &authorization.active_control_key_record_digest,
        &authorization.active_control_public_key_spki_digest,
        &authorization.control_key_non_revocation_receipt_digest,
        &authorization.control_keyring_anti_rollback_receipt_digest,
        &authorization.policy_generation_currentness_receipt_digest,
        &authorization.currentness_backend_profile_digest,
        &authorization.trusted_time_receipt_digest,
        &authorization.currentness_observation_receipt_digest,
        &authorization.input_custody_digest,
        &authorization.pre_dispatch_plan_evidence_digest,
        &authorization.authorization_nonce_digest,
        &authorization.authorization_digest,
    ]
    .into_iter()
    .any(|value| !is_sha256(value))
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_RECURSIVE_POLICY_CURRENTNESS_DIGEST_CHANGED");
    }
    Ok(())
}

fn valid_control_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
