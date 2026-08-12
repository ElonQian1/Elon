use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_adoption::{
        canonical_adoption_receipt_json_and_digest,
        canonical_adoption_terminal_receipt_json_and_digest, validate_adoption_receipt,
        validate_adoption_terminal_receipt, ADOPTION_CURRENTNESS_SCHEMA,
    },
    store::{
        compute_external_pool_adapter_artifact_sandbox_conformance::external_pool_adapter_sandbox_conformance_receipt_authority_on,
        compute_external_pool_adapter_credential_verification::external_pool_adapter_credential_verification_receipt_authority_on,
        Store,
    },
};

use super::types::*;

pub(super) fn adoption_by_id_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterAdoption>> {
    adoption_on(conn, "adoption_receipt_id=?1", params![receipt_id])
}

pub(super) fn adoption_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterAdoption>> {
    adoption_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

pub(super) fn adoption_by_lineage_on(
    conn: &Connection,
    application_id: &str,
    admission_id: &str,
) -> Result<Option<StoredExternalPoolAdapterAdoption>> {
    adoption_on(
        conn,
        "application_id=?1 AND admission_id=?2",
        params![application_id, admission_id],
    )
}

fn adoption_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterAdoption>> {
    conn.query_row(
        &format!("SELECT receipt_json FROM compute_external_pool_adapter_adoption_receipts WHERE {filter}"),
        values,
        |row| decode_adoption(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_adoption(conn, stored))
    .transpose()
}

pub(super) fn terminal_by_adoption_on(
    conn: &Connection,
    adoption_receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterAdoptionTerminal>> {
    conn.query_row(
        "SELECT receipt_json FROM compute_external_pool_adapter_adoption_terminal_receipts
          WHERE adoption_receipt_id=?1",
        [adoption_receipt_id],
        |row| decode_terminal(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_terminal(conn, stored))
    .transpose()
}

pub(super) fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterAdoptionTerminal>> {
    conn.query_row(
        "SELECT receipt_json FROM compute_external_pool_adapter_adoption_terminal_receipts
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
        |row| decode_terminal(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_terminal(conn, stored))
    .transpose()
}

fn decode_adoption(receipt_json: String) -> rusqlite::Result<StoredExternalPoolAdapterAdoption> {
    let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredExternalPoolAdapterAdoption {
        receipt,
        receipt_json,
    })
}

fn decode_terminal(
    receipt_json: String,
) -> rusqlite::Result<StoredExternalPoolAdapterAdoptionTerminal> {
    let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredExternalPoolAdapterAdoptionTerminal {
        receipt,
        receipt_json,
    })
}

fn audit_adoption(
    conn: &Connection,
    stored: StoredExternalPoolAdapterAdoption,
) -> Result<StoredExternalPoolAdapterAdoption> {
    validate_adoption_receipt(&stored.receipt)?;
    let (json, digest) = canonical_adoption_receipt_json_and_digest(&stored.receipt)?;
    let binding = &stored.receipt.adoption.binding;
    let sandbox = external_pool_adapter_sandbox_conformance_receipt_authority_on(
        conn,
        &binding.admission_id,
        &binding.sandbox_conformance_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter adoption lost sandbox receipt root"))?;
    let credential = external_pool_adapter_credential_verification_receipt_authority_on(
        conn,
        &binding.credential_verification_receipt_id,
        &binding.credential_verification_receipt_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("Adapter adoption lost credential receipt root"))?;
    let sandbox_binding = &sandbox.receipt().conformance.binding;
    let credential_binding = &credential.receipt().verification.binding;
    if json != stored.receipt_json
        || digest != stored.receipt.adoption_receipt_digest
        || sandbox_binding.admission_digest != binding.admission_digest
        || sandbox_binding.adapter_id != binding.adapter_id
        || sandbox_binding.release_version != binding.adapter_release_version
        || sandbox_binding.declared_implementation_sha256 != binding.declared_implementation_sha256
        || sandbox_binding.capability_set_digest != binding.capability_set_digest
        || sandbox.receipt().sandbox_conformance_receipt_id
            != binding.sandbox_conformance_receipt_id
        || sandbox_binding.report_expires_at != binding.sandbox_report_expires_at
        || credential_binding.application_id != binding.application_id
        || credential_binding.application_digest != binding.application_digest
        || credential_binding.provider_id != binding.provider_id
        || credential_binding.provider_owner_account_id != binding.provider_owner_account_id
        || credential_binding.provider_policy_revision != binding.provider_policy_revision
        || credential_binding.provider_digest != binding.provider_digest
        || credential_binding.admission_id != binding.admission_id
        || credential_binding.admission_digest != binding.admission_digest
        || credential_binding.adapter_id != binding.adapter_id
        || credential_binding.adapter_release_version != binding.adapter_release_version
        || credential_binding.adapter_config_revision != binding.adapter_config_revision
        || credential_binding.adapter_config_digest != binding.adapter_config_digest
        || credential_binding.credential_locator_commitment != binding.credential_locator_commitment
        || credential_binding.report_expires_at != binding.credential_report_expires_at
        || !exact_adoption_projection(conn, &stored)?
    {
        bail!("Adapter adoption failed exact readback audit");
    }
    Ok(stored)
}

fn audit_terminal(
    conn: &Connection,
    stored: StoredExternalPoolAdapterAdoptionTerminal,
) -> Result<StoredExternalPoolAdapterAdoptionTerminal> {
    validate_adoption_terminal_receipt(&stored.receipt)?;
    let (json, digest) = canonical_adoption_terminal_receipt_json_and_digest(&stored.receipt)?;
    let item = &stored.receipt.terminal;
    let adoption = adoption_by_id_on(conn, &item.adoption_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("Adapter adoption terminal lost adoption root"))?;
    if json != stored.receipt_json
        || digest != stored.receipt.terminal_receipt_digest
        || adoption.receipt.adoption_receipt_digest != item.adoption_receipt_digest
        || !exact_terminal_projection(conn, &stored)?
    {
        bail!("Adapter adoption terminal failed exact readback audit");
    }
    Ok(stored)
}

fn exact_adoption_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterAdoption,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.adoption;
    let binding = &item.binding;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_adoption_receipts
          WHERE adoption_receipt_id=?1 AND adoption_receipt_digest=?2 AND receipt_json=?3
            AND adoption_material_digest=?4 AND application_id=?5 AND application_digest=?6
            AND provider_id=?7 AND provider_owner_account_id=?8
            AND provider_policy_revision=?9 AND provider_digest=?10
            AND admission_id=?11 AND admission_digest=?12 AND adapter_id=?13
            AND adapter_release_version=?14 AND adapter_config_revision=?15
            AND adapter_config_digest=?16 AND declared_implementation_sha256=?17
            AND capability_set_digest=?18 AND sandbox_conformance_receipt_id=?19
            AND sandbox_conformance_receipt_digest=?20 AND sandbox_report_expires_at=?21
            AND credential_verification_receipt_id=?22
            AND credential_verification_receipt_digest=?23
            AND credential_locator_commitment=?24 AND credential_report_expires_at=?25
            AND adopted_by_admin_user_id=?26 AND confirmation=?27
            AND idempotency_scope=?28 AND idempotency_key=?29 AND adopted_at=?30
            AND recorded_at=?31 AND adoption_effect=?32 AND install_effect=?33
            AND provider_effect=?34 AND route_effect=?35 AND execution_effect=?36
            AND settlement_effect=?37",
            params![
                receipt.adoption_receipt_id,
                receipt.adoption_receipt_digest,
                stored.receipt_json,
                receipt.adoption_material_digest,
                binding.application_id,
                binding.application_digest,
                binding.provider_id,
                binding.provider_owner_account_id,
                binding.provider_policy_revision,
                binding.provider_digest,
                binding.admission_id,
                binding.admission_digest,
                binding.adapter_id,
                binding.adapter_release_version,
                binding.adapter_config_revision,
                binding.adapter_config_digest,
                binding.declared_implementation_sha256,
                binding.capability_set_digest,
                binding.sandbox_conformance_receipt_id,
                binding.sandbox_conformance_receipt_digest,
                binding.sandbox_report_expires_at,
                binding.credential_verification_receipt_id,
                binding.credential_verification_receipt_digest,
                binding.credential_locator_commitment,
                binding.credential_report_expires_at,
                item.adopted_by_admin_user_id,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.adopted_at,
                item.recorded_at,
                item.adoption_effect,
                item.install_effect,
                item.provider_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn exact_terminal_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterAdoptionTerminal,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.terminal;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_adoption_terminal_receipts
              WHERE terminal_receipt_id=?1 AND terminal_receipt_digest=?2
                AND receipt_json=?3 AND terminal_material_digest=?4
                AND adoption_receipt_id=?5 AND adoption_receipt_digest=?6
                AND revoked_by_admin_user_id=?7 AND reason=?8 AND confirmation=?9
                AND idempotency_scope=?10 AND idempotency_key=?11 AND revoked_at=?12
                AND recorded_at=?13 AND adoption_effect=?14 AND provider_effect=?15
                AND route_effect=?16 AND execution_effect=?17 AND settlement_effect=?18",
            params![
                receipt.terminal_receipt_id,
                receipt.terminal_receipt_digest,
                stored.receipt_json,
                receipt.terminal_material_digest,
                item.adoption_receipt_id,
                item.adoption_receipt_digest,
                item.revoked_by_admin_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.revoked_at,
                item.recorded_at,
                item.adoption_effect,
                item.provider_effect,
                item.route_effect,
                item.execution_effect,
                item.settlement_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn currentness_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<ExternalPoolAdapterAdoptionCurrentness>> {
    let Some(stored) = adoption_by_id_on(conn, receipt_id)? else {
        return Ok(None);
    };
    let terminal = terminal_by_adoption_on(conn, receipt_id)?;
    let statuses: (String, String, String, String) = conn.query_row(
        "SELECT current_status,sandbox_conformance_status,credential_verification_status,
                terminal_status FROM compute_external_pool_adapter_adoption_current
          WHERE adoption_receipt_id=?1 AND adoption_receipt_digest=?2",
        params![receipt_id, stored.receipt.adoption_receipt_digest],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    Ok(Some(ExternalPoolAdapterAdoptionCurrentness {
        schema: ADOPTION_CURRENTNESS_SCHEMA,
        adoption: stored.summary(),
        terminal: terminal
            .as_ref()
            .map(StoredExternalPoolAdapterAdoptionTerminal::summary),
        current_status: statuses.0,
        sandbox_conformance_status: statuses.1,
        credential_verification_status: statuses.2,
        terminal_status: statuses.3,
    }))
}

pub(in crate::store) fn current_external_pool_adapter_adoption_authority_on(
    conn: &Connection,
    receipt_id: &str,
    expected_receipt_digest: &str,
) -> Result<Option<CurrentExternalPoolAdapterAdoptionAuthority>> {
    let Some(currentness) = currentness_on(conn, receipt_id)? else {
        return Ok(None);
    };
    if currentness.current_status != "adopted_current"
        || currentness.adoption.adoption_receipt_digest != expected_receipt_digest
    {
        bail!("Adapter adoption authority is not current and exact");
    }
    Ok(adoption_by_id_on(conn, receipt_id)?
        .map(|stored| CurrentExternalPoolAdapterAdoptionAuthority::new(stored.receipt)))
}

impl Store {
    pub(crate) fn external_pool_adapter_adoption_currentness(
        &self,
        receipt_id: &str,
    ) -> Result<Option<ExternalPoolAdapterAdoptionCurrentness>> {
        let connection = self.conn()?;
        currentness_on(&connection, receipt_id)
    }
}
