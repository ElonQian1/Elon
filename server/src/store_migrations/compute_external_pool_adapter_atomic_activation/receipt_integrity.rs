use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_atomic_activation::{
    canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest,
    validate_external_pool_adapter_atomic_activation_receipt,
    ExternalPoolAdapterAtomicActivationReceipt, ATOMIC_ACTIVATION_MAX_JSON_BYTES,
};

const RECEIPT_EXACT: &str = "elon_v277_external_pool_adapter_atomic_activation_receipt_is_exact";
const PENDING_MATCHES: &str =
    "elon_v277_external_pool_adapter_atomic_activation_pending_plan_matches";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(RECEIPT_EXACT, 1, flags, |context| {
        Ok(i64::from(
            context
                .get_raw(0)
                .as_str()
                .ok()
                .is_some_and(receipt_is_exact),
        ))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_atomic_activation_receipts
         WHEN {RECEIPT_EXACT}(NEW.activation_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V277 atomic activation canonical integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS v277_atomic_activation_receipt_pending_plan
         BEFORE INSERT ON compute_external_pool_adapter_atomic_activation_receipts
         WHEN {PENDING_MATCHES}('activation_receipt',NEW.activation_receipt_id,
              NEW.activation_receipt_digest,NEW.activation_root_digest,
              NEW.activation_receipt_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V277 atomic activation lacks exact pending plan'); END;"
    ))?;
    Ok(())
}

fn receipt_is_exact(json: &str) -> bool {
    if json.len() > ATOMIC_ACTIVATION_MAX_JSON_BYTES {
        return false;
    }
    let Ok(receipt) = serde_json::from_str::<ExternalPoolAdapterAtomicActivationReceipt>(json)
    else {
        return false;
    };
    validate_external_pool_adapter_atomic_activation_receipt(&receipt).is_ok()
        && canonical_external_pool_adapter_atomic_activation_receipt_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}
