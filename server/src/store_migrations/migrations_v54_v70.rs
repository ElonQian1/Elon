use anyhow::Result;
use rusqlite::Connection;

use super::add_column_if_missing;

pub(crate) fn migration_v54(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v55(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v56(conn: &Connection) -> Result<()> {
    crate::store::default_joint_projects::ensure_default_joint_project_memberships_for_all_users_conn(
        conn,
    )?;
    Ok(())
}

pub(crate) fn migration_v57(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO billing_config (key, value, updated_at)
         VALUES ('external_app_fb2_trial_credit_fen', '100', datetime('now'))",
        [],
    )?;
    Ok(())
}

pub(crate) fn migration_v58(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "projects", "landing_json", "landing_json TEXT")?;
    Ok(())
}

pub(crate) fn migration_v59(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v60(conn: &Connection) -> Result<()> {
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

pub(crate) fn migration_v61(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO billing_config (key, value, updated_at)
         VALUES ('new_user_trial_credit_fen', '100', datetime('now'))",
        [],
    )?;
    Ok(())
}

pub(crate) fn migration_v62(conn: &Connection) -> Result<()> {
    crate::store::default_joint_projects::remove_legacy_default_joint_project_memberships_conn(
        conn,
    )?;
    Ok(())
}

pub(crate) fn migration_v63(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_dev_profiles (
          project_id          TEXT PRIMARY KEY,
          project_type        TEXT,
          package_manager     TEXT,
          run_command         TEXT,
          test_command        TEXT,
          build_command       TEXT,
          detected_files_json TEXT NOT NULL DEFAULT '[]',
          source              TEXT,
          updated_at          TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v64(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS route_c_runtime_budget_events (
          id                  TEXT PRIMARY KEY,
          user_id             TEXT NOT NULL,
          request_fingerprint TEXT NOT NULL,
          route_day           TEXT NOT NULL,
          created_at          TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_route_c_budget_day_time
          ON route_c_runtime_budget_events(route_day, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_route_c_budget_user_time
          ON route_c_runtime_budget_events(user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v65(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_route_c_budget_day_user
          ON route_c_runtime_budget_events(route_day, user_id, created_at DESC)",
        [],
    )?;
    Ok(())
}

pub(crate) fn migration_v66(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "route_c_runtime_budget_events",
        "outcome",
        "outcome TEXT NOT NULL DEFAULT 'admitted'",
    )?;
    add_column_if_missing(
        conn,
        "route_c_runtime_budget_events",
        "completed_at",
        "completed_at TEXT",
    )?;
    add_column_if_missing(conn, "route_c_runtime_budget_events", "model", "model TEXT")?;
    add_column_if_missing(
        conn,
        "route_c_runtime_budget_events",
        "total_tokens",
        "total_tokens INTEGER",
    )?;
    add_column_if_missing(
        conn,
        "route_c_runtime_budget_events",
        "error_summary",
        "error_summary TEXT",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_route_c_budget_outcome_time
          ON route_c_runtime_budget_events(outcome, created_at DESC)",
        [],
    )?;
    Ok(())
}

pub(crate) fn migration_v67(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO billing_config (key, value, updated_at)
         VALUES ('external_app_bb64a_trial_credit_fen', '100', datetime('now'))",
        [],
    )?;
    Ok(())
}

pub(crate) fn migration_v68(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_runtime_permissions_v68 (
          project_id TEXT PRIMARY KEY,
          mode       TEXT NOT NULL DEFAULT 'project_write'
                     CHECK (mode IN ('project_write', 'full_access', 'danger_full_access')),
          updated_by TEXT,
          updated_at TEXT,
          expires_at TEXT,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        INSERT OR REPLACE INTO project_runtime_permissions_v68
          (project_id, mode, updated_by, updated_at, expires_at)
        SELECT project_id,
               CASE
                 WHEN mode IN ('project_write', 'full_access', 'danger_full_access') THEN mode
                 ELSE 'project_write'
               END,
               updated_by,
               updated_at,
               expires_at
          FROM project_runtime_permissions;

        DROP TABLE project_runtime_permissions;
        ALTER TABLE project_runtime_permissions_v68 RENAME TO project_runtime_permissions;
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v69(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_member_audit (
          id             TEXT PRIMARY KEY,
          project_id     TEXT NOT NULL,
          actor_user_id  TEXT,
          target_user_id TEXT,
          action         TEXT NOT NULL,
          old_role       TEXT,
          new_role       TEXT,
          note           TEXT,
          created_at     TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (actor_user_id) REFERENCES users(id),
          FOREIGN KEY (target_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_member_audit_project_time
          ON project_member_audit(project_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_member_audit_target_time
          ON project_member_audit(target_user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v70(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_member_restrictions (
          project_id   TEXT NOT NULL,
          user_id      TEXT NOT NULL,
          muted_until  TEXT,
          banned_at    TEXT,
          banned_until TEXT,
          note         TEXT,
          updated_by   TEXT,
          updated_at   TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_member_restrictions_project
          ON project_member_restrictions(project_id, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_member_restrictions_user
          ON project_member_restrictions(user_id, updated_at DESC);
        "#,
    )?;
    Ok(())
}
