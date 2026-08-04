use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;

use crate::compute_federation::execution::{
    ComputeAttemptLease, ATTEMPT_STATUS_RUNNING, ATTEMPT_STATUS_STAGING, ATTEMPT_STATUS_TERMINAL,
};

use super::{new_id, Store};

mod support;

use support::{
    audit_renewal, ensure_expected_state, ensure_renewal_owner, ensure_renewal_window,
    renewal_by_idempotency_on, renewal_event_digest, renewal_request_digest, validate_exact,
    validate_renewal_input,
};
pub(super) use support::{compute_attempt_lease_digest, current_lease_state_on, StoredLeaseState};

pub(crate) const COMPUTE_ATTEMPT_LEASE_STATE_SCHEMA: &str =
    "compute_federation.attempt_lease_state.v1";
pub(crate) const COMPUTE_ATTEMPT_LEASE_RENEWAL_SCHEMA: &str =
    "compute_federation.attempt_lease_renewal.v1";

#[derive(Debug, Clone)]
pub(crate) struct RenewComputeAttemptLeaseRequest {
    pub lease_id: String,
    pub provider_id: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub executor_heartbeat_ref: String,
    pub expires_at: String,
    pub idempotency_key: String,
    pub renewed_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptLeaseStateReceipt {
    pub schema: &'static str,
    pub lease: ComputeAttemptLease,
    pub lease_revision: i64,
    pub lease_digest: String,
    pub updated_by_user_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeAttemptLeaseRenewalReceipt {
    pub schema: &'static str,
    pub renewal_id: String,
    pub previous_lease_revision: i64,
    pub previous_lease_digest: String,
    pub state: ComputeAttemptLeaseStateReceipt,
    pub executor_heartbeat_ref: String,
    pub request_digest: String,
    pub event_digest: String,
    pub renewed_by_user_id: String,
    pub renewed_at: String,
    pub execution_effect: &'static str,
    pub capacity_effect: &'static str,
    pub reservation_effect: &'static str,
    pub money_effect: &'static str,
    pub replayed: bool,
}

impl Store {
    pub(crate) fn renew_compute_attempt_lease(
        &self,
        input: &RenewComputeAttemptLeaseRequest,
    ) -> Result<ComputeAttemptLeaseRenewalReceipt> {
        validate_renewal_input(input)?;
        let request_digest = renewal_request_digest(input)?;
        let idempotency_scope = format!("compute_attempt_lease_renewal:{}", input.provider_id);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            renewal_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同 Attempt Lease 续租幂等键不能用于不同请求");
            }
            audit_renewal(&stored)?;
            tx.commit()?;
            return Ok(stored.into_receipt(true)?);
        }

        let current = current_lease_state_on(&tx, &input.lease_id)?
            .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
        ensure_renewal_owner(&tx, input, &current)?;
        ensure_expected_state(input, &current)?;
        let renewed_at = Utc::now().to_rfc3339();
        ensure_renewal_window(input, &current, &renewed_at)?;

        let mut target_lease = current.lease.clone();
        target_lease.status = ATTEMPT_STATUS_RUNNING.to_string();
        target_lease.last_heartbeat_at = Some(renewed_at.clone());
        target_lease.expires_at = input.expires_at.clone();
        let target_digest = compute_attempt_lease_digest(&target_lease)?;
        let target_revision = current
            .lease_revision
            .checked_add(1)
            .context("Attempt Lease 修订号溢出")?;
        let renewal_id = new_id("compute_attempt_lease_renewal");
        let event_digest = renewal_event_digest(
            &renewal_id,
            &target_lease.lease_id,
            current.lease_revision,
            &current.lease_digest,
            target_revision,
            &target_digest,
            &input.executor_heartbeat_ref,
            &request_digest,
            &input.renewed_by_user_id,
            &renewed_at,
        )?;

        let changed = tx.execute(
            "UPDATE compute_attempt_lease_states
                SET lease_revision=?1, lease_digest=?2, lease_json=?3,
                    status=?4, expires_at=?5, last_heartbeat_at=?6,
                    updated_by_user_id=?7, updated_at=?8
              WHERE lease_id=?9 AND lease_revision=?10 AND lease_digest=?11",
            params![
                target_revision,
                target_digest,
                serde_json::to_string(&target_lease)?,
                target_lease.status,
                target_lease.expires_at,
                target_lease.last_heartbeat_at,
                input.renewed_by_user_id,
                renewed_at,
                input.lease_id,
                current.lease_revision,
                current.lease_digest,
            ],
        )?;
        if changed != 1 {
            bail!("Attempt Lease 已被并发修改，请重新读取当前状态");
        }
        tx.execute(
            "INSERT INTO compute_attempt_lease_renewals (
                renewal_id, lease_id, provider_id, consumer_account_id,
                previous_lease_revision, previous_lease_digest,
                target_lease_revision, target_lease_digest, target_lease_json,
                previous_status, target_status, fencing_generation,
                previous_expires_at, target_expires_at, hard_deadline_at,
                executor_heartbeat_ref, request_digest, event_digest,
                idempotency_scope, idempotency_key,
                renewed_by_user_id, renewed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?22)",
            params![
                renewal_id,
                input.lease_id,
                input.provider_id,
                current.consumer_account_id,
                current.lease_revision,
                current.lease_digest,
                target_revision,
                target_digest,
                serde_json::to_string(&target_lease)?,
                current.lease.status,
                target_lease.status,
                target_lease.fencing_generation,
                current.lease.expires_at,
                target_lease.expires_at,
                target_lease.hard_deadline_at,
                input.executor_heartbeat_ref,
                request_digest,
                event_digest,
                idempotency_scope,
                input.idempotency_key,
                input.renewed_by_user_id,
                renewed_at,
            ],
        )?;
        let stored = renewal_by_idempotency_on(&tx, &idempotency_scope, &input.idempotency_key)?
            .ok_or_else(|| anyhow!("Attempt Lease 续租回执写入后不可见"))?;
        audit_renewal(&stored)?;
        tx.commit()?;
        stored.into_receipt(false)
    }

    pub(crate) fn compute_attempt_lease_state(
        &self,
        lease_id: &str,
    ) -> Result<ComputeAttemptLeaseStateReceipt> {
        validate_exact("Attempt Lease ID", lease_id, 200)?;
        compute_attempt_lease_state_on(&*self.conn()?, lease_id)
    }
}

pub(super) struct TerminateStagingAttemptLease<'a> {
    pub lease_id: &'a str,
    pub expected_revision: i64,
    pub expected_digest: &'a str,
    pub expected_fencing_generation: i64,
    pub reason_code: &'a str,
    pub actor_user_id: &'a str,
    pub terminated_at: &'a str,
}

pub(super) fn compute_attempt_lease_state_on(
    conn: &Connection,
    lease_id: &str,
) -> Result<ComputeAttemptLeaseStateReceipt> {
    validate_exact("Attempt Lease ID", lease_id, 200)?;
    current_lease_state_on(conn, lease_id)?
        .map(StoredLeaseState::into_receipt)
        .transpose()?
        .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))
}

pub(super) fn terminate_staging_attempt_lease_on(
    conn: &Connection,
    input: TerminateStagingAttemptLease<'_>,
) -> Result<ComputeAttemptLeaseStateReceipt> {
    validate_exact("Attempt Lease ID", input.lease_id, 200)?;
    validate_exact("Attempt 中止原因码", input.reason_code, 160)?;
    validate_exact("Attempt 中止执行人", input.actor_user_id, 160)?;
    let current = current_lease_state_on(conn, input.lease_id)?
        .ok_or_else(|| anyhow!("Attempt Lease 当前状态不存在"))?;
    if current.lease_revision != input.expected_revision
        || current.lease_digest != input.expected_digest
        || current.lease.fencing_generation != input.expected_fencing_generation
        || current.lease.status != ATTEMPT_STATUS_STAGING
        || current.lease.last_heartbeat_at.is_some()
    {
        bail!("只有当前精确版本且从未记录心跳的 staging Lease 可以无用量中止");
    }
    let mut terminal = current.lease.clone();
    terminal.status = ATTEMPT_STATUS_TERMINAL.to_string();
    terminal.terminal_reason_code = Some(input.reason_code.to_string());
    let target_revision = current
        .lease_revision
        .checked_add(1)
        .context("Attempt Lease 修订号溢出")?;
    let target_digest = compute_attempt_lease_digest(&terminal)?;
    let changed = conn.execute(
        "UPDATE compute_attempt_lease_states
            SET lease_revision=?1, lease_digest=?2, lease_json=?3,
                status=?4, updated_by_user_id=?5, updated_at=?6
          WHERE lease_id=?7 AND lease_revision=?8 AND lease_digest=?9
            AND status='staging' AND last_heartbeat_at IS NULL",
        params![
            target_revision,
            target_digest,
            serde_json::to_string(&terminal)?,
            terminal.status,
            input.actor_user_id,
            input.terminated_at,
            input.lease_id,
            current.lease_revision,
            current.lease_digest,
        ],
    )?;
    if changed != 1 {
        bail!("Attempt Lease 已被并发修改，请重新读取当前状态");
    }
    compute_attempt_lease_state_on(conn, input.lease_id)
}

pub(super) fn initialize_compute_attempt_lease_state_on(
    conn: &Connection,
    consumer_account_id: &str,
    lease: &ComputeAttemptLease,
    lease_digest: &str,
    actor_user_id: &str,
    activated_at: &str,
) -> Result<()> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO compute_attempt_lease_states (
            lease_id, provider_id, consumer_account_id, lease_revision,
            lease_digest, lease_json, status, fencing_generation,
            expires_at, hard_deadline_at, last_heartbeat_at,
            updated_by_user_id, updated_at
         ) VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            lease.lease_id,
            lease.provider_id,
            consumer_account_id,
            lease_digest,
            serde_json::to_string(lease)?,
            lease.status,
            lease.fencing_generation,
            lease.expires_at,
            lease.hard_deadline_at,
            lease.last_heartbeat_at,
            actor_user_id,
            activated_at,
        ],
    )?;
    if inserted == 0 {
        let existing = current_lease_state_on(conn, &lease.lease_id)?
            .ok_or_else(|| anyhow!("Attempt Lease 状态初始化冲突"))?;
        if existing.lease_revision != 1 || existing.lease_digest != lease_digest {
            bail!("Attempt Lease 状态已绑定不同激活合同");
        }
    }
    Ok(())
}
