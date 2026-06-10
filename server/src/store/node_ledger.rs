//! 节点积分账本：积分余额查询、扣费、结算；节点凭证管理。
//!
//! 三张表（由 migration_v21 创建）：
//! - `node_balances`      每用户的提供者积分余额
//! - `node_transactions`  每次 LLM 推理完成后的完整流水记录
//! - `node_credentials`   用户注册的 PC 节点凭证（存 SHA-256 hash）

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, Store};

// ── 公共数据结构 ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NodeBalance {
    pub user_id: String,
    pub credits: f64,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct NodeTransaction {
    pub id: String,
    pub consumer_user_id: String,
    pub provider_user_id: String,
    pub node_id: String,
    pub model_id: String,
    pub feature: Option<String>,
    pub usage_mode: Option<String>,
    pub compute_call_id: Option<String>,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub charged_credits: f64,
    pub settled_credits: f64,
    pub platform_fee_rate: f64,
    pub billed_cost_rmb_fen: i64,
    pub provider_earned_fen: i64,
    pub provider_revenue_share_x1000: i64,
    pub settlement_status: String,
    pub created_at: String,
}

/// 参数：一次 LLM 推理完成后调用，执行原子扣费+结算。
pub struct SettleParams<'a> {
    pub consumer_user_id: &'a str,
    pub provider_user_id: &'a str,
    pub node_id: &'a str,
    pub model_id: &'a str,
    pub feature: &'a str,
    pub usage_mode: &'a str,
    pub compute_call_id: Option<&'a str>,
    pub token_usage_event_id: Option<&'a str>,
    pub billing_event_id: Option<&'a str>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// 兼容旧节点市场展示的每 1k tokens 价格（积分）。
    pub price_per_1k_credits: f64,
    /// 真实消费者账单金额（人民币分）。提供者收益只从 billed 状态的真实扣费中分账。
    pub billed_cost_rmb_fen: i64,
    pub accounting_status: Option<&'a str>,
    /// 提供者分账比例 × 1000（800 = 80%）。
    pub provider_revenue_share_x1000: i64,
    /// 平台抽成比例（兼容旧字段展示，默认 0.2）。
    pub platform_fee_rate: f64,
}

/// 节点凭证（不含 secret_hash，仅供 API 展示）
#[derive(Debug, Serialize)]
pub struct NodeCredential {
    pub agent_id: String,
    pub owner_user_id: String,
    pub label: String,
    pub device_name: Option<String>,
    pub created_at: String,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 获取节点提供者的积分余额，不存在则返回 0.0。
    pub fn get_node_balance(&self, user_id: &str) -> Result<f64> {
        Ok(fen_to_credits(self.get_node_balance_fen(user_id)?))
    }

    /// 获取节点提供者当前可提现余额（人民币分），用于资金对账。
    pub fn get_node_balance_fen(&self, user_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let balance_fen: i64 = conn
            .query_row(
                "SELECT available_fen FROM node_balances WHERE user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(balance_fen.max(0))
    }

    /// 查询某提供者用户的累计历史总收益。
    pub fn get_lifetime_earned(&self, user_id: &str) -> Result<f64> {
        Ok(fen_to_credits(self.get_lifetime_earned_fen(user_id)?))
    }

    /// 查询某提供者用户的累计历史总收益（人民币分）。
    pub fn get_lifetime_earned_fen(&self, user_id: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let total_fen: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(provider_earned_fen), 0)
                 FROM node_transactions
                 WHERE provider_user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(total_fen.max(0))
    }

    /// 结算一次 LLM 推理：
    /// 1. 绑定真实 token/billing 事件，按实际扣费金额计算提供者收益
    /// 2. 写入 node_transactions 流水
    /// 3. 更新 node_balances 提供者余额
    ///
    /// 注意：消费者 RMB 余额由统一 token 结算入口扣减；本函数只负责节点收益流水。
    pub fn settle_node_inference(&self, p: SettleParams<'_>) -> Result<NodeTransaction> {
        let total_tokens = p.prompt_tokens + p.completion_tokens;
        let _fallback_charged = (total_tokens as f64 / 1000.0) * p.price_per_1k_credits;
        let status = settlement_status(
            p.accounting_status,
            p.billing_event_id,
            p.billed_cost_rmb_fen,
        );
        let revenue_share = p.provider_revenue_share_x1000.clamp(0, 1000);
        let billed_cost_fen = p.billed_cost_rmb_fen.max(0);
        let provider_earned_fen = if status == "billed" {
            provider_earned_fen(billed_cost_fen, revenue_share)
        } else {
            0
        };
        let charged = if status == "billed" {
            fen_to_credits(billed_cost_fen)
        } else {
            0.0
        };
        let settled = fen_to_credits(provider_earned_fen);

        let tx_id = new_id("ntx");
        let ts = now();

        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        if let Some(token_event_id) = normalize_optional(p.token_usage_event_id) {
            let existing = tx
                .query_row(
                    node_transaction_select_sql(
                        "WHERE token_usage_event_id = ?1 ORDER BY created_at DESC LIMIT 1",
                    )
                    .as_str(),
                    params![token_event_id],
                    read_node_transaction,
                )
                .optional()?;
            if let Some(existing) = existing {
                tx.commit()?;
                return Ok(existing);
            }
        }

        // 写入流水
        tx.execute(
            "INSERT INTO node_transactions
             (id, consumer_user_id, provider_user_id, node_id, model_id,
              feature, usage_mode, compute_call_id, token_usage_event_id, billing_event_id,
              prompt_tokens, completion_tokens, charged_credits, settled_credits,
              platform_fee_rate, billed_cost_rmb_fen, provider_earned_fen,
              provider_revenue_share_x1000, settlement_status, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                tx_id,
                p.consumer_user_id,
                p.provider_user_id,
                p.node_id,
                p.model_id,
                p.feature,
                p.usage_mode,
                normalize_optional(p.compute_call_id),
                normalize_optional(p.token_usage_event_id),
                normalize_optional(p.billing_event_id),
                p.prompt_tokens as i64,
                p.completion_tokens as i64,
                charged,
                settled,
                p.platform_fee_rate,
                billed_cost_fen,
                provider_earned_fen,
                revenue_share,
                status,
                ts
            ],
        )?;

        if provider_earned_fen > 0 {
            // 累加提供者余额（UPSERT）。
            tx.execute(
                "INSERT INTO node_balances (user_id, credits, available_fen, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(user_id) DO UPDATE SET
                   available_fen = node_balances.available_fen + excluded.available_fen,
                   credits       = (node_balances.available_fen + excluded.available_fen) / 100.0,
                   updated_at    = excluded.updated_at",
                params![p.provider_user_id, settled, provider_earned_fen, ts],
            )?;
        }
        tx.commit()?;

        Ok(NodeTransaction {
            id: tx_id,
            consumer_user_id: p.consumer_user_id.to_string(),
            provider_user_id: p.provider_user_id.to_string(),
            node_id: p.node_id.to_string(),
            model_id: p.model_id.to_string(),
            feature: Some(p.feature.to_string()),
            usage_mode: Some(p.usage_mode.to_string()),
            compute_call_id: normalize_optional(p.compute_call_id).map(ToOwned::to_owned),
            token_usage_event_id: normalize_optional(p.token_usage_event_id).map(ToOwned::to_owned),
            billing_event_id: normalize_optional(p.billing_event_id).map(ToOwned::to_owned),
            prompt_tokens: p.prompt_tokens as i64,
            completion_tokens: p.completion_tokens as i64,
            charged_credits: charged,
            settled_credits: settled,
            platform_fee_rate: p.platform_fee_rate,
            billed_cost_rmb_fen: billed_cost_fen,
            provider_earned_fen,
            provider_revenue_share_x1000: revenue_share,
            settlement_status: status,
            created_at: ts,
        })
    }

    /// 查询某提供者用户最近 N 条流水（按时间倒序）。
    pub fn list_node_transactions(
        &self,
        provider_user_id: &str,
        limit: i64,
    ) -> Result<Vec<NodeTransaction>> {
        let conn = self.conn.lock().unwrap();
        let sql = node_transaction_select_sql(
            "WHERE provider_user_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![provider_user_id, limit], read_node_transaction)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// 注册一个新节点凭证（存 secret 的 SHA-256 hash，不存明文）。
    pub fn create_node_credential(
        &self,
        agent_id: &str,
        secret_hash: &str,
        owner_user_id: &str,
        label: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO node_credentials (agent_id, secret_hash, owner_user_id, label, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                agent_id,
                secret_hash,
                owner_user_id,
                label.unwrap_or(""),
                now()
            ],
        )?;
        Ok(())
    }

    /// 查询节点凭证的 secret_hash（用于鉴权）。
    pub fn get_node_credential_hash(&self, agent_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let hash = conn
            .query_row(
                "SELECT secret_hash FROM node_credentials WHERE agent_id = ?1",
                params![agent_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(hash)
    }

    /// 查询节点凭证所属的 owner_user_id。
    pub fn get_node_credential_owner(&self, agent_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let owner = conn
            .query_row(
                "SELECT owner_user_id FROM node_credentials WHERE agent_id = ?1",
                params![agent_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(owner)
    }

    /// 回填节点设备名。设备名来自 PC 系统名，和用户自定义 label 分开保存。
    pub fn update_node_credential_device_name(
        &self,
        agent_id: &str,
        owner_user_id: &str,
        device_name: &str,
    ) -> Result<()> {
        let device_name = device_name.trim();
        if device_name.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE node_credentials
             SET device_name = ?3
             WHERE agent_id = ?1
               AND owner_user_id = ?2",
            params![agent_id, owner_user_id, device_name],
        )?;
        Ok(())
    }

    /// 列出某用户注册的所有节点凭证（不含 secret_hash）。
    pub fn list_node_credentials(&self, owner_user_id: &str) -> Result<Vec<NodeCredential>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT agent_id, owner_user_id, label, device_name, created_at
             FROM node_credentials WHERE owner_user_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![owner_user_id], |row| {
            Ok(NodeCredential {
                agent_id: row.get(0)?,
                owner_user_id: row.get(1)?,
                label: row.get(2)?,
                device_name: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn node_transaction_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT id, consumer_user_id, provider_user_id, node_id, model_id,
                feature, usage_mode, compute_call_id, token_usage_event_id, billing_event_id,
                prompt_tokens, completion_tokens, charged_credits, settled_credits,
                platform_fee_rate, billed_cost_rmb_fen, provider_earned_fen,
                provider_revenue_share_x1000, settlement_status, created_at
         FROM node_transactions {where_clause}"
    )
}

fn read_node_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeTransaction> {
    Ok(NodeTransaction {
        id: row.get(0)?,
        consumer_user_id: row.get(1)?,
        provider_user_id: row.get(2)?,
        node_id: row.get(3)?,
        model_id: row.get(4)?,
        feature: row.get(5)?,
        usage_mode: row.get(6)?,
        compute_call_id: row.get(7)?,
        token_usage_event_id: row.get(8)?,
        billing_event_id: row.get(9)?,
        prompt_tokens: row.get(10)?,
        completion_tokens: row.get(11)?,
        charged_credits: row.get(12)?,
        settled_credits: row.get(13)?,
        platform_fee_rate: row.get(14)?,
        billed_cost_rmb_fen: row.get(15)?,
        provider_earned_fen: row.get(16)?,
        provider_revenue_share_x1000: row.get(17)?,
        settlement_status: row.get(18)?,
        created_at: row.get(19)?,
    })
}

fn normalize_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn settlement_status(
    accounting_status: Option<&str>,
    billing_event_id: Option<&str>,
    cost_fen: i64,
) -> String {
    let accounting_status = accounting_status
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("missing_accounting");
    if accounting_status == "billed"
        && normalize_optional(billing_event_id).is_some()
        && cost_fen > 0
    {
        "billed".to_string()
    } else {
        accounting_status.to_string()
    }
}

fn provider_earned_fen(billed_cost_fen: i64, share_x1000: i64) -> i64 {
    if billed_cost_fen <= 0 || share_x1000 <= 0 {
        return 0;
    }
    let earned = ((billed_cost_fen as i128 * share_x1000 as i128) + 500) / 1000;
    earned.clamp(0, billed_cost_fen as i128) as i64
}

fn fen_to_credits(fen: i64) -> f64 {
    fen.max(0) as f64 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-ledger-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        (Store::open(&path).expect("store should open"), path)
    }

    #[test]
    fn settlement_uses_real_billed_cost_and_is_idempotent() {
        let (store, path) = temp_store();
        let consumer = store
            .create_user("node-ledger-consumer@example.com", "secret1", None, None)
            .unwrap();
        let provider = store
            .create_user("node-ledger-provider@example.com", "secret1", None, None)
            .unwrap();

        let params = SettleParams {
            consumer_user_id: &consumer.id,
            provider_user_id: &provider.id,
            node_id: "node-a",
            model_id: "pc-cli/codex",
            feature: "pc_agent_cli_dev",
            usage_mode: "pc_agent_cli",
            compute_call_id: Some("pc_agent_cli:req-1"),
            token_usage_event_id: Some("tok-real-1"),
            billing_event_id: Some("bev-real-1"),
            prompt_tokens: 400,
            completion_tokens: 600,
            price_per_1k_credits: 99.0,
            billed_cost_rmb_fen: 123,
            accounting_status: Some("billed"),
            provider_revenue_share_x1000: 800,
            platform_fee_rate: 0.2,
        };

        let first = store.settle_node_inference(params).unwrap();
        let second = store
            .settle_node_inference(SettleParams {
                consumer_user_id: &consumer.id,
                provider_user_id: &provider.id,
                node_id: "node-a",
                model_id: "pc-cli/codex",
                feature: "pc_agent_cli_dev",
                usage_mode: "pc_agent_cli",
                compute_call_id: Some("pc_agent_cli:req-1"),
                token_usage_event_id: Some("tok-real-1"),
                billing_event_id: Some("bev-real-1"),
                prompt_tokens: 400,
                completion_tokens: 600,
                price_per_1k_credits: 99.0,
                billed_cost_rmb_fen: 123,
                accounting_status: Some("billed"),
                provider_revenue_share_x1000: 800,
                platform_fee_rate: 0.2,
            })
            .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(first.charged_credits, 1.23);
        assert_eq!(first.provider_earned_fen, 98);
        assert_eq!(first.settled_credits, 0.98);
        assert_eq!(first.billing_event_id.as_deref(), Some("bev-real-1"));
        assert_eq!(first.token_usage_event_id.as_deref(), Some("tok-real-1"));
        assert_eq!(store.get_node_balance_fen(&provider.id).unwrap(), 98);
        assert_eq!(store.get_node_balance(&provider.id).unwrap(), 0.98);
        assert_eq!(store.get_lifetime_earned_fen(&provider.id).unwrap(), 98);
        assert_eq!(store.get_lifetime_earned(&provider.id).unwrap(), 0.98);

        let txs = store.list_node_transactions(&provider.id, 10).unwrap();
        assert_eq!(txs.len(), 1);

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unbilled_usage_does_not_increase_provider_balance() {
        let (store, path) = temp_store();
        let consumer = store
            .create_user(
                "node-ledger-unbilled-consumer@example.com",
                "secret1",
                None,
                None,
            )
            .unwrap();
        let provider = store
            .create_user(
                "node-ledger-unbilled-provider@example.com",
                "secret1",
                None,
                None,
            )
            .unwrap();

        let tx = store
            .settle_node_inference(SettleParams {
                consumer_user_id: &consumer.id,
                provider_user_id: &provider.id,
                node_id: "node-b",
                model_id: "local/qwen",
                feature: "node_llm",
                usage_mode: "server_node_llm",
                compute_call_id: Some("node_llm:req-2"),
                token_usage_event_id: Some("tok-unbilled-1"),
                billing_event_id: None,
                prompt_tokens: 500,
                completion_tokens: 500,
                price_per_1k_credits: 99.0,
                billed_cost_rmb_fen: 0,
                accounting_status: Some("unbilled_no_balance"),
                provider_revenue_share_x1000: 800,
                platform_fee_rate: 0.2,
            })
            .unwrap();

        assert_eq!(tx.settlement_status, "unbilled_no_balance");
        assert_eq!(tx.provider_earned_fen, 0);
        assert_eq!(tx.settled_credits, 0.0);
        assert_eq!(store.get_node_balance(&provider.id).unwrap(), 0.0);

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
