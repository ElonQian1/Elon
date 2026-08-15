use anyhow::Result;
use rusqlite::Connection;

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::{
    server_task_protocol_conformance_fixture_catalog,
    server_task_protocol_conformance_profile_catalog,
};

pub(super) fn install(conn: &Connection) -> Result<()> {
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture = server_task_protocol_conformance_fixture_catalog()?;
    let sql = include_str!("view.sql")
        .replace(
            "__TASK_PROTOCOL_PROFILE_DIGEST_SQL__",
            &super::tables::literal(&profile.profile_digest),
        )
        .replace(
            "__FIXTURE_CATALOG_DIGEST_SQL__",
            &super::tables::literal(&fixture.catalog_digest),
        );
    conn.execute_batch(&sql)?;
    Ok(())
}
