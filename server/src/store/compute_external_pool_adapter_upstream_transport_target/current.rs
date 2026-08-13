use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_upstream_transport_target::{
            UPSTREAM_TRANSPORT_TARGET_CURRENTNESS_SCHEMA, UPSTREAM_TRANSPORT_TARGET_STATUS,
        },
    },
    store::{
        compute_external_pool_adapter_runtime_launch_profile::current_external_pool_adapter_runtime_launch_profile_authority_on,
        Store,
    },
};

use super::{policy::upstream_transport_target_policy_catalog, read::*, roots::*, types::*};

impl Store {
    pub(crate) fn external_pool_adapter_upstream_transport_target_currentness(
        &self,
        target_id: &str,
        expected_target_digest: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<ExternalPoolAdapterUpstreamTransportTargetCurrentness>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = now();
        let Some(authority) = current_external_pool_adapter_upstream_transport_target_authority_on(
            &tx,
            target_id,
            expected_target_digest,
            prepared,
            &checked_at,
        )?
        else {
            return Ok(None);
        };
        let output = ExternalPoolAdapterUpstreamTransportTargetCurrentness {
            schema: UPSTREAM_TRANSPORT_TARGET_CURRENTNESS_SCHEMA,
            target: target_summary(authority.target()),
            current_status: UPSTREAM_TRANSPORT_TARGET_STATUS.into(),
            provider_status: "registering".into(),
            profile_status: "launch_profile_current_inert".into(),
            target_policy_status: "server_policy_current".into(),
            revocation_status: "unrevoked".into(),
            broker_connect_ready: false,
            upstream_probe_observed: false,
            runtime_launch_ready: false,
            activation_ready: false,
            checked_at: authority.checked_at().to_string(),
        };
        tx.commit()?;
        Ok(Some(output))
    }
}

pub(in crate::store) fn current_external_pool_adapter_upstream_transport_target_authority_on(
    conn: &Connection,
    target_id: &str,
    expected_target_digest: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterUpstreamTransportTargetAuthority>> {
    let Some(target) = target_by_id_on(conn, target_id)? else {
        return Ok(None);
    };
    if target.receipt.target_digest != expected_target_digest {
        bail!("upstream transport target expected digest is not exact");
    }
    validate_checked_at(checked_at, &target.receipt.target.recorded_at)?;
    let head = target_head_by_binding_on(conn, &target.receipt.target.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("upstream transport target lost lineage head"))?;
    if head.receipt.target_id != target.receipt.target_id
        || revocation_by_target_on(conn, target_id)?.is_some()
    {
        bail!("upstream transport target is historical, superseded, or revoked");
    }
    let t = &target.receipt.target;
    let profile = current_external_pool_adapter_runtime_launch_profile_authority_on(
        conn,
        &t.profile_id,
        prepared,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V255 runtime launch profile was not found"))?;
    let policy = upstream_transport_target_policy_catalog()?;
    if policy.digest != t.target_policy_digest || policy.policy != t.target_policy {
        bail!("upstream transport target policy is historical");
    }
    audit_current_roots(&profile, &target.receipt)?;
    if profile.checked_at() != checked_at {
        bail!("upstream transport target roots were not checked at one instant");
    }
    Ok(Some(
        CurrentExternalPoolAdapterUpstreamTransportTargetAuthority::new(
            target.receipt,
            profile,
            checked_at.to_string(),
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
        bail!("upstream transport target checked_at is not a current canonical observation");
    }
    Ok(())
}
