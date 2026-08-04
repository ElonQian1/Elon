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
pub(super) struct StoredLeaseState {
    pub(in crate::store) provider_id: String,
    pub(in crate::store) consumer_account_id: String,
    pub(in crate::store) lease_revision: i64,
    pub(in crate::store) lease_digest: String,
    pub(in crate::store) lease: ComputeAttemptLease,
    status: String,
    fencing_generation: i64,
    expires_at: String,
    hard_deadline_at: String,
    last_heartbeat_at: Option<String>,
    pub(super) updated_by_user_id: String,
    pub(super) updated_at: String,
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
    renewal_id: String,
    pub(super) previous_lease_revision: i64,
    previous_lease_digest: String,
    pub(super) target_lease_revision: i64,
    pub(super) target_lease_digest: String,
    pub(super) target_lease: ComputeAttemptLease,
    executor_heartbeat_ref: String,
    pub(super) request_digest: String,
    event_digest: String,
    renewed_by_user_id: String,
    renewed_at: String,
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

pub(super) fn current_lease_state_on(
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
        "SELECT renewal_id, previous_lease_revision, previous_lease_digest,
                target_lease_revision, target_lease_digest, target_lease_json,
                executor_heartbeat_ref, request_digest, event_digest,
                renewed_by_user_id, renewed_at
           FROM compute_attempt_lease_renewals
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
        |row| {
            let target_json: String = row.get(5)?;
            let target_lease = serde_json::from_str(&target_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(StoredRenewal {
                renewal_id: row.get(0)?,
                previous_lease_revision: row.get(1)?,
                previous_lease_digest: row.get(2)?,
                target_lease_revision: row.get(3)?,
                target_lease_digest: row.get(4)?,
                target_lease,
                executor_heartbeat_ref: row.get(6)?,
                request_digest: row.get(7)?,
                event_digest: row.get(8)?,
                renewed_by_user_id: row.get(9)?,
                renewed_at: row.get(10)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
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
        || state.lease_digest != compute_attempt_lease_digest(&state.lease)?
    {
        bail!("Attempt Lease 当前状态投影审计失败");
    }
    Ok(())
}

pub(super) fn audit_renewal(stored: &StoredRenewal) -> Result<()> {
    if stored.target_lease_revision != stored.previous_lease_revision + 1
        || stored.target_lease.status != ATTEMPT_STATUS_RUNNING
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

pub(super) fn compute_attempt_lease_digest(lease: &ComputeAttemptLease) -> Result<String> {
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
