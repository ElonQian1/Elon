//! Homepage-domain control proof for developer-App manifest revisions.

use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v152(conn: &Connection) -> Result<()> {
    for (column, definition) in [
        (
            "domain_verification_status",
            "domain_verification_status TEXT NOT NULL DEFAULT 'pending' CHECK(domain_verification_status IN ('pending', 'failed', 'verified'))",
        ),
        ("domain_verification_host", "domain_verification_host TEXT"),
        (
            "domain_verification_revision",
            "domain_verification_revision INTEGER",
        ),
        (
            "domain_verification_challenge_hash",
            "domain_verification_challenge_hash TEXT",
        ),
        (
            "domain_verification_expires_at",
            "domain_verification_expires_at TEXT",
        ),
        (
            "domain_verification_attempted_at",
            "domain_verification_attempted_at TEXT",
        ),
        ("domain_verified_at", "domain_verified_at TEXT"),
        (
            "domain_verification_error_code",
            "domain_verification_error_code TEXT",
        ),
    ] {
        crate::store_migrations::add_column_if_missing(
            conn,
            "open_commerce_developer_apps",
            column,
            definition,
        )?;
    }
    Ok(())
}
