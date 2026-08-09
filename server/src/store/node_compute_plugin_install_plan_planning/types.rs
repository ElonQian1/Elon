use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct NodeComputePluginInstallPlanPlanningDispatchIntentV2 {
    pub(crate) planning_delivery_id: String,
    pub(crate) cloud_session_id: String,
    pub(crate) source_sharing_delivery_id: String,
    pub(crate) source_preparation_id: String,
    pub(crate) source_preparation_delivery_id: String,
    pub(crate) source_preparation_observation_id: String,
    pub(crate) source_preparation_observation_digest: String,
    pub(crate) source_preparation_request_digest: String,
    pub(crate) source_bootstrap_instance_id: String,
    pub(crate) source_configuration_generation: u64,
    pub(crate) source_cancellation_generation: u64,
    pub(crate) request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
    pub(crate) request_json: String,
    pub(crate) request_digest: String,
    pub(crate) consent_receipt_id: String,
    pub(crate) replayed: bool,
    pub(crate) dispatchable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PlanningDeliveryRequestEnvelopeV2 {
    pub(super) schema: String,
    pub(super) planning_delivery_id: String,
    pub(super) cloud_session_id: String,
    pub(super) source_sharing_delivery_id: String,
    pub(super) source_preparation_observation_id: String,
    pub(super) source_preparation_request_digest: String,
    pub(super) source_bootstrap_instance_id: String,
    pub(super) source_configuration_generation: u64,
    pub(super) source_cancellation_generation: u64,
    pub(super) consent_receipt_id: String,
    pub(super) request: homecli_proto::ComputePluginInstallPlanPlanningSnapshotRequestV2,
}

#[derive(Debug, Clone)]
pub(crate) struct DurableComputePluginInstallPlanPlanningSnapshotV2 {
    pub(super) snapshot_id: String,
    pub(super) snapshot: homecli_proto::HashedComputePluginInstallPlanPlanningSnapshotV2,
    pub(super) snapshot_json: String,
    pub(super) planning_delivery_id: String,
    pub(super) consent_receipt_id: String,
    pub(super) source_preparation_observation_id: String,
    pub(super) source_preparation_request_digest: String,
}

#[derive(Debug, Clone)]
pub(crate) enum PlanningSnapshotObservationCommitV2 {
    ObservedWithoutSnapshot,
    Snapshot(DurableComputePluginInstallPlanPlanningSnapshotV2),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginInstallPlanGenerationRequestV1 {
    pub(crate) schema: String,
    pub(crate) generation_request_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) snapshot_digest: String,
    pub(crate) node_id: String,
    pub(crate) owner_user_id: String,
    pub(crate) installation_identity_digest: String,
    pub(crate) policy_revision: u64,
    pub(crate) policy_digest: String,
    pub(crate) authorization_ref: String,
    pub(crate) authorization_revision: u64,
    pub(crate) authorization_digest: String,
    pub(crate) requested_control_keyring_revision: u64,
    pub(crate) requested_control_keyring_digest: String,
    pub(crate) signer_profile: String,
    pub(crate) requested_at_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct DurableComputePluginInstallPlanGenerationRequestV1 {
    pub(crate) request: ComputePluginInstallPlanGenerationRequestV1,
    pub(crate) request_json: String,
    pub(crate) request_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputePluginInstallPlanGenerationOutcomeV1 {
    pub(crate) schema: String,
    pub(crate) outcome_id: String,
    pub(crate) generation_request_id: String,
    pub(crate) generation_request_digest: String,
    pub(crate) outcome_kind: String,
    pub(crate) detail_code: String,
    pub(crate) retryable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DurableComputePluginInstallPlanGenerationOutcomeV1 {
    pub(crate) outcome: ComputePluginInstallPlanGenerationOutcomeV1,
    pub(crate) outcome_json: String,
    pub(crate) outcome_digest: String,
}

#[derive(Debug, Clone)]
pub(super) struct PlanningSourceV2 {
    pub(super) source_sharing_delivery_id: String,
    pub(super) source_preparation_observation_id: String,
    pub(super) source_preparation_request_digest: String,
    pub(super) consent_receipt_id: String,
    pub(super) source_bootstrap_instance_id: String,
    pub(super) source_configuration_generation: u64,
    pub(super) source_cancellation_generation: u64,
}
