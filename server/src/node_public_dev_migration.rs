use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v89(conn: &Connection) -> Result<()> {
    add_column_if_missing(
        conn,
        "node_credentials",
        "public_dev_enabled",
        "public_dev_enabled INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "public_dev_allowed_clis_json",
        "public_dev_allowed_clis_json TEXT NOT NULL DEFAULT '[\"codex\",\"copilot\",\"claude\",\"gemini\"]'",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "public_dev_permission_level",
        "public_dev_permission_level TEXT NOT NULL DEFAULT 'project_write'",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_at",
        "last_handshake_at TEXT",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_agent_version",
        "last_handshake_agent_version TEXT",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_allowed_clis_json",
        "last_handshake_allowed_clis_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_route_a_ready",
        "last_handshake_route_a_ready INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_api_runtime_ready",
        "last_handshake_api_runtime_ready INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_server_runtime_ready",
        "last_handshake_server_runtime_ready INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        conn,
        "node_credentials",
        "last_handshake_ai_cli_ready",
        "last_handshake_ai_cli_ready INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_node_credentials_public_dev
           ON node_credentials(public_dev_enabled, owner_user_id, created_at DESC)",
        [],
    )?;
    Ok(())
}
