use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
        validate_supervisor_session_companion_receipt,
        validate_supervisor_session_companion_revocation_receipt,
    },
    store::{
        compute_external_pool_adapter_installation::external_pool_adapter_installation_receipt_authority_on,
        Store,
    },
};

use super::{audit_companion::audit_companion, audit_revocation::audit_revocation, types::*};

pub(super) fn companion_by_id_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredSupervisorSessionPolicyCompanion>> {
    companion_on(conn, "companion_id=?1", params![id])
}
pub(super) fn companion_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSupervisorSessionPolicyCompanion>> {
    companion_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}
pub(super) fn companion_head_by_binding_on(
    conn: &Connection,
    binding_id: &str,
) -> Result<Option<StoredSupervisorSessionPolicyCompanion>> {
    companion_on(
        conn,
        "provider_binding_id=?1 ORDER BY sequence DESC LIMIT 1",
        params![binding_id],
    )
}
fn companion_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSupervisorSessionPolicyCompanion>> {
    conn.query_row(&format!("SELECT companion_json FROM compute_external_pool_adapter_supervisor_session_policy_companions WHERE {filter}"),values,|row|decode(row,0).map(|(receipt,receipt_json)|StoredSupervisorSessionPolicyCompanion{receipt,receipt_json})).optional()?.map(|s|{validate_supervisor_session_companion_receipt(&s.receipt)?;audit_companion(conn,s)}).transpose()
}
pub(super) fn revocation_by_companion_on(
    conn: &Connection,
    id: &str,
) -> Result<Option<StoredSupervisorSessionPolicyCompanionRevocation>> {
    revocation_on(conn, "companion_id=?1", params![id])
}
pub(super) fn revocation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredSupervisorSessionPolicyCompanionRevocation>> {
    revocation_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}
fn revocation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredSupervisorSessionPolicyCompanionRevocation>> {
    conn.query_row(&format!("SELECT revocation_json FROM compute_external_pool_adapter_supervisor_session_policy_companion_revocations WHERE {filter}"),values,|row|decode(row,0).map(|(receipt,receipt_json)|StoredSupervisorSessionPolicyCompanionRevocation{receipt,receipt_json})).optional()?.map(|s|{validate_supervisor_session_companion_revocation_receipt(&s.receipt)?;audit_revocation(conn,s)}).transpose()
}

impl Store {
    pub(crate) fn external_pool_adapter_supervisor_session_policy_companion_audit_target(
        &self,
        companion_id: &str,
    ) -> Result<Option<ExternalPoolAdapterSupervisorSessionPolicyCompanionAuditTarget>> {
        identifier(companion_id)?;
        let conn = self.conn()?;
        let Some(stored) = companion_by_id_on(&conn, companion_id)? else {
            return Ok(None);
        };
        let c = &stored.receipt.companion;
        let Some(installation) = external_pool_adapter_installation_receipt_authority_on(
            &conn,
            &c.installation_receipt_id,
            &c.installation_receipt_digest,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(
            ExternalPoolAdapterSupervisorSessionPolicyCompanionAuditTarget {
                companion_id: stored.receipt.companion_id,
                companion_digest: stored.receipt.companion_digest,
                target_id: c.target_id.clone(),
                target_digest: c.target_digest.clone(),
                profile_id: c.profile_id.clone(),
                profile_digest: c.profile_digest.clone(),
                candidate_id: c.candidate_id.clone(),
                provider_binding_id: c.provider_binding_id.clone(),
                provider_owner_account_id: c.provider_owner_account_id.clone(),
                installation_binding: installation.receipt().installation.binding.clone(),
            },
        ))
    }
}
fn decode<T: serde::de::DeserializeOwned>(
    row: &rusqlite::Row<'_>,
    index: usize,
) -> rusqlite::Result<(T, String)> {
    let json: String = row.get(index)?;
    let receipt = serde_json::from_str(&json)
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(e)))?;
    Ok((receipt, json))
}
fn identifier(v: &str) -> Result<()> {
    if v.is_empty() || v.trim() != v || v.len() > 240 || v.chars().any(char::is_control) {
        bail!("supervisor session companion identifier is invalid")
    }
    Ok(())
}
