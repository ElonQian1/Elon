use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat};

use super::{
    canonical::{
        canonical_compute_capacity_instrument_activation_json_and_digest,
        canonical_compute_capacity_instrument_json_and_digest,
        canonical_compute_capacity_instrument_offer_adoption_json_and_digest,
        canonical_compute_capacity_instrument_retirement_json_and_digest,
    },
    types::*,
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

pub(crate) fn validate_compute_capacity_instrument(
    instrument: &ComputeCapacityInstrument,
) -> Result<()> {
    validate_metadata(
        &instrument.schema,
        COMPUTE_CAPACITY_INSTRUMENT_SCHEMA,
        &instrument.canonicalization,
        &instrument.digest_algorithm,
    )?;
    validate_identifier(&instrument.instrument_id, "instrument ID", 160)?;
    if instrument.instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION {
        bail!("capacity-instrument revision is unsupported");
    }
    validate_digest(&instrument.instrument_digest, "instrument digest")?;
    validate_identifier(&instrument.sku_id, "SKU ID", 160)?;
    validate_digest(&instrument.sku_digest, "SKU digest")?;
    validate_delivery_window(&instrument.delivery_window)?;
    validate_contract_units(&instrument.contract_units)?;
    for (label, value) in [
        (
            "availability SLA tier",
            instrument.availability_sla_tier.as_str(),
        ),
        (
            "region or data zone",
            instrument.region_or_data_zone.as_str(),
        ),
        ("verification tier", instrument.verification_tier.as_str()),
        (
            "registration actor ID",
            instrument.registered_by_admin_user_id.as_str(),
        ),
        ("idempotency scope", instrument.idempotency_scope.as_str()),
        ("idempotency key", instrument.idempotency_key.as_str()),
    ] {
        validate_identifier(value, label, 200)?;
    }
    if instrument.settlement_currency != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY
        || instrument.settlement_unit != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT
        || instrument.confirmation != COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION
    {
        bail!("capacity-instrument settlement or confirmation is not exact");
    }
    validate_timestamps(&instrument.registered_at, &instrument.recorded_at)?;
    if canonical_compute_capacity_instrument_json_and_digest(instrument)?.1
        != instrument.instrument_digest
    {
        bail!("capacity-instrument digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_compute_capacity_instrument_activation_receipt(
    receipt: &ComputeCapacityInstrumentActivationReceipt,
) -> Result<()> {
    validate_lifecycle_common(
        &receipt.schema,
        COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_RECEIPT_SCHEMA,
        &receipt.activation_receipt_id,
        &receipt.activation_receipt_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
        &receipt.instrument_id,
        receipt.instrument_revision,
        &receipt.instrument_digest,
        &receipt.activated_by_admin_user_id,
        &receipt.confirmation,
        COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION,
        &receipt.idempotency_scope,
        &receipt.idempotency_key,
        &receipt.activated_at,
        &receipt.recorded_at,
    )?;
    if canonical_compute_capacity_instrument_activation_json_and_digest(receipt)?.1
        != receipt.activation_receipt_digest
    {
        bail!("capacity-instrument activation digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_compute_capacity_instrument_retirement_receipt(
    receipt: &ComputeCapacityInstrumentRetirementReceipt,
) -> Result<()> {
    validate_lifecycle_common(
        &receipt.schema,
        COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_RECEIPT_SCHEMA,
        &receipt.retirement_receipt_id,
        &receipt.retirement_receipt_digest,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
        &receipt.instrument_id,
        receipt.instrument_revision,
        &receipt.instrument_digest,
        &receipt.retired_by_admin_user_id,
        &receipt.confirmation,
        COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION,
        &receipt.idempotency_scope,
        &receipt.idempotency_key,
        &receipt.retired_at,
        &receipt.recorded_at,
    )?;
    validate_text(&receipt.reason, "retirement reason", 8, 2_000)?;
    if canonical_compute_capacity_instrument_retirement_json_and_digest(receipt)?.1
        != receipt.retirement_receipt_digest
    {
        bail!("capacity-instrument retirement digest is not canonical");
    }
    Ok(())
}

pub(crate) fn validate_compute_capacity_instrument_offer_adoption_receipt(
    receipt: &ComputeCapacityInstrumentOfferAdoptionReceipt,
) -> Result<()> {
    validate_metadata(
        &receipt.schema,
        COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_RECEIPT_SCHEMA,
        &receipt.canonicalization,
        &receipt.digest_algorithm,
    )?;
    for (label, value) in [
        ("adoption receipt ID", receipt.adoption_receipt_id.as_str()),
        ("instrument ID", receipt.instrument_id.as_str()),
        ("Offer ID", receipt.offer_id.as_str()),
        ("publication ID", receipt.publication_id.as_str()),
        (
            "adoption actor ID",
            receipt.adopted_by_admin_user_id.as_str(),
        ),
        ("idempotency scope", receipt.idempotency_scope.as_str()),
        ("idempotency key", receipt.idempotency_key.as_str()),
    ] {
        validate_identifier(value, label, 200)?;
    }
    for (label, value) in [
        (
            "adoption receipt digest",
            receipt.adoption_receipt_digest.as_str(),
        ),
        ("instrument digest", receipt.instrument_digest.as_str()),
        ("Offer digest", receipt.offer_digest.as_str()),
        ("publication digest", receipt.publication_digest.as_str()),
    ] {
        validate_digest(value, label)?;
    }
    if receipt.instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION
        || !(1..=MAX_SAFE_INTEGER).contains(&receipt.offer_version)
        || receipt.confirmation != COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION
    {
        bail!("capacity-instrument Offer adoption binding is invalid");
    }
    validate_timestamps(&receipt.adopted_at, &receipt.recorded_at)?;
    if canonical_compute_capacity_instrument_offer_adoption_json_and_digest(receipt)?.1
        != receipt.adoption_receipt_digest
    {
        bail!("capacity-instrument Offer adoption digest is not canonical");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_lifecycle_common(
    schema: &str,
    expected_schema: &str,
    receipt_id: &str,
    receipt_digest: &str,
    canonicalization: &str,
    digest_algorithm: &str,
    instrument_id: &str,
    instrument_revision: i64,
    instrument_digest: &str,
    actor_id: &str,
    confirmation: &str,
    expected_confirmation: &str,
    idempotency_scope: &str,
    idempotency_key: &str,
    occurred_at: &str,
    recorded_at: &str,
) -> Result<()> {
    validate_metadata(schema, expected_schema, canonicalization, digest_algorithm)?;
    for (label, value) in [
        ("receipt ID", receipt_id),
        ("instrument ID", instrument_id),
        ("lifecycle actor ID", actor_id),
        ("idempotency scope", idempotency_scope),
        ("idempotency key", idempotency_key),
    ] {
        validate_identifier(value, label, 200)?;
    }
    validate_digest(receipt_digest, "receipt digest")?;
    validate_digest(instrument_digest, "instrument digest")?;
    if instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION
        || confirmation != expected_confirmation
    {
        bail!("capacity-instrument lifecycle binding is invalid");
    }
    validate_timestamps(occurred_at, recorded_at)
}

fn validate_metadata(
    schema: &str,
    expected_schema: &str,
    canonicalization: &str,
    digest_algorithm: &str,
) -> Result<()> {
    if schema != expected_schema
        || canonicalization != COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION
        || digest_algorithm != COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM
    {
        bail!("capacity-instrument metadata is unsupported");
    }
    Ok(())
}

fn validate_delivery_window(window: &super::super::market::ComputeDeliveryWindow) -> Result<()> {
    validate_identifier(&window.binding.window_id, "delivery window ID", 160)?;
    validate_digest(&window.binding.window_digest, "delivery window digest")?;
    let starts = parse_utc(&window.starts_at_utc, "delivery window start")?;
    let ends = parse_utc(&window.ends_at_utc, "delivery window end")?;
    if starts >= ends {
        bail!("capacity-instrument delivery window must be a positive half-open interval");
    }
    Ok(())
}

fn validate_contract_units(units: &[ComputeCapacityInstrumentContractUnit]) -> Result<()> {
    if units.is_empty() || units.len() > 64 {
        bail!("capacity-instrument contract units must contain 1 to 64 meters");
    }
    let mut meters = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for unit in units {
        validate_identifier(&unit.meter, "contract-unit meter", 160)?;
        if !(1..=MAX_SAFE_INTEGER).contains(&unit.unit_size)
            || !(1..=MAX_SAFE_INTEGER).contains(&unit.quantity_units)
            || unit.quantity_units % unit.unit_size != 0
        {
            bail!("capacity-instrument contract unit quantities are invalid");
        }
        if !meters.insert(unit.meter.as_str())
            || previous.is_some_and(|value| value >= unit.meter.as_str())
        {
            bail!("capacity-instrument contract units must be unique and ordered by meter");
        }
        previous = Some(unit.meter.as_str());
    }
    Ok(())
}

fn parse_utc(value: &str, label: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) != value
    {
        bail!("{label} must use canonical UTC nanoseconds");
    }
    Ok(parsed)
}

fn validate_timestamps(first: &str, second: &str) -> Result<()> {
    if first != second {
        bail!("capacity-instrument timestamps must be identical");
    }
    parse_utc(first, "capacity-instrument timestamp").map(|_| ())
}

fn validate_identifier(value: &str, label: &str, maximum: usize) -> Result<()> {
    validate_text(value, label, 1, maximum)
}

fn validate_text(value: &str, label: &str, minimum: usize, maximum: usize) -> Result<()> {
    let length = value.chars().count();
    if value.trim() != value
        || !(minimum..=maximum).contains(&length)
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
