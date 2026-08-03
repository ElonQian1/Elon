//! Reviewable and revocable public-network admission state for developer Apps.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v153(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS open_commerce_developer_app_admissions (
           id                       TEXT PRIMARY KEY,
           app_record_id            TEXT NOT NULL UNIQUE,
           project_id               TEXT NOT NULL,
           manifest_revision        INTEGER NOT NULL,
           organization_name        TEXT NOT NULL,
           jurisdiction             TEXT NOT NULL,
           registration_id          TEXT NOT NULL,
           attested_at              TEXT NOT NULL,
           status                   TEXT NOT NULL CHECK(status IN (
                                        'submitted', 'changes_requested',
                                        'approved', 'suspended'
                                    )),
           requested_at             TEXT NOT NULL,
           reviewed_at              TEXT,
           reviewed_by_user_id      TEXT,
           review_note              TEXT,
           risk_tier                TEXT CHECK(risk_tier IS NULL OR risk_tier IN (
                                        'low', 'standard', 'enhanced'
                                    )),
           suspended_at             TEXT,
           created_at               TEXT NOT NULL,
           updated_at               TEXT NOT NULL,
           FOREIGN KEY(app_record_id) REFERENCES open_commerce_developer_apps(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_open_commerce_developer_app_admissions_review
           ON open_commerce_developer_app_admissions(status, requested_at ASC);
         CREATE INDEX IF NOT EXISTS idx_open_commerce_developer_app_admissions_project
           ON open_commerce_developer_app_admissions(project_id, updated_at DESC);",
    )?;
    Ok(())
}
