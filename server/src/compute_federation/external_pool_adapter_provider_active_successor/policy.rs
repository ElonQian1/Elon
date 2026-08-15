use anyhow::{bail, Result};

use super::*;

pub(crate) const PROVIDER_ACTIVE_SUCCESSOR_RELATIONAL_CURRENT_STATUS: &str =
    "relationally_current_requires_process_custody_and_active_root_reproof";

pub(crate) fn provider_active_successor_effects_none(
) -> ExternalPoolAdapterProviderActiveSuccessorEffects {
    ExternalPoolAdapterProviderActiveSuccessorEffects {
        credential_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        adapter_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        provider_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        route_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        activation_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        execution_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        usage_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        market_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
        settlement_effect: PROVIDER_ACTIVE_SUCCESSOR_NO_EFFECT.into(),
    }
}

pub(crate) fn provider_active_successor_readiness_none(
) -> ExternalPoolAdapterProviderActiveSuccessorReadiness {
    ExternalPoolAdapterProviderActiveSuccessorReadiness {
        process_spawn_ready: false,
        ipc_session_ready: false,
        secret_delivery_ready: false,
        broker_connect_ready: false,
        upstream_probe_ready: false,
        runtime_launch_ready: false,
        route_ready: false,
        execution_ready: false,
        activation_ready: false,
    }
}

pub(crate) fn validate_provider_active_successor_boundary(
    effects: &ExternalPoolAdapterProviderActiveSuccessorEffects,
    readiness: &ExternalPoolAdapterProviderActiveSuccessorReadiness,
) -> Result<()> {
    if effects != &provider_active_successor_effects_none()
        || readiness != &provider_active_successor_readiness_none()
    {
        bail!("provider active successor cannot carry effects or readiness")
    }
    Ok(())
}
