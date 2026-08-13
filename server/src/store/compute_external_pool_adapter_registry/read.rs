use anyhow::Result;
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use super::{audit::*, types::*};

pub(super) fn release_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRegistryRelease>> {
    release_on(conn, "registry_release_id=?1", params![id])
}

pub(in crate::store) fn historical_external_pool_adapter_registry_release_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterRegistryReleaseAuthority>> {
    let Some(stored) = release_by_id_on(conn, id)? else {
        return Ok(None);
    };
    if stored.receipt.registry_release_digest != expected_digest {
        anyhow::bail!("Adapter registry release history is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterRegistryReleaseAuthority::new(stored.receipt),
    ))
}

pub(super) fn release_by_adapter_version_on(
    conn: &Connection,
    adapter_id: &str,
    version: &str,
) -> Result<Option<StoredRegistryRelease>> {
    release_on(
        conn,
        "adapter_id=?1 AND release_version=?2",
        params![adapter_id, version],
    )
}

fn release_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRegistryRelease>> {
    conn.query_row(&format!("SELECT receipt_json FROM compute_external_pool_adapter_registry_releases WHERE {filter}"), values, |row| {
        let receipt_json: String=row.get(0)?;
        let receipt=serde_json::from_str(&receipt_json).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0,Type::Text,Box::new(e)))?;
        Ok(StoredRegistryRelease{receipt,receipt_json})
    }).optional()?.map(|stored| audit_release(conn,stored)).transpose()
}

pub(super) fn binding_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredRegistryProviderBinding>> {
    binding_on(conn, "provider_binding_id=?1", params![id])
}

pub(in crate::store) fn historical_external_pool_adapter_registry_provider_binding_authority_on(
    conn: &Connection,
    id: &str,
    expected_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterRegistryProviderBindingAuthority>> {
    let Some(stored) = binding_by_id_on(conn, id)? else {
        return Ok(None);
    };
    if stored.receipt.provider_binding_digest != expected_digest {
        anyhow::bail!("Adapter registry Provider binding history is not exact");
    }
    Ok(Some(
        HistoricalExternalPoolAdapterRegistryProviderBindingAuthority::new(stored.receipt),
    ))
}

pub(super) fn binding_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRegistryProviderBinding>> {
    binding_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn binding_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredRegistryProviderBinding>> {
    conn.query_row(&format!("SELECT receipt_json FROM compute_external_pool_adapter_registry_provider_bindings WHERE {filter}"), values, |row| {
        let receipt_json:String=row.get(0)?;
        let receipt=serde_json::from_str(&receipt_json).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0,Type::Text,Box::new(e)))?;
        Ok(StoredRegistryProviderBinding{receipt,receipt_json})
    }).optional()?.map(|stored| audit_binding(conn,stored)).transpose()
}
