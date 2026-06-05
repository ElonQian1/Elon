use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, safe_external_id, NativeAgentSessionState, Store};

impl Store {
    pub fn get_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .get_native_agent_session_state(
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
            )?
            .map(|state| state.native_session_id))
    }

    pub fn latest_native_agent_session_for_conversation(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        provider: &str,
    ) -> Result<Option<String>> {
        let conversation_id = safe_external_id(conversation_id, "default");
        self.conn()?
            .query_row(
                "SELECT native_session_id
                 FROM agent_native_sessions
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND provider = ?4
                   AND status = 'active'
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![project_id, user_id, conversation_id, provider],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn get_native_agent_session_state(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
    ) -> Result<Option<NativeAgentSessionState>> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?
            .query_row(
                "SELECT native_session_id, chat_bootstrapped, dev_bootstrapped
                 FROM agent_native_sessions
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND provider = ?4
                   AND agent_id = ?5
                   AND workspace_path = ?6
                   AND status = 'active'
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![
                    project_id,
                    user_id,
                    conversation_id,
                    provider,
                    agent_id,
                    workspace_path
                ],
                |row| {
                    Ok(NativeAgentSessionState {
                        native_session_id: row.get(0)?,
                        chat_bootstrapped: row.get::<_, i64>(1)? != 0,
                        dev_bootstrapped: row.get::<_, i64>(2)? != 0,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn upsert_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO agent_native_sessions (
                id, project_id, user_id, conversation_id, provider, agent_id,
                workspace_path, native_session_id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?9)
             ON CONFLICT(project_id, user_id, conversation_id, provider, agent_id, workspace_path)
             DO UPDATE SET
                native_session_id = excluded.native_session_id,
                chat_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.chat_bootstrapped
                    ELSE 0
                END,
                dev_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.dev_bootstrapped
                    ELSE 0
                END,
                status = 'active',
                updated_at = excluded.updated_at",
            params![
                new_id("ans"),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id,
                now
            ],
        )?;
        Ok(())
    }

    /// 标记当前用户任务为 running（服务启动时或任务开始时调用）
    pub fn upsert_native_agent_session_if_no_active(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<bool> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let conn = self.conn()?;
        let existing: Option<String> = conn
            .query_row(
                "SELECT native_session_id
                 FROM agent_native_sessions
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND provider = ?4
                   AND agent_id = ?5
                   AND workspace_path = ?6
                   AND status = 'active'
                 LIMIT 1",
                params![
                    project_id,
                    user_id,
                    conversation_id,
                    provider,
                    agent_id,
                    workspace_path
                ],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(false);
        }

        let now = now();
        conn.execute(
            "INSERT INTO agent_native_sessions (
                id, project_id, user_id, conversation_id, provider, agent_id,
                workspace_path, native_session_id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'active', ?9, ?9)
             ON CONFLICT(project_id, user_id, conversation_id, provider, agent_id, workspace_path)
             DO UPDATE SET
                native_session_id = excluded.native_session_id,
                chat_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.chat_bootstrapped
                    ELSE 0
                END,
                dev_bootstrapped = CASE
                    WHEN agent_native_sessions.native_session_id = excluded.native_session_id
                    THEN agent_native_sessions.dev_bootstrapped
                    ELSE 0
                END,
                status = 'active',
                updated_at = excluded.updated_at",
            params![
                new_id("ans"),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id,
                now
            ],
        )?;
        Ok(true)
    }

    pub fn deactivate_native_agent_session(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?.execute(
            "UPDATE agent_native_sessions
             SET status = 'stale', updated_at = ?1
             WHERE project_id = ?2
               AND user_id = ?3
               AND conversation_id = ?4
               AND provider = ?5
               AND agent_id = ?6
               AND workspace_path = ?7
               AND native_session_id = ?8
               AND status = 'active'",
            params![
                now(),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id
            ],
        )?;
        Ok(())
    }

    pub fn mark_native_agent_session_bootstrapped(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        provider: &str,
        agent_id: &str,
        workspace_path: &str,
        native_session_id: &str,
        development: bool,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let column = if development {
            "dev_bootstrapped"
        } else {
            "chat_bootstrapped"
        };
        let sql = format!(
            "UPDATE agent_native_sessions
             SET {column} = 1, updated_at = ?1
             WHERE project_id = ?2
               AND user_id = ?3
               AND conversation_id = ?4
               AND provider = ?5
               AND agent_id = ?6
               AND workspace_path = ?7
               AND native_session_id = ?8
               AND status = 'active'"
        );
        self.conn()?.execute(
            &sql,
            params![
                now(),
                project_id,
                user_id,
                conversation_id,
                provider,
                agent_id,
                workspace_path,
                native_session_id
            ],
        )?;
        Ok(())
    }
}
