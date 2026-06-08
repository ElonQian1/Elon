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
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub charged_credits: f64,
    pub settled_credits: f64,
    pub platform_fee_rate: f64,
    pub created_at: String,
}

/// 参数：一次 LLM 推理完成后调用，执行原子扣费+结算。
pub struct SettleParams<'a> {
    pub consumer_user_id: &'a str,
    pub provider_user_id: &'a str,
    pub node_id: &'a str,
    pub model_id: &'a str,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// 每 1k tokens 的价格（积分）
    pub price_per_1k_credits: f64,
    /// 平台抽成比例（默认 0.2）
    pub platform_fee_rate: f64,
}

/// 节点凭证（不含 secret_hash，仅供 API 展示）
#[derive(Debug, Serialize)]
pub struct NodeCredential {
    pub agent_id: String,
    pub owner_user_id: String,
    pub label: String,
    pub created_at: String,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 获取节点提供者的积分余额，不存在则返回 0.0。
    pub fn get_node_balance(&self, user_id: &str) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let balance: f64 = conn
            .query_row(
                "SELECT credits FROM node_balances WHERE user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        Ok(balance)
    }

    /// 查询某提供者用户的累计历史总收益（SUM settled_credits）。
    pub fn get_lifetime_earned(&self, user_id: &str) -> Result<f64> {
        let conn = self.conn.lock().unwrap();
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(settled_credits), 0.0) FROM node_transactions WHERE provider_user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .unwrap_or(0.0);
        Ok(total)
    }

    /// 结算一次 LLM 推理：
    /// 1. 计算消费金额和提供者收益
    /// 2. 写入 node_transactions 流水
    /// 3. 更新 node_balances 提供者余额
    ///
    /// 注意：消费者 RMB 余额由 `node_router` 的统一 token 结算入口扣减；
    /// 本函数只负责节点积分流水和提供者收益。
    pub fn settle_node_inference(&self, p: SettleParams<'_>) -> Result<NodeTransaction> {
        let total_tokens = p.prompt_tokens + p.completion_tokens;
        let charged = (total_tokens as f64 / 1000.0) * p.price_per_1k_credits;
        let settled = charged * (1.0 - p.platform_fee_rate);

        let tx_id = new_id("ntx");
        let ts = now();

        let conn = self.conn.lock().unwrap();

        // 写入流水
        conn.execute(
            "INSERT INTO node_transactions
             (id, consumer_user_id, provider_user_id, node_id, model_id,
              prompt_tokens, completion_tokens, charged_credits, settled_credits,
              platform_fee_rate, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                tx_id,
                p.consumer_user_id,
                p.provider_user_id,
                p.node_id,
                p.model_id,
                p.prompt_tokens as i64,
                p.completion_tokens as i64,
                charged,
                settled,
                p.platform_fee_rate,
                ts
            ],
        )?;

        // 累加提供者余额（UPSERT）
        conn.execute(
            "INSERT INTO node_balances (user_id, credits, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id) DO UPDATE SET
               credits    = credits + excluded.credits,
               updated_at = excluded.updated_at",
            params![p.provider_user_id, settled, ts],
        )?;

        Ok(NodeTransaction {
            id: tx_id,
            consumer_user_id: p.consumer_user_id.to_string(),
            provider_user_id: p.provider_user_id.to_string(),
            node_id: p.node_id.to_string(),
            model_id: p.model_id.to_string(),
            prompt_tokens: p.prompt_tokens as i64,
            completion_tokens: p.completion_tokens as i64,
            charged_credits: charged,
            settled_credits: settled,
            platform_fee_rate: p.platform_fee_rate,
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
        let mut stmt = conn.prepare(
            "SELECT id, consumer_user_id, provider_user_id, node_id, model_id,
                    prompt_tokens, completion_tokens, charged_credits, settled_credits,
                    platform_fee_rate, created_at
             FROM node_transactions
             WHERE provider_user_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![provider_user_id, limit], |row| {
            Ok(NodeTransaction {
                id: row.get(0)?,
                consumer_user_id: row.get(1)?,
                provider_user_id: row.get(2)?,
                node_id: row.get(3)?,
                model_id: row.get(4)?,
                prompt_tokens: row.get(5)?,
                completion_tokens: row.get(6)?,
                charged_credits: row.get(7)?,
                settled_credits: row.get(8)?,
                platform_fee_rate: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?;
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

    /// 列出某用户注册的所有节点凭证（不含 secret_hash）。
    pub fn list_node_credentials(&self, owner_user_id: &str) -> Result<Vec<NodeCredential>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT agent_id, owner_user_id, label, created_at
             FROM node_credentials WHERE owner_user_id = ?1
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(params![owner_user_id], |row| {
            Ok(NodeCredential {
                agent_id: row.get(0)?,
                owner_user_id: row.get(1)?,
                label: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}
