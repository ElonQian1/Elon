use serde::{Deserialize, Serialize};

use super::super::route_authority::ComputeRouteCapabilityBinding;

pub(crate) const ATOMIC_ACTIVATION_RECEIPT_SCHEMA: &str =
    "compute_federation.external_pool_adapter_atomic_activation_receipt.v1";
pub(crate) const PROJECTED_ACTIVE_TRANSITION_PROOF_SCHEMA: &str =
    "external_pool_adapter_credential_projected_active_transition_proof_v1";
pub(crate) const TASK_PROTOCOL_ACTIVE_CARRIER_SCHEMA: &str =
    "compute_federation.external_pool_adapter_task_protocol_active_carrier.v1";
pub(crate) const ATOMIC_ACTIVATION_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const ATOMIC_ACTIVATION_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const ATOMIC_ACTIVATION_ACTOR_KIND: &str = "provider_owner";
pub(crate) const ATOMIC_ACTIVATION_IDEMPOTENCY_SCOPE: &str =
    "external_pool_adapter_atomic_activation";
pub(crate) const ATOMIC_ACTIVATION_CONFIRMATION: &str =
    "I_CONFIRM_EXTERNAL_POOL_ADAPTER_ATOMIC_ACTIVATION";
pub(crate) const ATOMIC_ACTIVATION_MAX_JSON_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationIdentity {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationProviderEvidence {
    pub provider_id: String,
    pub provider_policy_revision: i64,
    pub provider_json: String,
    pub provider_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationProviderTransition {
    pub source_registering_provider: ExternalPoolAdapterAtomicActivationProviderEvidence,
    pub target_active_provider: ExternalPoolAdapterAtomicActivationProviderEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterCredentialProjectedActiveTransitionProofMaterial {
    pub schema: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub source_registering_provider_id: String,
    pub source_registering_provider_policy_revision: i64,
    pub source_registering_provider_json: String,
    pub source_registering_provider_digest: String,
    pub target_active_provider_id: String,
    pub target_active_provider_policy_revision: i64,
    pub target_active_provider_json: String,
    pub target_active_provider_digest: String,
    pub registering_reattestation_receipt_id: String,
    pub registering_reattestation_receipt_digest: String,
    pub logical_adapter_id: String,
    pub route_adapter_projection_id: String,
    pub evidence_checked_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationV253GenesisInput {
    pub registering_reattestation_receipt_id: String,
    pub registering_reattestation_receipt_digest: String,
    pub projected_transition_proof_material_json: String,
    pub projected_transition_proof_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolStableExecutorIdMaterial {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub route_adapter_projection_id: String,
    pub service_actor_id: String,
    pub task_production_carrier_policy_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolStableExecutorBindingMaterial {
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub route_adapter_projection_id: String,
    pub service_actor_id: String,
    pub task_production_carrier_policy_digest: String,
    pub executor_id: String,
    pub logical_projection_compatibility_digest: String,
    pub projected_v211_adapter_binding_digest: String,
    pub lane_subject_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolStableExecutorBinding {
    pub executor_id: String,
    pub executor_id_hash: String,
    pub executor_id_material_json: String,
    pub executor_binding_material_json: String,
    pub stable_executor_binding_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolProjectedV211AdapterBinding {
    pub projected_v211_adapter_binding_json: String,
    pub projected_v211_adapter_binding_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationRouteClosure {
    pub route_adapter_projection_id: String,
    pub route_adapter_revision: i64,
    pub route_adapter_digest: String,
    pub service_actor_id: String,
    pub service_actor_authorization_id: String,
    pub service_actor_authorization_digest: String,
    pub route_credential_id: String,
    pub route_credential_revision: i64,
    pub route_credential_digest: String,
    pub route_authorization_id: String,
    pub route_authorization_revision: i64,
    pub route_authorization_digest: String,
    pub capabilities: Vec<ComputeRouteCapabilityBinding>,
    pub route_capability_count: i64,
    pub route_capability_set_digest: String,
    pub route_seal_id: String,
    pub route_seal_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProtocolActiveCarrierMaterial {
    pub schema: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub target_active_provider_id: String,
    pub target_active_provider_policy_revision: i64,
    pub target_active_provider_digest: String,
    pub route_adapter_projection_id: String,
    pub task_protocol_conformance_run_receipt_id: String,
    pub task_protocol_conformance_run_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationRenewableEvidence {
    pub active_runtime_observation_id: String,
    pub active_runtime_observation_digest: String,
    pub observation_started_at: String,
    pub observation_completed_at: String,
    pub observation_expires_at: String,
    pub task_protocol_conformance_run_receipt_id: String,
    pub task_protocol_conformance_run_receipt_digest: String,
    pub task_protocol_conformance_expires_at: String,
    pub task_protocol_active_carrier_material_json: String,
    pub task_protocol_active_carrier_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationIdempotencyMaterial {
    pub actor_kind: String,
    pub actor_user_id: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
    pub scope: String,
    pub key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationConfirmationMaterial {
    pub confirmation: String,
    pub actor_kind: String,
    pub actor_user_id: String,
    pub idempotency_digest: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub activation_root_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationAudit {
    pub activated_by_actor_kind: String,
    pub activated_by_actor_user_id: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub idempotency_material_json: String,
    pub idempotency_digest: String,
    pub confirmation: String,
    pub confirmation_material_json: String,
    pub confirmation_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationMaterial {
    pub identity: ExternalPoolAdapterAtomicActivationIdentity,
    pub provider_transition: ExternalPoolAdapterAtomicActivationProviderTransition,
    pub v253_genesis_input: ExternalPoolAdapterAtomicActivationV253GenesisInput,
    pub stable_executor: ExternalPoolStableExecutorBinding,
    pub projected_v211_binding: ExternalPoolProjectedV211AdapterBinding,
    pub route_closure: ExternalPoolAdapterAtomicActivationRouteClosure,
    pub renewable_evidence: ExternalPoolAdapterAtomicActivationRenewableEvidence,
    pub audit: ExternalPoolAdapterAtomicActivationAudit,
    pub activation_target_updated_at: String,
    pub evidence_checked_at: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterAtomicActivationReceipt {
    pub schema: String,
    pub activation_receipt_id: String,
    pub activation_receipt_digest: String,
    pub canonicalization: String,
    pub digest_algorithm: String,
    pub activation: ExternalPoolAdapterAtomicActivationMaterial,
}
