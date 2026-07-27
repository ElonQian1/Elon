//! Idempotent cloud materialization for node-local tasks before completion.

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::{clean_optional, new_id, now, safe_external_id, Store};
#[path = "task_start_sync_events.rs"]
mod events;
use events::{insert_dispatch_event, insert_revision_event};

pub struct PcLocalTaskStartApply<'a> {
    pub request_id: &'a str,
    pub revision: &'a str,
    pub project_id: &'a str,
    pub channel_id: &'a str,
    pub conversation_id: &'a str,
    pub user_id: &'a str,
    pub node_id: &'a str,
    pub prompt: &'a str,
    pub workspace_path: &'a str,
    pub cli: &'a str,
    pub status: &'a str,
    pub codex_session_id: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcLocalTaskStartOutcome {
    pub task_id: String,
    pub project_id: String,
    pub channel_id: String,
    pub conversation_id: String,
    pub changed: bool,
    pub created: bool,
}

impl Store {
    pub fn apply_pc_local_task_start(
        &self,
        input: PcLocalTaskStartApply<'_>,
    ) -> Result<PcLocalTaskStartOutcome> {
        validate(&input)?;
        let conversation_id = safe_external_id(input.conversation_id, "default");
        let timestamp = now();
        let client_request_id = format!("pc_local_task:{}", input.request_id);
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let title = crate::task_title::readable_task_title(input.prompt);
        let mut changed = false;

        tx.execute(
            "INSERT INTO conversations (
                project_id, user_id, id, title, status, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?5)
             ON CONFLICT(project_id, user_id, id) DO UPDATE SET
                updated_at = excluded.updated_at",
            params![
                input.project_id,
                input.user_id,
                conversation_id,
                title,
                timestamp
            ],
        )?;

        let existing = tx
            .query_row(
                "SELECT id, project_id, user_id, conversation_id
                   FROM tasks WHERE client_request_id = ?1 LIMIT 1",
                params![client_request_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let (task_id, created) = match existing {
            Some((task_id, project_id, user_id, bound_conversation_id)) => {
                if project_id != input.project_id
                    || user_id != input.user_id
                    || bound_conversation_id != conversation_id
                {
                    return Err(anyhow!("本机任务同步与既有云端任务归属冲突"));
                }
                (task_id, false)
            }
            None => {
                let task_id = new_id("tsk");
                tx.execute(
                    "INSERT INTO tasks (
                        id, project_id, user_id, conversation_id, client_request_id,
                        message, status, codex_thread_id, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![
                        task_id,
                        input.project_id,
                        input.user_id,
                        conversation_id,
                        client_request_id,
                        input.prompt,
                        cloud_status(input.status),
                        clean_optional(input.codex_session_id),
                        timestamp,
                    ],
                )?;
                (task_id, true)
            }
        };

        changed |= created;
        tx.execute(
            "UPDATE tasks
                SET status = CASE
                        WHEN status IN ('done','failed','canceled') THEN status
                        ELSE ?2
                    END,
                    codex_thread_id = COALESCE(?3, codex_thread_id),
                    updated_at = ?4
              WHERE id = ?1",
            params![
                task_id,
                cloud_status(input.status),
                clean_optional(input.codex_session_id),
                timestamp
            ],
        )?;
        changed |= tx.execute(
            "INSERT INTO messages (
                id, project_id, conversation_id, task_id, user_id, role, content, created_at
             )
             SELECT ?1, ?2, ?3, ?4, ?5, 'user', ?6, ?7
              WHERE NOT EXISTS (
                SELECT 1 FROM messages WHERE task_id = ?4 AND role = 'user'
              )",
            params![
                new_id("msg"),
                input.project_id,
                conversation_id,
                task_id,
                input.user_id,
                input.prompt,
                timestamp
            ],
        )? > 0;
        changed |= tx.execute(
            "INSERT INTO project_channel_messages (
                id, project_id, channel_id, sender_user_id, kind, content, task_id, created_at
             )
             SELECT ?1, ?2, ?3, ?4, 'ai_task', ?5, ?6, ?7
              WHERE NOT EXISTS (
                SELECT 1 FROM project_channel_messages
                 WHERE project_id = ?2 AND channel_id = ?3
                   AND task_id = ?6 AND kind = 'ai_task'
              )",
            params![
                new_id("pcm"),
                input.project_id,
                input.channel_id,
                input.user_id,
                format!("发起本机 Codex 开发任务：{}", input.prompt),
                task_id,
                timestamp
            ],
        )? > 0;

        changed |= insert_dispatch_event(&tx, &task_id, &input, &timestamp)?;
        changed |= insert_revision_event(&tx, &task_id, &input, &timestamp)?;
        tx.execute(
            "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
            params![input.project_id, timestamp],
        )?;
        tx.execute(
            "UPDATE project_channels SET updated_at = ?3
              WHERE project_id = ?1 AND id = ?2",
            params![input.project_id, input.channel_id, timestamp],
        )?;
        tx.commit()?;

        Ok(PcLocalTaskStartOutcome {
            task_id,
            project_id: input.project_id.to_string(),
            channel_id: input.channel_id.to_string(),
            conversation_id,
            changed,
            created,
        })
    }
}

fn cloud_status(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "resume_required" | "interrupted" => "interrupted",
        _ => "running",
    }
}

fn validate(input: &PcLocalTaskStartApply<'_>) -> Result<()> {
    for (name, value) in [
        ("request_id", input.request_id),
        ("revision", input.revision),
        ("project_id", input.project_id),
        ("channel_id", input.channel_id),
        ("conversation_id", input.conversation_id),
        ("user_id", input.user_id),
        ("node_id", input.node_id),
        ("prompt", input.prompt),
        ("workspace_path", input.workspace_path),
        ("cli", input.cli),
    ] {
        if value.trim().is_empty() {
            return Err(anyhow!("{name} 不能为空"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "task_start_sync_tests.rs"]
mod tests;
