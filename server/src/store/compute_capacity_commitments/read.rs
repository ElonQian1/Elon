use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState},
    capacity_commitment::{
        ComputeCapacityCommitment, ComputeCapacityCommitmentQuantity,
        ComputeCapacityCommitmentTerminalReceipt, CAPACITY_COMMITMENT_STATUS_CANCELED,
        CAPACITY_COMMITMENT_STATUS_COMMITTED, CAPACITY_COMMITMENT_STATUS_EXPIRED,
    },
};

use super::{
    super::{
        compute_capacity_claim_rows::{stored_claim_on, stored_claim_version_on},
        compute_capacity_posting::{
            capacity_causal_transaction_on, held_claim_causal_transaction_on,
        },
        compute_delivery_allocations::{
            delivery_allocation_commitment_status_on, DeliveryAllocationCommitmentState,
        },
    },
    audit_historical_immutable_dependencies_on, audit_immutable_dependencies_on,
    audit_ledger_binding_on,
    canonical::{canonical_commitment_json_and_digest, canonical_terminal_json_and_digest},
    claim_state_name,
    types::{ComputeCapacityCommitmentCreateReceipt, ComputeCapacityCommitmentDetail},
};

pub(super) fn commitment_by_id_on(
    conn: &Connection,
    commitment_id: &str,
) -> Result<Option<ComputeCapacityCommitment>> {
    commitment_by_id_with_dependency_policy_on(conn, commitment_id, false)
}

pub(super) fn historical_commitment_by_id_on(
    conn: &Connection,
    commitment_id: &str,
) -> Result<Option<ComputeCapacityCommitment>> {
    commitment_by_id_with_dependency_policy_on(conn, commitment_id, true)
}

fn commitment_by_id_with_dependency_policy_on(
    conn: &Connection,
    commitment_id: &str,
    use_historical_dependencies: bool,
) -> Result<Option<ComputeCapacityCommitment>> {
    let stored = conn
        .query_row(
            "SELECT commitment_json, commitment_digest
               FROM compute_capacity_commitments WHERE commitment_id=?1",
            params![commitment_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((json, indexed_digest)) = stored else {
        return Ok(None);
    };
    let commitment: ComputeCapacityCommitment =
        serde_json::from_str(&json).map_err(|error| anyhow!("容量承诺 JSON 无效: {error}"))?;
    let (canonical, digest) = canonical_commitment_json_and_digest(&commitment)?;
    if commitment.commitment_id != commitment_id
        || commitment.commitment_digest != indexed_digest
        || digest != indexed_digest
        || canonical != json
    {
        bail!("容量承诺 JSON、身份或摘要审计失败");
    }
    audit_commitment_indexes_on(conn, &commitment)?;
    if use_historical_dependencies {
        audit_historical_immutable_dependencies_on(conn, &commitment)?;
    } else {
        audit_immutable_dependencies_on(conn, &commitment)?;
    }
    Ok(Some(commitment))
}

pub(super) fn commitment_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<ComputeCapacityCommitment>> {
    let commitment_id = conn
        .query_row(
            "SELECT commitment_id FROM compute_capacity_commitments
              WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    commitment_id
        .map(|commitment_id| commitment_by_id_on(conn, &commitment_id))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn terminal_by_commitment_on(
    conn: &Connection,
    commitment: &ComputeCapacityCommitment,
) -> Result<Option<ComputeCapacityCommitmentTerminalReceipt>> {
    let stored = conn
        .query_row(
            "SELECT terminal_receipt_json, terminal_receipt_digest
               FROM compute_capacity_commitment_terminal_receipts
              WHERE commitment_id=?1",
            params![commitment.commitment_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((json, indexed_digest)) = stored else {
        return Ok(None);
    };
    let receipt: ComputeCapacityCommitmentTerminalReceipt = serde_json::from_str(&json)
        .map_err(|error| anyhow!("容量承诺终态回执 JSON 无效: {error}"))?;
    let (canonical, digest) = canonical_terminal_json_and_digest(&receipt)?;
    if receipt.terminal_receipt_digest != indexed_digest
        || digest != indexed_digest
        || canonical != json
        || receipt.commitment_id != commitment.commitment_id
        || receipt.commitment_digest != commitment.commitment_digest
    {
        bail!("容量承诺终态回执 JSON、身份或摘要审计失败");
    }
    audit_terminal_indexes_on(conn, &receipt)?;
    audit_terminal_claim_and_ledger_on(conn, commitment, &receipt)?;
    Ok(Some(receipt))
}

pub(super) fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<
    Option<(
        ComputeCapacityCommitment,
        ComputeCapacityCommitmentTerminalReceipt,
    )>,
> {
    let commitment_id = conn
        .query_row(
            "SELECT commitment_id FROM compute_capacity_commitment_terminal_receipts
              WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(commitment_id) = commitment_id else {
        return Ok(None);
    };
    let commitment = commitment_by_id_on(conn, &commitment_id)?
        .ok_or_else(|| anyhow!("容量承诺终态重放缺少 immutable root"))?;
    let terminal = terminal_by_commitment_on(conn, &commitment)?
        .ok_or_else(|| anyhow!("容量承诺终态重放缺少 immutable receipt"))?;
    Ok(Some((commitment, terminal)))
}

pub(super) fn create_receipt_on(
    conn: &Connection,
    commitment: ComputeCapacityCommitment,
    replayed: bool,
) -> Result<ComputeCapacityCommitmentCreateReceipt> {
    let quantities = audit_claim_and_hold_on(conn, &commitment)?;
    Ok(ComputeCapacityCommitmentCreateReceipt {
        commitment,
        quantities,
        replayed,
    })
}

pub(super) fn detail_on(
    conn: &Connection,
    commitment: ComputeCapacityCommitment,
) -> Result<ComputeCapacityCommitmentDetail> {
    let quantities = audit_claim_and_hold_on(conn, &commitment)?;
    let terminal_receipt = terminal_by_commitment_on(conn, &commitment)?;
    let allocation = delivery_allocation_commitment_status_on(
        conn,
        &commitment.commitment_id,
        &commitment.commitment_digest,
    )?;
    if terminal_receipt.is_some()
        && allocation
            .as_ref()
            .is_some_and(|status| status.blocks_commitment_terminal())
    {
        bail!("容量承诺不能同时具有 v225 terminal 与 active/exercised Allocation");
    }
    let current = stored_claim_on(conn, &commitment.claim.claim_id)?
        .ok_or_else(|| anyhow!("容量承诺 current Claim 缺失"))?;
    match (terminal_receipt.as_ref(), allocation.as_ref()) {
        (None, Some(status))
            if status.state == DeliveryAllocationCommitmentState::Exercised
                && current.revision == commitment.claim.claim_revision + 1
                && current.state == ComputeCapacityClaimState::Released => {}
        (None, _)
            if current.revision == commitment.claim.claim_revision
                && current.claim_digest == commitment.claim.claim_digest
                && current.state == ComputeCapacityClaimState::Held => {}
        (Some(terminal), _)
            if current.revision == terminal.result_claim_revision
                && current.claim_digest == terminal.result_claim_digest
                && claim_state_name(current.state) == terminal.result_claim_state => {}
        _ => bail!("容量承诺 current Claim 与派生状态不一致"),
    }
    let current_status = terminal_receipt
        .as_ref()
        .map(|receipt| receipt.terminal_status.clone())
        .unwrap_or_else(|| {
            if allocation
                .as_ref()
                .is_some_and(|status| status.state == DeliveryAllocationCommitmentState::Exercised)
            {
                "allocated".to_string()
            } else {
                CAPACITY_COMMITMENT_STATUS_COMMITTED.to_string()
            }
        });
    Ok(ComputeCapacityCommitmentDetail {
        commitment,
        terminal_receipt,
        current_status,
        quantities,
    })
}

pub(super) fn due_commitment_ids_on(
    conn: &Connection,
    recorded_at: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT commitments.commitment_id
           FROM compute_capacity_commitments commitments
           LEFT JOIN compute_capacity_commitment_terminal_receipts terminal
             ON terminal.commitment_id=commitments.commitment_id
          WHERE terminal.commitment_id IS NULL
            AND NOT EXISTS (
                SELECT 1
                  FROM compute_delivery_allocation_grants allocation_grant
                  LEFT JOIN compute_delivery_allocation_terminal_receipts allocation_terminal
                    ON allocation_terminal.grant_id=allocation_grant.grant_id
                 WHERE allocation_grant.commitment_id=commitments.commitment_id
                   AND (allocation_terminal.grant_id IS NULL
                        OR allocation_terminal.terminal_status='exercised')
            )
            AND julianday(commitments.expires_at)<=julianday(?1)
          ORDER BY commitments.expires_at, commitments.commitment_id
          LIMIT ?2",
    )?;
    let commitment_ids = statement
        .query_map(params![recorded_at, limit as i64], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(commitment_ids)
}

fn audit_commitment_indexes_on(conn: &Connection, value: &ComputeCapacityCommitment) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_commitments WHERE
                commitment_id=?1 AND commitment_schema=?2 AND commitment_revision=?3
                AND commitment_status=?4 AND commitment_digest=?5
                AND owner_account_id=?6 AND provider_id=?7
                AND provider_policy_revision=?8 AND provider_digest=?9
                AND offer_id=?10 AND offer_version=?11 AND offer_digest=?12
                AND pool_id=?13 AND capacity_epoch=?14 AND pool_revision=?15 AND pool_digest=?16
                AND delivery_window_id=?17 AND delivery_window_digest=?18
                AND delivery_window_starts_at=?19 AND delivery_window_ends_at=?20
                AND price_snapshot_id=?21 AND price_snapshot_digest=?22
                AND reference_binding_id=?23 AND reference_binding_digest=?24
                AND instrument_id=?25 AND claim_id=?26 AND claim_revision=?27 AND claim_digest=?28
                AND hold_transaction_id=?29 AND hold_transaction_digest=?30
                AND hold_ledger_sequence=?31 AND hold_event_kind=?32
                AND idempotency_scope=?33 AND idempotency_key=?34 AND request_digest=?35
                AND created_at=?36 AND expires_at=?37
                AND canonicalization='rfc8785_jcs' AND digest_algorithm='sha256'",
            params![
                value.commitment_id,
                value.schema,
                value.commitment_revision,
                value.commitment_status,
                value.commitment_digest,
                value.owner_account_id,
                value.provider.provider_id,
                value.provider.policy_revision,
                value.provider.provider_digest,
                value.offer.offer_id,
                value.offer.offer_version,
                value.offer.offer_digest,
                value.pool.pool_id,
                value.pool.capacity_epoch,
                value.pool.pool_revision,
                value.pool.pool_digest,
                value.delivery_window.binding.window_id,
                value.delivery_window.binding.window_digest,
                value.delivery_window.starts_at_utc,
                value.delivery_window.ends_at_utc,
                value.price_snapshot_id,
                value.price_snapshot_digest,
                value.reference_binding.binding_id,
                value.reference_binding.binding_digest,
                value.instrument_id,
                value.claim.claim_id,
                value.claim.claim_revision,
                value.claim.claim_digest,
                value.creation_ledger.transaction_id,
                value.creation_ledger.transaction_digest,
                value.creation_ledger.ledger_sequence,
                value.creation_ledger.event_kind,
                value.idempotency_scope,
                value.idempotency_key,
                value.request_digest,
                value.created_at,
                value.expires_at,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("容量承诺 indexed columns 与 immutable JSON 不一致");
    }
    Ok(())
}

fn audit_claim_and_hold_on(
    conn: &Connection,
    value: &ComputeCapacityCommitment,
) -> Result<Vec<ComputeCapacityCommitmentQuantity>> {
    let claim = stored_claim_version_on(conn, &value.claim.claim_id, value.claim.claim_revision)?
        .ok_or_else(|| anyhow!("容量承诺 Claim revision 1 缺失"))?;
    if claim.claim_digest != value.claim.claim_digest
        || claim.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || claim.state != ComputeCapacityClaimState::Held
        || claim.subject_kind != "compute_capacity_commitment"
        || claim.subject_id != value.commitment_id
        || claim.parent_claim_id.is_some()
        || claim.pool != value.pool
        || claim.delivery_window != value.delivery_window.binding
        || claim.created_at != value.created_at
        || claim.expires_at.as_deref() != Some(value.expires_at.as_str())
    {
        bail!("容量承诺 Claim revision 1 绑定审计失败");
    }
    let held = held_claim_causal_transaction_on(conn, &claim.claim_id, &claim.idempotency_key)?
        .ok_or_else(|| anyhow!("容量承诺原始 held ledger 缺失"))?;
    if held.transaction_id != value.creation_ledger.transaction_id
        || held.causal_binding.offer.as_ref() != Some(&value.offer)
        || held.causal_binding.job_id.is_some()
        || held.causal_binding.reservation_id.is_some()
        || held.causal_binding.attempt_lease_id.is_some()
        || held.causal_binding.fencing_generation.is_some()
        || held.pool_id != value.pool.pool_id
        || held.capacity_epoch != value.pool.capacity_epoch
        || held.delivery_window_id != value.delivery_window.binding.window_id
        || held.subject_kind != "compute_capacity_commitment"
        || held.subject_id != value.commitment_id
    {
        bail!("容量承诺原始 held causal binding 审计失败");
    }
    audit_ledger_binding_on(conn, &value.creation_ledger, &claim.claim_id)?;
    let mut quantities = claim
        .lines
        .into_iter()
        .map(|line| ComputeCapacityCommitmentQuantity {
            meter: line.bucket.meter,
            quantity_units: line.quantity_units,
        })
        .collect::<Vec<_>>();
    quantities.sort_by(|left, right| left.meter.cmp(&right.meter));
    Ok(quantities)
}

fn audit_terminal_indexes_on(
    conn: &Connection,
    value: &ComputeCapacityCommitmentTerminalReceipt,
) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_commitment_terminal_receipts WHERE
                terminal_receipt_id=?1 AND terminal_schema=?2 AND terminal_revision=?3
                AND terminal_status=?4 AND terminal_receipt_digest=?5
                AND commitment_id=?6 AND commitment_revision=1 AND commitment_digest=?7
                AND claim_id=?8 AND prior_claim_revision=?9 AND prior_claim_digest=?10
                AND result_claim_revision=?11 AND result_claim_digest=?12
                AND result_claim_state=?13 AND terminal_transaction_id=?14
                AND terminal_transaction_digest=?15 AND terminal_ledger_sequence=?16
                AND terminal_event_kind=?17 AND causal_transaction_id=?18
                AND actor_kind=?19 AND actor_id=?20 AND reason IS ?21
                AND idempotency_scope=?22 AND idempotency_key=?23 AND request_digest=?24
                AND occurred_at=?25 AND recorded_at=?26
                AND canonicalization='rfc8785_jcs' AND digest_algorithm='sha256'",
            params![
                value.terminal_receipt_id,
                value.schema,
                value.terminal_revision,
                value.terminal_status,
                value.terminal_receipt_digest,
                value.commitment_id,
                value.commitment_digest,
                value.claim_id,
                value.prior_claim_revision,
                value.prior_claim_digest,
                value.result_claim_revision,
                value.result_claim_digest,
                value.result_claim_state,
                value.ledger.transaction_id,
                value.ledger.transaction_digest,
                value.ledger.ledger_sequence,
                value.ledger.event_kind,
                value.ledger.causal_transaction_id,
                value.actor_kind,
                value.actor_id,
                value.reason,
                value.idempotency_scope,
                value.idempotency_key,
                value.request_digest,
                value.occurred_at,
                value.recorded_at,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("容量承诺 terminal indexed columns 与 immutable JSON 不一致");
    }
    Ok(())
}

fn audit_terminal_claim_and_ledger_on(
    conn: &Connection,
    commitment: &ComputeCapacityCommitment,
    receipt: &ComputeCapacityCommitmentTerminalReceipt,
) -> Result<()> {
    let _ = audit_claim_and_hold_on(conn, commitment)?;
    let claim = stored_claim_version_on(conn, &receipt.claim_id, receipt.result_claim_revision)?
        .ok_or_else(|| anyhow!("容量承诺 terminal Claim revision 2 缺失"))?;
    let (expected_state, expected_event_kind, expected_actor_kind) =
        match receipt.terminal_status.as_str() {
            CAPACITY_COMMITMENT_STATUS_CANCELED => (
                ComputeCapacityClaimState::Released,
                "reservation_released",
                "provider_owner",
            ),
            CAPACITY_COMMITMENT_STATUS_EXPIRED => (
                ComputeCapacityClaimState::Expired,
                "reservation_expired",
                "platform_admin",
            ),
            _ => bail!("容量承诺 terminal status 不受支持"),
        };
    if receipt.claim_id != commitment.claim.claim_id
        || commitment.claim.claim_revision != 1
        || receipt.prior_claim_revision != 1
        || receipt.prior_claim_revision != commitment.claim.claim_revision
        || receipt.prior_claim_digest != commitment.claim.claim_digest
        || receipt.terminal_revision != 2
        || receipt.result_claim_revision != 2
        || claim.claim_digest != receipt.result_claim_digest
        || claim.state != expected_state
        || receipt.result_claim_state != claim_state_name(expected_state)
        || receipt.ledger.event_kind != expected_event_kind
        || receipt.actor_kind != expected_actor_kind
        || receipt.actor_id.trim().is_empty()
        || receipt.actor_id != receipt.actor_id.trim()
        || claim.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || claim.subject_kind != "compute_capacity_commitment"
        || claim.subject_id != commitment.commitment_id
        || claim.parent_claim_id.is_some()
        || claim.pool != commitment.pool
        || claim.delivery_window != commitment.delivery_window.binding
        || claim.expires_at.as_deref() != Some(commitment.expires_at.as_str())
    {
        bail!("容量承诺 terminal Claim 绑定审计失败");
    }
    audit_ledger_binding_on(conn, &receipt.ledger, &receipt.claim_id)?;
    let causal = capacity_causal_transaction_on(conn, &receipt.ledger.transaction_id)?;
    if causal.causal_binding.offer.as_ref() != Some(&commitment.offer)
        || causal.causal_binding.job_id.is_some()
        || causal.causal_binding.reservation_id.is_some()
        || causal.causal_binding.attempt_lease_id.is_some()
        || causal.causal_binding.fencing_generation.is_some()
        || causal.pool_id != commitment.pool.pool_id
        || causal.capacity_epoch != commitment.pool.capacity_epoch
        || causal.delivery_window_id != commitment.delivery_window.binding.window_id
        || causal.subject_kind != "compute_capacity_commitment"
        || causal.subject_id != commitment.commitment_id
        || receipt.ledger.causal_transaction_id.as_deref()
            != Some(commitment.creation_ledger.transaction_id.as_str())
    {
        bail!("容量承诺 terminal ledger causal binding 审计失败");
    }
    Ok(())
}
