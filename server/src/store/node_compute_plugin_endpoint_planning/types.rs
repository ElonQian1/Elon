use super::super::{
    node_compute_plugin_install_plan_planning::NodeComputePluginInstallPlanPlanningDispatchIntentV2,
    node_compute_plugin_install_plan_preparation::NodeComputePluginInstallPlanPreparationDispatchIntent,
    node_compute_plugin_sharing::NodeComputePluginSharingDispatchIntent,
};

#[derive(Clone)]
pub(crate) struct NodeComputePluginEndpointPlanningBootstrapSharingIntentV1 {
    pub(super) message: homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1,
    pub(super) event: EndpointPlanningEventWrite,
    pub(super) sharing: NodeComputePluginSharingDispatchIntent,
}

impl NodeComputePluginEndpointPlanningBootstrapSharingIntentV1 {
    pub(crate) fn message(&self) -> &homecli_proto::NodeEndpointPlanningBootstrapSharingRequestV1 {
        &self.message
    }
}

#[derive(Clone)]
pub(crate) struct NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1 {
    pub(super) message: homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1,
    pub(super) event: EndpointPlanningEventWrite,
    pub(super) preparation: NodeComputePluginInstallPlanPreparationDispatchIntent,
}

impl NodeComputePluginEndpointPlanningBootstrapPreparationIntentV1 {
    pub(crate) fn message(
        &self,
    ) -> &homecli_proto::NodeEndpointPlanningBootstrapPreparationRequestV1 {
        &self.message
    }
}

#[derive(Clone)]
pub(crate) struct NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1 {
    pub(super) message: homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1,
    pub(super) event: EndpointPlanningEventWrite,
    pub(super) planning: NodeComputePluginInstallPlanPlanningDispatchIntentV2,
}

impl NodeComputePluginEndpointPlanningBootstrapSnapshotIntentV1 {
    pub(crate) fn message(&self) -> &homecli_proto::NodeEndpointPlanningBootstrapSnapshotRequestV1 {
        &self.message
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NodeComputePluginEndpointPlanningBootstrapTerminalV1 {
    pub(super) _sealed: (),
}

#[derive(Clone)]
pub(super) struct EndpointPlanningEventWrite {
    pub(super) event_id: String,
    pub(super) bootstrap_id: String,
    pub(super) message_sequence: i64,
    pub(super) message_kind: &'static str,
    pub(super) previous_message_sequence: Option<i64>,
    pub(super) previous_event_id: Option<String>,
    pub(super) next_message_sequence: Option<i64>,
    pub(super) next_event_id: Option<String>,
    pub(super) message_schema: &'static str,
    pub(super) message_json: String,
    pub(super) message_digest: String,
    pub(super) previous_message_digest: String,
    pub(super) delivery_id: String,
    pub(super) agent_id: String,
    pub(super) owner_user_id: String,
    pub(super) install_id: String,
    pub(super) installation_binding_digest: String,
    pub(super) plugin_installation_identity_digest: String,
    pub(super) credential_id: String,
    pub(super) credential_revision: i64,
    pub(super) credential_digest: String,
    pub(super) authentication_receipt_id: String,
    pub(super) authentication_digest: String,
    pub(super) session_id: String,
    pub(super) session_generation: i64,
    pub(super) server_instance_id: String,
    pub(super) agent_version: String,
    pub(super) authenticated_at: String,
    pub(super) expires_at: String,
    pub(super) protocol_version: i64,
    pub(super) capability_count: i64,
    pub(super) capability_set_digest: String,
    pub(super) consent_receipt_id: String,
    pub(super) policy_revision: i64,
    pub(super) policy_digest: String,
    pub(super) policy_snapshot_digest: String,
    pub(super) plugin_runtime_requested: bool,
    pub(super) sharing_delivery_id: String,
    pub(super) sharing_observation_id: Option<String>,
    pub(super) sharing_observation_digest: Option<String>,
    pub(super) preparation_id: Option<String>,
    pub(super) preparation_delivery_id: Option<String>,
    pub(super) preparation_request_digest: Option<String>,
    pub(super) preparation_observation_id: Option<String>,
    pub(super) preparation_observation_digest: Option<String>,
    pub(super) planning_delivery_id: Option<String>,
    pub(super) planning_request_digest: Option<String>,
    pub(super) planning_observation_event_id: Option<String>,
    pub(super) planning_observation_digest: Option<String>,
    pub(super) accepted: Option<bool>,
    pub(super) replayed: Option<bool>,
    pub(super) snapshot_ready: Option<bool>,
    pub(super) recorded_at: String,
}
