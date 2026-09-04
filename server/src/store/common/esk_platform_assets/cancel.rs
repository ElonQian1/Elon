use anyhow::Result;
use rusqlite::{params, TransactionBehavior};

use crate::{
    esk_asset::platform::{PlatformAllocationRecord, PlatformError, PlatformPolicy},
    store::Store,
};

use super::{
    ensure_admin, now, policy_on,
    read::{ensure_recording_integrity, record_on},
    require_same_policy,
};

impl Store {
    /// Cancels only an unrecorded application; it never reverses a balance entry.
    pub(crate) fn cancel_esk_platform_allocation(
        &self,
        policy: &PlatformPolicy,
        allocation_id: &str,
        expected_digest: &str,
        actor_user_id: &str,
        actor_session_token: &str,
    ) -> Result<PlatformAllocationRecord> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_admin(&tx, actor_user_id, actor_session_token)?;
        let stored_policy = policy_on(&tx)?.ok_or(PlatformError::NotFound)?;
        require_same_policy(&stored_policy, policy)?;
        ensure_recording_integrity(&tx)?;
        let mut record = record_on(&tx, allocation_id, policy)?.ok_or(PlatformError::NotFound)?;
        if record.recorded_at.is_some() || record.input.request_digest != expected_digest {
            return Err(PlatformError::Conflict.into());
        }
        // An unavailable recipient must not permanently lock an incorrect prepared payment.
        if record.canceled_at.is_some() {
            record.replayed = true;
            ensure_admin(&tx, actor_user_id, actor_session_token)?;
            tx.commit()?;
            return Ok(record);
        }
        tx.execute(
            "INSERT INTO esk_platform_cancellations (
               allocation_id, request_digest, canceled_by, created_at
             ) VALUES (?1, ?2, ?3, ?4)",
            params![allocation_id, expected_digest, actor_user_id, now()],
        )?;
        ensure_recording_integrity(&tx)?;
        let result = record_on(&tx, allocation_id, policy)?.ok_or(PlatformError::CorruptLedger)?;
        if result.canceled_at.is_none() || result.recorded_at.is_some() {
            return Err(PlatformError::CorruptLedger.into());
        }
        ensure_admin(&tx, actor_user_id, actor_session_token)?;
        tx.commit()?;
        Ok(result)
    }
}
