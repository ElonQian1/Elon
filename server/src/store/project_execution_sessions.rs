//! PC 项目执行会话记录。
//!
//! 这里记录的是「某次项目会话在 PC 节点上的 CLI 执行状态」，用于项目工作区健康页
//! 判断最近一次会话 worktree 是否成功合并、是否仍在运行或失败。

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, task_completion_replay::is_automatic_communication_failure, Store};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectExecutionSession {
    pub id: String,
    pub project_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub node_id: String,
    pub request_id: String,
    pub task_id: Option<String>,
    pub base_workspace_path: Option<String>,
    pub active_workspace_path: Option<String>,
    pub branch: Option<String>,
    pub isolated: bool,
    pub status: String,
    pub merge_status: Option<String>,
    pub last_error: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: i64,
    pub cached_input_tokens: i64,
    pub completion_tokens: i64,
    pub reasoning_tokens: i64,
    pub total_tokens: i64,
    pub token_usage_event_id: Option<String>,
    pub billing_event_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectExecutionSessionStart<'a> {
    pub project_id: &'a str,
    pub conversation_id: &'a str,
    pub user_id: &'a str,
    pub node_id: &'a str,
    pub request_id: &'a str,
    pub requested_workspace_path: Option<&'a str>,
    pub model: Option<&'a str>,
}

pub struct ProjectExecutionSessionFinish<'a> {
    pub request_id: &'a str,
    pub project_id: &'a str,
    pub conversation_id: &'a str,
    pub user_id: &'a str,
    pub node_id: &'a str,
    pub base_workspace_path: Option<&'a str>,
    pub active_workspace_path: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub isolated: bool,
    pub status: &'a str,
    pub merge_status: Option<&'a str>,
    pub last_error: Option<&'a str>,
    pub model: Option<&'a str>,
    pub prompt_tokens: Option<i64>,
    pub cached_input_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub token_usage_event_id: Option<&'a str>,
    pub billing_event_id: Option<&'a str>,
}

impl Store {
    pub fn record_project_execution_started(
        &self,
        session: ProjectExecutionSessionStart<'_>,
    ) -> Result<bool> {
        let now = now();
        let changed = self.conn()?.execute(
            "INSERT INTO project_execution_sessions (
                id, project_id, conversation_id, user_id, node_id, request_id,
                active_workspace_path, isolated, status, model, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'running', ?8, ?9, ?9)
             ON CONFLICT(request_id) DO UPDATE SET
                active_workspace_path = excluded.active_workspace_path,
                status = CASE
                    WHEN project_execution_sessions.status IN ('done','failed','canceled')
                    THEN project_execution_sessions.status
                    ELSE 'running'
                END,
                model = COALESCE(excluded.model, project_execution_sessions.model),
                updated_at = excluded.updated_at
              WHERE project_execution_sessions.project_id = excluded.project_id
                AND project_execution_sessions.conversation_id = excluded.conversation_id
                AND project_execution_sessions.user_id = excluded.user_id
                AND project_execution_sessions.node_id = excluded.node_id",
            params![
                new_id("pes"),
                session.project_id,
                session.conversation_id,
                session.user_id,
                session.node_id,
                session.request_id,
                session.requested_workspace_path,
                session.model,
                now
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn record_project_execution_finished(
        &self,
        session: ProjectExecutionSessionFinish<'_>,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let current = tx
            .query_row(
                "SELECT status, last_error
                   FROM project_execution_sessions
                  WHERE request_id = ?1
                    AND project_id = ?2
                    AND conversation_id = ?3
                    AND user_id = ?4
                    AND node_id = ?5",
                params![
                    session.request_id,
                    session.project_id,
                    session.conversation_id,
                    session.user_id,
                    session.node_id,
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((current_status, current_error)) = current else {
            tx.commit()?;
            return Ok(false);
        };
        let preserve_terminal = match current_status.as_str() {
            "canceled" => true,
            "done" => session.status != "done",
            "failed" if session.status == "done" => {
                !is_automatic_communication_failure(current_error.as_deref().unwrap_or_default())
            }
            _ => false,
        };
        let changed = tx.execute(
            "UPDATE project_execution_sessions
             SET base_workspace_path = ?2,
                 active_workspace_path = COALESCE(?3, active_workspace_path),
                 branch = ?4,
                 isolated = ?5,
                 status = CASE
                     WHEN project_execution_sessions.status = 'canceled' OR EXISTS (
                         SELECT 1 FROM tasks t
                          WHERE t.id = project_execution_sessions.task_id
                            AND t.status = 'canceled'
                      ) THEN 'canceled'
                     WHEN ?18 != 0 THEN project_execution_sessions.status
                     ELSE ?6
                 END,
                 merge_status = CASE
                     WHEN project_execution_sessions.status = 'canceled' OR EXISTS (
                         SELECT 1 FROM tasks t
                          WHERE t.id = project_execution_sessions.task_id
                            AND t.status = 'canceled'
                      ) THEN COALESCE(merge_status, 'canceled')
                     WHEN ?18 != 0 THEN merge_status
                     ELSE ?7
                 END,
                 last_error = CASE
                     WHEN project_execution_sessions.status = 'canceled' OR EXISTS (
                         SELECT 1 FROM tasks t
                          WHERE t.id = project_execution_sessions.task_id
                            AND t.status = 'canceled'
                     ) THEN COALESCE(
                         (SELECT t.error FROM tasks t WHERE t.id = project_execution_sessions.task_id),
                         last_error,
                          '用户已取消任务'
                      )
                     WHEN ?18 != 0 THEN last_error
                     ELSE ?8
                 END,
                 model = COALESCE(?9, model),
                 prompt_tokens = COALESCE(?10, prompt_tokens),
                 cached_input_tokens = COALESCE(?11, cached_input_tokens),
                 completion_tokens = COALESCE(?12, completion_tokens),
                 reasoning_tokens = COALESCE(?13, reasoning_tokens),
                 total_tokens = COALESCE(?14, total_tokens),
                 token_usage_event_id = COALESCE(?15, token_usage_event_id),
                 billing_event_id = COALESCE(?16, billing_event_id),
                 updated_at = ?17
             WHERE request_id = ?1
               AND project_id = ?19
               AND conversation_id = ?20
               AND user_id = ?21
               AND node_id = ?22",
            params![
                session.request_id,
                session.base_workspace_path,
                session.active_workspace_path,
                session.branch,
                session.isolated as i64,
                session.status,
                session.merge_status,
                session.last_error,
                session.model,
                session.prompt_tokens,
                session.cached_input_tokens,
                session.completion_tokens,
                session.reasoning_tokens,
                session.total_tokens,
                session.token_usage_event_id,
                session.billing_event_id,
                now(),
                preserve_terminal as i64,
                session.project_id,
                session.conversation_id,
                session.user_id,
                session.node_id,
            ],
        )?;
        tx.commit()?;
        Ok(changed > 0)
    }

    pub fn mark_interrupted_running_project_execution_sessions(&self) -> Result<usize> {
        let ts = now();
        let n = self.conn()?.execute(
            "UPDATE project_execution_sessions
             SET status = 'failed',
                 merge_status = COALESCE(merge_status, 'interrupted'),
                 last_error = COALESCE(last_error, 'server restarted before PC CLI terminal event'),
                 updated_at = ?1
             WHERE status = 'running'",
            params![ts],
        )?;
        Ok(n)
    }

    /// Bind the transient node request id to the durable cloud task id.
    ///
    /// An existing different binding is never overwritten: replay must not be
    /// able to move an execution receipt between cloud tasks.
    pub fn bind_project_execution_task_id(&self, request_id: &str, task_id: &str) -> Result<bool> {
        let request_id = required_binding_id(request_id, "request_id")?;
        let task_id = required_binding_id(task_id, "task_id")?;
        let changed = self.conn()?.execute(
            "UPDATE project_execution_sessions
                SET task_id = ?2,
                    updated_at = ?3
              WHERE request_id = ?1
                AND (task_id IS NULL OR task_id = ?2)",
            params![request_id, task_id, now()],
        )?;
        Ok(changed > 0)
    }

    pub fn latest_project_execution_session(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectExecutionSession>> {
        self.conn()?
            .query_row(
                "SELECT id, project_id, conversation_id, user_id, node_id, request_id, task_id,
                        base_workspace_path, active_workspace_path, branch, isolated,
                        status, merge_status, last_error, model,
                        prompt_tokens, cached_input_tokens, completion_tokens, reasoning_tokens,
                        total_tokens, token_usage_event_id, billing_event_id,
                        created_at, updated_at
                 FROM project_execution_sessions
                 WHERE project_id = ?1
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![project_id],
                project_execution_session_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn list_project_execution_sessions(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<ProjectExecutionSession>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, conversation_id, user_id, node_id, request_id, task_id,
                    base_workspace_path, active_workspace_path, branch, isolated,
                    status, merge_status, last_error, model,
                    prompt_tokens, cached_input_tokens, completion_tokens, reasoning_tokens,
                    total_tokens, token_usage_event_id, billing_event_id,
                    created_at, updated_at
             FROM project_execution_sessions
             WHERE project_id = ?1
             ORDER BY updated_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![project_id, limit.clamp(1, 200) as i64], |row| {
                project_execution_session_from_row(row)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get_project_execution_session_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Option<ProjectExecutionSession>> {
        let request_id = request_id.trim();
        if request_id.is_empty() {
            return Ok(None);
        }
        self.conn()?
            .query_row(
                "SELECT id, project_id, conversation_id, user_id, node_id, request_id, task_id,
                        base_workspace_path, active_workspace_path, branch, isolated,
                        status, merge_status, last_error, model,
                        prompt_tokens, cached_input_tokens, completion_tokens, reasoning_tokens,
                        total_tokens, token_usage_event_id, billing_event_id,
                        created_at, updated_at
                 FROM project_execution_sessions
                 WHERE request_id = ?1
                 LIMIT 1",
                params![request_id],
                project_execution_session_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_project_execution_session_by_task_id(
        &self,
        task_id: &str,
    ) -> Result<Option<ProjectExecutionSession>> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Ok(None);
        }
        self.conn()?
            .query_row(
                "SELECT id, project_id, conversation_id, user_id, node_id, request_id, task_id,
                        base_workspace_path, active_workspace_path, branch, isolated,
                        status, merge_status, last_error, model,
                        prompt_tokens, cached_input_tokens, completion_tokens, reasoning_tokens,
                        total_tokens, token_usage_event_id, billing_event_id,
                        created_at, updated_at
                 FROM project_execution_sessions
                 WHERE task_id = ?1
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![task_id],
                project_execution_session_from_row,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn project_execution_session_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ProjectExecutionSession> {
    Ok(ProjectExecutionSession {
        id: row.get(0)?,
        project_id: row.get(1)?,
        conversation_id: row.get(2)?,
        user_id: row.get(3)?,
        node_id: row.get(4)?,
        request_id: row.get(5)?,
        task_id: row.get(6)?,
        base_workspace_path: row.get(7)?,
        active_workspace_path: row.get(8)?,
        branch: row.get(9)?,
        isolated: row.get::<_, i64>(10)? != 0,
        status: row.get(11)?,
        merge_status: row.get(12)?,
        last_error: row.get(13)?,
        model: row.get(14)?,
        prompt_tokens: row.get(15)?,
        cached_input_tokens: row.get(16)?,
        completion_tokens: row.get(17)?,
        reasoning_tokens: row.get(18)?,
        total_tokens: row.get(19)?,
        token_usage_event_id: row.get(20)?,
        billing_event_id: row.get(21)?,
        created_at: row.get(22)?,
        updated_at: row.get(23)?,
    })
}

fn required_binding_id(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow::anyhow!("{field} 不能为空"));
    }
    if value.chars().count() > 200 || value.chars().any(char::is_control) {
        return Err(anyhow::anyhow!("{field} 格式无效"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
#[path = "project_execution_sessions_tests.rs"]
mod tests;
