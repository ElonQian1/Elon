//! Idempotent materialization of a durable PC CLI completion into project tasks.

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{clean_optional, new_id, now, safe_external_id, Store};

pub struct PcCliTaskCompletionApply<'a> {
    pub completion_event_id: &'a str,
    pub task_id: Option<&'a str>,
    pub project_id: &'a str,
    pub channel_id: Option<&'a str>,
    pub conversation_id: &'a str,
    pub user_id: &'a str,
    pub prompt: Option<&'a str>,
    pub final_reply: &'a str,
    pub channel_result: &'a str,
    pub status: &'a str,
    pub error: Option<&'a str>,
    pub codex_session_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcCliTaskCompletionOutcome {
    pub task_id: String,
    pub project_id: String,
    pub channel_id: Option<String>,
    pub conversation_id: String,
    pub changed: bool,
    pub created: bool,
    pub canceled: bool,
    pub terminal_conflict: bool,
}

#[derive(Debug)]
struct TaskTarget {
    task_id: String,
    project_id: String,
    user_id: String,
    conversation_id: String,
    channel_id: Option<String>,
    status: String,
    error: Option<String>,
    created: bool,
}

impl Store {
    /// Apply a replayed terminal result exactly once. Locally-created work is
    /// first turned into a normal cloud task using the completion event as its
    /// idempotency key. A real late result may repair communication-generated
    /// interruption states, but it never overwrites a user cancellation or an
    /// unrelated business failure.
    pub fn apply_pc_cli_task_completion(
        &self,
        input: PcCliTaskCompletionApply<'_>,
    ) -> Result<PcCliTaskCompletionOutcome> {
        validate_input(&input)?;
        let conversation_id = safe_external_id(input.conversation_id, "default");
        let timestamp = now();
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;

        let target = if let Some(task_id) = clean_optional(input.task_id) {
            load_cloud_target(&tx, task_id)?
                .ok_or_else(|| anyhow!("completion 绑定的云端任务不存在"))?
        } else {
            create_or_load_local_target(&tx, &input, &conversation_id, &timestamp)?
        };

        if target.project_id != input.project_id
            || target.user_id != input.user_id
            || target.conversation_id != conversation_id
        {
            return Err(anyhow!("completion 与云端任务归属不一致"));
        }
        if let Some(expected_channel_id) = clean_optional(input.channel_id) {
            if target.channel_id.as_deref() != Some(expected_channel_id) {
                return Err(anyhow!("completion 与云端任务频道不一致"));
            }
        }

        let canceled = target.status == "canceled";
        if canceled {
            tx.commit()?;
            return Ok(outcome(&target, false, true, false));
        }

        let correctable = task_state_is_correctable(&target.status, target.error.as_deref());
        let already_same_terminal = target.status == input.status;
        let terminal_conflict = !correctable
            && matches!(target.status.as_str(), "done" | "failed")
            && !already_same_terminal;
        if terminal_conflict {
            tx.commit()?;
            return Ok(outcome(&target, false, false, true));
        }
        let should_materialize = correctable || already_same_terminal;
        let mut changed = false;

        if correctable {
            changed |= tx.execute(
                "UPDATE tasks
                    SET status = ?2,
                        error = ?3,
                        codex_thread_id = COALESCE(?4, codex_thread_id),
                        updated_at = ?5
                  WHERE id = ?1",
                params![
                    target.task_id,
                    normalize_completion_status(input.status),
                    clean_optional(input.error),
                    clean_optional(input.codex_session_id),
                    timestamp,
                ],
            )? > 0;
        } else if already_same_terminal && clean_optional(input.codex_session_id).is_some() {
            changed |= tx.execute(
                "UPDATE tasks
                    SET codex_thread_id = COALESCE(codex_thread_id, ?2), updated_at = ?3
                  WHERE id = ?1 AND codex_thread_id IS NULL",
                params![
                    target.task_id,
                    clean_optional(input.codex_session_id),
                    timestamp
                ],
            )? > 0;
        }

        if should_materialize {
            changed |=
                upsert_assistant_reply(&tx, &target, input.final_reply, correctable, &timestamp)?;
            if let Some(channel_id) = target.channel_id.as_deref() {
                changed |= upsert_channel_result(
                    &tx,
                    &target.project_id,
                    channel_id,
                    &target.task_id,
                    input.channel_result,
                    correctable,
                    &timestamp,
                )?;
            }
        }

        let event_json = serde_json::json!({
            "type": "completion_replayed",
            "task_id": target.task_id,
            "completion_event_id": input.completion_event_id,
            "status": normalize_completion_status(input.status),
            "corrected_previous_state": correctable,
        })
        .to_string();
        let event_marker = format!("\"completion_event_id\":\"{}\"", input.completion_event_id);
        changed |= tx.execute(
            "INSERT INTO task_events (id, task_id, event_json, created_at)
             SELECT ?1, ?2, ?3, ?4
              WHERE NOT EXISTS (
                SELECT 1 FROM task_events
                 WHERE task_id = ?2 AND instr(event_json, ?5) > 0
              )",
            params![
                new_id("tev"),
                target.task_id,
                event_json,
                timestamp,
                event_marker
            ],
        )? > 0;

        tx.execute(
            "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
            params![target.project_id, timestamp],
        )?;
        if let Some(channel_id) = target.channel_id.as_deref() {
            tx.execute(
                "UPDATE project_channels SET updated_at = ?3
                  WHERE project_id = ?1 AND id = ?2",
                params![target.project_id, channel_id, timestamp],
            )?;
        }
        tx.commit()?;
        Ok(outcome(&target, changed, false, false))
    }
}

fn create_or_load_local_target(
    tx: &rusqlite::Transaction<'_>,
    input: &PcCliTaskCompletionApply<'_>,
    conversation_id: &str,
    timestamp: &str,
) -> Result<TaskTarget> {
    let channel_id = clean_optional(input.channel_id)
        .ok_or_else(|| anyhow!("local_offline completion 缺少 channel_id"))?;
    let prompt = clean_optional(input.prompt)
        .ok_or_else(|| anyhow!("local_offline completion 缺少 prompt"))?;
    let client_request_id = format!("pc_offline:{}", input.completion_event_id);

    tx.execute(
        "INSERT INTO conversations (
            project_id, user_id, id, title, status, created_at, updated_at
         ) VALUES (?1, ?2, ?3, '本机离线任务', 'active', ?4, ?4)
         ON CONFLICT(project_id, user_id, id) DO UPDATE SET updated_at = excluded.updated_at",
        params![input.project_id, input.user_id, conversation_id, timestamp],
    )?;

    let existing_id: Option<String> = tx
        .query_row(
            "SELECT id FROM tasks
              WHERE project_id = ?1 AND user_id = ?2 AND conversation_id = ?3
                AND client_request_id = ?4
              LIMIT 1",
            params![
                input.project_id,
                input.user_id,
                conversation_id,
                client_request_id
            ],
            |row| row.get(0),
        )
        .optional()?;
    let (task_id, created) = match existing_id {
        Some(task_id) => (task_id, false),
        None => {
            let task_id = new_id("tsk");
            tx.execute(
                "INSERT INTO tasks (
                    id, project_id, user_id, conversation_id, client_request_id,
                    message, status, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'running', ?7, ?7)",
                params![
                    task_id,
                    input.project_id,
                    input.user_id,
                    conversation_id,
                    client_request_id,
                    prompt,
                    timestamp,
                ],
            )?;
            tx.execute(
                "INSERT INTO messages (
                    id, project_id, conversation_id, task_id, user_id, role, content, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 'user', ?6, ?7)",
                params![
                    new_id("msg"),
                    input.project_id,
                    conversation_id,
                    task_id,
                    input.user_id,
                    prompt,
                    timestamp,
                ],
            )?;
            (task_id, true)
        }
    };

    tx.execute(
        "INSERT INTO project_channel_messages (
            id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
         )
         SELECT ?1, ?2, ?3, ?4, 'ai_task', ?5, ?6, ?7
          WHERE NOT EXISTS (
            SELECT 1 FROM project_channel_messages
             WHERE project_id = ?2 AND channel_id = ?3 AND task_id = ?6 AND kind = 'ai_task'
          )",
        params![
            new_id("pcm"),
            input.project_id,
            channel_id,
            input.user_id,
            format!("发起本机离线 AI 开发任务：{prompt}"),
            task_id,
            timestamp,
        ],
    )?;

    Ok(TaskTarget {
        task_id,
        project_id: input.project_id.to_string(),
        user_id: input.user_id.to_string(),
        conversation_id: conversation_id.to_string(),
        channel_id: Some(channel_id.to_string()),
        status: if created {
            "running".to_string()
        } else {
            load_task_status(tx, &client_request_id)?
        },
        error: if created {
            None
        } else {
            load_task_error(tx, &client_request_id)?
        },
        created,
    })
}

fn load_cloud_target(tx: &rusqlite::Transaction<'_>, task_id: &str) -> Result<Option<TaskTarget>> {
    tx.query_row(
        "SELECT t.id, t.project_id, t.user_id, COALESCE(t.conversation_id, 'default'),
                (SELECT m.channel_id FROM project_channel_messages m
                  WHERE m.task_id = t.id AND m.kind = 'ai_task'
                  ORDER BY m.created_at LIMIT 1),
                t.status, t.error
           FROM tasks t WHERE t.id = ?1 LIMIT 1",
        params![task_id],
        |row| {
            Ok(TaskTarget {
                task_id: row.get(0)?,
                project_id: row.get(1)?,
                user_id: row.get(2)?,
                conversation_id: row.get(3)?,
                channel_id: row.get(4)?,
                status: row.get(5)?,
                error: row.get(6)?,
                created: false,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

fn load_task_status(tx: &rusqlite::Transaction<'_>, client_request_id: &str) -> Result<String> {
    tx.query_row(
        "SELECT status FROM tasks WHERE client_request_id = ?1 LIMIT 1",
        params![client_request_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn load_task_error(
    tx: &rusqlite::Transaction<'_>,
    client_request_id: &str,
) -> Result<Option<String>> {
    tx.query_row(
        "SELECT error FROM tasks WHERE client_request_id = ?1 LIMIT 1",
        params![client_request_id],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn upsert_assistant_reply(
    tx: &rusqlite::Transaction<'_>,
    target: &TaskTarget,
    final_reply: &str,
    replace_existing: bool,
    timestamp: &str,
) -> Result<bool> {
    if replace_existing {
        let updated = tx.execute(
            "UPDATE messages SET content = ?2
              WHERE task_id = ?1 AND role = 'assistant'",
            params![target.task_id, final_reply],
        )?;
        if updated > 0 {
            return Ok(true);
        }
    }
    let inserted = tx.execute(
        "INSERT INTO messages (
            id, project_id, conversation_id, task_id, user_id, role, content, created_at
         )
         SELECT ?1, ?2, ?3, ?4, ?5, 'assistant', ?6, ?7
          WHERE NOT EXISTS (
            SELECT 1 FROM messages WHERE task_id = ?4 AND role = 'assistant'
          )",
        params![
            new_id("msg"),
            target.project_id,
            target.conversation_id,
            target.task_id,
            target.user_id,
            final_reply,
            timestamp,
        ],
    )?;
    Ok(inserted > 0)
}

fn upsert_channel_result(
    tx: &rusqlite::Transaction<'_>,
    project_id: &str,
    channel_id: &str,
    task_id: &str,
    content: &str,
    replace_existing: bool,
    timestamp: &str,
) -> Result<bool> {
    if replace_existing {
        let updated = tx.execute(
            "UPDATE project_channel_messages SET content = ?4
              WHERE project_id = ?1 AND channel_id = ?2 AND task_id = ?3 AND kind = 'ai_result'",
            params![project_id, channel_id, task_id, content],
        )?;
        if updated > 0 {
            return Ok(true);
        }
    }
    let inserted = tx.execute(
        "INSERT INTO project_channel_messages (
            id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
         )
         SELECT ?1, ?2, ?3, NULL, 'ai_result', ?4, ?5, ?6
          WHERE NOT EXISTS (
            SELECT 1 FROM project_channel_messages
             WHERE project_id = ?2 AND channel_id = ?3 AND task_id = ?5 AND kind = 'ai_result'
          )",
        params![
            new_id("pcm"),
            project_id,
            channel_id,
            content,
            task_id,
            timestamp
        ],
    )?;
    Ok(inserted > 0)
}

fn task_state_is_correctable(status: &str, error: Option<&str>) -> bool {
    matches!(status, "running" | "recovering" | "interrupted")
        || status == "failed" && is_automatic_communication_failure(error.unwrap_or_default())
}

pub(crate) fn is_automatic_communication_failure(error: &str) -> bool {
    let error = error.trim();
    matches!(
        error,
        "PC节点通信自动恢复超时"
            | "server update recovery pending"
            | "server update recovery timed out"
            | "server restarted before PC CLI terminal event"
            | "PC agent CLI 连接中断（未收到 CliDone）"
            | "PC 节点通信临时中断：Win 端正在更新升级/重启或节点重新注册，旧连接已关闭。"
            | "PC 节点通信临时中断：服务器正在更新升级或 Win 端正在更新升级/重启时会临时断开；系统会等待节点重新连接并尝试恢复。"
    ) || error
        .strip_prefix("PC agent CLI 等待终态超时（")
        .is_some_and(|suffix| suffix.ends_with("），已取消本机任务"))
        || error.starts_with(
            "PC 节点短线恢复等待超时；稍后到达的本机结果仍会通过离线账本同步。断线原因：",
        )
        || error.starts_with(
            "PC 节点通信临时中断：服务器正在更新升级、Win 端正在更新升级/重启或节点连接重建时，会短暂打断 Codex CLI 通信。本轮已停止等待，避免重复执行。",
        )
}

fn normalize_completion_status(status: &str) -> &'static str {
    if status == "done" {
        "done"
    } else {
        "failed"
    }
}

fn validate_input(input: &PcCliTaskCompletionApply<'_>) -> Result<()> {
    for (field, value) in [
        ("completion_event_id", input.completion_event_id),
        ("project_id", input.project_id),
        ("conversation_id", input.conversation_id),
        ("user_id", input.user_id),
    ] {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
            return Err(anyhow!("{field} 无效"));
        }
    }
    if !matches!(input.status, "done" | "failed") {
        return Err(anyhow!("completion status 无效"));
    }
    if input.final_reply.trim().is_empty() || input.channel_result.trim().is_empty() {
        return Err(anyhow!("completion 结果内容不能为空"));
    }
    Ok(())
}

fn outcome(
    target: &TaskTarget,
    changed: bool,
    canceled: bool,
    terminal_conflict: bool,
) -> PcCliTaskCompletionOutcome {
    PcCliTaskCompletionOutcome {
        task_id: target.task_id.clone(),
        project_id: target.project_id.clone(),
        channel_id: target.channel_id.clone(),
        conversation_id: target.conversation_id.clone(),
        changed,
        created: target.created,
        canceled,
        terminal_conflict,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_automatic_communication_failure, PcCliTaskCompletionApply};
    use crate::store::Store;
    use rusqlite::params;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-task-completion-replay-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    fn channel_task(store: &Store, account: &str) -> (String, String, String, String) {
        let user = store
            .create_user(account, "secret1", None, None)
            .expect("user");
        let project = store
            .create_project(&user.id, "Replay project", None, None)
            .expect("project")
            .project;
        let channel = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels")
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("development channel");
        let task_id = store
            .create_task(&project.id, &user.id, Some("conversation-a"), "修复项目")
            .expect("task");
        store
            .insert_project_channel_message(
                &project.id,
                &channel.id,
                Some(&user.id),
                "ai_task",
                "发起 AI 开发任务：修复项目",
                Some(&task_id),
                None,
            )
            .expect("channel task");
        (user.id, project.id, channel.id, task_id)
    }

    fn apply<'a>(
        store: &Store,
        event_id: &'a str,
        task_id: Option<&'a str>,
        user_id: &'a str,
        project_id: &'a str,
        channel_id: &'a str,
        status: &'a str,
        reply: &'a str,
    ) -> super::PcCliTaskCompletionOutcome {
        store
            .apply_pc_cli_task_completion(PcCliTaskCompletionApply {
                completion_event_id: event_id,
                task_id,
                project_id,
                channel_id: Some(channel_id),
                conversation_id: "conversation-a",
                user_id,
                prompt: task_id.is_none().then_some("离线修复项目"),
                final_reply: reply,
                channel_result: reply,
                status,
                error: (status == "failed").then_some(reply),
                codex_session_id: Some("session-a"),
            })
            .expect("completion should apply")
    }

    #[test]
    fn only_communication_failures_are_correctable() {
        assert!(is_automatic_communication_failure(
            "PC 节点通信临时中断：Win 端正在更新升级/重启或节点重新注册，旧连接已关闭。"
        ));
        assert!(is_automatic_communication_failure(
            "server restarted before PC CLI terminal event"
        ));
        assert!(is_automatic_communication_failure(
            "PC agent CLI 等待终态超时（120s），已取消本机任务"
        ));
        assert!(!is_automatic_communication_failure(
            "cargo test failed because an assertion failed"
        ));
        assert!(!is_automatic_communication_failure(
            "PC 节点通信临时中断：旧连接已关闭"
        ));
        for business_error in [
            "数据库连接中断导致迁移失败",
            "业务等待终态超时，需要人工检查",
            "PC节点通信模块测试失败",
            "git fetch: connection reset by peer",
        ] {
            assert!(
                !is_automatic_communication_failure(business_error),
                "business failure must remain terminal: {business_error}"
            );
        }
    }

    #[test]
    fn late_real_result_replaces_automatic_failure_idempotently() {
        let store = temp_store();
        let (user_id, project_id, channel_id, task_id) =
            channel_task(&store, "replay-recovery@example.com");
        let old_created_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
        store
            .conn()
            .unwrap()
            .execute(
                "UPDATE tasks SET created_at = ?1 WHERE id = ?2",
                params![old_created_at, task_id],
            )
            .unwrap();
        store
            .mark_stale_running_tasks_with_channel_results(10 * 60)
            .unwrap();

        let (stale_status, stale_error): (String, Option<String>) = store
            .conn()
            .unwrap()
            .query_row(
                "SELECT status, error FROM tasks WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(stale_status, "failed");
        assert_eq!(stale_error.as_deref(), Some("PC节点通信自动恢复超时"));

        let first = apply(
            &store,
            "event-recovery",
            Some(&task_id),
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "真实离线结果已完成",
        );
        assert!(first.changed);
        let second = apply(
            &store,
            "event-recovery",
            Some(&task_id),
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "真实离线结果已完成",
        );
        assert!(!second.changed);

        let conn = store.conn().unwrap();
        let (status, error): (String, Option<String>) = conn
            .query_row(
                "SELECT status, error FROM tasks WHERE id = ?1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "done");
        assert!(error.is_none());
        let result_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_channel_messages
                  WHERE task_id = ?1 AND kind = 'ai_result'
                    AND content = '真实离线结果已完成'",
                params![task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(result_count, 1);
        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_events WHERE task_id = ?1
                    AND instr(event_json, 'event-recovery') > 0",
                params![task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn cancellation_and_real_business_failure_are_never_overwritten() {
        let store = temp_store();
        let (user_id, project_id, channel_id, canceled_task) =
            channel_task(&store, "replay-canceled@example.com");
        store
            .finish_task(
                &canceled_task,
                "canceled",
                Some("用户已停止"),
                None,
                Some("用户已停止"),
            )
            .unwrap();
        let canceled = apply(
            &store,
            "event-canceled",
            Some(&canceled_task),
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "迟到成功结果",
        );
        assert!(canceled.canceled);

        let (user_id, project_id, channel_id, failed_task) =
            channel_task(&store, "replay-business-failed@example.com");
        store
            .finish_task(
                &failed_task,
                "failed",
                Some("cargo test 断言失败"),
                None,
                Some("cargo test 断言失败"),
            )
            .unwrap();
        let conflict = apply(
            &store,
            "event-conflict",
            Some(&failed_task),
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "不应覆盖",
        );
        assert!(conflict.terminal_conflict);
    }

    #[test]
    fn local_offline_task_materializes_once() {
        let store = temp_store();
        let (user_id, project_id, channel_id, _unused_task) =
            channel_task(&store, "replay-local@example.com");
        let first = apply(
            &store,
            "event-local",
            None,
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "离线任务完成",
        );
        assert!(first.created);
        let second = apply(
            &store,
            "event-local",
            None,
            &user_id,
            &project_id,
            &channel_id,
            "done",
            "离线任务完成",
        );
        assert_eq!(first.task_id, second.task_id);
        assert!(!second.created);
        let count: i64 = store
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE client_request_id = 'pc_offline:event-local'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
