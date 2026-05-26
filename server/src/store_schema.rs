//! SQLite Schema 迁移管理器。
//!
//! 所有表结构变更必须通过 `MIGRATIONS` 列表追加新版本。
//! v1 包含初始全量表结构（`IF NOT EXISTS` 确保在已有数据库上幂等运行）。
//! v2 包含历史 `add_column_if_missing` 补丁（已内化为正式迁移）。
//! 服务器启动时调用 [`apply_migrations`]，首次运行会建立迁移记录表并顺序应用。

use anyhow::Result;
use rusqlite::{params, Connection};

// ── 公开入口 ──────────────────────────────────────────────────────────────────

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
            let now = chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            conn.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, now],
            )?;
        }
    }
    Ok(())
}

// ── 迁移表 ────────────────────────────────────────────────────────────────────

/// (版本号, 描述, 迁移函数)
///
/// **追加规则**：只能向末尾追加；禁止修改已发布版本的内容；每次发布新 DDL
/// 都作为新版本条目添加，服务器在下次启动时自动检测并应用。
static MIGRATIONS: &[(u32, &str, fn(&Connection) -> Result<()>)] = &[
    (1, "初始全量表结构（幂等）", migration_v1),
    (2, "补充缺失列与辅助索引（幂等）", migration_v2),
    (3, "将所有现有项目设为公开可见（一次性）", migration_v3),
];

// ── v1：初始表结构 ────────────────────────────────────────────────────────────

fn migration_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
          id            TEXT PRIMARY KEY,
          phone         TEXT UNIQUE,
          email         TEXT UNIQUE,
          password_hash TEXT NOT NULL,
          nickname      TEXT,
          role          TEXT NOT NULL DEFAULT 'user',
          status        TEXT NOT NULL DEFAULT 'active',
          created_at    TEXT NOT NULL,
          updated_at    TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
          id          TEXT PRIMARY KEY,
          user_id     TEXT NOT NULL,
          token_hash  TEXT NOT NULL UNIQUE,
          device_name TEXT,
          apk_version TEXT,
          expires_at  TEXT NOT NULL,
          created_at  TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS user_friends (
          user_id TEXT NOT NULL,
          friend_user_id TEXT NOT NULL,
          created_at TEXT NOT NULL,
          PRIMARY KEY (user_id, friend_user_id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (friend_user_id) REFERENCES users(id),
          CHECK (user_id != friend_user_id)
        );

        CREATE TABLE IF NOT EXISTS friend_messages (
          id TEXT PRIMARY KEY,
          sender_user_id TEXT NOT NULL,
          receiver_user_id TEXT NOT NULL,
          content TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (sender_user_id) REFERENCES users(id),
          FOREIGN KEY (receiver_user_id) REFERENCES users(id),
          CHECK (sender_user_id != receiver_user_id)
        );

        CREATE TABLE IF NOT EXISTS projects (
          id             TEXT PRIMARY KEY,
          name           TEXT NOT NULL,
          description    TEXT,
          workspace_key  TEXT NOT NULL UNIQUE,
          template       TEXT NOT NULL DEFAULT 'android',
          source_type    TEXT NOT NULL DEFAULT 'template',
          repo_url       TEXT,
          branch         TEXT,
          workspace_path TEXT,
          status         TEXT NOT NULL DEFAULT 'active',
          created_by     TEXT NOT NULL,
          created_at     TEXT NOT NULL,
          updated_at     TEXT NOT NULL,
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_members (
          project_id TEXT NOT NULL,
          user_id    TEXT NOT NULL,
          role       TEXT NOT NULL,
          created_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS conversations (
          project_id TEXT NOT NULL,
          user_id    TEXT NOT NULL,
          id         TEXT NOT NULL,
          title      TEXT,
          status     TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id, id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS tasks (
          id                TEXT PRIMARY KEY,
          project_id        TEXT NOT NULL,
          user_id           TEXT NOT NULL,
          conversation_id   TEXT,
          client_request_id TEXT,
          message           TEXT NOT NULL,
          status            TEXT NOT NULL,
          git_branch        TEXT,
          git_commit        TEXT,
          apk_url           TEXT,
          error             TEXT,
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_events (
          id           TEXT PRIMARY KEY,
          project_id   TEXT NOT NULL,
          user_id      TEXT,
          event_type   TEXT NOT NULL,
          payload_json TEXT,
          created_at   TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS artifacts (
          id          TEXT PRIMARY KEY,
          project_id  TEXT NOT NULL,
          task_id     TEXT,
          file_name   TEXT NOT NULL,
          file_path   TEXT NOT NULL,
          sha256      TEXT,
          size_bytes  INTEGER,
          created_at  TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id)    REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS messages (
          id              TEXT PRIMARY KEY,
          project_id      TEXT NOT NULL,
          conversation_id TEXT,
          task_id         TEXT,
          user_id         TEXT,
          role            TEXT NOT NULL,
          content         TEXT NOT NULL,
          created_at      TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id)    REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS task_events (
          id         TEXT PRIMARY KEY,
          task_id    TEXT NOT NULL,
          event_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS agent_native_sessions (
          id                TEXT PRIMARY KEY,
          project_id        TEXT NOT NULL,
          user_id           TEXT NOT NULL,
          conversation_id   TEXT NOT NULL,
          provider          TEXT NOT NULL,
          agent_id          TEXT NOT NULL,
          workspace_path    TEXT NOT NULL,
          native_session_id TEXT NOT NULL,
          chat_bootstrapped INTEGER NOT NULL DEFAULT 0,
          dev_bootstrapped  INTEGER NOT NULL DEFAULT 0,
          status            TEXT NOT NULL DEFAULT 'active',
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          UNIQUE(project_id, user_id, conversation_id, provider, agent_id, workspace_path),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS ws_task_log (
          workspace_user_id TEXT PRIMARY KEY,
          message           TEXT NOT NULL,
          status            TEXT NOT NULL,
          started_at        TEXT NOT NULL,
          finished_at       TEXT
        );
        "#,
    )?;
    Ok(())
}

// ── v2：补充缺失列与索引（历史补丁，现为正式迁移）────────────────────────────

fn migration_v2(conn: &Connection) -> Result<()> {
    // 这些列已内化进 v1 DDL，但对升级前的旧数据库仍需幂等补充
    add_column_if_missing(conn, "projects", "source_type",    "source_type    TEXT NOT NULL DEFAULT 'template'")?;
    add_column_if_missing(conn, "projects", "repo_url",       "repo_url       TEXT")?;
    add_column_if_missing(conn, "projects", "branch",         "branch         TEXT")?;
    add_column_if_missing(conn, "projects", "workspace_path", "workspace_path TEXT")?;
    add_column_if_missing(conn, "tasks",    "conversation_id",   "conversation_id   TEXT")?;
    add_column_if_missing(conn, "tasks",    "client_request_id", "client_request_id TEXT")?;
    add_column_if_missing(conn, "messages", "conversation_id",   "conversation_id   TEXT")?;
    add_column_if_missing(conn, "agent_native_sessions", "chat_bootstrapped", "chat_bootstrapped INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "agent_native_sessions", "dev_bootstrapped",  "dev_bootstrapped  INTEGER NOT NULL DEFAULT 0")?;
    add_column_if_missing(conn, "sessions", "apk_version", "apk_version TEXT")?;
    // 项目商店：公开可见性 + 加入方式
    add_column_if_missing(
        conn,
        "projects",
        "is_public",
        "is_public INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "join_mode",
        "join_mode TEXT NOT NULL DEFAULT 'open'",
    )?;
    // 项目成员角色补全（role 字段已存在，仅确保索引存在）
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_project_members_user
         ON project_members(user_id)",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_client_request
         ON tasks(project_id, user_id, conversation_id, client_request_id)
         WHERE client_request_id IS NOT NULL",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_events_task_id
         ON task_events(task_id, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_friends_friend
         ON user_friends(friend_user_id, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_friend_messages_pair_created
         ON friend_messages(sender_user_id, receiver_user_id, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_friend_messages_receiver_created
         ON friend_messages(receiver_user_id, sender_user_id, created_at)",
        [],
    )?;
    Ok(())
}

// ── v3：将所有现有项目设为公开 ────────────────────────────────────────────────

fn migration_v3(conn: &Connection) -> Result<()> {
    // 将迁移前已存在的项目全部设为公开（is_public=0 的都是历史默认値，并非用户主动设为私有）
    conn.execute(
        "UPDATE projects SET is_public = 1 WHERE status != 'deleted'",
        [],
    )?;
    Ok(())
}

// ── 内部工具 ──────────────────────────────────────────────────────────────────

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {} ADD COLUMN {}", table, definition),
            [],
        )?;
    }
    Ok(())
}