use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::store::{
    compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
    Store,
};

use super::{read::binding_by_idempotency_on, types::ExternalPoolAdapterRegistryAuditTarget};

impl Store {
    pub(crate) fn external_pool_adapter_registry_fresh_target(
        &self,
        installation_receipt_id: &str,
        expected_installation_receipt_digest: &str,
    ) -> Result<Option<ExternalPoolAdapterRegistryAuditTarget>> {
        validate_target(
            installation_receipt_id,
            expected_installation_receipt_digest,
        )?;
        let conn = self.conn()?;
        if binding_by_installation_on(&conn, installation_receipt_id)?.is_some() {
            bail!("Adapter installation already has a registry companion");
        }
        target_on(
            &conn,
            installation_receipt_id,
            expected_installation_receipt_digest,
        )
    }

    pub(crate) fn external_pool_adapter_registry_replay_target(
        &self,
        idempotency_scope: &str,
        idempotency_key: &str,
    ) -> Result<Option<ExternalPoolAdapterRegistryAuditTarget>> {
        validate_identifier(idempotency_scope, 240)?;
        validate_identifier(idempotency_key, 240)?;
        let conn = self.conn()?;
        let Some(binding) = binding_by_idempotency_on(&conn, idempotency_scope, idempotency_key)?
        else {
            return Ok(None);
        };
        let item = &binding.receipt.binding;
        target_on(
            &conn,
            &item.installation_receipt_id,
            &item.installation_receipt_digest,
        )
    }

    pub(crate) fn external_pool_adapter_registry_provider_binding_audit_target(
        &self,
        provider_binding_id: &str,
    ) -> Result<Option<ExternalPoolAdapterRegistryAuditTarget>> {
        validate_identifier(provider_binding_id, 200)?;
        let conn = self.conn()?;
        let Some(binding) = super::read::binding_by_id_on(&conn, provider_binding_id)? else {
            return Ok(None);
        };
        let item = &binding.receipt.binding;
        target_on(
            &conn,
            &item.installation_receipt_id,
            &item.installation_receipt_digest,
        )
    }
}

fn target_on(
    conn: &Connection,
    receipt_id: &str,
    expected: &str,
) -> Result<Option<ExternalPoolAdapterRegistryAuditTarget>> {
    let Some(authority) =
        external_pool_adapter_installation_receipt_authority_on(conn, receipt_id, expected)?
    else {
        return Ok(None);
    };
    let receipt = authority.receipt();
    Ok(Some(ExternalPoolAdapterRegistryAuditTarget {
        installation_receipt_id: receipt.installation_receipt_id.clone(),
        installation_receipt_digest: receipt.installation_receipt_digest.clone(),
        installation_binding: receipt.installation.binding.clone(),
    }))
}

fn binding_by_installation_on(conn: &Connection, receipt_id: &str) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT provider_binding_id FROM compute_external_pool_adapter_registry_provider_bindings WHERE installation_receipt_id=?1",
        params![receipt_id], |row| row.get(0),
    ).optional()?)
}

fn validate_target(id: &str, digest_value: &str) -> Result<()> {
    validate_identifier(id, 200)?;
    if digest_value.len() != 64
        || !digest_value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        bail!("Adapter registry target digest is invalid");
    }
    Ok(())
}

fn validate_identifier(value: &str, maximum: usize) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().count() > maximum
        || value.chars().any(char::is_control)
    {
        bail!("Adapter registry target identifier is invalid");
    }
    Ok(())
}
