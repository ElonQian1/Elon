use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use super::{
    AdminConversationEntry, ConversationMessage, MAX_TASK_EVENTS_PER_TASK, Store, TaskSnapshot,
    clean_optional, new_id, now, safe_external_id,
};

impl Store {
    pub fn ensure_conversation(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        title: Option<&str>,
    ) -> Result<String> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let now = now();
        self.conn()?.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, title, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET
                title = COALESCE(excluded.title, conversations.title),
                updated_at = excluded.updated_at",
            params![
                project_id,
                user_id,
                conversation_id,
                clean_optional(title),
                now
            ],
        )?;
        Ok(conversation_id)
    }

    pub fn create_task(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        message: &str,
    ) -> Result<String> {
        self.create_task_with_client_request(project_id, user_id, conversation_id, None, message)
    }

    pub fn create_task_with_client_request(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        client_request_id: Option<&str>,
        message: &str,
    ) -> Result<String> {
        let id = new_id("tsk");
        let now = now();
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let client_request_id = clean_optional(client_request_id);
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, 'active', ?4, ?4)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET updated_at = excluded.updated_at",
            params![project_id, user_id, conversation_id, now],
        )?;
        tx.execute(
            "INSERT INTO tasks (
                id, project_id, user_id, conversation_id, client_request_id, message, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7, ?7)",
            params![
                id,
                project_id,
                user_id,
                conversation_id,
                client_request_id,
                message,
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, 'user', ?6, ?7)",
            params![
                new_id("msg"),
                project_id,
                conversation_id,
                id,
                user_id,
                message,
                now
            ],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn get_task_by_client_request(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        client_request_id: &str,
    ) -> Result<Option<TaskSnapshot>> {
        let Some(client_request_id) = clean_optional(Some(client_request_id)) else {
            return Ok(None);
        };
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?
            .query_row(
                "SELECT id, project_id, user_id, conversation_id, message, status, apk_url, error
                 FROM tasks
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND client_request_id = ?4
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![project_id, user_id, conversation_id, client_request_id],
                |row| {
                    Ok(TaskSnapshot {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        user_id: row.get(2)?,
                        conversation_id: row.get(3)?,
                        message: row.get(4)?,
                        status: row.get(5)?,
                        apk_url: row.get(6)?,
                        error: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn set_task_running(&self, task_id: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE tasks
             SET status = 'running', error = NULL, updated_at = ?1
             WHERE id = ?2",
            params![now(), task_id],
        )?;
        Ok(())
    }

    pub fn finish_task(
        &self,
        task_id: &str,
        status: &str,
        reply: Option<&str>,
        apk_url: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let now = now();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE tasks
             SET status = ?1, apk_url = ?2, error = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                status,
                clean_optional(apk_url),
                clean_optional(error),
                now,
                task_id
            ],
        )?;
        conn.execute(
            "UPDATE projects
             SET updated_at = ?1
             WHERE id = (SELECT project_id FROM tasks WHERE id = ?2)",
            params![now, task_id],
        )?;

        if let Some(reply) = clean_optional(reply) {
            let task_context: Option<(String, String, Option<String>)> = conn
                .query_row(
                    "SELECT project_id, user_id, conversation_id FROM tasks WHERE id = ?1",
                    params![task_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            if let Some((project_id, user_id, conversation_id)) = task_context {
                drop(conn);
                self.add_message(
                    &project_id,
                    conversation_id.as_deref(),
                    Some(task_id),
                    Some(&user_id),
                    "assistant",
                    reply,
                )?;
            }
        }

        Ok(())
    }

    /// 把 Codex/CopilotCLI 的 native thread ID 写到对应任务上，方便后续诊断。
    /// 取 project+user+conversation 下创建时间最近的任务（通常就是当前运行的那条）。
    pub fn set_latest_task_thread_id(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
        thread_id: &str,
    ) -> Result<()> {
        self.conn()?.execute(
            "UPDATE tasks SET codex_thread_id = ?1, updated_at = ?2
             WHERE id = (
               SELECT id FROM tasks
               WHERE project_id = ?3
                 AND user_id = ?4
                 AND conversation_id = ?5
               ORDER BY created_at DESC
               LIMIT 1
             )",
            params![thread_id, now(), project_id, user_id, conversation_id],
        )?;
        Ok(())
    }

    pub fn latest_task_codex_thread_id(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<String>> {
        let conversation_id = safe_external_id(conversation_id, "default");
        self.conn()?
            .query_row(
                "SELECT codex_thread_id
                 FROM tasks
                 WHERE project_id = ?1
                   AND user_id = ?2
                   AND conversation_id = ?3
                   AND codex_thread_id IS NOT NULL
                   AND codex_thread_id <> ''
                 ORDER BY updated_at DESC, created_at DESC
                 LIMIT 1",
                params![project_id, user_id, conversation_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn record_task_event(&self, task_id: &str, event_json: &str) -> Result<()> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO task_events (id, task_id, event_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![new_id("tev"), task_id, event_json, now()],
        )?;
        tx.execute(
            "DELETE FROM task_events
             WHERE task_id = ?1
               AND rowid NOT IN (
                 SELECT rowid
                 FROM task_events
                 WHERE task_id = ?1
                 ORDER BY created_at DESC, rowid DESC
                 LIMIT ?2
               )",
            params![task_id, MAX_TASK_EVENTS_PER_TASK],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_task_events(&self, task_id: &str, limit: usize) -> Result<Vec<String>> {
        let limit = limit.clamp(1, MAX_TASK_EVENTS_PER_TASK as usize) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT event_json
             FROM (
               SELECT rowid, created_at, event_json
               FROM task_events
               WHERE task_id = ?1
               ORDER BY created_at DESC, rowid DESC
               LIMIT ?2
             )
             ORDER BY created_at ASC, rowid ASC",
        )?;
        let events = stmt
            .query_map(params![task_id, limit], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(events)
    }

    /// 获取项目的构建缓存：`Some((git_sha, apk_url))`。
    /// 若 last_build_sha 或 last_build_apk_url 任一为空则返回 None。
    pub fn get_project_build_cache(
        &self,
        project_id: &str,
    ) -> Result<Option<(String, String)>> {
        self.conn()?
            .query_row(
                "SELECT last_build_sha, last_build_apk_url \
                 FROM projects \
                 WHERE id = ?1 \
                   AND last_build_sha IS NOT NULL \
                   AND last_build_apk_url IS NOT NULL",
                params![project_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    /// 构建成功后更新项目的构建 SHA 缓存。
    /// 下次相同 HEAD SHA 的纯构建请求可直接跳过 Gradle，返回缓存的 APK URL。
    pub fn set_project_build_cache(
        &self,
        project_id: &str,
        sha: &str,
        apk_url: &str,
    ) -> Result<()> {
        self.conn()?.execute(
            "UPDATE projects \
             SET last_build_sha = ?1, last_build_apk_url = ?2, updated_at = ?3 \
             WHERE id = ?4",
            params![sha, apk_url, now(), project_id],
        )?;
        Ok(())
    }

    /// 检查该项目是否有过成功构建并产出 APK 的历史记录。
    /// 用于跨会话 worktree 判断——不依赖当前 worktree 的文件系统。
    pub fn project_has_built_apk(&self, project_id: &str) -> Result<bool> {
        let count: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND apk_url IS NOT NULL AND apk_url != ''",
            params![project_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn add_message(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        task_id: Option<&str>,
        user_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<()> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        self.conn()?.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("msg"),
                project_id,
                conversation_id,
                clean_optional(task_id),
                clean_optional(user_id),
                role,
                content,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn list_recent_conversation_messages(
        &self,
        project_id: &str,
        conversation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>> {
        let conversation_id = safe_external_id(conversation_id.unwrap_or("default"), "default");
        let limit = limit.clamp(1, 30) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT role, content
             FROM (
                SELECT role, content, created_at, id
                FROM messages
                WHERE project_id = ?1
                  AND conversation_id = ?2
                ORDER BY created_at DESC, id DESC
                LIMIT ?3
             )
             ORDER BY created_at ASC, id ASC",
        )?;
        let messages = stmt
            .query_map(params![project_id, conversation_id, limit], |row| {
                Ok(ConversationMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(messages)
    }

    /// 管理员总览：列出某项目下所有会话，附带消息数、任务数和最后任务状态
    pub fn list_conversations_for_project_admin(
        &self,
        project_id: &str,
    ) -> Result<Vec<AdminConversationEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT
               c.id,
               c.project_id,
               c.user_id,
               COALESCE(u.phone, u.email, c.user_id) AS user_account,
               c.title,
               c.status,
               (SELECT COUNT(*) FROM messages m
                WHERE m.project_id = c.project_id
                  AND m.conversation_id = c.id) AS message_count,
               (SELECT COUNT(*) FROM tasks t
                WHERE t.project_id = c.project_id
                  AND t.conversation_id = c.id) AS task_count,
               (SELECT t2.status FROM tasks t2
                WHERE t2.project_id = c.project_id
                  AND t2.conversation_id = c.id
                ORDER BY t2.created_at DESC LIMIT 1) AS last_task_status,
               c.created_at,
               c.updated_at
             FROM conversations c
             LEFT JOIN users u ON u.id = c.user_id
             WHERE c.project_id = ?1
             ORDER BY c.updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(AdminConversationEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    user_id: row.get(2)?,
                    user_account: row.get(3)?,
                    title: row.get(4)?,
                    status: row.get(5)?,
                    message_count: row.get(6)?,
                    task_count: row.get(7)?,
                    last_task_status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path =
            std::env::temp_dir().join(format!("elon_store_test_{}.db", Uuid::new_v4().simple()));
        Store::open(&path).expect("store should open")
    }

    fn temp_task(store: &Store) -> String {
        let user = store
            .create_user("events@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Task Events", None, None)
            .expect("project should be created");
        store
            .create_task(&project.id, &user.id, Some("conv"), "run task")
            .expect("task should be created")
    }

    fn event_message(raw: &str) -> String {
        serde_json::from_str::<serde_json::Value>(raw)
            .expect("event should be json")
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn lists_latest_task_events_in_chronological_order() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..5 {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let messages = store
            .list_task_events(&task_id, 3)
            .expect("events should list")
            .into_iter()
            .map(|raw| event_message(&raw))
            .collect::<Vec<_>>();

        assert_eq!(messages, vec!["step 2", "step 3", "step 4"]);
    }

    #[test]
    fn prunes_old_task_events_per_task() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..(MAX_TASK_EVENTS_PER_TASK + 5) {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let events = store
            .list_task_events(&task_id, MAX_TASK_EVENTS_PER_TASK as usize + 100)
            .expect("events should list");

        assert_eq!(events.len(), MAX_TASK_EVENTS_PER_TASK as usize);
        assert_eq!(event_message(events.first().unwrap()), "step 5");
        assert_eq!(event_message(events.last().unwrap()), "step 1004");
    }
}
