use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const ENV_KEYRING: &str = "YILONG_QUANT_ESK_RECEIPT_KEYRING_JSON";
const KEYRING_SCHEMA: &str = "yilong.quant.esk_allocation_receipt_keyring.v1";
const RECEIPT_SCHEMA: &str = "yilong.quant.esk_allocation_receipt.v1";
const TOKEN_PREFIX: &str = "yqar1";
const MAX_KEYRING_BYTES: usize = 16 * 1024;
const MAX_TOKEN_BYTES: usize = 8192;
const MAX_PAYLOAD_BYTES: usize = 4096;

#[derive(Debug)]
pub(crate) enum ReceiptVerifierConfigError {
    Disabled,
    Invalid,
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedEskAllocationReceipt {
    pub receipt_id: String,
    pub key_id: String,
    pub event: String,
    pub binding_id: String,
    pub request_id: String,
    pub participant_ref: String,
    pub amount: String,
    pub amount_base_units: String,
    pub authorization_id: String,
    pub binding_revision: i64,
    pub previous_receipt_digest: Option<String>,
    pub occurred_at_unix: i64,
    pub receipt_digest: String,
}

pub(crate) struct EskAllocationReceiptVerifier {
    keys: HashMap<String, TrustedKey>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum KeyStatus {
    Active,
    Retiring,
    Revoked,
}

struct TrustedKey {
    public_key: [u8; 32],
    status: KeyStatus,
    not_before: Option<i64>,
    not_after: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyringConfig {
    schema: String,
    keys: Vec<KeyConfig>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyConfig {
    key_id: String,
    public_key_base64url: String,
    status: KeyStatus,
    issued_at_not_before_unix: Option<i64>,
    issued_at_not_after_unix: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    schema: String,
    receipt_id: String,
    issuer: String,
    audience: String,
    project_id: String,
    key_id: String,
    event: String,
    binding_id: String,
    binding_status: String,
    request_id: String,
    participant_ref: String,
    amount: String,
    amount_base_units: String,
    authorization_id: String,
    binding_revision: i64,
    previous_receipt_digest: Option<String>,
    occurred_at_unix: i64,
    simulated: bool,
    funds_moved: bool,
    position_kind: String,
    quant_units_issued: bool,
    nav_participation: bool,
    trading_started: bool,
}

impl EskAllocationReceiptVerifier {
    pub(crate) fn from_env() -> Result<Self, ReceiptVerifierConfigError> {
        match std::env::var(ENV_KEYRING) {
            Ok(value) => Self::from_keyring_json(&value),
            Err(std::env::VarError::NotPresent) => Err(ReceiptVerifierConfigError::Disabled),
            Err(std::env::VarError::NotUnicode(_)) => Err(ReceiptVerifierConfigError::Invalid),
        }
    }

    pub(crate) fn from_keyring_json(value: &str) -> Result<Self, ReceiptVerifierConfigError> {
        if value.is_empty() || value.len() > MAX_KEYRING_BYTES {
            return Err(ReceiptVerifierConfigError::Invalid);
        }
        let config: KeyringConfig =
            serde_json::from_str(value).map_err(|_| ReceiptVerifierConfigError::Invalid)?;
        if config.schema != KEYRING_SCHEMA || config.keys.is_empty() || config.keys.len() > 8 {
            return Err(ReceiptVerifierConfigError::Invalid);
        }
        let mut keys = HashMap::new();
        let mut public_keys = HashSet::new();
        for entry in config.keys {
            if !valid_identifier(&entry.key_id)
                || !valid_window(
                    entry.status,
                    entry.issued_at_not_before_unix,
                    entry.issued_at_not_after_unix,
                )
            {
                return Err(ReceiptVerifierConfigError::Invalid);
            }
            let decoded = URL_SAFE_NO_PAD
                .decode(&entry.public_key_base64url)
                .map_err(|_| ReceiptVerifierConfigError::Invalid)?;
            if decoded.len() != 32 || !public_keys.insert(decoded.clone()) {
                return Err(ReceiptVerifierConfigError::Invalid);
            }
            let mut public_key = [0_u8; 32];
            public_key.copy_from_slice(&decoded);
            if keys
                .insert(
                    entry.key_id,
                    TrustedKey {
                        public_key,
                        status: entry.status,
                        not_before: entry.issued_at_not_before_unix,
                        not_after: entry.issued_at_not_after_unix,
                    },
                )
                .is_some()
            {
                return Err(ReceiptVerifierConfigError::Invalid);
            }
        }
        Ok(Self { keys })
    }

    pub(crate) fn verify(
        &self,
        token: &str,
        now_unix: i64,
    ) -> Result<VerifiedEskAllocationReceipt, ()> {
        if token.len() > MAX_TOKEN_BYTES || now_unix <= 0 {
            return Err(());
        }
        let mut segments = token.split('.');
        let prefix = segments.next().unwrap_or_default();
        let payload_segment = segments.next().unwrap_or_default();
        let signature_segment = segments.next().unwrap_or_default();
        if prefix != TOKEN_PREFIX
            || payload_segment.is_empty()
            || signature_segment.is_empty()
            || segments.next().is_some()
        {
            return Err(());
        }
        let payload = URL_SAFE_NO_PAD.decode(payload_segment).map_err(|_| ())?;
        if payload.is_empty() || payload.len() > MAX_PAYLOAD_BYTES {
            return Err(());
        }
        let signature = URL_SAFE_NO_PAD.decode(signature_segment).map_err(|_| ())?;
        let claims: Claims = serde_json::from_slice(&payload).map_err(|_| ())?;
        let key = self.keys.get(&claims.key_id).ok_or(())?;
        if key.status == KeyStatus::Revoked
            || key
                .not_before
                .is_some_and(|value| claims.occurred_at_unix < value)
            || key
                .not_after
                .is_some_and(|value| claims.occurred_at_unix > value)
        {
            return Err(());
        }
        UnparsedPublicKey::new(&ED25519, &key.public_key)
            .verify(&payload, &signature)
            .map_err(|_| ())?;
        validate_claims(claims, token, now_unix)
    }
}

fn validate_claims(
    claims: Claims,
    token: &str,
    now_unix: i64,
) -> Result<VerifiedEskAllocationReceipt, ()> {
    let event_shape = match claims.event.as_str() {
        "accepted" => {
            claims.binding_status == "accepted"
                && claims.binding_revision == 1
                && claims.previous_receipt_digest.is_none()
        }
        "released" => {
            claims.binding_status == "released"
                && claims.binding_revision == 2
                && claims
                    .previous_receipt_digest
                    .as_deref()
                    .is_some_and(valid_digest)
        }
        _ => false,
    };
    if claims.schema != RECEIPT_SCHEMA
        || claims.issuer != "yilong-quant"
        || claims.audience != "yilong-main"
        || claims.project_id != "esk"
        || !event_shape
        || !claims.simulated
        || claims.funds_moved
        || claims.position_kind != "esk_paper_allocation_binding"
        || claims.quant_units_issued
        || claims.nav_participation
        || claims.trading_started
        || !prefixed_hex(&claims.receipt_id, "eskrcpt_", 32)
        || !prefixed_hex(&claims.binding_id, "eskbind_", 32)
        || !prefixed_hex(&claims.request_id, "eskq_", 32)
        || !prefixed_hex(&claims.participant_ref, "yp1_", 40)
        || !prefixed_hex(&claims.authorization_id, "eskauth_", 32)
        || claims.occurred_at_unix <= 0
        || claims.occurred_at_unix > now_unix + 30
    {
        return Err(());
    }
    let base_units = parse_base_units(&claims.amount_base_units)?;
    if base_units <= 0 || format_amount(base_units) != claims.amount {
        return Err(());
    }
    Ok(VerifiedEskAllocationReceipt {
        receipt_id: claims.receipt_id,
        key_id: claims.key_id,
        event: claims.event,
        binding_id: claims.binding_id,
        request_id: claims.request_id,
        participant_ref: claims.participant_ref,
        amount: claims.amount,
        amount_base_units: claims.amount_base_units,
        authorization_id: claims.authorization_id,
        binding_revision: claims.binding_revision,
        previous_receipt_digest: claims.previous_receipt_digest,
        occurred_at_unix: claims.occurred_at_unix,
        receipt_digest: format!("sha256:{:x}", Sha256::digest(token.as_bytes())),
    })
}

fn parse_base_units(value: &str) -> Result<i64, ()> {
    if value.is_empty()
        || value.len() > 19
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(());
    }
    value.parse().map_err(|_| ())
}
fn format_amount(value: i64) -> String {
    format!("{}.{:06}", value / 1_000_000, value % 1_000_000)
}
fn prefixed_hex(value: &str, prefix: &str, digits: usize) -> bool {
    value.len() == prefix.len() + digits
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn valid_digest(value: &str) -> bool {
    prefixed_hex(value, "sha256:", 64)
}
fn valid_identifier(value: &str) -> bool {
    (3..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
fn valid_window(status: KeyStatus, start: Option<i64>, end: Option<i64>) -> bool {
    if start.is_some_and(|value| value <= 0)
        || end.is_some_and(|value| value <= 0)
        || matches!((start, end), (Some(start), Some(end)) if start > end)
    {
        return false;
    }
    status != KeyStatus::Retiring || end.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};

    #[test]
    fn verifies_receipts_and_rejects_unsafe_or_revoked_claims() {
        let pair = Ed25519KeyPair::from_seed_unchecked(&[71; 32]).unwrap();
        let keyring = serde_json::json!({
            "schema": KEYRING_SCHEMA,
            "keys": [{"key_id":"quant-key-1","public_key_base64url":URL_SAFE_NO_PAD.encode(pair.public_key().as_ref()),"status":"active"}]
        }).to_string();
        let verifier = EskAllocationReceiptVerifier::from_keyring_json(&keyring).unwrap();
        let mut claims = valid_claims();
        let token = sign(&pair, &claims);
        let receipt = verifier.verify(&token, 1_788_192_030).unwrap();
        assert_eq!(receipt.event, "accepted");
        assert_eq!(receipt.amount_base_units, "12345678");
        claims["funds_moved"] = true.into();
        assert!(verifier
            .verify(&sign(&pair, &claims), 1_788_192_030)
            .is_err());
        let revoked = keyring.replace("\"active\"", "\"revoked\"");
        assert!(EskAllocationReceiptVerifier::from_keyring_json(&revoked)
            .unwrap()
            .verify(&token, 1_788_192_030)
            .is_err());
    }

    fn valid_claims() -> serde_json::Value {
        serde_json::json!({
            "schema":RECEIPT_SCHEMA,"receipt_id":"eskrcpt_0123456789abcdef0123456789abcdef",
            "issuer":"yilong-quant","audience":"yilong-main","project_id":"esk","key_id":"quant-key-1",
            "event":"accepted","binding_id":"eskbind_0123456789abcdef0123456789abcdef","binding_status":"accepted",
            "request_id":"eskq_0123456789abcdef0123456789abcdef","participant_ref":"yp1_0123456789abcdef0123456789abcdef01234567",
            "amount":"12.345678","amount_base_units":"12345678","authorization_id":"eskauth_0123456789abcdef0123456789abcdef",
            "binding_revision":1,"previous_receipt_digest":null,"occurred_at_unix":1_788_192_020_i64,
            "simulated":true,"funds_moved":false,"position_kind":"esk_paper_allocation_binding",
            "quant_units_issued":false,"nav_participation":false,"trading_started":false
        })
    }
    fn sign(pair: &Ed25519KeyPair, claims: &serde_json::Value) -> String {
        let payload = serde_json::to_vec(claims).unwrap();
        format!(
            "yqar1.{}.{}",
            URL_SAFE_NO_PAD.encode(&payload),
            URL_SAFE_NO_PAD.encode(pair.sign(&payload).as_ref())
        )
    }
}
