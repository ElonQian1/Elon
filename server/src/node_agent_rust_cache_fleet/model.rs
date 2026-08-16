use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ENVELOPE_SCHEMA: &str = "elon.rust_cache.fleet_envelope.v1";
const REPORT_SCHEMA: &str = "elon.rust_cache.fleet_report.v1";
const ACK_SCHEMA: &str = "elon.rust_cache.fleet_ack.v1";
const MAX_ENVELOPE_BYTES: usize = 2 * 1024 * 1024;
const MAX_REPORT_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub(super) struct ValidatedEnvelope {
    pub(super) envelope_id: String,
    pub(super) node_id: String,
    pub(super) report_sha256: String,
    pub(super) envelope_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct UploadReceipt {
    pub(super) schema: String,
    pub(super) accepted: bool,
    pub(super) deduplicated: bool,
    pub(super) envelope_id: String,
    pub(super) node_id: String,
    pub(super) report_sha256: String,
    pub(super) received_at: String,
    pub(super) destructive_actions_authorized: bool,
    pub(super) envelope_sha256: String,
}

#[derive(Debug, Serialize)]
pub(super) struct UploadFailure {
    pub(super) schema: &'static str,
    pub(super) category: &'static str,
    pub(super) code: &'static str,
    pub(super) http_status: Option<u16>,
    pub(super) destructive_actions_authorized: bool,
}

impl UploadFailure {
    pub(super) fn local(code: &'static str) -> Self {
        Self::new("local", code, None)
    }

    pub(super) fn network(code: &'static str) -> Self {
        Self::new("network", code, None)
    }

    pub(super) fn http(status: u16) -> Self {
        Self::new("http", "server-rejected", Some(status))
    }

    fn new(category: &'static str, code: &'static str, http_status: Option<u16>) -> Self {
        Self {
            schema: "elon.rust_cache.fleet_upload_failure.v1",
            category,
            code,
            http_status,
            destructive_actions_authorized: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetEnvelope {
    schema: String,
    envelope_id: String,
    created_at_utc: String,
    node_id: String,
    report: FleetEnvelopeReport,
    security: FleetEnvelopeSecurity,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetEnvelopeReport {
    schema: String,
    content_type: String,
    content_sha256: String,
    byte_length: u64,
    json: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetEnvelopeSecurity {
    receiver_must_authenticate_node: bool,
    destructive_actions_authorized: bool,
    absolute_paths_included: bool,
    secrets_included: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetAck {
    schema: String,
    accepted: bool,
    deduplicated: bool,
    envelope_id: String,
    node_id: String,
    report_sha256: String,
    received_at: String,
    destructive_actions_authorized: bool,
}

pub(super) fn validate_envelope(bytes: &[u8], expected_node_id: &str) -> Result<ValidatedEnvelope> {
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(anyhow!("invalid envelope size"));
    }
    let envelope: FleetEnvelope = serde_json::from_slice(bytes)?;
    if envelope.schema != ENVELOPE_SCHEMA
        || envelope.report.schema != REPORT_SCHEMA
        || envelope.report.content_type != "application/json"
        || envelope.node_id != expected_node_id
        || !valid_hex(&envelope.envelope_id, 32)
        || !valid_hex(&envelope.report.content_sha256, 64)
        || envelope.report.json.is_empty()
        || envelope.report.json.len() > MAX_REPORT_BYTES
        || envelope.report.byte_length != envelope.report.json.len() as u64
        || !envelope.security.receiver_must_authenticate_node
        || envelope.security.destructive_actions_authorized
        || envelope.security.absolute_paths_included
        || envelope.security.secrets_included
    {
        return Err(anyhow!("invalid envelope contract"));
    }
    chrono::DateTime::parse_from_rfc3339(&envelope.created_at_utc)?;
    let report_sha256 = hex::encode(Sha256::digest(envelope.report.json.as_bytes()));
    if report_sha256 != envelope.report.content_sha256 {
        return Err(anyhow!("report hash mismatch"));
    }
    Ok(ValidatedEnvelope {
        envelope_id: envelope.envelope_id,
        node_id: envelope.node_id,
        report_sha256,
        envelope_sha256: hex::encode(Sha256::digest(bytes)),
    })
}

pub(super) fn validate_ack(bytes: &[u8], envelope: &ValidatedEnvelope) -> Result<UploadReceipt> {
    let ack: FleetAck = serde_json::from_slice(bytes)?;
    if ack.schema != ACK_SCHEMA
        || !ack.accepted
        || ack.envelope_id != envelope.envelope_id
        || ack.node_id != envelope.node_id
        || ack.report_sha256 != envelope.report_sha256
        || ack.destructive_actions_authorized
    {
        return Err(anyhow!("fleet ACK does not match the uploaded envelope"));
    }
    chrono::DateTime::parse_from_rfc3339(&ack.received_at)?;
    Ok(UploadReceipt {
        schema: "elon.rust_cache.fleet_upload_receipt.v1".into(),
        accepted: ack.accepted,
        deduplicated: ack.deduplicated,
        envelope_id: ack.envelope_id,
        node_id: ack.node_id,
        report_sha256: ack.report_sha256,
        received_at: ack.received_at,
        destructive_actions_authorized: false,
        envelope_sha256: envelope.envelope_sha256.clone(),
    })
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value == value.to_ascii_lowercase()
        && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn envelope_and_ack_are_bound_by_identity_and_hash() {
        let report = "{\"schema\":\"elon.rust_cache.fleet_report.v1\"}";
        let report_hash = hex::encode(Sha256::digest(report.as_bytes()));
        let bytes = serde_json::to_vec(&json!({
            "schema": ENVELOPE_SCHEMA,
            "envelope_id": "a".repeat(32),
            "created_at_utc": "2026-08-16T00:00:00Z",
            "node_id": "node-a",
            "report": {
                "schema": REPORT_SCHEMA,
                "content_type": "application/json",
                "content_sha256": report_hash,
                "byte_length": report.len(),
                "json": report
            },
            "security": {
                "receiver_must_authenticate_node": true,
                "destructive_actions_authorized": false,
                "absolute_paths_included": false,
                "secrets_included": false
            }
        }))
        .unwrap();
        let envelope = validate_envelope(&bytes, "node-a").unwrap();
        let ack = serde_json::to_vec(&json!({
            "schema": ACK_SCHEMA,
            "accepted": true,
            "deduplicated": false,
            "envelope_id": envelope.envelope_id,
            "node_id": "node-a",
            "report_sha256": envelope.report_sha256,
            "received_at": "2026-08-16T00:00:01Z",
            "destructive_actions_authorized": false
        }))
        .unwrap();
        assert!(validate_ack(&ack, &envelope).unwrap().accepted);
    }
}
