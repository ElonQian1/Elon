use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    execution::{ComputeAttemptLease, ATTEMPT_STATUS_RUNNING, ATTEMPT_STATUS_STAGING},
    provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
};

use super::{
    super::compute_provider_registry::current_registered_provider_on,
    ComputeAttemptLeaseRenewalReceipt, ComputeAttemptLeaseStateReceipt,
    RenewComputeAttemptLeaseRequest, COMPUTE_ATTEMPT_LEASE_RENEWAL_SCHEMA,
    COMPUTE_ATTEMPT_LEASE_STATE_SCHEMA,
};

#[derive(Debug, Clone)]
pub(crate) struct StoredLeaseState {
    pub(in crate::store) provider_id: String,
    pub(in crate::store) consumer_account_id: String,
    pub(in crate::store) lease_revision: i64,
    pub(in crate::store) lease_digest: String,
    pub(in crate::store) lease: ComputeAttemptLease,
    lease_json: String,
    status: String,
    fencing_generation: i64,
    expires_at: String,
    hard_deadline_at: String,
    last_heartbeat_at: Option<String>,
    pub(super) updated_by_user_id: String,
    pub(in crate::store) updated_at: String,
}

impl StoredLeaseState {
    pub(super) fn into_receipt(self) -> Result<ComputeAttemptLeaseStateReceipt> {
        audit_state(&self)?;
        Ok(ComputeAttemptLeaseStateReceipt {
            schema: COMPUTE_ATTEMPT_LEASE_STATE_SCHEMA,
            lease: self.lease,
            lease_revision: self.lease_revision,
            lease_digest: self.lease_digest,
            updated_by_user_id: self.updated_by_user_id,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct StoredRenewal {
    pub(super) renewal_id: String,
    pub(super) lease_id: String,
    pub(super) provider_id: String,
    pub(super) consumer_account_id: String,
    pub(super) previous_lease_revision: i64,
    pub(super) previous_lease_digest: String,
    pub(super) target_lease_revision: i64,
    pub(super) target_lease_digest: String,
    pub(super) target_lease: ComputeAttemptLease,
    pub(super) target_lease_json: String,
    pub(super) previous_status: String,
    pub(super) target_status: String,
    pub(super) fencing_generation: i64,
    pub(super) previous_expires_at: String,
    pub(super) target_expires_at: String,
    pub(super) hard_deadline_at: String,
    pub(super) executor_heartbeat_ref: String,
    pub(super) request_digest: String,
    pub(super) event_digest: String,
    pub(super) idempotency_scope: String,
    pub(super) idempotency_key: String,
    pub(super) renewed_by_user_id: String,
    pub(super) renewed_at: String,
    pub(super) created_at: String,
}

impl StoredRenewal {
    pub(super) fn into_receipt(self, replayed: bool) -> Result<ComputeAttemptLeaseRenewalReceipt> {
        Ok(ComputeAttemptLeaseRenewalReceipt {
            schema: COMPUTE_ATTEMPT_LEASE_RENEWAL_SCHEMA,
            renewal_id: self.renewal_id,
            previous_lease_revision: self.previous_lease_revision,
            previous_lease_digest: self.previous_lease_digest,
            state: ComputeAttemptLeaseStateReceipt {
                schema: COMPUTE_ATTEMPT_LEASE_STATE_SCHEMA,
                lease: self.target_lease,
                lease_revision: self.target_lease_revision,
                lease_digest: self.target_lease_digest,
                updated_by_user_id: self.renewed_by_user_id.clone(),
                updated_at: self.renewed_at.clone(),
            },
            executor_heartbeat_ref: self.executor_heartbeat_ref,
            request_digest: self.request_digest,
            event_digest: self.event_digest,
            renewed_by_user_id: self.renewed_by_user_id,
            renewed_at: self.renewed_at,
            execution_effect: "external_liveness_assertion_only",
            capacity_effect: "unchanged",
            reservation_effect: "unchanged",
            money_effect: "preauthorization_unchanged",
            replayed,
        })
    }
}

pub(crate) fn current_lease_state_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<Option<StoredLeaseState>> {
    conn.query_row(
        "SELECT provider_id, consumer_account_id, lease_revision, lease_digest,
                lease_json, status, fencing_generation, expires_at,
                hard_deadline_at, last_heartbeat_at, updated_by_user_id, updated_at
           FROM compute_attempt_lease_states WHERE lease_id=?1",
        params![lease_id],
        stored_state_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(crate) fn list_current_lease_states_on(
    conn: &Connection,
    provider_id: &str,
    limit: usize,
) -> Result<Vec<StoredLeaseState>> {
    let mut statement = conn.prepare(
        "SELECT provider_id, consumer_account_id, lease_revision, lease_digest,
                lease_json, status, fencing_generation, expires_at,
                hard_deadline_at, last_heartbeat_at, updated_by_user_id, updated_at
           FROM compute_attempt_lease_states
          WHERE provider_id=?1
          ORDER BY updated_at DESC, lease_id ASC
          LIMIT ?2",
    )?;
    let result = statement
        .query_map(params![provider_id, limit as i64], stored_state_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into);
    result
}

fn stored_state_from_row(row: &Row<'_>) -> rusqlite::Result<StoredLeaseState> {
    let lease_json: String = row.get(4)?;
    let lease = serde_json::from_str(&lease_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(StoredLeaseState {
        provider_id: row.get(0)?,
        consumer_account_id: row.get(1)?,
        lease_revision: row.get(2)?,
        lease_digest: row.get(3)?,
        lease,
        lease_json,
        status: row.get(5)?,
        fencing_generation: row.get(6)?,
        expires_at: row.get(7)?,
        hard_deadline_at: row.get(8)?,
        last_heartbeat_at: row.get(9)?,
        updated_by_user_id: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub(super) fn renewal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRenewal>> {
    conn.query_row(
        "SELECT renewal_id, lease_id, provider_id, consumer_account_id,
                previous_lease_revision, previous_lease_digest,
                target_lease_revision, target_lease_digest, target_lease_json,
                previous_status, target_status, fencing_generation,
                previous_expires_at, target_expires_at, hard_deadline_at,
                executor_heartbeat_ref, request_digest, event_digest,
                idempotency_scope, idempotency_key, renewed_by_user_id,
                renewed_at, created_at
           FROM compute_attempt_lease_renewals
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
        stored_renewal_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn renewals_for_lease_through_revision_on(
    conn: &Connection,
    lease_id: &str,
    target_lease_revision: i64,
) -> Result<Vec<StoredRenewal>> {
    let mut statement = conn.prepare(
        "SELECT renewal_id, lease_id, provider_id, consumer_account_id,
                previous_lease_revision, previous_lease_digest,
                target_lease_revision, target_lease_digest, target_lease_json,
                previous_status, target_status, fencing_generation,
                previous_expires_at, target_expires_at, hard_deadline_at,
                executor_heartbeat_ref, request_digest, event_digest,
                idempotency_scope, idempotency_key, renewed_by_user_id,
                renewed_at, created_at
           FROM compute_attempt_lease_renewals
          WHERE lease_id=?1 AND target_lease_revision<=?2
          ORDER BY target_lease_revision ASC",
    )?;
    let rows = statement.query_map(
        params![lease_id, target_lease_revision],
        stored_renewal_from_row,
    )?;
    let renewals = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(renewals)
}

fn stored_renewal_from_row(row: &Row<'_>) -> rusqlite::Result<StoredRenewal> {
    let target_json: String = row.get(8)?;
    let target_lease = serde_json::from_str(&target_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(StoredRenewal {
        renewal_id: row.get(0)?,
        lease_id: row.get(1)?,
        provider_id: row.get(2)?,
        consumer_account_id: row.get(3)?,
        previous_lease_revision: row.get(4)?,
        previous_lease_digest: row.get(5)?,
        target_lease_revision: row.get(6)?,
        target_lease_digest: row.get(7)?,
        target_lease,
        target_lease_json: target_json,
        previous_status: row.get(9)?,
        target_status: row.get(10)?,
        fencing_generation: row.get(11)?,
        previous_expires_at: row.get(12)?,
        target_expires_at: row.get(13)?,
        hard_deadline_at: row.get(14)?,
        executor_heartbeat_ref: row.get(15)?,
        request_digest: row.get(16)?,
        event_digest: row.get(17)?,
        idempotency_scope: row.get(18)?,
        idempotency_key: row.get(19)?,
        renewed_by_user_id: row.get(20)?,
        renewed_at: row.get(21)?,
        created_at: row.get(22)?,
    })
}

pub(super) fn ensure_renewal_owner(
    conn: &Connection,
    input: &RenewComputeAttemptLeaseRequest,
    current: &StoredLeaseState,
) -> Result<()> {
    let provider = current_registered_provider_on(conn, &input.provider_id)?
        .ok_or_else(|| anyhow!("Attempt Lease Provider 不存在"))?;
    if current.provider_id != input.provider_id
        || provider.provider.owner_account_id != input.renewed_by_user_id
        || !matches!(
            provider.provider.status.as_str(),
            PROVIDER_STATUS_ACTIVE | PROVIDER_STATUS_DRAINING
        )
    {
        bail!("只有当前 Provider 所有者可为 active/draining Provider 的既有 Lease 续租");
    }
    Ok(())
}

pub(super) fn ensure_expected_state(
    input: &RenewComputeAttemptLeaseRequest,
    current: &StoredLeaseState,
) -> Result<()> {
    if current.lease_revision != input.expected_lease_revision
        || current.lease_digest != input.expected_lease_digest
        || current.lease.fencing_generation != input.expected_fencing_generation
        || !matches!(
            current.lease.status.as_str(),
            ATTEMPT_STATUS_STAGING | ATTEMPT_STATUS_RUNNING
        )
    {
        bail!("Attempt Lease 只能基于当前 staging/running 状态的精确版本、摘要和 fencing 代次续租");
    }
    Ok(())
}

pub(super) fn ensure_renewal_window(
    input: &RenewComputeAttemptLeaseRequest,
    current: &StoredLeaseState,
    renewed_at: &str,
) -> Result<()> {
    let now = parse_utc(renewed_at)?;
    let current_expiry = parse_utc(&current.lease.expires_at)?;
    let target_expiry = parse_utc(&input.expires_at)?;
    let hard_deadline = parse_utc(&current.lease.hard_deadline_at)?;
    if now >= current_expiry || now >= hard_deadline {
        bail!("已过期的 Attempt Lease 不可续租或复活");
    }
    if target_expiry <= current_expiry || target_expiry > hard_deadline {
        bail!("续租后的软期限必须晚于当前期限且不得越过不可变硬期限");
    }
    Ok(())
}

fn audit_state(state: &StoredLeaseState) -> Result<()> {
    if state.lease_revision <= 0
        || state.provider_id != state.lease.provider_id
        || state.status != state.lease.status
        || state.fencing_generation != state.lease.fencing_generation
        || state.expires_at != state.lease.expires_at
        || state.hard_deadline_at != state.lease.hard_deadline_at
        || state.last_heartbeat_at != state.lease.last_heartbeat_at
        || state.lease_json != serde_json::to_string(&state.lease)?
        || state.lease_digest != compute_attempt_lease_digest(&state.lease)?
    {
        bail!("Attempt Lease 当前状态投影审计失败");
    }
    Ok(())
}

pub(super) fn audit_renewal(stored: &StoredRenewal) -> Result<()> {
    let request = RenewComputeAttemptLeaseRequest {
        lease_id: stored.lease_id.clone(),
        provider_id: stored.provider_id.clone(),
        expected_lease_revision: stored.previous_lease_revision,
        expected_lease_digest: stored.previous_lease_digest.clone(),
        expected_fencing_generation: stored.fencing_generation,
        executor_heartbeat_ref: stored.executor_heartbeat_ref.clone(),
        expires_at: stored.target_expires_at.clone(),
        idempotency_key: stored.idempotency_key.clone(),
        renewed_by_user_id: stored.renewed_by_user_id.clone(),
    };
    validate_renewal_input(&request)?;
    if stored.target_lease_revision
        != stored
            .previous_lease_revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("Attempt Lease 续租修订号溢出"))?
        || stored.lease_id != stored.target_lease.lease_id
        || stored.provider_id != stored.target_lease.provider_id
        || stored.consumer_account_id.trim().is_empty()
        || stored.previous_status
            != if stored.previous_lease_revision == 1 {
                ATTEMPT_STATUS_STAGING
            } else {
                ATTEMPT_STATUS_RUNNING
            }
        || stored.target_status != ATTEMPT_STATUS_RUNNING
        || stored.target_lease.status != ATTEMPT_STATUS_RUNNING
        || stored.target_status != stored.target_lease.status
        || stored.target_lease_json != serde_json::to_string(&stored.target_lease)?
        || stored.fencing_generation != stored.target_lease.fencing_generation
        || stored.target_expires_at != stored.target_lease.expires_at
        || stored.hard_deadline_at != stored.target_lease.hard_deadline_at
        || stored.created_at != stored.renewed_at
        || stored.idempotency_scope
            != format!("compute_attempt_lease_renewal:{}", stored.provider_id)
        || renewal_request_digest(&request)? != stored.request_digest
        || stored.target_lease_digest != compute_attempt_lease_digest(&stored.target_lease)?
        || stored.target_lease.last_heartbeat_at.as_deref() != Some(stored.renewed_at.as_str())
        || stored.event_digest
            != renewal_event_digest(
                &stored.renewal_id,
                &stored.target_lease.lease_id,
                stored.previous_lease_revision,
                &stored.previous_lease_digest,
                stored.target_lease_revision,
                &stored.target_lease_digest,
                &stored.executor_heartbeat_ref,
                &stored.request_digest,
                &stored.renewed_by_user_id,
                &stored.renewed_at,
            )?
    {
        bail!("Attempt Lease 续租回执审计失败");
    }
    Ok(())
}

pub(super) fn renewal_request_digest(input: &RenewComputeAttemptLeaseRequest) -> Result<String> {
    digest_json(&serde_json::json!({
        "purpose":"compute_attempt_lease_renewal",
        "lease_id":input.lease_id,
        "provider_id":input.provider_id,
        "expected_lease_revision":input.expected_lease_revision,
        "expected_lease_digest":input.expected_lease_digest,
        "expected_fencing_generation":input.expected_fencing_generation,
        "executor_heartbeat_ref":input.executor_heartbeat_ref,
        "expires_at":input.expires_at,
        "idempotency_key":input.idempotency_key,
        "renewed_by_user_id":input.renewed_by_user_id,
    }))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn renewal_event_digest(
    renewal_id: &str,
    lease_id: &str,
    previous_revision: i64,
    previous_digest: &str,
    target_revision: i64,
    target_digest: &str,
    heartbeat_ref: &str,
    request_digest: &str,
    actor: &str,
    renewed_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ATTEMPT_LEASE_RENEWAL_SCHEMA,
        "renewal_id":renewal_id,
        "lease_id":lease_id,
        "previous_lease_revision":previous_revision,
        "previous_lease_digest":previous_digest,
        "target_lease_revision":target_revision,
        "target_lease_digest":target_digest,
        "executor_heartbeat_ref":heartbeat_ref,
        "request_digest":request_digest,
        "renewed_by_user_id":actor,
        "renewed_at":renewed_at,
    }))
}

pub(crate) fn compute_attempt_lease_digest(lease: &ComputeAttemptLease) -> Result<String> {
    digest_json(lease)
}

fn digest_json(value: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(value)?)))
}

pub(super) fn validate_renewal_input(input: &RenewComputeAttemptLeaseRequest) -> Result<()> {
    for (label, value, max_len) in [
        ("Attempt Lease ID", input.lease_id.as_str(), 200),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("执行器心跳引用", input.executor_heartbeat_ref.as_str(), 500),
        ("续租软期限", input.expires_at.as_str(), 100),
        ("续租幂等键", input.idempotency_key.as_str(), 160),
        ("续租执行人", input.renewed_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    if input.expected_lease_revision <= 0 || input.expected_fencing_generation <= 0 {
        bail!("预期 Lease 修订号和 fencing 代次必须为正整数");
    }
    validate_digest("预期 Lease 摘要", &input.expected_lease_digest)?;
    parse_utc(&input.expires_at)?;
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

pub(super) fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
