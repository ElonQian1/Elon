//! Append-only storage for inert Provider-specific runtime launch profiles.

mod audit;
mod build;
mod current;
mod input;
mod persistence;
mod policy;
mod read;
mod roots;
mod types;
mod write;

pub(crate) mod api {
    pub(crate) use super::types::{
        CreateExternalPoolAdapterRuntimeLaunchProfile,
        ExternalPoolAdapterRuntimeLaunchPolicySummary,
        ExternalPoolAdapterRuntimeLaunchProfileAuditTarget,
        ExternalPoolAdapterRuntimeLaunchProfileCurrentness,
        ExternalPoolAdapterRuntimeLaunchProfileRevocationSummary,
        ExternalPoolAdapterRuntimeLaunchProfileRevocationWriteReceipt,
        ExternalPoolAdapterRuntimeLaunchProfileSummary,
        ExternalPoolAdapterRuntimeLaunchProfileWriteReceipt,
        RevokeExternalPoolAdapterRuntimeLaunchProfile,
    };
}

pub(in crate::store) use current::current_external_pool_adapter_runtime_launch_profile_authority_on;
pub(in crate::store) use read::historical_external_pool_adapter_runtime_launch_profile_authority_on;
pub(in crate::store) use types::CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority;
