use super::types::{
    ComputeUserNodeReadyHealthSourceRefV1, ComputeUserNodeReadyModelBindingV1,
    ComputeUserNodeReadyWorkAdmissionSourceRefV1,
    UntrustedComputeUserNodeHostResourceObservationV1,
    UntrustedComputeUserNodeHostRuntimeObservationV1,
};

#[derive(Debug)]
pub(crate) struct ComputeUserNodeReadySourceLineageSources {
    pub(crate) work_admission: ComputeUserNodeReadyWorkAdmissionSourceRefV1,
    pub(crate) ready_health: ComputeUserNodeReadyHealthSourceRefV1,
    pub(crate) host_runtime_observation: UntrustedComputeUserNodeHostRuntimeObservationV1,
}

#[derive(Debug)]
pub(crate) struct UntrustedComputeUserNodeHostRuntimeObservationDraftV1 {
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
