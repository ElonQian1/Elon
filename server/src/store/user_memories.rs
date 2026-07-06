//! 用户长期记忆存储。
//!
//! 每个用户在每个作用域最多保存 MAX_MEMORIES_PER_SCOPE 条记忆；超出时自动淘汰
//! importance 最低的条目。
//! 记忆在对话前注入系统提示词，由 `agent_api_loop` 调用；提取由 `user_memory_extract` 异步完成。

use anyhow::Result;
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use super::{new_id, now, Store};

const MAX_MEMORIES_PER_SCOPE: i64 = 50;

pub const MEMORY_SCOPE_GLOBAL: &str = "global";
pub const MEMORY_SCOPE_PHONE_CONTROL: &str = "phone_control";
pub const MEMORY_SCOPE_CHAT: &str = "chat_memory";
pub const MEMORY_SCOPE_PROJECT: &str = "project";

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
    /// 作用域类型：global / phone_control / chat_memory / project
    pub scope_type: String,
    /// 作用域 ID：project 作用域时为 project_id；global 可为空
    pub scope_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Store 方法 ────────────────────────────────────────────────────────────────

impl Store {
    /// 取某用户 importance 最高的前 N 条记忆（用于注入对话上下文）。
    pub fn get_user_memories(&self, user_id: &str, limit: i64) -> Result<Vec<UserMemory>> {
        self.get_user_memories_for_scope(user_id, MEMORY_SCOPE_GLOBAL, None, limit)
    }

    /// 取指定作用域的记忆，同时包含全局记忆。
    pub fn get_user_memories_for_scope(
        &self,
        user_id: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<UserMemory>> {
        let conn = self.conn()?;
        let scope_type = normalize_scope_type(scope_type);
        let scope_id = normalize_scope_id_for_type(&scope_type, scope_id);
        let mut stmt = conn.prepare(
            "SELECT id, user_id, content, category, importance, source_conv_id,
                    scope_type, scope_id, created_at, updated_at
             FROM user_memories
             WHERE user_id = ?1
               AND (
                 scope_type = 'global'
                 OR (
                   scope_type = ?2
                   AND ((?3 IS NULL AND scope_id IS NULL) OR scope_id = ?3)
                 )
               )
             ORDER BY importance DESC, updated_at DESC
             LIMIT ?4",
        )?;
        let rows = stmt.query_map(
            params![user_id, scope_type, scope_id.as_deref(), limit],
            map_memory_row,
        )?;
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
            "SELECT id, user_id, content, category, importance, source_conv_id,
                    scope_type, scope_id, created_at, updated_at
             FROM user_memories
             WHERE user_id = ?1
             ORDER BY importance DESC, updated_at DESC
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![user_id, page_size, offset], map_memory_row)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// 分页列出某用户在指定作用域内的记忆（不包含 global 合并）。
    pub fn list_user_memories_for_scope(
        &self,
        user_id: &str,
        scope_type: &str,
        scope_id: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<UserMemory>> {
        let conn = self.conn()?;
        let scope_type = normalize_scope_type(scope_type);
        let scope_id = normalize_scope_id_for_type(&scope_type, scope_id);
        let offset = (page.max(1) - 1) * page_size;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, content, category, importance, source_conv_id,
                    scope_type, scope_id, created_at, updated_at
             FROM user_memories
             WHERE user_id = ?1
               AND scope_type = ?2
               AND ((?3 IS NULL AND scope_id IS NULL) OR scope_id = ?3)
             ORDER BY importance DESC, updated_at DESC
             LIMIT ?4 OFFSET ?5",
        )?;
        let rows = stmt.query_map(
            params![user_id, scope_type, scope_id.as_deref(), page_size, offset],
            map_memory_row,
        )?;
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
        self.insert_user_memory_scoped(
            user_id,
            content,
            category,
            importance,
            source_conv_id,
            MEMORY_SCOPE_GLOBAL,
            None,
        )
    }

    /// 插入一条指定作用域的新记忆，并自动淘汰该作用域内超出上限的条目。
    pub fn insert_user_memory_scoped(
        &self,
        user_id: &str,
        content: &str,
        category: &str,
        importance: i64,
        source_conv_id: Option<&str>,
        scope_type: &str,
        scope_id: Option<&str>,
    ) -> Result<()> {
        let importance = importance.clamp(1, 10);
        let scope_type = normalize_scope_type(scope_type);
        let scope_id = normalize_scope_id_for_type(&scope_type, scope_id);
        let conn = self.conn()?;
        let id = new_id("mem");
        let ts = now();
        conn.execute(
            "INSERT INTO user_memories
             (id, user_id, content, category, importance, source_conv_id,
              scope_type, scope_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                id,
                user_id,
                content,
                category,
                importance,
                source_conv_id,
                scope_type,
                scope_id.as_deref(),
                ts
            ],
        )?;
        // 超出上限时只淘汰同一作用域内的低优先级条目，避免项目记忆挤掉全局记忆。
        conn.execute(
            "DELETE FROM user_memories
             WHERE user_id = ?1
               AND scope_type = ?2
               AND ((?3 IS NULL AND scope_id IS NULL) OR scope_id = ?3)
               AND id NOT IN (
                 SELECT id FROM user_memories
                 WHERE user_id = ?1
                   AND scope_type = ?2
                   AND ((?3 IS NULL AND scope_id IS NULL) OR scope_id = ?3)
                 ORDER BY importance DESC, updated_at DESC
                 LIMIT ?4
               )",
            params![
                user_id,
                scope_type,
                scope_id.as_deref(),
                MAX_MEMORIES_PER_SCOPE
            ],
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

fn map_memory_row(row: &Row<'_>) -> rusqlite::Result<UserMemory> {
    Ok(UserMemory {
        id: row.get(0)?,
        user_id: row.get(1)?,
        content: row.get(2)?,
        category: row.get(3)?,
        importance: row.get(4)?,
        source_conv_id: row.get(5)?,
        scope_type: row.get(6)?,
        scope_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn normalize_scope_type(scope_type: &str) -> String {
    let scope_type = scope_type.trim();
    if scope_type.is_empty() {
        MEMORY_SCOPE_GLOBAL.to_string()
    } else {
        scope_type.to_string()
    }
}

fn normalize_scope_id_for_type(scope_type: &str, scope_id: Option<&str>) -> Option<String> {
    if scope_type == MEMORY_SCOPE_GLOBAL {
        None
    } else {
        normalize_scope_id(scope_id)
    }
}

fn normalize_scope_id(scope_id: Option<&str>) -> Option<String> {
    scope_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}


#[cfg(test)]
#[path = "user_memories_tests.rs"]
mod tests;
