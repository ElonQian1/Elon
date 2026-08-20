//! Purpose-specific V277 planned and projected-active V272 carrier paths.

mod current;
mod roots;
mod types;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod write;

pub(in crate::store) use current::current_external_pool_adapter_task_protocol_conformance_projected_active_authority_on;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use types::prepare_external_pool_adapter_task_protocol_planned_active_carrier_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterTaskProtocolProjectedActiveAuthority;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use types::PreparedExternalPoolAdapterTaskProtocolPlannedActiveCarrier;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(in crate::store) use write::create_external_pool_adapter_task_protocol_conformance_run_for_projected_active;
