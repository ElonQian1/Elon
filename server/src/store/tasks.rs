use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{
    clean_optional, new_id, now, safe_external_id, Store, TaskEventRecord, TaskSnapshot,
    MAX_TASK_EVENTS_PER_TASK,
};

impl Store {
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
        self.create_task_with_client_request_and_display_message(
            project_id,
            user_id,
            conversation_id,
            client_request_id,
            message,
            message,
        )
    }

    pub fn create_task_with_display_message(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        message: &str,
        display_message: &str,
    ) -> Result<String> {
        self.create_task_with_client_request_and_display_message(
            project_id,
            user_id,
            conversation_id,
            None,
            message,
            display_message,
        )
    }

    pub fn create_task_with_client_request_and_display_message(
        &self,
        project_id: &str,
        user_id: &str,
        conversation_id: Option<&str>,
        client_request_id: Option<&str>,
        message: &str,
        display_message: &str,
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
                display_message,
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

    pub fn get_channel_task_snapshot(
        &self,
        project_id: &str,
        channel_id: &str,
        task_id: &str,
    ) -> Result<Option<TaskSnapshot>> {
        self.conn()?
            .query_row(
                "SELECT t.id, t.project_id, t.user_id, t.conversation_id,
                        t.message, t.status, t.apk_url, t.error
                 FROM tasks t
                 WHERE t.id = ?3
                   AND t.project_id = ?1
                   AND EXISTS (
                     SELECT 1
                     FROM project_channel_messages m
                     WHERE m.project_id = ?1
                       AND m.channel_id = ?2
                       AND m.task_id = t.id
                   )
                 LIMIT 1",
                params![project_id, channel_id, task_id],
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
        self.finish_task_inner(task_id, status, reply, apk_url, error, false)
            .map(|_| ())
    }

    // 频道 AI runner 可能在恢复逻辑已经写入终态后才返回；这里用活动态 CAS
    // 防止旧 runner 覆盖 failed/canceled 等已经对用户可见的终态。
    pub fn finish_running_task(
        &self,
        task_id: &str,
        status: &str,
        reply: Option<&str>,
        apk_url: Option<&str>,
        error: Option<&str>,
    ) -> Result<bool> {
        self.finish_task_inner(task_id, status, reply, apk_url, error, true)
    }

    fn finish_task_inner(
        &self,
        task_id: &str,
        status: &str,
        reply: Option<&str>,
        apk_url: Option<&str>,
        error: Option<&str>,
        require_running: bool,
    ) -> Result<bool> {
        let now = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE tasks
             SET status = ?1, apk_url = ?2, error = ?3, updated_at = ?4
             WHERE id = ?5
               AND (?6 = 0 OR status IN ('running', 'recovering'))",
            params![
                status,
                clean_optional(apk_url),
                clean_optional(error),
                now,
                task_id,
                if require_running { 1 } else { 0 }
            ],
        )?;
        if changed == 0 {
            return Ok(false);
        }
        conn.execute(
            "UPDATE projects
             SET updated_at = ?1
             WHERE id = (SELECT project_id FROM tasks WHERE id = ?2)",
            params![now, task_id],
        )?;
        super::project_releases::insert_task_apk_release_locked(&conn, task_id, status, apk_url)?;

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

        Ok(true)
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

    pub fn latest_task_event_seq(&self, task_id: &str) -> Result<i64> {
        self.conn()?
            .query_row(
                "SELECT COALESCE(MAX(rowid), 0)
                 FROM task_events
                 WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn list_task_events_after(
        &self,
        task_id: &str,
        since_seq: i64,
        limit: usize,
    ) -> Result<Vec<TaskEventRecord>> {
        let since_seq = since_seq.max(0);
        let limit = limit.clamp(1, MAX_TASK_EVENTS_PER_TASK as usize) as i64;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT rowid, event_json, created_at
             FROM task_events
             WHERE task_id = ?1
               AND rowid > ?2
             ORDER BY rowid ASC
             LIMIT ?3",
        )?;
        let events = stmt
            .query_map(params![task_id, since_seq, limit], |row| {
                Ok(TaskEventRecord {
                    seq: row.get(0)?,
                    event_json: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(events)
    }

    /// 获取项目的构建缓存：`Some((git_sha, apk_url))`。
    /// 若 last_build_sha 或 last_build_apk_url 任一为空则返回 None。
    pub fn get_project_build_cache(&self, project_id: &str) -> Result<Option<(String, String)>> {
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

    pub fn project_has_built_apk(&self, project_id: &str) -> Result<bool> {
        let count: i64 = self.conn()?.query_row(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ?1 AND apk_url IS NOT NULL AND apk_url != ''",
            params![project_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn latest_project_apk_url(&self, project_id: &str) -> Result<Option<String>> {
        Ok(self
            .latest_project_apk_delivery(project_id)?
            .map(|(_, apk_url, _)| apk_url))
    }

    /// 获取项目最近一次任务产出的 APK 交付记录。
    ///
    /// 返回 `(task_id, apk_url, updated_at)`，用于客户端判断稳定下载地址背后是否已有新交付。
    pub fn latest_project_apk_delivery(
        &self,
        project_id: &str,
    ) -> Result<Option<(String, String, String)>> {
        if let Some(release) = self.latest_project_release(project_id)? {
            return Ok(Some((release.id, release.apk_url, release.updated_at)));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod task_release_tests;


#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tasks_tests;
