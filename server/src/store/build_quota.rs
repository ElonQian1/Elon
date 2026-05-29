//! 每用户每日编译配额。
//!
//! 每次本地构建前调用 [`Store::check_and_increment_build_quota`]：
//! - 若当日计数 < limit，则原子 +1 并返回 `Ok(count)`
//! - 若已达上限，返回 `Err`（含剩余额度说明）

use anyhow::{anyhow, Result};
use rusqlite::params;

use super::Store;

impl Store {
    /// 检查并递增用户当日编译次数。
    ///
    /// `limit` 通常来自环境变量 `DAILY_BUILD_QUOTA`（默认 10）。
    /// 返回值：`Ok(new_count)` 表示本次已计入，`Err` 表示已超限。
    pub fn check_and_increment_build_quota(&self, user_id: &str, limit: i64) -> Result<i64> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let conn = self.conn()?;

        // 原子 UPSERT：若今日无记录则插入 count=1；有记录且未超限则 +1；已超限则 CHECK 失败
        let rows = conn.execute(
            "INSERT INTO build_quota (user_id, date, count) VALUES (?1, ?2, 1)
             ON CONFLICT (user_id, date) DO UPDATE SET count = count + 1
             WHERE count < ?3",
            params![user_id, today, limit],
        )?;

        if rows == 0 {
            // UPSERT 的 WHERE 未命中，说明已达上限
            let current: i64 = conn.query_row(
                "SELECT count FROM build_quota WHERE user_id = ?1 AND date = ?2",
                params![user_id, today],
                |r| r.get(0),
            )?;
            return Err(anyhow!(
                "今日构建次数（{}次）已达上限（{}次/天）。明天刷新，或联系管理员调整配额。",
                current,
                limit
            ));
        }

        // 返回本次写入后的计数
        let new_count: i64 = conn.query_row(
            "SELECT count FROM build_quota WHERE user_id = ?1 AND date = ?2",
            params![user_id, today],
            |r| r.get(0),
        )?;
        Ok(new_count)
    }
}
