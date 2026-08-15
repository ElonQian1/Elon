use serde::{Deserialize, Serialize};

use super::common::{
    ExternalPoolAdapterTaskProductionBoundary, ExternalPoolAdapterTaskProductionSessionRoots,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeSource {
    pub source_kind: String,
    pub source_id: String,
    pub source_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskAdapterBinding {
    pub provider_id: String,
    pub adapter_id: String,
    pub adapter_revision: u64,
    pub adapter_registry_digest: String,
    pub adapter_implementation_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskCommandBinding {
    pub command_id: String,
    pub command_digest: String,
    pub outbox_id: String,
    pub outbox_digest: String,
    pub send_attempt_id: String,
    pub send_attempt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskRouteBinding {
    pub route_authorization_id: String,
    pub route_authorization_digest: String,
    pub route_credential_id: String,
    pub route_credential_revision: u64,
    pub route_credential_digest: String,
    pub credential_verification_receipt_id: String,
    pub credential_verification_receipt_digest: String,
    pub credential_verifier_id: String,
    pub credential_verifier_revision: u64,
    pub credential_verifier_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskSessionBinding {
    pub roots: ExternalPoolAdapterTaskProductionSessionRoots,
    pub session_roots_digest: String,
    pub session_transcript_digest: String,
    pub upstream_transport_target_id: String,
    pub task_protocol_conformance_run_receipt_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeIdentity {
    pub operation_kind: String,
    pub source: ExternalPoolAdapterTaskExchangeSource,
    pub adapter: ExternalPoolAdapterTaskAdapterBinding,
    pub command: ExternalPoolAdapterTaskCommandBinding,
    pub route: ExternalPoolAdapterTaskRouteBinding,
    pub executor_binding_digest: String,
    pub fencing_generation: u64,
    pub fence_digest: String,
    pub session: ExternalPoolAdapterTaskSessionBinding,
    pub request_digest: String,
    pub delivery_attempt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeAttemptMaterial {
    pub identity: ExternalPoolAdapterTaskExchangeIdentity,
    pub started_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeAttemptEnvelope {
    pub schema: String,
    pub exchange_attempt_id: String,
    pub exchange_attempt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub attempt: ExternalPoolAdapterTaskExchangeAttemptMaterial,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeReceiptMaterial {
    pub exchange_attempt_id: String,
    pub exchange_attempt_digest: String,
    pub identity: ExternalPoolAdapterTaskExchangeIdentity,
    pub exchange_ordinal: u64,
    pub exchange_nonce_digest: String,
    pub upstream_request_bytes: u64,
    pub upstream_request_sha256: String,
    pub upstream_response_bytes: u64,
    pub upstream_response_sha256: String,
    pub semantic_observation_bytes: u64,
    pub semantic_observation_sha256: String,
    pub session_transcript_digest: String,
    pub exchange_root: String,
    pub authenticated_at: String,
    pub received_at: String,
    pub recorded_at: String,
    pub boundary: ExternalPoolAdapterTaskProductionBoundary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskExchangeReceiptEnvelope {
    pub schema: String,
    pub exchange_receipt_id: String,
    pub exchange_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub receipt: ExternalPoolAdapterTaskExchangeReceiptMaterial,
}
