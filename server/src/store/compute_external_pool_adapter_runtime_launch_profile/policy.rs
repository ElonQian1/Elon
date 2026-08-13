use anyhow::Result;

use crate::{
    compute_federation::external_pool_adapter_runtime_launch_profile::{
        server_runtime_launch_policy_catalog, RUNTIME_LAUNCH_PROFILE_EFFECT,
        RUNTIME_LAUNCH_PROFILE_NO_EFFECT,
    },
    store::Store,
};

use super::types::{
    ExternalPoolAdapterRuntimeLaunchPolicySummary, RuntimeLaunchPolicyCatalogEntry,
};

impl Store {
    pub(crate) fn external_pool_adapter_runtime_launch_policy_summary(
        &self,
    ) -> Result<ExternalPoolAdapterRuntimeLaunchPolicySummary> {
        let entry = runtime_launch_policy_catalog()?;
        Ok(ExternalPoolAdapterRuntimeLaunchPolicySummary {
            schema: "compute_federation.external_pool_adapter_runtime_launch_policy_summary.v1",
            policy_id: entry.policy.policy_id.clone(),
            policy_revision: entry.policy.policy_revision,
            policy_digest: entry.digest,
            runtime_kind: entry.policy.runtime_kind.clone(),
            host_os: entry.policy.host_os.clone(),
            host_arch: entry.policy.host_arch.clone(),
            host_environment: entry.policy.host_environment.clone(),
            executable_kind: entry.policy.executable_kind.clone(),
            binary_format: entry.policy.binary_format.clone(),
            resolver_backend_policy_id: entry.policy.resolver_backend_policy_id.clone(),
            resolver_backend_policy_revision: entry.policy.resolver_backend_policy_revision,
            process_isolation_policy_id: entry.policy.process_isolation_policy_id.clone(),
            resource_policy_id: entry.policy.resource_policy_id.clone(),
            network_egress_policy_id: entry.policy.network_egress_policy_id.clone(),
            profile_effect: RUNTIME_LAUNCH_PROFILE_EFFECT.into(),
            adapter_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
            runtime_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
            usage_effect: RUNTIME_LAUNCH_PROFILE_NO_EFFECT.into(),
        })
    }
}

pub(super) fn runtime_launch_policy_catalog() -> Result<RuntimeLaunchPolicyCatalogEntry> {
    let (policy, digest) = server_runtime_launch_policy_catalog()?;
    Ok(RuntimeLaunchPolicyCatalogEntry { policy, digest })
}
