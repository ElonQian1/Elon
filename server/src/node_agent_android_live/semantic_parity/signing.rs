use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), canonical(value)))
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        _ => value.clone(),
    }
}
pub(crate) fn sign(payload: &Value, key: &[u8]) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).context("SEMANTIC_RECEIPT_KEY_INVALID")?;
    mac.update(b"elon.semantic-parity.receipt.v1\0");
    mac.update(&serde_json::to_vec(&canonical(payload))?);
    Ok(hex::encode(mac.finalize().into_bytes()))
}
pub(crate) fn verify(payload: &Value, signature: &str, key: &[u8]) -> Result<()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).context("SEMANTIC_RECEIPT_KEY_INVALID")?;
    mac.update(b"elon.semantic-parity.receipt.v1\0");
    mac.update(&serde_json::to_vec(&canonical(payload))?);
    mac.verify_slice(&hex::decode(signature).context("SEMANTIC_RECEIPT_SIGNATURE_INVALID")?)
        .context("SEMANTIC_RECEIPT_SIGNATURE_INVALID")
}
