use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, SecondsFormat};

use crate::compute_federation::market::{PRICING_MODE_CAPACITY_FUTURE, PRICING_MODE_SPOT};

use super::{
    canonical::{
        canonical_platform_reference_price_curve_batch_json_and_digest,
        canonical_platform_reference_price_curve_batch_material_digest,
        canonical_platform_reference_price_curve_entry_json_and_digest,
        canonical_platform_reference_price_curve_entry_set_digest,
    },
    types::{
        ComputePlatformReferencePriceCurveBatch, ComputePlatformReferencePriceCurveBatchEnvelope,
        ComputePlatformReferencePriceCurveComponent,
        ComputePlatformReferencePriceCurveEntryEnvelope,
        ComputePlatformReferencePriceCurveEntryIntent,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CURRENCY,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY,
        COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
const MIN_ENTRY_COUNT: usize = 1;
const MAX_ENTRY_COUNT: usize = 32;
const MIN_QUOTE_TTL_SECONDS: i64 = 30;
const MAX_QUOTE_TTL_SECONDS: i64 = 3_600;

pub(crate) fn validate_platform_reference_price_curve_batch_envelope(
    envelope: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> Result<()> {
    validate_identifier(&envelope.batch_id, "reference price curve batch ID", 160)?;
    validate_digest(&envelope.batch_digest, "reference price curve batch digest")?;
    validate_digest(
        &envelope.batch_material_digest,
        "reference price curve batch material digest",
    )?;
    if envelope.schema != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_BATCH_SCHEMA
        || envelope.canonicalization != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CANONICALIZATION
        || envelope.digest_algorithm != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_DIGEST_ALGORITHM
    {
        bail!("platform reference price curve batch metadata is not supported");
    }
    validate_platform_reference_price_curve_batch_material(&envelope.batch)?;
    validate_envelope_submission_time(&envelope.batch)?;
    let material_digest =
        canonical_platform_reference_price_curve_batch_material_digest(&envelope.batch)?;
    if material_digest != envelope.batch_material_digest {
        bail!("platform reference price curve batch material digest is not canonical");
    }
    let (_, digest) = canonical_platform_reference_price_curve_batch_json_and_digest(envelope)?;
    if digest != envelope.batch_digest {
        bail!("platform reference price curve batch digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_platform_reference_price_curve_batch_material(
    batch: &ComputePlatformReferencePriceCurveBatch,
) -> Result<()> {
    validate_identifier(
        &batch.submitted_by_admin_user_id,
        "reference price curve submitting administrator",
        160,
    )?;
    validate_identifier(&batch.curve_id, "reference price curve ID", 160)?;
    validate_safe_positive(batch.curve_version, "reference price curve version")?;
    if batch.methodology_kind != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_METHODOLOGY {
        bail!("platform reference price curve must use source-only fallback methodology");
    }
    if !(MIN_QUOTE_TTL_SECONDS..=MAX_QUOTE_TTL_SECONDS).contains(&batch.quote_ttl_seconds) {
        bail!("platform reference price curve quote TTL is outside the supported range");
    }
    validate_times(batch)?;
    if batch.rounding_mode != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ROUNDING_MODE {
        bail!("platform reference price curve must use half_even rounding");
    }
    validate_entries(batch)?;
    validate_digest(
        &batch.entry_set_digest,
        "reference price curve entry set digest",
    )?;
    let entry_set_digest =
        canonical_platform_reference_price_curve_entry_set_digest(&batch.entries)?;
    if entry_set_digest != batch.entry_set_digest {
        bail!("platform reference price curve entry set digest is not canonical");
    }
    validate_identifier(
        &batch.idempotency_key,
        "reference price curve idempotency key",
        160,
    )?;
    if batch.confirmation != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CONFIRMATION {
        bail!("platform reference price curve confirmation is not exact");
    }
    validate_text(
        &batch.submission_note,
        "reference price curve submission note",
        2_000,
        true,
    )?;
    let _ = canonical_platform_reference_price_curve_batch_material_digest(batch)?;
    Ok(())
}

pub(crate) fn validate_platform_reference_price_curve_entry_envelope(
    envelope: &ComputePlatformReferencePriceCurveEntryEnvelope,
) -> Result<()> {
    if envelope.schema != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_ENTRY_SCHEMA {
        bail!("platform reference price curve entry schema is not supported");
    }
    validate_identifier(&envelope.batch_id, "reference price curve batch ID", 160)?;
    validate_digest(&envelope.batch_digest, "reference price curve batch digest")?;
    validate_identifier(&envelope.entry_id, "reference price curve entry ID", 200)?;
    validate_digest(&envelope.entry_digest, "reference price curve entry digest")?;
    if !(1..=MAX_ENTRY_COUNT as i64).contains(&envelope.ordinal) {
        bail!("platform reference price curve entry ordinal is invalid");
    }
    validate_platform_reference_price_curve_entry_intent(&envelope.entry)?;
    let (_, digest) = canonical_platform_reference_price_curve_entry_json_and_digest(envelope)?;
    if digest != envelope.entry_digest {
        bail!("platform reference price curve entry digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_platform_reference_price_curve_entry_against_batch(
    entry: &ComputePlatformReferencePriceCurveEntryEnvelope,
    batch: &ComputePlatformReferencePriceCurveBatchEnvelope,
) -> Result<()> {
    validate_platform_reference_price_curve_entry_envelope(entry)?;
    validate_platform_reference_price_curve_batch_envelope(batch)?;
    let index = usize::try_from(entry.ordinal - 1).context("entry ordinal conversion failed")?;
    if entry.batch_id != batch.batch_id
        || entry.batch_digest != batch.batch_digest
        || batch.batch.entries.get(index) != Some(&entry.entry)
    {
        bail!("platform reference price curve entry is not bound to the exact batch ordinal");
    }
    Ok(())
}

pub(crate) fn validate_platform_reference_price_curve_entry_intent(
    entry: &ComputePlatformReferencePriceCurveEntryIntent,
) -> Result<()> {
    for (value, label, limit) in [
        (&entry.entry_key, "reference price curve entry key", 160),
        (&entry.provider_id, "reference price curve Provider ID", 160),
        (&entry.offer_id, "reference price curve Offer ID", 200),
        (&entry.sku_id, "reference price curve SKU ID", 200),
        (
            &entry.delivery_window_id,
            "reference price curve delivery window ID",
            200,
        ),
    ] {
        validate_identifier(value, label, limit)?;
    }
    validate_safe_positive(entry.offer_version, "reference price curve Offer version")?;
    for (value, label) in [
        (&entry.offer_digest, "reference price curve Offer digest"),
        (&entry.sku_digest, "reference price curve SKU digest"),
        (
            &entry.delivery_window_digest,
            "reference price curve delivery window digest",
        ),
    ] {
        validate_digest(value, label)?;
    }
    if !matches!(
        entry.pricing_mode.as_str(),
        PRICING_MODE_SPOT | PRICING_MODE_CAPACITY_FUTURE
    ) {
        bail!("platform reference price curve pricing mode is not source-only V1");
    }
    if entry.currency != COMPUTE_PLATFORM_REFERENCE_PRICE_CURVE_CURRENCY {
        bail!("platform reference price curve must use CNY");
    }
    validate_optional_curve(entry)?;
    validate_instrument(entry)?;
    validate_components_and_limits(entry)?;
    if !entry.fee_rules.is_empty() {
        bail!("platform reference price curve V1 fee rules must be empty");
    }
    Ok(())
}

fn validate_times(batch: &ComputePlatformReferencePriceCurveBatch) -> Result<()> {
    let valid_from = parse_timestamp(&batch.valid_from, "reference price valid_from")?;
    let valid_until = parse_timestamp(&batch.valid_until, "reference price valid_until")?;
    if valid_from >= valid_until {
        bail!("platform reference price curve time order is invalid");
    }
    let minimum_expiry = valid_from
        .checked_add_signed(Duration::seconds(batch.quote_ttl_seconds))
        .context("reference price curve TTL overflows")?;
    if minimum_expiry > valid_until {
        bail!("platform reference price curve validity cannot fit one approved quote TTL");
    }
    Ok(())
}

fn validate_envelope_submission_time(
    batch: &ComputePlatformReferencePriceCurveBatch,
) -> Result<()> {
    let submitted_at = parse_timestamp(&batch.submitted_at, "reference price submitted_at")?;
    let valid_from = parse_timestamp(&batch.valid_from, "reference price valid_from")?;
    if submitted_at > valid_from {
        bail!("platform reference price curve submission time order is invalid");
    }
    Ok(())
}

fn validate_entries(batch: &ComputePlatformReferencePriceCurveBatch) -> Result<()> {
    if !(MIN_ENTRY_COUNT..=MAX_ENTRY_COUNT).contains(&batch.entries.len()) {
        bail!("platform reference price curve requires 1 to 32 entries");
    }
    let mut previous_key: Option<&str> = None;
    let mut offer_windows = BTreeSet::new();
    for entry in &batch.entries {
        validate_platform_reference_price_curve_entry_intent(entry)?;
        if previous_key.is_some_and(|previous| previous >= entry.entry_key.as_str()) {
            bail!("platform reference price curve entries must be strictly ordered by key");
        }
        previous_key = Some(&entry.entry_key);
        if !offer_windows.insert((
            entry.offer_id.as_str(),
            entry.offer_version,
            entry.delivery_window_id.as_str(),
        )) {
            bail!("platform reference price curve cannot repeat an Offer delivery window");
        }
    }
    Ok(())
}

fn validate_optional_curve(entry: &ComputePlatformReferencePriceCurveEntryIntent) -> Result<()> {
    match (&entry.offer_curve_id, entry.offer_curve_version) {
        (None, None) => Ok(()),
        (Some(curve_id), Some(version)) => {
            validate_identifier(curve_id, "reference price Offer curve ID", 160)?;
            validate_safe_positive(version, "reference price Offer curve version")
        }
        _ => bail!("platform reference price Offer curve ID and version must appear together"),
    }
}

fn validate_instrument(entry: &ComputePlatformReferencePriceCurveEntryIntent) -> Result<()> {
    match (entry.pricing_mode.as_str(), entry.instrument_id.as_deref()) {
        (PRICING_MODE_SPOT, None) => Ok(()),
        (PRICING_MODE_CAPACITY_FUTURE, Some(instrument_id)) => {
            validate_identifier(instrument_id, "reference price instrument ID", 160)
        }
        (PRICING_MODE_SPOT, Some(_)) => {
            bail!("spot reference price entry cannot claim a capacity instrument")
        }
        (PRICING_MODE_CAPACITY_FUTURE, None) => {
            bail!("capacity_future reference price entry requires an instrument")
        }
        _ => bail!("platform reference price curve pricing mode is unsupported"),
    }
}

fn validate_components_and_limits(
    entry: &ComputePlatformReferencePriceCurveEntryIntent,
) -> Result<()> {
    if entry.components.is_empty() || entry.components.len() > 32 {
        bail!("platform reference price curve requires 1 to 32 price components");
    }
    let mut meters = BTreeSet::new();
    let mut consumer_total = 0_i128;
    let mut provider_total = 0_i128;
    for component in &entry.components {
        validate_component(component)?;
        if !meters.insert(component.meter.as_str()) {
            bail!("platform reference price component meters must be unique");
        }
        let quanta = i128::from(component.max_units / component.unit_size);
        consumer_total = consumer_total
            .checked_add(quanta * i128::from(component.consumer_unit_price_micros))
            .context("reference price consumer component total overflows")?;
        provider_total = provider_total
            .checked_add(quanta * i128::from(component.provider_unit_price_micros))
            .context("reference price Provider component total overflows")?;
    }
    if !(0..=MAX_SAFE_INTEGER).contains(&entry.consumer_max_amount_micros)
        || !(0..=MAX_SAFE_INTEGER).contains(&entry.provider_max_amount_micros)
        || entry.provider_max_amount_micros > entry.consumer_max_amount_micros
        || i128::from(entry.consumer_max_amount_micros) < consumer_total
        || i128::from(entry.provider_max_amount_micros) < provider_total
    {
        bail!("platform reference price maximum amounts are invalid");
    }
    Ok(())
}

fn validate_component(component: &ComputePlatformReferencePriceCurveComponent) -> Result<()> {
    validate_identifier(&component.meter, "reference price component meter", 80)?;
    for (value, label) in [
        (component.unit_size, "reference price component unit size"),
        (component.max_units, "reference price component max units"),
    ] {
        validate_safe_positive(value, label)?;
    }
    if component.max_units % component.unit_size != 0
        || !(0..=MAX_SAFE_INTEGER).contains(&component.consumer_unit_price_micros)
        || !(0..=MAX_SAFE_INTEGER).contains(&component.provider_unit_price_micros)
        || component.provider_unit_price_micros > component.consumer_unit_price_micros
    {
        bail!("platform reference price component integer contract is invalid");
    }
    Ok(())
}

fn validate_safe_positive(value: i64, label: &str) -> Result<()> {
    if !(1..=MAX_SAFE_INTEGER).contains(&value) {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str, limit: usize) -> Result<()> {
    validate_text(value, label, limit, false)
}

fn validate_text(value: &str, label: &str, limit: usize, allow_empty: bool) -> Result<()> {
    if (!allow_empty && value.is_empty())
        || value.trim() != value
        || value.len() > limit
        || value.chars().any(char::is_control)
    {
        bail!("{label} is invalid");
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{label} must be a lowercase SHA-256 digest");
    }
    Ok(())
}

fn parse_timestamp(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| anyhow::anyhow!("{label} is not RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}
