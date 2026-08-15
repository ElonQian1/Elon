use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_conformance::*;

pub(super) const RUN_RECEIPT_EXACT: &str =
    "elon_v272_task_protocol_conformance_run_receipt_is_exact";
pub(super) const REVOCATION_RECEIPT_EXACT: &str =
    "elon_v272_task_protocol_conformance_revocation_receipt_is_exact";
pub(super) const RECEIPT_INTEGRITY_EXACT: &str =
    "elon_v272_task_protocol_conformance_receipt_integrity_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(RUN_RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(run_receipt_is_exact),
        ))
    })?;
    conn.create_scalar_function(REVOCATION_RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(
            text(context, 0).is_some_and(revocation_receipt_is_exact),
        ))
    })?;
    conn.create_scalar_function(RECEIPT_INTEGRITY_EXACT, 4, flags, |context| {
        let values = [
            text(context, 0),
            text(context, 1),
            text(context, 2),
            text(context, 3),
        ];
        Ok(i64::from(
            matches!(values, [Some(run), Some(epoch), Some(seal), Some(integrity)]
            if task_protocol_conformance_receipt_integrity_digest(run, epoch, seal)
                .is_ok_and(|expected| expected == integrity)),
        ))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_run_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
         WHEN {RUN_RECEIPT_EXACT}(NEW.run_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V272 task protocol canonical run receipt integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_process_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_run_receipts
         WHEN {RECEIPT_INTEGRITY_EXACT}(
                NEW.run_receipt_digest,NEW.runtime_custody_epoch_digest,
                NEW.process_hmac_seal,NEW.receipt_integrity_digest) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V272 task protocol process receipt integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v272_task_protocol_conformance_revocation_integrity
         BEFORE INSERT ON compute_external_pool_adapter_task_protocol_conformance_revocations
         WHEN {REVOCATION_RECEIPT_EXACT}(NEW.revocation_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V272 task protocol revocation receipt integrity mismatch'); END;"
    ))?;
    Ok(())
}

fn text(context: &rusqlite::functions::Context<'_>, index: usize) -> Option<&str> {
    context.get_raw(index).as_str().ok()
}

fn run_receipt_is_exact(json: &str) -> bool {
    let Ok(receipt) = bounded_parse::<ExternalPoolAdapterTaskProtocolConformanceRunReceipt>(json)
    else {
        return false;
    };
    validate_task_protocol_conformance_run_receipt(&receipt).is_ok()
        && canonical_task_protocol_conformance_run_receipt_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn revocation_receipt_is_exact(json: &str) -> bool {
    let Ok(receipt) =
        bounded_parse::<ExternalPoolAdapterTaskProtocolConformanceRevocationReceipt>(json)
    else {
        return false;
    };
    validate_task_protocol_conformance_revocation_receipt(&receipt).is_ok()
        && canonical_task_protocol_conformance_revocation_receipt_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn bounded_parse<T: serde::de::DeserializeOwned>(json: &str) -> Result<T> {
    if json.len() > TASK_PROTOCOL_CONFORMANCE_MAX_RECEIPT_JSON_BYTES {
        anyhow::bail!("V272 receipt exceeds the durable bound")
    }
    Ok(serde_json::from_str(json)?)
}
