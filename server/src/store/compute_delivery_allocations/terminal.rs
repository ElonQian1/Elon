use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::compute_federation::delivery_allocation::{
    ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
    COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA, DELIVERY_ALLOCATION_ACTOR_ADMIN,
    DELIVERY_ALLOCATION_ACTOR_CONSUMER, DELIVERY_ALLOCATION_STATUS_DECLINED,
    DELIVERY_ALLOCATION_STATUS_EXPIRED,
};

use super::{
    super::{new_id, now, Store},
    canonical::{
        canonical_terminal_json_and_digest, decline_request_digest, expire_idempotency_key,
        expire_request_digest,
    },
    read::{due_grant_ids_on, grant_by_id_on, terminal_by_grant_on, terminal_by_idempotency_on},
    types::{
        ComputeDeliveryAllocationExpiryItem, ComputeDeliveryAllocationExpiryReport,
        ComputeDeliveryAllocationTerminalWriteReceipt, DeclineComputeDeliveryAllocationGrant,
        ExpireDueComputeDeliveryAllocationGrants,
    },
    validation::{
        parse_utc, validate_decline_input, validate_expire_input, validate_nonexercise_source_on,
    },
};

const EXPIRE_IDEMPOTENCY_SCOPE: &str = "delivery-allocation:expire-due";

impl Store {
    pub(crate) fn decline_compute_delivery_allocation_grant(
        &self,
        input: DeclineComputeDeliveryAllocationGrant,
    ) -> Result<ComputeDeliveryAllocationTerminalWriteReceipt> {
        validate_decline_input(&input)?;
        let request_digest = decline_request_digest(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some((grant, terminal)) = terminal_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            if terminal.request_digest != request_digest
                || terminal.terminal_status != DELIVERY_ALLOCATION_STATUS_DECLINED
            {
                bail!("相同 DeliveryAllocation Decline 幂等键不能用于不同请求");
            }
            transaction.commit()?;
            return Ok(ComputeDeliveryAllocationTerminalWriteReceipt {
                grant,
                terminal_receipt: terminal,
                replayed: true,
            });
        }
        let grant = grant_by_id_on(&transaction, &input.grant_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation Grant 不存在"))?;
        validate_expected_consumer_grant(
            &grant,
            &input.consumer_account_id,
            input.expected_grant_revision,
            &input.expected_grant_digest,
        )?;
        if terminal_by_grant_on(&transaction, &grant)?.is_some() {
            bail!("DeliveryAllocation Grant 已有终态，不能用新幂等键覆盖");
        }
        validate_nonexercise_source_on(&transaction, &grant)?;
        let recorded_at = now();
        if parse_utc("DeliveryAllocation Decline time", &recorded_at)?
            >= parse_utc(
                "DeliveryAllocation exercise expiry",
                &grant.exercise_expires_at,
            )?
        {
            bail!("DeliveryAllocation Grant 已过消费者 Decline 截止时间");
        }
        let terminal = new_terminal(
            &grant,
            DELIVERY_ALLOCATION_STATUS_DECLINED,
            DELIVERY_ALLOCATION_ACTOR_CONSUMER,
            &input.consumer_account_id,
            &input.idempotency_scope,
            &input.idempotency_key,
            &request_digest,
            &recorded_at,
            &recorded_at,
        );
        let terminal = persist_terminal_on(&transaction, &grant, terminal)?;
        transaction.commit()?;
        Ok(ComputeDeliveryAllocationTerminalWriteReceipt {
            grant,
            terminal_receipt: terminal,
            replayed: false,
        })
    }

    pub(crate) fn expire_due_compute_delivery_allocation_grants(
        &self,
        input: ExpireDueComputeDeliveryAllocationGrants,
    ) -> Result<ComputeDeliveryAllocationExpiryReport> {
        validate_expire_input(&input)?;
        let recovery_started_at = now();
        let ids = {
            let connection = self.conn()?;
            due_grant_ids_on(&connection, &recovery_started_at, input.limit)?
        };
        let mut items = Vec::with_capacity(ids.len());
        let mut expired_count = 0;
        let mut replayed_count = 0;
        let mut failed_count = 0;
        for grant_id in ids {
            match self.expire_one_delivery_allocation_grant(&grant_id, &input.admin_user_id) {
                Ok(receipt) => {
                    expired_count += 1;
                    if receipt.replayed {
                        replayed_count += 1;
                    }
                    items.push(ComputeDeliveryAllocationExpiryItem {
                        grant_id,
                        status: receipt.terminal_receipt.terminal_status.clone(),
                        replayed: receipt.replayed,
                        terminal_receipt: Some(receipt.terminal_receipt),
                        error: None,
                    });
                }
                Err(error) => {
                    failed_count += 1;
                    items.push(ComputeDeliveryAllocationExpiryItem {
                        grant_id,
                        status: "failed".to_string(),
                        replayed: false,
                        terminal_receipt: None,
                        error: Some(error.to_string()),
                    });
                }
            }
        }
        Ok(ComputeDeliveryAllocationExpiryReport {
            recovery_started_at,
            selected_count: items.len(),
            expired_count,
            replayed_count,
            failed_count,
            items,
        })
    }

    fn expire_one_delivery_allocation_grant(
        &self,
        grant_id: &str,
        admin_user_id: &str,
    ) -> Result<ComputeDeliveryAllocationTerminalWriteReceipt> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let grant = grant_by_id_on(&transaction, grant_id)?
            .ok_or_else(|| anyhow!("到期 DeliveryAllocation Grant 不存在"))?;
        let idempotency_key = expire_idempotency_key(&grant)?;
        let request_digest = expire_request_digest(&grant)?;
        if let Some((stored, terminal)) =
            terminal_by_idempotency_on(&transaction, EXPIRE_IDEMPOTENCY_SCOPE, &idempotency_key)?
        {
            if stored.grant_digest != grant.grant_digest
                || terminal.request_digest != request_digest
                || terminal.terminal_status != DELIVERY_ALLOCATION_STATUS_EXPIRED
            {
                bail!("DeliveryAllocation Expire deterministic 幂等键冲突");
            }
            transaction.commit()?;
            return Ok(ComputeDeliveryAllocationTerminalWriteReceipt {
                grant: stored,
                terminal_receipt: terminal,
                replayed: true,
            });
        }
        if terminal_by_grant_on(&transaction, &grant)?.is_some() {
            bail!("DeliveryAllocation Grant 已被 Exercise/Decline/Expire 竞争者终结");
        }
        validate_nonexercise_source_on(&transaction, &grant)?;
        let recorded_at = now();
        if parse_utc("DeliveryAllocation Expire Store time", &recorded_at)?
            < parse_utc(
                "DeliveryAllocation exercise expiry",
                &grant.exercise_expires_at,
            )?
        {
            bail!("DeliveryAllocation Grant 尚未到期");
        }
        let terminal = new_terminal(
            &grant,
            DELIVERY_ALLOCATION_STATUS_EXPIRED,
            DELIVERY_ALLOCATION_ACTOR_ADMIN,
            admin_user_id,
            EXPIRE_IDEMPOTENCY_SCOPE,
            &idempotency_key,
            &request_digest,
            &grant.exercise_expires_at,
            &recorded_at,
        );
        let terminal = persist_terminal_on(&transaction, &grant, terminal)?;
        transaction.commit()?;
        Ok(ComputeDeliveryAllocationTerminalWriteReceipt {
            grant,
            terminal_receipt: terminal,
            replayed: false,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn new_terminal(
    grant: &ComputeDeliveryAllocationGrant,
    terminal_status: &str,
    actor_kind: &str,
    actor_id: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
    request_digest: &str,
    occurred_at: &str,
    recorded_at: &str,
) -> ComputeDeliveryAllocationTerminalReceipt {
    ComputeDeliveryAllocationTerminalReceipt {
        schema: COMPUTE_DELIVERY_ALLOCATION_TERMINAL_RECEIPT_SCHEMA.to_string(),
        terminal_receipt_id: new_id("compute_delivery_allocation_terminal"),
        terminal_revision: 2,
        terminal_receipt_digest: String::new(),
        terminal_status: terminal_status.to_string(),
        grant_id: grant.grant_id.clone(),
        grant_digest: grant.grant_digest.clone(),
        commitment: grant.commitment.clone(),
        actor_kind: actor_kind.to_string(),
        actor_id: actor_id.to_string(),
        exercise: None,
        idempotency_scope: idempotency_scope.to_string(),
        idempotency_key: idempotency_key.to_string(),
        request_digest: request_digest.to_string(),
        occurred_at: occurred_at.to_string(),
        recorded_at: recorded_at.to_string(),
    }
}

pub(super) fn persist_terminal_on(
    transaction: &Transaction<'_>,
    grant: &ComputeDeliveryAllocationGrant,
    mut value: ComputeDeliveryAllocationTerminalReceipt,
) -> Result<ComputeDeliveryAllocationTerminalReceipt> {
    let (_, digest) = canonical_terminal_json_and_digest(&value)?;
    value.terminal_receipt_digest = digest;
    let (json, verified_digest) = canonical_terminal_json_and_digest(&value)?;
    if verified_digest != value.terminal_receipt_digest {
        bail!("DeliveryAllocation terminal canonical digest 不稳定");
    }
    insert_terminal_on(transaction, &value, &json)?;
    let stored = terminal_by_grant_on(transaction, grant)?
        .ok_or_else(|| anyhow!("DeliveryAllocation terminal 插入后无法 exact readback"))?;
    if stored != value {
        bail!("DeliveryAllocation terminal immutable readback 与候选不一致");
    }
    Ok(stored)
}

fn insert_terminal_on(
    transaction: &Transaction<'_>,
    value: &ComputeDeliveryAllocationTerminalReceipt,
    json: &str,
) -> Result<()> {
    let exercise = value.exercise.as_ref();
    let changed = transaction.execute(
        "INSERT INTO compute_delivery_allocation_terminal_receipts (
            terminal_receipt_id, terminal_schema, terminal_revision, terminal_status,
            terminal_receipt_digest, terminal_receipt_json, canonicalization, digest_algorithm,
            grant_id, grant_digest, commitment_id, commitment_revision, commitment_digest,
            actor_kind, actor_id, idempotency_scope, idempotency_key, request_digest,
            occurred_at, recorded_at, parent_claim_id, parent_prior_claim_revision,
            parent_prior_claim_digest, parent_result_claim_revision, parent_result_claim_digest,
            parent_result_claim_state, parent_release_transaction_id,
            parent_release_transaction_digest, parent_release_ledger_sequence,
            parent_release_event_kind, parent_release_causal_transaction_id,
            reservation_claim_id, reservation_claim_revision, reservation_claim_digest,
            reservation_parent_claim_id, reservation_hold_transaction_id,
            reservation_hold_transaction_digest, reservation_hold_ledger_sequence,
            reservation_hold_event_kind, reservation_hold_causal_transaction_id,
            reservation_id, reservation_revision, reservation_digest, source_job_revision,
            source_job_digest, reserved_job_revision, reserved_job_digest,
            budget_reservation_id, reserved_amount_fen, broker_reserve_request_digest
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,'rfc8785_jcs','sha256',?7,?8,?9,?10,?11,?12,?13,
            ?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,
            ?30,?31,?32,?33,?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,
            ?46,?47,?48
         )",
        params![
            value.terminal_receipt_id,
            value.schema,
            value.terminal_revision,
            value.terminal_status,
            value.terminal_receipt_digest,
            json,
            value.grant_id,
            value.grant_digest,
            value.commitment.commitment_id,
            value.commitment.commitment_revision,
            value.commitment.commitment_digest,
            value.actor_kind,
            value.actor_id,
            value.idempotency_scope,
            value.idempotency_key,
            value.request_digest,
            value.occurred_at,
            value.recorded_at,
            exercise.map(|x| x.parent_claim_id.as_str()),
            exercise.map(|x| x.parent_prior_claim_revision),
            exercise.map(|x| x.parent_prior_claim_digest.as_str()),
            exercise.map(|x| x.parent_result_claim_revision),
            exercise.map(|x| x.parent_result_claim_digest.as_str()),
            exercise.map(|x| x.parent_result_claim_state.as_str()),
            exercise.map(|x| x.parent_release_ledger.transaction_id.as_str()),
            exercise.map(|x| x.parent_release_ledger.transaction_digest.as_str()),
            exercise.map(|x| x.parent_release_ledger.ledger_sequence),
            exercise.map(|x| x.parent_release_ledger.event_kind.as_str()),
            exercise.map(|x| x.parent_release_ledger.causal_transaction_id.as_str()),
            exercise.map(|x| x.reservation_claim.claim_id.as_str()),
            exercise.map(|x| x.reservation_claim.claim_revision),
            exercise.map(|x| x.reservation_claim.claim_digest.as_str()),
            exercise.map(|x| x.reservation_claim.parent_claim_id.as_str()),
            exercise.map(|x| x.reservation_hold_ledger.transaction_id.as_str()),
            exercise.map(|x| x.reservation_hold_ledger.transaction_digest.as_str()),
            exercise.map(|x| x.reservation_hold_ledger.ledger_sequence),
            exercise.map(|x| x.reservation_hold_ledger.event_kind.as_str()),
            exercise.map(|x| x.reservation_hold_ledger.causal_transaction_id.as_str()),
            exercise.map(|x| x.reservation.reservation_id.as_str()),
            exercise.map(|x| x.reservation.reservation_revision),
            exercise.map(|x| x.reservation.reservation_digest.as_str()),
            exercise.map(|x| x.source_job_revision),
            exercise.map(|x| x.source_job_digest.as_str()),
            exercise.map(|x| x.reserved_job_revision),
            exercise.map(|x| x.reserved_job_digest.as_str()),
            exercise.map(|x| x.budget_reservation_id.as_str()),
            exercise.map(|x| x.reserved_amount_fen),
            exercise.map(|x| x.broker_reserve_request_digest.as_str()),
        ],
    )?;
    if changed != 1 {
        bail!("DeliveryAllocation terminal immutable insert 数量异常");
    }
    Ok(())
}

fn validate_expected_consumer_grant(
    grant: &ComputeDeliveryAllocationGrant,
    consumer_account_id: &str,
    expected_revision: i64,
    expected_digest: &str,
) -> Result<()> {
    if grant.consumer_account_id != consumer_account_id {
        bail!("DeliveryAllocation Grant 不属于当前 consumer");
    }
    if grant.grant_revision != expected_revision || grant.grant_digest != expected_digest {
        bail!("DeliveryAllocation expected Grant revision/digest 已变化");
    }
    Ok(())
}
