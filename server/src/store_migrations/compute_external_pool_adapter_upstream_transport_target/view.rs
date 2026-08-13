use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_upstream_transport_target::server_upstream_transport_target_policy_catalog;

pub(super) fn install(conn: &Connection) -> Result<()> {
    let (policy, policy_digest) = server_upstream_transport_target_policy_catalog()?;
    let policy_json =
        crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
            &policy,
            1024 * 1024,
        )?
        .0;
    let sql = include_str!("view.sql")
        .replace("__POLICY_ID_SQL__", &literal(&policy.policy_id))
        .replace("__POLICY_REVISION__", &policy.policy_revision.to_string())
        .replace("__POLICY_DIGEST_SQL__", &literal(&policy_digest))
        .replace("__POLICY_JSON_SQL__", &literal(&policy_json));
    conn.execute_batch(&sql)?;
    Ok(())
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
