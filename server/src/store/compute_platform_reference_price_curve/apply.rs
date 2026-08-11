use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction, TransactionBehavior};

use crate::{
    compute_federation::platform_reference_price_curve::{
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
    },
    store::{compute_price_snapshot_registry::register_compute_price_snapshot_on, new_id, Store},
};

use super::{
    canonical::{
        canonical_application_json_and_digest, canonical_json,
        canonical_snapshot_binding_set_digest,
    },
    read::{
        application_by_batch_on, application_by_idempotency_on, batch_by_id_on,
        bindings_by_application_on, entries_by_batch_on, review_by_batch_on,
    },
    review::{now_nanos, validate_digest, validate_exact, validate_optional_note},
    snapshot::{prepare_snapshot_binding, PreparedSnapshotBinding},
    types::{
        ApplyComputePlatformReferencePriceCurveBatch,
        ComputePlatformReferencePriceCurveApplicationReceipt, StoredApplication,
        StoredApplicationEnvelope, StoredApplicationMaterial, StoredBatch, StoredReview,
        StoredSnapshotBinding, APPLICATION_STATUS_APPLIED,
        PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA,
        PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION, REVIEW_DECISION_APPROVED,
    },
};

impl Store {
    pub(crate) fn apply_compute_platform_reference_price_curve_batch(
        &self,
        mut input: ApplyComputePlatformReferencePriceCurveBatch,
    ) -> Result<ComputePlatformReferencePriceCurveApplicationReceipt> {
        validate_input(&mut input)?;
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(stored) = application_by_idempotency_on(
            &transaction,
            &input.idempotency_scope,
            &input.idempotency_key,
        )? {
            ensure_replay(&stored, &input)?;
            let receipt = application_receipt(&transaction, stored, true)?;
            transaction.commit()?;
            return Ok(receipt);
        }
        if let Some(stored) = application_by_batch_on(&transaction, &input.batch_id)? {
            ensure_replay(&stored, &input)?;
            let receipt = application_receipt(&transaction, stored, true)?;
            transaction.commit()?;
            return Ok(receipt);
        }

        let batch = batch_by_id_on(&transaction, &input.batch_id)?
            .ok_or_else(|| anyhow::anyhow!("platform reference price curve batch is absent"))?;
        let review = review_by_batch_on(&transaction, &input.batch_id)?
            .ok_or_else(|| anyhow::anyhow!("platform reference price curve review is absent"))?;
        ensure_exact_approval(&batch, &review, &input)?;

        let application_id = new_id("compute_platform_reference_price_curve_application");
        let applied_at = now_nanos();
        let applied_time = DateTime::parse_from_rfc3339(&applied_at)?.with_timezone(&Utc);
        let entries = entries_by_batch_on(&transaction, &input.batch_id)?;
        if entries.len() != batch.envelope.batch.entries.len() {
            bail!("platform reference price curve batch lost an entry before application");
        }
        let mut prepared = Vec::with_capacity(entries.len());
        for entry in &entries {
            prepared.push(prepare_snapshot_binding(
                &transaction,
                &application_id,
                &batch,
                &review,
                entry,
                &applied_at,
            )?);
        }
        let binding_digests = prepared
            .iter()
            .map(|value| value.envelope.binding_digest.clone())
            .collect::<Vec<_>>();
        let binding_set_digest = canonical_snapshot_binding_set_digest(&binding_digests)?;
        let mut envelope = StoredApplicationEnvelope {
            schema: PLATFORM_REFERENCE_PRICE_CURVE_APPLICATION_SCHEMA.to_string(),
            application_id,
            application_digest: String::new(),
            canonicalization: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM.to_string(),
            application: StoredApplicationMaterial {
                batch_id: batch.envelope.batch_id.clone(),
                batch_digest: batch.envelope.batch_digest.clone(),
                batch_material_digest: batch.envelope.batch_material_digest.clone(),
                review_id: review.envelope.review_id.clone(),
                review_digest: review.envelope.review_digest.clone(),
                curve_id: batch.envelope.batch.curve_id.clone(),
                curve_version: batch.envelope.batch.curve_version,
                binding_digests,
                binding_set_digest,
                submitted_by_admin_user_id: batch.envelope.batch.submitted_by_admin_user_id.clone(),
                reviewed_by_admin_user_id: review.envelope.review.reviewed_by_admin_user_id.clone(),
                applied_by_admin_user_id: input.applied_by_admin_user_id.clone(),
                apply_confirmation: input.apply_confirmation.clone(),
                apply_note: input.apply_note.clone(),
                applied_at: applied_at.clone(),
                status: APPLICATION_STATUS_APPLIED.to_string(),
            },
        };
        let (_, digest) = canonical_application_json_and_digest(&envelope)?;
        envelope.application_digest = digest;
        let (application_json, digest) = canonical_application_json_and_digest(&envelope)?;
        if digest != envelope.application_digest {
            bail!("platform reference price curve application digest changed before persistence");
        }

        for value in &prepared {
            insert_binding(&transaction, value)?;
        }
        for value in &prepared {
            let receipt =
                register_compute_price_snapshot_on(&transaction, &value.snapshot, &applied_time)?;
            if receipt.replayed || receipt.snapshot != value.snapshot {
                bail!(
                    "platform reference price curve application did not create one exact Snapshot"
                );
            }
        }
        insert_application(
            &transaction,
            &envelope,
            &application_json,
            &input.idempotency_scope,
            &input.idempotency_key,
        )?;
        let stored = application_by_batch_on(&transaction, &input.batch_id)?.ok_or_else(|| {
            anyhow::anyhow!("platform reference price curve application is absent after insert")
        })?;
        let receipt = application_receipt(&transaction, stored, false)?;
        transaction.commit()?;
        Ok(receipt)
    }
}

fn insert_binding(transaction: &Transaction<'_>, value: &PreparedSnapshotBinding) -> Result<()> {
    let envelope = &value.envelope;
    let binding = &envelope.binding;
    transaction.execute(
        "INSERT INTO compute_platform_reference_price_curve_snapshot_bindings (
            binding_id, binding_schema, binding_digest, binding_json,
            canonicalization, digest_algorithm, application_id, batch_id, batch_digest,
            review_id, review_digest, entry_id, entry_digest, ordinal, entry_key,
            curve_id, curve_version, snapshot_id, snapshot_digest, quote_id,
            source_kind, source_id, source_version, source_digest,
            quoted_at, expires_at, status, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?25
         )",
        params![
            envelope.binding_id,
            envelope.schema,
            envelope.binding_digest,
            value.binding_json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            binding.application_id,
            binding.batch_id,
            binding.batch_digest,
            binding.review_id,
            binding.review_digest,
            binding.entry_id,
            binding.entry_digest,
            binding.ordinal,
            binding.entry_key,
            binding.curve_id,
            binding.curve_version,
            binding.snapshot_id,
            binding.snapshot_digest,
            binding.quote_id,
            binding.source_kind,
            binding.source_id,
            binding.source_version,
            binding.source_digest,
            binding.quoted_at,
            binding.expires_at,
            binding.status,
        ],
    )?;
    Ok(())
}

fn insert_application(
    transaction: &Transaction<'_>,
    envelope: &StoredApplicationEnvelope,
    application_json: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
) -> Result<()> {
    let application = &envelope.application;
    let binding_digests_json = canonical_json(&application.binding_digests)?;
    transaction.execute(
        "INSERT INTO compute_platform_reference_price_curve_applications (
            application_id, application_schema, application_digest, application_json,
            canonicalization, digest_algorithm, batch_id, batch_digest,
            batch_material_digest, review_id, review_digest, curve_id, curve_version,
            binding_digests_json, binding_count, binding_set_digest,
            submitted_by_admin_user_id, reviewed_by_admin_user_id, applied_by_admin_user_id,
            apply_confirmation, apply_note, applied_at, status,
            idempotency_scope, idempotency_key, created_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
            ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?22
         )",
        params![
            envelope.application_id,
            envelope.schema,
            envelope.application_digest,
            application_json,
            envelope.canonicalization,
            envelope.digest_algorithm,
            application.batch_id,
            application.batch_digest,
            application.batch_material_digest,
            application.review_id,
            application.review_digest,
            application.curve_id,
            application.curve_version,
            binding_digests_json,
            i64::try_from(application.binding_digests.len())?,
            application.binding_set_digest,
            application.submitted_by_admin_user_id,
            application.reviewed_by_admin_user_id,
            application.applied_by_admin_user_id,
            application.apply_confirmation,
            application.apply_note,
            application.applied_at,
            application.status,
            idempotency_scope,
            idempotency_key,
        ],
    )?;
    Ok(())
}

fn ensure_exact_approval(
    batch: &StoredBatch,
    review: &StoredReview,
    input: &ApplyComputePlatformReferencePriceCurveBatch,
) -> Result<()> {
    let review_material = &review.envelope.review;
    if batch.status != REVIEW_DECISION_APPROVED
        || batch.envelope.batch_digest != input.expected_batch_digest
        || batch.envelope.batch_material_digest != input.expected_batch_material_digest
        || review.envelope.review_id != input.expected_review_id
        || review.envelope.review_digest != input.expected_review_digest
        || review_material.batch_id != input.batch_id
        || review_material.batch_digest != input.expected_batch_digest
        || review_material.batch_material_digest != input.expected_batch_material_digest
        || review_material.decision != REVIEW_DECISION_APPROVED
        || review_material.reviewed_by_admin_user_id
            == batch.envelope.batch.submitted_by_admin_user_id
        || batch.reviewed_by_admin_user_id.as_deref()
            != Some(review_material.reviewed_by_admin_user_id.as_str())
    {
        bail!("only the exact approved platform reference price curve can be applied");
    }
    Ok(())
}

fn ensure_replay(
    stored: &StoredApplication,
    input: &ApplyComputePlatformReferencePriceCurveBatch,
) -> Result<()> {
    let application = &stored.envelope.application;
    if application.batch_id != input.batch_id
        || application.batch_digest != input.expected_batch_digest
        || application.batch_material_digest != input.expected_batch_material_digest
        || application.review_id != input.expected_review_id
        || application.review_digest != input.expected_review_digest
        || application.applied_by_admin_user_id != input.applied_by_admin_user_id
        || application.apply_confirmation != input.apply_confirmation
        || application.apply_note != input.apply_note
        || stored.idempotency_scope != input.idempotency_scope
        || stored.idempotency_key != input.idempotency_key
    {
        bail!("platform reference price curve application replay conflicts with history");
    }
    Ok(())
}

fn application_receipt(
    transaction: &Transaction<'_>,
    stored: StoredApplication,
    replayed: bool,
) -> Result<ComputePlatformReferencePriceCurveApplicationReceipt> {
    let bindings = bindings_by_application_on(transaction, &stored.envelope.application_id)?;
    if replayed {
        revalidate_replay_currentness(transaction, &stored, &bindings)?;
    }
    let bindings = bindings
        .into_iter()
        .map(|binding| binding.into_receipt())
        .collect();
    Ok(stored.into_receipt(bindings, replayed))
}

fn revalidate_replay_currentness(
    transaction: &Transaction<'_>,
    application: &StoredApplication,
    bindings: &[StoredSnapshotBinding],
) -> Result<()> {
    let material = &application.envelope.application;
    let batch = batch_by_id_on(transaction, &material.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve replay lost its batch"))?;
    let review = review_by_batch_on(transaction, &material.batch_id)?
        .ok_or_else(|| anyhow::anyhow!("reference price curve replay lost its review"))?;
    let entries = entries_by_batch_on(transaction, &material.batch_id)?;
    if entries.len() != bindings.len() {
        bail!("reference price curve replay binding count changed");
    }
    for (entry, binding) in entries.iter().zip(bindings) {
        let prepared = prepare_snapshot_binding(
            transaction,
            &application.envelope.application_id,
            &batch,
            &review,
            entry,
            &material.applied_at,
        )?;
        if prepared.envelope != binding.envelope || prepared.binding_json != binding.binding_json {
            bail!("reference price curve replay is stale against the current active Offer");
        }
    }
    Ok(())
}

fn validate_input(input: &mut ApplyComputePlatformReferencePriceCurveBatch) -> Result<()> {
    for (value, label, max) in [
        (&input.batch_id, "application batch ID", 160),
        (&input.expected_review_id, "application review ID", 160),
        (
            &input.applied_by_admin_user_id,
            "applying administrator",
            160,
        ),
        (
            &input.idempotency_scope,
            "application idempotency scope",
            200,
        ),
        (&input.idempotency_key, "application idempotency key", 160),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [
        (&input.expected_batch_digest, "application batch digest"),
        (
            &input.expected_batch_material_digest,
            "application batch material digest",
        ),
        (&input.expected_review_digest, "application review digest"),
    ] {
        validate_digest(value, label)?;
    }
    if input.apply_confirmation != PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION {
        bail!("platform reference price curve application confirmation is not exact");
    }
    input.apply_note = input.apply_note.trim().to_string();
    validate_optional_note(&input.apply_note, "application note", 2_000)?;
    Ok(())
}
