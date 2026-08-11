use anyhow::{bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::platform_reference_price_curve::{
        canonical_platform_reference_price_curve_batch_json_and_digest,
        canonical_platform_reference_price_curve_batch_material_digest,
        canonical_platform_reference_price_curve_entry_json_and_digest,
        canonical_platform_reference_price_curve_entry_set_digest,
        validate_platform_reference_price_curve_batch_envelope,
        validate_platform_reference_price_curve_batch_material,
        validate_platform_reference_price_curve_entry_against_batch,
        ComputePlatformReferencePriceCurveBatch, ComputePlatformReferencePriceCurveBatchEnvelope,
        ComputePlatformReferencePriceCurveEntryEnvelope,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA,
    },
    store::{new_id, Store},
};

use super::{
    canonical::canonical_json,
    read::{batch_by_curve_on, batch_by_id_on, batch_by_idempotency_on, entries_by_batch_on},
    review::{now_nanos, validate_exact},
    types::{
        ComputePlatformReferencePriceCurveBatchReceipt,
        SubmitComputePlatformReferencePriceCurveBatch,
    },
};

impl Store {
    pub(in crate::store) fn submit_compute_platform_reference_price_curve_batch(
        &self,
        input: SubmitComputePlatformReferencePriceCurveBatch,
    ) -> Result<ComputePlatformReferencePriceCurveBatchReceipt> {
        validate_exact(&input.idempotency_scope, "batch idempotency scope", 200)?;
        validate_exact(&input.idempotency_key, "batch idempotency key", 160)?;
        let entry_set_digest =
            canonical_platform_reference_price_curve_entry_set_digest(&input.entries)?;
        let material = batch_material(&input, entry_set_digest, String::new());
        validate_platform_reference_price_curve_batch_material(&material)?;
        let material_digest =
            canonical_platform_reference_price_curve_batch_material_digest(&material)?;

        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(stored) = batch_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input, &material_digest)?;
            let entries = entries_by_batch_on(&transaction, &stored.envelope.batch_id)?
                .into_iter()
                .map(|entry| entry.into_receipt())
                .collect();
            let receipt = stored.into_receipt(entries, true);
            transaction.commit()?;
            return Ok(receipt);
        }
        if batch_by_curve_on(&transaction, &input.curve_id, input.curve_version)?.is_some() {
            bail!("platform reference price curve version already has an immutable batch");
        }

        let batch_id = new_id("compute_platform_reference_price_curve_batch");
        let submitted_at = now_nanos();
        let batch = batch_material(
            &input,
            material.entry_set_digest.clone(),
            submitted_at.clone(),
        );
        let mut envelope = ComputePlatformReferencePriceCurveBatchEnvelope {
            schema: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA.to_string(),
            batch_id,
            batch_digest: String::new(),
            batch_material_digest: material_digest,
            canonicalization: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM.to_string(),
            batch,
        };
        let (_, batch_digest) =
            canonical_platform_reference_price_curve_batch_json_and_digest(&envelope)?;
        envelope.batch_digest = batch_digest;
        validate_platform_reference_price_curve_batch_envelope(&envelope)?;
        let (batch_json, digest) =
            canonical_platform_reference_price_curve_batch_json_and_digest(&envelope)?;
        if digest != envelope.batch_digest {
            bail!("platform reference price curve batch digest changed before persistence");
        }

        insert_entries(&transaction, &envelope, &submitted_at)?;
        insert_batch(
            &transaction,
            &envelope,
            &batch_json,
            &input.idempotency_scope,
            &submitted_at,
        )?;
        let stored = batch_by_id_on(&transaction, &envelope.batch_id)?
            .ok_or_else(|| anyhow::anyhow!("reference price curve batch is absent after insert"))?;
        let entries = entries_by_batch_on(&transaction, &envelope.batch_id)?
            .into_iter()
            .map(|entry| entry.into_receipt())
            .collect();
        let receipt = stored.into_receipt(entries, false);
        transaction.commit()?;
        Ok(receipt)
    }
}

fn batch_material(
    input: &SubmitComputePlatformReferencePriceCurveBatch,
    entry_set_digest: String,
    submitted_at: String,
) -> ComputePlatformReferencePriceCurveBatch {
    ComputePlatformReferencePriceCurveBatch {
        submitted_by_admin_user_id: input.submitted_by_admin_user_id.clone(),
        curve_id: input.curve_id.clone(),
        curve_version: input.curve_version,
        methodology_kind: input.methodology_kind.clone(),
        valid_from: input.valid_from.clone(),
        valid_until: input.valid_until.clone(),
        quote_ttl_seconds: input.quote_ttl_seconds,
        rounding_mode: input.rounding_mode.clone(),
        entries: input.entries.clone(),
        entry_set_digest,
        idempotency_key: input.idempotency_key.clone(),
        confirmation: input.confirmation.clone(),
        submission_note: input.submission_note.clone(),
        submitted_at,
    }
}

fn insert_entries(
    transaction: &Transaction<'_>,
    batch: &ComputePlatformReferencePriceCurveBatchEnvelope,
    created_at: &str,
) -> Result<()> {
    for (index, entry) in batch.batch.entries.iter().enumerate() {
        let mut envelope = ComputePlatformReferencePriceCurveEntryEnvelope {
            schema: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA.to_string(),
            batch_id: batch.batch_id.clone(),
            batch_digest: batch.batch_digest.clone(),
            entry_id: new_id("compute_platform_reference_price_curve_entry"),
            entry_digest: String::new(),
            ordinal: i64::try_from(index + 1)?,
            entry: entry.clone(),
        };
        let (_, entry_digest) =
            canonical_platform_reference_price_curve_entry_json_and_digest(&envelope)?;
        envelope.entry_digest = entry_digest;
        validate_platform_reference_price_curve_entry_against_batch(&envelope, batch)?;
        let (entry_json, digest) =
            canonical_platform_reference_price_curve_entry_json_and_digest(&envelope)?;
        if digest != envelope.entry_digest {
            bail!("platform reference price curve entry digest changed before persistence");
        }
        let components_json = canonical_json(&entry.components)?;
        let fee_rules_json = canonical_json(&entry.fee_rules)?;
        transaction.execute(
            "INSERT INTO compute_platform_reference_price_curve_entries (
                entry_id, entry_schema, entry_digest, entry_json,
                batch_id, batch_digest, ordinal, entry_key,
                provider_id, offer_id, offer_version, offer_digest,
                sku_id, sku_digest, delivery_window_id, delivery_window_digest,
                pricing_mode, currency, offer_curve_id, offer_curve_version,
                instrument_id, components_json, fee_rules_json,
                consumer_max_amount_micros, provider_max_amount_micros, created_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26
             )",
            params![
                envelope.entry_id,
                envelope.schema,
                envelope.entry_digest,
                entry_json,
                envelope.batch_id,
                envelope.batch_digest,
                envelope.ordinal,
                entry.entry_key,
                entry.provider_id,
                entry.offer_id,
                entry.offer_version,
                entry.offer_digest,
                entry.sku_id,
                entry.sku_digest,
                entry.delivery_window_id,
                entry.delivery_window_digest,
                entry.pricing_mode,
                entry.currency,
                entry.offer_curve_id,
                entry.offer_curve_version,
                entry.instrument_id,
                components_json,
                fee_rules_json,
                entry.consumer_max_amount_micros,
                entry.provider_max_amount_micros,
                created_at,
            ],
        )?;
    }
    Ok(())
}

fn insert_batch(
    transaction: &Transaction<'_>,
    envelope: &ComputePlatformReferencePriceCurveBatchEnvelope,
    batch_json: &str,
    idempotency_scope: &str,
    created_at: &str,
) -> Result<()> {
    let batch = &envelope.batch;
    transaction.execute(
        "INSERT INTO compute_platform_reference_price_curve_batches (
            batch_id, batch_schema, batch_digest, batch_json, batch_material_digest,
            canonicalization, digest_algorithm, curve_id, curve_version, methodology_kind,
            valid_from, valid_until, quote_ttl_seconds, rounding_mode, entry_count, entry_set_digest,
            confirmation, submission_note, submitted_by_admin_user_id, submitted_at,
            status, reviewed_by_admin_user_id, reviewed_at, applied_by_admin_user_id, applied_at,
            idempotency_scope, idempotency_key, created_at, updated_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, ?17, ?18, ?19, ?20, 'submitted', NULL, NULL, NULL, NULL,
            ?21, ?22, ?23, ?23
         )",
        params![
            envelope.batch_id,
            envelope.schema,
            envelope.batch_digest,
            batch_json,
            envelope.batch_material_digest,
            envelope.canonicalization,
            envelope.digest_algorithm,
            batch.curve_id,
            batch.curve_version,
            batch.methodology_kind,
            batch.valid_from,
            batch.valid_until,
            batch.quote_ttl_seconds,
            batch.rounding_mode,
            i64::try_from(batch.entries.len())?,
            batch.entry_set_digest,
            batch.confirmation,
            batch.submission_note,
            batch.submitted_by_admin_user_id,
            batch.submitted_at,
            idempotency_scope,
            batch.idempotency_key,
            created_at,
        ],
    )?;
    Ok(())
}

fn ensure_replay(
    stored: &super::types::StoredBatch,
    input: &SubmitComputePlatformReferencePriceCurveBatch,
    material_digest: &str,
) -> Result<()> {
    let batch = &stored.envelope.batch;
    if stored.envelope.batch_material_digest != material_digest
        || batch.submitted_by_admin_user_id != input.submitted_by_admin_user_id
        || batch.curve_id != input.curve_id
        || batch.curve_version != input.curve_version
        || batch.methodology_kind != input.methodology_kind
        || batch.valid_from != input.valid_from
        || batch.valid_until != input.valid_until
        || batch.quote_ttl_seconds != input.quote_ttl_seconds
        || batch.rounding_mode != input.rounding_mode
        || batch.entries != input.entries
        || batch.idempotency_key != input.idempotency_key
        || batch.confirmation != input.confirmation
        || batch.submission_note != input.submission_note
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("platform reference price curve batch replay conflicts with immutable history");
    }
    Ok(())
}
