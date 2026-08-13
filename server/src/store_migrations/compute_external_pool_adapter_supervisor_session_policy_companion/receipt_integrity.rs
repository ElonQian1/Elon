use anyhow::Result;
use rusqlite::{functions::FunctionFlags, Connection};

use crate::compute_federation::external_pool_adapter_supervisor_session_policy_companion::{
    canonical_supervisor_session_companion_json_and_digest,
    canonical_supervisor_session_companion_revocation_json_and_digest,
    validate_supervisor_session_companion_receipt,
    validate_supervisor_session_companion_revocation_receipt,
    ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt,
    ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
};

const COMPANION_RECEIPT_IS_EXACT: &str = "elon_v259_supervisor_session_companion_receipt_is_exact";
const REVOCATION_RECEIPT_IS_EXACT: &str =
    "elon_v259_supervisor_session_companion_revocation_receipt_is_exact";

pub(super) fn register(conn: &Connection) -> Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    conn.create_scalar_function(COMPANION_RECEIPT_IS_EXACT, 1, flags, |context| {
        let exact = context
            .get_raw(0)
            .as_str()
            .ok()
            .is_some_and(companion_receipt_is_exact);
        Ok(i64::from(exact))
    })?;
    conn.create_scalar_function(REVOCATION_RECEIPT_IS_EXACT, 1, flags, |context| {
        let exact = context
            .get_raw(0)
            .as_str()
            .ok()
            .is_some_and(revocation_receipt_is_exact);
        Ok(i64::from(exact))
    })?;
    Ok(())
}

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companions
         WHEN {COMPANION_RECEIPT_IS_EXACT}(NEW.companion_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V259 supervisor/session companion receipt canonical/digest integrity mismatch'); END;
         CREATE TRIGGER IF NOT EXISTS external_pool_adapter_supervisor_session_policy_companion_revocation_receipt_integrity
         BEFORE INSERT ON compute_external_pool_adapter_supervisor_session_policy_companion_revocations
         WHEN {REVOCATION_RECEIPT_IS_EXACT}(NEW.revocation_json) IS NOT 1
         BEGIN SELECT RAISE(ABORT,'V259 supervisor/session companion revocation receipt canonical/digest integrity mismatch'); END;"
    ))?;
    Ok(())
}

fn companion_receipt_is_exact(json: &str) -> bool {
    if json.len() > 1_048_576 {
        return false;
    }
    let Ok(receipt) =
        serde_json::from_str::<ExternalPoolAdapterSupervisorSessionPolicyCompanionReceipt>(json)
    else {
        return false;
    };
    validate_supervisor_session_companion_receipt(&receipt).is_ok()
        && canonical_supervisor_session_companion_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}

fn revocation_receipt_is_exact(json: &str) -> bool {
    if json.len() > 1_048_576 {
        return false;
    }
    let Ok(receipt) = serde_json::from_str::<
        ExternalPoolAdapterSupervisorSessionPolicyCompanionRevocationReceipt,
    >(json) else {
        return false;
    };
    validate_supervisor_session_companion_revocation_receipt(&receipt).is_ok()
        && canonical_supervisor_session_companion_revocation_json_and_digest(&receipt)
            .is_ok_and(|(canonical, _)| canonical == json)
}
