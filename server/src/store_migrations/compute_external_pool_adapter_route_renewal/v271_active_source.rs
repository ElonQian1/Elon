use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(connection: &Connection) -> Result<()> {
    super::super::compute_external_pool_adapter_route_source_projection::reinstall_exact_source_trigger_for_v278(
        connection,
    )
}
