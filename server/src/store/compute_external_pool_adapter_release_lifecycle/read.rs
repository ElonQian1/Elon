use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::external_pool_adapter_release_lifecycle::{
        validate_external_pool_adapter_release_admission_terminal_receipt,
        ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION,
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION,
    },
    store::{compute_external_pool_adapter_release::admission_by_id_on, Store},
};

use super::{
    canonical::canonical_terminal_json_and_digest,
    types::{
        AuditedAdmission, CurrentExternalPoolAdapterReleaseAdmissionAuthority,
        ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt, StoredTerminalReceipt,
        CURRENTNESS_RECEIPT_SCHEMA,
    },
};

struct CurrentViewRow {
    admission_id: String,
    admission_digest: String,
    adapter_id: String,
    release_version: String,
    applied_at: String,
    admission_status: String,
    current_status: String,
    terminal_receipt_id: Option<String>,
    terminal_receipt_digest: Option<String>,
    terminal_occurred_at: Option<String>,
    successor_admission_id: Option<String>,
    successor_admission_digest: Option<String>,
    successor_release_version: Option<String>,
}

pub(super) fn historical_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<AuditedAdmission>> {
    let Some(admission) = admission_by_id_on(conn, admission_id)? else {
        return Ok(None);
    };
    let applied_at = conn
        .query_row(
            "SELECT applied_at FROM compute_external_pool_adapter_release_admissions
              WHERE admission_id=?1 AND admission_digest=?2 AND adapter_id=?3
                AND release_version=?4 AND status='staged'",
            params![
                admission.admission_id,
                admission.admission_digest,
                admission.adapter_id,
                admission.release_version,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("audited Adapter release admission projection drifted"))?;
    canonical_nanos(&applied_at)?;
    Ok(Some(AuditedAdmission {
        admission,
        applied_at,
    }))
}

pub(super) fn terminal_by_admission_on(
    conn: &Connection,
    admission_id: &str,
) -> Result<Option<StoredTerminalReceipt>> {
    terminal_on(conn, "WHERE admission_id=?1", params![admission_id])
}

pub(super) fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredTerminalReceipt>> {
    terminal_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn terminal_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredTerminalReceipt>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT terminal_receipt_json
                   FROM compute_external_pool_adapter_release_admission_terminal_receipts
                   {filter}"
            ),
            values,
            |row| {
                let terminal_receipt_json: String = row.get(0)?;
                let terminal_receipt: ComputeExternalPoolAdapterReleaseAdmissionTerminalReceipt =
                    serde_json::from_str(&terminal_receipt_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                    })?;
                Ok(StoredTerminalReceipt {
                    terminal_receipt,
                    terminal_receipt_json,
                })
            },
        )
        .optional()?;
    stored.map(|value| audit_terminal(conn, value)).transpose()
}

fn audit_terminal(
    conn: &Connection,
    stored: StoredTerminalReceipt,
) -> Result<StoredTerminalReceipt> {
    validate_external_pool_adapter_release_admission_terminal_receipt(&stored.terminal_receipt)?;
    let (terminal_json, terminal_digest) =
        canonical_terminal_json_and_digest(&stored.terminal_receipt)?;
    let terminal = &stored.terminal_receipt.terminal;
    let base =
        historical_admission_on(conn, &terminal.admission.admission_id)?.ok_or_else(|| {
            anyhow::anyhow!("Adapter release admission terminal lost its immutable admission")
        })?;
    if terminal.admission.admission_digest != base.admission.admission_digest
        || terminal.admission.adapter_id != base.admission.adapter_id
        || terminal.admission.release_version != base.admission.release_version
        || base.applied_at > terminal.occurred_at
    {
        bail!("Adapter release admission terminal base lineage drifted");
    }
    if let Some(successor) = terminal.successor_admission.as_ref() {
        let audited = historical_admission_on(conn, &successor.admission_id)?.ok_or_else(|| {
            anyhow::anyhow!("Adapter release admission terminal lost its successor")
        })?;
        if successor.admission_digest != audited.admission.admission_digest
            || successor.release_version != audited.admission.release_version
            || audited.admission.adapter_id != base.admission.adapter_id
            || audited.admission.admission_id == base.admission.admission_id
            || audited.admission.release_version == base.admission.release_version
            || audited.applied_at < base.applied_at
            || audited.applied_at > terminal.occurred_at
        {
            bail!("Adapter release admission terminal successor lineage drifted");
        }
    }
    let successor = terminal.successor_admission.as_ref();
    let projected = conn
        .query_row(
            "SELECT 1
               FROM compute_external_pool_adapter_release_admission_terminal_receipts
              WHERE terminal_receipt_id=?1 AND terminal_receipt_schema=?2
                AND terminal_receipt_digest=?3 AND terminal_receipt_json=?4
                AND canonicalization=?5 AND digest_algorithm=?6 AND request_digest=?7
                AND admission_id=?8 AND admission_digest=?9 AND adapter_id=?10
                AND release_version=?11 AND prior_status=?12 AND terminal_status=?13
                AND successor_admission_id IS ?14 AND successor_admission_digest IS ?15
                AND successor_release_version IS ?16 AND actor_kind=?17 AND actor_id=?18
                AND reason=?19 AND confirmation=?20 AND idempotency_scope=?21
                AND idempotency_key=?22 AND occurred_at=?23 AND recorded_at=?24
                AND currentness_effect=?25 AND artifact_intake_effect=?26
                AND existing_artifact_source_effect=?27 AND adapter_effect=?28
                AND route_effect=?29",
            params![
                stored.terminal_receipt.terminal_receipt_id,
                stored.terminal_receipt.schema,
                stored.terminal_receipt.terminal_receipt_digest,
                stored.terminal_receipt_json,
                stored.terminal_receipt.canonicalization,
                stored.terminal_receipt.digest_algorithm,
                stored.terminal_receipt.request_digest,
                terminal.admission.admission_id,
                terminal.admission.admission_digest,
                terminal.admission.adapter_id,
                terminal.admission.release_version,
                terminal.prior_status,
                terminal.terminal_status,
                successor.map(|value| value.admission_id.as_str()),
                successor.map(|value| value.admission_digest.as_str()),
                successor.map(|value| value.release_version.as_str()),
                terminal.actor_kind,
                terminal.actor_id,
                terminal.reason,
                terminal.confirmation,
                terminal.idempotency_scope,
                terminal.idempotency_key,
                terminal.occurred_at,
                terminal.recorded_at,
                terminal.currentness_effect,
                terminal.artifact_intake_effect,
                terminal.existing_artifact_source_effect,
                terminal.adapter_effect,
                terminal.route_effect,
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if terminal_json != stored.terminal_receipt_json
        || terminal_digest != stored.terminal_receipt.terminal_receipt_digest
        || !projected
    {
        bail!("Adapter release admission terminal failed exact readback audit");
    }
    Ok(stored)
}

fn current_view_on(conn: &Connection, admission_id: &str) -> Result<Option<CurrentViewRow>> {
    conn.query_row(
        "SELECT admission_id, admission_digest, adapter_id, release_version, applied_at,
                admission_status, current_status, terminal_receipt_id, terminal_receipt_digest,
                terminal_occurred_at, successor_admission_id, successor_admission_digest,
                successor_release_version
           FROM compute_external_pool_adapter_release_admission_current
          WHERE admission_id=?1",
        params![admission_id],
        |row| {
            Ok(CurrentViewRow {
                admission_id: row.get(0)?,
                admission_digest: row.get(1)?,
                adapter_id: row.get(2)?,
                release_version: row.get(3)?,
                applied_at: row.get(4)?,
                admission_status: row.get(5)?,
                current_status: row.get(6)?,
                terminal_receipt_id: row.get(7)?,
                terminal_receipt_digest: row.get(8)?,
                terminal_occurred_at: row.get(9)?,
                successor_admission_id: row.get(10)?,
                successor_admission_digest: row.get(11)?,
                successor_release_version: row.get(12)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn lifecycle_on(
    conn: &Connection,
    admission_id: &str,
    expected_admission_digest: &str,
) -> Result<
    Option<(
        AuditedAdmission,
        ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt,
    )>,
> {
    let Some(base) = historical_admission_on(conn, admission_id)? else {
        return Ok(None);
    };
    if base.admission.admission_digest != expected_admission_digest {
        bail!("Adapter release admission currentness digest is not exact");
    }
    let terminal = terminal_by_admission_on(conn, admission_id)?;
    let view = current_view_on(conn, admission_id)?.ok_or_else(|| {
        anyhow::anyhow!("Adapter release admission is absent from its current view")
    })?;
    let terminal_material = terminal
        .as_ref()
        .map(|value| &value.terminal_receipt.terminal);
    let successor = terminal_material.and_then(|value| value.successor_admission.as_ref());
    let expected_status = terminal_material
        .map(|value| value.terminal_status.as_str())
        .unwrap_or(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED);
    if view.admission_id != base.admission.admission_id
        || view.admission_digest != base.admission.admission_digest
        || view.adapter_id != base.admission.adapter_id
        || view.release_version != base.admission.release_version
        || view.applied_at != base.applied_at
        || view.admission_status != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED
        || view.current_status != expected_status
        || view.terminal_receipt_id.as_deref()
            != terminal
                .as_ref()
                .map(|value| value.terminal_receipt.terminal_receipt_id.as_str())
        || view.terminal_receipt_digest.as_deref()
            != terminal
                .as_ref()
                .map(|value| value.terminal_receipt.terminal_receipt_digest.as_str())
        || view.terminal_occurred_at.as_deref()
            != terminal_material.map(|value| value.occurred_at.as_str())
        || view.successor_admission_id.as_deref()
            != successor.map(|value| value.admission_id.as_str())
        || view.successor_admission_digest.as_deref()
            != successor.map(|value| value.admission_digest.as_str())
        || view.successor_release_version.as_deref()
            != successor.map(|value| value.release_version.as_str())
    {
        bail!("Adapter release admission current view failed exact audit");
    }
    let receipt = ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt {
        schema: CURRENTNESS_RECEIPT_SCHEMA,
        admission_id: base.admission.admission_id.clone(),
        admission_digest: base.admission.admission_digest.clone(),
        adapter_id: base.admission.adapter_id.clone(),
        release_version: base.admission.release_version.clone(),
        admission_status: EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED,
        current_status: expected_status.to_string(),
        applied_at: base.applied_at.clone(),
        terminal_receipt_id: view.terminal_receipt_id,
        terminal_receipt_digest: view.terminal_receipt_digest,
        terminal_occurred_at: view.terminal_occurred_at,
        successor_admission: successor.cloned(),
    };
    Ok(Some((base, receipt)))
}

pub(in crate::store) fn current_external_pool_adapter_release_admission_authority_on(
    conn: &Connection,
    admission_id: &str,
    expected_admission_digest: &str,
) -> Result<Option<CurrentExternalPoolAdapterReleaseAdmissionAuthority>> {
    let Some((base, currentness)) = lifecycle_on(conn, admission_id, expected_admission_digest)?
    else {
        return Ok(None);
    };
    if currentness.current_status != EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_STAGED {
        bail!("Adapter release admission is terminal and cannot be consumed");
    }
    Ok(Some(
        CurrentExternalPoolAdapterReleaseAdmissionAuthority::new(base.admission, base.applied_at),
    ))
}

impl Store {
    pub(crate) fn external_pool_adapter_release_admission_currentness(
        &self,
        admission_id: &str,
    ) -> Result<Option<ExternalPoolAdapterReleaseAdmissionCurrentnessReceipt>> {
        validate_exact(admission_id, "Adapter release admission ID", 160)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction()?;
        let receipt = match historical_admission_on(&transaction, admission_id)? {
            Some(base) => {
                lifecycle_on(&transaction, admission_id, &base.admission.admission_digest)?
                    .map(|(_, value)| value)
            }
            None => None,
        };
        transaction.commit()?;
        Ok(receipt)
    }
}

pub(super) fn expected_confirmation(status: &str) -> Result<&'static str> {
    match status {
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_WITHDRAWN => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_WITHDRAWAL_CONFIRMATION)
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_REVOKED => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_REVOCATION_CONFIRMATION)
        }
        EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_STATUS_SUPERSEDED => {
            Ok(EXTERNAL_POOL_ADAPTER_RELEASE_ADMISSION_SUPERSESSION_CONFIRMATION)
        }
        _ => bail!("Adapter release admission terminal status is unsupported"),
    }
}

pub(super) fn validate_exact(value: &str, label: &str, max: usize) -> Result<()> {
    if value.is_empty()
        || value.chars().count() > max
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("{label} must be non-empty, bounded, and exact");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

pub(super) fn validate_reason(value: &str) -> Result<()> {
    let length = value.chars().count();
    if !(8..=2_000).contains(&length)
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("Adapter release admission terminal reason must be exact and 8..2000 characters");
    }
    Ok(())
}

fn canonical_nanos(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("Adapter release admission terminal timestamp is not canonical UTC nanoseconds");
    }
    Ok(())
}
