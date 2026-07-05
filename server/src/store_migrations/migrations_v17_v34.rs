use anyhow::Result;
use rusqlite::Connection;

use super::add_column_if_missing;

// ── v17：人民币预存计费系统 ────────────────────────────────────────────────────

pub(crate) fn migration_v17(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v18(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v19(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v20(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v21(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v22(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "conversations",
        "locked_agent_name",
        "locked_agent_name TEXT",
    )?;
    Ok(())
}

// ── v23：节点凭证记录 PC 设备名 ───────────────────────────────────────────────

pub(crate) fn migration_v23(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "node_credentials", "device_name", "device_name TEXT")?;
    Ok(())
}

// ── v24：用户记忆作用域 ─────────────────────────────────────────────────────

pub(crate) fn migration_v24(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v25(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v26(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v27(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v28(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v29(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v30(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v31(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v32(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v33(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v34(conn: &Connection) -> Result<()> {
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
