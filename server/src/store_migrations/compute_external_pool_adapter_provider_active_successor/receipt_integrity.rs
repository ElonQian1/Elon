use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::{
    compute_federation::external_pool_adapter_provider_active_successor::*,
    store::compute_external_pool_adapter_runtime_bundle::verify_pending_external_pool_adapter_provider_active_successor_process_seal,
};

const RECEIPT_EXACT: &str = "elon_v274_provider_active_successor_receipt_is_exact";
const REVOCATION_EXACT: &str = "elon_v274_provider_active_successor_revocation_is_exact";
const INTEGRITY_EXACT: &str = "elon_v274_provider_active_successor_receipt_integrity_is_exact";
const PENDING_EXACT: &str = "elon_v274_provider_active_successor_pending_process_seal_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let deterministic = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(RECEIPT_EXACT, 1, deterministic, |context| {
        Ok(i64::from(text(context, 0).is_some_and(receipt_is_exact)))
    })?;
    conn.create_scalar_function(REVOCATION_EXACT, 1, deterministic, |context| {
        Ok(i64::from(text(context, 0).is_some_and(revocation_is_exact)))
    })?;
    conn.create_scalar_function(INTEGRITY_EXACT, 6, deterministic, |context| {
        Ok(i64::from(integrity_is_exact(context)))
    })?;
    let pending = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(PENDING_EXACT, 7, pending, |context| {
        Ok(i64::from(pending_process_seal_is_exact(context)))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
         WHEN {RECEIPT_EXACT}(NEW.receipt_json) IS NOT 1
           OR {INTEGRITY_EXACT}('provider_active_successor_receipt',NEW.receipt_digest,
                NEW.process_custody_epoch_digest,NEW.process_custody_nonce_digest,
                NEW.process_custody_seal_digest,NEW.receipt_integrity_digest) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V274 active successor canonical/private integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_receipt_pending_seal
         BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_receipts
         WHEN {PENDING_EXACT}('provider_active_successor_receipt',NEW.active_successor_receipt_id,
                NEW.receipt_digest,NEW.process_custody_epoch_digest,
                NEW.process_custody_nonce_digest,NEW.process_custody_seal_digest,
                NEW.receipt_integrity_digest) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V274 active successor lacks exact pending process seal'); END;
         CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_integrity
         BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_revocations
         WHEN {REVOCATION_EXACT}(NEW.revocation_json) IS NOT 1
           OR {INTEGRITY_EXACT}('provider_active_successor_revocation',NEW.revocation_digest,
                NEW.process_custody_epoch_digest,NEW.process_custody_nonce_digest,
                NEW.process_custody_seal_digest,NEW.receipt_integrity_digest) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V274 active successor revocation integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v274_provider_active_successor_revocation_pending_seal
         BEFORE INSERT ON compute_external_pool_adapter_provider_active_successor_revocations
         WHEN {PENDING_EXACT}('provider_active_successor_revocation',NEW.active_successor_revocation_id,
                NEW.revocation_digest,NEW.process_custody_epoch_digest,
                NEW.process_custody_nonce_digest,NEW.process_custody_seal_digest,
                NEW.receipt_integrity_digest) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V274 revocation lacks exact pending process seal'); END;"
    ))?;
    Ok(())
}

fn receipt_is_exact(json: &str) -> bool {
    let Ok(receipt) = bounded_parse::<ExternalPoolAdapterProviderActiveSuccessorReceipt>(json)
    else {
        return false;
    };
    validate_external_pool_adapter_provider_active_successor_receipt(&receipt).is_ok()
        && canonical_external_pool_adapter_provider_active_successor_receipt_json_and_digest(
            &receipt,
        )
        .is_ok_and(|(canonical, _)| canonical == json)
}

fn revocation_is_exact(json: &str) -> bool {
    let Ok(receipt) =
        bounded_parse::<ExternalPoolAdapterProviderActiveSuccessorRevocationReceipt>(json)
    else {
        return false;
    };
    validate_external_pool_adapter_provider_active_successor_revocation(&receipt).is_ok()
        && canonical_external_pool_adapter_provider_active_successor_revocation_json_and_digest(
            &receipt,
        )
        .is_ok_and(|(canonical, _)| canonical == json)
}

fn integrity_is_exact(context: &rusqlite::functions::Context<'_>) -> bool {
    let Some(kind) = text(context, 0) else {
        return false;
    };
    if !matches!(
        kind,
        PROVIDER_ACTIVE_SUCCESSOR_RECEIPT_PROCESS_KIND
            | PROVIDER_ACTIVE_SUCCESSOR_REVOCATION_PROCESS_KIND
    ) {
        return false;
    }
    let Some(entity_digest) = text(context, 1) else {
        return false;
    };
    let Some(epoch) = text(context, 2) else {
        return false;
    };
    let Some(nonce) = text(context, 3) else {
        return false;
    };
    let Some(seal) = text(context, 4) else {
        return false;
    };
    let Some(expected) = text(context, 5) else {
        return false;
    };
    let custody = ExternalPoolAdapterProviderActiveSuccessorProcessCustody {
        process_custody_epoch_digest: epoch.into(),
        process_custody_nonce_digest: nonce.into(),
        process_custody_seal_digest: seal.into(),
    };
    provider_active_successor_private_integrity_digest(kind, entity_digest, &custody)
        .is_ok_and(|actual| actual == expected)
}

fn pending_process_seal_is_exact(context: &rusqlite::functions::Context<'_>) -> bool {
    let values = (0..7)
        .map(|index| text(context, index))
        .collect::<Option<Vec<_>>>();
    let Some(values) = values else {
        return false;
    };
    verify_pending_external_pool_adapter_provider_active_successor_process_seal(
        values[0], values[1], values[2], values[3], values[4], values[5], values[6],
    )
}

fn text<'a>(context: &'a rusqlite::functions::Context<'a>, index: usize) -> Option<&'a str> {
    context.get_raw(index).as_str().ok()
}

fn bounded_parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > PROVIDER_ACTIVE_SUCCESSOR_MAX_JSON_BYTES {
        anyhow::bail!("V274 entity exceeds the durable bound")
    }
    Ok(serde_json::from_str(json)?)
}
