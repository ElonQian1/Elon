use anyhow::Result;

use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
        server_supervisor_session_policy_catalog, SUPERVISOR_SESSION_COMPANION_EFFECT,
        SUPERVISOR_SESSION_COMPANION_NO_EFFECT,
    },
    store::Store,
};

use super::types::*;

impl Store {
    pub(crate) fn external_pool_adapter_supervisor_session_policy_summary(
        &self,
    ) -> Result<ExternalPoolAdapterSupervisorSessionPolicySummary> {
        let entry = supervisor_session_policy_catalog()?;
        Ok(ExternalPoolAdapterSupervisorSessionPolicySummary {
            schema: "compute_federation.external_pool_adapter_supervisor_session_policy_summary.v1",
            policy_digest: entry.digest,
            policy: entry.policy,
            companion_effect: SUPERVISOR_SESSION_COMPANION_EFFECT.into(),
            adapter_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            runtime_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            provider_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            credential_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            route_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            execution_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            usage_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            market_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            settlement_effect: SUPERVISOR_SESSION_COMPANION_NO_EFFECT.into(),
            process_spawn_ready: false,
            ipc_session_ready: false,
            secret_delivery_ready: false,
            broker_connect_ready: false,
            upstream_probe_observed: false,
            runtime_launch_ready: false,
            activation_ready: false,
        })
    }
}

pub(super) fn supervisor_session_policy_catalog() -> Result<SupervisorSessionPolicyCatalogEntry> {
    let (policy, digest) = server_supervisor_session_policy_catalog()?;
    Ok(SupervisorSessionPolicyCatalogEntry { policy, digest })
}
