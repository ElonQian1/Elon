//! 用户长期记忆存储。
//!
//! 每个用户最多保存 MAX_MEMORIES_PER_USER 条记忆；超出时自动淘汰 importance 最低的条目。
//! 记忆在对话前注入系统提示词，由 `agent_api_loop` 调用；提取由 `user_memory_extract` 异步完成。

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{new_id, now, Store};

const MAX_MEMORIES_PER_USER: i64 = 50;

// ── 数据结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMemory {
    pub id: String,
    pub user_id: String,
    /// 记忆正文，例如："用户是 Rust 开发者，偏好完整代码"
    pub content: String,
    /// 分类：preference / profile / goal / fact
    pub category: String,
    /// 重要程度 1–10；越高越优先注入，超限时低分先淘汰
    pub importance: i64,
    /// 来源对话 ID（可选，用于溯源）
    pub source_conv_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 取某用户 importance 最高的前 N 条记忆（用于注入对话上下文）。
    pub fn get_user_memories(&self, user_id: &str, limit: i64) -> Result<Vec<UserMemory>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, content, category, importance, source_conv_id, created_at, updated_at
             FROM user_memories
             WHERE user_id = ?1
             ORDER BY importance DESC, updated_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![user_id, limit], |row| {
            Ok(UserMemory {
                id: row.get(0)?,
                user_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                importance: row.get(4)?,
                source_conv_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 分页列出某用户所有记忆（供用户通过 API 查看）。
    pub fn list_user_memories(
        &self,
        user_id: &str,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<UserMemory>> {
        let conn = self.conn()?;
        let offset = (page.max(1) - 1) * page_size;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, content, category, importance, source_conv_id, created_at, updated_at
             FROM user_memories
             WHERE user_id = ?1
             ORDER BY importance DESC, updated_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![user_id, page_size, offset], |row| {
            Ok(UserMemory {
                id: row.get(0)?,
                user_id: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                importance: row.get(4)?,
                source_conv_id: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 插入一条新记忆，并自动淘汰超出上限的条目。
    pub fn insert_user_memory(
        &self,
        user_id: &str,
        content: &str,
        category: &str,
        importance: i64,
        source_conv_id: Option<&str>,
    ) -> Result<()> {
        let importance = importance.clamp(1, 10);
        let conn = self.conn()?;
        let id = new_id("mem");
        let ts = now();
        conn.execute(
            "INSERT INTO user_memories
             (id, user_id, content, category, importance, source_conv_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![id, user_id, content, category, importance, source_conv_id, ts],
        )?;
        // 超出上限时删最低 importance 的条目（在同一连接内，避免 Mutex 重入）
        conn.execute(
            "DELETE FROM user_memories
             WHERE user_id = ?1
               AND id NOT IN (
                 SELECT id FROM user_memories
                 WHERE user_id = ?1
                 ORDER BY importance DESC, updated_at DESC
                 LIMIT ?2
               )",
            params![user_id, MAX_MEMORIES_PER_USER],
        )?;
        Ok(())
    }

    /// 删除指定记忆，带 user_id 权限校验（防止越权删除他人记忆）。
    /// 返回 true 表示删除成功，false 表示记录不存在或不属于该用户。
    pub fn delete_user_memory(&self, memory_id: &str, user_id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let count = conn.execute(
            "DELETE FROM user_memories WHERE id = ?1 AND user_id = ?2",
            params![memory_id, user_id],
        )?;
        Ok(count > 0)
    }
}
