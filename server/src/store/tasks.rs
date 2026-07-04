use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    clean_optional, new_id, now, safe_external_id, Store, TaskEventRecord, TaskSnapshot,
    MAX_TASK_EVENTS_PER_TASK,
};

const CHANNEL_TASK_INTERRUPTED_RESULT: &str =
    "任务已中断：服务器重启前任务未完成。请点击“继续”让 AI 检查当前工作区后接着处理。";
const CHANNEL_TASK_STALE_RESULT: &str =
    "任务失败：PC 节点断线或任务超时自动终止。请点击“继续”让 AI 检查当前工作区后接着处理。";
#[derive(Debug, Clone)]
struct ChannelTaskResultTarget {
    task_id: String,
    project_id: String,
    channel_id: String,
}

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

    // 频道 AI runner 可能在恢复逻辑已经写入终态后才返回；这里用 running CAS
    // 防止旧 runner 覆盖 interrupted/failed/canceled 等已经对用户可见的终态。
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
               AND (?6 = 0 OR status = 'running')",
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

    /// 服务重启恢复：标记运行中任务，并给频道 AI 任务补一条终态消息。
    ///
    /// `project_channel_messages` 是 PC 页面任务卡的权威输入；只更新 `tasks`
    /// 表会让前端仍然看到一个没有 `ai_result` 的运行中任务卡。
    pub fn mark_interrupted_running_tasks_with_channel_results(&self) -> Result<usize> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let targets = running_channel_task_result_targets(&tx)?;
        let n = tx.execute(
            "UPDATE tasks
             SET status = 'interrupted',
                 error = COALESCE(error, 'server restarted before task finished'),
                 updated_at = ?1
             WHERE status = 'running'",
            params![now()],
        )?;
        insert_missing_channel_ai_results(&tx, &targets, CHANNEL_TASK_INTERRUPTED_RESULT)?;
        tx.commit()?;
        Ok(n)
    }

    /// 超时清理：把长期 running 的任务置为 failed，并补齐频道终态。
    ///
    /// 这一步不是“继续原进程”，只是避免用户界面卡在永久运行态；真正续跑仍由
    /// 前端“继续”入口让 AI 重新检查工作区后接着处理。
    pub fn mark_stale_running_tasks_with_channel_results(
        &self,
        older_than_secs: u64,
    ) -> Result<usize> {
        self.mark_stale_running_tasks_with_channel_results_excluding(older_than_secs, &[])
    }

    pub fn mark_stale_running_tasks_with_channel_results_excluding(
        &self,
        older_than_secs: u64,
        excluded_task_ids: &[String],
    ) -> Result<usize> {
        use chrono::{Duration, Utc};
        use std::collections::HashSet;

        let cutoff = (Utc::now() - Duration::seconds(older_than_secs as i64)).to_rfc3339();
        let excluded = excluded_task_ids
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .collect::<HashSet<_>>();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let targets = stale_channel_task_result_targets(&tx, &cutoff)?
            .into_iter()
            .filter(|target| !excluded.contains(target.task_id.as_str()))
            .collect::<Vec<_>>();
        let n = if excluded.is_empty() {
            tx.execute(
                "UPDATE tasks
             SET status = 'failed',
                 error = COALESCE(error, 'PC节点断线或任务超时自动终止'),
                 updated_at = ?1
             WHERE status = 'running'
               AND created_at < ?2",
                params![now(), cutoff],
            )?
        } else {
            let placeholders = std::iter::repeat("?")
                .take(excluded.len())
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "UPDATE tasks
                 SET status = 'failed',
                     error = COALESCE(error, 'PC节点断线或任务超时自动终止'),
                     updated_at = ?
                 WHERE status = 'running'
                   AND created_at < ?
                   AND id NOT IN ({placeholders})"
            );
            let now_value = now();
            let mut values = vec![now_value, cutoff.clone()];
            values.extend(excluded.iter().map(|value| (*value).to_string()));
            tx.execute(&sql, rusqlite::params_from_iter(values.iter()))?
        };
        insert_missing_channel_ai_results(&tx, &targets, CHANNEL_TASK_STALE_RESULT)?;
        tx.commit()?;
        Ok(n)
    }
}

fn running_channel_task_result_targets(conn: &Connection) -> Result<Vec<ChannelTaskResultTarget>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.id, m.project_id, m.channel_id
         FROM tasks t
         JOIN project_channel_messages m
           ON m.task_id = t.id
          AND m.kind = 'ai_task'
         WHERE t.status = 'running'
           AND NOT EXISTS (
             SELECT 1
             FROM project_channel_messages r
             WHERE r.project_id = m.project_id
               AND r.channel_id = m.channel_id
               AND r.task_id = t.id
               AND r.kind = 'ai_result'
           )",
    )?;
    let targets = stmt
        .query_map([], |row| {
            Ok(ChannelTaskResultTarget {
                task_id: row.get(0)?,
                project_id: row.get(1)?,
                channel_id: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(targets)
}

fn stale_channel_task_result_targets(
    conn: &Connection,
    cutoff: &str,
) -> Result<Vec<ChannelTaskResultTarget>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.id, m.project_id, m.channel_id
         FROM tasks t
         JOIN project_channel_messages m
           ON m.task_id = t.id
          AND m.kind = 'ai_task'
         WHERE t.status = 'running'
           AND t.created_at < ?1
           AND NOT EXISTS (
             SELECT 1
             FROM project_channel_messages r
             WHERE r.project_id = m.project_id
               AND r.channel_id = m.channel_id
               AND r.task_id = t.id
               AND r.kind = 'ai_result'
           )",
    )?;
    let targets = stmt
        .query_map(params![cutoff], |row| {
            Ok(ChannelTaskResultTarget {
                task_id: row.get(0)?,
                project_id: row.get(1)?,
                channel_id: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(targets)
}

fn insert_missing_channel_ai_results(
    conn: &Connection,
    targets: &[ChannelTaskResultTarget],
    content: &str,
) -> Result<usize> {
    let mut inserted = 0;
    for target in targets {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id
                 FROM project_channel_messages
                 WHERE project_id = ?1
                   AND channel_id = ?2
                   AND task_id = ?3
                   AND kind = 'ai_result'
                 LIMIT 1",
                params![target.project_id, target.channel_id, target.task_id],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            continue;
        }

        let created_at = now();
        conn.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
             )
             VALUES (?1, ?2, ?3, NULL, 'ai_result', ?4, ?5, ?6)",
            params![
                new_id("pcm"),
                target.project_id,
                target.channel_id,
                content,
                target.task_id,
                created_at
            ],
        )?;
        conn.execute(
            "UPDATE project_channels
             SET updated_at = ?1
             WHERE project_id = ?2 AND id = ?3",
            params![created_at, target.project_id, target.channel_id],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(test)]
mod task_release_tests;

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
            .expect("project should be created")
            .project;
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

    #[test]
    fn lists_task_events_after_stable_rowid_cursor() {
        let store = temp_store();
        let task_id = temp_task(&store);

        for step in 0..4 {
            store
                .record_task_event(
                    &task_id,
                    &format!(r#"{{"type":"progress","message":"step {step}"}}"#),
                )
                .expect("event should be recorded");
        }

        let first_page = store
            .list_task_events_after(&task_id, 0, 2)
            .expect("events should list");
        assert_eq!(first_page.len(), 2);
        assert_eq!(event_message(&first_page[0].event_json), "step 0");
        assert_eq!(event_message(&first_page[1].event_json), "step 1");

        let second_page = store
            .list_task_events_after(&task_id, first_page[1].seq, 10)
            .expect("events after cursor should list");
        let messages = second_page
            .iter()
            .map(|event| event_message(&event.event_json))
            .collect::<Vec<_>>();
        assert_eq!(messages, vec!["step 2", "step 3"]);
        assert_eq!(
            store
                .latest_task_event_seq(&task_id)
                .expect("latest seq should load"),
            second_page.last().expect("event should exist").seq
        );
    }

    #[test]
    fn channel_task_snapshot_requires_channel_task_link() {
        let store = temp_store();
        let user = store
            .create_user("channel-task-snapshot@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Channel Task Snapshot", None, None)
            .expect("project should be created")
            .project;
        let channels = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list");
        let ai_channel = channels
            .iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai channel should exist");
        let discussion_channel = channels
            .iter()
            .find(|channel| channel.kind == "discussion")
            .expect("discussion channel should exist");
        let task_id = store
            .create_task(&project.id, &user.id, Some("channel-dev"), "继续修复")
            .expect("task should create");

        assert!(store
            .get_channel_task_snapshot(&project.id, &ai_channel.id, &task_id)
            .expect("snapshot query should work")
            .is_none());
        store
            .insert_project_channel_message(
                &project.id,
                &ai_channel.id,
                Some(&user.id),
                "ai_task",
                "发起 AI 开发任务：继续修复",
                Some(&task_id),
                None,
            )
            .expect("task message should insert");

        let snapshot = store
            .get_channel_task_snapshot(&project.id, &ai_channel.id, &task_id)
            .expect("snapshot should query")
            .expect("linked task should be visible");
        assert_eq!(snapshot.id, task_id);
        assert_eq!(snapshot.status, "running");
        assert!(store
            .get_channel_task_snapshot(&project.id, &discussion_channel.id, &snapshot.id)
            .expect("snapshot query should work")
            .is_none());
    }

    mod task_recovery_tests {
        include!("task_recovery_tests.rs");
    }
}
