use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_QUARANTINED},
    },
    compute_federation_activation_quarantine_model::{
        ComputeActivationQuarantineReceipt, COMPUTE_ACTIVATION_QUARANTINE_SCHEMA,
    },
};

use super::{
    compute_activation_applications::{
        application_by_request_on, audit_applied_state_on, StoredApplication,
    },
    compute_capacity_pool_lifecycle::{
        transition_compute_capacity_pool_status_on, TransitionComputeCapacityPoolStatus,
    },
    compute_capacity_pool_queries::current_capacity_pool_on,
    compute_provider_registry::{
        current_registered_provider_on, register_compute_provider_on,
        registered_provider_version_on,
    },
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct QuarantineComputeActivationApplication {
    pub request_id: String,
    pub expected_application_digest: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub quarantined_by_user_id: String,
}

impl Store {
    pub(crate) fn quarantine_compute_activation_application(
        &self,
        input: QuarantineComputeActivationApplication,
    ) -> Result<ComputeActivationQuarantineReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = quarantine_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let application = application_by_request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活应用回执不存在"))?;
        if let Some(existing) = quarantine_by_application_on(&tx, &application.application_id)? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        audit_applied_state_on(&tx, &application)?;
        if application.application_digest != input.expected_application_digest {
            bail!("激活应用摘要已变化");
        }

        let provider = current_registered_provider_on(&tx, &application.provider_id)?
            .ok_or_else(|| anyhow!("激活应用引用的当前 Provider 不存在"))?;
        let pool = current_capacity_pool_on(&tx, &application.pool_id)?
            .ok_or_else(|| anyhow!("激活应用引用的当前 CapacityPool 不存在"))?;
        if provider.provider.status != PROVIDER_STATUS_ACTIVE
            || pool.status != ComputeCapacityPoolStatus::Active
            || pool.provider_id != application.provider_id
        {
            bail!("只有当前 Provider 和 CapacityPool 均为 active 的激活结果可以隔离");
        }

        let quarantined_at = now();
        let previous_provider_policy_revision = provider.provider.policy_revision;
        let previous_provider_digest = provider.provider_digest.clone();
        let mut quarantined_provider = provider.provider;
        quarantined_provider.policy_revision = quarantined_provider
            .policy_revision
            .checked_add(1)
            .ok_or_else(|| anyhow!("Provider policy revision 溢出"))?;
        quarantined_provider.status = PROVIDER_STATUS_QUARANTINED.to_string();
        quarantined_provider.updated_at = quarantined_at.clone();
        let provider_receipt = register_compute_provider_on(&tx, &quarantined_provider)?;
        if provider_receipt.replayed {
            bail!("Provider 隔离版本发生非预期重放");
        }

        let pool_event = transition_compute_capacity_pool_status_on(
            &tx,
            &TransitionComputeCapacityPoolStatus {
                pool_id: application.pool_id.clone(),
                expected_capacity_epoch: pool.binding.capacity_epoch,
                expected_status: ComputeCapacityPoolStatus::Active,
                target_status: ComputeCapacityPoolStatus::Quarantined,
                reason_code: "activation_application_quarantined".to_string(),
                subject_kind: "compute_activation_application".to_string(),
                subject_id: application.application_id.clone(),
                idempotency_scope: format!(
                    "compute_activation_quarantine:{}",
                    application.application_id
                ),
                idempotency_key: "pool_quarantined".to_string(),
                request_digest: application.application_digest.clone(),
                occurred_at: quarantined_at.clone(),
            },
        )?;
        if pool_event.replayed || pool_event.current_status != "quarantined" {
            bail!("CapacityPool 未按隔离请求转换为 quarantined");
        }

        let quarantine_id = new_id("compute_activation_quarantine");
        let quarantine_digest = quarantine_digest(
            &quarantine_id,
            &application,
            previous_provider_policy_revision,
            &previous_provider_digest,
            quarantined_provider.policy_revision,
            &provider_receipt.provider_digest,
            pool.binding.capacity_epoch,
            &pool_event.event_id,
            input.reason.trim(),
            input.quarantined_by_user_id.trim(),
            &quarantined_at,
        )?;
        tx.execute(
            "INSERT INTO compute_activation_quarantines (
                quarantine_id, application_id, request_id, provider_id, pool_id,
                application_digest, previous_provider_policy_revision,
                previous_provider_digest, quarantined_provider_policy_revision,
                quarantined_provider_digest, capacity_epoch, pool_lifecycle_event_id,
                reason, quarantine_digest, idempotency_scope, idempotency_key,
                quarantined_by_user_id, quarantined_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?18)",
            params![
                quarantine_id,
                application.application_id,
                application.request_id,
                application.provider_id,
                application.pool_id,
                application.application_digest,
                previous_provider_policy_revision,
                previous_provider_digest,
                quarantined_provider.policy_revision,
                provider_receipt.provider_digest,
                pool.binding.capacity_epoch,
                pool_event.event_id,
                input.reason.trim(),
                quarantine_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.quarantined_by_user_id.trim(),
                quarantined_at,
            ],
        )?;
        let stored = quarantine_by_application_on(&tx, &application.application_id)?
            .ok_or_else(|| anyhow!("隔离回执写入后无法读取"))?;
        audit_quarantine_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_activation_quarantine_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationQuarantineReceipt>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        quarantine_by_request_on(&conn, request_id.trim())?
            .map(|stored| {
                audit_quarantine_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }
}

#[derive(Debug, Clone)]
pub(super) struct StoredQuarantine {
    pub(super) quarantine_id: String,
    pub(super) application_id: String,
    pub(super) request_id: String,
    pub(super) provider_id: String,
    pub(super) pool_id: String,
    application_digest: String,
    previous_provider_policy_revision: i64,
    previous_provider_digest: String,
    pub(super) quarantined_provider_policy_revision: i64,
    pub(super) quarantined_provider_digest: String,
    pub(super) capacity_epoch: i64,
    pool_lifecycle_event_id: String,
    reason: String,
    pub(super) quarantine_digest: String,
    quarantined_by_user_id: String,
    quarantined_at: String,
}

impl StoredQuarantine {
    fn into_receipt(self, replayed: bool) -> ComputeActivationQuarantineReceipt {
        ComputeActivationQuarantineReceipt {
            schema: COMPUTE_ACTIVATION_QUARANTINE_SCHEMA,
            quarantine_id: self.quarantine_id,
            application_id: self.application_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            application_digest: self.application_digest,
            previous_provider_policy_revision: self.previous_provider_policy_revision,
            previous_provider_digest: self.previous_provider_digest,
            quarantined_provider_policy_revision: self.quarantined_provider_policy_revision,
            quarantined_provider_digest: self.quarantined_provider_digest,
            capacity_epoch: self.capacity_epoch,
            pool_lifecycle_event_id: self.pool_lifecycle_event_id,
            reason: self.reason,
            quarantine_digest: self.quarantine_digest,
            quarantined_by_user_id: self.quarantined_by_user_id,
            quarantined_at: self.quarantined_at,
            replayed,
            provider_effect: "quarantined",
            pool_effect: "quarantined",
            offer_effect: "none_direct",
        }
    }
}

fn validate_replay(
    conn: &Connection,
    input: &QuarantineComputeActivationApplication,
    existing: &StoredQuarantine,
) -> Result<()> {
    if existing.request_id != input.request_id.trim()
        || existing.application_digest != input.expected_application_digest.trim()
        || existing.reason != input.reason.trim()
    {
        bail!("相同隔离幂等键或激活应用已绑定不同请求");
    }
    audit_quarantine_on(conn, existing)
}

pub(super) fn audit_quarantine_on(conn: &Connection, stored: &StoredQuarantine) -> Result<()> {
    let application = application_by_request_on(conn, &stored.request_id)?
        .ok_or_else(|| anyhow!("隔离回执引用的激活应用不存在"))?;
    audit_applied_state_on(conn, &application)?;
    let previous_provider = registered_provider_version_on(
        conn,
        &stored.provider_id,
        stored.previous_provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("隔离前 Provider 历史版本不存在"))?;
    let quarantined_provider = registered_provider_version_on(
        conn,
        &stored.provider_id,
        stored.quarantined_provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("隔离后 Provider 历史版本不存在"))?;
    let pool_event_matches = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_pool_lifecycle_events
              WHERE event_id=?1 AND pool_id=?2 AND capacity_epoch=?3
                AND previous_status='active' AND target_status='quarantined'
                AND subject_kind='compute_activation_application' AND subject_id=?4
                AND request_digest=?5 AND occurred_at=?6",
            params![
                stored.pool_lifecycle_event_id,
                stored.pool_id,
                stored.capacity_epoch,
                stored.application_id,
                stored.application_digest,
                stored.quarantined_at,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    let expected_digest = quarantine_digest(
        &stored.quarantine_id,
        &application,
        stored.previous_provider_policy_revision,
        &stored.previous_provider_digest,
        stored.quarantined_provider_policy_revision,
        &stored.quarantined_provider_digest,
        stored.capacity_epoch,
        &stored.pool_lifecycle_event_id,
        &stored.reason,
        &stored.quarantined_by_user_id,
        &stored.quarantined_at,
    )?;
    let expected_next_revision = stored.previous_provider_policy_revision.checked_add(1);
    if application.application_id != stored.application_id
        || application.provider_id != stored.provider_id
        || application.pool_id != stored.pool_id
        || application.application_digest != stored.application_digest
        || previous_provider.provider_digest != stored.previous_provider_digest
        || previous_provider.provider.status != PROVIDER_STATUS_ACTIVE
        || expected_next_revision != Some(stored.quarantined_provider_policy_revision)
        || quarantined_provider.provider_digest != stored.quarantined_provider_digest
        || quarantined_provider.provider.status != PROVIDER_STATUS_QUARANTINED
        || !pool_event_matches
        || expected_digest != stored.quarantine_digest
    {
        bail!("激活隔离回执与历史 Provider、Pool 或应用状态不一致");
    }
    Ok(())
}

fn quarantine_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredQuarantine>> {
    quarantine_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn quarantine_by_application_on(
    conn: &Connection,
    application_id: &str,
) -> Result<Option<StoredQuarantine>> {
    quarantine_on(conn, "WHERE application_id=?1", params![application_id])
}

pub(super) fn quarantine_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredQuarantine>> {
    quarantine_on(conn, "WHERE request_id=?1", params![request_id])
}

fn quarantine_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredQuarantine>> {
    conn.query_row(
        &format!(
            "SELECT quarantine_id, application_id, request_id, provider_id, pool_id,
                    application_digest, previous_provider_policy_revision,
                    previous_provider_digest, quarantined_provider_policy_revision,
                    quarantined_provider_digest, capacity_epoch, pool_lifecycle_event_id,
                    reason, quarantine_digest, quarantined_by_user_id, quarantined_at
               FROM compute_activation_quarantines {filter}"
        ),
        parameters,
        stored_quarantine_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_quarantine_from_row(row: &Row<'_>) -> rusqlite::Result<StoredQuarantine> {
    Ok(StoredQuarantine {
        quarantine_id: row.get(0)?,
        application_id: row.get(1)?,
        request_id: row.get(2)?,
        provider_id: row.get(3)?,
        pool_id: row.get(4)?,
        application_digest: row.get(5)?,
        previous_provider_policy_revision: row.get(6)?,
        previous_provider_digest: row.get(7)?,
        quarantined_provider_policy_revision: row.get(8)?,
        quarantined_provider_digest: row.get(9)?,
        capacity_epoch: row.get(10)?,
        pool_lifecycle_event_id: row.get(11)?,
        reason: row.get(12)?,
        quarantine_digest: row.get(13)?,
        quarantined_by_user_id: row.get(14)?,
        quarantined_at: row.get(15)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn quarantine_digest(
    quarantine_id: &str,
    application: &StoredApplication,
    previous_provider_policy_revision: i64,
    previous_provider_digest: &str,
    quarantined_provider_policy_revision: i64,
    quarantined_provider_digest: &str,
    capacity_epoch: i64,
    pool_lifecycle_event_id: &str,
    reason: &str,
    quarantined_by_user_id: &str,
    quarantined_at: &str,
) -> Result<String> {
    let value = serde_json::json!({
        "schema":COMPUTE_ACTIVATION_QUARANTINE_SCHEMA,
        "quarantine_id":quarantine_id,
        "application_id":application.application_id,
        "request_id":application.request_id,
        "provider_id":application.provider_id,
        "pool_id":application.pool_id,
        "application_digest":application.application_digest,
        "previous_provider_policy_revision":previous_provider_policy_revision,
        "previous_provider_digest":previous_provider_digest,
        "quarantined_provider_policy_revision":quarantined_provider_policy_revision,
        "quarantined_provider_digest":quarantined_provider_digest,
        "capacity_epoch":capacity_epoch,
        "pool_lifecycle_event_id":pool_lifecycle_event_id,
        "reason":reason,
        "quarantined_by_user_id":quarantined_by_user_id,
        "quarantined_at":quarantined_at,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn validate_input(input: &QuarantineComputeActivationApplication) -> Result<()> {
    for (label, value, max_len) in [
        ("激活证据申请 ID", input.request_id.as_str(), 160),
        ("激活隔离原因", input.reason.as_str(), 1000),
        ("激活隔离幂等范围", input.idempotency_scope.as_str(), 200),
        ("激活隔离幂等键", input.idempotency_key.as_str(), 160),
        ("激活隔离执行人", input.quarantined_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_digest("激活应用摘要", &input.expected_application_digest)
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

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
