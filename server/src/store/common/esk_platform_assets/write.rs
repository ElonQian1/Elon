use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

use crate::{
    esk_asset::platform::{
        validate_prepared_input, PlatformAllocationInput, PlatformAllocationRecord, PlatformError,
        PlatformPolicy,
    },
    store::Store,
};

use super::{
    ensure_active_user, ensure_admin, new_id, now, policy_on,
    read::{checked_totals_on, ensure_recording_integrity, record_on},
    require_same_policy,
};

impl Store {
    pub(crate) fn prepare_esk_platform_allocation(
        &self,
        policy: &PlatformPolicy,
        input: &PlatformAllocationInput,
        actor_user_id: &str,
        actor_session_token: &str,
    ) -> Result<PlatformAllocationRecord> {
        validate_prepared_input(policy, input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_admin(&tx, actor_user_id, actor_session_token)?;
        ensure_active_user(&tx, &input.user_id)?;
        ensure_recording_integrity(&tx)?;
        pin_policy_on(&tx, policy, actor_user_id)?;
        let existing = tx
            .query_row(
                "SELECT a.allocation_id FROM esk_platform_allocations a WHERE a.payment_key = ?1
                   AND NOT EXISTS (SELECT 1 FROM esk_platform_cancellations c WHERE c.allocation_id = a.allocation_id)",
                params![input.payment_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(id) = existing {
            let mut record = record_on(&tx, &id, policy)?.ok_or(PlatformError::CorruptLedger)?;
            if record.input != *input {
                return Err(PlatformError::Conflict.into());
            }
            record.replayed = true;
            ensure_admin(&tx, actor_user_id, actor_session_token)?;
            tx.commit()?;
            return Ok(record);
        }
        let allocation_id = new_id("eskp_allocation");
        tx.execute(
            "INSERT INTO esk_platform_allocations (
               allocation_id, payment_key, policy_digest, user_id, amount_base_units,
               request_digest, input_json, prepared_by, prepared_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                allocation_id,
                input.payment_key,
                input.policy_digest,
                input.user_id,
                input.amount_base_units,
                input.request_digest,
                serde_json::to_string(input)?,
                actor_user_id,
                now(),
            ],
        )?;
        let record = record_on(&tx, &allocation_id, policy)?.ok_or(PlatformError::CorruptLedger)?;
        ensure_admin(&tx, actor_user_id, actor_session_token)?;
        tx.commit()?;
        Ok(record)
    }

    pub(crate) fn record_esk_platform_allocation(
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
        if record.canceled_at.is_some() || record.input.request_digest != expected_digest {
            return Err(PlatformError::Conflict.into());
        }
        ensure_active_user(&tx, &record.input.user_id)?;
        if record.recorded_at.is_some() {
            record.replayed = true;
            ensure_admin(&tx, actor_user_id, actor_session_token)?;
            tx.commit()?;
            return Ok(record);
        }
        check_issuance_limit(&tx, policy, &record.input)?;
        let approval_id = new_id("eskp_approval");
        let created_at = now();
        tx.execute(
            "INSERT INTO esk_platform_approvals (
               approval_id, allocation_id, request_digest, approved_by, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                approval_id,
                allocation_id,
                expected_digest,
                actor_user_id,
                created_at
            ],
        )?;
        tx.execute(
            "INSERT INTO esk_platform_ledger_entries (
               entry_id, allocation_id, approval_id, user_id, amount_base_units, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                new_id("eskp_entry"),
                allocation_id,
                approval_id,
                record.input.user_id,
                record.input.amount_base_units,
                created_at,
            ],
        )?;
        ensure_recording_integrity(&tx)?;
        let result = record_on(&tx, allocation_id, policy)?.ok_or(PlatformError::CorruptLedger)?;
        if result.recorded_at.is_none() {
            return Err(PlatformError::CorruptLedger.into());
        }
        ensure_admin(&tx, actor_user_id, actor_session_token)?;
        tx.commit()?;
        Ok(result)
    }
}

fn pin_policy_on(conn: &Connection, policy: &PlatformPolicy, actor_user_id: &str) -> Result<()> {
    if let Some(stored) = policy_on(conn)? {
        return require_same_policy(&stored, policy);
    }
    conn.execute(
        "INSERT INTO esk_platform_policy (
           singleton, policy_digest, source_fingerprint, source_json,
           issuance_limit_base_units, pinned_by_user_id, created_at
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            policy.policy_digest,
            policy.source_fingerprint,
            serde_json::to_string(&policy.source)?,
            policy.issuance_limit_base_units,
            actor_user_id,
            now(),
        ],
    )?;
    Ok(())
}

fn check_issuance_limit(
    conn: &Connection,
    policy: &PlatformPolicy,
    input: &PlatformAllocationInput,
) -> Result<()> {
    let (global, user) = checked_totals_on(conn, &input.user_id)?;
    let next_global = global
        .checked_add(input.amount_base_units)
        .ok_or(PlatformError::LimitExceeded)?;
    user.checked_add(input.amount_base_units)
        .ok_or(PlatformError::LimitExceeded)?;
    if next_global > policy.issuance_limit_base_units {
        return Err(PlatformError::LimitExceeded.into());
    }
    Ok(())
}
