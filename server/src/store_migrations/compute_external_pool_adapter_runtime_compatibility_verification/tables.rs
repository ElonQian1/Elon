use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_runtime_compatibility_verification::RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES;

pub(super) fn create(conn: &Connection) -> Result<()> {
    let maximum = RUNTIME_COMPATIBILITY_VERIFICATION_MAX_RECEIPT_JSON_BYTES.to_string();
    for source in [
        include_str!("tables/challenges.sql"),
        include_str!("tables/run_observations.sql"),
        include_str!("tables/verifications.sql"),
        include_str!("tables/revocations.sql"),
    ] {
        conn.execute_batch(&source.replace("__MAX_RECEIPT_JSON_BYTES_SQL__", &maximum))?;
    }
    conn.execute_batch(include_str!("tables/indexes.sql"))?;
    Ok(())
}
