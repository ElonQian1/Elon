use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_provider_runtime_readiness::*;

const RECEIPT_EXACT: &str = "elon_v270_provider_runtime_readiness_receipt_is_exact";
const REVOCATION_EXACT: &str = "elon_v270_provider_runtime_readiness_revocation_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(readiness_receipt_is_exact),
        ))
    })?;
    conn.create_scalar_function(REVOCATION_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(revocation_receipt_is_exact),
        ))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_receipts
         WHEN {RECEIPT_EXACT}(NEW.readiness_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V270 readiness canonical receipt integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v270_provider_runtime_readiness_revocation_integrity
         BEFORE INSERT ON compute_external_pool_adapter_provider_runtime_readiness_revocations
         WHEN {REVOCATION_EXACT}(NEW.revocation_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V270 readiness revocation canonical integrity mismatch'); END;"
    ))?;
    Ok(())
}

fn text(context: &rusqlite::functions::Context<'_>, index: usize) -> Option<&str> {
    context.get_raw(index).as_str().ok()
}

fn readiness_receipt_is_exact(json: &str) -> bool {
    let Ok(receipt) = bounded_parse::<ExternalPoolAdapterProviderRuntimeReadinessReceipt>(json)
    else {
        return false;
    };
    validate_provider_runtime_readiness_receipt(&receipt).is_ok()
        && canonical_provider_runtime_readiness_receipt_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn revocation_receipt_is_exact(json: &str) -> bool {
    let Ok(receipt) =
        bounded_parse::<ExternalPoolAdapterProviderRuntimeReadinessRevocationReceipt>(json)
    else {
        return false;
    };
    validate_provider_runtime_readiness_revocation_receipt(&receipt).is_ok()
        && canonical_provider_runtime_readiness_revocation_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn bounded_parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > PROVIDER_RUNTIME_READINESS_MAX_RECEIPT_JSON_BYTES {
        anyhow::bail!("V270 receipt exceeds the durable bound")
    }
    Ok(serde_json::from_str(json)?)
}
