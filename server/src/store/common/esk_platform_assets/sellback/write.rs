use anyhow::Result;
use rusqlite::{params, TransactionBehavior};

use crate::{esk_asset::platform::sellback::*, store::Store};

use super::{
    authenticate, new_id, now,
    records::global_reserved_on,
    snapshot::{scan_on, Selection, Snapshot},
};

impl Store {
    pub(crate) fn submit_esk_platform_sellback(
        &self,
        user: &str,
        token: &str,
        input: &SellbackSubmitInput,
        config: &SellbackConfiguration,
    ) -> Result<SellbackResult> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        authenticate(&tx, user, token)?;
        validate_input(input)?;
        let before = scan_on(
            &tx,
            user,
            token,
            config,
            Selection::Key(&input.idempotency_key),
        )?;
        if let Some(existing) = &before.selected {
            if existing.input != *input {
                return Err(SellbackError::Conflict.into());
            }
            authenticate(&tx, user, token)?;
            let result = before.result(true)?;
            tx.commit()?;
            return Ok(result);
        }
        let policy = new_policy(&before, input)?;
        let formal = before.formal.as_ref().ok_or(SellbackError::Corrupt)?;
        let global = global_reserved_on(&tx, formal)?;
        let global_limit = positive_units(&policy.body.max_reserved_base_units_global)?;
        if global
            .checked_add(input.amount_base_units)
            .filter(|value| *value <= global_limit)
            .is_none()
        {
            return Err(SellbackError::LimitExceeded.into());
        }
        let record = SellbackRecord {
            request_id: new_id("eskpsr"),
            user_id: user.into(),
            input: input.clone(),
            request_digest: request_digest(user, policy, input)?,
            policy: policy.clone(),
            created_at: now(),
            canceled_at: None,
            cancel_event_id: None,
        };
        validate_stored_request(&record)?;
        tx.execute(
            "INSERT INTO esk_platform_sellback_requests(request_id,user_id,idempotency_key,
             amount_base_units,request_digest,input_json,policy_json,platform_policy_digest,source_fingerprint,created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![record.request_id, user, input.idempotency_key, input.amount_base_units,
                record.request_digest, serde_json::to_string(input)?, serde_json::to_string(policy)?,
                formal.policy_digest, formal.source_fingerprint, record.created_at],
        )?;
        let result =
            scan_on(&tx, user, token, config, Selection::Id(&record.request_id))?.result(false)?;
        authenticate(&tx, user, token)?;
        tx.commit()?;
        Ok(result)
    }
}

fn new_policy<'a>(
    snapshot: &'a Snapshot,
    input: &SellbackSubmitInput,
) -> Result<&'a SellbackPolicy> {
    let summary = &snapshot.page.summary;
    let policy = summary.availability.policy.as_ref().ok_or_else(|| {
        match summary.availability.reason.as_str() {
            "user_not_eligible" => SellbackError::Ineligible,
            "source_mismatch" => SellbackError::PolicyChanged,
            _ => SellbackError::Disabled,
        }
    })?;
    if input.policy_digest != policy.policy_digest || input.terms_digest != policy.body.terms_digest
    {
        return Err(SellbackError::PolicyChanged.into());
    }
    if input.expected_snapshot_digest != summary.snapshot_digest {
        return Err(SellbackError::SnapshotChanged.into());
    }
    let count_limit = positive_units(&policy.body.max_open_requests_per_user)?;
    let user_limit = positive_units(&policy.body.max_reserved_base_units_per_user)?;
    if input.amount_base_units < positive_units(&policy.body.min_request_base_units)?
        || input.amount_base_units > positive_units(&policy.body.max_request_base_units)?
        || summary
            .open_request_count
            .checked_add(1)
            .filter(|value| *value <= count_limit)
            .is_none()
        || summary
            .reserved_base_units
            .checked_add(input.amount_base_units)
            .filter(|value| *value <= user_limit)
            .is_none()
    {
        return Err(SellbackError::LimitExceeded.into());
    }
    if input.amount_base_units > summary.available_base_units {
        return Err(SellbackError::InsufficientAvailable.into());
    }
    Ok(policy)
}
