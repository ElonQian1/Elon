use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension};

use crate::task_settlement::{
    ledger::{ensure_balanced, LedgerPosting},
    model::{
        CreateSettlementIntent, CreateSettlementReceipt, CreateUsageReceipt, LedgerEntry,
        LedgerTransaction, SettlementIntent, SettlementReceipt, TaskEconomyProjectSetting,
        UsageReceipt, INTENT_PENDING, INTENT_POSTED, INTENT_VOIDED,
    },
};

use super::task_settlement_rows::{
    ensure_same_settlement, intent_select, read_intent, read_project_setting, read_settlement,
    read_usage, select_intent_by_id, select_intent_by_key, select_settlement_by_intent,
    select_usage_by_source, settlement_select, usage_select, usage_select_alias,
};
use super::{new_id, now, Store};

impl Store {
    pub(crate) fn task_economy_project_setting(
        &self,
        project_id: &str,
    ) -> Result<TaskEconomyProjectSetting> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT project_id, enabled, shadow_only, updated_by_user_id, updated_at
                   FROM task_economy_project_settings WHERE project_id = ?1",
                params![project_id.trim()],
                read_project_setting,
            )
            .optional()?
            .unwrap_or_else(|| TaskEconomyProjectSetting {
                project_id: project_id.trim().to_string(),
                enabled: false,
                shadow_only: true,
                updated_by_user_id: None,
                updated_at: None,
            }))
    }

    pub(crate) fn set_task_economy_project_enabled(
        &self,
        project_id: &str,
        actor_user_id: &str,
        enabled: bool,
    ) -> Result<TaskEconomyProjectSetting> {
        let timestamp = now();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO task_economy_project_settings (
               project_id, enabled, shadow_only, updated_by_user_id, updated_at
             ) VALUES (?1, ?2, 1, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET
               enabled = excluded.enabled,
               shadow_only = 1,
               updated_by_user_id = excluded.updated_by_user_id,
               updated_at = excluded.updated_at",
            params![
                project_id.trim(),
                if enabled { 1 } else { 0 },
                actor_user_id.trim(),
                timestamp
            ],
        )?;
        drop(conn);
        self.task_economy_project_setting(project_id)
    }

    pub(crate) fn insert_task_usage_receipt(
        &self,
        input: CreateUsageReceipt<'_>,
    ) -> Result<UsageReceipt> {
        validate_amounts(
            input.amount_micros,
            input.provider_amount_micros,
            "用量凭证",
        )?;
        let timestamp = now();
        let id = new_id("usage");
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO task_usage_receipts (
               id, project_id, subject_type, subject_id, source_type, source_id,
               source_digest, consumer_user_id, provider_user_id, units,
               amount_micros, provider_amount_micros, currency, billing_source,
               source_status, occurred_at, created_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
               ?14, ?15, ?16, ?17
             )",
            params![
                id,
                input.project_id.trim(),
                input.subject_type.trim(),
                input.subject_id.trim(),
                input.source_type.trim(),
                input.source_id.trim(),
                input.source_digest.trim(),
                input.consumer_user_id.trim(),
                clean(input.provider_user_id),
                input.units.max(0),
                input.amount_micros,
                input.provider_amount_micros,
                input.currency.trim(),
                input.billing_source.trim(),
                input.source_status.trim(),
                input.occurred_at.trim(),
                timestamp
            ],
        )?;
        let receipt =
            select_usage_by_source(&conn, input.project_id, input.source_type, input.source_id)?
                .ok_or_else(|| anyhow!("用量凭证写入后无法读取"))?;
        if receipt.source_digest != input.source_digest.trim() {
            bail!("同一用量来源对应的事实摘要发生冲突");
        }
        Ok(receipt)
    }

    pub(crate) fn create_task_settlement_intent(
        &self,
        input: CreateSettlementIntent<'_>,
    ) -> Result<SettlementIntent> {
        let timestamp = now();
        let id = new_id("intent");
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT OR IGNORE INTO task_settlement_intents (
               id, project_id, matter_id, assignment_id, payer_user_id, payee_user_id,
               idempotency_key, policy_version, policy_digest, status, shadow_only,
               created_at, updated_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', 1, ?10, ?10
             )",
            params![
                id,
                input.project_id.trim(),
                clean(input.matter_id),
                clean(input.assignment_id),
                input.payer_user_id.trim(),
                clean(input.payee_user_id),
                input.idempotency_key.trim(),
                input.policy_version.trim(),
                input.policy_digest.trim(),
                timestamp
            ],
        )?;
        let intent = select_intent_by_key(&tx, input.project_id, input.idempotency_key)?
            .ok_or_else(|| anyhow!("影子结算意图写入后无法读取"))?;
        if intent.policy_digest != input.policy_digest.trim() {
            bail!("同一结算幂等键对应的策略摘要发生冲突");
        }
        tx.execute(
            "INSERT OR IGNORE INTO task_settlement_intent_sources (
               intent_id, usage_receipt_id, created_at
             ) VALUES (?1, ?2, ?3)",
            params![intent.id, input.usage_receipt_id.trim(), timestamp],
        )?;
        tx.commit()?;
        Ok(intent)
    }

    pub(crate) fn list_task_usage_receipts(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<UsageReceipt>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            usage_select()
        ))?;
        let rows = stmt.query_map(
            params![project_id.trim(), limit.clamp(1, 500) as i64],
            read_usage,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn list_task_settlement_intents(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<SettlementIntent>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            intent_select()
        ))?;
        let rows = stmt.query_map(
            params![project_id.trim(), limit.clamp(1, 500) as i64],
            read_intent,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn list_task_settlement_intents_for_matter(
        &self,
        project_id: &str,
        matter_id: &str,
    ) -> Result<Vec<SettlementIntent>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 AND matter_id = ?2 ORDER BY created_at",
            intent_select()
        ))?;
        let rows = stmt.query_map(params![project_id.trim(), matter_id.trim()], read_intent)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn task_usage_receipts_for_intent(
        &self,
        intent_id: &str,
    ) -> Result<Vec<UsageReceipt>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} JOIN task_settlement_intent_sources source ON source.usage_receipt_id = receipt.id
                WHERE source.intent_id = ?1 ORDER BY receipt.created_at",
            usage_select_alias()
        ))?;
        let rows = stmt.query_map(params![intent_id.trim()], read_usage)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn list_task_settlement_receipts(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<SettlementReceipt>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{} WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            settlement_select()
        ))?;
        let rows = stmt.query_map(
            params![project_id.trim(), limit.clamp(1, 500) as i64],
            read_settlement,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub(crate) fn post_task_shadow_settlement(
        &self,
        input: CreateSettlementReceipt<'_>,
        postings: &[LedgerPosting],
    ) -> Result<SettlementReceipt> {
        validate_amounts(
            input.compute_amount_micros,
            input.provider_amount_micros,
            "影子结算",
        )?;
        if input.platform_amount_micros
            != input.compute_amount_micros - input.provider_amount_micros
        {
            bail!("平台影子金额必须等于真实成本减节点收益");
        }
        ensure_balanced(postings)?;

        let timestamp = now();
        let receipt_id = new_id("settlement");
        let transaction_id = new_id("ledger");
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        if let Some(existing) = select_settlement_by_intent(&tx, input.intent_id)? {
            ensure_same_settlement(&existing, &input)?;
            tx.commit()?;
            return Ok(existing);
        }
        let intent = select_intent_by_id(&tx, input.intent_id)?
            .ok_or_else(|| anyhow!("影子结算意图不存在"))?;
        if intent.project_id != input.project_id {
            bail!("影子结算意图不属于当前项目");
        }
        if intent.status == INTENT_VOIDED {
            bail!("已作废的影子结算意图不能过账");
        }

        tx.execute(
            "INSERT INTO task_settlement_receipts (
               id, project_id, intent_id, posting_key, status,
               compute_amount_micros, provider_amount_micros, platform_amount_micros,
               outcome_reward_micros, review_reward_micros, currency, shadow_only,
               accepted_matter_id, reason, created_at
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?13, ?14
             )",
            params![
                receipt_id,
                input.project_id.trim(),
                input.intent_id.trim(),
                input.posting_key.trim(),
                input.status.trim(),
                input.compute_amount_micros,
                input.provider_amount_micros,
                input.platform_amount_micros,
                input.outcome_reward_micros.max(0),
                input.review_reward_micros.max(0),
                input.currency.trim(),
                clean(input.accepted_matter_id),
                input.reason.trim(),
                timestamp
            ],
        )?;

        if !postings.is_empty() {
            tx.execute(
                "INSERT INTO task_ledger_transactions (
                   id, project_id, settlement_receipt_id, posting_key, description, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    transaction_id,
                    input.project_id.trim(),
                    receipt_id,
                    input.posting_key.trim(),
                    "accepted task compute cost mirror",
                    timestamp
                ],
            )?;
            for posting in postings {
                tx.execute(
                    "INSERT INTO task_ledger_entries (
                       id, transaction_id, account_key, user_id, side,
                       amount_micros, currency, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        new_id("entry"),
                        transaction_id,
                        posting.account_key,
                        posting.user_id,
                        posting.side,
                        posting.amount_micros,
                        input.currency.trim(),
                        timestamp
                    ],
                )?;
            }
        }
        tx.execute(
            "UPDATE task_settlement_intents
                SET status = ?1, updated_at = ?2
              WHERE id = ?3 AND status = ?4",
            params![
                INTENT_POSTED,
                timestamp,
                input.intent_id.trim(),
                INTENT_PENDING
            ],
        )?;
        let receipt = select_settlement_by_intent(&tx, input.intent_id)?
            .ok_or_else(|| anyhow!("影子结算凭证写入后无法读取"))?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn void_task_settlement_intent(
        &self,
        project_id: &str,
        intent_id: &str,
        reason: &str,
    ) -> Result<SettlementReceipt> {
        let timestamp = now();
        let posting_key = format!("task-shadow-void:v1:{}", intent_id.trim());
        let receipt_id = new_id("settlement");
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        if let Some(existing) = select_settlement_by_intent(&tx, intent_id)? {
            tx.commit()?;
            return Ok(existing);
        }
        let changed = tx.execute(
            "UPDATE task_settlement_intents
                SET status = ?1, updated_at = ?2
              WHERE id = ?3 AND project_id = ?4 AND status = ?5",
            params![
                INTENT_VOIDED,
                timestamp,
                intent_id.trim(),
                project_id.trim(),
                INTENT_PENDING
            ],
        )?;
        if changed == 0 {
            bail!("影子结算意图不存在或已完成");
        }
        tx.execute(
            "INSERT INTO task_settlement_receipts (
               id, project_id, intent_id, posting_key, status,
               compute_amount_micros, provider_amount_micros, platform_amount_micros,
               outcome_reward_micros, review_reward_micros, currency, shadow_only,
               accepted_matter_id, reason, created_at
             ) VALUES (?1, ?2, ?3, ?4, 'voided', 0, 0, 0, 0, 0, 'CNY', 1, NULL, ?5, ?6)",
            params![
                receipt_id,
                project_id.trim(),
                intent_id.trim(),
                posting_key,
                reason.trim(),
                timestamp
            ],
        )?;
        let receipt = select_settlement_by_intent(&tx, intent_id)?
            .ok_or_else(|| anyhow!("作废凭证写入后无法读取"))?;
        tx.commit()?;
        Ok(receipt)
    }

    pub(crate) fn task_settlement_receipt(
        &self,
        project_id: &str,
        receipt_id: &str,
    ) -> Result<Option<SettlementReceipt>> {
        let conn = self.conn()?;
        conn.query_row(
            &format!("{} WHERE project_id = ?1 AND id = ?2", settlement_select()),
            params![project_id.trim(), receipt_id.trim()],
            read_settlement,
        )
        .optional()
        .map_err(Into::into)
    }

    pub(crate) fn task_settlement_intent(
        &self,
        intent_id: &str,
    ) -> Result<Option<SettlementIntent>> {
        let conn = self.conn()?;
        select_intent_by_id(&conn, intent_id)
    }

    pub(crate) fn task_ledger_transaction_for_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<LedgerTransaction>> {
        let conn = self.conn()?;
        let Some(mut transaction) = conn
            .query_row(
                "SELECT id, project_id, settlement_receipt_id, posting_key,
                        description, created_at
                   FROM task_ledger_transactions WHERE settlement_receipt_id = ?1",
                params![receipt_id.trim()],
                |row| {
                    Ok(LedgerTransaction {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        settlement_receipt_id: row.get(2)?,
                        posting_key: row.get(3)?,
                        description: row.get(4)?,
                        created_at: row.get(5)?,
                        entries: Vec::new(),
                    })
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut stmt = conn.prepare(
            "SELECT id, transaction_id, account_key, user_id, side,
                    amount_micros, currency, created_at
               FROM task_ledger_entries WHERE transaction_id = ?1 ORDER BY id",
        )?;
        transaction.entries = stmt
            .query_map(params![transaction.id], |row| {
                Ok(LedgerEntry {
                    id: row.get(0)?,
                    transaction_id: row.get(1)?,
                    account_key: row.get(2)?,
                    user_id: row.get(3)?,
                    side: row.get(4)?,
                    amount_micros: row.get(5)?,
                    currency: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(transaction))
    }
}

fn validate_amounts(total: i64, provider: i64, label: &str) -> Result<()> {
    if total < 0 || provider < 0 || provider > total {
        bail!("{label}金额无效");
    }
    Ok(())
}

fn clean(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
