use super::*;

#[derive(Debug, Clone)]
struct StoredRecoveryPlanSupersession {
    recovery_supersession_id: String,
    recovery_plan_id: String,
    quarantine_id: String,
    request_id: String,
    provider_id: String,
    pool_id: String,
    plan_digest: String,
    reason: String,
    request_digest: String,
    supersession_digest: String,
    superseded_by_user_id: String,
    superseded_at: String,
}

impl StoredRecoveryPlanSupersession {
    fn into_receipt(self, replayed: bool) -> ComputeActivationRecoveryPlanSupersessionReceipt {
        ComputeActivationRecoveryPlanSupersessionReceipt {
            schema: RECOVERY_SUPERSESSION_SCHEMA,
            recovery_supersession_id: self.recovery_supersession_id,
            recovery_plan_id: self.recovery_plan_id,
            quarantine_id: self.quarantine_id,
            request_id: self.request_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            plan_digest: self.plan_digest,
            reason: self.reason,
            request_digest: self.request_digest,
            supersession_digest: self.supersession_digest,
            superseded_by_user_id: self.superseded_by_user_id,
            superseded_at: self.superseded_at,
            replayed,
            recovery_effect: "plan_superseded",
            provider_effect: "none",
            pool_effect: "none",
            offer_effect: "none",
            node_effect: "none",
            money_effect: "none",
        }
    }
}

impl Store {
    pub(crate) fn supersede_compute_activation_recovery_plan(
        &self,
        input: SupersedeComputeActivationRecoveryPlan,
    ) -> Result<ComputeActivationRecoveryPlanSupersessionReceipt> {
        validate_supersession_input(&input)?;
        let request_digest = supersession_request_digest(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = supersession_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            ensure_supersession_replay(&tx, &existing, &request_digest)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let plan = current_recovery_plan_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("当前 prepared 隔离恢复计划不存在"))?;
        if plan.status != "prepared" || plan.plan_digest != input.expected_plan_digest.trim() {
            bail!("只有当前摘要匹配的 prepared 隔离恢复计划可以废止");
        }
        if supersession_by_plan_on(&tx, &plan.recovery_plan_id)?.is_some() {
            bail!("隔离恢复计划已经存在废止回执");
        }

        let recovery_supersession_id = new_id("compute_activation_recovery_supersession");
        let superseded_at = now();
        let supersession_digest = recovery_supersession_digest(
            &recovery_supersession_id,
            &plan,
            input.reason.trim(),
            &request_digest,
            input.superseded_by_user_id.trim(),
            &superseded_at,
        )?;
        if tx.execute(
            "UPDATE compute_activation_recovery_plans
                SET status='superseded', superseded_at=?1, updated_at=?1
              WHERE recovery_plan_id=?2 AND status='prepared' AND plan_digest=?3",
            params![superseded_at, plan.recovery_plan_id, plan.plan_digest],
        )? != 1
        {
            bail!("隔离恢复计划状态发生并发变化");
        }
        tx.execute(
            "INSERT INTO compute_activation_recovery_plan_supersessions (
                recovery_supersession_id, recovery_plan_id, quarantine_id,
                request_id, provider_id, pool_id, plan_digest, reason,
                request_digest, supersession_digest, idempotency_scope,
                idempotency_key, superseded_by_user_id, superseded_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?14)",
            params![
                recovery_supersession_id,
                plan.recovery_plan_id,
                plan.quarantine_id,
                plan.request_id,
                plan.provider_id,
                plan.pool_id,
                plan.plan_digest,
                input.reason.trim(),
                request_digest,
                supersession_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.superseded_by_user_id.trim(),
                superseded_at,
            ],
        )?;
        let supersession = supersession_by_plan_on(&tx, &plan.recovery_plan_id)?
            .ok_or_else(|| anyhow!("隔离恢复计划废止回执写入后无法读取"))?;
        audit_recovery_supersession_on(&tx, &supersession)?;
        tx.commit()?;
        Ok(supersession.into_receipt(false))
    }

    pub(crate) fn compute_activation_recovery_supersession_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationRecoveryPlanSupersessionReceipt>> {
        validate_exact("申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        supersession_by_request_on(&conn, request_id)?
            .map(|supersession| {
                audit_recovery_supersession_on(&conn, &supersession)?;
                Ok(supersession.into_receipt(false))
            })
            .transpose()
    }
}

fn validate_supersession_input(input: &SupersedeComputeActivationRecoveryPlan) -> Result<()> {
    for (label, value, max) in [
        ("申请 ID", input.request_id.as_str(), 160),
        ("废止原因", input.reason.as_str(), 1000),
        ("幂等范围", input.idempotency_scope.as_str(), 200),
        ("幂等键", input.idempotency_key.as_str(), 160),
        ("废止执行人", input.superseded_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max)?;
    }
    validate_digest("恢复计划摘要", &input.expected_plan_digest)
}

fn supersession_request_digest(input: &SupersedeComputeActivationRecoveryPlan) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": "compute_federation.activation_recovery_plan_supersession_request.v1",
        "request_id": input.request_id,
        "expected_plan_digest": input.expected_plan_digest,
        "reason": input.reason,
        "superseded_by_user_id": input.superseded_by_user_id,
    }))
}

fn recovery_supersession_digest(
    id: &str,
    plan: &ComputeActivationRecoveryPlan,
    reason: &str,
    request_digest: &str,
    actor: &str,
    superseded_at: &str,
) -> Result<String> {
    digest_json(&serde_json::json!({
        "schema": RECOVERY_SUPERSESSION_SCHEMA,
        "recovery_supersession_id": id,
        "recovery_plan_id": plan.recovery_plan_id,
        "quarantine_id": plan.quarantine_id,
        "request_id": plan.request_id,
        "provider_id": plan.provider_id,
        "pool_id": plan.pool_id,
        "plan_digest": plan.plan_digest,
        "reason": reason,
        "request_digest": request_digest,
        "superseded_by_user_id": actor,
        "superseded_at": superseded_at,
    }))
}

fn ensure_supersession_replay(
    conn: &Connection,
    stored: &StoredRecoveryPlanSupersession,
    request_digest: &str,
) -> Result<()> {
    if stored.request_digest != request_digest {
        bail!("隔离恢复计划废止幂等键已绑定不同请求");
    }
    audit_recovery_supersession_on(conn, stored)
}

fn audit_recovery_supersession_on(
    conn: &Connection,
    stored: &StoredRecoveryPlanSupersession,
) -> Result<()> {
    let plan = recovery_plan_by_id_on(conn, &stored.recovery_plan_id)?
        .ok_or_else(|| anyhow!("废止回执引用的隔离恢复计划不存在"))?;
    let request_digest = digest_json(&serde_json::json!({
        "schema": "compute_federation.activation_recovery_plan_supersession_request.v1",
        "request_id": stored.request_id,
        "expected_plan_digest": stored.plan_digest,
        "reason": stored.reason,
        "superseded_by_user_id": stored.superseded_by_user_id,
    }))?;
    let supersession_digest = recovery_supersession_digest(
        &stored.recovery_supersession_id,
        &plan,
        &stored.reason,
        &stored.request_digest,
        &stored.superseded_by_user_id,
        &stored.superseded_at,
    )?;
    if plan.status != "superseded"
        || plan.superseded_at.as_deref() != Some(stored.superseded_at.as_str())
        || plan.quarantine_id != stored.quarantine_id
        || plan.request_id != stored.request_id
        || plan.provider_id != stored.provider_id
        || plan.pool_id != stored.pool_id
        || plan.plan_digest != stored.plan_digest
        || request_digest != stored.request_digest
        || supersession_digest != stored.supersession_digest
    {
        bail!("隔离恢复计划废止回执审计失败");
    }
    Ok(())
}

fn supersession_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRecoveryPlanSupersession>> {
    supersession_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn supersession_by_plan_on(
    conn: &Connection,
    plan_id: &str,
) -> Result<Option<StoredRecoveryPlanSupersession>> {
    supersession_on(conn, "WHERE recovery_plan_id=?1", params![plan_id])
}

fn supersession_by_request_on(
    conn: &Connection,
    request_id: &str,
) -> Result<Option<StoredRecoveryPlanSupersession>> {
    supersession_on(
        conn,
        "WHERE request_id=?1 ORDER BY superseded_at DESC, recovery_supersession_id DESC LIMIT 1",
        params![request_id],
    )
}

fn supersession_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    params: P,
) -> Result<Option<StoredRecoveryPlanSupersession>> {
    conn.query_row(
        &format!(
            "SELECT recovery_supersession_id, recovery_plan_id, quarantine_id,
                    request_id, provider_id, pool_id, plan_digest, reason,
                    request_digest, supersession_digest, superseded_by_user_id,
                    superseded_at
               FROM compute_activation_recovery_plan_supersessions {filter}"
        ),
        params,
        |row| {
            Ok(StoredRecoveryPlanSupersession {
                recovery_supersession_id: row.get(0)?,
                recovery_plan_id: row.get(1)?,
                quarantine_id: row.get(2)?,
                request_id: row.get(3)?,
                provider_id: row.get(4)?,
                pool_id: row.get(5)?,
                plan_digest: row.get(6)?,
                reason: row.get(7)?,
                request_digest: row.get(8)?,
                supersession_digest: row.get(9)?,
                superseded_by_user_id: row.get(10)?,
                superseded_at: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}
