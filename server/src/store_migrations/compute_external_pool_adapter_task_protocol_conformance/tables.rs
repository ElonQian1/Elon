use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::{
    external_pool_adapter_runtime_compatibility_verification::server_runtime_compatibility_v2_profile_catalog,
    external_pool_adapter_task_protocol_conformance::{
        server_task_protocol_conformance_fixture_catalog,
        server_task_protocol_conformance_profile_catalog,
        TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES,
    },
};

pub(super) fn create(conn: &Connection) -> Result<()> {
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    let compatibility = server_runtime_compatibility_v2_profile_catalog()?;
    let replacements = [
        (
            "__MAX_RECEIPT_JSON_BYTES_SQL__",
            TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES.to_string(),
        ),
        (
            "__TASK_PROTOCOL_PROFILE_DIGEST_SQL__",
            literal(&profile.profile_digest),
        ),
        (
            "__FIXTURE_CATALOG_DIGEST_SQL__",
            literal(&fixture.catalog_digest),
        ),
        (
            "__SUPERVISOR_SESSION_POLICY_DIGEST_SQL__",
            literal(
                &compatibility
                    .profile
                    .supervisor_session_policy
                    .policy_digest,
            ),
        ),
    ];
    for source in [
        include_str!("tables/run_receipts.sql"),
        include_str!("tables/revocations.sql"),
    ] {
        let sql = replacements
            .iter()
            .fold(source.to_owned(), |sql, (key, value)| {
                sql.replace(key, value)
            });
        conn.execute_batch(&sql)?;
    }
    conn.execute_batch(include_str!("tables/indexes.sql"))?;
    Ok(())
}

pub(super) fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
