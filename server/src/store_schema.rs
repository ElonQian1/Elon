//! SQLite Schema 迁移管理器。
//!
//! 所有表结构变更必须通过 [`crate::store_migrations::MIGRATIONS`] 列表追加新版本。
//! v1 包含初始全量表结构（`IF NOT EXISTS` 确保在已有数据库上幂等运行）。
//! 服务器启动时调用 [`apply_migrations`]，首次运行会建立迁移记录表并顺序应用。

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::store_migrations::MIGRATIONS;

/// 将所有尚未应用的 schema 迁移顺序执行到数据库。
///
/// 幂等：已应用的版本不会重复执行；`schema_migrations` 表不存在时自动创建。
pub(crate) fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version    INTEGER PRIMARY KEY,
           applied_at TEXT    NOT NULL
         );",
    )?;

    let applied: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;

    for (version, description, apply_fn) in MIGRATIONS {
        if *version > applied {
            tracing::info!("数据库迁移 v{}: {}", version, description);
            apply_fn(conn)?;
            let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, now],
            )?;
        }
    }
    Ok(())
}
