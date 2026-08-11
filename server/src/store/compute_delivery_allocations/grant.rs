use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::compute_federation::{
    delivery_allocation::{
        ComputeDeliveryAllocationCommitmentBinding, ComputeDeliveryAllocationGrant,
        COMPUTE_DELIVERY_ALLOCATION_GRANT_SCHEMA, DELIVERY_ALLOCATION_STATUS_GRANTED,
    },
    execution::ComputeJobVersionBinding,
};

use super::{
    super::{new_id, now, Store},
    canonical::{canonical_grant_json_and_digest, create_request_digest},
    read::{
        grant_by_commitment_on, grant_by_id_on, grant_by_idempotency_on, grant_by_job_on,
        grant_receipt_on,
    },
    types::{ComputeDeliveryAllocationGrantWriteReceipt, CreateComputeDeliveryAllocationGrant},
    validation::{validate_create_input, validate_grant_source_on},
};

impl Store {
    pub(crate) fn create_compute_delivery_allocation_grant(
        &self,
        input: CreateComputeDeliveryAllocationGrant,
    ) -> Result<ComputeDeliveryAllocationGrantWriteReceipt> {
        validate_create_input(&input)?;
        let request_digest = create_request_digest(&input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = grant_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            if existing.request_digest != request_digest {
                bail!("相同 DeliveryAllocation Grant 幂等键不能用于不同请求");
            }
            let receipt = grant_receipt_on(&transaction, existing, true)?;
            transaction.commit()?;
            return Ok(receipt);
        }

        let created_at = now();
        let source = validate_grant_source_on(&transaction, &input, &created_at)?;
        if grant_by_commitment_on(&transaction, &input.commitment_id)?.is_some() {
            bail!("一份 Commitment 一生只能创建一份 DeliveryAllocation Grant");
        }
        if grant_by_job_on(&transaction, &input.job_id)?.is_some() {
            bail!("一个 Job 一生只能绑定一份 DeliveryAllocation Grant");
        }
        let commitment = source.commitment.commitment;
        let mut grant = ComputeDeliveryAllocationGrant {
            schema: COMPUTE_DELIVERY_ALLOCATION_GRANT_SCHEMA.to_string(),
            grant_id: new_id("compute_delivery_allocation_grant"),
            grant_revision: 1,
            grant_digest: String::new(),
            grant_status: DELIVERY_ALLOCATION_STATUS_GRANTED.to_string(),
            commitment: ComputeDeliveryAllocationCommitmentBinding {
                commitment_id: commitment.commitment_id,
                commitment_revision: commitment.commitment_revision,
                commitment_digest: commitment.commitment_digest,
            },
            provider_owner_account_id: input.provider_owner_account_id,
            consumer_account_id: input.consumer_account_id,
            project_id: source.source_job.job.project_id.clone(),
            job: ComputeJobVersionBinding {
                job_id: source.source_job.job.job_id,
                job_revision: source.source_job.revision,
                job_digest: source.source_job.job_digest,
            },
            exercise_expires_at: commitment.delivery_window.starts_at_utc,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            request_digest,
            created_at,
        };
        let (_, digest) = canonical_grant_json_and_digest(&grant)?;
        grant.grant_digest = digest;
        let (json, verified_digest) = canonical_grant_json_and_digest(&grant)?;
        if verified_digest != grant.grant_digest {
            bail!("DeliveryAllocation Grant canonical digest 不稳定");
        }
        insert_grant_on(&transaction, &grant, &json)?;
        let stored = grant_by_id_on(&transaction, &grant.grant_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation Grant 插入后无法 exact readback"))?;
        if stored != grant {
            bail!("DeliveryAllocation Grant immutable readback 与候选不一致");
        }
        let receipt = grant_receipt_on(&transaction, stored, false)?;
        transaction.commit()?;
        Ok(receipt)
    }
}

fn insert_grant_on(
    transaction: &Transaction<'_>,
    value: &ComputeDeliveryAllocationGrant,
    json: &str,
) -> Result<()> {
    let changed = transaction.execute(
        "INSERT INTO compute_delivery_allocation_grants (
            grant_id, grant_schema, grant_revision, grant_status, grant_digest, grant_json,
            canonicalization, digest_algorithm, commitment_id, commitment_revision,
            commitment_digest, provider_owner_account_id, consumer_account_id, project_id,
            job_id, job_revision, job_digest, exercise_expires_at, idempotency_scope,
            idempotency_key, request_digest, created_at
         ) VALUES (
            ?1,?2,?3,?4,?5,?6,'rfc8785_jcs','sha256',?7,?8,?9,?10,?11,?12,
            ?13,?14,?15,?16,?17,?18,?19,?20
         )",
        params![
            value.grant_id,
            value.schema,
            value.grant_revision,
            value.grant_status,
            value.grant_digest,
            json,
            value.commitment.commitment_id,
            value.commitment.commitment_revision,
            value.commitment.commitment_digest,
            value.provider_owner_account_id,
            value.consumer_account_id,
            value.project_id,
            value.job.job_id,
            value.job.job_revision,
            value.job.job_digest,
            value.exercise_expires_at,
            value.idempotency_scope,
            value.idempotency_key,
            value.request_digest,
            value.created_at,
        ],
    )?;
    if changed != 1 {
        bail!("DeliveryAllocation Grant immutable insert 数量异常");
    }
    Ok(())
}
