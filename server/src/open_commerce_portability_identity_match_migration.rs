use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v144(conn: &Connection) -> Result<()> {
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_portability_relationship_mappings",
        "identity_match_status",
        "identity_match_status TEXT NOT NULL DEFAULT 'not_verified'",
    )?;
    crate::store_migrations::add_column_if_missing(
        conn,
        "open_commerce_portability_relationship_mappings",
        "identity_match_key_id",
        "identity_match_key_id TEXT",
    )?;
    Ok(())
}
