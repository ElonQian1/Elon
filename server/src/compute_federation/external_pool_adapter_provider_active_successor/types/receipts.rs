use serde::{Deserialize, Serialize};

use super::{
    ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    ExternalPoolAdapterProviderActiveSuccessorEffects,
    ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    ExternalPoolAdapterProviderActiveSuccessorReadiness,
};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence {
    pub reattestation_receipt_id: String,
    pub reattestation_receipt_digest: String,
    pub observed_provider: ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation {
    pub runtime_observation_id: String,
    pub runtime_observation_digest: String,
    pub observed_provider: ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    pub observation_started_at: String,
    pub observation_completed_at: String,
    pub observation_expires_at: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence {
    pub task_protocol_conformance_run_receipt_id: String,
    pub task_protocol_conformance_run_receipt_digest: String,
    pub task_protocol_conformance_expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// V277 binds this witness to the stable activation root and activation closure only. It must
/// never bind the enclosing V274 receipt identity/digest, which would create a digest cycle.
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorActivationWitness {
    pub activation_witness_id: String,
    pub activation_witness_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorLineage {
    pub successor_sequence: u64,
    pub predecessor_active_successor_receipt_id: Option<String>,
    pub predecessor_active_successor_receipt_digest: Option<String>,
}

/// Canonical successor material. It has no `Debug` implementation because custody is private.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorMaterial {
    pub activation: ExternalPoolAdapterProviderActiveSuccessorActivationRoot,
    pub lineage: ExternalPoolAdapterProviderActiveSuccessorLineage,
    pub evidence_provider: ExternalPoolAdapterProviderActiveSuccessorProviderEvidence,
    pub credential_evidence: ExternalPoolAdapterProviderActiveSuccessorCredentialEvidence,
    pub runtime_observation: ExternalPoolAdapterProviderActiveSuccessorRuntimeObservation,
    pub task_protocol_evidence: ExternalPoolAdapterProviderActiveSuccessorTaskProtocolEvidence,
    pub activation_witness: ExternalPoolAdapterProviderActiveSuccessorActivationWitness,
    pub activation_target_updated_at: String,
    pub evidence_checked_at: String,
    pub created_at: String,
    pub effects: ExternalPoolAdapterProviderActiveSuccessorEffects,
    pub readiness: ExternalPoolAdapterProviderActiveSuccessorReadiness,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorReceipt {
    pub schema: String,
    pub active_successor_receipt_id: String,
    pub receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub successor: ExternalPoolAdapterProviderActiveSuccessorMaterial,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorRevocationMaterial {
    pub target_active_successor_receipt_id: String,
    pub target_active_successor_receipt_digest: String,
    pub provider_binding_id: String,
    pub activation_root_digest: String,
    pub revoked_by_actor_kind: String,
    pub revoked_by_actor_user_id: String,
    pub reason_code: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub idempotency_digest: String,
    pub confirmation: String,
    pub confirmation_digest: String,
    pub revoked_at: String,
    pub effects: ExternalPoolAdapterProviderActiveSuccessorEffects,
    pub readiness: ExternalPoolAdapterProviderActiveSuccessorReadiness,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt {
    pub schema: String,
    pub active_successor_revocation_id: String,
    pub revocation_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub revocation: ExternalPoolAdapterProviderActiveSuccessorRevocationMaterial,
}
