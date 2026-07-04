use anyhow::Result;
use rusqlite::Connection;

use crate::store_migrations::add_column_if_missing;

pub(crate) fn migration_v86(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "node_credentials", "install_id", "install_id TEXT")?;
    conn.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_node_credentials_owner_install
          ON node_credentials(owner_user_id, install_id)
          WHERE install_id IS NOT NULL AND trim(install_id) != '';
        "#,
    )?;
    Ok(())
}
