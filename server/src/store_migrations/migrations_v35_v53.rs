use anyhow::Result;
use rusqlite::Connection;

use super::add_column_if_missing;

pub(crate) fn migration_v35(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v36(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v37(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v38(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v39(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "billing_price_rules",
        "version",
        "version INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column_if_missing(
        conn,
        "billing_events",
        "price_rule_id",
        "price_rule_id TEXT",
    )?;
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

pub(crate) fn migration_v40(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v41(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v42(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v43(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v44(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v45(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "icon_data_url", "icon_data_url TEXT")?;
    Ok(())
}

pub(crate) fn migration_v46(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v47(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v48(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "storage_node_id", "storage_node_id TEXT")?;
    add_column_if_missing(
        conn,
        "projects",
        "storage_repo_path",
        "storage_repo_path TEXT",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "storage_repo_url",
        "storage_repo_url TEXT",
    )?;
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

pub(crate) fn migration_v49(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "projects",
        "storage_worktree_path",
        "storage_worktree_path TEXT",
    )?;
    Ok(())
}

pub(crate) fn migration_v50(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "display_name", "display_name TEXT")?;
    Ok(())
}

pub(crate) fn migration_v51(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v52(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v53(conn: &Connection) -> Result<()> {
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
