use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::HeaderMap;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rsa::{
    pkcs1v15::{Signature as RsaSignature, VerifyingKey},
    signature::Verifier,
    traits::PublicKeyParts,
    BigUint, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const DESKTOP_REVIEW_CREDENTIAL_ENV: &str = "ELON_DESKTOP_REVIEW_CREDENTIAL";
pub(crate) const DESKTOP_REVIEW_PUBLIC_KEYS_ENV: &str = "ELON_DESKTOP_REVIEW_PUBLIC_KEYS";
pub(crate) const DESKTOP_REVIEW_NONCE_LEDGER_ENV: &str = "ELON_DESKTOP_REVIEW_NONCE_LEDGER";
pub(crate) const DESKTOP_REVIEW_ALLOW_V2_ENV: &str = "ELON_DESKTOP_REVIEW_ALLOW_V2";
pub(crate) const DESKTOP_REVIEW_TICKET_HEADER: &str = "x-elon-desktop-review-ticket";
const MAX_TICKET_LIFETIME_SECS: u64 = 180;
const CLOCK_SKEW_SECS: u64 = 15;
const MAX_LEDGER_ENTRIES: usize = 4096;

#[derive(Clone, Debug)]
pub(crate) struct DesktopReviewAuth {
    credential: Option<Arc<[u8]>>,
    public_keys: Arc<Vec<(String, RsaPublicKey)>>,
    allow_v2: bool,
    nonce_ledger: Option<Arc<NonceLedger>>,
}

#[derive(Debug)]
struct NonceLedger {
    path: PathBuf,
    lock: Mutex<()>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NonceLedgerFile {
    schema: u8,
    entries: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DesktopReviewAuthError {
    NotConfigured,
    Missing,
    Invalid,
    Expired,
    Replayed,
    LedgerUnavailable,
}

impl DesktopReviewAuth {
    pub(crate) fn from_env() -> Self {
        let credential = std::env::var(DESKTOP_REVIEW_CREDENTIAL_ENV)
            .ok()
            .map(|v| v.trim().as_bytes().to_vec())
            .filter(|v| v.len() >= 32)
            .map(Arc::<[u8]>::from);
        let public_keys = std::env::var(DESKTOP_REVIEW_PUBLIC_KEYS_ENV)
            .ok()
            .map(|v| parse_public_keys(&v))
            .unwrap_or_default();
        let nonce_ledger = std::env::var(DESKTOP_REVIEW_NONCE_LEDGER_ENV)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(|v| {
                Arc::new(NonceLedger {
                    path: PathBuf::from(v),
                    lock: Mutex::new(()),
                })
            });
        Self {
            credential,
            public_keys: Arc::new(public_keys),
            allow_v2: env_true(DESKTOP_REVIEW_ALLOW_V2_ENV),
            nonce_ledger,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(credential: &str) -> Self {
        Self {
            credential: Some(Arc::from(credential.as_bytes())),
            public_keys: Arc::new(Vec::new()),
            allow_v2: false,
            nonce_ledger: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_v3_route_test(public_key: RsaPublicKey, ledger_path: PathBuf) -> Self {
        Self {
            credential: None,
            public_keys: Arc::new(vec![("0000000000000000".to_string(), public_key)]),
            allow_v2: false,
            nonce_ledger: Some(Arc::new(NonceLedger {
                path: ledger_path,
                lock: Mutex::new(()),
            })),
        }
    }

    #[cfg(test)]
    pub(crate) fn verify_headers(
        &self,
        headers: &HeaderMap,
        owner: &str,
        task: &str,
    ) -> Result<(), DesktopReviewAuthError> {
        let ticket = headers
            .get(DESKTOP_REVIEW_TICKET_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(DesktopReviewAuthError::Missing)?;
        self.verify_v1(ticket, owner, task, now_secs())
    }

    pub(crate) fn verify_and_consume(
        &self,
        headers: &HeaderMap,
        owner: &str,
        task: &str,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<(), DesktopReviewAuthError> {
        let ticket = headers
            .get(DESKTOP_REVIEW_TICKET_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or(DesktopReviewAuthError::Missing)?;
        let now = now_secs();
        if ticket.starts_with("v3.") {
            let (nonce_key, expires) =
                self.verify_v3(ticket, owner, task, method, path, body, now)?;
            return self.consume_nonce(&nonce_key, expires, now);
        }
        // Public-key mode is a downgrade boundary. v2 is migration-only and v1 is never accepted.
        if !self.public_keys.is_empty() {
            if self.allow_v2 && ticket.starts_with("v2.") {
                self.verify_v2(ticket, owner, task, now)?;
                return Ok(());
            }
            return Err(DesktopReviewAuthError::Invalid);
        }
        self.verify_v1(ticket, owner, task, now)
    }

    fn verify_v3(
        &self,
        ticket: &str,
        owner: &str,
        task: &str,
        method: &str,
        path: &str,
        body: &[u8],
        now: u64,
    ) -> Result<(String, u64), DesktopReviewAuthError> {
        let (key_id, expires, nonce, signature) = parse_asymmetric_ticket(ticket, "v3")?;
        validate_expiry(expires, now)?;
        let (_, key) = self
            .public_keys
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(key_id))
            .ok_or(if self.public_keys.is_empty() {
                DesktopReviewAuthError::NotConfigured
            } else {
                DesktopReviewAuthError::Invalid
            })?;
        let body_hash = hex::encode(Sha256::digest(body));
        let message = ticket_message_v3(
            owner,
            task,
            &method.to_ascii_uppercase(),
            path,
            &body_hash,
            expires,
            nonce,
            key_id,
        );
        verify_signature(key, signature, message.as_bytes())?;
        Ok((
            format!("{}:{}", key_id.to_ascii_lowercase(), nonce),
            expires,
        ))
    }

    fn verify_v2(
        &self,
        ticket: &str,
        owner: &str,
        task: &str,
        now: u64,
    ) -> Result<(), DesktopReviewAuthError> {
        let (key_id, expires, nonce, signature) = parse_asymmetric_ticket(ticket, "v2")?;
        validate_expiry(expires, now)?;
        let (_, key) = self
            .public_keys
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(key_id))
            .ok_or(DesktopReviewAuthError::Invalid)?;
        verify_signature(
            key,
            signature,
            format!("v2\n{owner}\n{task}\n{expires}\n{nonce}").as_bytes(),
        )
    }

    fn verify_v1(
        &self,
        ticket: &str,
        owner: &str,
        task: &str,
        now: u64,
    ) -> Result<(), DesktopReviewAuthError> {
        let credential = self
            .credential
            .as_deref()
            .ok_or(DesktopReviewAuthError::NotConfigured)?;
        let mut p = ticket.split('.');
        let version = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let expires = p
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or(DesktopReviewAuthError::Invalid)?;
        let nonce = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let signature = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
        if p.next().is_some() || version != "v1" || !valid_nonce(nonce) {
            return Err(DesktopReviewAuthError::Invalid);
        }
        validate_expiry(expires, now)?;
        let expected = hmac_sha256(
            credential,
            format!("v1\n{owner}\n{task}\n{expires}\n{nonce}").as_bytes(),
        );
        let supplied = hex::decode(signature).map_err(|_| DesktopReviewAuthError::Invalid)?;
        if constant_time_eq(&expected, &supplied) {
            Ok(())
        } else {
            Err(DesktopReviewAuthError::Invalid)
        }
    }

    fn consume_nonce(
        &self,
        nonce: &str,
        expires: u64,
        now: u64,
    ) -> Result<(), DesktopReviewAuthError> {
        let ledger = self
            .nonce_ledger
            .as_ref()
            .ok_or(DesktopReviewAuthError::NotConfigured)?;
        let _guard = ledger
            .lock
            .lock()
            .map_err(|_| DesktopReviewAuthError::LedgerUnavailable)?;
        let mut state = read_ledger(&ledger.path)?;
        state
            .entries
            .retain(|_, expiry| expiry.saturating_add(CLOCK_SKEW_SECS) >= now);
        if state.entries.contains_key(nonce) {
            return Err(DesktopReviewAuthError::Replayed);
        }
        if state.entries.len() >= MAX_LEDGER_ENTRIES {
            return Err(DesktopReviewAuthError::LedgerUnavailable);
        }
        state.entries.insert(nonce.to_string(), expires);
        write_ledger_atomic(&ledger.path, &state)
    }

    #[cfg(test)]
    pub(crate) fn mint_for_test(
        &self,
        owner: &str,
        task: &str,
        expires: u64,
        nonce: &str,
    ) -> String {
        let key = self.credential.as_deref().unwrap();
        let msg = format!("v1\n{owner}\n{task}\n{expires}\n{nonce}");
        format!(
            "v1.{expires}.{nonce}.{}",
            hex::encode(hmac_sha256(key, msg.as_bytes()))
        )
    }
}

fn parse_asymmetric_ticket<'a>(
    ticket: &'a str,
    expected: &str,
) -> Result<(&'a str, u64, &'a str, &'a str), DesktopReviewAuthError> {
    let mut p = ticket.split('.');
    let version = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
    let key_id = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
    let expires = p
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or(DesktopReviewAuthError::Invalid)?;
    let nonce = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
    let signature = p.next().ok_or(DesktopReviewAuthError::Invalid)?;
    if p.next().is_some()
        || version != expected
        || key_id.len() != 16
        || !key_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !valid_nonce(nonce)
    {
        return Err(DesktopReviewAuthError::Invalid);
    }
    Ok((key_id, expires, nonce, signature))
}

fn valid_nonce(value: &str) -> bool {
    (16..=96).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn validate_expiry(expires: u64, now: u64) -> Result<(), DesktopReviewAuthError> {
    if expires.saturating_add(CLOCK_SKEW_SECS) < now
        || expires > now.saturating_add(MAX_TICKET_LIFETIME_SECS)
    {
        Err(DesktopReviewAuthError::Expired)
    } else {
        Ok(())
    }
}
fn field(value: &str) -> String {
    format!("{}:{value}", value.as_bytes().len())
}
pub(crate) fn endpoint_path(task_id: &str) -> String {
    let encoded = task_id
        .as_bytes()
        .iter()
        .fold(String::new(), |mut out, byte| {
            if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
                out.push(*byte as char);
            } else {
                out.push_str(&format!("%{byte:02X}"));
            }
            out
        });
    format!("/api/local-tasks/{encoded}/supervision/desktop-review")
}
fn ticket_message_v3(
    owner: &str,
    task: &str,
    method: &str,
    path: &str,
    body_hash: &str,
    expires: u64,
    nonce: &str,
    key_id: &str,
) -> String {
    [
        "v3".to_string(),
        field(owner),
        field(task),
        field(method),
        field(path),
        field(body_hash),
        expires.to_string(),
        field(nonce),
        field(key_id),
    ]
    .join("\n")
}
fn verify_signature(
    key: &RsaPublicKey,
    signature: &str,
    message: &[u8],
) -> Result<(), DesktopReviewAuthError> {
    let supplied = BASE64
        .decode(signature)
        .ok()
        .and_then(|b| RsaSignature::try_from(b.as_slice()).ok())
        .ok_or(DesktopReviewAuthError::Invalid)?;
    VerifyingKey::<Sha256>::new(key.clone())
        .verify(message, &supplied)
        .map_err(|_| DesktopReviewAuthError::Invalid)
}
fn read_ledger(path: &Path) -> Result<NonceLedgerFile, DesktopReviewAuthError> {
    if !path.exists() {
        return Ok(NonceLedgerFile {
            schema: 1,
            entries: BTreeMap::new(),
        });
    }
    let bytes = fs::read(path).map_err(|_| DesktopReviewAuthError::LedgerUnavailable)?;
    if bytes.len() > 512 * 1024 {
        return Err(DesktopReviewAuthError::LedgerUnavailable);
    }
    let state: NonceLedgerFile =
        serde_json::from_slice(&bytes).map_err(|_| DesktopReviewAuthError::LedgerUnavailable)?;
    if state.schema != 1 || state.entries.len() > MAX_LEDGER_ENTRIES {
        return Err(DesktopReviewAuthError::LedgerUnavailable);
    }
    Ok(state)
}
fn write_ledger_atomic(path: &Path, state: &NonceLedgerFile) -> Result<(), DesktopReviewAuthError> {
    let bytes = serde_json::to_vec(state).map_err(|_| DesktopReviewAuthError::LedgerUnavailable)?;
    crate::node_agent_atomic_file::write(path, &bytes)
        .map_err(|_| DesktopReviewAuthError::LedgerUnavailable)
}
fn env_true(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner = [0x36u8; 64];
    let mut outer = [0x5cu8; 64];
    for i in 0..64 {
        inner[i] ^= normalized[i];
        outer[i] ^= normalized[i];
    }
    let mut hash = Sha256::new();
    hash.update(inner);
    hash.update(message);
    let digest = hash.finalize();
    let mut hash = Sha256::new();
    hash.update(outer);
    hash.update(digest);
    hash.finalize().into()
}
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_public_keys(value: &str) -> Vec<(String, RsaPublicKey)> {
    value
        .split(';')
        .filter_map(|entry| {
            let mut f = entry.trim().split(':');
            let id = f.next()?;
            let n = f.next()?;
            let e = f.next()?;
            if f.next().is_some() || id.len() != 16 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let key = RsaPublicKey::new(
                BigUint::from_bytes_be(&BASE64.decode(n).ok()?),
                BigUint::from_bytes_be(&BASE64.decode(e).ok()?),
            )
            .ok()?;
            (key.size() >= 256).then(|| (id.to_ascii_lowercase(), key))
        })
        .collect()
}

#[cfg(test)]
#[path = "node_agent_desktop_review_auth_tests.rs"]
mod tests;
