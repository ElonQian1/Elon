use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        capacity::ComputeCapacityPoolStatus,
        provider::{ComputeProvider, PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_REGISTERING},
    },
    compute_federation_activation_model::ACTIVATION_REQUEST_STATUS_APPROVED,
    compute_federation_activation_plan_model::{
        ComputeActivationPlan, ComputeActivationPlanReceipt, COMPUTE_ACTIVATION_PLAN_SCHEMA,
    },
};

use super::{
    compute_activation_requests::request_on,
    compute_capacity_audit::stable_compute_capacity_pool_audit_digest,
    compute_capacity_pool_queries::current_capacity_pool_on,
    compute_provider_registry::{
        current_registered_provider_on, validate_compute_provider_contract,
    },
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct PrepareComputeActivationPlan {
    pub request_id: String,
    pub provider_id: String,
    pub pool_id: String,
    pub expected_request_digest: String,
    pub expected_provider_policy_revision: i64,
    pub expected_provider_digest: String,
    pub expected_capacity_epoch: i64,
    pub expected_pool_revision: i64,
    pub expected_pool_digest: String,
    pub target_provider: ComputeProvider,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub prepared_by_user_id: String,
}

impl Store {
    pub(crate) fn prepare_compute_activation_plan(
        &self,
        input: PrepareComputeActivationPlan,
    ) -> Result<ComputeActivationPlanReceipt> {
        validate_input(&input)?;
        let target_provider_json = serde_json::to_string(&input.target_provider)?;
        let target_provider_digest = digest_bytes(target_provider_json.as_bytes());
        let endpoint = input
            .target_provider
            .endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("激活计划目标 Provider 缺少路由引用"))?;
        let endpoint_digest = digest_json(endpoint)?;
        let plan_digest = plan_digest(&input, &target_provider_digest, &endpoint_digest)?;

        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = plan_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            ensure_plan_replay(&existing, &plan_digest)?;
            tx.commit()?;
            return Ok(ComputeActivationPlanReceipt {
                plan: existing,
                replayed: true,
                activation_effect: "none",
            });
        }
        if let Some(existing) = plan_by_request_on(&tx, input.request_id.trim())? {
            ensure_plan_replay(&existing, &plan_digest)?;
            tx.commit()?;
            return Ok(ComputeActivationPlanReceipt {
                plan: existing,
                replayed: true,
                activation_effect: "none",
            });
        }
        validate_current_dependencies_on(&tx, &input)?;

        let plan_id = new_id("compute_activation_plan");
        let prepared_at = now();
        tx.execute(
            "INSERT INTO compute_activation_plans (
                plan_id, request_id, provider_id, pool_id,
                expected_request_digest, expected_provider_policy_revision,
                expected_provider_digest, expected_capacity_epoch,
                expected_pool_revision, expected_pool_digest,
                target_provider_policy_revision, target_provider_digest,
                target_provider_json, endpoint_digest, status,
                idempotency_scope, idempotency_key, plan_digest,
                prepared_by_user_id, prepared_at, applied_at, superseded_at,
                created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, 'prepared', ?15, ?16, ?17, ?18, ?19, NULL, NULL,
                ?19, ?19
             )",
            params![
                plan_id,
                input.request_id.trim(),
                input.provider_id.trim(),
                input.pool_id.trim(),
                input.expected_request_digest.trim(),
                input.expected_provider_policy_revision,
                input.expected_provider_digest.trim(),
                input.expected_capacity_epoch,
                input.expected_pool_revision,
                input.expected_pool_digest.trim(),
                input.target_provider.policy_revision,
                target_provider_digest,
                target_provider_json,
                endpoint_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                plan_digest,
                input.prepared_by_user_id.trim(),
                prepared_at,
            ],
        )?;
        let plan = plan_on(&tx, &plan_id)?.ok_or_else(|| anyhow!("激活计划写入后无法读取"))?;
        tx.commit()?;
        Ok(ComputeActivationPlanReceipt {
            plan,
            replayed: false,
            activation_effect: "none",
        })
    }

    pub(crate) fn compute_activation_plan_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationPlan>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        plan_by_request_on(&self.conn()?, request_id.trim())
    }
}

fn validate_input(input: &PrepareComputeActivationPlan) -> Result<()> {
    for (label, value, max_len) in [
        ("激活证据申请 ID", input.request_id.as_str(), 160),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("CapacityPool ID", input.pool_id.as_str(), 160),
        ("激活计划幂等范围", input.idempotency_scope.as_str(), 200),
        ("激活计划幂等键", input.idempotency_key.as_str(), 160),
        ("激活计划准备人", input.prepared_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    for (label, value) in [
        ("激活证据申请摘要", input.expected_request_digest.as_str()),
        ("Provider 摘要", input.expected_provider_digest.as_str()),
        ("CapacityPool 摘要", input.expected_pool_digest.as_str()),
    ] {
        validate_digest(label, value)?;
    }
    if input.expected_provider_policy_revision <= 0
        || input.expected_capacity_epoch <= 0
        || input.expected_pool_revision <= 0
        || input.target_provider.policy_revision != input.expected_provider_policy_revision + 1
    {
        bail!("激活计划版本无效或目标 Provider revision 不连续");
    }
    if input.target_provider.provider_id != input.provider_id
        || input.target_provider.status != PROVIDER_STATUS_ACTIVE
        || input.target_provider.endpoint.is_none()
        || input.target_provider.adapter.is_some()
    {
        bail!("激活计划目标 Provider 身份、状态或路由类型无效");
    }
    validate_compute_provider_contract(&input.target_provider)
}

fn validate_current_dependencies_on(
    conn: &Connection,
    input: &PrepareComputeActivationPlan,
) -> Result<()> {
    let request =
        request_on(conn, input.request_id.trim())?.ok_or_else(|| anyhow!("激活证据申请不存在"))?;
    if request.status != ACTIVATION_REQUEST_STATUS_APPROVED
        || request.request_digest != input.expected_request_digest
        || request.provider_id != input.provider_id
        || request.pool_id != input.pool_id
        || request.expected_provider_policy_revision != input.expected_provider_policy_revision
        || request.expected_provider_digest != input.expected_provider_digest
        || request.expected_capacity_epoch != input.expected_capacity_epoch
        || request.expected_pool_revision != input.expected_pool_revision
        || request.expected_pool_digest != input.expected_pool_digest
    {
        bail!("激活证据申请状态、摘要或依赖版本已变化");
    }

    let provider = current_registered_provider_on(conn, input.provider_id.trim())?
        .ok_or_else(|| anyhow!("算力 Provider 不存在"))?;
    if provider.provider.status != PROVIDER_STATUS_REGISTERING
        || provider.provider.policy_revision != input.expected_provider_policy_revision
        || provider.provider_digest != input.expected_provider_digest
        || input.target_provider.policy_revision != provider.provider.policy_revision + 1
        || input.target_provider.provider_id != provider.provider.provider_id
        || input.target_provider.provider_kind != provider.provider.provider_kind
        || input.target_provider.owner_account_id != provider.provider.owner_account_id
        || input.target_provider.created_at != provider.provider.created_at
    {
        bail!("算力 Provider 状态、身份或版本已变化");
    }

    let pool = current_capacity_pool_on(conn, input.pool_id.trim())?
        .ok_or_else(|| anyhow!("容量池不存在"))?;
    if pool.provider_id != input.provider_id
        || pool.status != ComputeCapacityPoolStatus::Registering
        || pool.binding.capacity_epoch != input.expected_capacity_epoch
        || pool.binding.pool_revision != input.expected_pool_revision
        || pool.binding.pool_digest != input.expected_pool_digest
    {
        bail!("容量池归属、状态或版本已变化");
    }
    let audit = Store::audit_compute_capacity_pool_epoch_on(
        conn,
        input.pool_id.trim(),
        input.expected_capacity_epoch,
    )?;
    if !audit.healthy
        || audit.current_capacity_epoch != input.expected_capacity_epoch
        || stable_compute_capacity_pool_audit_digest(&audit)? != request.ledger_audit_digest
    {
        bail!("容量池账本审计结果已变化");
    }
    Ok(())
}

fn plan_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<ComputeActivationPlan>> {
    let plan_id = conn
        .query_row(
            "SELECT plan_id FROM compute_activation_plans
              WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    plan_id
        .map(|plan_id| plan_on(conn, &plan_id))
        .transpose()
        .map(Option::flatten)
}

fn plan_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<ComputeActivationPlan>> {
    let plan_id = conn
        .query_row(
            "SELECT plan_id FROM compute_activation_plans WHERE request_id=?1",
            params![request_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    plan_id
        .map(|plan_id| plan_on(conn, &plan_id))
        .transpose()
        .map(Option::flatten)
}

fn plan_on(conn: &Connection, plan_id: &str) -> Result<Option<ComputeActivationPlan>> {
    let stored = conn
        .query_row(PLAN_SELECT, params![plan_id], stored_plan_from_row)
        .optional()?;
    stored.map(audit_stored_plan).transpose()
}

fn audit_stored_plan(stored: StoredPlan) -> Result<ComputeActivationPlan> {
    let target_provider: ComputeProvider = serde_json::from_str(&stored.target_provider_json)?;
    validate_compute_provider_contract(&target_provider)?;
    if target_provider.provider_id != stored.provider_id
        || target_provider.policy_revision != stored.target_provider_policy_revision
        || target_provider.status != PROVIDER_STATUS_ACTIVE
    {
        bail!("激活计划目标 Provider 身份、版本或状态不一致");
    }
    let target_provider_digest = digest_bytes(stored.target_provider_json.as_bytes());
    if target_provider_digest != stored.target_provider_digest {
        bail!("激活计划目标 Provider 摘要不一致");
    }
    let endpoint = target_provider
        .endpoint
        .as_ref()
        .ok_or_else(|| anyhow!("激活计划目标 Provider 缺少路由引用"))?;
    if digest_json(endpoint)? != stored.endpoint_digest {
        bail!("激活计划路由引用摘要不一致");
    }
    let input = stored.as_input(target_provider.clone());
    if plan_digest(
        &input,
        &stored.target_provider_digest,
        &stored.endpoint_digest,
    )? != stored.plan_digest
    {
        bail!("激活计划规范摘要不一致");
    }
    Ok(stored.into_plan(target_provider))
}

fn ensure_plan_replay(existing: &ComputeActivationPlan, expected_digest: &str) -> Result<()> {
    if existing.plan_digest != expected_digest {
        bail!("该证据申请或激活计划幂等键已绑定不同目标合同");
    }
    Ok(())
}

fn plan_digest(
    input: &PrepareComputeActivationPlan,
    target_provider_digest: &str,
    endpoint_digest: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema":COMPUTE_ACTIVATION_PLAN_SCHEMA,
        "request_id":input.request_id.trim(),
        "provider_id":input.provider_id.trim(),
        "pool_id":input.pool_id.trim(),
        "expected_request_digest":input.expected_request_digest.trim(),
        "expected_provider_policy_revision":input.expected_provider_policy_revision,
        "expected_provider_digest":input.expected_provider_digest.trim(),
        "expected_capacity_epoch":input.expected_capacity_epoch,
        "expected_pool_revision":input.expected_pool_revision,
        "expected_pool_digest":input.expected_pool_digest.trim(),
        "target_provider_policy_revision":input.target_provider.policy_revision,
        "target_provider_digest":target_provider_digest,
        "endpoint_digest":endpoint_digest,
        "prepared_by_user_id":input.prepared_by_user_id.trim(),
    }))
}

fn digest_json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(digest_bytes(&serde_json::to_vec(value)?))
}

fn digest_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
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

struct StoredPlan {
    plan_id: String,
    request_id: String,
    provider_id: String,
    pool_id: String,
    expected_request_digest: String,
    expected_provider_policy_revision: i64,
    expected_provider_digest: String,
    expected_capacity_epoch: i64,
    expected_pool_revision: i64,
    expected_pool_digest: String,
    target_provider_policy_revision: i64,
    target_provider_digest: String,
    target_provider_json: String,
    endpoint_digest: String,
    status: String,
    plan_digest: String,
    prepared_by_user_id: String,
    prepared_at: String,
    applied_at: Option<String>,
    superseded_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl StoredPlan {
    fn as_input(&self, target_provider: ComputeProvider) -> PrepareComputeActivationPlan {
        PrepareComputeActivationPlan {
            request_id: self.request_id.clone(),
            provider_id: self.provider_id.clone(),
            pool_id: self.pool_id.clone(),
            expected_request_digest: self.expected_request_digest.clone(),
            expected_provider_policy_revision: self.expected_provider_policy_revision,
            expected_provider_digest: self.expected_provider_digest.clone(),
            expected_capacity_epoch: self.expected_capacity_epoch,
            expected_pool_revision: self.expected_pool_revision,
            expected_pool_digest: self.expected_pool_digest.clone(),
            target_provider,
            idempotency_scope: String::new(),
            idempotency_key: String::new(),
            prepared_by_user_id: self.prepared_by_user_id.clone(),
        }
    }

    fn into_plan(self, target_provider: ComputeProvider) -> ComputeActivationPlan {
        ComputeActivationPlan {
            schema: COMPUTE_ACTIVATION_PLAN_SCHEMA,
            plan_id: self.plan_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            expected_request_digest: self.expected_request_digest,
            expected_provider_policy_revision: self.expected_provider_policy_revision,
            expected_provider_digest: self.expected_provider_digest,
            expected_capacity_epoch: self.expected_capacity_epoch,
            expected_pool_revision: self.expected_pool_revision,
            expected_pool_digest: self.expected_pool_digest,
            target_provider_policy_revision: self.target_provider_policy_revision,
            target_provider_digest: self.target_provider_digest,
            target_provider,
            endpoint_digest: self.endpoint_digest,
            status: self.status,
            plan_digest: self.plan_digest,
            prepared_by_user_id: self.prepared_by_user_id,
            prepared_at: self.prepared_at,
            applied_at: self.applied_at,
            superseded_at: self.superseded_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn stored_plan_from_row(row: &Row<'_>) -> rusqlite::Result<StoredPlan> {
    Ok(StoredPlan {
        plan_id: row.get(0)?,
        request_id: row.get(1)?,
        provider_id: row.get(2)?,
        pool_id: row.get(3)?,
        expected_request_digest: row.get(4)?,
        expected_provider_policy_revision: row.get(5)?,
        expected_provider_digest: row.get(6)?,
        expected_capacity_epoch: row.get(7)?,
        expected_pool_revision: row.get(8)?,
        expected_pool_digest: row.get(9)?,
        target_provider_policy_revision: row.get(10)?,
        target_provider_digest: row.get(11)?,
        target_provider_json: row.get(12)?,
        endpoint_digest: row.get(13)?,
        status: row.get(14)?,
        plan_digest: row.get(15)?,
        prepared_by_user_id: row.get(16)?,
        prepared_at: row.get(17)?,
        applied_at: row.get(18)?,
        superseded_at: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

const PLAN_SELECT: &str = "SELECT plan_id, request_id, provider_id, pool_id,
       expected_request_digest, expected_provider_policy_revision,
       expected_provider_digest, expected_capacity_epoch, expected_pool_revision,
       expected_pool_digest, target_provider_policy_revision, target_provider_digest,
       target_provider_json, endpoint_digest, status, plan_digest,
       prepared_by_user_id, prepared_at, applied_at, superseded_at,
       created_at, updated_at
  FROM compute_activation_plans WHERE plan_id=?1";
