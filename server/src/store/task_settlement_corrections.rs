use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};

use crate::{
    group_ai::types::CreateMatterRecord,
    task_settlement::model::{
        CreateSettlementCorrection, SettlementCorrectionDetail, CORRECTION_CANCELED,
        CORRECTION_MATTER_PENDING, CORRECTION_POSTED, DISPUTE_ACCEPTED,
        RECEIPT_KIND_CORRECTION_REPLACEMENT, RECEIPT_KIND_CORRECTION_REVERSAL,
        RECEIPT_KIND_STANDARD, RECEIPT_RECONCILED,
    },
};

use super::{
    group_ai::insert_project_ai_matter_locked,
    new_id, now,
    task_settlement_correction_posting::{
        insert_correction_leg, CorrectionLeg, REPLACEMENT_POLICY, REVERSAL_POLICY,
    },
    task_settlement_correction_rows::{
        clean, correction_detail_with_conn, correction_select, insert_correction_event,
        read_correction, select_active_correction, select_correction, select_receipt,
    },
    Store,
};

impl Store {
    pub(crate) fn create_task_settlement_correction_with_matter(
        &self,
        input: CreateSettlementCorrection<'_>,
        matter_record: CreateMatterRecord,
    ) -> Result<SettlementCorrectionDetail> {
        validate_corrected_amounts(
            input.corrected_compute_amount_micros,
            input.corrected_provider_amount_micros,
        )?;
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let (dispute_status, original_receipt_id): (String, String) = tx
            .query_row(
                "SELECT status, settlement_receipt_id
                   FROM task_settlement_disputes
                  WHERE project_id=?1 AND id=?2",
                params![input.project_id.trim(), input.dispute_id.trim()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("影子结算争议不存在"))?;
        if dispute_status != DISPUTE_ACCEPTED {
            bail!("只有已接受的争议可以创建纠正 Matter");
        }
        let original = select_receipt(&tx, input.project_id, &original_receipt_id)?
            .ok_or_else(|| anyhow!("争议关联的原影子凭证不存在"))?;
        if original.status != RECEIPT_RECONCILED
            || original.receipt_kind == RECEIPT_KIND_CORRECTION_REVERSAL
        {
            bail!("作废凭证或纠正冲销凭证不能再次发起纠正");
        }
        if let Some(existing) = select_active_correction(&tx, input.project_id, input.dispute_id)? {
            if existing.status == CORRECTION_POSTED {
                bail!("该争议已经完成纠正，不能重复创建");
            }
            let same = existing.corrected_compute_amount_micros
                == input.corrected_compute_amount_micros
                && existing.corrected_provider_amount_micros
                    == input.corrected_provider_amount_micros
                && existing.summary == input.summary.trim()
                && existing.evidence_ref.as_deref() == clean(input.evidence_ref)
                && existing.created_by_user_id == input.actor_user_id.trim();
            if !same {
                bail!("该争议已有待验收纠正 Matter，内容发生冲突");
            }
            let id = existing.id.clone();
            tx.commit()?;
            drop(conn);
            return self
                .task_settlement_correction_detail(input.project_id, &id)?
                .ok_or_else(|| anyhow!("纠正流程幂等读取失败"));
        }
        if matter_record.project_id.trim() != input.project_id.trim()
            || matter_record.requester_user_id.trim() != input.actor_user_id.trim()
        {
            bail!("纠正 Matter 的项目或创建者不一致");
        }
        let matter_id = insert_project_ai_matter_locked(&tx, matter_record)?;
        let correction_id = new_id("settlement_correction");
        let timestamp = now();
        let platform =
            input.corrected_compute_amount_micros - input.corrected_provider_amount_micros;
        tx.execute(
            "INSERT INTO task_settlement_corrections (
               id, project_id, dispute_id, original_settlement_receipt_id,
               correction_matter_id, status, corrected_compute_amount_micros,
               corrected_provider_amount_micros, corrected_platform_amount_micros,
               summary, evidence_ref, created_by_user_id, posted_by_user_id,
               reversal_receipt_id, replacement_receipt_id, created_at, posted_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, 'matter_pending', ?6, ?7, ?8, ?9, ?10, ?11,
               NULL, NULL, NULL, ?12, NULL, ?12
             )",
            params![
                correction_id,
                input.project_id.trim(),
                input.dispute_id.trim(),
                original_receipt_id,
                matter_id,
                input.corrected_compute_amount_micros,
                input.corrected_provider_amount_micros,
                platform,
                input.summary.trim(),
                clean(input.evidence_ref),
                input.actor_user_id.trim(),
                timestamp,
            ],
        )?;
        insert_correction_event(
            &tx,
            &correction_id,
            "matter_created",
            None,
            CORRECTION_MATTER_PENDING,
            input.actor_user_id,
            Some("纠正 Matter 已创建，等待执行、证据与人工验收"),
            &timestamp,
        )?;
        tx.commit()?;
        drop(conn);
        self.task_settlement_correction_detail(input.project_id, &correction_id)?
            .ok_or_else(|| anyhow!("影子结算纠正流程写入后无法读取"))
    }

    pub(crate) fn list_task_settlement_corrections(
        &self,
        project_id: &str,
        receipt_id: &str,
        limit: usize,
    ) -> Result<Vec<SettlementCorrectionDetail>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE correction.project_id=?1
                  AND correction.original_settlement_receipt_id=?2
                ORDER BY correction.created_at DESC LIMIT ?3",
            correction_select()
        ))?;
        let rows = stmt
            .query_map(
                params![
                    project_id.trim(),
                    receipt_id.trim(),
                    limit.clamp(1, 100) as i64
                ],
                read_correction,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|correction| correction_detail_with_conn(&conn, correction))
            .collect()
    }

    pub(crate) fn task_settlement_correction_detail(
        &self,
        project_id: &str,
        correction_id: &str,
    ) -> Result<Option<SettlementCorrectionDetail>> {
        let conn = self.conn()?;
        select_correction(&conn, project_id, correction_id)?
            .map(|correction| correction_detail_with_conn(&conn, correction))
            .transpose()
    }

    pub(crate) fn post_task_settlement_correction(
        &self,
        project_id: &str,
        correction_id: &str,
        actor_user_id: &str,
    ) -> Result<SettlementCorrectionDetail> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let correction = select_correction(&tx, project_id, correction_id)?
            .ok_or_else(|| anyhow!("影子结算纠正流程不存在"))?;
        if correction.status == CORRECTION_POSTED {
            tx.commit()?;
            drop(conn);
            return self
                .task_settlement_correction_detail(project_id, correction_id)?
                .ok_or_else(|| anyhow!("已过账纠正流程读取失败"));
        }
        if correction.status != CORRECTION_MATTER_PENDING {
            bail!("已取消的纠正流程不能过账");
        }
        if correction.matter_status != "done"
            || correction.matter_final_decision.as_deref() != Some("accepted")
        {
            bail!("纠正 Matter 尚未通过人工验收，不能过账");
        }
        let dispute_status: String = tx.query_row(
            "SELECT status FROM task_settlement_disputes WHERE project_id=?1 AND id=?2",
            params![project_id.trim(), correction.dispute_id],
            |row| row.get(0),
        )?;
        if dispute_status != DISPUTE_ACCEPTED {
            bail!("纠正关联的争议不再是已接受状态");
        }
        let original = select_receipt(&tx, project_id, &correction.original_settlement_receipt_id)?
            .ok_or_else(|| anyhow!("纠正关联的原凭证不存在"))?;
        let (payer_user_id, payee_user_id): (String, Option<String>) = tx.query_row(
            "SELECT payer_user_id, payee_user_id FROM task_settlement_intents WHERE id=?1",
            [&original.intent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if correction.corrected_provider_amount_micros > 0 && payee_user_id.is_none() {
            bail!("纠正后的节点金额大于零，但原凭证没有节点收款方");
        }
        let timestamp = now();
        let reversal = CorrectionLeg {
            key: "reversal",
            policy: REVERSAL_POLICY,
            receipt_kind: RECEIPT_KIND_CORRECTION_REVERSAL,
            compute_amount_micros: original.compute_amount_micros,
            provider_amount_micros: original.provider_amount_micros,
            platform_amount_micros: original.platform_amount_micros,
            reverse_postings: true,
        };
        let replacement = CorrectionLeg {
            key: "replacement",
            policy: REPLACEMENT_POLICY,
            receipt_kind: RECEIPT_KIND_CORRECTION_REPLACEMENT,
            compute_amount_micros: correction.corrected_compute_amount_micros,
            provider_amount_micros: correction.corrected_provider_amount_micros,
            platform_amount_micros: correction.corrected_platform_amount_micros,
            reverse_postings: false,
        };
        let reversal_receipt_id = insert_correction_leg(
            &tx,
            &correction,
            &original.currency,
            &payer_user_id,
            payee_user_id.as_deref(),
            &reversal,
            &timestamp,
        )?;
        let replacement_receipt_id = insert_correction_leg(
            &tx,
            &correction,
            &original.currency,
            &payer_user_id,
            payee_user_id.as_deref(),
            &replacement,
            &timestamp,
        )?;
        tx.execute(
            "UPDATE task_settlement_corrections
                SET status='posted', posted_by_user_id=?3,
                    reversal_receipt_id=?4, replacement_receipt_id=?5,
                    posted_at=?6, updated_at=?6
              WHERE project_id=?1 AND id=?2 AND status='matter_pending'",
            params![
                project_id.trim(),
                correction_id.trim(),
                actor_user_id.trim(),
                reversal_receipt_id,
                replacement_receipt_id,
                timestamp,
            ],
        )?;
        insert_correction_event(
            &tx,
            correction_id,
            CORRECTION_POSTED,
            Some(CORRECTION_MATTER_PENDING),
            CORRECTION_POSTED,
            actor_user_id,
            Some("纠正 Matter 已验收，冲销与替换凭证已原子追加"),
            &timestamp,
        )?;
        tx.commit()?;
        drop(conn);
        self.task_settlement_correction_detail(project_id, correction_id)?
            .ok_or_else(|| anyhow!("影子结算纠正过账后无法读取"))
    }

    pub(crate) fn post_task_settlement_corrections_for_matter(
        &self,
        project_id: &str,
        matter_id: &str,
        actor_user_id: &str,
    ) -> Result<usize> {
        let ids = {
            let conn = self.conn()?;
            let mut stmt = conn.prepare(
                "SELECT id FROM task_settlement_corrections
                  WHERE project_id=?1 AND correction_matter_id=?2 AND status='matter_pending'",
            )?;
            let rows = stmt
                .query_map(params![project_id.trim(), matter_id.trim()], |row| {
                    row.get(0)
                })?
                .collect::<rusqlite::Result<Vec<String>>>()?;
            rows
        };
        for id in &ids {
            self.post_task_settlement_correction(project_id, id, actor_user_id)?;
        }
        Ok(ids.len())
    }

    pub(crate) fn cancel_task_settlement_corrections_for_matter(
        &self,
        project_id: &str,
        matter_id: &str,
        actor_user_id: &str,
    ) -> Result<usize> {
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let ids = {
            let mut stmt = tx.prepare(
                "SELECT id FROM task_settlement_corrections
                  WHERE project_id=?1 AND correction_matter_id=?2 AND status='matter_pending'",
            )?;
            let rows = stmt
                .query_map(params![project_id.trim(), matter_id.trim()], |row| {
                    row.get(0)
                })?
                .collect::<rusqlite::Result<Vec<String>>>()?;
            rows
        };
        for id in &ids {
            tx.execute(
                "UPDATE task_settlement_corrections
                    SET status='canceled', updated_at=?3
                  WHERE project_id=?1 AND id=?2 AND status='matter_pending'",
                params![project_id.trim(), id, timestamp],
            )?;
            insert_correction_event(
                &tx,
                id,
                CORRECTION_CANCELED,
                Some(CORRECTION_MATTER_PENDING),
                CORRECTION_CANCELED,
                actor_user_id,
                Some("纠正 Matter 已取消，未生成任何纠正凭证"),
                &timestamp,
            )?;
        }
        tx.commit()?;
        Ok(ids.len())
    }

    pub(crate) fn task_settlement_receipt_is_correction(
        &self,
        project_id: &str,
        receipt_id: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let kind = conn
            .query_row(
                "SELECT receipt_kind FROM task_settlement_receipts
                  WHERE project_id=?1 AND id=?2",
                params![project_id.trim(), receipt_id.trim()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| anyhow!("影子结算凭证不存在"))?;
        Ok(kind != RECEIPT_KIND_STANDARD)
    }
}

fn validate_corrected_amounts(compute: i64, provider: i64) -> Result<()> {
    if compute < 0 || provider < 0 {
        bail!("纠正金额不能为负数");
    }
    if provider > compute {
        bail!("纠正后的节点金额不能高于计算金额");
    }
    Ok(())
}
