//! PC 项目执行会话记录。
//!
//! 这里记录的是「某次项目会话在 PC 节点上的 CLI 执行状态」，用于项目工作区健康页
//! 判断最近一次会话 worktree 是否成功合并、是否仍在运行或失败。

use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{new_id, now, Store};

#[derive(Debug, Clone, Serialize)]
pub struct ProjectExecutionSession {
    pub id: String,
    pub project_id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub node_id: String,
    pub request_id: String,
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
    ) -> Result<()> {
        let now = now();
        self.conn()?.execute(
            "INSERT INTO project_execution_sessions (
                id, project_id, conversation_id, user_id, node_id, request_id,
                active_workspace_path, isolated, status, model, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'running', ?8, ?9, ?9)
             ON CONFLICT(request_id) DO UPDATE SET
                active_workspace_path = excluded.active_workspace_path,
                status = 'running',
                model = COALESCE(excluded.model, project_execution_sessions.model),
                updated_at = excluded.updated_at",
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
        Ok(())
    }

    pub fn record_project_execution_finished(
        &self,
        session: ProjectExecutionSessionFinish<'_>,
    ) -> Result<()> {
        self.conn()?.execute(
            "UPDATE project_execution_sessions
             SET base_workspace_path = ?2,
                 active_workspace_path = COALESCE(?3, active_workspace_path),
                 branch = ?4,
                 isolated = ?5,
                 status = ?6,
                 merge_status = ?7,
                 last_error = ?8,
                 model = COALESCE(?9, model),
                 prompt_tokens = COALESCE(?10, prompt_tokens),
                 cached_input_tokens = COALESCE(?11, cached_input_tokens),
                 completion_tokens = COALESCE(?12, completion_tokens),
                 reasoning_tokens = COALESCE(?13, reasoning_tokens),
                 total_tokens = COALESCE(?14, total_tokens),
                 token_usage_event_id = COALESCE(?15, token_usage_event_id),
                 billing_event_id = COALESCE(?16, billing_event_id),
                 updated_at = ?17
             WHERE request_id = ?1",
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
                now()
            ],
        )?;
        Ok(())
    }

    pub fn latest_project_execution_session(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectExecutionSession>> {
        self.conn()?
            .query_row(
                "SELECT id, project_id, conversation_id, user_id, node_id, request_id,
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
        base_workspace_path: row.get(6)?,
        active_workspace_path: row.get(7)?,
        branch: row.get(8)?,
        isolated: row.get::<_, i64>(9)? != 0,
        status: row.get(10)?,
        merge_status: row.get(11)?,
        last_error: row.get(12)?,
        model: row.get(13)?,
        prompt_tokens: row.get(14)?,
        cached_input_tokens: row.get(15)?,
        completion_tokens: row.get(16)?,
        reasoning_tokens: row.get(17)?,
        total_tokens: row.get(18)?,
        token_usage_event_id: row.get(19)?,
        billing_event_id: row.get(20)?,
        created_at: row.get(21)?,
        updated_at: row.get(22)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_project_execution_sessions_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn latest_session_tracks_workspace_finish() {
        let store = temp_store();
        let user = store
            .create_user("project-execution@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "执行会话项目", None, Some("android"))
            .expect("project should be created")
            .project;
        store
            .record_project_execution_started(ProjectExecutionSessionStart {
                project_id: &project.id,
                conversation_id: "conv-a",
                user_id: &user.id,
                node_id: "node-a",
                request_id: "req-a",
                requested_workspace_path: Some("D:/repo"),
                model: Some("codex"),
            })
            .expect("start should record");
        store
            .record_project_execution_finished(ProjectExecutionSessionFinish {
                request_id: "req-a",
                base_workspace_path: Some("D:/repo"),
                active_workspace_path: Some("D:/wt"),
                branch: Some("ai/session/prj-a/conv-a"),
                isolated: true,
                status: "done",
                merge_status: Some("merged"),
                last_error: None,
                model: Some("gpt-5"),
                prompt_tokens: Some(100),
                cached_input_tokens: Some(20),
                completion_tokens: Some(30),
                reasoning_tokens: Some(5),
                total_tokens: Some(130),
                token_usage_event_id: Some("tok-a"),
                billing_event_id: Some("bev-a"),
            })
            .expect("finish should update");

        let latest = store
            .latest_project_execution_session(&project.id)
            .expect("latest should query")
            .expect("latest should exist");
        assert_eq!(latest.status, "done");
        assert_eq!(latest.active_workspace_path.as_deref(), Some("D:/wt"));
        assert!(latest.isolated);
        assert_eq!(latest.model.as_deref(), Some("gpt-5"));
        assert_eq!(latest.prompt_tokens, 100);
        assert_eq!(latest.cached_input_tokens, 20);
        assert_eq!(latest.completion_tokens, 30);
        assert_eq!(latest.reasoning_tokens, 5);
        assert_eq!(latest.total_tokens, 130);
        assert_eq!(latest.token_usage_event_id.as_deref(), Some("tok-a"));
        assert_eq!(latest.billing_event_id.as_deref(), Some("bev-a"));
    }
}
