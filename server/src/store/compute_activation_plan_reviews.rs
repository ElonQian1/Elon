use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation_activation_plan_model::{
    ComputeActivationPlan, ACTIVATION_PLAN_STATUS_PREPARED,
};

use super::{compute_activation_plans::plan_by_request_on, new_id, now, Store};

pub(crate) const COMPUTE_ACTIVATION_PLAN_REVIEW_SCHEMA: &str =
    "compute_federation.activation_plan_review.v1";

#[derive(Debug, Clone)]
pub(crate) struct ReviewComputeActivationPlan {
    pub request_id: String,
    pub expected_plan_digest: String,
    pub review_note: Option<String>,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub reviewed_by_user_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeActivationPlanReviewReceipt {
    pub schema: &'static str,
    pub review_id: String,
    pub plan_id: String,
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub plan_digest: String,
    pub prepared_by_user_id: String,
    pub reviewed_by_user_id: String,
    pub review_note: Option<String>,
    pub request_digest: String,
    pub review_digest: String,
    pub reviewed_at: String,
    pub replayed: bool,
    pub activation_effect: &'static str,
}

impl Store {
    pub(crate) fn review_compute_activation_plan(
        &self,
        input: ReviewComputeActivationPlan,
    ) -> Result<ComputeActivationPlanReviewReceipt> {
        let input = normalize_input(input)?;
        let request_digest = review_request_digest(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) =
            review_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            if stored.request_digest != request_digest {
                bail!("相同激活计划复核幂等键不能用于不同请求");
            }
            let receipt = review_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let plan =
            plan_by_request_on(&tx, &input.request_id)?.ok_or_else(|| anyhow!("激活计划不存在"))?;
        validate_review_target(&plan, &input)?;

        if let Some(stored) = review_by_plan_on(&tx, &plan.plan_id)? {
            if stored.request_digest != request_digest {
                bail!("同一激活计划已由另一份复核回执锁定");
            }
            let receipt = review_receipt_on(&tx, stored, true)?;
            tx.commit()?;
            return Ok(receipt);
        }

        let review_id = new_id("compute_activation_plan_review");
        let reviewed_at = now();
        let review_digest =
            review_event_digest(&review_id, &plan, &input, &request_digest, &reviewed_at)?;
        tx.execute(
            "INSERT INTO compute_activation_plan_reviews (
                review_id, plan_id, request_id, provider_id, pool_id,
                plan_digest, prepared_by_user_id, reviewed_by_user_id,
                review_note, request_digest, review_digest,
                idempotency_scope, idempotency_key, reviewed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
            params![
                review_id,
                plan.plan_id,
                plan.request_id,
                plan.provider_id,
                plan.pool_id,
                plan.plan_digest,
                plan.prepared_by_user_id,
                input.reviewed_by_user_id,
                input.review_note,
                request_digest,
                review_digest,
                input.idempotency_scope,
                input.idempotency_key,
                reviewed_at,
            ],
        )?;
        let stored = review_by_plan_on(&tx, &plan.plan_id)?
            .ok_or_else(|| anyhow!("激活计划复核回执写入后无法读取"))?;
        let receipt = review_receipt_on(&tx, stored, false)?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn compute_activation_plan_review_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationPlanReviewReceipt>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        let Some(stored) = review_by_request_on(&conn, request_id.trim())? else {
            return Ok(None);
        };
        Ok(Some(review_receipt_on(&conn, stored, false)?))
    }
}

#[derive(Debug, Clone)]
struct StoredActivationPlanReview {
    review_id: String,
    plan_id: String,
    request_id: String,
    provider_id: String,
    pool_id: String,
    plan_digest: String,
    prepared_by_user_id: String,
    reviewed_by_user_id: String,
    review_note: Option<String>,
    request_digest: String,
    review_digest: String,
    reviewed_at: String,
}

impl StoredActivationPlanReview {
    fn into_receipt(self, replayed: bool) -> ComputeActivationPlanReviewReceipt {
        ComputeActivationPlanReviewReceipt {
            schema: COMPUTE_ACTIVATION_PLAN_REVIEW_SCHEMA,
            review_id: self.review_id,
            plan_id: self.plan_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            plan_digest: self.plan_digest,
            prepared_by_user_id: self.prepared_by_user_id,
            reviewed_by_user_id: self.reviewed_by_user_id,
            review_note: self.review_note,
            request_digest: self.request_digest,
            review_digest: self.review_digest,
            reviewed_at: self.reviewed_at,
            replayed,
            activation_effect: "none",
        }
    }
}

pub(super) fn require_activation_plan_review_on(
    conn: &Connection,
    plan: &ComputeActivationPlan,
) -> Result<()> {
    let stored = review_by_plan_on(conn, &plan.plan_id)?
        .ok_or_else(|| anyhow!("激活计划尚未完成第二人复核"))?;
    audit_review_on(plan, &stored)
}

fn review_receipt_on(
    conn: &Connection,
    stored: StoredActivationPlanReview,
    replayed: bool,
) -> Result<ComputeActivationPlanReviewReceipt> {
    let plan = plan_by_request_on(conn, &stored.request_id)?
        .ok_or_else(|| anyhow!("激活计划复核回执引用的计划不存在"))?;
    audit_review_on(&plan, &stored)?;
    Ok(stored.into_receipt(replayed))
}

fn audit_review_on(
    plan: &ComputeActivationPlan,
    stored: &StoredActivationPlanReview,
) -> Result<()> {
    let input = ReviewComputeActivationPlan {
        request_id: stored.request_id.clone(),
        expected_plan_digest: stored.plan_digest.clone(),
        review_note: stored.review_note.clone(),
        idempotency_scope: String::new(),
        idempotency_key: String::new(),
        reviewed_by_user_id: stored.reviewed_by_user_id.clone(),
    };
    let expected_request_digest = review_request_digest(&input)?;
    let expected_review_digest = review_event_digest(
        &stored.review_id,
        plan,
        &input,
        &stored.request_digest,
        &stored.reviewed_at,
    )?;
    if stored.plan_id != plan.plan_id
        || stored.request_id != plan.request_id
        || stored.provider_id != plan.provider_id
        || stored.pool_id != plan.pool_id
        || stored.plan_digest != plan.plan_digest
        || stored.prepared_by_user_id != plan.prepared_by_user_id
        || stored.reviewed_by_user_id == plan.prepared_by_user_id
        || stored.request_digest != expected_request_digest
        || stored.review_digest != expected_review_digest
    {
        bail!("激活计划复核回执与不可变计划或复核参与者不一致");
    }
    Ok(())
}

fn validate_review_target(
    plan: &ComputeActivationPlan,
    input: &ReviewComputeActivationPlan,
) -> Result<()> {
    if plan.status != ACTIVATION_PLAN_STATUS_PREPARED
        || plan.plan_digest != input.expected_plan_digest
    {
        bail!("只有当前摘要匹配的 prepared 激活计划可以复核");
    }
    if plan.prepared_by_user_id == input.reviewed_by_user_id {
        bail!("激活计划准备人不能复核自己准备的计划");
    }
    Ok(())
}

fn normalize_input(mut input: ReviewComputeActivationPlan) -> Result<ReviewComputeActivationPlan> {
    validate_exact("激活证据申请 ID", &input.request_id, 160)?;
    validate_digest("激活计划摘要", &input.expected_plan_digest)?;
    validate_exact("激活计划复核幂等范围", &input.idempotency_scope, 200)?;
    validate_exact("激活计划复核幂等键", &input.idempotency_key, 160)?;
    validate_exact("激活计划复核人", &input.reviewed_by_user_id, 160)?;
    input.review_note = normalize_note(input.review_note)?;
    Ok(input)
}

fn normalize_note(note: Option<String>) -> Result<Option<String>> {
    let Some(note) = note else { return Ok(None) };
    let normalized = note.trim().to_string();
    if normalized.is_empty() || normalized.chars().count() > 1000 {
        bail!("激活计划复核说明为空或超过 1000 字");
    }
    if normalized
        .chars()
        .any(|value| value.is_control() && value != '\n' && value != '\t')
    {
        bail!("激活计划复核说明包含无效控制字符");
    }
    Ok(Some(normalized))
}

fn review_request_digest(input: &ReviewComputeActivationPlan) -> Result<String> {
    let value = serde_json::json!({
        "schema":"compute_federation.activation_plan_review_request.v1",
        "request_id":input.request_id,
        "expected_plan_digest":input.expected_plan_digest,
        "review_note":input.review_note,
        "reviewed_by_user_id":input.reviewed_by_user_id,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn review_event_digest(
    review_id: &str,
    plan: &ComputeActivationPlan,
    input: &ReviewComputeActivationPlan,
    request_digest: &str,
    reviewed_at: &str,
) -> Result<String> {
    let value = serde_json::json!({
        "schema":COMPUTE_ACTIVATION_PLAN_REVIEW_SCHEMA,
        "review_id":review_id,
        "plan_id":plan.plan_id,
        "request_id":plan.request_id,
        "provider_id":plan.provider_id,
        "pool_id":plan.pool_id,
        "plan_digest":plan.plan_digest,
        "prepared_by_user_id":plan.prepared_by_user_id,
        "reviewed_by_user_id":input.reviewed_by_user_id,
        "review_note":input.review_note,
        "request_digest":request_digest,
        "reviewed_at":reviewed_at,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn review_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredActivationPlanReview>> {
    review_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn review_by_plan_on(
    conn: &Connection,
    plan_id: &str,
) -> Result<Option<StoredActivationPlanReview>> {
    review_on(conn, "WHERE plan_id=?1", params![plan_id])
}

fn review_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredActivationPlanReview>> {
    review_on(conn, "WHERE request_id=?1", params![request_id])
}

fn review_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredActivationPlanReview>> {
    conn.query_row(
        &format!(
            "SELECT review_id, plan_id, request_id, provider_id, pool_id,
                    plan_digest, prepared_by_user_id, reviewed_by_user_id,
                    review_note, request_digest, review_digest, reviewed_at
               FROM compute_activation_plan_reviews {filter}"
        ),
        parameters,
        stored_review_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_review_from_row(row: &Row<'_>) -> rusqlite::Result<StoredActivationPlanReview> {
    Ok(StoredActivationPlanReview {
        review_id: row.get(0)?,
        plan_id: row.get(1)?,
        request_id: row.get(2)?,
        provider_id: row.get(3)?,
        pool_id: row.get(4)?,
        plan_digest: row.get(5)?,
        prepared_by_user_id: row.get(6)?,
        reviewed_by_user_id: row.get(7)?,
        review_note: row.get(8)?,
        request_digest: row.get(9)?,
        review_digest: row.get(10)?,
        reviewed_at: row.get(11)?,
    })
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
