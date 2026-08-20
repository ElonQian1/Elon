//! Store-owned V272 task-protocol conformance orchestration and current authority.

mod active_carrier;
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use active_carrier::create_external_pool_adapter_task_protocol_conformance_run_for_projected_active;
pub(in crate::store) use active_carrier::{
    build_external_pool_adapter_task_protocol_active_refresh_input_on,
    current_external_pool_adapter_task_protocol_conformance_for_renewed_route_carrier_on,
    current_external_pool_adapter_task_protocol_conformance_leaf_for_renewed_route_carrier_on,
    current_external_pool_adapter_task_protocol_conformance_projected_active_authority_on,
    CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority,
    CurrentExternalPoolAdapterTaskProtocolProjectedActiveLeafAuthority,
};
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use active_carrier::{
    build_external_pool_adapter_task_protocol_registering_activation_input_on,
    prepare_external_pool_adapter_task_protocol_planned_active_carrier_on,
    PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier,
};
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
