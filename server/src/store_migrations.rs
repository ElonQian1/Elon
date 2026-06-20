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
    (15, "用户长期记忆表", migration_v15),
    (16, "token 用量事件表 + 用户 token 配额表", migration_v16),
    (
        17,
        "人民币预存计费：用户余额、充值记录、扣费明细、计费配置",
        migration_v17,
    ),
    (18, "微信支付订单表", migration_v18),
    (19, "项目加入申请表（approval 审批流程）", migration_v19),
    (20, "PC 本地项目绑定节点 ID", migration_v20),
    (21, "分布式节点积分账本与节点凭证表", migration_v21),
    (22, "conversations.locked_agent_name 会话首次 CLI 锁定", migration_v22),
    (23, "node_credentials.device_name PC 设备展示名", migration_v23),
    (24, "user_memories 记忆作用域", migration_v24),
    (25, "收紧一龙自项目默认成员与加入权限", migration_v25),
    (26, "指定钱一龙账号为一龙自项目管理员", migration_v26),
    (27, "项目成员会话人类讨论消息", migration_v27),
    (28, "项目成员个人会话公开状态", migration_v28),
    (29, "PC 项目执行会话与工作区状态", migration_v29),
    (30, "token 用量与扣费事件原子对账字段", migration_v30),
    (31, "token 用量可信记账幂等键", migration_v31),
    (32, "PC 项目执行会话 token 用量字段", migration_v32),
    (33, "计费调用预授权冻结与对账摘要", migration_v33),
    (34, "非 CLI 算力预授权配置", migration_v34),
    (35, "PC 项目工作区健康快照", migration_v35),
    (36, "算力多单位计量明细账本", migration_v36),
    (37, "模型与算力计价规则配置表", migration_v37),
    (38, "计费自动对账告警表", migration_v38),
    (39, "扣费计价规则版本与价格快照", migration_v39),
    (40, "节点收益流水绑定真实扣费事件", migration_v40),
    (41, "PC 节点硬件画像快照", migration_v41),
    (42, "节点收益提现申请表", migration_v42),
    (43, "节点收益整数资金账本", migration_v43),
    (44, "节点算力执行证明与质量评分基础表", migration_v44),
    (45, "项目 APK 图标数据", migration_v45),
    (46, "project channel message reply parent", migration_v46),
    (47, "指定钱一龙为一龙自项目创建者与 owner", migration_v47),
    (48, "PC 硬盘节点项目仓库绑定", migration_v48),
    (49, "PC 硬盘节点 owner checkout 路径", migration_v49),
    (50, "项目展示别名", migration_v50),
    (51, "一龙自项目公开展示并审批加入", migration_v51),
    (52, "项目级 AI 运行权限授权", migration_v52),
    (53, "群聊 AI 文档、Context Pack 与总结帖", migration_v53),
    (54, "外部应用账号、默认群映射与授权码", migration_v54),
    (55, "项目代码身份去重索引", migration_v55),
    (56, "所有用户默认加入指定联合开发项目", migration_v56),
    (57, "fb2 外部应用 AI 回复试用额度配置", migration_v57),
    (58, "项目首页 landing manifest 云端快照", migration_v58),
    (59, "项目首页上传凭证", migration_v59),
    (60, "外部应用工具执行审计", migration_v60),
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
    add_column_if_missing(
        conn,
        "projects",
        "last_build_sha",
        "last_build_sha     TEXT",
    )?;
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

// ── v15：用户长期记忆表 ───────────────────────────────────────────────────────

fn migration_v15(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_memories (
          id             TEXT PRIMARY KEY,
          user_id        TEXT NOT NULL,
          content        TEXT NOT NULL,
          category       TEXT NOT NULL DEFAULT 'fact',
          importance     INTEGER NOT NULL DEFAULT 5,
          source_conv_id TEXT,
          created_at     TEXT NOT NULL,
          updated_at     TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_user_memories_user_importance
          ON user_memories(user_id, importance DESC, updated_at DESC);
        "#,
    )?;
    Ok(())
}

// ── v16：token 用量事件表 + 用户配额表 ───────────────────────────────────────────

fn migration_v16(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- token 用量事件（按调用记录）
        CREATE TABLE IF NOT EXISTS token_usage_events (
          id                  TEXT PRIMARY KEY,
          user_id             TEXT NOT NULL,
          feature             TEXT NOT NULL DEFAULT 'unknown',
          usage_mode          TEXT NOT NULL DEFAULT 'server_api_key',
          model               TEXT,
          input_tokens        INTEGER NOT NULL DEFAULT 0,
          cached_input_tokens INTEGER NOT NULL DEFAULT 0,
          output_tokens       INTEGER NOT NULL DEFAULT 0,
          reasoning_tokens    INTEGER NOT NULL DEFAULT 0,
          total_tokens        INTEGER NOT NULL DEFAULT 0,
          created_at          TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_token_usage_user_time
          ON token_usage_events(user_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_token_usage_model_time
          ON token_usage_events(model, created_at DESC);

        -- 用户 token 月度配额
        CREATE TABLE IF NOT EXISTS user_token_quota (
          user_id              TEXT PRIMARY KEY,
          monthly_token_limit  INTEGER,
          is_blocked           INTEGER NOT NULL DEFAULT 0,
          block_reason         TEXT,
          created_at           TEXT NOT NULL,
          updated_at           TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );
        "#,
    )?;
    Ok(())
}

// ── 内部工具 ───────────────────────────────────────────────────────────────────

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

// ── v17：人民币预存计费系统 ────────────────────────────────────────────────────

fn migration_v17(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- 用户余额（分为单位，不存小数）
        CREATE TABLE IF NOT EXISTS user_balance (
            user_id     TEXT PRIMARY KEY,
            balance_fen INTEGER NOT NULL DEFAULT 0,
            updated_at  TEXT NOT NULL
        );

        -- 充值记录
        CREATE TABLE IF NOT EXISTS recharge_records (
            id          TEXT PRIMARY KEY,
            user_id     TEXT NOT NULL,
            amount_fen  INTEGER NOT NULL,
            method      TEXT NOT NULL DEFAULT 'manual',
            operator_id TEXT NOT NULL DEFAULT 'admin',
            note        TEXT,
            created_at  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_recharge_user_time
            ON recharge_records(user_id, created_at DESC);

        -- 每次 LLM 调用的扣费明细
        CREATE TABLE IF NOT EXISTS billing_events (
            id                   TEXT PRIMARY KEY,
            user_id              TEXT NOT NULL,
            model                TEXT,
            input_tokens         INTEGER NOT NULL DEFAULT 0,
            cached_input_tokens  INTEGER NOT NULL DEFAULT 0,
            output_tokens        INTEGER NOT NULL DEFAULT 0,
            cost_rmb_fen         INTEGER NOT NULL DEFAULT 0,
            exchange_rate_x10000 INTEGER NOT NULL DEFAULT 73000,
            markup_x1000         INTEGER NOT NULL DEFAULT 1200,
            created_at           TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_billing_events_user_time
            ON billing_events(user_id, created_at DESC);

        -- 计费全局配置（键值对）
        CREATE TABLE IF NOT EXISTS billing_config (
            key        TEXT PRIMARY KEY,
            value      TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO billing_config (key, value, updated_at) VALUES
            ('usd_to_rmb_rate_x10000', '73000', datetime('now')),
            ('markup_x1000',           '1200',  datetime('now')),
            ('low_balance_threshold_fen', '100', datetime('now'));
        "#,
    )?;
    Ok(())
}

// ── v18：微信支付订单表 ────────────────────────────────────────────────────────

fn migration_v18(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- 微信支付待确认订单（仅用于回调时查找对应用户）
        CREATE TABLE IF NOT EXISTS wechat_pay_orders (
            out_trade_no   TEXT PRIMARY KEY,  -- 商户订单号
            user_id        TEXT NOT NULL,
            amount_fen     INTEGER NOT NULL,
            status         TEXT NOT NULL DEFAULT 'pending', -- pending | paid | failed
            wechat_tx_id   TEXT,             -- 微信交易号（paid 后填充）
            created_at     TEXT NOT NULL,
            updated_at     TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_wechat_pay_orders_user
            ON wechat_pay_orders(user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

// ── v19：项目加入申请表 ────────────────────────────────────────────────────────

fn migration_v19(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- 项目加入申请（join_mode='approval' 时使用）
        CREATE TABLE IF NOT EXISTS project_join_requests (
            id           TEXT PRIMARY KEY,
            project_id   TEXT NOT NULL,
            user_id      TEXT NOT NULL,
            message      TEXT,                               -- 申请留言
            status       TEXT NOT NULL DEFAULT 'pending',   -- pending | approved | rejected
            reviewed_by  TEXT,                              -- 审批人（owner）user_id
            reviewed_at  TEXT,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            UNIQUE(project_id, user_id),
            FOREIGN KEY (project_id) REFERENCES projects(id),
            FOREIGN KEY (user_id)    REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_join_requests_project_status
            ON project_join_requests(project_id, status, created_at);
        CREATE INDEX IF NOT EXISTS idx_join_requests_user
            ON project_join_requests(user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

// ── v20：PC 本地项目绑定节点 ID ────────────────────────────────────────────────

fn migration_v20(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "node_id", "node_id TEXT")?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_projects_node_id
          ON projects(node_id)
          WHERE node_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

// ── v21：分布式节点积分账本 ────────────────────────────────────────────────────

fn migration_v21(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_balances (
          user_id    TEXT PRIMARY KEY,
          credits    REAL NOT NULL DEFAULT 0,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS node_transactions (
          id                TEXT PRIMARY KEY,
          consumer_user_id  TEXT NOT NULL,
          provider_user_id  TEXT NOT NULL,
          node_id           TEXT NOT NULL,
          model_id          TEXT NOT NULL,
          prompt_tokens     INTEGER NOT NULL DEFAULT 0,
          completion_tokens INTEGER NOT NULL DEFAULT 0,
          charged_credits   REAL NOT NULL DEFAULT 0,
          settled_credits   REAL NOT NULL DEFAULT 0,
          platform_fee_rate REAL NOT NULL DEFAULT 0,
          created_at        TEXT NOT NULL,
          FOREIGN KEY (consumer_user_id) REFERENCES users(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_node_transactions_provider_time
          ON node_transactions(provider_user_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_node_transactions_consumer_time
          ON node_transactions(consumer_user_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS node_credentials (
          agent_id      TEXT PRIMARY KEY,
          secret_hash   TEXT NOT NULL,
          owner_user_id TEXT NOT NULL,
          label         TEXT NOT NULL DEFAULT '',
          created_at    TEXT NOT NULL,
          FOREIGN KEY (owner_user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_node_credentials_owner
          ON node_credentials(owner_user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

// ── v22：会话首次 CLI 软锁定 ──────────────────────────────────────────────────

fn migration_v22(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "conversations",
        "locked_agent_name",
        "locked_agent_name TEXT",
    )?;
    Ok(())
}

// ── v23：节点凭证记录 PC 设备名 ───────────────────────────────────────────────

fn migration_v23(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "node_credentials", "device_name", "device_name TEXT")?;
    Ok(())
}

// ── v24：用户记忆作用域 ─────────────────────────────────────────────────────

fn migration_v24(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "user_memories",
        "scope_type",
        "scope_type TEXT NOT NULL DEFAULT 'global'",
    )?;
    add_column_if_missing(conn, "user_memories", "scope_id", "scope_id TEXT")?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_user_memories_scope
          ON user_memories(user_id, scope_type, scope_id, importance DESC, updated_at DESC);
        "#,
    )?;
    Ok(())
}

// ── v25：一龙自项目不再默认让所有用户加入 ─────────────────────────────────────

fn migration_v25(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        UPDATE projects
           SET join_mode = 'readonly',
               updated_at = datetime('now')
         WHERE id = 'elon-self'
           AND status != 'deleted';

        INSERT OR IGNORE INTO project_members (project_id, user_id, role, created_at)
        SELECT id, created_by, 'owner', datetime('now')
          FROM projects
         WHERE id = 'elon-self'
           AND status != 'deleted';

        UPDATE project_members
           SET role = 'owner'
         WHERE project_id = 'elon-self'
           AND user_id IN (
             SELECT created_by
               FROM projects
              WHERE id = 'elon-self'
                AND status != 'deleted'
           );

        DELETE FROM project_members
         WHERE project_id = 'elon-self'
           AND user_id NOT IN (
             SELECT created_by
               FROM projects
              WHERE id = 'elon-self'
                AND status != 'deleted'
           );
        "#,
    )?;
    Ok(())
}

// ── v26：指定钱一龙账号为一龙自项目管理员 ─────────────────────────────────────

fn migration_v26(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        INSERT INTO project_members (project_id, user_id, role, created_at)
        SELECT 'elon-self', u.id, 'admin', datetime('now')
          FROM users u
          JOIN projects p ON p.id = 'elon-self' AND p.status != 'deleted'
         WHERE u.status = 'active'
           AND (u.phone = '15692409892' OR u.nickname = '钱一龙')
           AND u.id != p.created_by
        ON CONFLICT(project_id, user_id) DO UPDATE SET role = 'admin'
          WHERE project_members.role != 'owner';
        "#,
    )?;
    Ok(())
}

// ── v27：项目成员会话人类讨论消息 ─────────────────────────────────────────────

fn migration_v27(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_member_conversation_discussion_messages (
          id              TEXT PRIMARY KEY,
          project_id      TEXT NOT NULL,
          member_user_id  TEXT NOT NULL,
          conversation_id TEXT NOT NULL,
          sender_user_id  TEXT NOT NULL,
          content         TEXT NOT NULL,
          created_at      TEXT NOT NULL,
          FOREIGN KEY (project_id)     REFERENCES projects(id),
          FOREIGN KEY (member_user_id) REFERENCES users(id),
          FOREIGN KEY (sender_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_member_conversation_discussion_timeline
          ON project_member_conversation_discussion_messages(
            project_id, member_user_id, conversation_id, created_at
          );
        "#,
    )?;
    Ok(())
}

fn migration_v28(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "conversations",
        "is_public",
        "is_public INTEGER NOT NULL DEFAULT 1",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_conversations_project_user_public_updated
          ON conversations(project_id, user_id, is_public, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v29(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_execution_sessions (
          id                    TEXT PRIMARY KEY,
          project_id            TEXT NOT NULL,
          conversation_id       TEXT NOT NULL,
          user_id               TEXT NOT NULL,
          node_id               TEXT NOT NULL,
          request_id            TEXT NOT NULL UNIQUE,
          base_workspace_path   TEXT,
          active_workspace_path TEXT,
          branch                TEXT,
          isolated              INTEGER NOT NULL DEFAULT 0,
          status                TEXT NOT NULL DEFAULT 'running',
          merge_status          TEXT,
          last_error            TEXT,
          model                 TEXT,
          created_at            TEXT NOT NULL,
          updated_at            TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id)    REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_project_execution_sessions_latest
          ON project_execution_sessions(project_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_project_execution_sessions_conversation
          ON project_execution_sessions(project_id, conversation_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v30(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "token_usage_events",
        "accounting_status",
        "accounting_status TEXT NOT NULL DEFAULT 'not_billable'",
    )?;
    add_column_if_missing(
        conn,
        "token_usage_events",
        "billing_event_id",
        "billing_event_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "token_usage_events",
        "cost_rmb_fen",
        "cost_rmb_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "token_usage_events",
        "balance_after_fen",
        "balance_after_fen INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "token_usage_event_id",
        "token_usage_event_id TEXT",
    )?;
    conn.execute_batch(
        r#"
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_required_for_all_users', 'true', datetime('now'));

        CREATE INDEX IF NOT EXISTS idx_token_usage_accounting_status
          ON token_usage_events(accounting_status, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_token_usage_billing_event
          ON token_usage_events(billing_event_id)
          WHERE billing_event_id IS NOT NULL;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_events_token_usage_event
          ON billing_events(token_usage_event_id)
          WHERE token_usage_event_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

fn migration_v31(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "token_usage_events",
        "idempotency_key",
        "idempotency_key TEXT",
    )?;
    conn.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_token_usage_user_idempotency
          ON token_usage_events(user_id, idempotency_key)
          WHERE idempotency_key IS NOT NULL;
        "#,
    )?;
    Ok(())
}

fn migration_v32(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "prompt_tokens",
        "prompt_tokens INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "cached_input_tokens",
        "cached_input_tokens INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "completion_tokens",
        "completion_tokens INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "reasoning_tokens",
        "reasoning_tokens INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "total_tokens",
        "total_tokens INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "token_usage_event_id",
        "token_usage_event_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "project_execution_sessions",
        "billing_event_id",
        "billing_event_id TEXT",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_project_execution_sessions_node_time
          ON project_execution_sessions(node_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v33(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS billing_reservations (
          id                   TEXT PRIMARY KEY,
          user_id              TEXT NOT NULL,
          compute_call_id      TEXT NOT NULL,
          feature              TEXT NOT NULL,
          usage_mode           TEXT NOT NULL,
          model                TEXT,
          reserved_fen         INTEGER NOT NULL DEFAULT 0,
          settled_cost_fen     INTEGER NOT NULL DEFAULT 0,
          refunded_fen         INTEGER NOT NULL DEFAULT 0,
          status               TEXT NOT NULL DEFAULT 'reserved',
          token_usage_event_id TEXT,
          billing_event_id     TEXT,
          created_at           TEXT NOT NULL,
          updated_at           TEXT NOT NULL,
          expires_at           TEXT,
          balance_after_fen    INTEGER,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_reservations_user_call
          ON billing_reservations(user_id, compute_call_id);
        CREATE INDEX IF NOT EXISTS idx_billing_reservations_status
          ON billing_reservations(status, expires_at);
        CREATE INDEX IF NOT EXISTS idx_billing_reservations_user_time
          ON billing_reservations(user_id, created_at DESC);

        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_default_reservation_fen', '1', datetime('now'));
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_cli_dev_reservation_fen', '100', datetime('now'));
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_cli_chat_reservation_fen', '10', datetime('now'));
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_node_llm_min_reservation_fen', '1', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v34(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_image_min_reservation_fen', '1', datetime('now'));
        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_realtime_voice_min_reservation_fen', '1', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v35(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_workspace_health_snapshots (
          id                         TEXT PRIMARY KEY,
          project_id                 TEXT NOT NULL UNIQUE,
          node_id                    TEXT,
          workspace_path             TEXT,
          can_run_on_pc              INTEGER NOT NULL DEFAULT 0,
          verified_can_run_on_pc     INTEGER,
          health_label               TEXT NOT NULL,
          health_tone                TEXT NOT NULL,
          recommended_action         TEXT NOT NULL,
          warning_count              INTEGER NOT NULL DEFAULT 0,
          warnings_json              TEXT NOT NULL DEFAULT '[]',
          live_inspect_json          TEXT,
          inspect_error              TEXT,
          disk_free_bytes            INTEGER,
          path_exists                INTEGER,
          is_dir                     INTEGER,
          is_git_worktree            INTEGER,
          cli_available              INTEGER,
          captured_at                TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_workspace_health_node_latest
          ON project_workspace_health_snapshots(node_id, captured_at DESC)
          WHERE node_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

fn migration_v36(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_meter_events (
          id                    TEXT PRIMARY KEY,
          user_id               TEXT NOT NULL,
          compute_call_id       TEXT,
          feature               TEXT NOT NULL,
          usage_mode            TEXT NOT NULL,
          model                 TEXT,
          source                TEXT NOT NULL,
          input_unit_kind       TEXT NOT NULL,
          output_unit_kind      TEXT NOT NULL,
          input_units           INTEGER NOT NULL DEFAULT 0,
          output_units          INTEGER NOT NULL DEFAULT 0,
          metered_input_tokens  INTEGER NOT NULL DEFAULT 0,
          metered_output_tokens INTEGER NOT NULL DEFAULT 0,
          metered_total_tokens  INTEGER NOT NULL DEFAULT 0,
          token_usage_event_id  TEXT,
          billing_event_id      TEXT,
          cost_rmb_fen          INTEGER NOT NULL DEFAULT 0,
          accounting_status     TEXT NOT NULL DEFAULT 'unknown',
          created_at            TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_compute_meter_events_time
          ON compute_meter_events(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_compute_meter_events_user_time
          ON compute_meter_events(user_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_compute_meter_events_call
          ON compute_meter_events(user_id, compute_call_id);
        CREATE INDEX IF NOT EXISTS idx_compute_meter_events_feature_mode
          ON compute_meter_events(feature, usage_mode, created_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v37(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS billing_price_rules (
          id               TEXT PRIMARY KEY,
          pattern          TEXT NOT NULL UNIQUE,
          input_usd_per_m  REAL NOT NULL,
          cached_usd_per_m REAL NOT NULL,
          output_usd_per_m REAL NOT NULL,
          priority         INTEGER NOT NULL DEFAULT 0,
          enabled          INTEGER NOT NULL DEFAULT 1,
          note             TEXT,
          updated_at       TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_billing_price_rules_enabled_priority
          ON billing_price_rules(enabled, priority DESC);

        INSERT OR IGNORE INTO billing_price_rules
          (id, pattern, input_usd_per_m, cached_usd_per_m, output_usd_per_m, priority, enabled, note, updated_at)
        VALUES
          ('bpr_seed_gpt_4o_mini_dash', 'gpt-4o-mini', 0.15, 0.075, 0.60, 100, 1, '默认 OpenAI mini 定价', datetime('now')),
          ('bpr_seed_gpt4o_mini', 'gpt4o-mini', 0.15, 0.075, 0.60, 100, 1, '默认 OpenAI mini 定价别名', datetime('now')),
          ('bpr_seed_gpt_4o_dash', 'gpt-4o', 2.5, 1.25, 10.0, 90, 1, '默认 OpenAI 4o 定价', datetime('now')),
          ('bpr_seed_gpt4o', 'gpt4o', 2.5, 1.25, 10.0, 90, 1, '默认 OpenAI 4o 定价别名', datetime('now')),
          ('bpr_seed_o3_mini', 'o3-mini', 1.1, 0.55, 4.4, 100, 1, '默认 o3-mini 定价', datetime('now')),
          ('bpr_seed_claude_35_haiku_dash', 'claude-3-5-haiku', 0.25, 0.03, 1.25, 100, 1, '默认 Claude Haiku 定价', datetime('now')),
          ('bpr_seed_claude_35_haiku_dot', 'claude-3.5-haiku', 0.25, 0.03, 1.25, 100, 1, '默认 Claude Haiku 定价别名', datetime('now')),
          ('bpr_seed_claude_3_haiku', 'claude-3-haiku', 0.25, 0.03, 1.25, 90, 1, '默认 Claude 3 Haiku 定价', datetime('now')),
          ('bpr_seed_claude_opus', 'claude-opus', 15.0, 1.5, 75.0, 80, 1, '默认 Claude Opus 定价', datetime('now')),
          ('bpr_seed_claude_sonnet', 'claude-sonnet', 3.0, 0.3, 15.0, 80, 1, '默认 Claude Sonnet 定价', datetime('now')),
          ('bpr_seed_claude_37_dash', 'claude-3-7', 3.0, 0.3, 15.0, 90, 1, '默认 Claude 3.7 定价', datetime('now')),
          ('bpr_seed_claude_37_dot', 'claude-3.7', 3.0, 0.3, 15.0, 90, 1, '默认 Claude 3.7 定价别名', datetime('now')),
          ('bpr_seed_claude_35_sonnet_dash', 'claude-3-5-sonnet', 3.0, 0.3, 15.0, 90, 1, '默认 Claude 3.5 Sonnet 定价', datetime('now')),
          ('bpr_seed_claude_35_sonnet_dot', 'claude-3.5-sonnet', 3.0, 0.3, 15.0, 90, 1, '默认 Claude 3.5 Sonnet 定价别名', datetime('now')),
          ('bpr_seed_claude', 'claude', 3.0, 0.3, 15.0, 10, 1, 'Claude 默认兜底定价', datetime('now')),
          ('bpr_seed_deepseek', 'deepseek', 0.14, 0.014, 0.28, 80, 1, '默认 DeepSeek 定价', datetime('now')),
          ('bpr_seed_metered_image', 'metered-image', 0.0, 0.0, 5.0, 100, 1, '图片算力内部计量单位', datetime('now')),
          ('bpr_seed_metered_realtime', 'metered-realtime', 1.0, 0.0, 2.0, 100, 1, '实时语音内部计量单位', datetime('now')),
          ('bpr_seed_default', '*', 3.0, 0.3, 15.0, -100, 1, '未知模型保守兜底定价', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v38(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS billing_alerts (
          id             TEXT PRIMARY KEY,
          fingerprint    TEXT NOT NULL UNIQUE,
          severity       TEXT NOT NULL,
          status         TEXT NOT NULL DEFAULT 'open',
          title          TEXT NOT NULL,
          detail         TEXT NOT NULL,
          metric_value   INTEGER NOT NULL DEFAULT 0,
          first_seen_at  TEXT NOT NULL,
          updated_at     TEXT NOT NULL,
          resolved_at    TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_billing_alerts_status_updated
          ON billing_alerts(status, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_billing_alerts_severity
          ON billing_alerts(severity, status);

        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('billing_open_reservation_alert_threshold', '100', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v39(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "billing_price_rules",
        "version",
        "version INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column_if_missing(conn, "billing_events", "price_rule_id", "price_rule_id TEXT")?;
    add_column_if_missing(
        conn,
        "billing_events",
        "price_rule_version",
        "price_rule_version INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "price_rule_pattern",
        "price_rule_pattern TEXT",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "input_usd_per_m",
        "input_usd_per_m REAL",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "cached_usd_per_m",
        "cached_usd_per_m REAL",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "output_usd_per_m",
        "output_usd_per_m REAL",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "price_source",
        "price_source TEXT NOT NULL DEFAULT 'legacy'",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_billing_events_price_rule
          ON billing_events(price_rule_id, price_rule_version);
        "#,
    )?;
    Ok(())
}

fn migration_v40(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "node_transactions", "feature", "feature TEXT")?;
    add_column_if_missing(conn, "node_transactions", "usage_mode", "usage_mode TEXT")?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "compute_call_id",
        "compute_call_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "token_usage_event_id",
        "token_usage_event_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "billing_event_id",
        "billing_event_id TEXT",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "billed_cost_rmb_fen",
        "billed_cost_rmb_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "provider_earned_fen",
        "provider_earned_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "provider_revenue_share_x1000",
        "provider_revenue_share_x1000 INTEGER NOT NULL DEFAULT 800",
    )?;
    add_column_if_missing(
        conn,
        "node_transactions",
        "settlement_status",
        "settlement_status TEXT NOT NULL DEFAULT 'legacy_credit'",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_node_transactions_compute_call
          ON node_transactions(compute_call_id)
          WHERE compute_call_id IS NOT NULL;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_node_transactions_token_usage_event_unique
          ON node_transactions(token_usage_event_id)
          WHERE token_usage_event_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_node_transactions_billing_event
          ON node_transactions(billing_event_id)
          WHERE billing_event_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_node_transactions_settlement_status
          ON node_transactions(settlement_status, created_at DESC);

        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('node_provider_revenue_share_x1000', '800', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v41(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_hardware_snapshots (
          node_id       TEXT PRIMARY KEY,
          owner_user_id TEXT NOT NULL,
          device_name   TEXT,
          hardware_json TEXT NOT NULL,
          created_at    TEXT NOT NULL,
          updated_at    TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_node_hardware_owner_updated
          ON node_hardware_snapshots(owner_user_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v42(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_payout_requests (
          id               TEXT PRIMARY KEY,
          provider_user_id TEXT NOT NULL,
          amount_fen       INTEGER NOT NULL,
          amount_credits   REAL NOT NULL,
          payout_method    TEXT NOT NULL,
          payout_account   TEXT NOT NULL,
          contact          TEXT,
          status           TEXT NOT NULL DEFAULT 'pending',
          admin_note       TEXT,
          created_at       TEXT NOT NULL,
          updated_at       TEXT NOT NULL,
          resolved_at      TEXT,
          resolved_by      TEXT,
          FOREIGN KEY (provider_user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_node_payout_provider_time
          ON node_payout_requests(provider_user_id, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_node_payout_status_time
          ON node_payout_requests(status, created_at DESC);

        INSERT OR IGNORE INTO billing_config (key, value, updated_at)
          VALUES ('node_payout_min_fen', '100', datetime('now'));
        "#,
    )?;
    Ok(())
}

fn migration_v43(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "node_balances",
        "available_fen",
        "available_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_balances",
        "frozen_fen",
        "frozen_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_balances",
        "paid_fen",
        "paid_fen INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute_batch(
        r#"
        UPDATE node_balances
           SET available_fen = CAST(ROUND(credits * 100.0) AS INTEGER)
         WHERE available_fen = 0
           AND ABS(credits) > 0.000001;

        UPDATE node_balances
           SET frozen_fen = COALESCE((
                 SELECT SUM(amount_fen)
                   FROM node_payout_requests
                  WHERE provider_user_id = node_balances.user_id
                    AND status = 'pending'
               ), 0);

        UPDATE node_balances
           SET paid_fen = COALESCE((
                 SELECT SUM(amount_fen)
                   FROM node_payout_requests
                  WHERE provider_user_id = node_balances.user_id
                    AND status = 'paid'
               ), 0);

        UPDATE node_balances
           SET credits = available_fen / 100.0;
        "#,
    )?;
    Ok(())
}

fn migration_v44(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_compute_runs (
          id                       TEXT PRIMARY KEY,
          compute_call_id          TEXT NOT NULL UNIQUE,
          consumer_user_id         TEXT NOT NULL,
          provider_user_id         TEXT,
          node_id                  TEXT NOT NULL,
          model_id                 TEXT,
          feature                  TEXT NOT NULL,
          usage_mode               TEXT NOT NULL,
          status                   TEXT NOT NULL DEFAULT 'started',
          started_at               TEXT NOT NULL,
          finished_at              TEXT,
          duration_ms              INTEGER,
          prompt_tokens            INTEGER NOT NULL DEFAULT 0,
          completion_tokens        INTEGER NOT NULL DEFAULT 0,
          billed_cost_rmb_fen      INTEGER NOT NULL DEFAULT 0,
          provider_earned_fen      INTEGER NOT NULL DEFAULT 0,
          settlement_status        TEXT,
          route_reason             TEXT,
          error_message            TEXT,
          created_at               TEXT NOT NULL,
          updated_at               TEXT NOT NULL,
          FOREIGN KEY (consumer_user_id) REFERENCES users(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id)
        );
        CREATE INDEX IF NOT EXISTS idx_node_compute_runs_node_time
          ON node_compute_runs(node_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_node_compute_runs_provider_time
          ON node_compute_runs(provider_user_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_node_compute_runs_status_time
          ON node_compute_runs(status, started_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v45(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "icon_data_url", "icon_data_url TEXT")?;
    Ok(())
}

fn migration_v46(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "project_channel_messages",
        "reply_to_message_id",
        "reply_to_message_id TEXT",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_project_channel_messages_reply_to
         ON project_channel_messages(project_id, channel_id, reply_to_message_id, created_at)",
        [],
    )?;
    Ok(())
}

// ── v47：指定钱一龙为一龙自项目创建者与 owner ────────────────────────────────

fn migration_v47(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        UPDATE users
           SET nickname = '钱一龙',
               updated_at = datetime('now')
         WHERE id = (
             SELECT id
               FROM users
              WHERE status = 'active'
                AND (phone = '15692409892' OR nickname = '钱一龙')
              ORDER BY CASE
                  WHEN phone = '15692409892' THEN 0
                  WHEN nickname = '钱一龙' THEN 1
                  ELSE 2
              END
              LIMIT 1
         );

        INSERT INTO project_members (project_id, user_id, role, created_at)
        SELECT 'elon-self', u.id, 'owner', datetime('now')
          FROM users u
          JOIN projects p ON p.id = 'elon-self' AND p.status != 'deleted'
         WHERE u.id = (
             SELECT id
               FROM users
              WHERE status = 'active'
                AND (phone = '15692409892' OR nickname = '钱一龙')
              ORDER BY CASE
                  WHEN phone = '15692409892' THEN 0
                  WHEN nickname = '钱一龙' THEN 1
                  ELSE 2
              END
              LIMIT 1
         )
        ON CONFLICT(project_id, user_id) DO UPDATE SET role = 'owner';

        UPDATE projects
           SET created_by = (
                   SELECT id
                     FROM users
                    WHERE status = 'active'
                      AND (phone = '15692409892' OR nickname = '钱一龙')
                    ORDER BY CASE
                        WHEN phone = '15692409892' THEN 0
                        WHEN nickname = '钱一龙' THEN 1
                        ELSE 2
                    END
                    LIMIT 1
               ),
               updated_at = datetime('now')
         WHERE id = 'elon-self'
           AND status != 'deleted'
           AND EXISTS (
               SELECT 1
                 FROM users
                WHERE status = 'active'
                  AND (phone = '15692409892' OR nickname = '钱一龙')
           );
        "#,
    )?;
    Ok(())
}

fn migration_v48(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "storage_node_id", "storage_node_id TEXT")?;
    add_column_if_missing(conn, "projects", "storage_repo_path", "storage_repo_path TEXT")?;
    add_column_if_missing(conn, "projects", "storage_repo_url", "storage_repo_url TEXT")?;
    add_column_if_missing(
        conn,
        "projects",
        "storage_status",
        "storage_status TEXT NOT NULL DEFAULT 'none'",
    )?;
    conn.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_projects_storage_node_id
          ON projects(storage_node_id)
          WHERE storage_node_id IS NOT NULL;
        "#,
    )?;
    Ok(())
}

fn migration_v49(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "projects",
        "storage_worktree_path",
        "storage_worktree_path TEXT",
    )?;
    Ok(())
}

fn migration_v50(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "display_name", "display_name TEXT")?;
    Ok(())
}

fn migration_v51(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        UPDATE projects
           SET is_public = 1,
               join_mode = 'approval',
               updated_at = datetime('now')
         WHERE id = 'elon-self'
           AND status != 'deleted';
        "#,
    )?;
    Ok(())
}

fn migration_v52(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_runtime_permissions (
          project_id TEXT PRIMARY KEY,
          mode       TEXT NOT NULL DEFAULT 'project_write'
                     CHECK (mode IN ('project_write', 'full_access')),
          updated_by TEXT,
          updated_at TEXT,
          expires_at TEXT,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_runtime_permission_audit (
          id         TEXT PRIMARY KEY,
          project_id TEXT NOT NULL,
          user_id    TEXT NOT NULL,
          old_mode   TEXT,
          new_mode   TEXT NOT NULL,
          created_at TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_runtime_permission_audit_project_time
          ON project_runtime_permission_audit(project_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

fn migration_v53(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS friend_group_ai_documents (
          group_id    TEXT NOT NULL,
          path        TEXT NOT NULL,
          title       TEXT NOT NULL,
          content     TEXT NOT NULL,
          position    INTEGER NOT NULL DEFAULT 0,
          updated_by  TEXT,
          updated_at  TEXT NOT NULL,
          PRIMARY KEY (group_id, path),
          FOREIGN KEY (group_id) REFERENCES friend_groups(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS friend_group_summary_context_packs (
          id              TEXT PRIMARY KEY,
          group_id        TEXT NOT NULL,
          purpose         TEXT NOT NULL,
          query           TEXT,
          payload_json    TEXT NOT NULL,
          source_start_at TEXT,
          source_end_at   TEXT,
          message_count   INTEGER NOT NULL DEFAULT 0,
          created_by      TEXT NOT NULL,
          created_at      TEXT NOT NULL,
          FOREIGN KEY (group_id) REFERENCES friend_groups(id),
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS friend_group_summary_posts (
          id                   TEXT PRIMARY KEY,
          group_id             TEXT NOT NULL,
          title                TEXT NOT NULL,
          topic                TEXT,
          summary              TEXT NOT NULL,
          status               TEXT NOT NULL DEFAULT 'generating',
          context_pack_id      TEXT NOT NULL,
          source_start_at      TEXT,
          source_end_at        TEXT,
          source_message_count INTEGER NOT NULL DEFAULT 0,
          model_used           TEXT,
          error                TEXT,
          pinned_at            TEXT,
          pinned_by            TEXT,
          created_by           TEXT NOT NULL,
          created_at           TEXT NOT NULL,
          updated_at           TEXT NOT NULL,
          FOREIGN KEY (group_id) REFERENCES friend_groups(id),
          FOREIGN KEY (context_pack_id) REFERENCES friend_group_summary_context_packs(id),
          FOREIGN KEY (pinned_by) REFERENCES users(id),
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS friend_group_summary_post_sources (
          post_id    TEXT NOT NULL,
          message_id TEXT NOT NULL,
          position   INTEGER NOT NULL DEFAULT 0,
          excerpt    TEXT NOT NULL,
          PRIMARY KEY (post_id, message_id),
          FOREIGN KEY (post_id) REFERENCES friend_group_summary_posts(id),
          FOREIGN KEY (message_id) REFERENCES friend_group_messages(id)
        );

        CREATE INDEX IF NOT EXISTS idx_group_ai_documents_group_position
          ON friend_group_ai_documents(group_id, position);

        CREATE INDEX IF NOT EXISTS idx_group_summary_context_group_time
          ON friend_group_summary_context_packs(group_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_group_summary_posts_group_pinned
          ON friend_group_summary_posts(group_id, pinned_at DESC, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_group_summary_posts_group_updated
          ON friend_group_summary_posts(group_id, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_group_summary_sources_message
          ON friend_group_summary_post_sources(message_id);
        "#,
    )?;
    Ok(())
}

fn migration_v54(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS external_app_accounts (
          app_id           TEXT NOT NULL,
          external_user_id TEXT NOT NULL,
          account          TEXT NOT NULL,
          display_name     TEXT,
          avatar_url       TEXT,
          main_user_id     TEXT,
          status           TEXT NOT NULL DEFAULT 'active'
                           CHECK (status IN ('active', 'disabled')),
          created_at       TEXT NOT NULL,
          updated_at       TEXT NOT NULL,
          last_seen_at     TEXT,
          PRIMARY KEY (app_id, external_user_id),
          UNIQUE(app_id, account),
          FOREIGN KEY (main_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_external_app_accounts_account
          ON external_app_accounts(account, status);

        CREATE INDEX IF NOT EXISTS idx_external_app_accounts_main_user
          ON external_app_accounts(main_user_id, app_id);

        CREATE TABLE IF NOT EXISTS external_app_groups (
          app_id            TEXT NOT NULL,
          external_group_id TEXT NOT NULL,
          group_id          TEXT NOT NULL,
          name              TEXT NOT NULL,
          position          INTEGER NOT NULL DEFAULT 0,
          auto_join         INTEGER NOT NULL DEFAULT 0,
          metadata_json     TEXT,
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          PRIMARY KEY (app_id, external_group_id),
          UNIQUE(app_id, group_id),
          FOREIGN KEY (group_id) REFERENCES friend_groups(id)
        );

        CREATE INDEX IF NOT EXISTS idx_external_app_groups_group
          ON external_app_groups(group_id);

        CREATE TABLE IF NOT EXISTS external_app_auth_codes (
          id           TEXT PRIMARY KEY,
          app_id       TEXT NOT NULL,
          code_hash    TEXT NOT NULL UNIQUE,
          user_id      TEXT NOT NULL,
          scopes_json  TEXT NOT NULL,
          redirect_uri TEXT,
          expires_at   TEXT NOT NULL,
          consumed_at  TEXT,
          created_at   TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_external_app_auth_codes_app_user
          ON external_app_auth_codes(app_id, user_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_external_app_auth_codes_expiry
          ON external_app_auth_codes(expires_at, consumed_at);
        "#,
    )?;
    Ok(())
}

fn migration_v55(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_identities (
          id             TEXT PRIMARY KEY,
          project_id     TEXT NOT NULL,
          owner_user_id  TEXT NOT NULL,
          scope_key      TEXT NOT NULL,
          node_id        TEXT,
          identity_type  TEXT NOT NULL,
          identity_value TEXT NOT NULL,
          confidence     INTEGER NOT NULL DEFAULT 100,
          source         TEXT NOT NULL DEFAULT 'register_external_project',
          created_at     TEXT NOT NULL,
          updated_at     TEXT NOT NULL,
          UNIQUE(owner_user_id, scope_key, identity_type, identity_value),
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
          FOREIGN KEY (owner_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_identities_project
          ON project_identities(project_id);

        CREATE INDEX IF NOT EXISTS idx_project_identities_owner_updated
          ON project_identities(owner_user_id, updated_at DESC);

        INSERT OR IGNORE INTO project_identities (
          id, project_id, owner_user_id, scope_key, node_id, identity_type,
          identity_value, confidence, source, created_at, updated_at
        )
        SELECT
          'pident_m55_ws_' || p.id,
          p.id,
          p.created_by,
          CASE
            WHEN p.node_id IS NULL OR TRIM(p.node_id) = ''
              THEN 'node:unknown'
            ELSE 'node:' || LOWER(TRIM(p.node_id))
          END,
          NULLIF(TRIM(p.node_id), ''),
          'workspace_path',
          LOWER(RTRIM(REPLACE(TRIM(p.workspace_path), '\', '/'), '/')),
          100,
          'migration_v55',
          COALESCE(p.created_at, datetime('now')),
          datetime('now')
        FROM projects p
        WHERE p.status != 'deleted'
          AND p.created_by IS NOT NULL
          AND p.source_type IN ('local_path', 'pc_managed')
          AND p.workspace_path IS NOT NULL
          AND TRIM(p.workspace_path) != ''
        ORDER BY p.updated_at DESC;
        "#,
    )?;
    Ok(())
}

fn migration_v56(conn: &Connection) -> Result<()> {
    crate::store::default_joint_projects::ensure_default_joint_project_memberships_for_all_users_conn(
        conn,
    )?;
    Ok(())
}

fn migration_v57(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO billing_config (key, value, updated_at)
         VALUES ('external_app_fb2_trial_credit_fen', '100', datetime('now'))",
        [],
    )?;
    Ok(())
}

fn migration_v58(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "landing_json", "landing_json TEXT")?;
    Ok(())
}

fn migration_v59(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_landing_upload_tokens (
          id           TEXT PRIMARY KEY,
          project_id   TEXT NOT NULL UNIQUE,
          token_hash   TEXT NOT NULL UNIQUE,
          created_by   TEXT,
          created_at   TEXT NOT NULL,
          last_used_at TEXT,
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_landing_upload_tokens_project
          ON project_landing_upload_tokens(project_id);
        "#,
    )?;
    Ok(())
}

fn migration_v60(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS external_app_tool_executions (
          execution_id           TEXT PRIMARY KEY,
          app_id                 TEXT NOT NULL,
          main_group_id          TEXT,
          external_group_id      TEXT NOT NULL,
          main_user_id           TEXT,
          external_user_id       TEXT,
          context_audit_id       TEXT,
          topic_hint             TEXT,
          status                 TEXT NOT NULL,
          planned_count          INTEGER NOT NULL DEFAULT 0,
          result_count           INTEGER NOT NULL DEFAULT 0,
          ready_count            INTEGER NOT NULL DEFAULT 0,
          grounded_result_count  INTEGER NOT NULL DEFAULT 0,
          weak_result_count      INTEGER NOT NULL DEFAULT 0,
          unsafe_result_count    INTEGER NOT NULL DEFAULT 0,
          source_id_count        INTEGER NOT NULL DEFAULT 0,
          duration_ms            INTEGER NOT NULL DEFAULT 0,
          plan_json              TEXT NOT NULL,
          results_json           TEXT NOT NULL,
          audit_json             TEXT NOT NULL,
          execution_json         TEXT NOT NULL,
          created_at             TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_external_app_tool_exec_app_group_time
          ON external_app_tool_executions(app_id, external_group_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_external_app_tool_exec_user_time
          ON external_app_tool_executions(app_id, main_user_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_external_app_tool_exec_grounding
          ON external_app_tool_executions(app_id, status, grounded_result_count, weak_result_count, unsafe_result_count);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, OptionalExtension};

    #[test]
    fn migration_v47_promotes_qian_yilong_only_for_elon_self() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_old_owner", "18800000000", "旧 owner"],
        )
        .expect("legacy owner should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_qian", "15692409892", "旧昵称"],
        )
        .expect("qian user should insert");

        insert_project(&conn, "elon-self", "一龙项目", "usr_old_owner");
        insert_project(&conn, "prj_joint", "普通联合项目", "usr_old_owner");
        insert_member(&conn, "elon-self", "usr_old_owner", "owner");
        insert_member(&conn, "elon-self", "usr_qian", "admin");
        insert_member(&conn, "prj_joint", "usr_old_owner", "owner");

        migration_v47(&conn).expect("migration should apply");

        let elon_created_by: String = conn
            .query_row(
                "SELECT created_by FROM projects WHERE id = 'elon-self'",
                [],
                |row| row.get(0),
            )
            .expect("elon self project should load");
        let elon_role: String = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = 'elon-self' AND user_id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .expect("qian membership should load");
        let nickname: String = conn
            .query_row(
                "SELECT nickname FROM users WHERE id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .expect("qian user should load");
        let joint_created_by: String = conn
            .query_row(
                "SELECT created_by FROM projects WHERE id = 'prj_joint'",
                [],
                |row| row.get(0),
            )
            .expect("joint project should load");
        let qian_joint_role: Option<String> = conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = 'prj_joint' AND user_id = 'usr_qian'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("joint membership query should run");

        assert_eq!(elon_created_by, "usr_qian");
        assert_eq!(elon_role, "owner");
        assert_eq!(nickname, "钱一龙");
        assert_eq!(joint_created_by, "usr_old_owner");
        assert!(qian_joint_role.is_none());
    }

    #[test]
    fn migration_v51_publishes_elon_self_with_approval() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v2(&conn).expect("visibility columns should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_owner", "18800000000", "owner"],
        )
        .expect("owner should insert");
        insert_project(&conn, "elon-self", "一龙项目", "usr_owner");
        insert_project(&conn, "prj_joint", "普通联合项目", "usr_owner");
        conn.execute(
            "UPDATE projects SET is_public = 0, join_mode = 'readonly' WHERE id = 'elon-self'",
            [],
        )
        .expect("elon self should update");
        conn.execute(
            "UPDATE projects SET is_public = 1, join_mode = 'open' WHERE id = 'prj_joint'",
            [],
        )
        .expect("joint project should update");

        migration_v51(&conn).expect("migration should apply");

        let elon_visibility: (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects WHERE id = 'elon-self'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("elon self project should load");
        let joint_visibility: (i64, String) = conn
            .query_row(
                "SELECT is_public, join_mode FROM projects WHERE id = 'prj_joint'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("joint project should load");

        assert_eq!(elon_visibility, (1, "approval".to_string()));
        assert_eq!(joint_visibility, (1, "open".to_string()));
    }

    #[test]
    fn migration_v55_backfills_workspace_project_identities() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        migration_v1(&conn).expect("base schema should apply");
        migration_v20(&conn).expect("node_id column should apply");

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, ?2, NULL, 'hash', ?3, 'user', 'active', 'now', 'now')",
            params!["usr_owner", "18800000000", "owner"],
        )
        .expect("owner should insert");
        insert_project(&conn, "prj_jian", "江西吉安商会", "usr_owner");
        conn.execute(
            "UPDATE projects
             SET workspace_path = ?1,
                 node_id = ?2
             WHERE id = 'prj_jian'",
            params![r"D:\rust\active-projects\江西吉安商会\", "node-a"],
        )
        .expect("project workspace should update");

        migration_v55(&conn).expect("identity migration should apply");

        let identity: (String, String, String) = conn
            .query_row(
                "SELECT scope_key, identity_type, identity_value
                 FROM project_identities
                 WHERE project_id = 'prj_jian'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("identity should be backfilled");
        assert_eq!(identity.0, "node:node-a");
        assert_eq!(identity.1, "workspace_path");
        assert_eq!(identity.2, "d:/rust/active-projects/江西吉安商会");
    }

    fn insert_project(conn: &Connection, id: &str, name: &str, created_by: &str) {
        conn.execute(
            "INSERT INTO projects (id, name, description, workspace_key, template, source_type, status, created_by, created_at, updated_at)
             VALUES (?1, ?2, '测试项目', ?1, 'local', 'local_path', 'active', ?3, 'now', 'now')",
            params![id, name, created_by],
        )
        .expect("project should insert");
    }

    fn insert_member(conn: &Connection, project_id: &str, user_id: &str, role: &str) {
        conn.execute(
            "INSERT INTO project_members (project_id, user_id, role, created_at)
             VALUES (?1, ?2, ?3, 'now')",
            params![project_id, user_id, role],
        )
        .expect("membership should insert");
    }
}
