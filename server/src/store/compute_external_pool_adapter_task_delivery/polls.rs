use anyhow::{ensure, Result};
use chrono::{DateTime, FixedOffset, SecondsFormat};
use rusqlite::{params, Connection};

use crate::compute_federation::external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskPollCommandBinding;

use crate::store::{hash_token, new_id};

use super::{
    historical_cleanup::HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority,
    poll_plan::poll_cas_values,
    reachability_pending_plan::{
        install_external_pool_adapter_task_reachability_pending_plan_on,
        ExternalPoolAdapterTaskReachabilityPendingPlan,
        ExternalPoolAdapterTaskReachabilityPendingWrite,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    },
    read::{read_event_poll_on, read_reconcile_poll_on},
    types::{ExternalPoolAdapterTaskPollClaim, CLAIM_STATUS_CLAIMED, CLAIM_STATUS_PENDING},
};

#[allow(dead_code)]
pub(in crate::store) fn try_claim_reconcile_poll_at_on<'tx, 'conn>(
    conn: &'tx rusqlite::Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<ExternalPoolAdapterTaskPollClaim>> {
    ensure!(
        claimed_at == authority.checked_at(),
        "V278 reconcile claim time differs from its historical reproof"
    );
    validate_claim_input(claim_owner_id, claimed_at, claim_expires_at)?;
    ensure!(
        claim_expires_at <= authority.cleanup_expires_at(),
        "V278 reconcile claim exceeds its route cleanup horizon"
    );
    let before = read_reconcile_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 reconcile poll disappeared before claim"))?;
    ensure_poll_binds_history(&before.envelope.poll.command, authority)?;
    ensure!(
        before.envelope.reconcile_poll_digest == poll_digest
            && before.claim.status == CLAIM_STATUS_PENDING
            && before.claim.revision == expected_revision
            && before.claim.generation == expected_generation,
        "V273 reconcile poll claim expectation is stale"
    );
    let raw_claim_token = new_id("v273_reconcile_claim");
    let claim_token_digest = hash_token(&raw_claim_token);
    let pending = install_claim_plan_on(
        conn,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::ReconcilePollCas,
        poll_id,
        poll_digest,
        &before.claim,
        claim_owner_id,
        &claim_token_digest,
        claim_expires_at,
    )?;
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_reconcile_polls
            SET claim_status='claimed',claim_revision=claim_revision+1,
                claim_generation=claim_generation+1,claim_owner_id=?1,
                claim_token_digest=?2,claim_expires_at=?3
          WHERE reconcile_poll_id=?4 AND reconcile_poll_digest=?5
            AND claim_status='pending' AND claim_revision=?6 AND claim_generation=?7
            AND not_before<=?8 AND ?8<not_after
            AND ?8<?3 AND ?3<=not_after",
        params![
            claim_owner_id,
            claim_token_digest,
            claim_expires_at,
            poll_id,
            poll_digest,
            i64::try_from(expected_revision)?,
            i64::try_from(expected_generation)?,
            claimed_at,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    ensure!(
        changed == 1,
        "V273 reconcile poll claim changed multiple rows"
    );
    let after = read_reconcile_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 claimed reconcile poll disappeared"))?;
    let claimed = finish_claim(
        poll_id,
        poll_digest,
        expected_revision,
        expected_generation,
        claim_owner_id,
        raw_claim_token,
        claim_token_digest,
        claim_expires_at,
        after.envelope.reconcile_poll_digest,
        after.claim,
    )?;
    pending.ensure_fully_consumed()?;
    Ok(Some(claimed))
}

#[allow(dead_code)]
pub(in crate::store) fn try_claim_event_poll_at_on<'tx, 'conn>(
    conn: &'tx rusqlite::Transaction<'conn>,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'tx, 'conn>,
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    claimed_at: &str,
    claim_expires_at: &str,
) -> Result<Option<ExternalPoolAdapterTaskPollClaim>> {
    ensure!(
        claimed_at == authority.checked_at(),
        "V278 event claim time differs from its historical reproof"
    );
    validate_claim_input(claim_owner_id, claimed_at, claim_expires_at)?;
    ensure!(
        claim_expires_at <= authority.cleanup_expires_at(),
        "V278 event claim exceeds its route cleanup horizon"
    );
    let before = read_event_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 event poll disappeared before claim"))?;
    ensure_poll_binds_history(&before.envelope.poll.command, authority)?;
    ensure!(
        before.envelope.event_poll_digest == poll_digest
            && before.claim.status == CLAIM_STATUS_PENDING
            && before.claim.revision == expected_revision
            && before.claim.generation == expected_generation,
        "V273 event poll claim expectation is stale"
    );
    let raw_claim_token = new_id("v273_event_claim");
    let claim_token_digest = hash_token(&raw_claim_token);
    let pending = install_claim_plan_on(
        conn,
        ExternalPoolAdapterTaskReachabilityPendingWriteKind::EventPollCas,
        poll_id,
        poll_digest,
        &before.claim,
        claim_owner_id,
        &claim_token_digest,
        claim_expires_at,
    )?;
    let changed = conn.execute(
        "UPDATE compute_external_pool_adapter_task_event_polls
            SET claim_status='claimed',claim_revision=claim_revision+1,
                claim_generation=claim_generation+1,claim_owner_id=?1,
                claim_token_digest=?2,claim_expires_at=?3
          WHERE event_poll_id=?4 AND event_poll_digest=?5
            AND claim_status='pending' AND claim_revision=?6 AND claim_generation=?7
            AND not_before<=?8 AND ?8<not_after
            AND ?8<?3 AND ?3<=not_after",
        params![
            claim_owner_id,
            claim_token_digest,
            claim_expires_at,
            poll_id,
            poll_digest,
            i64::try_from(expected_revision)?,
            i64::try_from(expected_generation)?,
            claimed_at,
        ],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    ensure!(changed == 1, "V273 event poll claim changed multiple rows");
    let after = read_event_poll_on(conn, poll_id)?
        .ok_or_else(|| anyhow::anyhow!("V273 claimed event poll disappeared"))?;
    let claimed = finish_claim(
        poll_id,
        poll_digest,
        expected_revision,
        expected_generation,
        claim_owner_id,
        raw_claim_token,
        claim_token_digest,
        claim_expires_at,
        after.envelope.event_poll_digest,
        after.claim,
    )?;
    pending.ensure_fully_consumed()?;
    Ok(Some(claimed))
}

#[allow(clippy::too_many_arguments)]
fn install_claim_plan_on(
    conn: &Connection,
    kind: ExternalPoolAdapterTaskReachabilityPendingWriteKind,
    poll_id: &str,
    poll_digest: &str,
    before: &super::types::PollClaimProjection,
    claim_owner_id: &str,
    claim_token_digest: &str,
    claim_expires_at: &str,
) -> Result<super::reachability_pending_plan::ExternalPoolAdapterTaskReachabilityPendingPlanGuard> {
    let after = super::types::PollClaimProjection {
        status: CLAIM_STATUS_CLAIMED.to_string(),
        revision: before
            .revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("V273 poll claim revision overflow"))?,
        generation: before
            .generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("V273 poll claim generation overflow"))?,
        owner_id: Some(claim_owner_id.to_string()),
        token_digest: Some(claim_token_digest.to_string()),
        expires_at: Some(claim_expires_at.to_string()),
    };
    let values = poll_cas_values(poll_id, poll_digest, before, &after)?;
    let plan = ExternalPoolAdapterTaskReachabilityPendingPlan::new(vec![
        ExternalPoolAdapterTaskReachabilityPendingWrite::new(kind, values)?,
    ])?;
    install_external_pool_adapter_task_reachability_pending_plan_on(conn, plan)
}

#[allow(clippy::too_many_arguments)]
fn finish_claim(
    poll_id: &str,
    poll_digest: &str,
    expected_revision: u64,
    expected_generation: u64,
    claim_owner_id: &str,
    raw_claim_token: String,
    claim_token_digest: String,
    claim_expires_at: &str,
    actual_poll_digest: String,
    actual: super::types::PollClaimProjection,
) -> Result<ExternalPoolAdapterTaskPollClaim> {
    ensure!(
        actual_poll_digest == poll_digest
            && actual.status == CLAIM_STATUS_CLAIMED
            && actual.revision
                == expected_revision
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("V273 poll claim revision overflow"))?
            && actual.generation
                == expected_generation
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("V273 poll claim generation overflow"))?
            && actual.owner_id.as_deref() == Some(claim_owner_id)
            && actual.token_digest.as_deref() == Some(claim_token_digest.as_str())
            && actual.expires_at.as_deref() == Some(claim_expires_at),
        "V273 poll claim readback is not exact"
    );
    Ok(ExternalPoolAdapterTaskPollClaim {
        poll_id: poll_id.to_string(),
        poll_digest: poll_digest.to_string(),
        claim_revision: actual.revision,
        claim_generation: actual.generation,
        claim_owner_id: claim_owner_id.to_string(),
        raw_claim_token,
        claim_expires_at: claim_expires_at.to_string(),
    })
}

fn validate_claim_input(owner: &str, claimed_at: &str, expires_at: &str) -> Result<()> {
    ensure!(
        !owner.is_empty()
            && owner.trim() == owner
            && owner.chars().count() <= 240
            && !owner.chars().any(char::is_control),
        "V273 poll claim owner is invalid"
    );
    for value in [claimed_at, expires_at] {
        ensure!(value.len() == 30, "V273 poll claim time is not UTC nanos");
        let parsed: DateTime<FixedOffset> = DateTime::parse_from_rfc3339(value)?;
        ensure!(
            parsed.offset().local_minus_utc() == 0
                && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value,
            "V273 poll claim time is not canonical UTC nanos"
        );
    }
    ensure!(claimed_at < expires_at, "V273 poll claim window is empty");
    Ok(())
}

pub(super) fn ensure_poll_binds_history(
    command: &ExternalPoolAdapterTaskPollCommandBinding,
    authority: &HistoricalExternalPoolAdapterTaskExchangeCleanupAuthority<'_, '_>,
) -> Result<()> {
    let identity = &authority.exchange_attempt().attempt.identity;
    ensure!(
        command.command_id == identity.command.command_id
            && command.command_digest == identity.command.command_digest
            && command.outbox_id == identity.command.outbox_id
            && command.outbox_digest == identity.command.outbox_digest
            && command.send_attempt_id == identity.command.send_attempt_id
            && command.send_attempt_digest == identity.command.send_attempt_digest
            && command.route_authorization_id == identity.route.route_authorization_id
            && command.route_authorization_digest == identity.route.route_authorization_digest
            && command.executor_binding_digest == identity.executor_binding_digest
            && command.fencing_generation == identity.fencing_generation
            && command.fence_digest == identity.fence_digest,
        "V278 poll claim does not bind the historical execution exchange"
    );
    Ok(())
}
