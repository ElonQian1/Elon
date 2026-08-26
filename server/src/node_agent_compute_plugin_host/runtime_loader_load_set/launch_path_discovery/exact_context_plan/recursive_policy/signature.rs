//! Typed signed-envelope and signature-verification evidence for recursive policy authority.

use std::convert::Infallible;

pub(super) const SIGNED_POLICY_ENVELOPE_SCHEMA: &str =
    "elon.compute_plugin.windows_recursive_policy_signed_envelope.v1";
pub(super) const SIGNATURE_VERIFICATION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.windows_recursive_policy_signature_verification_receipt.v1";
pub(super) const POLICY_CANONICALIZATION: &str = "rfc8785_jcs";
pub(super) const POLICY_DIGEST_ALGORITHM: &str = "sha256";
pub(super) const POLICY_SIGNATURE_ALGORITHM: &str = "ed25519";
pub(super) const POLICY_SIGNATURE_DOMAIN: &str = "ELON_WINDOWS_RECURSIVE_POLICY_SIGNATURE_V1";

/// Immutable control-plane envelope. It adds authority lifetime and generation around the stable
/// policy payload V1 without changing that payload's hash domain.
#[must_use = "signed recursive policy envelope must remain with verification custody"]
pub(super) struct SignedWindowsRecursiveResolutionPolicyEnvelope {
    pub(super) schema: &'static str,
    pub(super) canonicalization: &'static str,
    pub(super) digest_algorithm: &'static str,
    pub(super) signature_algorithm: &'static str,
    pub(super) signature_domain: &'static str,
    pub(super) policy_scope_digest: String,
    pub(super) policy_generation: u64,
    pub(super) not_before_ms: i64,
    pub(super) not_after_ms: i64,
    pub(super) policy_payload_digest: String,
    pub(super) signature_profile_digest: String,
    pub(super) control_key_id: String,
    pub(super) control_key_record_digest: String,
    pub(super) control_public_key_spki_digest: String,
    pub(super) signing_control_keyring_generation: u64,
    /// SHA-256 of the JCS canonical unsigned envelope material. A future verifier must verify
    /// Ed25519 over `signature_domain || 0x00 || decoded(signature_material_digest)`.
    pub(super) signature_material_digest: String,
    pub(super) signature_bytes: Vec<u8>,
    pub(super) signature_bytes_digest: String,
    pub(super) signed_envelope_digest: String,
}

/// Exact successful signature verification. The private uninhabited field prevents shape-correct
/// strings from becoming authentication evidence before a real cryptographic verifier exists.
#[must_use = "signature verification evidence must remain inside authenticated policy custody"]
pub(super) struct WindowsRecursivePolicySignatureVerificationReceipt {
    pub(super) schema: &'static str,
    pub(super) canonicalization: &'static str,
    pub(super) digest_algorithm: &'static str,
    pub(super) signature_algorithm: &'static str,
    pub(super) signature_domain: &'static str,
    pub(super) policy_payload_digest: String,
    pub(super) signed_envelope_digest: String,
    pub(super) signature_profile_digest: String,
    pub(super) signature_material_digest: String,
    pub(super) signature_bytes_digest: String,
    pub(super) signature_message_digest: String,
    pub(super) control_key_id: String,
    pub(super) control_key_record_digest: String,
    pub(super) control_public_key_spki_digest: String,
    pub(super) signing_control_keyring_generation: u64,
    pub(super) verified_policy_payload_digest: String,
    pub(super) verifier_profile_digest: String,
    pub(super) verified_at_ms: i64,
    pub(super) trusted_time_receipt_digest: String,
    pub(super) verification_nonce_digest: String,
    pub(super) signature_verification_receipt_digest: String,
    pub(super) _signature_verifier_backend_unavailable: Infallible,
}
