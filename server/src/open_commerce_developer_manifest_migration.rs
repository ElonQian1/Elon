//! Reviewable developer-App manifest metadata and state.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v151(conn: &Connection) -> Result<()> {
    for (column, definition) in [
        ("homepage_url", "homepage_url TEXT"),
        ("privacy_policy_url", "privacy_policy_url TEXT"),
        ("terms_url", "terms_url TEXT"),
        ("support_email", "support_email TEXT"),
        (
            "requested_scopes_json",
            "requested_scopes_json TEXT NOT NULL DEFAULT '[]'",
        ),
        (
            "manifest_status",
            "manifest_status TEXT NOT NULL DEFAULT 'draft' CHECK(manifest_status IN ('draft', 'submitted', 'changes_requested', 'approved'))",
        ),
        (
            "manifest_revision",
            "manifest_revision INTEGER NOT NULL DEFAULT 0",
        ),
        ("submitted_at", "submitted_at TEXT"),
        ("reviewed_at", "reviewed_at TEXT"),
        ("reviewed_by_user_id", "reviewed_by_user_id TEXT"),
        ("review_note", "review_note TEXT"),
    ] {
        crate::store_migrations::add_column_if_missing(
            conn,
            "open_commerce_developer_apps",
            column,
            definition,
        )?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_open_commerce_developer_apps_manifest_review
           ON open_commerce_developer_apps(manifest_status, submitted_at DESC);",
    )?;
    Ok(())
}
