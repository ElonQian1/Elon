use anyhow::{bail, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeCapacityInstrument, ComputeCapacityInstrumentActivationReceipt,
    ComputeCapacityInstrumentOfferAdoptionReceipt, ComputeCapacityInstrumentRetirementReceipt,
};

const MAX_CAPACITY_INSTRUMENT_JSON_BYTES: usize = 256 * 1024;
const INSTRUMENT_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-INSTRUMENT-V1";
const ACTIVATION_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-INSTRUMENT-ACTIVATION-RECEIPT-V1";
const RETIREMENT_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-INSTRUMENT-RETIREMENT-RECEIPT-V1";
const OFFER_ADOPTION_DOMAIN: &[u8] = b"ELON-COMPUTE-CAPACITY-INSTRUMENT-OFFER-ADOPTION-RECEIPT-V1";

pub(crate) fn canonical_compute_capacity_instrument_json_and_digest(
    instrument: &ComputeCapacityInstrument,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(instrument, "instrument_digest", INSTRUMENT_DOMAIN)
}

pub(crate) fn canonical_compute_capacity_instrument_activation_json_and_digest(
    receipt: &ComputeCapacityInstrumentActivationReceipt,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(receipt, "activation_receipt_digest", ACTIVATION_DOMAIN)
}

pub(crate) fn canonical_compute_capacity_instrument_retirement_json_and_digest(
    receipt: &ComputeCapacityInstrumentRetirementReceipt,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(receipt, "retirement_receipt_digest", RETIREMENT_DOMAIN)
}

pub(crate) fn canonical_compute_capacity_instrument_offer_adoption_json_and_digest(
    receipt: &ComputeCapacityInstrumentOfferAdoptionReceipt,
) -> Result<(String, String)> {
    canonical_envelope_json_and_digest(receipt, "adoption_receipt_digest", OFFER_ADOPTION_DOMAIN)
}

fn canonical_envelope_json_and_digest<T: Serialize>(
    envelope: &T,
    digest_field: &str,
    domain: &[u8],
) -> Result<(String, String)> {
    let value = serde_json::to_value(envelope)?;
    let mut projection = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("capacity-instrument envelope must be an object"))?
        .clone();
    if projection
        .insert(
            digest_field.to_string(),
            serde_json::Value::String(String::new()),
        )
        .is_none()
    {
        bail!("capacity-instrument envelope lacks digest field");
    }
    let digest = domain_digest(domain, &projection)?;
    let json = canonical_json(envelope)?;
    Ok((json, digest))
}

fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    canonical_compute_plugin_ijson_and_sha256(value, MAX_CAPACITY_INSTRUMENT_JSON_BYTES)
        .map(|(json, _)| json)
}

fn domain_digest<T: Serialize + ?Sized>(domain: &[u8], value: &T) -> Result<String> {
    let json = canonical_json(value)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
