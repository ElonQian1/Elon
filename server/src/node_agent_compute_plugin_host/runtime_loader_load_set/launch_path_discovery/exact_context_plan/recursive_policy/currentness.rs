//! One-use signature-verification and policy-currentness authority for an exact recursive dispatch.

use std::convert::Infallible;

use anyhow::Result;
use serde_json::{json, Value};

use super::{validation, AuthenticatedWindowsRecursiveResolutionPolicy};

pub(super) const POLICY_DISPATCH_AUTHORIZATION_SCHEMA: &str =
    "elon.compute_plugin.windows_recursive_policy_dispatch_authorization.v1";

/// Point-of-use proof that the exact signed policy, signer key and policy generation remain
/// current for one A0/Ak dispatch coordinate.
///
/// This is a success-only, linear authority. There is deliberately no status boolean, public
/// constructor, clone, serializer or detached retry permit. A future backend must establish the
/// active key record, non-revocation, non-supersession and trusted time in one typed transition.
#[must_use = "recursive policy dispatch authorization must be consumed by its exact A0/Ak owner"]
pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) struct WindowsRecursivePolicyDispatchAuthorization
{
    pub(super) schema: &'static str,
    pub(super) authenticated_recursive_policy_digest: String,
    pub(super) policy_payload_digest: String,
    pub(super) signed_envelope_digest: String,
    pub(super) signature_verification_receipt_digest: String,
    pub(super) policy_scope_digest: String,
    pub(super) policy_generation: u64,
    pub(super) policy_not_before_ms: i64,
    pub(super) policy_not_after_ms: i64,
    pub(super) control_key_id: String,
    pub(super) control_key_record_digest: String,
    pub(super) control_public_key_spki_digest: String,
    pub(super) signing_control_keyring_generation: u64,
    pub(super) observed_control_keyring_generation: u64,
    pub(super) control_keyring_snapshot_digest: String,
    pub(super) active_control_key_record_digest: String,
    pub(super) active_control_public_key_spki_digest: String,
    pub(super) control_key_non_revocation_receipt_digest: String,
    pub(super) control_keyring_anti_rollback_receipt_digest: String,
    pub(super) policy_scope_current_generation: u64,
    pub(super) policy_generation_currentness_receipt_digest: String,
    pub(super) currentness_backend_profile_digest: String,
    pub(super) trusted_now_ms: i64,
    pub(super) trusted_time_attestation_sequence: i64,
    pub(super) trusted_time_receipt_digest: String,
    pub(super) currentness_observation_receipt_digest: String,
    pub(super) acquisition_receipt_ordinal: usize,
    pub(super) producer_wave_ordinal: usize,
    pub(super) input_custody_digest: String,
    pub(super) pre_dispatch_plan_evidence_digest: String,
    pub(super) authorization_nonce_digest: String,
    pub(super) authorization_digest: String,
    pub(super) _policy_currentness_backend_unavailable: Infallible,
}

impl WindowsRecursivePolicyDispatchAuthorization {
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn digest(
        &self,
    ) -> &str {
        &self.authorization_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn nonce_digest(
        &self,
    ) -> &str {
        &self.authorization_nonce_digest
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn observed_control_keyring_generation(
        &self,
    ) -> u64 {
        self.observed_control_keyring_generation
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn trusted_now_ms(
        &self,
    ) -> i64 {
        self.trusted_now_ms
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn trusted_time_attestation_sequence(
        &self,
    ) -> i64 {
        self.trusted_time_attestation_sequence
    }

    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn validate_against(
        &self,
        policy: &AuthenticatedWindowsRecursiveResolutionPolicy,
        acquisition_receipt_ordinal: usize,
        producer_wave_ordinal: usize,
        input_custody_digest: &str,
        pre_dispatch_plan_evidence_digest: &str,
    ) -> Result<()> {
        validation::validate_dispatch_authorization(
            self,
            policy,
            acquisition_receipt_ordinal,
            producer_wave_ordinal,
            input_custody_digest,
            pre_dispatch_plan_evidence_digest,
        )
    }

    /// Full immutable material for inclusion in an acquisition receipt. The authorization digest
    /// is included alongside every field it commits; callers must not replace this value with a
    /// detached digest-only projection.
    pub(in crate::node_agent_compute_plugin_host::runtime_loader_load_set) fn canonical_material(
        &self,
    ) -> Value {
        json!({
            "schema": self.schema,
            "authenticated_recursive_policy_digest": &self.authenticated_recursive_policy_digest,
            "policy_payload_digest": &self.policy_payload_digest,
            "signed_envelope_digest": &self.signed_envelope_digest,
            "signature_verification_receipt_digest": &self.signature_verification_receipt_digest,
            "policy_scope_digest": &self.policy_scope_digest,
            "policy_generation": self.policy_generation,
            "policy_not_before_ms": self.policy_not_before_ms,
            "policy_not_after_ms": self.policy_not_after_ms,
            "control_key_id": &self.control_key_id,
            "control_key_record_digest": &self.control_key_record_digest,
            "control_public_key_spki_digest": &self.control_public_key_spki_digest,
            "signing_control_keyring_generation": self.signing_control_keyring_generation,
            "observed_control_keyring_generation": self.observed_control_keyring_generation,
            "control_keyring_snapshot_digest": &self.control_keyring_snapshot_digest,
            "active_control_key_record_digest": &self.active_control_key_record_digest,
            "active_control_public_key_spki_digest": &self.active_control_public_key_spki_digest,
            "control_key_non_revocation_receipt_digest": &self.control_key_non_revocation_receipt_digest,
            "control_keyring_anti_rollback_receipt_digest": &self.control_keyring_anti_rollback_receipt_digest,
            "policy_scope_current_generation": self.policy_scope_current_generation,
            "policy_generation_currentness_receipt_digest": &self.policy_generation_currentness_receipt_digest,
            "currentness_backend_profile_digest": &self.currentness_backend_profile_digest,
            "trusted_now_ms": self.trusted_now_ms,
            "trusted_time_attestation_sequence": self.trusted_time_attestation_sequence,
            "trusted_time_receipt_digest": &self.trusted_time_receipt_digest,
            "currentness_observation_receipt_digest": &self.currentness_observation_receipt_digest,
            "acquisition_receipt_ordinal": self.acquisition_receipt_ordinal,
            "producer_wave_ordinal": self.producer_wave_ordinal,
            "input_custody_digest": &self.input_custody_digest,
            "pre_dispatch_plan_evidence_digest": &self.pre_dispatch_plan_evidence_digest,
            "authorization_nonce_digest": &self.authorization_nonce_digest,
            "authorization_digest": &self.authorization_digest,
        })
    }
}
