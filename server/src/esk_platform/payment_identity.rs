//! Byte-compatible payment identities shared with esk-paid-reconciliation/identity.js.
//! These hashes identify submitted facts; they do not authenticate a payment.

use anyhow::Result;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::model::{PaymentSource, PlatformError};

pub(super) fn bounded_ascii(value: &str, maximum: usize, lowercase: bool) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_digit()
                || byte.is_ascii_lowercase()
                || (!lowercase && byte.is_ascii_uppercase())
                || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
}

pub(super) fn normalized_source(source: &PaymentSource) -> Result<PaymentSource> {
    if !bounded_ascii(&source.namespace, 96, true)
        || !bounded_ascii(&source.network, 96, true)
        || source.asset_symbol != "USDT"
        || !bounded_ascii(&source.asset_reference, 160, false)
        || source.decimals > 18
        || !matches!(source.reference_format.as_str(), "hex32" | "opaque")
    {
        return Err(PlatformError::InvalidInput.into());
    }
    let mut normalized = source.clone();
    let raw = source.asset_reference.as_str();
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        if !hex.is_empty() && hex.len() <= 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            normalized.asset_reference = format!("0x{:0>64}", hex.to_ascii_lowercase());
        }
    }
    Ok(normalized)
}

/// Stable sorted-key JSON, independent of serde_json's preserve_order feature.
fn canonical(value: &Value) -> Result<String> {
    Ok(match value {
        Value::Object(object) => {
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort_unstable();
            let mut members = Vec::with_capacity(keys.len());
            for key in keys {
                let name = serde_json::to_string(key).map_err(|_| PlatformError::InvalidInput)?;
                members.push(format!("{name}:{}", canonical(&object[key])?));
            }
            format!("{{{}}}", members.join(","))
        }
        Value::Array(values) => {
            let members = values.iter().map(canonical).collect::<Result<Vec<_>>>()?;
            format!("[{}]", members.join(","))
        }
        _ => serde_json::to_string(value).map_err(|_| PlatformError::InvalidInput)?,
    })
}

pub(super) fn fingerprint(value: &Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(canonical(value)?.as_bytes())))
}

pub(crate) fn source_fingerprint(source: &PaymentSource) -> Result<String> {
    let source = normalized_source(source)?;
    fingerprint(&json!({
        "schema": "yilong.payment_source.v1",
        "namespace": source.namespace,
        "network": source.network,
        "asset_symbol": source.asset_symbol,
        "asset_reference": source.asset_reference,
        "decimals": source.decimals,
        "reference_format": source.reference_format,
    }))
}

pub(crate) fn payment_key(
    source: &PaymentSource,
    external_reference: &str,
    transfer_index: u32,
) -> Result<String> {
    let source = normalized_source(source)?;
    if transfer_index > i32::MAX as u32 {
        return Err(PlatformError::InvalidInput.into());
    }
    let reference = if source.reference_format == "hex32" {
        let hex = external_reference
            .strip_prefix("0x")
            .or_else(|| external_reference.strip_prefix("0X"))
            .unwrap_or(external_reference);
        if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PlatformError::InvalidInput.into());
        }
        hex.to_ascii_lowercase()
    } else {
        if !bounded_ascii(external_reference, 128, false) {
            return Err(PlatformError::InvalidInput.into());
        }
        external_reference.to_owned()
    };
    fingerprint(&json!({
        "schema": "yilong.payment_identity.v1",
        "namespace": source.namespace,
        "network": source.network,
        "asset_symbol": source.asset_symbol,
        "asset_reference": source.asset_reference,
        "external_payment_reference": reference,
        "transfer_index": transfer_index,
    }))
}
