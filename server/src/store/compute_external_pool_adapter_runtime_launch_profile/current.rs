use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use rusqlite::Connection;

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_runtime_launch_profile::{
            RUNTIME_LAUNCH_PROFILE_CURRENTNESS_SCHEMA, RUNTIME_LAUNCH_PROFILE_STATUS,
        },
    },
    store::{
        compute_external_pool_provider_activation_candidate::current_external_pool_provider_activation_candidate_static_authority_on,
        Store,
    },
};

use super::{policy::runtime_launch_policy_catalog, read::*, roots::*, types::*};

impl Store {
    pub(crate) fn external_pool_adapter_runtime_launch_profile_currentness(
        &self,
        profile_id: &str,
        prepared: PreparedExternalPoolAdapterInstallation,
    ) -> Result<Option<ExternalPoolAdapterRuntimeLaunchProfileCurrentness>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let checked_at = now();
        let Some(authority) = current_external_pool_adapter_runtime_launch_profile_authority_on(
            &tx,
            profile_id,
            prepared,
            &checked_at,
        )?
        else {
            return Ok(None);
        };
        let output = ExternalPoolAdapterRuntimeLaunchProfileCurrentness {
            schema: RUNTIME_LAUNCH_PROFILE_CURRENTNESS_SCHEMA,
            profile: profile_summary(authority.profile()),
            current_status: RUNTIME_LAUNCH_PROFILE_STATUS.into(),
            provider_status: "registering".into(),
            candidate_status: "candidate_current_not_activation_ready".into(),
            file_inventory_status: "prepared_exact".into(),
            launch_policy_status: "server_policy_current".into(),
            revocation_status: "unrevoked".into(),
            runtime_launch_ready: false,
            checked_at: authority.checked_at().to_string(),
        };
        tx.commit()?;
        Ok(Some(output))
    }
}

pub(in crate::store) fn current_external_pool_adapter_runtime_launch_profile_authority_on(
    conn: &Connection,
    profile_id: &str,
    prepared: PreparedExternalPoolAdapterInstallation,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority>> {
    let Some(profile) = profile_by_id_on(conn, profile_id)? else {
        return Ok(None);
    };
    validate_checked_at(checked_at, &profile.receipt.profile.recorded_at)?;
    let head = profile_head_by_binding_on(conn, &profile.receipt.profile.provider_binding_id)?
        .ok_or_else(|| anyhow::anyhow!("runtime launch profile lost lineage head"))?;
    if head.receipt.profile_id != profile.receipt.profile_id
        || revocation_by_profile_on(conn, profile_id)?.is_some()
    {
        bail!("runtime launch profile is historical, superseded, or revoked");
    }
    let p = &profile.receipt.profile;
    let candidate = current_external_pool_provider_activation_candidate_static_authority_on(
        conn,
        prepared,
        &p.candidate_id,
        &p.candidate_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("current exact V254 candidate was not found"))?;
    let policy = runtime_launch_policy_catalog()?;
    if policy.digest != p.launch_policy_digest || policy.policy != p.launch_policy {
        bail!("runtime launch profile policy is historical");
    }
    let (scheme, commitment) = credential_subject_on(conn, candidate.registry())?;
    audit_current_roots(
        candidate.registry(),
        candidate.candidate(),
        &profile.receipt,
        &scheme,
        &commitment,
    )?;
    if candidate.checked_at() != checked_at {
        bail!("runtime launch profile current roots were not checked at one instant");
    }
    Ok(Some(
        CurrentExternalPoolAdapterRuntimeLaunchProfileAuthority::new(
            profile.receipt,
            candidate,
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
        || checked > Utc::now() + Duration::minutes(5)
    {
        bail!("runtime launch profile checked_at is not a current canonical observation");
    }
    Ok(())
}
