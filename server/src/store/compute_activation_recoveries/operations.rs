use super::*;

impl Store {
    pub(crate) fn prepare_compute_activation_recovery_plan(
        &self,
        input: PrepareComputeActivationRecoveryPlan,
    ) -> Result<ComputeActivationRecoveryPlanReceipt> {
        validate_prepare_input(&input)?;
        let target_json = serde_json::to_string(&input.target_provider)?;
        let target_digest = digest_bytes(target_json.as_bytes());
        let routing_digest = digest_json(&serde_json::json!({
            "endpoint":input.target_provider.endpoint,
            "adapter":input.target_provider.adapter,
        }))?;
        let evidence_refs = normalize_refs(input.evidence_refs.clone())?;
        let evidence_refs_digest = digest_json(&evidence_refs)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let quarantine = quarantine_by_request_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("激活隔离回执不存在"))?;
        audit_quarantine_on(&tx, &quarantine)?;
        if let Some(existing) = recovery_plan_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            let plan_digest = recovery_plan_digest(
                &quarantine,
                &input,
                &target_digest,
                &routing_digest,
                &evidence_refs_digest,
                existing.expected_pool_revision,
                &existing.expected_pool_digest,
            )?;
            ensure_plan_replay(&existing, &plan_digest)?;
            tx.commit()?;
            return Ok(plan_receipt(existing, true));
        }
        if let Some(existing) = current_recovery_plan_on(&tx, input.request_id.trim())? {
            let plan_digest = recovery_plan_digest(
                &quarantine,
                &input,
                &target_digest,
                &routing_digest,
                &evidence_refs_digest,
                existing.expected_pool_revision,
                &existing.expected_pool_digest,
            )?;
            ensure_plan_replay(&existing, &plan_digest)?;
            tx.commit()?;
            return Ok(plan_receipt(existing, true));
        }
        validate_recovery_dependencies(&tx, &quarantine, &input, &target_digest)?;
        let pool = current_capacity_pool_on(&tx, &quarantine.pool_id)?
            .ok_or_else(|| anyhow!("CapacityPool 不存在"))?;
        let plan_digest = recovery_plan_digest(
            &quarantine,
            &input,
            &target_digest,
            &routing_digest,
            &evidence_refs_digest,
            pool.binding.pool_revision,
            &pool.binding.pool_digest,
        )?;

        let recovery_plan_id = new_id("compute_activation_recovery_plan");
        let prepared_at = now();
        tx.execute(
            "INSERT INTO compute_activation_recovery_plans (
                recovery_plan_id, quarantine_id, application_id, request_id,
                provider_id, pool_id, expected_quarantine_digest,
                expected_provider_policy_revision, expected_provider_digest,
                expected_capacity_epoch, expected_pool_revision, expected_pool_digest,
                target_provider_policy_revision, target_provider_digest,
                target_provider_json, routing_digest, remediation_summary,
                evidence_refs_json, evidence_refs_digest, status, plan_digest,
                idempotency_scope, idempotency_key, prepared_by_user_id,
                prepared_at, applied_at, superseded_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                       ?13, ?14, ?15, ?16, ?17, ?18, ?19, 'prepared', ?20,
                       ?21, ?22, ?23, ?24, NULL, NULL, ?24, ?24)",
            params![
                recovery_plan_id,
                quarantine.quarantine_id,
                quarantine.application_id,
                quarantine.request_id,
                quarantine.provider_id,
                quarantine.pool_id,
                quarantine.quarantine_digest,
                quarantine.quarantined_provider_policy_revision,
                quarantine.quarantined_provider_digest,
                quarantine.capacity_epoch,
                pool.binding.pool_revision,
                pool.binding.pool_digest,
                input.target_provider.policy_revision,
                target_digest,
                target_json,
                routing_digest,
                input.remediation_summary.trim(),
                serde_json::to_string(&evidence_refs)?,
                evidence_refs_digest,
                plan_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.prepared_by_user_id.trim(),
                prepared_at,
            ],
        )?;
        let plan = recovery_plan_by_id_on(&tx, &recovery_plan_id)?
            .ok_or_else(|| anyhow!("恢复计划写入后无法读取"))?;
        tx.commit()?;
        Ok(plan_receipt(plan, false))
    }

    pub(crate) fn review_compute_activation_recovery_plan(
        &self,
        input: ReviewComputeActivationRecoveryPlan,
    ) -> Result<ComputeActivationRecoveryReviewReceipt> {
        validate_review_input(&input)?;
        let request_digest = review_request_digest(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = recovery_review_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            ensure_review_replay(&tx, &existing, &request_digest)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        let plan = current_recovery_plan_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("prepared 恢复计划不存在"))?;
        if plan.status != "prepared" || plan.plan_digest != input.expected_plan_digest {
            bail!("只有当前摘要匹配的 prepared 恢复计划可以复核");
        }
        if plan.prepared_by_user_id == input.reviewed_by_user_id {
            bail!("恢复计划准备人不能复核自己准备的计划");
        }
        if let Some(existing) = recovery_review_by_plan_on(&tx, &plan.recovery_plan_id)? {
            ensure_review_replay(&tx, &existing, &request_digest)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        let recovery_review_id = new_id("compute_activation_recovery_review");
        let reviewed_at = now();
        let review_digest = recovery_review_digest(
            &recovery_review_id,
            &plan,
            &input,
            &request_digest,
            &reviewed_at,
        )?;
        tx.execute(
            "INSERT INTO compute_activation_recovery_reviews (
                recovery_review_id, recovery_plan_id, request_id, plan_digest,
                prepared_by_user_id, reviewed_by_user_id, review_note,
                request_digest, review_digest, idempotency_scope,
                idempotency_key, reviewed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
            params![
                recovery_review_id,
                plan.recovery_plan_id,
                plan.request_id,
                plan.plan_digest,
                plan.prepared_by_user_id,
                input.reviewed_by_user_id.trim(),
                normalize_note(input.review_note.clone())?,
                request_digest,
                review_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                reviewed_at,
            ],
        )?;
        let review = recovery_review_by_plan_on(&tx, &plan.recovery_plan_id)?
            .ok_or_else(|| anyhow!("恢复复核回执写入后无法读取"))?;
        audit_recovery_review_on(&tx, &review)?;
        tx.commit()?;
        Ok(review.into_receipt(false))
    }

    pub(crate) fn apply_compute_activation_recovery_plan(
        &self,
        input: ApplyComputeActivationRecoveryPlan,
    ) -> Result<ComputeActivationRecoveryApplicationReceipt> {
        validate_apply_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = recovery_application_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            ensure_application_replay(&tx, &existing, &input)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        if let Some(existing) = recovery_application_by_request_on(&tx, input.request_id.trim())? {
            ensure_application_replay(&tx, &existing, &input)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        let plan = current_recovery_plan_on(&tx, input.request_id.trim())?
            .ok_or_else(|| anyhow!("prepared 恢复计划不存在"))?;
        if plan.status != "prepared" || plan.plan_digest != input.expected_plan_digest {
            bail!("只有当前摘要匹配的 prepared 恢复计划可以应用");
        }
        let review = recovery_review_by_plan_on(&tx, &plan.recovery_plan_id)?
            .ok_or_else(|| anyhow!("恢复计划尚未完成第二人复核"))?;
        audit_recovery_review_on(&tx, &review)?;
        validate_apply_dependencies(&tx, &plan)?;

        let provider_receipt = register_compute_provider_on(&tx, &plan.target_provider)?;
        if provider_receipt.replayed
            || provider_receipt.provider_digest != plan.target_provider_digest
        {
            bail!("Provider 未按恢复计划创建精确 active 版本");
        }
        let applied_at = now();
        let pool_event = transition_compute_capacity_pool_status_on(
            &tx,
            &TransitionComputeCapacityPoolStatus {
                pool_id: plan.pool_id.clone(),
                expected_capacity_epoch: plan.expected_capacity_epoch,
                expected_status: ComputeCapacityPoolStatus::Quarantined,
                target_status: ComputeCapacityPoolStatus::Active,
                reason_code: "activation_recovery_applied".to_string(),
                subject_kind: "compute_activation_recovery_plan".to_string(),
                subject_id: plan.recovery_plan_id.clone(),
                idempotency_scope: format!("compute_activation_recovery:{}", plan.recovery_plan_id),
                idempotency_key: "pool_active".to_string(),
                request_digest: plan.plan_digest.clone(),
                occurred_at: applied_at.clone(),
            },
        )?;
        if pool_event.replayed || pool_event.current_status != "active" {
            bail!("CapacityPool 未按恢复计划转换为 active");
        }
        if tx.execute(
            "UPDATE compute_activation_recovery_plans
                SET status='applied', applied_at=?1, updated_at=?1
              WHERE recovery_plan_id=?2 AND status='prepared' AND plan_digest=?3",
            params![applied_at, plan.recovery_plan_id, plan.plan_digest],
        )? != 1
        {
            bail!("恢复计划状态发生并发变化");
        }
        let recovery_application_id = new_id("compute_activation_recovery_application");
        let application_digest = recovery_application_digest(
            &recovery_application_id,
            &plan,
            &review,
            &pool_event.event_id,
            input.applied_by_user_id.trim(),
            &applied_at,
        )?;
        tx.execute(
            "INSERT INTO compute_activation_recovery_applications (
                recovery_application_id, recovery_plan_id, recovery_review_id,
                quarantine_id, request_id, provider_id, pool_id, plan_digest,
                review_digest, recovered_provider_policy_revision,
                recovered_provider_digest, capacity_epoch, pool_lifecycle_event_id,
                application_digest, idempotency_scope, idempotency_key,
                applied_by_user_id, applied_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                       ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?18)",
            params![
                recovery_application_id,
                plan.recovery_plan_id,
                review.recovery_review_id,
                plan.quarantine_id,
                plan.request_id,
                plan.provider_id,
                plan.pool_id,
                plan.plan_digest,
                review.review_digest,
                plan.target_provider_policy_revision,
                plan.target_provider_digest,
                plan.expected_capacity_epoch,
                pool_event.event_id,
                application_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.applied_by_user_id.trim(),
                applied_at,
            ],
        )?;
        let stored = recovery_application_by_plan_on(&tx, &plan.recovery_plan_id)?
            .ok_or_else(|| anyhow!("恢复应用回执写入后无法读取"))?;
        audit_recovery_application_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_activation_recovery_plan_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationRecoveryPlan>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        current_recovery_plan_on(&*self.conn()?, request_id.trim())
    }

    pub(crate) fn compute_activation_recovery_review_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationRecoveryReviewReceipt>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        let Some(plan) = current_recovery_plan_on(&conn, request_id.trim())? else {
            return Ok(None);
        };
        recovery_review_by_plan_on(&conn, &plan.recovery_plan_id)?
            .map(|review| {
                audit_recovery_review_on(&conn, &review)?;
                Ok(review.into_receipt(false))
            })
            .transpose()
    }

    pub(crate) fn compute_activation_recovery_application_for_request(
        &self,
        request_id: &str,
    ) -> Result<Option<ComputeActivationRecoveryApplicationReceipt>> {
        validate_exact("激活证据申请 ID", request_id, 160)?;
        let conn = self.conn()?;
        recovery_application_by_request_on(&conn, request_id.trim())?
            .map(|stored| {
                audit_recovery_application_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }

    pub(crate) fn compute_activation_recovery_active_offer_count(
        &self,
        provider_id: &str,
    ) -> Result<i64> {
        validate_exact("Provider ID", provider_id, 160)?;
        self.conn()?
            .query_row(
                "SELECT COUNT(*) FROM compute_offers WHERE provider_id=?1 AND status='active'",
                params![provider_id.trim()],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }
}
