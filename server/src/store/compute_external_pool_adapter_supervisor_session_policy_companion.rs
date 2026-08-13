#[path = "compute_external_pool_adapter_supervisor_session_policy_companion/audit_companion.rs"]
mod audit_companion;
#[path = "compute_external_pool_adapter_supervisor_session_policy_companion/audit_revocation.rs"]
mod audit_revocation;
mod build;
mod current;
mod input;
mod persistence;
mod policy;
mod read;
mod roots;
mod types;
mod write;

pub(in crate::store) use current::current_external_pool_adapter_supervisor_session_policy_companion_authority_on;
pub(crate) use types::*;
