use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, FixedOffset, SecondsFormat};
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::{
        market::{
            ComputeDeliveryWindow, ComputePriceSnapshot, ComputePriceSource,
            COMPUTE_PRICE_SNAPSHOT_SCHEMA,
        },
        offer::{ComputeOffer, OFFER_STATUS_ACTIVE},
        platform_reference_price_curve::{
            ComputePlatformReferencePriceCurveEntryIntent,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SAMPLE_COUNT,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_ID_PREFIX,
            COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_KIND,
        },
        provider::PROVIDER_STATUS_ACTIVE,
    },
    store::{
        compute_offer_registry::current_registered_offer_on,
        compute_price_snapshot_validation::compute_price_snapshot_digest,
        compute_provider_registry::current_registered_provider_on,
    },
};

use super::{
    canonical::canonical_snapshot_binding_json_and_digest,
    types::{
        StoredBatch, StoredEntry, StoredReview, StoredSnapshotBindingEnvelope,
        StoredSnapshotBindingMaterial, PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA,
        SNAPSHOT_BINDING_STATUS_REGISTERED,
    },
};

pub(super) struct PreparedSnapshotBinding {
    pub(super) snapshot: ComputePriceSnapshot,
    pub(super) envelope: StoredSnapshotBindingEnvelope,
    pub(super) binding_json: String,
}

pub(super) fn prepare_snapshot_binding(
    conn: &Connection,
    application_id: &str,
    batch: &StoredBatch,
    review: &StoredReview,
    entry: &StoredEntry,
    applied_at: &str,
) -> Result<PreparedSnapshotBinding> {
    let provider = current_registered_provider_on(conn, &entry.envelope.entry.provider_id)?
        .ok_or_else(|| anyhow!("platform reference price curve Provider is absent"))?;
    if provider.provider.status != PROVIDER_STATUS_ACTIVE {
        bail!("platform reference price curve requires a current active Provider");
    }
    let offer_receipt = current_registered_offer_on(conn, &entry.envelope.entry.offer_id)?
        .ok_or_else(|| anyhow!("platform reference price curve Offer is absent"))?;
    if offer_receipt.provider_policy_revision != provider.provider.policy_revision
        || offer_receipt.provider_digest != provider.provider_digest
    {
        bail!("platform reference price curve Offer is stale against the current Provider");
    }
    let offer = offer_receipt.offer;
    let window = exact_offer_window(&offer, &entry.envelope.entry)?;
    ensure_exact_offer(&offer, &entry.envelope.entry)?;

    let quoted_at = parse_time(applied_at, "application time")?;
    let expires_at = capped_expiry(batch, &offer, &quoted_at)?;
    let observation_start = quoted_at
        .clone()
        .checked_sub_signed(Duration::seconds(1))
        .context("platform reference price curve observation start overflows")?;
    let source_id = format!(
        "{COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_ID_PREFIX}{}",
        batch.envelope.batch.curve_id
    );
    let snapshot_id = deterministic_id("platform_reference_price_snapshot", batch, entry)?;
    let quote_id = deterministic_id("platform_reference_price_quote", batch, entry)?;
    let mut snapshot = ComputePriceSnapshot {
        schema: COMPUTE_PRICE_SNAPSHOT_SCHEMA.to_string(),
        snapshot_id,
        snapshot_digest: String::new(),
        quote_id,
        pricing_mode: offer.price_terms.pricing_mode.clone(),
        sku: offer.sku.clone(),
        provider_id: offer.provider_id.clone(),
        offer_id: offer.offer_id.clone(),
        offer_version: offer.offer_version,
        offer_digest: offer.offer_digest.clone(),
        delivery_window: window,
        currency: offer.price_terms.currency.clone(),
        components: offer.price_terms.components.clone(),
        fee_rules: offer.price_terms.fee_rules.clone(),
        consumer_max_amount_micros: entry.envelope.entry.consumer_max_amount_micros,
        provider_max_amount_micros: entry.envelope.entry.provider_max_amount_micros,
        price_source: ComputePriceSource {
            source_kind: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SOURCE_KIND.to_string(),
            source_id: source_id.clone(),
            source_version: batch.envelope.batch.curve_version,
            observation_window_start: observation_start.to_rfc3339_opts(SecondsFormat::Nanos, true),
            observation_window_end: applied_at.to_string(),
            sample_count: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_SAMPLE_COUNT,
            source_digest: entry.envelope.entry_digest.clone(),
        },
        trade_id: None,
        instrument_id: offer.price_terms.instrument_id.clone(),
        rounding_mode: batch.envelope.batch.rounding_mode.clone(),
        quoted_at: applied_at.to_string(),
        expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Nanos, true),
    };
    snapshot.snapshot_digest = compute_price_snapshot_digest(&snapshot)?;

    let mut envelope = StoredSnapshotBindingEnvelope {
        schema: PLATFORM_REFERENCE_PRICE_CURVE_SNAPSHOT_BINDING_SCHEMA.to_string(),
        binding_id: deterministic_id("platform_reference_price_binding", batch, entry)?,
        binding_digest: String::new(),
        canonicalization: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION.to_string(),
        digest_algorithm: COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM.to_string(),
        binding: StoredSnapshotBindingMaterial {
            application_id: application_id.to_string(),
            batch_id: batch.envelope.batch_id.clone(),
            batch_digest: batch.envelope.batch_digest.clone(),
            review_id: review.envelope.review_id.clone(),
            review_digest: review.envelope.review_digest.clone(),
            entry_id: entry.envelope.entry_id.clone(),
            entry_digest: entry.envelope.entry_digest.clone(),
            ordinal: entry.envelope.ordinal,
            entry_key: entry.envelope.entry.entry_key.clone(),
            curve_id: batch.envelope.batch.curve_id.clone(),
            curve_version: batch.envelope.batch.curve_version,
            snapshot_id: snapshot.snapshot_id.clone(),
            snapshot_digest: snapshot.snapshot_digest.clone(),
            quote_id: snapshot.quote_id.clone(),
            source_kind: snapshot.price_source.source_kind.clone(),
            source_id,
            source_version: snapshot.price_source.source_version,
            source_digest: snapshot.price_source.source_digest.clone(),
            quoted_at: snapshot.quoted_at.clone(),
            expires_at: snapshot.expires_at.clone(),
            status: SNAPSHOT_BINDING_STATUS_REGISTERED.to_string(),
        },
    };
    let (_, digest) = canonical_snapshot_binding_json_and_digest(&envelope)?;
    envelope.binding_digest = digest;
    let (binding_json, digest) = canonical_snapshot_binding_json_and_digest(&envelope)?;
    if digest != envelope.binding_digest {
        bail!("platform reference price curve binding digest changed before persistence");
    }
    Ok(PreparedSnapshotBinding {
        snapshot,
        envelope,
        binding_json,
    })
}

fn ensure_exact_offer(
    offer: &ComputeOffer,
    entry: &ComputePlatformReferencePriceCurveEntryIntent,
) -> Result<()> {
    let components_match = offer.price_terms.components.len() == entry.components.len()
        && offer
            .price_terms
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
    if offer.status != OFFER_STATUS_ACTIVE
        || offer.provider_id != entry.provider_id
        || offer.offer_id != entry.offer_id
        || offer.offer_version != entry.offer_version
        || offer.offer_digest != entry.offer_digest
        || offer.sku.sku_id != entry.sku_id
        || offer.sku.sku_digest != entry.sku_digest
        || offer.price_terms.pricing_mode != entry.pricing_mode
        || offer.price_terms.currency != entry.currency
        || offer.price_terms.curve_id != entry.offer_curve_id
        || offer.price_terms.curve_version != entry.offer_curve_version
        || offer.price_terms.instrument_id != entry.instrument_id
        || !components_match
        || !offer.price_terms.fee_rules.is_empty()
        || !entry.fee_rules.is_empty()
    {
        bail!("platform reference price curve entry no longer matches the exact active Offer");
    }
    Ok(())
}

fn exact_offer_window(
    offer: &ComputeOffer,
    entry: &ComputePlatformReferencePriceCurveEntryIntent,
) -> Result<ComputeDeliveryWindow> {
    offer
        .delivery_windows
        .iter()
        .find(|window| {
            window.binding.window_id == entry.delivery_window_id
                && window.binding.window_digest == entry.delivery_window_digest
        })
        .cloned()
        .ok_or_else(|| anyhow!("platform reference price curve delivery window is stale"))
}

fn capped_expiry(
    batch: &StoredBatch,
    offer: &ComputeOffer,
    quoted_at: &DateTime<FixedOffset>,
) -> Result<DateTime<FixedOffset>> {
    let valid_from = parse_time(&batch.envelope.batch.valid_from, "batch valid_from")?;
    let mut expiry = quoted_at
        .clone()
        .checked_add_signed(Duration::seconds(batch.envelope.batch.quote_ttl_seconds))
        .context("platform reference price curve TTL overflows")?;
    for value in [
        parse_time(&batch.envelope.batch.valid_until, "batch valid_until")?,
        parse_time(&offer.valid_until, "Offer valid_until")?,
        parse_time(&offer.price_terms.valid_until, "price terms valid_until")?,
    ] {
        if value < expiry {
            expiry = value;
        }
    }
    if quoted_at < &valid_from || &expiry <= quoted_at {
        bail!("platform reference price curve is not currently valid for a Snapshot");
    }
    Ok(expiry)
}

fn deterministic_id(purpose: &str, batch: &StoredBatch, entry: &StoredEntry) -> Result<String> {
    #[derive(Serialize)]
    struct Material<'a> {
        purpose: &'a str,
        batch_id: &'a str,
        batch_digest: &'a str,
        entry_id: &'a str,
        entry_digest: &'a str,
    }
    let material = Material {
        purpose,
        batch_id: &batch.envelope.batch_id,
        batch_digest: &batch.envelope.batch_digest,
        entry_id: &entry.envelope.entry_id,
        entry_digest: &entry.envelope.entry_digest,
    };
    Ok(format!(
        "{purpose}_{}",
        hex::encode(Sha256::digest(serde_json::to_vec(&material)?))
    ))
}

fn parse_time(value: &str, label: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("platform reference price curve {label} is not RFC3339"))
}
