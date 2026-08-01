//! Stable invocation response and privacy-preserving request metadata.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::open_commerce_model::OpenCommerceInvocation;

pub(crate) fn invocation_response(
    invocation: &OpenCommerceInvocation,
    replayed: bool,
) -> Result<Value> {
    Ok(json!({
        "schema": "open_commerce.invocation.v1",
        "invocation_id": invocation.id,
        "status": invocation.status,
        "replayed": replayed,
        "result": invocation.result,
        "error_code": invocation.error_code,
        "metering": {
            "units": invocation.units,
            "unit_price_micros": invocation.unit_price_micros,
            "amount_micros": invocation.amount_micros,
            "currency": invocation.currency,
            "settlement_status": invocation.settlement_status
        },
        "settlement_receipt": {
            "schema": "open_commerce.settlement_receipt.v1",
            "receipt_id": invocation.id,
            "billable_units": invocation.units,
            "amount_micros": invocation.amount_micros,
            "currency": invocation.currency,
            "status": invocation.settlement_status,
            "funds_moved": false
        }
    }))
}

pub(crate) fn request_digest(
    merchant_id: &str,
    capability_key: &str,
    requester_app_id: &str,
    input: &Value,
) -> Result<String> {
    let bytes = serde_json::to_vec(&json!({
        "merchant_id": merchant_id,
        "capability_key": capability_key,
        "requester_app_id": requester_app_id,
        "input": input
    }))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

pub(crate) fn request_shape(input: &Value) -> Result<Value> {
    let fields = input
        .as_object()
        .ok_or_else(|| anyhow!("调用输入必须是 JSON object"))?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    Ok(json!({
        "input_fields": fields,
        "input_bytes": serde_json::to_vec(input)?.len(),
        "contains_raw_values": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shape_never_contains_values() {
        let shape = request_shape(&json!({"phone": "secret", "count": 2})).unwrap();
        assert_eq!(shape["contains_raw_values"], false);
        assert_eq!(shape["input_fields"], json!(["count", "phone"]));
        assert!(!shape.to_string().contains("secret"));
    }
}
