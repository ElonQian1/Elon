use anyhow::{bail, Result};
use rusqlite::{params, Connection};

const V254_FENCES: [&str; 18] = [
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];

const RETAINED_ABSOLUTE_DENIES: [&str; 9] = [
    "v254_external_pool_provider_insert_active_fence",
    "v254_external_pool_provider_identity_update_fence",
    "v254_external_pool_provider_kind_update_fence",
    "v254_external_pool_capacity_pool_insert_active_fence",
    "v254_external_pool_capacity_pool_update_active_fence",
    "v254_external_pool_capacity_pool_version_active_fence",
    "v254_external_pool_offer_insert_market_fence",
    "v254_external_pool_offer_update_market_fence",
    "v254_external_pool_offer_version_market_fence",
];

const PENDING_PERMITS: [&str; 9] = [
    "v254_external_pool_provider_activation_fence",
    "v254_external_pool_provider_version_active_fence",
    "v254_external_pool_candidate_projection_adapter_fence",
    "v254_external_pool_candidate_projection_adapter_version_fence",
    "v254_external_pool_candidate_service_actor_fence",
    "v254_external_pool_route_credential_fence",
    "v254_external_pool_route_authorization_fence",
    "v254_external_pool_route_capability_fence",
    "v254_external_pool_route_seal_fence",
];

pub(super) fn before(conn: &Connection) -> Result<i64> {
    require_named_fences(conn)?;
    for name in RETAINED_ABSOLUTE_DENIES {
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![name],
            |row| row.get(0),
        )?;
        if sql.contains("pending_plan_matches") {
            bail!("V277 retained absolute deny unexpectedly consults pending plan: {name}")
        }
    }
    if table_exists(
        conn,
        "compute_external_pool_adapter_atomic_activation_receipts",
    )? {
        conn.query_row(
            "SELECT COUNT(*) FROM compute_external_pool_adapter_atomic_activation_receipts",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
    } else {
        Ok(0)
    }
}

pub(super) fn after(conn: &Connection, receipt_count_before: i64) -> Result<()> {
    require_named_fences(conn)?;
    if table_columns(
        conn,
        "compute_external_pool_adapter_atomic_activation_receipts",
    )? != 79
        || table_columns(
            conn,
            "compute_external_pool_adapter_provider_active_successor_receipts",
        )? != 85
        || table_columns(
            conn,
            "compute_external_pool_adapter_provider_active_successor_revocations",
        )? != 25
    {
        bail!("V277 durable table column contract drifted")
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM compute_external_pool_adapter_atomic_activation_receipts",
        [],
        |row| row.get(0),
    )?;
    if count != receipt_count_before {
        bail!("V277 migration must not seed activation receipts")
    }
    let namespace_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master
          WHERE type IN ('table','view')
            AND name GLOB 'compute_external_pool_adapter_atomic_activation*'",
        [],
        |row| row.get(0),
    )?;
    if namespace_count != 1 {
        bail!("V277 durable namespace must remain exactly one table and zero views")
    }
    for name in PENDING_PERMITS {
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![name],
            |row| row.get(0),
        )?;
        if !sql.contains("pending_plan_matches") {
            bail!("V277 permitted fence lacks exact pending-plan gate: {name}")
        }
    }
    for name in RETAINED_ABSOLUTE_DENIES {
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![name],
            |row| row.get(0),
        )?;
        if sql.contains("pending_plan_matches") {
            bail!("V277 opened a retained absolute deny: {name}")
        }
    }
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        params![table],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn require_named_fences(conn: &Connection) -> Result<()> {
    for name in V254_FENCES {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name=?1",
            params![name],
            |row| row.get(0),
        )?;
        if count != 1 {
            bail!("V277 requires exact V254 fence: {name}")
        }
    }
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<usize> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = statement.query_map([], |_| Ok(()))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?.len())
}
