use anyhow::{bail, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use crate::compute_federation::{
    market::ComputePriceSnapshot,
    platform_reference_price_curve::{
        ComputePlatformReferencePriceCurveBatchEnvelope,
        ComputePlatformReferencePriceCurveEntryIntent,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SAMPLE_COUNT,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_ID_PREFIX,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_KIND,
    },
};

use super::super::super::{
    review::{validate_digest, validate_exact, validate_optional_note},
    types::{
        canonical_nanos, StoredApplication, StoredSnapshotBinding, StoredSnapshotBindingMaterial,
        APPLICATION_STATUS_APPLIED, PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION,
        SNAPSHOT_BINDING_STATUS_REGISTERED,
    },
};

pub(super) fn snapshot_matches(
    snapshot: &ComputePriceSnapshot,
    binding: &StoredSnapshotBindingMaterial,
    entry: &ComputePlatformReferencePriceCurveEntryIntent,
    batch: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> bool {
    let quoted = DateTime::parse_from_rfc3339(&binding.quoted_at).ok();
    let expires = DateTime::parse_from_rfc3339(&binding.expires_at).ok();
    let valid_from = DateTime::parse_from_rfc3339(&batch.batch.valid_from).ok();
    let valid_until = DateTime::parse_from_rfc3339(&batch.batch.valid_until).ok();
    let observation_start = quoted
        .as_ref()
        .and_then(|value| value.checked_sub_signed(Duration::seconds(1)))
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Nanos, true));
    let components_match = snapshot.components.len() == entry.components.len()
        && snapshot
            .components
            .iter()
            .zip(&entry.components)
            .all(|(left, right)| {
                left.meter == right.meter
                    && left.unit_size == right.unit_size
                    && left.consumer_unit_price_micros == right.consumer_unit_price_micros
                    && left.provider_unit_price_micros == right.provider_unit_price_micros
                    && left.max_units == right.max_units
            });
    snapshot.snapshot_id == binding.snapshot_id
        && snapshot.snapshot_digest == binding.snapshot_digest
        && snapshot.quote_id == binding.quote_id
        && snapshot.pricing_mode == entry.pricing_mode
        && snapshot.sku.sku_id == entry.sku_id
        && snapshot.sku.sku_digest == entry.sku_digest
        && snapshot.provider_id == entry.provider_id
        && snapshot.offer_id == entry.offer_id
        && snapshot.offer_version == entry.offer_version
        && snapshot.offer_digest == entry.offer_digest
        && snapshot.delivery_window.binding.window_id == entry.delivery_window_id
        && snapshot.delivery_window.binding.window_digest == entry.delivery_window_digest
        && snapshot.currency == entry.currency
        && components_match
        && snapshot.fee_rules.is_empty()
        && snapshot.rounding_mode == batch.batch.rounding_mode
        && snapshot.consumer_max_amount_micros == entry.consumer_max_amount_micros
        && snapshot.provider_max_amount_micros == entry.provider_max_amount_micros
        && snapshot.price_source.source_kind == binding.source_kind
        && snapshot.price_source.source_id == binding.source_id
        && snapshot.price_source.source_version == binding.source_version
        && snapshot.price_source.source_digest == binding.source_digest
        && snapshot.price_source.sample_count == COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SAMPLE_COUNT
        && Some(snapshot.price_source.observation_window_start.as_str())
            == observation_start.as_deref()
        && snapshot.price_source.observation_window_end == binding.quoted_at
        && snapshot.trade_id.is_none()
        && snapshot.instrument_id == entry.instrument_id
        && snapshot.quoted_at == binding.quoted_at
        && snapshot.expires_at == binding.expires_at
        && matches!((quoted, expires, valid_from, valid_until),
            (Some(q), Some(e), Some(from), Some(until))
                if from <= q && q < e && e <= until
                    && e - q <= Duration::seconds(batch.batch.quote_ttl_seconds))
}

pub(super) fn validate_application_material(stored: &StoredApplication) -> Result<()> {
    let envelope = &stored.envelope;
    let app = &envelope.application;
    for (value, label, max) in [
        (&envelope.application_id, "stored application ID", 160),
        (&app.batch_id, "stored application batch ID", 160),
        (&app.review_id, "stored application review ID", 160),
        (&app.curve_id, "stored application curve ID", 160),
        (
            &app.submitted_by_admin_user_id,
            "stored submitting administrator",
            160,
        ),
        (
            &app.reviewed_by_admin_user_id,
            "stored reviewing administrator",
            160,
        ),
        (
            &app.applied_by_admin_user_id,
            "stored applying administrator",
            160,
        ),
        (
            &stored.idempotency_scope,
            "stored application idempotency scope",
            200,
        ),
        (
            &stored.idempotency_key,
            "stored application idempotency key",
            160,
        ),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [
        (&envelope.application_digest, "stored application digest"),
        (&app.batch_digest, "stored application batch digest"),
        (
            &app.batch_material_digest,
            "stored application batch material digest",
        ),
        (&app.review_digest, "stored application review digest"),
        (
            &app.binding_set_digest,
            "stored application binding-set digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    if !(1..=32).contains(&app.binding_digests.len()) {
        bail!("platform reference price curve application binding count is invalid");
    }
    for digest in &app.binding_digests {
        validate_digest(digest, "stored application binding digest")?;
    }
    validate_optional_note(&app.apply_note, "stored application note", 2_000)?;
    canonical_nanos(&app.applied_at)?;
    if app.curve_version <= 0
        || app.submitted_by_admin_user_id == app.reviewed_by_admin_user_id
        || app.apply_confirmation != PLATFORM_REFERENCE_PRICE_CURVE_APPLY_CONFIRMATION
        || app.status != APPLICATION_STATUS_APPLIED
    {
        bail!("platform reference price curve stored application authority is invalid");
    }
    Ok(())
}

pub(super) fn validate_binding_material(stored: &StoredSnapshotBinding) -> Result<()> {
    let envelope = &stored.envelope;
    let binding = &envelope.binding;
    for (value, label, max) in [
        (&envelope.binding_id, "stored binding ID", 200),
        (
            &binding.application_id,
            "stored binding application ID",
            160,
        ),
        (&binding.batch_id, "stored binding batch ID", 160),
        (&binding.review_id, "stored binding review ID", 160),
        (&binding.entry_id, "stored binding entry ID", 200),
        (&binding.entry_key, "stored binding entry key", 160),
        (&binding.curve_id, "stored binding curve ID", 160),
        (&binding.snapshot_id, "stored binding Snapshot ID", 200),
        (&binding.quote_id, "stored binding quote ID", 200),
    ] {
        validate_exact(value, label, max)?;
    }
    for (value, label) in [
        (&envelope.binding_digest, "stored binding digest"),
        (&binding.batch_digest, "stored binding batch digest"),
        (&binding.review_digest, "stored binding review digest"),
        (&binding.entry_digest, "stored binding entry digest"),
        (&binding.snapshot_digest, "stored binding Snapshot digest"),
        (&binding.source_digest, "stored binding source digest"),
    ] {
        validate_digest(value, label)?;
    }
    canonical_nanos(&binding.quoted_at)?;
    canonical_nanos(&binding.expires_at)?;
    if !(1..=32).contains(&binding.ordinal)
        || binding.curve_version <= 0
        || binding.source_kind != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_KIND
        || binding.source_id
            != format!(
                "{COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_ID_PREFIX}{}",
                binding.curve_id
            )
        || binding.source_version != binding.curve_version
        || binding.source_digest != binding.entry_digest
        || binding.quoted_at >= binding.expires_at
        || binding.status != SNAPSHOT_BINDING_STATUS_REGISTERED
    {
        bail!("platform reference price curve stored binding authority is invalid");
    }
    Ok(())
}
