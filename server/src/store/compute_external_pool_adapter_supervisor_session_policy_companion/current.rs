use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_supervisor_session_policy_companion::{
            SUPERVISOR_SESSION_COMPANION_CURRENTNESS_SCHEMA, SUPERVISOR_SESSION_COMPANION_STATUS,
        },
    },
    store::{
        compute_external_pool_adapter_runtime_bundle::external_pool_adapter_entrypoint_capsule_policy_root,
        compute_external_pool_adapter_upstream_transport_target::current_external_pool_adapter_upstream_transport_target_authority_on,
        Store,
    },
};

use super::{policy::supervisor_session_policy_catalog, read::*, roots::*, types::*};

impl Store {
    pub(crate) fn external_pool_adapter_supervisor_session_policy_companion_currentness(
        &self,
        companion_id: &str,
        expected_companion_digest: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = now();
        let Some(a) =
            current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
                &tx,
                companion_id,
                expected_companion_digest,
                prepared,
                &checked_at,
            )?
        else {
            tx.commit()?;
            return Ok(None);
        };
        let c = &a.companion().companion;
        let output = ExternalPoolAdapterSupervisorSessionPolicyCompanionCurrentness {
            schema: SUPERVISOR_SESSION_COMPANION_CURRENTNESS_SCHEMA,
            companion: companion_summary(a.companion()),
            current_status: SUPERVISOR_SESSION_COMPANION_STATUS.into(),
            provider_status: "registering".into(),
            profile_status: "launch_profile_current_inert".into(),
            target_status: "upstream_transport_target_current_inert".into(),
            policy_status: "server_policy_current".into(),
            revocation_status: "unrevoked".into(),
            adapter_effect: c.adapter_effect.clone(),
            runtime_effect: c.runtime_effect.clone(),
            provider_effect: c.provider_effect.clone(),
            credential_effect: c.credential_effect.clone(),
            route_effect: c.route_effect.clone(),
            execution_effect: c.execution_effect.clone(),
            usage_effect: c.usage_effect.clone(),
            market_effect: c.market_effect.clone(),
            settlement_effect: c.settlement_effect.clone(),
            process_spawn_ready: c.process_spawn_ready,
            ipc_session_ready: c.ipc_session_ready,
            secret_delivery_ready: c.secret_delivery_ready,
            broker_connect_ready: c.broker_connect_ready,
            upstream_probe_observed: c.upstream_probe_observed,
            runtime_launch_ready: c.runtime_launch_ready,
            activation_ready: c.activation_ready,
            checked_at: a.checked_at().into(),
        };
        tx.commit()?;
        Ok(Some(output))
    }
}

pub(in crate::store) fn current_external_pool_adapter_supervisor_session_policy_companion_authority_on(
    conn: &Connection,
    companion_id: &str,
    expected_companion_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority>> {
    let Some(stored) = companion_by_id_on(conn, companion_id)? else {
        return Ok(None);
    };
    if stored.receipt.companion_digest != expected_companion_digest {
        bail!("supervisor session companion expected digest is not exact")
    }
    validate_checked_at(checked_at, &stored.receipt.companion.recorded_at)?;
    let c = &stored.receipt.companion;
    let head = companion_head_by_binding_on(conn, &c.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("supervisor session companion lost lineage head"))?;
    if head.receipt.companion_id != stored.receipt.companion_id
        || revocation_by_companion_on(conn, companion_id)?.is_some()
    {
        bail!("supervisor session companion is historical, superseded, or revoked")
    }
    let target = current_external_pool_adapter_upstream_transport_target_authority_on(
        conn,
        &c.target_id,
        &c.target_digest,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V258 target was not found"))?;
    let policy = supervisor_session_policy_catalog()?;
    if policy.digest != c.supervisor_session_policy_digest
        || policy.policy != c.supervisor_session_policy
    {
        bail!("supervisor session companion policy is historical")
    }
    let (capsule_policy_id, capsule_policy_revision, capsule_policy_digest) =
        external_pool_adapter_entrypoint_capsule_policy_root()?;
    if capsule_policy_id != c.entrypoint_capsule_policy_id
        || capsule_policy_revision != c.entrypoint_capsule_policy_revision
        || capsule_policy_digest != c.entrypoint_capsule_policy_digest
    {
        bail!("supervisor session companion V257 capsule policy is historical")
    }
    audit_current_roots(&target, &stored.receipt)?;
    if target.checked_at() != checked_at {
        bail!("supervisor session companion roots were not checked at one instant")
    }
    Ok(Some(
        CurrentExternalPoolAdapterSupervisorSessionPolicyCompanionAuthority::new(
            stored.receipt,
            target,
            checked_at.into(),
        ),
    ))
}
fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
fn validate_checked_at(checked_at: &str, recorded_at: &str) -> Result<()> {
    let checked = DateTime::parse_from_rfc3339(checked_at)?;
    let recorded = DateTime::parse_from_rfc3339(recorded_at)?;
    if checked.offset().local_minus_utc() != 0
        || checked.to_rfc3339_opts(SecondsFormat::Nanos, true) != checked_at
        || recorded.offset().local_minus_utc() != 0
        || recorded.to_rfc3339_opts(SecondsFormat::Nanos, true) != recorded_at
        || checked < recorded
        || checked < Utc::now() - Duration::minutes(5)
        || checked > Utc::now() + Duration::minutes(5)
    {
        bail!("supervisor session companion checked_at is not current canonical UTC nanos")
    }
    Ok(())
}
