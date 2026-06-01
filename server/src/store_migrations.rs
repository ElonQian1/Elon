//! SQLite 迁移注册表与所有迁移函数。
//!
//! **追加规则**：只能在 `MIGRATIONS` 末尾追加新条目；禁止修改已发布版本的内容。
//! 每次新增表结构变更时添加新版本，服务器下次启动时自动检测并应用。

use anyhow::Result;
use rusqlite::Connection;

/// 迁移注册表：(版本号, 描述, 迁移函数)
pub(crate) static MIGRATIONS: &[(u32, &str, fn(&Connection) -> Result<()>)] = &[
    (1, "初始全量表结构（幂等）", migration_v1),
    (2, "补充缺失列与辅助索引（幂等）", migration_v2),
    (3, "将所有现有项目设为公开可见（一次性）", migration_v3),
    (4, "好友会话已读状态与未读提醒", migration_v4),
    (5, "用户头像数据（个人资料上传）", migration_v5),
    (6, "好友群聊基础表与未读状态", migration_v6),
    (7, "项目空间频道与共享频道消息", migration_v7),
    (8, "好友与群聊消息附件引用", migration_v8),
    (9, "同一用户禁止重名活跃项目", migration_v9),
    (
        10,
        "tasks.codex_thread_id + conversation_timeline 视图",
        migration_v10,
    ),
    (
        11,
        "projects 构建缓存（last_build_sha / last_build_apk_url）",
        migration_v11,
    ),
    (12, "好友聊天 EL 助手上下文消息", migration_v12),
    (13, "每日编译配额表（build_quota）", migration_v13),
    (14, "项目意见频道建议状态", migration_v14),
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

        CREATE TABLE IF NOT EXISTS friend_read_states (
          user_id TEXT NOT NULL,
          friend_user_id TEXT NOT NULL,
          last_read_at TEXT NOT NULL,
          PRIMARY KEY (user_id, friend_user_id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (friend_user_id) REFERENCES users(id),
          CHECK (user_id != friend_user_id)
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

// ── v2：补充缺失列与索引 ──────────────────────────────────────────────────────

fn migration_v2(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "projects",
        "source_type",
        "source_type    TEXT NOT NULL DEFAULT 'template'",
    )?;
    add_column_if_missing(conn, "projects", "repo_url", "repo_url       TEXT")?;
    add_column_if_missing(conn, "projects", "branch", "branch         TEXT")?;
    add_column_if_missing(conn, "projects", "workspace_path", "workspace_path TEXT")?;
    add_column_if_missing(conn, "tasks", "conversation_id", "conversation_id   TEXT")?;
    add_column_if_missing(conn, "tasks", "client_request_id", "client_request_id TEXT")?;
    add_column_if_missing(
        conn,
        "messages",
        "conversation_id",
        "conversation_id   TEXT",
    )?;
    add_column_if_missing(
        conn,
        "agent_native_sessions",
        "chat_bootstrapped",
        "chat_bootstrapped INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "agent_native_sessions",
        "dev_bootstrapped",
        "dev_bootstrapped  INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(conn, "sessions", "apk_version", "apk_version TEXT")?;
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
    conn.execute(
        "UPDATE projects SET is_public = 1 WHERE status != 'deleted'",
        [],
    )?;
    Ok(())
}

// ── v4：好友会话已读状态 ─────────────────────────────────────────────────────

fn migration_v4(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS friend_read_states (
          user_id TEXT NOT NULL,
          friend_user_id TEXT NOT NULL,
          last_read_at TEXT NOT NULL,
          PRIMARY KEY (user_id, friend_user_id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (friend_user_id) REFERENCES users(id),
          CHECK (user_id != friend_user_id)
        );

        CREATE INDEX IF NOT EXISTS idx_friend_read_states_user
          ON friend_read_states(user_id, friend_user_id);
        "#,
    )?;
    Ok(())
}

// ── v5：用户头像 ─────────────────────────────────────────────────────────────

fn migration_v5(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "users", "avatar_data_url", "avatar_data_url TEXT")?;
    Ok(())
}

// ── v6：好友群聊 ─────────────────────────────────────────────────────────────

fn migration_v6(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS friend_groups (
          id            TEXT PRIMARY KEY,
          name          TEXT NOT NULL,
          owner_user_id TEXT NOT NULL,
          created_at    TEXT NOT NULL,
          updated_at    TEXT NOT NULL,
          FOREIGN KEY (owner_user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS friend_group_members (
          group_id      TEXT NOT NULL,
          user_id       TEXT NOT NULL,
          created_at    TEXT NOT NULL,
          last_read_at  TEXT,
          PRIMARY KEY (group_id, user_id),
          FOREIGN KEY (group_id) REFERENCES friend_groups(id),
          FOREIGN KEY (user_id)  REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS friend_group_messages (
          id             TEXT PRIMARY KEY,
          group_id       TEXT NOT NULL,
          sender_user_id TEXT NOT NULL,
          content        TEXT NOT NULL,
          created_at     TEXT NOT NULL,
          FOREIGN KEY (group_id)       REFERENCES friend_groups(id),
          FOREIGN KEY (sender_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_friend_group_members_user
          ON friend_group_members(user_id, created_at);

        CREATE INDEX IF NOT EXISTS idx_friend_group_messages_group_created
          ON friend_group_messages(group_id, created_at);
        "#,
    )?;
    Ok(())
}

// ── v7：项目空间频道 ──────────────────────────────────────────────────────────

fn migration_v7(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_channels (
          id         TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          name       TEXT NOT NULL,
          kind       TEXT NOT NULL,
          position   INTEGER NOT NULL DEFAULT 0,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          UNIQUE(project_id, kind),
          FOREIGN KEY (project_id) REFERENCES projects(id)
        );

        CREATE TABLE IF NOT EXISTS project_channel_messages (
          id             TEXT PRIMARY KEY,
          project_id     TEXT NOT NULL,
          channel_id     TEXT NOT NULL,
          sender_user_id TEXT,
          kind           TEXT NOT NULL DEFAULT 'text',
          content        TEXT NOT NULL,
          task_id        TEXT,
          created_at     TEXT NOT NULL,
          FOREIGN KEY (project_id)     REFERENCES projects(id),
          FOREIGN KEY (channel_id)     REFERENCES project_channels(id),
          FOREIGN KEY (sender_user_id) REFERENCES users(id),
          FOREIGN KEY (task_id)        REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS project_channel_read_states (
          project_id   TEXT NOT NULL,
          channel_id   TEXT NOT NULL,
          user_id      TEXT NOT NULL,
          last_read_at TEXT NOT NULL,
          PRIMARY KEY (project_id, channel_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (channel_id) REFERENCES project_channels(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_channels_project_position
          ON project_channels(project_id, position);

        CREATE INDEX IF NOT EXISTS idx_project_channel_messages_channel_created
          ON project_channel_messages(project_id, channel_id, created_at);

        CREATE INDEX IF NOT EXISTS idx_project_channel_read_states_user
          ON project_channel_read_states(user_id, project_id);
        "#,
    )?;
    Ok(())
}

// ── v8：附件引用 ──────────────────────────────────────────────────────────────

fn migration_v8(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "friend_messages",
        "attachments_json",
        "attachments_json TEXT",
    )?;
    add_column_if_missing(
        conn,
        "friend_group_messages",
        "attachments_json",
        "attachments_json TEXT",
    )?;
    Ok(())
}

// ── v9：同一用户禁止重名活跃项目 ─────────────────────────────────────────────

fn migration_v9(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        UPDATE projects
           SET status = 'deleted'
         WHERE status != 'deleted'
           AND id NOT IN (
               SELECT MIN(id)
                 FROM projects
                WHERE status != 'deleted'
             GROUP BY created_by, name
           );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_owner_name_active
          ON projects(created_by, name)
          WHERE status != 'deleted';
        "#,
    )?;
    Ok(())
}

// ── v10：会话诊断列与视图 ─────────────────────────────────────────────────────

fn migration_v10(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "tasks", "codex_thread_id", "codex_thread_id TEXT")?;

    conn.execute_batch(
        r#"
        DROP VIEW IF EXISTS conversation_timeline;
        CREATE VIEW conversation_timeline AS
        SELECT
            m.created_at                              AS time,
            'message'                                 AS kind,
            COALESCE(t.project_id, m.project_id)      AS project_id,
            p.name                                    AS project_name,
            m.conversation_id                         AS conversation_id,
            m.task_id                                 AS task_id,
            t.codex_thread_id                         AS codex_thread_id,
            m.role                                    AS role,
            m.content                                 AS content,
            NULL                                      AS event_type,
            NULL                                      AS event_detail
        FROM messages m
        LEFT JOIN tasks t ON t.id = m.task_id
        LEFT JOIN projects p ON p.id = COALESCE(t.project_id, m.project_id)

        UNION ALL

        SELECT
            te.created_at                             AS time,
            'task_event'                              AS kind,
            t.project_id                              AS project_id,
            p.name                                    AS project_name,
            t.conversation_id                         AS conversation_id,
            te.task_id                                AS task_id,
            t.codex_thread_id                         AS codex_thread_id,
            NULL                                      AS role,
            te.event_json                             AS content,
            json_extract(te.event_json, '$.type')     AS event_type,
            json_extract(te.event_json, '$.text')     AS event_detail
        FROM task_events te
        JOIN tasks t ON t.id = te.task_id
        LEFT JOIN projects p ON p.id = t.project_id;
        "#,
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tasks_project_conversation_created
         ON tasks(project_id, conversation_id, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_project_conversation_created
         ON messages(project_id, conversation_id, created_at)",
        [],
    )?;

    Ok(())
}

// ── v11：项目构建缓存 ─────────────────────────────────────────────────────────

fn migration_v11(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "last_build_sha", "last_build_sha     TEXT")?;
    add_column_if_missing(
        conn,
        "projects",
        "last_build_apk_url",
        "last_build_apk_url TEXT",
    )?;
    Ok(())
}

// ── v12：好友聊天 EL 助手上下文消息 ─────────────────────────────────────────

fn migration_v12(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "friend_messages",
        "context_user_id",
        "context_user_id TEXT",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_friend_messages_ai_context
         ON friend_messages(receiver_user_id, context_user_id, created_at)",
        [],
    )?;
    Ok(())
}

fn migration_v13(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS build_quota (
          user_id  TEXT NOT NULL,
          date     TEXT NOT NULL,
          count    INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (user_id, date)
        );
        CREATE INDEX IF NOT EXISTS idx_build_quota_user_date
          ON build_quota(user_id, date);
        "#,
    )?;
    Ok(())
}

fn migration_v14(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "project_channel_messages",
        "suggestion_status",
        "suggestion_status TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_channel_messages",
        "suggestion_resolved_by",
        "suggestion_resolved_by TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_channel_messages",
        "suggestion_resolved_at",
        "suggestion_resolved_at TEXT",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_project_channel_messages_suggestions
         ON project_channel_messages(project_id, channel_id, suggestion_status, created_at)",
        [],
    )?;
    Ok(())
}

// ── 内部工具 ──────────────────────────────────────────────────────────────────

pub(crate) fn add_column_if_missing(
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
