//! Site-neutral, bounded private read snapshots. No exchange calculation or execution.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[path = "private_read_projection/storage.rs"]
pub(crate) mod storage;
#[path = "private_read_projection/strict_json.rs"]
mod strict_json;
#[path = "private_read_projection/transport.rs"]
pub(crate) mod transport;

pub(crate) const MAX_BYTES: usize = 256 * 1024;
pub(crate) const MAX_SOURCES: usize = 32;
pub(crate) const SOURCE: &str = "binance-futures-grid";
pub(crate) const SCOPE: &str = "grid.snapshot.read";

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Projection {
    pub schema: String,
    pub source: String,
    pub connection_id: String,
    pub generation: u64,
    pub revision: String,
    pub observed_at_ms: u64,
    pub fresh_until_ms: u64,
    pub status: String,
    pub payload: Value,
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis().min(i64::MAX as u128) as u64)
        .unwrap_or(0)
}
pub(crate) fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
pub(crate) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"-_.:".contains(&v))
}
pub(crate) fn parse(bytes: &[u8], at: u64) -> Result<Projection, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("projection_too_large");
    }
    let value = strict_json::parse(bytes)?;
    let projection: Projection = serde_json::from_value(value).map_err(|_| "projection_invalid")?;
    projection.validate(at)?;
    Ok(projection)
}
impl Projection {
    /// Sorted UTF-8 JSON, all wire fields except revision; no floating point business rules.
    pub(crate) fn expected_revision(&self) -> Result<String, &'static str> {
        let mut value = serde_json::to_value(self).map_err(|_| "projection_invalid")?;
        value
            .as_object_mut()
            .ok_or("projection_invalid")?
            .remove("revision");
        Ok(digest(canonical_json(&value)?.as_bytes()))
    }
    pub(crate) fn validate(&self, at: u64) -> Result<(), &'static str> {
        if self.schema != "yilong.private_read_projection.v1"
            || self.source != SOURCE
            || !identifier(&self.connection_id)
            || self.generation == 0
            || self.generation > i64::MAX as u64
            || self.observed_at_ms == 0
            || self.observed_at_ms > at.saturating_add(30_000)
            || self.observed_at_ms > i64::MAX as u64
            || self.fresh_until_ms < self.observed_at_ms
            || self.fresh_until_ms > self.observed_at_ms.saturating_add(300_000)
            || !matches!(self.status.as_str(), "fresh" | "stale" | "unavailable")
            || self.revision.len() != 64
            || !self
                .revision
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
            || self.expected_revision()? != self.revision
        {
            return Err("projection_invalid");
        }
        let object = self
            .payload
            .as_object()
            .ok_or("projection_payload_invalid")?;
        if object.len() != 2 || self.payload["schema"] != "yilong.quant.binance_grid_snapshot.v1" {
            return Err("projection_payload_invalid");
        }
        let bots = self.payload["bots"]
            .as_array()
            .ok_or("projection_payload_invalid")?;
        if bots.len() > 500
            || bots.iter().any(|row| {
                row.as_object().is_none_or(|r| r.len() != 3)
                    || !row["bot"].is_object()
                    || !row["provider_status"].is_string()
                    || !row["detail_available"].is_boolean()
            })
        {
            return Err("projection_payload_invalid");
        }
        let mut count = 0;
        validate_json(&self.payload, 0, &mut count)?;
        if serde_json::to_vec(self)
            .map_err(|_| "projection_invalid")?
            .len()
            > MAX_BYTES
        {
            return Err("projection_too_large");
        }
        Ok(())
    }
    pub(crate) fn projected(&self, projection_id: &str, _at: u64) -> Value {
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        value["projection_id"] = json!(projection_id);
        value
    }
}

/// Explicit sorting keeps the digest stable even if another crate enables preserve_order.
fn canonical_json(value: &Value) -> Result<String, &'static str> {
    match value {
        Value::Object(fields) => {
            let mut keys = fields.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let mut entries = Vec::with_capacity(keys.len());
            for key in keys {
                entries.push(format!(
                    "{}:{}",
                    serde_json::to_string(key).map_err(|_| "projection_invalid")?,
                    canonical_json(&fields[key])?
                ));
            }
            Ok(format!("{{{}}}", entries.join(",")))
        }
        Value::Array(items) => Ok(format!(
            "[{}]",
            items
                .iter()
                .map(canonical_json)
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        )),
        _ => serde_json::to_string(value).map_err(|_| "projection_invalid"),
    }
}
fn validate_json(value: &Value, depth: usize, count: &mut usize) -> Result<(), &'static str> {
    *count += 1;
    if depth > 24 || *count > 24_000 {
        return Err("projection_too_complex");
    }
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                let normalized: String = key
                    .chars()
                    .filter(|v| v.is_ascii_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect();
                if key.len() > 128
                    || matches!(
                        normalized.as_str(),
                        "cookie"
                            | "setcookie"
                            | "authorization"
                            | "password"
                            | "passwd"
                            | "secret"
                            | "apikey"
                            | "apisecret"
                            | "accesstoken"
                            | "refreshtoken"
                            | "csrftoken"
                            | "xsrftoken"
                            | "headers"
                            | "requestheaders"
                            | "responseheaders"
                            | "rawresponse"
                            | "token"
                            | "sessiontoken"
                            | "privatekey"
                            | "mnemonic"
                            | "seedphrase"
                            | "owneruserid"
                            | "ownerkey"
                    )
                {
                    return Err("projection_credentials_forbidden");
                }
                validate_json(child, depth + 1, count)?;
            }
        }
        Value::Array(items) => {
            for child in items {
                validate_json(child, depth + 1, count)?;
            }
        }
        Value::String(text) if text.len() > 8192 || text.starts_with("Bearer ") => {
            return Err("projection_value_invalid")
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
#[path = "private_read_projection/tests.rs"]
mod tests;
