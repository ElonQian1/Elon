use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection, TransactionBehavior};

use crate::{
    compute_federation::external_pool_adapter_runtime_compatibility_verification::*,
    store::{
        compute_external_pool_adapter_registry::current_external_pool_adapter_registry_release_authority_on,
        compute_external_pool_adapter_sandbox_verifier_key::current_sandbox_verifier_key_authority_on,
        Store,
    },
};

use super::{
    error::ExternalPoolAdapterRuntimeCompatibilityVerificationStoreError as StoreError,
    read::{
        revocation_by_verification_on, run_observation_by_id_on, verification_by_id_on,
        verification_head_by_release_on,
    },
    types::CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority,
};

impl Store {
    pub(crate) fn external_pool_adapter_runtime_compatibility_verification_currentness(
        &self,
        registry_release_id: &str,
    ) -> std::result::Result<
        Option<ExternalPoolAdapterRuntimeCompatibilityCurrentnessSummary>,
        StoreError,
    > {
        super::read::identifier(registry_release_id).map_err(StoreError::conflict)?;
        let mut conn = self.conn().map_err(StoreError::storage)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|error| StoreError::storage(error.into()))?;
        let result = (|| -> Result<_> {
            let Some(head) = verification_head_by_release_on(&tx, registry_release_id)? else {
                return Ok(None);
            };
            let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
            let v = &head.receipt.verification;
            let revocation =
                revocation_by_verification_on(&tx, &head.receipt.verification_receipt_id)?;
            let release_current: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_registry_release_current
                  WHERE registry_release_id=?1 AND registry_release_digest=?2 AND current_status='release_current')",
                params![v.registry_release.registry_release_id,v.registry_release.registry_release_digest],
                |row| row.get(0),
            )?;
            let key_current: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM compute_external_pool_adapter_sandbox_verifier_key_current
                  WHERE key_record_id=?1 AND key_record_digest=?2 AND key_id=?3 AND current_status='active')",
                params![v.sandbox_verifier_key_record_id,v.sandbox_verifier_key_record_digest,v.sandbox_verifier_key_id],
                |row| row.get(0),
            )?;
            let policy_current = current_policy_roots(v)?;
            let expired = canonical_time(&v.expires_at)? <= canonical_time(&checked_at)?;
            let mut reasons = Vec::new();
            if revocation.is_some() {
                reasons.push("verification_revoked".into());
            }
            if expired {
                reasons.push("verification_expired".into());
            }
            if !release_current {
                reasons.push("registry_release_historical".into());
            }
            if !key_current {
                reasons.push("verifier_key_not_current".into());
            }
            if !policy_current {
                reasons.push("profile_or_policy_historical".into());
            }
            let summary = ExternalPoolAdapterRuntimeCompatibilityCurrentnessSummary {
                schema: RUNTIME_COMPATIBILITY_VERIFICATION_CURRENTNESS_SCHEMA.into(),
                registry_release_id: v.registry_release.registry_release_id.clone(),
                adapter_id: v.registry_release.release.adapter_id.clone(),
                release_version: v.registry_release.release.release_version.clone(),
                profile_id: v.profile_id.clone(),
                profile_revision: v.profile_revision,
                verification_receipt_id: head.receipt.verification_receipt_id.clone(),
                verification_receipt_digest: head.receipt.verification_receipt_digest.clone(),
                sequence: v.sequence,
                verified_at: v.verified_at.clone(),
                expires_at: v.expires_at.clone(),
                revoked_at: revocation.map(|stored| stored.receipt.revocation.revoked_at),
                currentness_status: if reasons.is_empty() {
                    RUNTIME_COMPATIBILITY_VERIFICATION_CURRENT_STATUS
                } else {
                    RUNTIME_COMPATIBILITY_VERIFICATION_HISTORICAL_STATUS
                }
                .into(),
                historical_reasons: reasons,
                effects: v.effects.clone(),
                readiness: v.readiness.clone(),
            };
            validate_runtime_compatibility_currentness_summary(&summary)?;
            Ok(Some(summary))
        })()
        .map_err(StoreError::storage)?;
        tx.commit()
            .map_err(|error| StoreError::storage(error.into()))?;
        Ok(result)
    }
}

pub(in crate::store) fn current_external_pool_adapter_runtime_compatibility_verification_authority_on(
    conn: &Connection,
    verification_receipt_id: &str,
    expected_verification_receipt_digest: &str,
    checked_at: &str,
) -> Result<Option<CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority>> {
    validate_checked_at(checked_at)?;
    let Some(stored) = verification_by_id_on(conn, verification_receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.verification_receipt_digest != expected_verification_receipt_digest {
        bail!("V268 expected verification receipt is not exact");
    }
    let v = &stored.receipt.verification;
    let head = verification_head_by_release_on(conn, &v.registry_release.registry_release_id)?
        .ok_or_else(|| anyhow::anyhow!("V268 verification lineage head disappeared"))?;
    if head.receipt.verification_receipt_id != stored.receipt.verification_receipt_id
        || revocation_by_verification_on(conn, verification_receipt_id)?.is_some()
        || canonical_time(&v.expires_at)? <= canonical_time(checked_at)?
        || !current_policy_roots(v)?
    {
        bail!("V268 verification is historical, revoked, expired, or policy-stale");
    }
    let release = current_external_pool_adapter_registry_release_authority_on(
        conn,
        &v.registry_release.registry_release_id,
        &v.registry_release.registry_release_digest,
        checked_at,
    )?
    .ok_or_else(|| anyhow::anyhow!("V268 verification lost current V249 release"))?;
    let key = current_sandbox_verifier_key_authority_on(
        conn,
        &v.sandbox_verifier_key_record_id,
        &v.sandbox_verifier_key_record_digest,
        &v.sandbox_verifier_key_id,
    )?
    .ok_or_else(|| anyhow::anyhow!("V268 verification lost current V237 key"))?;
    let observation = run_observation_by_id_on(conn, &v.run_observation_id)?
        .ok_or_else(|| anyhow::anyhow!("V268 verification lost its observation"))?;
    Ok(Some(
        CurrentExternalPoolAdapterRuntimeCompatibilityVerificationAuthority::new(
            stored.receipt,
            observation.receipt,
            release.release().clone(),
            key,
            checked_at.into(),
        ),
    ))
}

fn current_policy_roots(
    value: &ExternalPoolAdapterRuntimeCompatibilityVerificationMaterial,
) -> Result<bool> {
    let profile = server_runtime_compatibility_v2_profile_catalog()?;
    let (_, runner_digest) = server_runtime_compatibility_runner_policy_catalog()?;
    let (_, fixture_digest) = server_runtime_compatibility_public_fixture_catalog()?;
    Ok(value.profile_id == profile.profile.profile_id
        && value.profile_revision == profile.profile.profile_revision
        && value.profile_digest == profile.profile_digest
        && value.runner_policy_digest == runner_digest
        && value.fixture_catalog_digest == fixture_digest)
}

fn validate_checked_at(value: &str) -> Result<()> {
    let checked = canonical_time(value)?;
    let current = Utc::now();
    if checked < current - chrono::Duration::minutes(5)
        || checked > current + chrono::Duration::minutes(5)
    {
        bail!("V268 checked_at is not a current canonical observation");
    }
    Ok(())
}

fn canonical_time(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("V268 timestamp is not canonical UTC nanoseconds");
    }
    Ok(parsed.with_timezone(&Utc))
}
