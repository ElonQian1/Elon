//! Last-moment, one-shot authorization claim before shared credentials leave the API.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;

use super::{
    codex_vault_dispatch_authorization::require_dispatch_authorization_in_tx, common::now,
    node_compute_runs::select_run_by_compute_call_id, Store,
};

pub(crate) struct CodexVaultEmergencyCredentialDeliveryClaim<'a> {
    pub lease_id: &'a str,
    pub expected_lease_updated_at: &'a str,
    pub grant_id: &'a str,
    pub provider_user_id: &'a str,
    pub consumer_user_id: &'a str,
    pub consumer_node_id: &'a str,
    pub provider_slot_id: &'a str,
    pub credential_version: i64,
    pub compute_call_id: Option<&'a str>,
    pub cloud_control_deadline: &'a str,
}

impl Store {
    /// Atomically claims the right to return an exact shared credential once.
    ///
    /// Revocation, clearing, expiry, superseding and (for a mid-run switch)
    /// dispatch authorization are checked under the same SQLite transaction.
    /// The `updated_at` CAS makes the response claim one-shot and detects any
    /// lease mutation committed after creation but before this boundary.
    pub(crate) fn claim_codex_vault_emergency_credential_delivery(
        &self,
        claim: CodexVaultEmergencyCredentialDeliveryClaim<'_>,
    ) -> Result<bool> {
        let claimed_at = delivery_claim_time(claim.expected_lease_updated_at)?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;

        if let Some(compute_call_id) = normalized(claim.compute_call_id) {
            let Some(run) = select_run_by_compute_call_id(&tx, compute_call_id)? else {
                tx.rollback()?;
                return Ok(false);
            };
            require_dispatch_authorization_in_tx(
                &tx,
                &run,
                true,
                Some(claim.cloud_control_deadline),
                Some(claim.lease_id),
                Some(claim.provider_user_id),
            )?;
        }

        let changed = tx.execute(
            "UPDATE codex_vault_emergency_leases
                SET updated_at = ?10
              WHERE id = ?1
                AND updated_at = ?2
                AND grant_id = ?3
                AND provider_user_id = ?4
                AND consumer_user_id = ?5
                AND consumer_node_id = ?6
                AND provider_slot_id = ?7
                AND status = 'active'
                AND cleared_at IS NULL
                AND julianday(expires_at) IS NOT NULL
                AND julianday(expires_at) > julianday(?10)
                AND expires_at = ?8
                AND EXISTS (
                    SELECT 1
                      FROM codex_vault_emergency_grants AS grant
                     WHERE grant.id = codex_vault_emergency_leases.grant_id
                       AND grant.provider_user_id = codex_vault_emergency_leases.provider_user_id
                       AND grant.consumer_user_id = codex_vault_emergency_leases.consumer_user_id
                       AND grant.status = 'active'
                       AND grant.revoked_at IS NULL
                       AND (grant.expires_at IS NULL OR (
                           julianday(grant.expires_at) IS NOT NULL
                           AND julianday(grant.expires_at) > julianday(?10)
                       ))
                )
                AND EXISTS (
                    SELECT 1
                      FROM user_codex_credential_slots AS slot
                     WHERE slot.slot_id = codex_vault_emergency_leases.provider_slot_id
                       AND slot.user_id = codex_vault_emergency_leases.provider_user_id
                       AND slot.credential_version = ?9
                       AND slot.status IN ('active', 'degraded')
                )
                AND NOT EXISTS (
                    SELECT 1
                      FROM codex_vault_emergency_leases AS competing
                     WHERE competing.consumer_user_id = codex_vault_emergency_leases.consumer_user_id
                       AND competing.consumer_node_id = codex_vault_emergency_leases.consumer_node_id
                       AND competing.id != codex_vault_emergency_leases.id
                       AND competing.status = 'active'
                       AND competing.cleared_at IS NULL
                       AND julianday(competing.expires_at) IS NOT NULL
                       AND julianday(competing.expires_at) > julianday(?10)
                )",
            params![
                claim.lease_id,
                claim.expected_lease_updated_at,
                claim.grant_id,
                claim.provider_user_id,
                claim.consumer_user_id,
                claim.consumer_node_id,
                claim.provider_slot_id,
                claim.cloud_control_deadline,
                claim.credential_version,
                claimed_at,
            ],
        )?;
        if changed != 1 {
            tx.rollback()?;
            return Ok(false);
        }
        tx.commit()?;
        Ok(true)
    }
}

fn delivery_claim_time(expected_updated_at: &str) -> Result<String> {
    let expected = DateTime::parse_from_rfc3339(expected_updated_at)
        .context("共享 Codex 租约更新时间无效")?
        .with_timezone(&Utc);
    let current = DateTime::parse_from_rfc3339(&now())
        .context("服务器时间无效")?
        .with_timezone(&Utc);
    Ok(if current > expected {
        current
    } else {
        expected + Duration::nanoseconds(1)
    }
    .to_rfc3339())
}

fn normalized(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "codex_vault_emergency_delivery_guard_tests.rs"]
mod tests;
