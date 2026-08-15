use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::{
    server_provider_runtime_readiness_policy_catalog,
    PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES,
};

pub(super) fn create(conn: &Connection) -> Result<()> {
    let policy = server_provider_runtime_readiness_policy_catalog()?;
    let maximum = PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES.to_string();
    let policy_digest = literal(&policy.policy_digest);
    for source in [
        include_str!("tables/receipts.sql"),
        include_str!("tables/revocations.sql"),
    ] {
        let sql = source
            .replace("__MAX_RECEIPT_JSON_BYTES_SQL__", &maximum)
            .replace("__POLICY_DIGEST_SQL__", &policy_digest);
        conn.execute_batch(&sql)?;
    }
    conn.execute_batch(include_str!("tables/indexes.sql"))?;
    Ok(())
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
