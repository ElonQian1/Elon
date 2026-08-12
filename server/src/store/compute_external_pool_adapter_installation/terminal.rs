use anyhow::{bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{
    params, types::Type, Connection, OptionalExtension, Transaction, TransactionBehavior,
};

use crate::{
    compute_federation::external_pool_adapter_installation::{
        canonical_external_pool_adapter_installation_terminal_receipt_json_and_digest,
        installation_terminal_material_digest,
        validate_external_pool_adapter_installation_terminal_receipt,
        ExternalPoolAdapterInstallationTerminalMaterial,
        ExternalPoolAdapterInstallationTerminalReceipt, INSTALLATION_CANONICALIZATION,
        INSTALLATION_DIGEST_ALGORITHM, INSTALLATION_NO_EFFECT,
        INSTALLATION_REVOCATION_CONFIRMATION, INSTALLATION_REVOKED_EFFECT,
        INSTALLATION_TERMINAL_KIND_REVOKED, INSTALLATION_TERMINAL_RECEIPT_SCHEMA,
    },
    store::{new_id, Store},
};

use super::{read::receipt_by_id_on, types::*};

pub(super) fn terminal_by_installation_on(
    conn: &Connection,
    installation_receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterInstallationTerminal>> {
    terminal_on(
        conn,
        "installation_receipt_id=?1",
        params![installation_receipt_id],
    )
}

/// Store-internal terminal probe for downstream authorities that must ignore expired
/// short-lived attestations while still failing closed on an explicit installation revoke.
pub(in crate::store) fn external_pool_adapter_installation_is_revoked_on(
    conn: &Connection,
    installation_receipt_id: &str,
) -> Result<bool> {
    Ok(terminal_by_installation_on(conn, installation_receipt_id)?.is_some())
}

fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredExternalPoolAdapterInstallationTerminal>> {
    terminal_on(
        conn,
        "idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn terminal_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterInstallationTerminal>> {
    conn.query_row(
        &format!(
            "SELECT receipt_json
               FROM compute_external_pool_adapter_installation_terminal_receipts
              WHERE {filter}"
        ),
        values,
        |row| decode_terminal(row.get(0)?),
    )
    .optional()?
    .map(|stored| audit_terminal(conn, stored))
    .transpose()
}

fn decode_terminal(
    receipt_json: String,
) -> rusqlite::Result<StoredExternalPoolAdapterInstallationTerminal> {
    let receipt = serde_json::from_str(&receipt_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
    })?;
    Ok(StoredExternalPoolAdapterInstallationTerminal {
        receipt,
        receipt_json,
    })
}

fn audit_terminal(
    conn: &Connection,
    stored: StoredExternalPoolAdapterInstallationTerminal,
) -> Result<StoredExternalPoolAdapterInstallationTerminal> {
    validate_external_pool_adapter_installation_terminal_receipt(&stored.receipt)?;
    let (json, digest) =
        canonical_external_pool_adapter_installation_terminal_receipt_json_and_digest(
            &stored.receipt,
        )?;
    let item = &stored.receipt.terminal;
    let installation = receipt_by_id_on(conn, &item.installation_receipt_id)?
        .ok_or_else(|| anyhow::anyhow!("Adapter installation terminal lost installation root"))?;
    if json != stored.receipt_json
        || digest != stored.receipt.terminal_receipt_digest
        || installation.receipt.installation_receipt_digest != item.installation_receipt_digest
        || !exact_projection(conn, &stored)?
    {
        bail!("Adapter installation terminal failed exact readback audit");
    }
    Ok(stored)
}

fn exact_projection(
    conn: &Connection,
    stored: &StoredExternalPoolAdapterInstallationTerminal,
) -> Result<bool> {
    let receipt = &stored.receipt;
    let item = &receipt.terminal;
    Ok(conn
        .query_row(
            "SELECT 1 FROM compute_external_pool_adapter_installation_terminal_receipts
              WHERE terminal_receipt_id=?1 AND terminal_receipt_digest=?2
                AND terminal_receipt_schema=?3 AND receipt_json=?4
                AND terminal_material_digest=?5 AND canonicalization=?6
                AND digest_algorithm=?7 AND installation_receipt_id=?8
                AND installation_receipt_digest=?9 AND terminal_kind=?10
                AND revoked_by_admin_user_id=?11 AND reason=?12 AND confirmation=?13
                AND idempotency_scope=?14 AND idempotency_key=?15 AND revoked_at=?16
                AND recorded_at=?17 AND installation_effect=?18
                AND credential_effect=?19 AND provider_effect=?20 AND route_effect=?21
                AND execution_effect=?22 AND settlement_effect=?23",
            params![
                receipt.terminal_receipt_id,
                receipt.terminal_receipt_digest,
                receipt.schema,
                stored.receipt_json,
                receipt.terminal_material_digest,
                receipt.canonicalization,
                receipt.digest_algorithm,
                item.installation_receipt_id,
                item.installation_receipt_digest,
                item.terminal_kind,
                item.revoked_by_admin_user_id,
                item.reason,
                item.confirmation,
                item.idempotency_scope,
                item.idempotency_key,
                item.revoked_at,
                item.recorded_at,
                item.installation_effect,
                item.credential_effect,
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

impl Store {
    pub(crate) fn revoke_external_pool_adapter_installation(
        &self,
        input: RevokeExternalPoolAdapterInstallation,
    ) -> Result<ExternalPoolAdapterInstallationTerminalWriteReceipt> {
        validate_input(&input)?;
        let mut connection = self.conn()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(terminal) =
            terminal_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            ensure_replay(&terminal.receipt, &input)?;
            let installation = receipt_by_id_on(&tx, &input.installation_receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("Adapter installation was not found"))?;
            let output = write_receipt(&installation, &terminal, true);
            tx.commit()?;
            return Ok(output);
        }
        let installation = receipt_by_id_on(&tx, &input.installation_receipt_id)?
            .ok_or_else(|| anyhow::anyhow!("Adapter installation was not found"))?;
        if installation.receipt.installation_receipt_digest
            != input.expected_installation_receipt_digest
        {
            bail!("Adapter installation revocation digest is stale");
        }
        if terminal_by_installation_on(&tx, &input.installation_receipt_id)?.is_some() {
            bail!("Adapter installation already has an immutable terminal receipt");
        }
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        let material = ExternalPoolAdapterInstallationTerminalMaterial {
            installation_receipt_id: input.installation_receipt_id,
            installation_receipt_digest: input.expected_installation_receipt_digest,
            terminal_kind: INSTALLATION_TERMINAL_KIND_REVOKED.to_string(),
            revoked_by_admin_user_id: input.revoked_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            revoked_at: timestamp.clone(),
            recorded_at: timestamp,
            installation_effect: INSTALLATION_REVOKED_EFFECT.to_string(),
            credential_effect: INSTALLATION_NO_EFFECT.to_string(),
            provider_effect: INSTALLATION_NO_EFFECT.to_string(),
            route_effect: INSTALLATION_NO_EFFECT.to_string(),
            execution_effect: INSTALLATION_NO_EFFECT.to_string(),
            settlement_effect: INSTALLATION_NO_EFFECT.to_string(),
        };
        let mut receipt = ExternalPoolAdapterInstallationTerminalReceipt {
            schema: INSTALLATION_TERMINAL_RECEIPT_SCHEMA.to_string(),
            terminal_receipt_id: new_id("external_pool_adapter_installation_terminal"),
            terminal_receipt_digest: String::new(),
            terminal_material_digest: installation_terminal_material_digest(&material)?,
            canonicalization: INSTALLATION_CANONICALIZATION.to_string(),
            digest_algorithm: INSTALLATION_DIGEST_ALGORITHM.to_string(),
            terminal: material,
        };
        receipt.terminal_receipt_digest =
            canonical_external_pool_adapter_installation_terminal_receipt_json_and_digest(
                &receipt,
            )?
            .1;
        validate_external_pool_adapter_installation_terminal_receipt(&receipt)?;
        let (json, digest) =
            canonical_external_pool_adapter_installation_terminal_receipt_json_and_digest(
                &receipt,
            )?;
        if digest != receipt.terminal_receipt_digest {
            bail!("Adapter installation terminal digest changed before persistence");
        }
        insert_terminal(&tx, &receipt, &json)?;
        let terminal = terminal_by_installation_on(&tx, &receipt.terminal.installation_receipt_id)?
            .ok_or_else(|| {
                anyhow::anyhow!("Adapter installation terminal disappeared after insert")
            })?;
        if terminal.receipt != receipt || terminal.receipt_json != json {
            bail!("Adapter installation terminal changed during exact readback");
        }
        let output = write_receipt(&installation, &terminal, false);
        tx.commit()?;
        Ok(output)
    }
}

fn insert_terminal(
    tx: &Transaction<'_>,
    receipt: &ExternalPoolAdapterInstallationTerminalReceipt,
    json: &str,
) -> Result<()> {
    let item = &receipt.terminal;
    tx.execute(
        "INSERT INTO compute_external_pool_adapter_installation_terminal_receipts(
        terminal_receipt_id,terminal_receipt_digest,terminal_receipt_schema,receipt_json,
        terminal_material_digest,canonicalization,digest_algorithm,installation_receipt_id,
        installation_receipt_digest,terminal_kind,revoked_by_admin_user_id,reason,confirmation,
        idempotency_scope,idempotency_key,revoked_at,recorded_at,installation_effect,
        credential_effect,provider_effect,route_effect,execution_effect,settlement_effect)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
                ?19,?20,?21,?22,?23)",
        params![
            receipt.terminal_receipt_id,
            receipt.terminal_receipt_digest,
            receipt.schema,
            json,
            receipt.terminal_material_digest,
            receipt.canonicalization,
            receipt.digest_algorithm,
            item.installation_receipt_id,
            item.installation_receipt_digest,
            item.terminal_kind,
            item.revoked_by_admin_user_id,
            item.reason,
            item.confirmation,
            item.idempotency_scope,
            item.idempotency_key,
            item.revoked_at,
            item.recorded_at,
            item.installation_effect,
            item.credential_effect,
            item.provider_effect,
            item.route_effect,
            item.execution_effect,
            item.settlement_effect,
        ],
    )?;
    Ok(())
}

fn ensure_replay(
    receipt: &ExternalPoolAdapterInstallationTerminalReceipt,
    input: &RevokeExternalPoolAdapterInstallation,
) -> Result<()> {
    let item = &receipt.terminal;
    if item.installation_receipt_id != input.installation_receipt_id
        || item.installation_receipt_digest != input.expected_installation_receipt_digest
        || item.revoked_by_admin_user_id != input.revoked_by_admin_user_id
        || item.reason != input.reason
        || item.confirmation != input.confirmation
        || item.idempotency_scope != input.idempotency_scope
        || item.idempotency_key != input.idempotency_key
    {
        bail!("Adapter installation revocation idempotency key conflicts with immutable history");
    }
    Ok(())
}

fn validate_input(input: &RevokeExternalPoolAdapterInstallation) -> Result<()> {
    for (value, maximum) in [
        (input.installation_receipt_id.as_str(), 200),
        (input.revoked_by_admin_user_id.as_str(), 200),
        (input.reason.as_str(), 1000),
        (input.idempotency_scope.as_str(), 240),
        (input.idempotency_key.as_str(), 240),
    ] {
        if value.is_empty()
            || value.trim() != value
            || value.chars().count() > maximum
            || value.chars().any(char::is_control)
        {
            bail!("Adapter installation revocation input is invalid");
        }
    }
    if !is_sha256(&input.expected_installation_receipt_digest)
        || input.confirmation != INSTALLATION_REVOCATION_CONFIRMATION
    {
        bail!("Adapter installation revocation digest or confirmation is invalid");
    }
    Ok(())
}

fn write_receipt(
    installation: &StoredExternalPoolAdapterInstallation,
    terminal: &StoredExternalPoolAdapterInstallationTerminal,
    replayed: bool,
) -> ExternalPoolAdapterInstallationTerminalWriteReceipt {
    ExternalPoolAdapterInstallationTerminalWriteReceipt {
        installation: installation.summary(),
        terminal: terminal.summary(),
        replayed,
    }
}
