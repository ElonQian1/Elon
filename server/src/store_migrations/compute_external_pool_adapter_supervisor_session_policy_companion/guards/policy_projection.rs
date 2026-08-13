use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog;

pub(in super::super) fn catalog_json_and_digest() -> Result<(String, String, String, u64)> {
    let (policy, digest) = server_supervisor_session_policy_catalog()?;
    let json = crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
        &policy,
        1024 * 1024,
    )?
    .0;
    Ok((json, digest, policy.policy_id, policy.policy_revision))
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    let (policy_json, policy_digest, _, _) = catalog_json_and_digest()?;
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_policy_json_projection
         BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companions
         WHEN NEW.supervisor_session_policy_digest IS NOT '{}'
           OR NEW.supervisor_session_policy_json IS NOT '{}'
           OR json(NEW.supervisor_session_policy_json) IS NOT json('{}')
         BEGIN SELECT RAISE(ABORT,'V259 server supervisor/session policy projection is not exact'); END;",
        quoted(&policy_digest),
        quoted(&policy_json),
        quoted(&policy_json),
    ))?;
    Ok(())
}

fn quoted(value: &str) -> String {
    value.replace('\'', "''")
}
