//! Store-owned V272 task-protocol conformance orchestration and current authority.

mod audit;
mod current;
#[path = "../compute_federation/external_pool_adapter_entrypoint_capsule.rs"]
mod entrypoint_capsule;
mod error;
mod persistence;
mod read;
mod revocation;
mod roots;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod run;
mod runtime;
mod types;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod write;

pub(in crate::store) use current::current_external_pool_adapter_task_protocol_conformance_authority_on;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use run::{
    execute_external_pool_adapter_task_protocol_conformance, TaskProtocolConformanceExecutionInput,
    TaskProtocolConformanceFixtureResourceIdentity,
};
pub(in crate::store) use types::CurrentExternalPoolAdapterTaskProtocolConformanceAuthority;

pub(crate) mod api {
    pub(crate) use super::error::ExternalPoolAdapterTaskProtocolConformanceStoreError;
    pub(crate) use super::runtime::{
        external_pool_adapter_task_protocol_conformance_runtime,
        initialize_external_pool_adapter_task_protocol_conformance_runtime,
        ExternalPoolAdapterTaskProtocolConformanceRuntime,
        ExternalPoolAdapterTaskProtocolConformanceUnavailable,
    };
    pub(crate) use super::types::{
        CreateExternalPoolAdapterTaskProtocolConformanceRun,
        ExternalPoolAdapterTaskProtocolConformanceCurrentness,
        ExternalPoolAdapterTaskProtocolConformanceRevocationWriteReceipt,
        ExternalPoolAdapterTaskProtocolConformanceRunWriteReceipt,
        RevokeExternalPoolAdapterTaskProtocolConformanceRun,
    };
}
