use std::collections::HashSet;

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use super::{
    model::{
        EskAllocationBatchInput, PaperAllocationBatchBody, MAX_PAPER_ALLOCATION_BATCH_ENTRIES,
    },
    parse_esk_amount,
    service::validate_bounded_label,
    EskAllocationInput,
};

const BATCH_DIGEST_DOMAIN: &[u8] = b"yilong.esk.paper_allocation_batch_request.v1";

pub(crate) fn prepare_paper_allocation_batch(
    body: PaperAllocationBatchBody,
) -> Result<EskAllocationBatchInput> {
    let batch_id = validate_bounded_label(&body.batch_id, "批次 ID", 160)?;
    if body.entries.is_empty() || body.entries.len() > MAX_PAPER_ALLOCATION_BATCH_ENTRIES {
        bail!("ESK Paper 批次条目数量必须为 1..={MAX_PAPER_ALLOCATION_BATCH_ENTRIES}");
    }

    let mut references = HashSet::with_capacity(body.entries.len());
    let mut idempotency_keys = HashSet::with_capacity(body.entries.len());
    let mut total_base_units = 0_i64;
    let mut entries = Vec::with_capacity(body.entries.len());
    for entry in body.entries {
        let user_id = validate_bounded_label(&entry.user_id, "用户 ID", 160)?;
        let reference = validate_bounded_label(&entry.reference, "登记引用", 240)?;
        let idempotency_key = validate_bounded_label(&entry.idempotency_key, "幂等键", 160)?;
        if !references.insert(reference.clone()) {
            bail!("ESK Paper 批次包含重复登记引用");
        }
        if !idempotency_keys.insert(idempotency_key.clone()) {
            bail!("ESK Paper 批次包含重复幂等键");
        }
        let amount_base_units = parse_esk_amount(&entry.amount)?;
        total_base_units = total_base_units
            .checked_add(amount_base_units)
            .ok_or_else(|| anyhow::anyhow!("ESK Paper 批次总金额超出范围"))?;
        entries.push(EskAllocationInput {
            user_id,
            amount_base_units,
            reference,
            idempotency_key,
        });
    }

    let request_digest = batch_request_digest(&batch_id, &entries);
    Ok(EskAllocationBatchInput {
        batch_id,
        request_digest,
        total_base_units,
        entries,
    })
}

fn batch_request_digest(batch_id: &str, entries: &[EskAllocationInput]) -> String {
    let mut digest = Sha256::new();
    update_digest_field(&mut digest, BATCH_DIGEST_DOMAIN);
    update_digest_field(&mut digest, batch_id.as_bytes());
    digest.update((entries.len() as u64).to_be_bytes());
    for entry in entries {
        update_digest_field(&mut digest, entry.user_id.as_bytes());
        digest.update(entry.amount_base_units.to_be_bytes());
        update_digest_field(&mut digest, entry.reference.as_bytes());
        update_digest_field(&mut digest, entry.idempotency_key.as_bytes());
    }
    hex::encode(digest.finalize())
}

fn update_digest_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}
