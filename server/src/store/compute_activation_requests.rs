use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::compute_federation_activation_model::{
    ComputeActivationEvidenceRequest, ComputeActivationEvidenceRequestReceipt,
    ACTIVATION_REQUEST_STATUS_APPROVED, ACTIVATION_REQUEST_STATUS_CANCELED,
    ACTIVATION_REQUEST_STATUS_CHANGES_REQUESTED, ACTIVATION_REQUEST_STATUS_REJECTED,
    ACTIVATION_REQUEST_STATUS_SUBMITTED, ACTIVATION_REQUEST_STATUS_SUPERSEDED,
    COMPUTE_ACTIVATION_EVIDENCE_REQUEST_SCHEMA,
};

use super::{new_id, now, Store};

#[derive(Debug, Clone)]
pub(crate) struct SubmitComputeActivationEvidenceRequest {
    pub provider_id: String,
    pub pool_id: String,
    pub owner_user_id: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub expected_capacity_epoch: i64,
    pub expected_pool_revision: i64,
    pub expected_pool_digest: String,
    pub node_binding_ref: String,
    pub ready_capability_digest: String,
    pub route_proof_digest: String,
    pub hardware_observation_digest: String,
    pub ledger_audit_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ReviewComputeActivationEvidenceRequest {
    pub request_id: String,
    pub expected_request_digest: String,
    pub reviewer_user_id: String,
    pub decision: String,
    pub review_note: Option<String>,
}

impl Store {
    pub(crate) fn compute_activation_evidence_request_by_idempotency(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<ComputeActivationEvidenceRequest>> {
        validate_exact("激活证据幂等范围", idempotency_scope, 200)?;
        validate_exact("激活证据幂等键", idempotency_key, 160)?;
        request_by_idempotency_on(
            &*self.conn()?,
            idempotency_scope.trim(),
            idempotency_key.trim(),
        )
    }

    pub(crate) fn submit_compute_activation_evidence_request(
        &self,
        input: SubmitComputeActivationEvidenceRequest,
    ) -> Result<ComputeActivationEvidenceRequestReceipt> {
        validate_submission(&input)?;
        let request_digest = submission_digest(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = request_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            if existing.request_digest != request_digest {
                bail!("相同激活证据申请幂等键不能用于不同请求");
            }
            tx.commit()?;
            return Ok(ComputeActivationEvidenceRequestReceipt {
                request: existing,
                replayed: true,
                activation_effect: "none",
            });
        }
        let active_request = tx
            .query_row(
                "SELECT request_id FROM compute_activation_evidence_requests
                  WHERE provider_id=?1 AND pool_id=?2
                    AND status IN ('submitted', 'approved')",
                params![input.provider_id.trim(), input.pool_id.trim()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(request_id) = active_request {
            bail!("该容量池已有待审或已批准的激活证据申请：{request_id}");
        }
        let request_id = new_id("compute_activation_request");
        let recorded_at = now();
        tx.execute(
            "INSERT INTO compute_activation_evidence_requests (
                request_id, provider_id, pool_id, owner_user_id,
                expected_provider_policy_revision, expected_provider_digest,
                expected_capacity_epoch, expected_pool_revision, expected_pool_digest,
                node_binding_ref, ready_capability_digest, route_proof_digest,
                hardware_observation_digest, ledger_audit_digest, status,
                idempotency_scope, idempotency_key, request_digest,
                requested_at, reviewed_at, reviewed_by_user_id, review_note,
                canceled_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, 'submitted', ?15, ?16, ?17, ?18, NULL, NULL, NULL,
                NULL, ?18, ?18
             )",
            params![
                request_id,
                input.provider_id.trim(),
                input.pool_id.trim(),
                input.owner_user_id.trim(),
                input.expected_provider_policy_revision,
                input.expected_provider_digest.trim(),
                input.expected_capacity_epoch,
                input.expected_pool_revision,
                input.expected_pool_digest.trim(),
                input.node_binding_ref.trim(),
                input.ready_capability_digest.trim(),
                input.route_proof_digest.trim(),
                input.hardware_observation_digest.trim(),
                input.ledger_audit_digest.trim(),
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                request_digest,
                recorded_at,
            ],
        )?;
        let request =
            request_on(&tx, &request_id)?.ok_or_else(|| anyhow!("激活证据申请写入后无法读取"))?;
        tx.commit()?;
        Ok(ComputeActivationEvidenceRequestReceipt {
            request,
            replayed: false,
            activation_effect: "none",
        })
    }

    pub(crate) fn compute_activation_evidence_request(
        &self,
        request_id: &str,
    ) -> Result<ComputeActivationEvidenceRequest> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        request_on(&*self.conn()?, request_id.trim())?.ok_or_else(|| anyhow!("激活证据申请不存在"))
    }

    pub(crate) fn list_compute_activation_evidence_requests_for_owner(
        &self,
        owner_user_id: &str,
        provider_id: &str,
        pool_id: &str,
        limit: usize,
    ) -> Result<Vec<ComputeActivationEvidenceRequest>> {
        validate_exact("激活证据申请所有者", owner_user_id, 160)?;
        validate_exact("算力 Provider ID", provider_id, 160)?;
        validate_exact("容量池 ID", pool_id, 160)?;
        list_requests(
            &*self.conn()?,
            "WHERE owner_user_id=?1 AND provider_id=?2 AND pool_id=?3
             ORDER BY requested_at DESC, request_id DESC LIMIT ?4",
            params![
                owner_user_id.trim(),
                provider_id.trim(),
                pool_id.trim(),
                i64::try_from(limit.clamp(1, 100))?
            ],
        )
    }

    pub(crate) fn list_reviewable_compute_activation_evidence_requests(
        &self,
        status: &str,
        limit: usize,
    ) -> Result<Vec<ComputeActivationEvidenceRequest>> {
        validate_review_status(status)?;
        list_requests(
            &*self.conn()?,
            "WHERE status=?1 ORDER BY requested_at ASC, request_id ASC LIMIT ?2",
            params![status.trim(), i64::try_from(limit.clamp(1, 100))?],
        )
    }

    pub(crate) fn review_compute_activation_evidence_request(
        &self,
        input: ReviewComputeActivationEvidenceRequest,
    ) -> Result<ComputeActivationEvidenceRequest> {
        validate_review(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活证据申请不存在"))?;
        if current.request_digest != input.expected_request_digest.trim() {
            bail!("激活证据申请内容已变化，请刷新后重试");
        }
        if current.status != ACTIVATION_REQUEST_STATUS_SUBMITTED {
            bail!("只有 submitted 激活证据申请可以审核");
        }
        let reviewed_at = now();
        let changed = tx.execute(
            "UPDATE compute_activation_evidence_requests
                SET status=?1, reviewed_at=?2, reviewed_by_user_id=?3,
                    review_note=?4, updated_at=?2
              WHERE request_id=?5 AND status='submitted' AND request_digest=?6",
            params![
                input.decision.trim(),
                reviewed_at,
                input.reviewer_user_id.trim(),
                clean_optional(input.review_note.as_deref()),
                input.request_id.trim(),
                input.expected_request_digest.trim(),
            ],
        )?;
        if changed != 1 {
            bail!("激活证据申请状态已并发变化");
        }
        let request = request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活证据申请审核后无法读取"))?;
        tx.commit()?;
        Ok(request)
    }

    pub(crate) fn cancel_compute_activation_evidence_request(
        &self,
        owner_user_id: &str,
        request_id: &str,
        expected_request_digest: &str,
    ) -> Result<ComputeActivationEvidenceRequest> {
        validate_exact("激活证据申请所有者", owner_user_id, 160)?;
        validate_exact("激活证据申请 ID", request_id, 160)?;
        validate_digest("激活证据申请摘要", expected_request_digest)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current =
            request_on(&tx, request_id.trim())?.ok_or_else(|| anyhow!("激活证据申请不存在"))?;
        if current.owner_user_id != owner_user_id.trim()
            || current.request_digest != expected_request_digest.trim()
        {
            bail!("激活证据申请不属于当前用户或内容已变化");
        }
        if current.status == ACTIVATION_REQUEST_STATUS_CANCELED {
            tx.commit()?;
            return Ok(current);
        }
        if current.status != ACTIVATION_REQUEST_STATUS_SUBMITTED {
            bail!("只有 submitted 激活证据申请可以取消");
        }
        let canceled_at = now();
        let changed = tx.execute(
            "UPDATE compute_activation_evidence_requests
                SET status='canceled', canceled_at=?1, updated_at=?1
              WHERE request_id=?2 AND owner_user_id=?3
                AND status='submitted' AND request_digest=?4",
            params![
                canceled_at,
                request_id.trim(),
                owner_user_id.trim(),
                expected_request_digest.trim(),
            ],
        )?;
        if changed != 1 {
            bail!("激活证据申请状态已并发变化");
        }
        let request = request_on(&tx, request_id.trim())?
            .ok_or_else(|| anyhow!("激活证据申请取消后无法读取"))?;
        tx.commit()?;
        Ok(request)
    }
}

fn validate_submission(input: &SubmitComputeActivationEvidenceRequest) -> Result<()> {
    validate_exact("算力 Provider ID", &input.provider_id, 160)?;
    validate_exact("容量池 ID", &input.pool_id, 160)?;
    validate_exact("激活证据申请所有者", &input.owner_user_id, 160)?;
    validate_exact("节点绑定引用", &input.node_binding_ref, 160)?;
    validate_exact("激活证据幂等范围", &input.idempotency_scope, 200)?;
    validate_exact("激活证据幂等键", &input.idempotency_key, 160)?;
    for (label, value) in [
        ("Provider 摘要", input.expected_provider_digest.as_str()),
        ("Pool 摘要", input.expected_pool_digest.as_str()),
        (
            "ReadyCapability 摘要",
            input.ready_capability_digest.as_str(),
        ),
        ("路由证明摘要", input.route_proof_digest.as_str()),
        ("硬件观测摘要", input.hardware_observation_digest.as_str()),
        ("账本审计摘要", input.ledger_audit_digest.as_str()),
    ] {
        validate_digest(label, value)?;
    }
    if input.expected_provider_policy_revision <= 0
        || input.expected_capacity_epoch <= 0
        || input.expected_pool_revision <= 0
    {
        bail!("激活证据申请的 Provider/Pool 版本必须为正整数");
    }
    Ok(())
}

fn validate_review(input: &ReviewComputeActivationEvidenceRequest) -> Result<()> {
    validate_exact("激活证据申请 ID", &input.request_id, 160)?;
    validate_exact("审核人", &input.reviewer_user_id, 160)?;
    validate_digest("激活证据申请摘要", &input.expected_request_digest)?;
    if !matches!(
        input.decision.trim(),
        ACTIVATION_REQUEST_STATUS_APPROVED
            | ACTIVATION_REQUEST_STATUS_CHANGES_REQUESTED
            | ACTIVATION_REQUEST_STATUS_REJECTED
    ) {
        bail!("激活证据审核决定不受支持");
    }
    let note = input.review_note.as_deref().unwrap_or("").trim();
    if note.chars().count() > 1000 || note.chars().any(char::is_control) {
        bail!("激活证据审核说明最多 1000 个可见字符");
    }
    if input.decision.trim() != ACTIVATION_REQUEST_STATUS_APPROVED && note.is_empty() {
        bail!("退回或拒绝激活证据申请时必须填写说明");
    }
    Ok(())
}

fn validate_review_status(status: &str) -> Result<()> {
    if !matches!(
        status.trim(),
        ACTIVATION_REQUEST_STATUS_SUBMITTED
            | ACTIVATION_REQUEST_STATUS_CHANGES_REQUESTED
            | ACTIVATION_REQUEST_STATUS_APPROVED
            | ACTIVATION_REQUEST_STATUS_REJECTED
            | ACTIVATION_REQUEST_STATUS_CANCELED
            | ACTIVATION_REQUEST_STATUS_SUPERSEDED
    ) {
        bail!("激活证据申请查询状态不受支持");
    }
    Ok(())
}

fn submission_digest(input: &SubmitComputeActivationEvidenceRequest) -> Result<String> {
    let payload = serde_json::json!({
        "schema":COMPUTE_ACTIVATION_EVIDENCE_REQUEST_SCHEMA,
        "provider_id":input.provider_id.trim(),
        "pool_id":input.pool_id.trim(),
        "owner_user_id":input.owner_user_id.trim(),
        "expected_provider_policy_revision":input.expected_provider_policy_revision,
        "expected_provider_digest":input.expected_provider_digest.trim(),
        "expected_capacity_epoch":input.expected_capacity_epoch,
        "expected_pool_revision":input.expected_pool_revision,
        "expected_pool_digest":input.expected_pool_digest.trim(),
        "node_binding_ref":input.node_binding_ref.trim(),
        "ready_capability_digest":input.ready_capability_digest.trim(),
        "route_proof_digest":input.route_proof_digest.trim(),
        "hardware_observation_digest":input.hardware_observation_digest.trim(),
        "ledger_audit_digest":input.ledger_audit_digest.trim(),
        "idempotency_scope":input.idempotency_scope.trim(),
        "idempotency_key":input.idempotency_key.trim(),
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

fn request_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<ComputeActivationEvidenceRequest>> {
    let request_id = conn
        .query_row(
            "SELECT request_id FROM compute_activation_evidence_requests
              WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    request_id
        .map(|request_id| request_on(conn, &request_id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<ComputeActivationEvidenceRequest>> {
    conn.query_row(
        &format!("{REQUEST_SELECT} WHERE request_id=?1"),
        params![request_id],
        request_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn list_requests<P: rusqlite::Params>(
    conn: &Connection,
    suffix: &str,
    params: P,
) -> Result<Vec<ComputeActivationEvidenceRequest>> {
    let mut statement = conn.prepare(&format!("{REQUEST_SELECT} {suffix}"))?;
    let rows = statement
        .query_map(params, request_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn request_from_row(row: &Row<'_>) -> rusqlite::Result<ComputeActivationEvidenceRequest> {
    Ok(ComputeActivationEvidenceRequest {
        schema: COMPUTE_ACTIVATION_EVIDENCE_REQUEST_SCHEMA,
        request_id: row.get(0)?,
        provider_id: row.get(1)?,
        pool_id: row.get(2)?,
        owner_user_id: row.get(3)?,
        expected_provider_policy_revision: row.get(4)?,
        expected_provider_digest: row.get(5)?,
        expected_capacity_epoch: row.get(6)?,
        expected_pool_revision: row.get(7)?,
        expected_pool_digest: row.get(8)?,
        node_binding_ref: row.get(9)?,
        ready_capability_digest: row.get(10)?,
        route_proof_digest: row.get(11)?,
        hardware_observation_digest: row.get(12)?,
        ledger_audit_digest: row.get(13)?,
        status: row.get(14)?,
        request_digest: row.get(15)?,
        requested_at: row.get(16)?,
        reviewed_at: row.get(17)?,
        reviewed_by_user_id: row.get(18)?,
        review_note: row.get(19)?,
        canceled_at: row.get(20)?,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
        superseded_at: row.get(23)?,
        superseded_by_user_id: row.get(24)?,
        supersede_reason: row.get(25)?,
    })
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

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn clean_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

const REQUEST_SELECT: &str = "SELECT request_id, provider_id, pool_id, owner_user_id,
            expected_provider_policy_revision, expected_provider_digest,
            expected_capacity_epoch, expected_pool_revision, expected_pool_digest,
            node_binding_ref, ready_capability_digest, route_proof_digest,
            hardware_observation_digest, ledger_audit_digest, status,
            request_digest, requested_at, reviewed_at, reviewed_by_user_id,
            review_note, canceled_at, created_at, updated_at,
            superseded_at, superseded_by_user_id, supersede_reason
       FROM compute_activation_evidence_requests";
