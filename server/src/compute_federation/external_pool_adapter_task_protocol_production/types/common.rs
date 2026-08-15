use serde::{Deserialize, Serialize};

use super::super::TASK_PRODUCTION_NO_EFFECT;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionEffects {
    pub credential_effect: String,
    pub adapter_effect: String,
    pub provider_effect: String,
    pub route_effect: String,
    pub activation_effect: String,
    pub execution_effect: String,
    pub usage_effect: String,
    pub market_effect: String,
    pub settlement_effect: String,
}

impl ExternalPoolAdapterTaskProductionEffects {
    pub(crate) fn none() -> Self {
        Self {
            credential_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            adapter_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            provider_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            route_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            activation_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            execution_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            usage_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            market_effect: TASK_PRODUCTION_NO_EFFECT.into(),
            settlement_effect: TASK_PRODUCTION_NO_EFFECT.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionReadiness {
    pub process_spawn_ready: bool,
    pub ipc_session_ready: bool,
    pub secret_delivery_ready: bool,
    pub broker_connect_ready: bool,
    pub upstream_probe_ready: bool,
    pub runtime_launch_ready: bool,
    pub route_ready: bool,
    pub execution_ready: bool,
    pub activation_ready: bool,
}

impl ExternalPoolAdapterTaskProductionReadiness {
    pub(crate) fn none() -> Self {
        Self {
            process_spawn_ready: false,
            ipc_session_ready: false,
            secret_delivery_ready: false,
            broker_connect_ready: false,
            upstream_probe_ready: false,
            runtime_launch_ready: false,
            route_ready: false,
            execution_ready: false,
            activation_ready: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionBoundary {
    pub authority_status: String,
    pub effects: ExternalPoolAdapterTaskProductionEffects,
    pub readiness: ExternalPoolAdapterTaskProductionReadiness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionLaneSubjectInput {
    pub provider_id: String,
    pub provider_owner_account_id: String,
    pub provider_binding_id: String,
    pub provider_binding_digest: String,
    pub registry_release_id: String,
    pub registry_release_digest: String,
    pub route_adapter_projection_id: String,
    pub logical_adapter_binding_digest: String,
    pub logical_projection_compatibility_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionLaneSubject {
    pub subject: ExternalPoolAdapterTaskProductionLaneSubjectInput,
    pub lane_subject_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskProductionSessionRoots {
    pub supervisor_session_policy_digest: String,
    pub runtime_launch_profile_digest: String,
    pub task_protocol_profile_digest: String,
    pub upstream_transport_target_digest: String,
    pub supervisor_session_policy_companion_digest: String,
    pub launch_image_sha256: String,
    pub ephemeral_task_secret_delivery_root: String,
    pub task_protocol_conformance_run_receipt_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskPollLineage {
    pub predecessor_id: Option<String>,
    pub predecessor_digest: Option<String>,
    pub poll_ordinal: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskRemoteIdentity {
    pub executor_binding_digest: String,
    pub remote_execution_id: Option<String>,
    pub remote_identity_digest: String,
    pub remote_execution_state: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExternalPoolAdapterTaskAuthenticatedRemoteSubject {
    pub remote: ExternalPoolAdapterTaskRemoteIdentity,
}
