use anyhow::{bail, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::compute_federation::external_pool_adapter_provider_active_successor::{
    ExternalPoolAdapterProviderActiveSuccessorProcessCustody,
    ExternalPoolAdapterProviderActiveSuccessorReceipt,
    ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt,
    PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES,
};

use super::{
    audit::{audit_receipt, audit_revocation},
    types::{
        StoredExternalPoolAdapterProviderActiveSuccessor,
        StoredExternalPoolAdapterProviderActiveSuccessorRevocation,
    },
};

pub(super) fn receipt_by_id_on(
    conn: &Connection,
    receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterProviderActiveSuccessor>> {
    receipt_on(conn, "active_successor_receipt_id=?1", params![receipt_id])
}

pub(super) fn head_by_binding_and_root_on(
    conn: &Connection,
    provider_binding_id: &str,
    activation_root_digest: &str,
) -> Result<Option<StoredExternalPoolAdapterProviderActiveSuccessor>> {
    receipt_on(
        conn,
        "provider_binding_id=?1 AND activation_root_digest=?2
         ORDER BY successor_sequence DESC,active_successor_receipt_id DESC LIMIT 1",
        params![provider_binding_id, activation_root_digest],
    )
}

fn receipt_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterProviderActiveSuccessor>> {
    let sql = format!(
        "SELECT receipt_json,process_custody_epoch_digest,process_custody_nonce_digest,
                process_custody_seal_digest,receipt_integrity_digest
           FROM compute_external_pool_adapter_provider_active_successor_receipts
          WHERE {filter}"
    );
    let stored = conn
        .query_row(&sql, values, |row| {
            let receipt_json: String = row.get(0)?;
            let receipt = bounded_decode(&receipt_json).map_err(sqlite_decode_error)?;
            Ok(StoredExternalPoolAdapterProviderActiveSuccessor {
                receipt,
                receipt_json,
                process_custody: ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
                    process_custody_epoch_digest: row.get(1)?,
                    process_custody_nonce_digest: row.get(2)?,
                    process_custody_seal_digest: row.get(3)?,
                },
                receipt_integrity_digest: row.get(4)?,
            })
        })
        .optional()?;
    stored.map(|value| audit_receipt(conn, value)).transpose()
}

pub(super) fn revocation_by_target_on(
    conn: &Connection,
    target_receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterProviderActiveSuccessorRevocation>> {
    let stored = conn
        .query_row(
            "SELECT revocation_json,process_custody_epoch_digest,process_custody_nonce_digest,
                    process_custody_seal_digest,receipt_integrity_digest
               FROM compute_external_pool_adapter_provider_active_successor_revocations
              WHERE target_active_successor_receipt_id=?1",
            params![target_receipt_id],
            |row| {
                let revocation_json: String = row.get(0)?;
                let receipt = bounded_decode(&revocation_json).map_err(sqlite_decode_error)?;
                Ok(StoredExternalPoolAdapterProviderActiveSuccessorRevocation {
                    receipt,
                    revocation_json,
                    process_custody: ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
                        process_custody_epoch_digest: row.get(1)?,
                        process_custody_nonce_digest: row.get(2)?,
                        process_custody_seal_digest: row.get(3)?,
                    },
                    receipt_integrity_digest: row.get(4)?,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_revocation(conn, value))
        .transpose()
}

fn bounded_decode<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES {
        bail!("provider active-successor durable JSON exceeds its fixed bound");
    }
    Ok(serde_json::from_str(json)?)
}

fn sqlite_decode_error(error: anyhow::Error) -> rusqlite::Error {
    let source = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(source))
}
