//! Revocable one-time production credentials for approved developer Apps.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v154(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_developer_production_credentials (
           id                       TEXT PRIMARY KEY,
           app_record_id            TEXT NOT NULL,
           project_id               TEXT NOT NULL,
           admission_id             TEXT NOT NULL,
           manifest_revision        INTEGER NOT NULL,
           scopes_json              TEXT NOT NULL,
           status                   TEXT NOT NULL CHECK(status IN ('active', 'revoked')),
           token_hash               TEXT NOT NULL UNIQUE,
           token_hint               TEXT NOT NULL,
           issued_by_user_id        TEXT NOT NULL,
           issued_at                TEXT NOT NULL,
           expires_at               TEXT NOT NULL,
           last_used_at             TEXT,
           revoked_at               TEXT,
           revocation_reason        TEXT,
           created_at               TEXT NOT NULL,
           updated_at               TEXT NOT NULL,
           FOREIGN KEY(app_record_id) REFERENCES open_commerce_developer_apps(id) ON DELETE CASCADE,
           FOREIGN KEY(admission_id) REFERENCES open_commerce_developer_app_admissions(id) ON DELETE RESTRICT
         );
         CREATE UNIQUE INDEX IF NOT EXISTS idx_open_commerce_production_credentials_active_app
           ON open_commerce_developer_production_credentials(app_record_id)
           WHERE status='active';
         CREATE INDEX IF NOT EXISTS idx_open_commerce_production_credentials_project
           ON open_commerce_developer_production_credentials(project_id, updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_production_credentials_expiry
           ON open_commerce_developer_production_credentials(status, expires_at);",
    )?;
    Ok(())
}
