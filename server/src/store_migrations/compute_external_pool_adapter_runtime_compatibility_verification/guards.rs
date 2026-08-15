use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_runtime_compatibility_verification::{
    server_runtime_compatibility_public_fixture_catalog,
    server_runtime_compatibility_runner_policy_catalog,
    server_runtime_compatibility_v2_profile_catalog,
};

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("guards/immutability.sql"))?;
    conn.execute_batch(include_str!("guards/no_replace.sql"))?;
    super::receipt_integrity::install(conn)?;
    conn.execute_batch(include_str!("guards/projection.sql"))?;
    let profile = server_runtime_compatibility_v2_profile_catalog()?;
    let (_, runner_digest) = server_runtime_compatibility_runner_policy_catalog()?;
    let (_, fixture_digest) = server_runtime_compatibility_public_fixture_catalog()?;
    let lineage = include_str!("guards/lineage.sql")
        .replace("__PROFILE_DIGEST_SQL__", &literal(&profile.profile_digest))
        .replace("__RUNNER_POLICY_DIGEST_SQL__", &literal(&runner_digest))
        .replace("__FIXTURE_CATALOG_DIGEST_SQL__", &literal(&fixture_digest));
    conn.execute_batch(&lineage)?;
    Ok(())
}

fn literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
