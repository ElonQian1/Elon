use anyhow::Result;
use rusqlite::Connection;

use super::guards::policy_projection::catalog_json_and_digest;

pub(super) fn install(conn: &Connection) -> Result<()> {
    let (policy_json, policy_digest, policy_id, policy_revision) = catalog_json_and_digest()?;
    let sql = include_str!("view.sql")
        .replace("__POLICY_ID_SQL__", &literal(&policy_id))
        .replace("__POLICY_REVISION__", &policy_revision.to_string())
        .replace("__POLICY_DIGEST_SQL__", &literal(&policy_digest))
        .replace("__POLICY_JSON_SQL__", &literal(&policy_json));
    conn.execute_batch(&sql)?;
    Ok(())
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
