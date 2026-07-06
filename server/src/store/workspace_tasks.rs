use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use super::common::now;
use super::Store;

impl Store {
    pub fn ws_task_started(&self, workspace_user_id: &str, message: &str) -> Result<()> {
        self.conn()?.execute(
            "INSERT OR REPLACE INTO ws_task_log (workspace_user_id, message, status, started_at, finished_at)
             VALUES (?1, ?2, 'running', ?3, NULL)",
            params![workspace_user_id, message, now()],
        )?;
        Ok(())
    }

    /// 标记任务完成（status: done / error）
    pub fn ws_task_finished(&self, workspace_user_id: &str, status: &str) -> Result<()> {
        self.conn()?.execute(
            "UPDATE ws_task_log SET status = ?1, finished_at = ?2 WHERE workspace_user_id = ?3",
            params![status, now(), workspace_user_id],
        )?;
        Ok(())
    }

    /// 查询该用户是否有被中断的任务，返回中断时的消息内容
    pub fn get_interrupted_ws_task(&self, workspace_user_id: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let msg: Option<String> = conn
            .query_row(
                "SELECT message FROM ws_task_log WHERE workspace_user_id = ?1 AND status = 'interrupted'",
                params![workspace_user_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(msg)
    }

    /// 服务器启动时：将所有 running 状态的任务标记为 interrupted
    pub fn mark_interrupted_running_ws_tasks(&self) -> Result<usize> {
        let n = self.conn()?.execute(
            "UPDATE ws_task_log SET status = 'interrupted', finished_at = ?1 WHERE status = 'running'",
            params![now()],
        )?;
        Ok(n)
    }

    pub fn mark_interrupted_running_tasks(&self) -> Result<usize> {
        let n = self.conn()?.execute(
            "UPDATE tasks
             SET status = 'interrupted',
                 error = COALESCE(error, 'server update/restart interrupted task communication'),
                 updated_at = ?1
             WHERE status = 'running'",
            params![now()],
        )?;
        Ok(n)
    }

    /// 定期清理：将 running 超过指定秒数的任务标记为 failed。
    /// 用于防止 PC 节点断线但任务因异常未收到 CliDone 而永久卡住。
    pub fn mark_stale_running_tasks(&self, older_than_secs: u64) -> Result<usize> {
        use chrono::{Duration, Utc};
        let cutoff = (Utc::now() - Duration::seconds(older_than_secs as i64)).to_rfc3339();
        let n = self.conn()?.execute(
            "UPDATE tasks
             SET status = 'failed',
                 error = COALESCE(error, 'PC节点通信自动恢复超时'),
                 updated_at = ?1
             WHERE status = 'running'
               AND created_at < ?2",
            params![now(), cutoff],
        )?;
        Ok(n)
    }
}
