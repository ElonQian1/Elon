use anyhow::Result;
use serde::{Deserialize, Serialize};

pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_SCHEMA: &str =
    "compute_federation.user_node_ready_source_lineage.v1";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_KIND: &str =
    "user_node_ready_source_lineage_v1";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_CANONICALIZATION: &str = "rfc8785_jcs";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_ALGORITHM: &str = "sha256";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-USER-NODE-READY-SOURCE-LINEAGE-V1";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_LINEAGE_MAX_JSON_BYTES: usize = 262_144;
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_STATUS: &str =
    "missing_node_currentness_runtime_transition_host_runtime_and_v15_session_authority";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_PROJECTION_EFFECT: &str =
    "untrusted_source_projection_only";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_NO_EFFECT: &str = "none";
pub(crate) const COMPUTE_USER_NODE_READY_SOURCE_AUTHORITY_MISSING: &str = "missing";
pub(crate) const COMPUTE_USER_NODE_READY_WORK_ADMISSION_SOURCE_SCHEMA: &str =
    "elon.compute_plugin.work_admission_source.v1";
pub(crate) const COMPUTE_USER_NODE_READY_WORK_ADMISSION_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.work_admission_receipt.v1";
pub(crate) const UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_SCHEMA: &str =
    "compute_federation.untrusted_user_node_host_runtime_observation.v1";
pub(crate) const UNTRUSTED_COMPUTE_USER_NODE_HOST_RUNTIME_OBSERVATION_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-UNTRUSTED-USER-NODE-HOST-RUNTIME-OBSERVATION-V1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyPluginReleaseRefV1 {
    pub(crate) plugin_id: String,
    pub(crate) plugin_version: String,
    pub(crate) target_id: String,
    pub(crate) manifest_digest: String,
    pub(crate) package_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyGrantedResourceCeilingV1 {
    pub(crate) max_cpu_millicores: i64,
    pub(crate) max_memory_bytes: i64,
    pub(crate) max_vram_bytes: i64,
    pub(crate) max_disk_bytes: i64,
    pub(crate) max_processes: i64,
    pub(crate) max_sidecar_uptime_seconds: i64,
    pub(crate) allow_network_egress: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyWorkAdmissionSourceRefV1 {
    pub(crate) source_schema: String,
    pub(crate) source_digest: String,
    pub(crate) receipt_schema: String,
    pub(crate) work_admission_id: String,
    pub(crate) receipt_digest: String,
    pub(crate) clock_epoch_digest: String,
    pub(crate) admitted_at_ms: i64,
    pub(crate) installation_identity_digest: String,
    pub(crate) plugin_id: String,
    pub(crate) slot_ref: String,
    pub(crate) release: ComputeUserNodeReadyPluginReleaseRefV1,
    pub(crate) install_receipt_id: String,
    pub(crate) install_receipt_digest: String,
    pub(crate) promotion_receipt_id: String,
    pub(crate) promotion_receipt_digest: String,
    pub(crate) plan_id: String,
    pub(crate) plan_digest: String,
    pub(crate) plan_policy_revision: i64,
    pub(crate) application_receipt_digest: String,
    pub(crate) grant_ref: String,
    pub(crate) grant_digest: String,
    pub(crate) install_generation: i64,
    pub(crate) activation_generation: i64,
    pub(crate) runtime_generation_before_ready: i64,
    pub(crate) work_admission_generation: i64,
    pub(crate) inventory_revision: i64,
    pub(crate) inventory_digest: String,
    pub(crate) authority_state_revision: i64,
    pub(crate) authority_epoch: i64,
    pub(crate) process_owner_epoch: i64,
    pub(crate) runner_digest: String,
    pub(crate) target_accelerator_kind: Option<String>,
    pub(crate) task_kinds: Vec<String>,
    pub(crate) granted_resources: ComputeUserNodeReadyGrantedResourceCeilingV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyTrustedTimeRefV1 {
    pub(crate) trusted_now: String,
    pub(crate) clock_epoch_digest: String,
    pub(crate) time_authority_id: String,
    pub(crate) attestation_digest: String,
    pub(crate) attestation_sequence: i64,
    pub(crate) signing_key_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyHealthSourceRefV1 {
    pub(crate) inventory_revision: i64,
    pub(crate) desired_policy_revision: i64,
    pub(crate) installation_identity_digest: String,
    pub(crate) plugin_id: String,
    pub(crate) last_plan_id: String,
    pub(crate) slot_ref: String,
    pub(crate) release: ComputeUserNodeReadyPluginReleaseRefV1,
    pub(crate) permission_grant_digest: String,
    pub(crate) install_generation: i64,
    pub(crate) activation_generation: i64,
    pub(crate) runtime_generation: i64,
    pub(crate) runner_digest: String,
    pub(crate) health_observation_digest: String,
    pub(crate) health_reason_codes: Vec<String>,
    pub(crate) health_observed_at: String,
    pub(crate) health_expires_at: String,
    pub(crate) trusted_time: ComputeUserNodeReadyTrustedTimeRefV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadyModelBindingV1 {
    pub(crate) model_id: String,
    pub(crate) model_digest: String,
    pub(crate) tokenizer_digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UntrustedComputeUserNodeHostResourceObservationV1 {
    pub(crate) accelerator_count: i64,
    pub(crate) cpu_millicores: i64,
    pub(crate) memory_bytes: i64,
    pub(crate) vram_bytes: i64,
    pub(crate) disk_bytes: i64,
    pub(crate) process_count: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UntrustedComputeUserNodeHostRuntimeObservationV1 {
    pub(crate) schema: String,
    pub(crate) observation_digest: String,
    pub(crate) executor_id: String,
    pub(crate) runner_id: String,
    pub(crate) runner_digest: String,
    pub(crate) runtime_digest: String,
    pub(crate) host_enforcement_ref: String,
    pub(crate) host_enforcement_digest: String,
    pub(crate) resource_profile_digest: String,
    pub(crate) task_kinds: Vec<String>,
    pub(crate) model_bindings: Vec<ComputeUserNodeReadyModelBindingV1>,
    pub(crate) supported_precisions: Vec<String>,
    pub(crate) resources: UntrustedComputeUserNodeHostResourceObservationV1,
    pub(crate) technical_concurrency_limit: i64,
    pub(crate) observed_at: String,
    pub(crate) expires_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadySourceAuthorityGapsV1 {
    pub(crate) node_local_authority_currentness: String,
    pub(crate) runtime_transition_authority: String,
    pub(crate) host_runtime_authority: String,
    pub(crate) v15_authenticated_session: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadySourceLineageEffectsV1 {
    pub(crate) projection_effect: String,
    pub(crate) readiness_effect: String,
    pub(crate) provider_effect: String,
    pub(crate) route_effect: String,
    pub(crate) offer_effect: String,
    pub(crate) capacity_effect: String,
    pub(crate) execution_effect: String,
    pub(crate) lease_effect: String,
    pub(crate) settlement_effect: String,
    pub(crate) money_effect: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ComputeUserNodeReadySourceLineageV1 {
    pub(crate) projection_status: String,
    pub(crate) work_admission: ComputeUserNodeReadyWorkAdmissionSourceRefV1,
    pub(crate) ready_health: ComputeUserNodeReadyHealthSourceRefV1,
    pub(crate) host_runtime_observation: UntrustedComputeUserNodeHostRuntimeObservationV1,
    pub(crate) authority_gaps: ComputeUserNodeReadySourceAuthorityGapsV1,
    pub(crate) effects: ComputeUserNodeReadySourceLineageEffectsV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UntrustedComputeUserNodeReadySourceLineageEnvelopeV1 {
    pub(crate) schema: String,
    pub(crate) lineage_kind: String,
    pub(crate) lineage_digest: String,
    pub(crate) canonicalization: String,
    pub(crate) digest_algorithm: String,
    pub(crate) lineage: ComputeUserNodeReadySourceLineageV1,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ProjectedComputeUserNodeReadySourceLineageV1 {
    pub(super) envelope: UntrustedComputeUserNodeReadySourceLineageEnvelopeV1,
}

impl ProjectedComputeUserNodeReadySourceLineageV1 {
    pub(crate) fn envelope(&self) -> &UntrustedComputeUserNodeReadySourceLineageEnvelopeV1 {
        &self.envelope
    }

    pub(crate) fn lineage_digest(&self) -> &str {
        &self.envelope.lineage_digest
    }

    pub(crate) fn lineage(&self) -> &ComputeUserNodeReadySourceLineageV1 {
        &self.envelope.lineage
    }

    pub(crate) fn canonical_json(&self) -> Result<String> {
        super::canonical::canonical_compute_user_node_ready_source_lineage_json_and_digest(
            &self.envelope,
        )
        .map(|(json, _)| json)
    }
}
