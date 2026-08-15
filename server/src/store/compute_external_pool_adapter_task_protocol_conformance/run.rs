//! Pure V272 execution boundary. Store transactions and current-authority checks live elsewhere.

mod execution;
mod oracle;
mod support;

use anyhow::Result;

use crate::compute_federation::{
    external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    external_pool_adapter_task_protocol_conformance::{
        ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
        TaskProtocolConformanceRunEvidence,
    },
};

use super::runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime;

#[derive(Clone)]
pub(in crate::store) struct TaskProtocolConformanceFixtureResourceIdentity {
    pub(in crate::store) purpose: String,
    pub(in crate::store) path: String,
    pub(in crate::store) sha256: String,
    pub(in crate::store) size_bytes: u64,
}

/// Fully preflighted Provider-neutral roots plus one fresh Prepared execution carrier.
/// The carrier itself is consumed by the runner and never enters returned evidence.
pub(in crate::store) struct TaskProtocolConformanceExecutionInput {
    pub(in crate::store) prepared_installation: PreparedExternalPoolAdapterInstallation,
    pub(in crate::store) registry_release:
        ExternalPoolAdapterTaskProtocolConformanceRegistryReleaseRoots,
    pub(in crate::store) supervisor_session_policy_digest: String,
    pub(in crate::store) sandbox_reattestation_receipt_digest: String,
    pub(in crate::store) runtime_compatibility_verification_receipt_digest: String,
    pub(in crate::store) source_capsule_sha256: String,
    pub(in crate::store) source_capsule_size_bytes: u64,
    pub(in crate::store) launch_image_sha256: String,
    pub(in crate::store) launch_image_size_bytes: u64,
    pub(in crate::store) fixture_resources: Vec<TaskProtocolConformanceFixtureResourceIdentity>,
}

pub(in crate::store) fn execute_external_pool_adapter_task_protocol_conformance(
    input: TaskProtocolConformanceExecutionInput,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
) -> Result<TaskProtocolConformanceRunEvidence> {
    execution::execute(input, runtime)
}
