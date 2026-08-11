use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, TransactionBehavior};

use crate::compute_federation::{
    capacity::{
        ComputeCapacityCausalBinding, ComputeCapacityClaimBinding, ComputeCapacityClaimKind,
    },
    capacity_commitment::{
        ComputeCapacityCommitment, ComputeCapacityCommitmentLedgerBinding,
        ComputeCapacityCommitmentProviderBinding, ComputeCapacityCommitmentReferenceBinding,
        CAPACITY_COMMITMENT_STATUS_COMMITTED, COMPUTE_CAPACITY_COMMITMENT_SCHEMA,
    },
};

use super::{
    super::{
        compute_capacity_claims::{hold_compute_capacity_claim_on, HoldComputeCapacityClaim},
        new_id, now, Store,
    },
    canonical::{canonical_commitment_json_and_digest, create_request_digest},
    read::{commitment_by_id_on, commitment_by_idempotency_on, create_receipt_on},
    types::{ComputeCapacityCommitmentCreateReceipt, CreateComputeCapacityCommitment},
    validation::{
        offer_binding, parse_utc, validate_create_dependencies_on, validate_create_input,
    },
};

impl Store {
    pub(crate) fn create_compute_capacity_commitment(
        &self,
        input: CreateComputeCapacityCommitment,
    ) -> Result<ComputeCapacityCommitmentCreateReceipt> {
        validate_create_input(&input)?;
        let request_digest = create_request_digest(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = commitment_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            if existing.request_digest != request_digest {
                bail!("相同容量承诺 Create 幂等键不能用于不同请求");
            }
            let receipt = create_receipt_on(&transaction, existing, true)?;
            transaction.commit()?;
            return Ok(receipt);
        }

        let preflight_at = now();
        let validated = validate_create_dependencies_on(&transaction, &input, &preflight_at)?;
        let commitment_id = new_id("compute_capacity_commitment");
        let offer = offer_binding(&validated.offer);
        let held = hold_compute_capacity_claim_on(
            &transaction,
            HoldComputeCapacityClaim {
                pool: input.pool.clone(),
                delivery_window: input.delivery_window.clone(),
                claim_kind: ComputeCapacityClaimKind::CapacityCommitment,
                subject_kind: "compute_capacity_commitment".to_string(),
                subject_id: commitment_id.clone(),
                idempotency_scope: format!(
                    "compute_capacity_commitment_create:{}",
                    input.idempotency_scope
                ),
                idempotency_key: input.idempotency_key.clone(),
                lines: validated.claim_lines,
                expires_at: Some(validated.delivery_window.ends_at_utc.clone()),
                occurred_at: preflight_at,
                causal_binding: ComputeCapacityCausalBinding {
                    offer: Some(offer.clone()),
                    job_id: None,
                    reservation_id: None,
                    attempt_lease_id: None,
                    fencing_generation: None,
                },
            },
        )
        .context("容量承诺 Claim/ledger hold 失败")?;
        let held_expires_at = parse_utc("Claim hold expires_at", &held.expires_at)?;
        let window_ends_at = parse_utc(
            "delivery window end",
            &validated.delivery_window.ends_at_utc,
        )?;
        if held.replayed
            || held.revision != 1
            || held.state != "held"
            || held.claim_kind != "capacity_commitment"
            || held_expires_at != window_ends_at
            || held.ledger.event_kind != "reservation_held"
        {
            bail!("容量承诺 Create 获得了无效或意外重放的 Claim hold");
        }

        let mut commitment = ComputeCapacityCommitment {
            schema: COMPUTE_CAPACITY_COMMITMENT_SCHEMA.to_string(),
            commitment_id,
            commitment_revision: 1,
            commitment_digest: String::new(),
            commitment_status: CAPACITY_COMMITMENT_STATUS_COMMITTED.to_string(),
            owner_account_id: input.owner_account_id,
            provider: ComputeCapacityCommitmentProviderBinding {
                provider_id: input.provider_id,
                policy_revision: input.provider_policy_revision,
                provider_digest: input.provider_digest,
            },
            offer,
            pool: input.pool,
            delivery_window: validated.delivery_window,
            price_snapshot_id: validated.snapshot.snapshot_id,
            price_snapshot_digest: validated.snapshot.snapshot_digest,
            reference_binding: ComputeCapacityCommitmentReferenceBinding {
                binding_id: input.reference_binding_id,
                binding_digest: input.reference_binding_digest,
            },
            instrument_id: input.instrument_id,
            claim: ComputeCapacityClaimBinding {
                claim_id: held.claim_id,
                claim_revision: held.revision,
                claim_digest: held.claim_digest,
            },
            creation_ledger: ComputeCapacityCommitmentLedgerBinding {
                transaction_id: held.ledger.transaction_id,
                transaction_digest: held.ledger.transaction_digest,
                ledger_sequence: held.ledger.ledger_sequence,
                event_kind: held.ledger.event_kind,
                causal_transaction_id: None,
            },
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            request_digest,
            created_at: held.recorded_at,
            expires_at: held.expires_at,
        };
        let (_, commitment_digest) = canonical_commitment_json_and_digest(&commitment)?;
        commitment.commitment_digest = commitment_digest;
        let (commitment_json, verified_digest) = canonical_commitment_json_and_digest(&commitment)?;
        if commitment_json.is_empty() || verified_digest != commitment.commitment_digest {
            bail!("容量承诺 canonical exact readback 准备失败");
        }
        insert_commitment_on(&transaction, &commitment, &commitment_json)?;
        let stored = commitment_by_id_on(&transaction, &commitment.commitment_id)?
            .ok_or_else(|| anyhow!("容量承诺插入后无法 exact readback"))?;
        if stored != commitment {
            bail!("容量承诺插入后的 immutable root 与候选不一致");
        }
        let receipt = create_receipt_on(&transaction, stored, false)?;
        transaction.commit()?;
        Ok(receipt)
    }
}

fn insert_commitment_on(
    transaction: &rusqlite::Transaction<'_>,
    value: &ComputeCapacityCommitment,
    json: &str,
) -> Result<()> {
    let changed = transaction.execute(
        "INSERT INTO compute_capacity_commitments (
            commitment_id, commitment_schema, commitment_revision, commitment_status,
            commitment_digest, commitment_json, canonicalization, digest_algorithm,
            owner_account_id, provider_id, provider_policy_revision, provider_digest,
            offer_id, offer_version, offer_digest, pool_id, capacity_epoch,
            pool_revision, pool_digest, delivery_window_id, delivery_window_digest,
            delivery_window_starts_at, delivery_window_ends_at, price_snapshot_id,
            price_snapshot_digest, reference_binding_id, reference_binding_digest,
            instrument_id, claim_id, claim_revision, claim_digest, hold_transaction_id,
            hold_transaction_digest, hold_ledger_sequence, hold_event_kind,
            idempotency_scope, idempotency_key, request_digest, created_at, expires_at
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,'rfc8785_jcs','sha256',?7,?8,?9,?10,
            ?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,
            ?26,?27,?28,?29,?30,?31,?32,?33,?34,?35,?36,?37,?38
        )",
        params![
            value.commitment_id,
            value.schema,
            value.commitment_revision,
            value.commitment_status,
            value.commitment_digest,
            json,
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
    )?;
    if changed != 1 {
        bail!("容量承诺 immutable root 插入数量异常");
    }
    Ok(())
}
