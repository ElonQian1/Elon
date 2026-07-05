use anyhow::Result;
use rusqlite::Connection;

use super::add_column_if_missing;

pub(crate) fn migration_v71(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_roles (
          id               TEXT PRIMARY KEY,
          project_id       TEXT NOT NULL,
          name             TEXT NOT NULL,
          color            TEXT,
          position         INTEGER NOT NULL DEFAULT 30,
          permissions_json TEXT NOT NULL DEFAULT '[]',
          created_by       TEXT,
          created_at       TEXT NOT NULL,
          updated_at       TEXT NOT NULL,
          UNIQUE(project_id, name),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_roles_project_position
          ON project_roles(project_id, position DESC, created_at);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v72(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_member_roles (
          project_id  TEXT NOT NULL,
          user_id     TEXT NOT NULL,
          role_id     TEXT NOT NULL,
          assigned_by TEXT,
          assigned_at TEXT NOT NULL,
          PRIMARY KEY (project_id, user_id, role_id),
          FOREIGN KEY (project_id, user_id) REFERENCES project_members(project_id, user_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (assigned_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_member_roles_project_role
          ON project_member_roles(project_id, role_id);

        CREATE INDEX IF NOT EXISTS idx_project_member_roles_user
          ON project_member_roles(user_id, project_id);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v73(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_channel_role_permissions (
          project_id  TEXT NOT NULL,
          channel_id  TEXT NOT NULL,
          role_id     TEXT NOT NULL,
          permission  TEXT NOT NULL,
          effect      TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
          updated_by  TEXT,
          updated_at  TEXT NOT NULL,
          PRIMARY KEY (project_id, channel_id, role_id, permission),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (channel_id) REFERENCES project_channels(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_channel_role_permissions_channel
          ON project_channel_role_permissions(project_id, channel_id, role_id);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v74(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_channel_member_permissions (
          project_id  TEXT NOT NULL,
          channel_id  TEXT NOT NULL,
          user_id     TEXT NOT NULL,
          permission  TEXT NOT NULL,
          effect      TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
          updated_by  TEXT,
          updated_at  TEXT NOT NULL,
          PRIMARY KEY (project_id, channel_id, user_id, permission),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (channel_id) REFERENCES project_channels(id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_channel_member_permissions_channel
          ON project_channel_member_permissions(project_id, channel_id, user_id);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v75(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "project_channels", "category_id", "category_id TEXT")?;
    add_column_if_missing(
        conn,
        "project_channels",
        "permission_sync",
        "permission_sync INTEGER NOT NULL DEFAULT 1",
    )?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_channel_categories (
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

        CREATE INDEX IF NOT EXISTS idx_project_channel_categories_project_position
          ON project_channel_categories(project_id, position);

        CREATE TABLE IF NOT EXISTS project_channel_category_role_permissions (
          project_id   TEXT NOT NULL,
          category_id  TEXT NOT NULL,
          role_id      TEXT NOT NULL,
          permission   TEXT NOT NULL,
          effect       TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
          updated_by   TEXT,
          updated_at   TEXT NOT NULL,
          PRIMARY KEY (project_id, category_id, role_id, permission),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (category_id) REFERENCES project_channel_categories(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_channel_category_role_permissions_category
          ON project_channel_category_role_permissions(project_id, category_id, role_id);

        CREATE TABLE IF NOT EXISTS project_channel_category_member_permissions (
          project_id   TEXT NOT NULL,
          category_id  TEXT NOT NULL,
          user_id      TEXT NOT NULL,
          permission   TEXT NOT NULL,
          effect       TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
          updated_by   TEXT,
          updated_at   TEXT NOT NULL,
          PRIMARY KEY (project_id, category_id, user_id, permission),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (category_id) REFERENCES project_channel_categories(id),
          FOREIGN KEY (user_id) REFERENCES users(id),
          FOREIGN KEY (updated_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_channel_category_member_permissions_category
          ON project_channel_category_member_permissions(project_id, category_id, user_id);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v76(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_presence_settings (
          user_id       TEXT PRIMARY KEY,
          status        TEXT NOT NULL DEFAULT 'online'
                        CHECK (status IN ('online', 'idle', 'dnd', 'invisible')),
          custom_status TEXT,
          activity      TEXT,
          updated_at    TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS project_invite_links (
          id          TEXT PRIMARY KEY,
          project_id  TEXT NOT NULL,
          code        TEXT NOT NULL UNIQUE,
          role        TEXT NOT NULL DEFAULT 'member',
          max_uses    INTEGER,
          use_count   INTEGER NOT NULL DEFAULT 0,
          expires_at  TEXT,
          temporary   INTEGER NOT NULL DEFAULT 0,
          revoked_at  TEXT,
          created_by  TEXT NOT NULL,
          created_at  TEXT NOT NULL,
          updated_at  TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (created_by) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_invite_links_project
          ON project_invite_links(project_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_invite_links_code
          ON project_invite_links(code);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v77(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "projects",
        "gallery_images_json",
        "gallery_images_json TEXT",
    )?;
    Ok(())
}

pub(crate) fn migration_v78(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_ai_node_authorizations (
          id                 TEXT PRIMARY KEY,
          project_id         TEXT NOT NULL,
          provider_user_id   TEXT NOT NULL,
          node_id            TEXT NOT NULL,
          allowed_clis_json  TEXT NOT NULL DEFAULT '[]',
          permission_level   TEXT NOT NULL DEFAULT 'project_write'
                             CHECK (permission_level IN ('project_write', 'full_access', 'danger_full_access')),
          enabled            INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
          created_by_user_id TEXT NOT NULL,
          created_at         TEXT NOT NULL,
          updated_at         TEXT NOT NULL,
          UNIQUE(project_id, node_id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id),
          FOREIGN KEY (created_by_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_node_authorizations_project
          ON project_ai_node_authorizations(project_id, enabled, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_node_authorizations_provider
          ON project_ai_node_authorizations(provider_user_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS project_ai_bots (
          id                TEXT PRIMARY KEY,
          project_id        TEXT NOT NULL,
          provider_user_id  TEXT NOT NULL,
          node_id           TEXT NOT NULL,
          display_name      TEXT NOT NULL,
          runtime_route     TEXT NOT NULL,
          cli_name          TEXT NOT NULL,
          capabilities_json TEXT NOT NULL DEFAULT '[]',
          risk_level        TEXT NOT NULL DEFAULT 'project_write',
          enabled           INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          UNIQUE(project_id, node_id, cli_name),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_bots_project
          ON project_ai_bots(project_id, enabled, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_bots_provider_node
          ON project_ai_bots(provider_user_id, node_id);

        CREATE TABLE IF NOT EXISTS project_ai_matters (
          id                       TEXT PRIMARY KEY,
          project_id               TEXT NOT NULL,
          channel_id               TEXT NOT NULL,
          requester_user_id        TEXT NOT NULL,
          decision_user_id         TEXT,
          source_message_id        TEXT,
          title                    TEXT NOT NULL,
          brief                    TEXT NOT NULL,
          collaboration_mode       TEXT NOT NULL DEFAULT 'solo'
                                   CHECK (collaboration_mode IN ('solo', 'critic', 'split')),
          status                   TEXT NOT NULL DEFAULT 'plan_ready'
                                   CHECK (status IN ('plan_ready', 'running', 'review_ready', 'done', 'canceled', 'failed')),
          participant_user_ids_json TEXT NOT NULL DEFAULT '[]',
          node_policy_json         TEXT NOT NULL DEFAULT '{}',
          acceptance_criteria_json TEXT NOT NULL DEFAULT '[]',
          plan_json                TEXT NOT NULL DEFAULT '{}',
          final_summary            TEXT,
          final_decision           TEXT,
          created_at               TEXT NOT NULL,
          updated_at               TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (requester_user_id) REFERENCES users(id),
          FOREIGN KEY (decision_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_matters_project_status
          ON project_ai_matters(project_id, status, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_matters_project_channel
          ON project_ai_matters(project_id, channel_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS project_ai_matter_assignments (
          id                TEXT PRIMARY KEY,
          matter_id         TEXT NOT NULL,
          bot_id            TEXT NOT NULL,
          assignee_user_id  TEXT,
          provider_user_id  TEXT NOT NULL,
          node_id           TEXT NOT NULL,
          role              TEXT NOT NULL,
          runtime_route     TEXT NOT NULL,
          cli_name          TEXT NOT NULL,
          worktree_path     TEXT,
          branch_name       TEXT,
          status            TEXT NOT NULL DEFAULT 'planned',
          result_summary    TEXT,
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          FOREIGN KEY (matter_id) REFERENCES project_ai_matters(id),
          FOREIGN KEY (assignee_user_id) REFERENCES users(id),
          FOREIGN KEY (provider_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_matter_assignments_matter
          ON project_ai_matter_assignments(matter_id, status, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_matter_assignments_provider
          ON project_ai_matter_assignments(provider_user_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS project_ai_reviews (
          id                   TEXT PRIMARY KEY,
          matter_id            TEXT NOT NULL,
          reviewer_bot_id      TEXT,
          reviewer_user_id     TEXT,
          target_assignment_id TEXT,
          severity             TEXT NOT NULL DEFAULT 'info',
          finding_json         TEXT NOT NULL DEFAULT '{}',
          status               TEXT NOT NULL DEFAULT 'open',
          created_at           TEXT NOT NULL,
          updated_at           TEXT NOT NULL,
          FOREIGN KEY (matter_id) REFERENCES project_ai_matters(id),
          FOREIGN KEY (reviewer_user_id) REFERENCES users(id),
          FOREIGN KEY (target_assignment_id) REFERENCES project_ai_matter_assignments(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_reviews_matter
          ON project_ai_reviews(matter_id, status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS project_ai_events (
          id            TEXT PRIMARY KEY,
          matter_id     TEXT NOT NULL,
          project_id    TEXT NOT NULL,
          actor_user_id TEXT,
          event_type    TEXT NOT NULL,
          payload_json  TEXT NOT NULL DEFAULT '{}',
          created_at    TEXT NOT NULL,
          FOREIGN KEY (matter_id) REFERENCES project_ai_matters(id),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (actor_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_events_matter_created
          ON project_ai_events(matter_id, created_at ASC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_events_project_created
          ON project_ai_events(project_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v79(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_ai_assignment_artifacts (
          id                TEXT PRIMARY KEY,
          project_id        TEXT NOT NULL,
          matter_id         TEXT NOT NULL,
          assignment_id     TEXT NOT NULL,
          uploader_user_id  TEXT,
          artifact_kind     TEXT NOT NULL DEFAULT 'execution_report',
          summary           TEXT,
          worktree_path     TEXT,
          branch_name       TEXT,
          files_json        TEXT NOT NULL DEFAULT '[]',
          diff_stat_json    TEXT NOT NULL DEFAULT '[]',
          test_results_json TEXT NOT NULL DEFAULT '[]',
          metadata_json     TEXT NOT NULL DEFAULT '{}',
          created_at        TEXT NOT NULL,
          updated_at        TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (matter_id) REFERENCES project_ai_matters(id),
          FOREIGN KEY (assignment_id) REFERENCES project_ai_matter_assignments(id),
          FOREIGN KEY (uploader_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_assignment_artifacts_assignment
          ON project_ai_assignment_artifacts(assignment_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_assignment_artifacts_matter
          ON project_ai_assignment_artifacts(matter_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS project_ai_merge_requests (
          id                   TEXT PRIMARY KEY,
          project_id           TEXT NOT NULL,
          matter_id            TEXT NOT NULL,
          assignment_id        TEXT NOT NULL,
          requested_by_user_id TEXT,
          worktree_path        TEXT,
          branch_name          TEXT,
          status               TEXT NOT NULL DEFAULT 'open'
                               CHECK (status IN ('open', 'approved', 'merged', 'rejected', 'canceled')),
          merge_strategy       TEXT NOT NULL DEFAULT 'manual',
          review_status        TEXT NOT NULL DEFAULT 'pending',
          risk_level           TEXT NOT NULL DEFAULT 'medium',
          notes                TEXT,
          created_at           TEXT NOT NULL,
          updated_at           TEXT NOT NULL,
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (matter_id) REFERENCES project_ai_matters(id),
          FOREIGN KEY (assignment_id) REFERENCES project_ai_matter_assignments(id),
          FOREIGN KEY (requested_by_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_ai_merge_requests_matter
          ON project_ai_merge_requests(matter_id, status, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_project_ai_merge_requests_assignment
          ON project_ai_merge_requests(assignment_id, status, updated_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v80(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_pc_workspace_bindings (
          id                        TEXT PRIMARY KEY,
          project_id                TEXT NOT NULL,
          owner_user_id             TEXT NOT NULL,
          node_id                   TEXT NOT NULL,
          workspace_path            TEXT NOT NULL,
          normalized_workspace_path TEXT NOT NULL,
          repo_url                  TEXT,
          branch                    TEXT,
          git_head                  TEXT,
          source                    TEXT NOT NULL DEFAULT 'manual',
          created_at                TEXT NOT NULL,
          updated_at                TEXT NOT NULL,
          UNIQUE(project_id, owner_user_id, node_id),
          UNIQUE(owner_user_id, node_id, normalized_workspace_path),
          FOREIGN KEY (project_id) REFERENCES projects(id),
          FOREIGN KEY (owner_user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_project_pc_workspace_bindings_project
          ON project_pc_workspace_bindings(project_id, owner_user_id, updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_project_pc_workspace_bindings_node
          ON project_pc_workspace_bindings(owner_user_id, node_id, updated_at DESC);

        INSERT OR IGNORE INTO project_pc_workspace_bindings (
          id, project_id, owner_user_id, node_id, workspace_path,
          normalized_workspace_path, repo_url, branch, git_head, source, created_at, updated_at
        )
        SELECT
          'ppwb_' || lower(hex(randomblob(16))),
          p.id,
          p.created_by,
          p.node_id,
          p.workspace_path,
          lower(replace(rtrim(rtrim(trim(p.workspace_path), '/'), '\'), '\', '/')),
          p.repo_url,
          p.branch,
          NULL,
          'backfill_projects',
          datetime('now'),
          datetime('now')
        FROM projects p
        WHERE p.node_id IS NOT NULL
          AND trim(p.node_id) != ''
          AND p.workspace_path IS NOT NULL
          AND trim(p.workspace_path) != ''
          AND p.status != 'deleted';
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v81(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS user_codex_credentials (
          user_id             TEXT PRIMARY KEY,
          auth_mode           TEXT NOT NULL,
          account_hint_hash   TEXT,
          source_device       TEXT,
          ciphertext_b64      TEXT NOT NULL,
          nonce_b64           TEXT NOT NULL,
          credential_version  INTEGER NOT NULL DEFAULT 1,
          last_backup_at      TEXT,
          last_lease_at       TEXT,
          created_at          TEXT NOT NULL,
          updated_at          TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS user_codex_credential_events (
          id          TEXT PRIMARY KEY,
          user_id     TEXT NOT NULL,
          event_type  TEXT NOT NULL,
          node_id     TEXT,
          success     INTEGER NOT NULL DEFAULT 1,
          error       TEXT,
          created_at  TEXT NOT NULL,
          FOREIGN KEY (user_id) REFERENCES users(id)
        );

        CREATE INDEX IF NOT EXISTS idx_user_codex_credential_events_user_time
          ON user_codex_credential_events(user_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn migration_v82(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "project_members", "display_name", "display_name TEXT")?;
    add_column_if_missing(conn, "project_members", "admin_note", "admin_note TEXT")?;
    Ok(())
}
