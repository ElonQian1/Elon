use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{new_id, now, Store, UI_TUNER_MODULE_KEY};

impl Store {
    pub(crate) fn record_ui_tuner_task_completion(
        &self,
        task_id: &str,
        final_status: &str,
        final_reply: &str,
    ) -> Result<bool> {
        let conn = self.conn()?;
        let context: Option<(String, String, String, String, String, String)> = conn
            .query_row(
                "SELECT a.project_id, a.user_id, a.conversation_id, a.id, a.user_intent,
                        w.stable_summary
                 FROM project_module_context_artifacts a
                 JOIN project_module_workspaces w
                   ON w.project_id = a.project_id AND w.user_id = a.user_id AND w.module_key = a.module_key
                 WHERE a.task_id = ?1 AND a.module_key = ?2",
                params![task_id, UI_TUNER_MODULE_KEY],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .optional()?;
        let Some((project_id, user_id, conversation_id, artifact_id, user_intent, old_summary)) =
            context
        else {
            return Ok(false);
        };
        let source_message_id: Option<String> = conn
            .query_row(
                "SELECT id FROM messages WHERE task_id = ?1 AND LOWER(role) IN ('assistant', 'ai')
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                params![task_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(source_message_id) = source_message_id else {
            return Ok(false);
        };
        let ts = now();
        let checkpoint_id = new_id("pmc");
        let success = final_status == "done";
        let next_summary = build_stable_summary(&old_summary, &user_intent, final_reply, success);
        let tx = conn.unchecked_transaction()?;
        let current_revision: i64 = tx.query_row(
            "SELECT memory_revision FROM project_module_workspaces
             WHERE project_id = ?1 AND user_id = ?2 AND module_key = ?3",
            params![project_id, user_id, UI_TUNER_MODULE_KEY],
            |row| row.get(0),
        )?;
        let next_revision = current_revision + 1;
        tx.execute(
            "INSERT INTO project_module_checkpoints
             (id, project_id, user_id, module_key, conversation_id, source_message_id, task_id,
              context_artifact_id, memory_revision, status, summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                checkpoint_id,
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY,
                conversation_id,
                source_message_id,
                task_id,
                artifact_id,
                next_revision,
                final_status,
                next_summary,
                ts
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO project_module_memories
             (id, project_id, owner_user_id, module_key, scope_type, category, content, status,
              importance, source_conversation_id, source_message_id, source_task_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'user', 'requirement', ?5, 'candidate', 7, ?6, ?7, ?8, ?9, ?9)",
            params![new_id("pmm"), project_id, user_id, UI_TUNER_MODULE_KEY,
                bounded_text(&user_intent, 800), conversation_id, source_message_id, task_id, ts],
        )?;
        tx.execute(
            "UPDATE project_module_conversations
             SET status = ?1, last_task_id = ?2, updated_at = ?3
             WHERE project_id = ?4 AND user_id = ?5 AND module_key = ?6 AND conversation_id = ?7",
            params![
                final_status,
                task_id,
                ts,
                project_id,
                user_id,
                UI_TUNER_MODULE_KEY,
                conversation_id
            ],
        )?;
        if success {
            tx.execute(
                "UPDATE project_module_workspaces
                 SET stable_summary = ?1, memory_revision = ?2, last_checkpoint_id = ?3,
                     active_conversation_id = ?4, updated_at = ?5
                 WHERE project_id = ?6 AND user_id = ?7 AND module_key = ?8",
                params![
                    next_summary,
                    next_revision,
                    checkpoint_id,
                    conversation_id,
                    ts,
                    project_id,
                    user_id,
                    UI_TUNER_MODULE_KEY
                ],
            )?;
        } else {
            tx.execute(
                "UPDATE project_module_workspaces SET memory_revision = ?1, updated_at = ?2
                 WHERE project_id = ?3 AND user_id = ?4 AND module_key = ?5",
                params![next_revision, ts, project_id, user_id, UI_TUNER_MODULE_KEY],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }
}

fn build_stable_summary(previous: &str, intent: &str, reply: &str, success: bool) -> String {
    let outcome = if success { "已完成" } else { "未完成" };
    bounded_text(
        &format!(
            "{}\n\n最近任务（{}）：{}\n执行结果：{}",
            bounded_text(previous, 1_500),
            outcome,
            bounded_text(intent, 600),
            bounded_text(reply, 1_200)
        ),
        3_500,
    )
}

fn bounded_text(value: &str, max_chars: usize) -> String {
    let clean = value.trim();
    if clean.chars().count() <= max_chars {
        return clean.to_string();
    }
    let mut out = clean
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    out.push('…');
    out
}
