use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::compute_federation::{
    capacity::{ComputeCapacityClaimKind, ComputeCapacityClaimState},
    delivery_allocation::{
        ComputeDeliveryAllocationExerciseEvidence, ComputeDeliveryAllocationLedgerEvidence,
        ComputeDeliveryAllocationReservationClaimEvidence,
        ComputeDeliveryAllocationReservationEvidence, ComputeDeliveryAllocationTerminalReceipt,
        COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA, DELIVERY_ALLOCATION_ACTOR_CONSUMER,
        DELIVERY_ALLOCATION_STATUS_EXERCISED,
    },
};

use super::{
    super::{
        compute_broker_reservation::{
            broker_reserve_binding_on, prepare_delivery_allocation_broker_budget_on,
            reserve_compute_job_with_preheld_claim_on, ReserveComputeBrokerRequest,
        },
        compute_capacity_claim_rows::stored_claim_on,
        compute_capacity_claim_transitions::{
            finish_compute_capacity_commitment_claim_on, ComputeCapacityClaimTerminalAction,
            FinishComputeCapacityClaim,
        },
        compute_capacity_claims::hold_parented_delivery_reservation_claim_on,
        new_id, now, Store,
    },
    canonical::exercise_request_digest,
    read::{grant_by_id_on, terminal_by_grant_on, terminal_by_idempotency_on},
    terminal::persist_terminal_on,
    types::{
        ComputeDeliveryAllocationExerciseWriteReceipt, DeliveryAllocationClaimTransferAuthority,
        DeliveryAllocationReservationAuthority, ExerciseComputeDeliveryAllocationGrant,
    },
    validation::{validate_exercise_input, validate_exercise_source_on},
};

impl Store {
    pub(crate) fn exercise_compute_delivery_allocation_grant(
        &self,
        input: ExerciseComputeDeliveryAllocationGrant,
    ) -> Result<ComputeDeliveryAllocationExerciseWriteReceipt> {
        validate_exercise_input(&input)?;
        let request_digest = exercise_request_digest(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some((grant, terminal)) = terminal_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            if terminal.request_digest != request_digest
                || terminal.terminal_status != DELIVERY_ALLOCATION_STATUS_EXERCISED
            {
                bail!("相同 DeliveryAllocation Exercise 幂等键不能用于不同请求");
            }
            transaction.commit()?;
            return Ok(ComputeDeliveryAllocationExerciseWriteReceipt {
                grant,
                terminal_receipt: terminal,
                replayed: true,
            });
        }

        let grant = grant_by_id_on(&transaction, &input.grant_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation Grant 不存在"))?;
        if grant.consumer_account_id != input.consumer_account_id {
            bail!("DeliveryAllocation Grant 不属于当前 consumer");
        }
        if grant.grant_revision != input.expected_grant_revision
            || grant.grant_digest != input.expected_grant_digest
        {
            bail!("DeliveryAllocation expected Grant revision/digest 已变化");
        }
        if terminal_by_grant_on(&transaction, &grant)?.is_some() {
            bail!("DeliveryAllocation Grant 已有终态，不能用新幂等键覆盖");
        }

        let exercise_occurred_at = now();
        let source = validate_exercise_source_on(&transaction, &grant, &exercise_occurred_at)?;
        let reservation_expires_at = source.source_job.job.workload.deadline_at.clone();
        let authority = DeliveryAllocationClaimTransferAuthority::new(
            grant.clone(),
            source.commitment.commitment,
            source.parent_claim,
            source.source_job,
            input.reservation_id.clone(),
            input.idempotency_key.clone(),
            reservation_expires_at.clone(),
            exercise_occurred_at.clone(),
        );
        let broker_request = ReserveComputeBrokerRequest {
            reservation_id: input.reservation_id,
            consumer_account_id: input.consumer_account_id.clone(),
            idempotency_key: input.idempotency_key.clone(),
            job_id: authority.job_id().to_string(),
            expected_job_revision: authority.source_job().revision,
            expected_job_digest: authority.source_job().job_digest.clone(),
            reserved_capacity: authority.reserved_capacity(),
            expires_at: reservation_expires_at,
        };
        let prepared = prepare_delivery_allocation_broker_budget_on(
            &transaction,
            &broker_request,
            &authority,
        )?;

        let parent_release = finish_compute_capacity_commitment_claim_on(
            &transaction,
            FinishComputeCapacityClaim {
                claim_id: authority.parent_claim().claim_id.clone(),
                expected_revision: authority.parent_claim().revision,
                action: ComputeCapacityClaimTerminalAction::Release,
                idempotency_scope: format!(
                    "delivery-allocation:parent-release:{}",
                    authority.grant().grant_id
                ),
                idempotency_key: input.idempotency_key.clone(),
                occurred_at: exercise_occurred_at,
            },
            authority.offer_binding().clone(),
            &authority.commitment().commitment_id,
        )?;
        if parent_release.replayed
            || parent_release.revision != 2
            || parent_release.state != "released"
            || parent_release.ledger.replayed
        {
            bail!("DeliveryAllocation parent release 返回意外重放或状态");
        }
        let child_hold =
            hold_parented_delivery_reservation_claim_on(&transaction, &authority, &parent_release)?;
        let parent_result = stored_claim_on(&transaction, &parent_release.claim_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation released parent Claim 无法读取"))?;
        let child_claim = stored_claim_on(&transaction, &child_hold.claim_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation held child Claim 无法读取"))?;
        ensure_fresh_transfer_result(&authority, &parent_result, &child_claim)?;
        let parent_release_ledger =
            ledger_evidence_on(&transaction, &parent_release.ledger.transaction_id)?;
        let child_hold_ledger =
            ledger_evidence_on(&transaction, &child_hold.ledger.transaction_id)?;
        let reservation_authority = DeliveryAllocationReservationAuthority::new(
            authority,
            parent_result,
            parent_release_ledger.clone(),
            child_claim,
            child_hold_ledger.clone(),
        );
        let broker = reserve_compute_job_with_preheld_claim_on(
            &transaction,
            prepared,
            &reservation_authority,
        )?;
        let broker_binding = broker_reserve_binding_on(
            &transaction,
            &broker.reservation_id,
            &input.consumer_account_id,
        )?;
        let terminal_recorded_at = now();
        let child = reservation_authority.child_claim();
        let parent = reservation_authority.parent_claim();
        let parent_result = reservation_authority.parent_result_claim();
        let mut terminal = ComputeDeliveryAllocationTerminalReceipt {
            schema: COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA.to_string(),
            terminal_receipt_id: new_id("compute_delivery_allocation_terminal"),
            terminal_revision: 2,
            terminal_receipt_digest: String::new(),
            terminal_status: DELIVERY_ALLOCATION_STATUS_EXERCISED.to_string(),
            grant_id: grant.grant_id.clone(),
            grant_digest: grant.grant_digest.clone(),
            commitment: grant.commitment.clone(),
            actor_kind: DELIVERY_ALLOCATION_ACTOR_CONSUMER.to_string(),
            actor_id: input.consumer_account_id,
            exercise: Some(ComputeDeliveryAllocationExerciseEvidence {
                parent_claim_id: parent.claim_id.clone(),
                parent_prior_claim_revision: parent.revision,
                parent_prior_claim_digest: parent.claim_digest.clone(),
                parent_result_claim_revision: parent_result.revision,
                parent_result_claim_digest: parent_result.claim_digest.clone(),
                parent_result_claim_state: "released".to_string(),
                parent_release_ledger,
                reservation_claim: ComputeDeliveryAllocationReservationClaimEvidence {
                    claim_id: child.claim_id.clone(),
                    claim_revision: child.revision,
                    claim_digest: child.claim_digest.clone(),
                    parent_claim_id: parent.claim_id.clone(),
                },
                reservation_hold_ledger: child_hold_ledger,
                reservation: ComputeDeliveryAllocationReservationEvidence {
                    reservation_id: broker.reservation_id,
                    reservation_revision: broker.reservation_revision,
                    reservation_digest: broker.reservation_digest,
                },
                source_job_revision: broker_binding.source_job.job_revision,
                source_job_digest: broker_binding.source_job.job_digest,
                reserved_job_revision: broker_binding.reserved_job.job_revision,
                reserved_job_digest: broker_binding.reserved_job.job_digest,
                budget_reservation_id: broker_binding.budget_reservation_id,
                reserved_amount_fen: broker_binding.budget_reserved_fen,
                broker_reserve_request_digest: broker_binding.request_digest,
            }),
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            request_digest,
            occurred_at: terminal_recorded_at.clone(),
            recorded_at: terminal_recorded_at,
        };
        terminal = persist_terminal_on(&transaction, &grant, terminal)?;
        transaction.commit()?;
        Ok(ComputeDeliveryAllocationExerciseWriteReceipt {
            grant,
            terminal_receipt: terminal,
            replayed: false,
        })
    }
}

fn ensure_fresh_transfer_result(
    authority: &DeliveryAllocationClaimTransferAuthority,
    parent_result: &crate::compute_federation::capacity::ComputeCapacityClaim,
    child: &crate::compute_federation::capacity::ComputeCapacityClaim,
) -> Result<()> {
    let parent = authority.parent_claim();
    if parent_result.claim_id != parent.claim_id
        || parent_result.revision != 2
        || parent_result.state != ComputeCapacityClaimState::Released
        || parent_result.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || child.revision != 1
        || child.state != ComputeCapacityClaimState::Held
        || child.claim_kind != ComputeCapacityClaimKind::Reservation
        || child.subject_kind != "compute_reservation"
        || child.subject_id != authority.reservation_id()
        || child.parent_claim_id.as_deref() != Some(parent.claim_id.as_str())
        || child.pool != parent.pool
        || child.delivery_window != parent.delivery_window
        || child.lines != parent.lines
    {
        bail!("DeliveryAllocation parent release→child hold 不是 whole-only exact transfer");
    }
    Ok(())
}

fn ledger_evidence_on(
    transaction: &Transaction<'_>,
    transaction_id: &str,
) -> Result<ComputeDeliveryAllocationLedgerEvidence> {
    transaction
        .query_row(
            "SELECT transaction_digest, ledger_sequence, event_kind, causal_transaction_id
               FROM compute_capacity_ledger_transactions WHERE transaction_id=?1",
            params![transaction_id],
            |row| {
                Ok(ComputeDeliveryAllocationLedgerEvidence {
                    transaction_id: transaction_id.to_string(),
                    transaction_digest: row.get(0)?,
                    ledger_sequence: row.get(1)?,
                    event_kind: row.get(2)?,
                    causal_transaction_id: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("DeliveryAllocation ledger transaction 无法读取"))
}
