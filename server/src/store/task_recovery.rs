use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

use super::{new_id, now, Store};

const SERVER_UPDATE_RECOVERY_ERROR: &str = "server update recovery pending";
const SERVER_UPDATE_RECOVERY_TIMEOUT_ERROR: &str = "server update recovery timed out";
const CHANNEL_TASK_SERVER_UPDATE_RECOVERY_RESULT: &str =
    "恢复失败：服务器或 Win 端更新升级后，通信没有在预期时间内自动恢复。请点击“继续”让 AI 检查当前工作区后接着处理。";
const CHANNEL_TASK_STALE_RESULT: &str =
    "任务失败：PC 节点断线、任务超时或通信长期没有结果。请点击“继续”让 AI 检查当前工作区后接着处理。";

#[derive(Debug, Clone)]
struct ChannelTaskTarget {
    task_id: String,
    project_id: String,
    channel_id: String,
    status: String,
}

impl Store {
    /// 服务重启恢复：把频道 AI 任务保留为非终态，并补一条结构化恢复进度。
    ///
    /// `project_channel_messages` 是 PC 页面任务卡的权威输入；这里插入 `ai_progress`
    /// 而不是 `ai_result`，避免把服务器发布/重启误展示成用户任务失败。
    pub fn mark_recovering_running_tasks_after_server_restart(&self) -> Result<usize> {
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let targets = running_channel_task_targets(&tx)?;
        let n = tx.execute(
            "UPDATE tasks
             SET status = 'recovering',
                 error = COALESCE(error, ?2),
                 updated_at = ?1
             WHERE status = 'running'",
            params![now(), SERVER_UPDATE_RECOVERY_ERROR],
        )?;
        insert_missing_channel_ai_progress(
            &tx,
            &targets,
            &server_update_recovering_progress_message(),
        )?;
        tx.commit()?;
        Ok(n)
    }

    /// 超时清理：把长期 running/recovering 的任务置为 failed，并补齐频道终态。
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
        let targets = stale_channel_task_targets(&tx, &cutoff)?
            .into_iter()
            .filter(|target| !excluded.contains(target.task_id.as_str()))
            .collect::<Vec<_>>();
        let n = if excluded.is_empty() {
            tx.execute(
                "UPDATE tasks
                 SET status = 'failed',
                     error = CASE
                         WHEN status = 'recovering' THEN ?3
                         ELSE COALESCE(error, 'PC节点断线或任务超时自动终止')
                     END,
                     updated_at = ?1
                 WHERE (status = 'running' AND created_at < ?2)
                    OR (status = 'recovering' AND updated_at < ?2)",
                params![now(), cutoff, SERVER_UPDATE_RECOVERY_TIMEOUT_ERROR],
            )?
        } else {
            let placeholders = std::iter::repeat("?")
                .take(excluded.len())
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "UPDATE tasks
                 SET status = 'failed',
                     error = CASE
                         WHEN status = 'recovering' THEN ?
                         ELSE COALESCE(error, 'PC节点断线或任务超时自动终止')
                     END,
                     updated_at = ?
                 WHERE ((status = 'running' AND created_at < ?)
                     OR (status = 'recovering' AND updated_at < ?))
                   AND id NOT IN ({placeholders})"
            );
            let now_value = now();
            let mut values = vec![
                SERVER_UPDATE_RECOVERY_TIMEOUT_ERROR.to_string(),
                now_value,
                cutoff.clone(),
                cutoff.clone(),
            ];
            values.extend(excluded.iter().map(|value| (*value).to_string()));
            tx.execute(&sql, rusqlite::params_from_iter(values.iter()))?
        };
        insert_missing_stale_channel_ai_results(&tx, &targets)?;
        tx.commit()?;
        Ok(n)
    }
}

fn server_update_recovering_progress_message() -> String {
    json!({
        "type": "runtime_status",
        "phase": "server_updating",
        "runtime": "一龙",
        "status": "recovering",
        "message": "服务器正在更新升级，通信临时中断，会自动恢复。任务现场已保留，正在等待 Win 端节点重连和过程回放。",
        "auto_recover": true,
    })
    .to_string()
}

fn running_channel_task_targets(conn: &Connection) -> Result<Vec<ChannelTaskTarget>> {
    channel_task_targets(conn, "t.status = 'running'", [])
}

fn stale_channel_task_targets(conn: &Connection, cutoff: &str) -> Result<Vec<ChannelTaskTarget>> {
    channel_task_targets(
        conn,
        "(t.status = 'running' AND t.created_at < ?1) OR (t.status = 'recovering' AND t.updated_at < ?1)",
        [cutoff],
    )
}

fn channel_task_targets<'a, P>(
    conn: &Connection,
    status_predicate: &str,
    params_values: P,
) -> Result<Vec<ChannelTaskTarget>>
where
    P: IntoIterator<Item = &'a str>,
{
    let sql = format!(
        "SELECT DISTINCT t.id, m.project_id, m.channel_id, t.status
         FROM tasks t
         JOIN project_channel_messages m
           ON m.task_id = t.id
          AND m.kind = 'ai_task'
         WHERE ({status_predicate})
           AND NOT EXISTS (
             SELECT 1
             FROM project_channel_messages r
             WHERE r.project_id = m.project_id
               AND r.channel_id = m.channel_id
               AND r.task_id = t.id
               AND r.kind = 'ai_result'
           )"
    );
    let values = params_values.into_iter().collect::<Vec<_>>();
    let mut stmt = conn.prepare(&sql)?;
    let targets = stmt
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok(ChannelTaskTarget {
                task_id: row.get(0)?,
                project_id: row.get(1)?,
                channel_id: row.get(2)?,
                status: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(targets)
}

fn insert_missing_channel_ai_progress(
    conn: &Connection,
    targets: &[ChannelTaskTarget],
    content: &str,
) -> Result<usize> {
    insert_missing_channel_messages(conn, targets, "ai_progress", content)
}

fn insert_missing_stale_channel_ai_results(
    conn: &Connection,
    targets: &[ChannelTaskTarget],
) -> Result<usize> {
    let mut inserted = 0;
    for target in targets {
        let content = if target.status == "recovering" {
            CHANNEL_TASK_SERVER_UPDATE_RECOVERY_RESULT
        } else {
            CHANNEL_TASK_STALE_RESULT
        };
        inserted += insert_missing_channel_messages(
            conn,
            std::slice::from_ref(target),
            "ai_result",
            content,
        )?;
    }
    Ok(inserted)
}

fn insert_missing_channel_messages(
    conn: &Connection,
    targets: &[ChannelTaskTarget],
    kind: &str,
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
                   AND kind = ?4
                   AND content = ?5
                 LIMIT 1",
                params![
                    target.project_id,
                    target.channel_id,
                    target.task_id,
                    kind,
                    content
                ],
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
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
            params![
                new_id("pcm"),
                target.project_id,
                target.channel_id,
                kind,
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
mod task_recovery_tests {
    include!("task_recovery_tests.rs");
}
