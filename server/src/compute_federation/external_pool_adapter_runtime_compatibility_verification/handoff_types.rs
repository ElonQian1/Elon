use serde::Serialize;

pub(crate) const RUNTIME_COMPATIBILITY_SIGNING_HANDOFF_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_signing_handoff.v1";
pub(crate) const RUNTIME_COMPATIBILITY_SIGNER_PAYLOAD_SCHEMA: &str =
    "compute_federation.external_pool_adapter_runtime_compatibility_signer_payload.v1";

#[derive(Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding {
    pub run_observation_id: String,
    pub run_observation_digest: String,
}

#[derive(Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySignerPayload {
    pub schema: &'static str,
    pub signature_algorithm: &'static str,
    pub sandbox_verifier_key_record_id: String,
    pub sandbox_verifier_key_record_digest: String,
    pub sandbox_verifier_key_id: String,
    pub signature_message_base64: String,
    pub signature_message_digest: String,
    pub expires_at: String,
}

#[derive(Serialize)]
pub(crate) struct ExternalPoolAdapterRuntimeCompatibilitySigningHandoff {
    pub schema: &'static str,
    pub record_binding: ExternalPoolAdapterRuntimeCompatibilitySigningHandoffRecordBinding,
    pub signer_payload: ExternalPoolAdapterRuntimeCompatibilitySignerPayload,
    pub replayed: bool,
}
