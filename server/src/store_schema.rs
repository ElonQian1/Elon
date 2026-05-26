use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS users (
          id TEXT PRIMARY KEY,
          phone TEXT UNIQUE,
          email TEXT UNIQUE,
          password_hash TEXT NOT NULL,
          nickname TEXT,
          role TEXT NOT NULL DEFAULT 'user',
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
          id TEXT PRIMARY KEY,
          user_id TEXT NOT NULL,
          token_hash TEXT NOT NULL UNIQUE,
          device_name TEXT,
          expires_at TEXT NOT NULL,
          created_at TEXT NOT NULL,
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
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          description TEXT,
          workspace_key TEXT NOT NULL UNIQUE,
          template TEXT NOT NULL DEFAULT 'android',
          source_type TEXT NOT NULL DEFAULT 'template',
          repo_url TEXT,
          branch TEXT,
          workspace_path TEXT,
          status TEXT NOT NULL DEFAULT 'active',
          created_by TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_members (
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          role TEXT NOT NULL,
          created_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS conversations (
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          id TEXT NOT NULL,
          title TEXT,
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id, id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS tasks (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          conversation_id TEXT,
          client_request_id TEXT,
          message TEXT NOT NULL,
          status TEXT NOT NULL,
          git_branch TEXT,
          git_commit TEXT,
          apk_url TEXT,
          error TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_events (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id TEXT,
          event_type TEXT NOT NULL,
          payload_json TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS artifacts (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          task_id TEXT,
          file_name TEXT NOT NULL,
          file_path TEXT NOT NULL,
          sha256 TEXT,
          size_bytes INTEGER,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS messages (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          conversation_id TEXT,
          task_id TEXT,
          user_id TEXT,
          role TEXT NOT NULL,
          content TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS task_events (
          id TEXT PRIMARY KEY,
          task_id TEXT NOT NULL,
          event_json TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (task_id) REFERENCES tasks(id)
        );

        CREATE TABLE IF NOT EXISTS agent_native_sessions (
          id TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id TEXT NOT NULL,
          conversation_id TEXT NOT NULL,
          provider TEXT NOT NULL,
          agent_id TEXT NOT NULL,
          workspace_path TEXT NOT NULL,
          native_session_id TEXT NOT NULL,
          chat_bootstrapped INTEGER NOT NULL DEFAULT 0,
          dev_bootstrapped INTEGER NOT NULL DEFAULT 0,
          status TEXT NOT NULL DEFAULT 'active',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          UNIQUE(project_id, user_id, conversation_id, provider, agent_id, workspace_path),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS ws_task_log (
          workspace_user_id TEXT PRIMARY KEY,
          message TEXT NOT NULL,
          status TEXT NOT NULL,
          started_at TEXT NOT NULL,
          finished_at TEXT
        );
        "#,
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "source_type",
        "source_type TEXT NOT NULL DEFAULT 'template'",
    )?;
    add_column_if_missing(conn, "projects", "repo_url", "repo_url TEXT")?;
    add_column_if_missing(conn, "projects", "branch", "branch TEXT")?;
    add_column_if_missing(conn, "projects", "workspace_path", "workspace_path TEXT")?;
    add_column_if_missing(conn, "tasks", "conversation_id", "conversation_id TEXT")?;
    add_column_if_missing(conn, "tasks", "client_request_id", "client_request_id TEXT")?;
    add_column_if_missing(conn, "messages", "conversation_id", "conversation_id TEXT")?;
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
        "dev_bootstrapped INTEGER NOT NULL DEFAULT 0",
    )?;
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
