use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{capacity::ComputeCapacityPoolStatus, provider::PROVIDER_STATUS_ACTIVE},
    compute_federation_activation_application_model::{
        ComputeActivationApplicationReceipt, COMPUTE_ACTIVATION_APPLICATION_SCHEMA,
    },
    compute_federation_activation_model::{
        ACTIVATION_REQUEST_STATUS_ACTIVATED, ACTIVATION_REQUEST_STATUS_APPROVED,
    },
    compute_federation_activation_plan_model::{
        ACTIVATION_PLAN_STATUS_APPLIED, ACTIVATION_PLAN_STATUS_PREPARED,
    },
};

use super::{
    compute_activation_plan_dependencies::validate_saved_plan_dependencies_on,
    compute_activation_plan_reviews::require_activation_plan_review_on,
    compute_activation_plans::plan_by_request_on,
    compute_activation_requests::request_on,
    compute_capacity_pool_lifecycle::{
        transition_compute_capacity_pool_status_on, TransitionComputeCapacityPoolStatus,
    },
    compute_provider_registry::{register_compute_provider_on, registered_provider_version_on},
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct ApplyComputeActivationPlan {
    pub request_id: String,
    pub expected_plan_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub applied_by_user_id: String,
}

impl Store {
    pub(crate) fn apply_compute_activation_plan(
        &self,
        input: ApplyComputeActivationPlan,
    ) -> Result<ComputeActivationApplicationReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = application_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let plan = plan_by_request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活计划不存在"))?;
        if let Some(existing) = application_by_plan_on(&tx, &plan.plan_id)? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        if plan.status != ACTIVATION_PLAN_STATUS_PREPARED
            || plan.plan_digest != input.expected_plan_digest
        {
            bail!("只有当前摘要匹配的 prepared 激活计划可以应用");
        }
        require_activation_plan_review_on(&tx, &plan)?;
        validate_saved_plan_dependencies_on(&tx, &plan)?;

        let provider = register_compute_provider_on(&tx, &plan.target_provider)?;
        if provider.replayed || provider.provider_digest != plan.target_provider_digest {
            bail!("目标 Provider 版本未按激活计划创建");
        }

        let applied_at = now();
        let pool_event = transition_compute_capacity_pool_status_on(
            &tx,
            &TransitionComputeCapacityPoolStatus {
                pool_id: plan.pool_id.clone(),
                expected_capacity_epoch: plan.expected_capacity_epoch,
                expected_status: ComputeCapacityPoolStatus::Registering,
                target_status: ComputeCapacityPoolStatus::Active,
                reason_code: "activation_plan_applied".to_string(),
                subject_kind: "compute_activation_plan".to_string(),
                subject_id: plan.plan_id.clone(),
                idempotency_scope: format!("compute_activation_plan:{}", plan.plan_id),
                idempotency_key: "pool_active".to_string(),
                request_digest: plan.plan_digest.clone(),
                occurred_at: applied_at.clone(),
            },
        )?;
        if pool_event.replayed || pool_event.current_status != "active" {
            bail!("CapacityPool 未按激活计划转换为 active");
        }

        let request_changed = tx.execute(
            "UPDATE compute_activation_evidence_requests
                SET status='activated', updated_at=?1
              WHERE request_id=?2 AND status='approved' AND request_digest=?3",
            params![applied_at, plan.request_id, plan.expected_request_digest,],
        )?;
        let plan_changed = tx.execute(
            "UPDATE compute_activation_plans
                SET status='applied', applied_at=?1, updated_at=?1
              WHERE plan_id=?2 AND status='prepared' AND plan_digest=?3",
            params![applied_at, plan.plan_id, plan.plan_digest],
        )?;
        if request_changed != 1 || plan_changed != 1 {
            bail!("激活申请或计划状态发生并发变化");
        }

        let application_id = new_id("compute_activation_application");
        let application_digest = application_digest(
            &application_id,
            &plan.plan_id,
            &plan.request_id,
            &plan.provider_id,
            &plan.pool_id,
            &plan.plan_digest,
            plan.target_provider_policy_revision,
            &plan.target_provider_digest,
            &pool_event.event_id,
            input.applied_by_user_id.trim(),
            &applied_at,
        )?;
        tx.execute(
            "INSERT INTO compute_activation_applications (
                application_id, plan_id, request_id, provider_id, pool_id,
                plan_digest, target_provider_policy_revision,
                target_provider_digest, pool_lifecycle_event_id,
                application_digest, idempotency_scope, idempotency_key,
                applied_by_user_id, applied_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                application_id,
                plan.plan_id,
                plan.request_id,
                plan.provider_id,
                plan.pool_id,
                plan.plan_digest,
                plan.target_provider_policy_revision,
                plan.target_provider_digest,
                pool_event.event_id,
                application_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.applied_by_user_id.trim(),
                applied_at,
            ],
        )?;
        let stored = application_by_plan_on(&tx, &plan.plan_id)?
            .ok_or_else(|| anyhow!("激活应用回执写入后无法读取"))?;
        audit_applied_state_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_activation_application_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationApplicationReceipt>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        application_by_request_on(&conn, request_id.trim())?
            .map(|stored| {
                audit_applied_state_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }
}

#[derive(Debug, Clone)]
pub(super) struct StoredApplication {
    pub(super) application_id: String,
    plan_id: String,
    pub(super) request_id: String,
    pub(super) provider_id: String,
    pub(super) pool_id: String,
    plan_digest: String,
    target_provider_policy_revision: i64,
    target_provider_digest: String,
    pool_lifecycle_event_id: String,
    pub(super) application_digest: String,
    applied_by_user_id: String,
    applied_at: String,
}

impl StoredApplication {
    fn into_receipt(self, replayed: bool) -> ComputeActivationApplicationReceipt {
        ComputeActivationApplicationReceipt {
            schema: COMPUTE_ACTIVATION_APPLICATION_SCHEMA,
            application_id: self.application_id,
            plan_id: self.plan_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            plan_digest: self.plan_digest,
            target_provider_policy_revision: self.target_provider_policy_revision,
            target_provider_digest: self.target_provider_digest,
            pool_lifecycle_event_id: self.pool_lifecycle_event_id,
            application_digest: self.application_digest,
            applied_by_user_id: self.applied_by_user_id,
            applied_at: self.applied_at,
            replayed,
            activation_effect: "provider_and_pool_active",
            offer_effect: "none",
        }
    }
}

fn validate_replay(
    conn: &Connection,
    input: &ApplyComputeActivationPlan,
    existing: &StoredApplication,
) -> Result<()> {
    if existing.request_id != input.request_id.trim()
        || existing.plan_digest != input.expected_plan_digest.trim()
    {
        bail!("相同激活应用幂等键或申请已绑定不同计划");
    }
    audit_applied_state_on(conn, existing)
}

pub(super) fn audit_applied_state_on(conn: &Connection, stored: &StoredApplication) -> Result<()> {
    let plan = plan_by_request_on(conn, &stored.request_id)?
        .ok_or_else(|| anyhow!("激活应用回执引用的计划不存在"))?;
    let request = request_on(conn, &stored.request_id)?
        .ok_or_else(|| anyhow!("激活应用回执引用的申请不存在"))?;
    let provider_version = registered_provider_version_on(
        conn,
        &stored.provider_id,
        stored.target_provider_policy_revision,
    )?
    .ok_or_else(|| anyhow!("激活应用回执引用的 Provider 历史版本不存在"))?;
    let pool_event_matches = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_pool_lifecycle_events
              WHERE event_id=?1 AND pool_id=?2 AND target_status='active'
                AND subject_kind='compute_activation_plan' AND subject_id=?3
                AND request_digest=?4 AND capacity_epoch=?5
                AND previous_status='registering' AND occurred_at=?6",
            params![
                stored.pool_lifecycle_event_id,
                stored.pool_id,
                stored.plan_id,
                stored.plan_digest,
                plan.expected_capacity_epoch,
                stored.applied_at,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    let expected_digest = application_digest(
        &stored.application_id,
        &stored.plan_id,
        &stored.request_id,
        &stored.provider_id,
        &stored.pool_id,
        &stored.plan_digest,
        stored.target_provider_policy_revision,
        &stored.target_provider_digest,
        &stored.pool_lifecycle_event_id,
        &stored.applied_by_user_id,
        &stored.applied_at,
    )?;
    if plan.status != ACTIVATION_PLAN_STATUS_APPLIED
        || plan.plan_id != stored.plan_id
        || plan.plan_digest != stored.plan_digest
        || request.status != ACTIVATION_REQUEST_STATUS_ACTIVATED
        || provider_version.provider.policy_revision != stored.target_provider_policy_revision
        || provider_version.provider_digest != stored.target_provider_digest
        || provider_version.provider.status != PROVIDER_STATUS_ACTIVE
        || !pool_event_matches
        || expected_digest != stored.application_digest
    {
        bail!("激活应用回执与当前 Provider、Pool、申请或计划状态不一致");
    }
    Ok(())
}

fn application_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredApplication>> {
    application_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn application_by_plan_on(conn: &Connection, plan_id: &str) -> Result<Option<StoredApplication>> {
    application_on(conn, "WHERE plan_id=?1", params![plan_id])
}

pub(super) fn application_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredApplication>> {
    application_on(conn, "WHERE request_id=?1", params![request_id])
}

fn application_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredApplication>> {
    conn.query_row(
        &format!(
            "SELECT application_id, plan_id, request_id, provider_id, pool_id,
                    plan_digest, target_provider_policy_revision,
                    target_provider_digest, pool_lifecycle_event_id,
                    application_digest, applied_by_user_id, applied_at
               FROM compute_activation_applications {filter}"
        ),
        parameters,
        stored_application_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_application_from_row(row: &Row<'_>) -> rusqlite::Result<StoredApplication> {
    Ok(StoredApplication {
        application_id: row.get(0)?,
        plan_id: row.get(1)?,
        request_id: row.get(2)?,
        provider_id: row.get(3)?,
        pool_id: row.get(4)?,
        plan_digest: row.get(5)?,
        target_provider_policy_revision: row.get(6)?,
        target_provider_digest: row.get(7)?,
        pool_lifecycle_event_id: row.get(8)?,
        application_digest: row.get(9)?,
        applied_by_user_id: row.get(10)?,
        applied_at: row.get(11)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn application_digest(
    application_id: &str,
    plan_id: &str,
    request_id: &str,
    provider_id: &str,
    pool_id: &str,
    plan_digest: &str,
    target_provider_policy_revision: i64,
    target_provider_digest: &str,
    pool_lifecycle_event_id: &str,
    applied_by_user_id: &str,
    applied_at: &str,
) -> Result<String> {
    let value = serde_json::json!({
        "schema":COMPUTE_ACTIVATION_APPLICATION_SCHEMA,
        "application_id":application_id,
        "plan_id":plan_id,
        "request_id":request_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
        "plan_digest":plan_digest,
        "target_provider_policy_revision":target_provider_policy_revision,
        "target_provider_digest":target_provider_digest,
        "pool_lifecycle_event_id":pool_lifecycle_event_id,
        "applied_by_user_id":applied_by_user_id,
        "applied_at":applied_at,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn validate_input(input: &ApplyComputeActivationPlan) -> Result<()> {
    for (label, value, max_len) in [
        ("激活证据申请 ID", input.request_id.as_str(), 160),
        ("激活应用幂等范围", input.idempotency_scope.as_str(), 200),
        ("激活应用幂等键", input.idempotency_key.as_str(), 160),
        ("激活应用执行人", input.applied_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    let digest = input.expected_plan_digest.as_str();
    if digest.len() != 64
        || digest != digest.to_ascii_lowercase()
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("激活计划摘要必须是 64 位小写十六进制 SHA-256");
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
