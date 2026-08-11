use anyhow::{anyhow, bail, Result};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use crate::compute_federation::capacity_commitment::{
    ComputeCapacityCommitment, ComputeCapacityCommitmentLedgerBinding,
    ComputeCapacityCommitmentTerminalReceipt, CAPACITY_COMMITMENT_STATUS_CANCELED,
    CAPACITY_COMMITMENT_STATUS_EXPIRED, COMPUTE_CAPACITY_COMMITMENT_TERMINAL_RECEIPT_SCHEMA,
};

use super::{
    super::{
        compute_capacity_claim_transitions::{
            finish_compute_capacity_commitment_claim_on, ComputeCapacityClaimTerminalAction,
            FinishComputeCapacityClaim,
        },
        compute_delivery_allocations::delivery_allocation_commitment_status_on,
        new_id, now, Store,
    },
    canonical::{
        cancel_request_digest, canonical_terminal_json_and_digest, expire_idempotency_key,
        expire_request_digest, normalized_reason,
    },
    read::{
        commitment_by_id_on, create_receipt_on, due_commitment_ids_on, terminal_by_commitment_on,
        terminal_by_idempotency_on,
    },
    types::{
        CancelComputeCapacityCommitment, ComputeCapacityCommitmentExpiryItem,
        ComputeCapacityCommitmentExpiryReport, ComputeCapacityCommitmentTerminalWriteReceipt,
        ExpireDueComputeCapacityCommitments,
    },
    validation::{offer_binding, parse_utc, validate_cancel_input, validate_expire_input},
};

const EXPIRE_IDEMPOTENCY_SCOPE: &str = "capacity-commitment:expire-due";

impl Store {
    pub(crate) fn cancel_compute_capacity_commitment(
        &self,
        input: CancelComputeCapacityCommitment,
    ) -> Result<ComputeCapacityCommitmentTerminalWriteReceipt> {
        validate_cancel_input(&input)?;
        let request_digest = cancel_request_digest(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some((commitment, terminal)) = terminal_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            if terminal.request_digest != request_digest
                || terminal.terminal_status != CAPACITY_COMMITMENT_STATUS_CANCELED
            {
                bail!("相同容量承诺 Cancel 幂等键不能用于不同请求");
            }
            let quantities = create_receipt_on(&transaction, commitment.clone(), true)?.quantities;
            transaction.commit()?;
            return Ok(ComputeCapacityCommitmentTerminalWriteReceipt {
                commitment,
                terminal_receipt: terminal,
                quantities,
                replayed: true,
            });
        }

        let commitment = commitment_by_id_on(&transaction, &input.commitment_id)?
            .ok_or_else(|| anyhow!("容量承诺不存在"))?;
        if commitment.owner_account_id != input.owner_account_id
            || commitment.provider.provider_id != input.provider_id
            || commitment.pool.pool_id != input.pool_id
        {
            bail!("容量承诺不属于当前 owner/provider/pool");
        }
        if commitment.commitment_revision != input.expected_commitment_revision
            || commitment.commitment_digest != input.expected_commitment_digest
        {
            bail!("容量承诺 expected revision/digest 已变化");
        }
        if terminal_by_commitment_on(&transaction, &commitment)?.is_some() {
            bail!("容量承诺已有终态；新幂等键不能覆盖或重开");
        }
        if delivery_allocation_commitment_status_on(
            &transaction,
            &commitment.commitment_id,
            &commitment.commitment_digest,
        )?
        .is_some_and(|status| status.blocks_commitment_terminal())
        {
            bail!("active/exercised Delivery Allocation 阻止容量承诺 Cancel");
        }
        let cancel_at = now();
        if parse_utc("capacity commitment cancel time", &cancel_at)?
            >= parse_utc(
                "capacity commitment delivery window start",
                &commitment.delivery_window.starts_at_utc,
            )?
        {
            bail!("容量承诺只能在交付窗口开始前取消");
        }
        let receipt = terminalize_on(
            &transaction,
            &commitment,
            ComputeCapacityClaimTerminalAction::Release,
            CAPACITY_COMMITMENT_STATUS_CANCELED,
            "provider_owner",
            &input.owner_account_id,
            normalized_reason(&input.reason),
            &input.idempotency_scope,
            &input.idempotency_key,
            &request_digest,
            &cancel_at,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }

    pub(crate) fn expire_due_compute_capacity_commitments(
        &self,
        input: ExpireDueComputeCapacityCommitments,
    ) -> Result<ComputeCapacityCommitmentExpiryReport> {
        validate_expire_input(&input)?;
        let recovery_started_at = now();
        let ids = {
            let connection = self.conn()?;
            due_commitment_ids_on(&connection, &recovery_started_at, input.limit)?
        };
        let mut items = Vec::with_capacity(ids.len());
        let mut expired_count = 0_usize;
        let mut replayed_count = 0_usize;
        let mut failed_count = 0_usize;
        for commitment_id in ids {
            match self.expire_one_capacity_commitment(&commitment_id, &input.admin_user_id) {
                Ok(receipt) => {
                    expired_count += 1;
                    if receipt.replayed {
                        replayed_count += 1;
                    }
                    items.push(ComputeCapacityCommitmentExpiryItem {
                        commitment_id,
                        status: receipt.terminal_receipt.terminal_status.clone(),
                        replayed: receipt.replayed,
                        terminal_receipt: Some(receipt.terminal_receipt),
                        error: None,
                    });
                }
                Err(error) => {
                    failed_count += 1;
                    items.push(ComputeCapacityCommitmentExpiryItem {
                        commitment_id,
                        status: "failed".to_string(),
                        replayed: false,
                        terminal_receipt: None,
                        error: Some(error.to_string()),
                    });
                }
            }
        }
        Ok(ComputeCapacityCommitmentExpiryReport {
            recovery_started_at,
            selected_count: items.len(),
            expired_count,
            replayed_count,
            failed_count,
            items,
        })
    }

    fn expire_one_capacity_commitment(
        &self,
        commitment_id: &str,
        admin_user_id: &str,
    ) -> Result<ComputeCapacityCommitmentTerminalWriteReceipt> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let commitment = commitment_by_id_on(&transaction, commitment_id)?
            .ok_or_else(|| anyhow!("到期容量承诺不存在"))?;
        let key = expire_idempotency_key(&commitment)?;
        let request_digest = expire_request_digest(&commitment)?;
        if let Some((stored, terminal)) =
            terminal_by_idempotency_on(&transaction, EXPIRE_IDEMPOTENCY_SCOPE, &key)?
        {
            if stored.commitment_digest != commitment.commitment_digest
                || terminal.request_digest != request_digest
                || terminal.terminal_status != CAPACITY_COMMITMENT_STATUS_EXPIRED
            {
                bail!("容量承诺 Expire deterministic 幂等键冲突");
            }
            let quantities = create_receipt_on(&transaction, stored.clone(), true)?.quantities;
            transaction.commit()?;
            return Ok(ComputeCapacityCommitmentTerminalWriteReceipt {
                commitment: stored,
                terminal_receipt: terminal,
                quantities,
                replayed: true,
            });
        }
        if terminal_by_commitment_on(&transaction, &commitment)?.is_some() {
            bail!("容量承诺已被 Cancel/Expire 竞争者终结");
        }
        if delivery_allocation_commitment_status_on(
            &transaction,
            &commitment.commitment_id,
            &commitment.commitment_digest,
        )?
        .is_some_and(|status| status.blocks_commitment_terminal())
        {
            bail!("active/exercised Delivery Allocation 阻止容量承诺 Expire");
        }
        let recorded_at = now();
        if parse_utc("capacity commitment recovery Store time", &recorded_at)?
            < parse_utc("capacity commitment expires_at", &commitment.expires_at)?
        {
            bail!("容量承诺尚未到期");
        }
        let receipt = terminalize_on(
            &transaction,
            &commitment,
            ComputeCapacityClaimTerminalAction::Expire,
            CAPACITY_COMMITMENT_STATUS_EXPIRED,
            "platform_admin",
            admin_user_id,
            Some("delivery_window_ended".to_string()),
            EXPIRE_IDEMPOTENCY_SCOPE,
            &key,
            &request_digest,
            &commitment.expires_at,
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
}

#[allow(clippy::too_many_arguments)]
fn terminalize_on(
    transaction: &Transaction<'_>,
    commitment: &ComputeCapacityCommitment,
    action: ComputeCapacityClaimTerminalAction,
    terminal_status: &str,
    actor_kind: &str,
    actor_id: &str,
    reason: Option<String>,
    idempotency_scope: &str,
    idempotency_key: &str,
    request_digest: &str,
    occurred_at: &str,
) -> Result<ComputeCapacityCommitmentTerminalWriteReceipt> {
    let finished = finish_compute_capacity_commitment_claim_on(
        transaction,
        FinishComputeCapacityClaim {
            claim_id: commitment.claim.claim_id.clone(),
            expected_revision: commitment.claim.claim_revision,
            action,
            idempotency_scope: idempotency_scope.to_string(),
            idempotency_key: idempotency_key.to_string(),
            occurred_at: occurred_at.to_string(),
        },
        commitment.offer.clone(),
        &commitment.commitment_id,
    )?;
    if finished.replayed || finished.revision != 2 {
        bail!("容量承诺缺少 terminal receipt 时不得采用 generic Claim replay");
    }
    let ledger_times = terminal_ledger_times_on(transaction, &finished.ledger.transaction_id)?;
    if ledger_times.2.as_deref() != Some(commitment.creation_ledger.transaction_id.as_str()) {
        bail!("容量承诺 terminal ledger 未引用原始 hold transaction");
    }
    let mut terminal = ComputeCapacityCommitmentTerminalReceipt {
        schema: COMPUTE_CAPACITY_COMMITMENT_TERMINAL_RECEIPT_SCHEMA.to_string(),
        terminal_receipt_id: new_id("compute_capacity_commitment_terminal"),
        terminal_revision: 2,
        terminal_receipt_digest: String::new(),
        terminal_status: terminal_status.to_string(),
        commitment_id: commitment.commitment_id.clone(),
        commitment_digest: commitment.commitment_digest.clone(),
        claim_id: commitment.claim.claim_id.clone(),
        prior_claim_revision: commitment.claim.claim_revision,
        prior_claim_digest: commitment.claim.claim_digest.clone(),
        result_claim_revision: finished.revision,
        result_claim_digest: finished.claim_digest,
        result_claim_state: finished.state,
        ledger: ComputeCapacityCommitmentLedgerBinding {
            transaction_id: finished.ledger.transaction_id,
            transaction_digest: finished.ledger.transaction_digest,
            ledger_sequence: finished.ledger.ledger_sequence,
            event_kind: finished.ledger.event_kind,
            causal_transaction_id: ledger_times.2,
        },
        actor_kind: actor_kind.to_string(),
        actor_id: actor_id.to_string(),
        reason,
        idempotency_scope: idempotency_scope.to_string(),
        idempotency_key: idempotency_key.to_string(),
        request_digest: request_digest.to_string(),
        occurred_at: ledger_times.0,
        recorded_at: ledger_times.1,
    };
    let (_, digest) = canonical_terminal_json_and_digest(&terminal)?;
    terminal.terminal_receipt_digest = digest;
    let (terminal_json, verified_digest) = canonical_terminal_json_and_digest(&terminal)?;
    if verified_digest != terminal.terminal_receipt_digest {
        bail!("容量承诺 terminal receipt canonical digest 不稳定");
    }
    insert_terminal_on(transaction, &terminal, &terminal_json)?;
    let stored = terminal_by_commitment_on(transaction, commitment)?
        .ok_or_else(|| anyhow!("容量承诺 terminal receipt 插入后无法 exact readback"))?;
    if stored != terminal {
        bail!("容量承诺 terminal receipt exact readback 不一致");
    }
    let quantities = create_receipt_on(transaction, commitment.clone(), false)?.quantities;
    Ok(ComputeCapacityCommitmentTerminalWriteReceipt {
        commitment: commitment.clone(),
        terminal_receipt: stored,
        quantities,
        replayed: false,
    })
}

fn terminal_ledger_times_on(
    transaction: &Transaction<'_>,
    transaction_id: &str,
) -> Result<(String, String, Option<String>)> {
    transaction
        .query_row(
            "SELECT occurred_at, recorded_at, causal_transaction_id
               FROM compute_capacity_ledger_transactions WHERE transaction_id=?1",
            params![transaction_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("容量承诺 terminal ledger transaction 缺失"))
}

fn insert_terminal_on(
    transaction: &Transaction<'_>,
    value: &ComputeCapacityCommitmentTerminalReceipt,
    json: &str,
) -> Result<()> {
    let causal_transaction_id = value
        .ledger
        .causal_transaction_id
        .as_deref()
        .ok_or_else(|| anyhow!("容量承诺 terminal receipt 缺少 causal transaction"))?;
    let changed = transaction.execute(
        "INSERT INTO compute_capacity_commitment_terminal_receipts (
            terminal_receipt_id, terminal_schema, terminal_revision, terminal_status,
            terminal_receipt_digest, terminal_receipt_json, canonicalization, digest_algorithm,
            commitment_id, commitment_revision, commitment_digest, claim_id,
            prior_claim_revision, prior_claim_digest, result_claim_revision,
            result_claim_digest, result_claim_state, terminal_transaction_id,
            terminal_transaction_digest, terminal_ledger_sequence, terminal_event_kind,
            causal_transaction_id, actor_kind, actor_id, reason, idempotency_scope,
            idempotency_key, request_digest, occurred_at, recorded_at
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,'rfc8785_jcs','sha256',?7,1,?8,?9,?10,?11,
            ?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27
        )",
        params![
            value.terminal_receipt_id,
            value.schema,
            value.terminal_revision,
            value.terminal_status,
            value.terminal_receipt_digest,
            json,
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
            causal_transaction_id,
            value.actor_kind,
            value.actor_id,
            value.reason,
            value.idempotency_scope,
            value.idempotency_key,
            value.request_digest,
            value.occurred_at,
            value.recorded_at,
        ],
    )?;
    if changed != 1 {
        bail!("容量承诺 terminal receipt 插入数量异常");
    }
    Ok(())
}
