use std::{
    sync::Arc,
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
use sha2::{Digest, Sha256};

pub(crate) const DESKTOP_REVIEW_CREDENTIAL_ENV: &str = "ELON_DESKTOP_REVIEW_CREDENTIAL";
pub(crate) const DESKTOP_REVIEW_PUBLIC_KEYS_ENV: &str = "ELON_DESKTOP_REVIEW_PUBLIC_KEYS";
pub(crate) const DESKTOP_REVIEW_TICKET_HEADER: &str = "x-elon-desktop-review-ticket";
const TICKET_VERSION: &str = "v1";
const MAX_TICKET_LIFETIME_SECS: u64 = 180;
const CLOCK_SKEW_SECS: u64 = 15;

#[derive(Clone, Debug)]
pub(crate) struct DesktopReviewAuth {
    credential: Option<Arc<[u8]>>,
    public_keys: Arc<Vec<(String, RsaPublicKey)>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DesktopReviewAuthError {
    NotConfigured,
    Missing,
    Invalid,
    Expired,
}

impl DesktopReviewAuth {
    pub(crate) fn from_env() -> Self {
        let credential = std::env::var(DESKTOP_REVIEW_CREDENTIAL_ENV)
            .ok()
            .map(|value| value.trim().as_bytes().to_vec())
            .filter(|value| value.len() >= 32)
            .map(Arc::<[u8]>::from);
        let public_keys = std::env::var(DESKTOP_REVIEW_PUBLIC_KEYS_ENV)
            .ok()
            .map(|value| parse_public_keys(&value))
            .unwrap_or_default();
        Self {
            credential,
            public_keys: Arc::new(public_keys),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(credential: &str) -> Self {
        Self {
            credential: Some(Arc::from(credential.as_bytes())),
            public_keys: Arc::new(Vec::new()),
        }
    }

    pub(crate) fn verify_headers(
        &self,
        headers: &HeaderMap,
        owner_user_id: &str,
        task_id: &str,
    ) -> Result<(), DesktopReviewAuthError> {
        let ticket = headers
            .get(DESKTOP_REVIEW_TICKET_HEADER)
            .and_then(|value| value.to_str().ok())
            .ok_or(DesktopReviewAuthError::Missing)?;
        self.verify_ticket(ticket, owner_user_id, task_id, now_secs())
    }

    fn verify_ticket(
        &self,
        ticket: &str,
        owner_user_id: &str,
        task_id: &str,
        now: u64,
    ) -> Result<(), DesktopReviewAuthError> {
        if ticket.starts_with("v2.") {
            return self.verify_v2_ticket(ticket, owner_user_id, task_id, now);
        }
        let credential = self.credential.as_deref().ok_or_else(|| {
            if self.public_keys.is_empty() {
                DesktopReviewAuthError::NotConfigured
            } else {
                DesktopReviewAuthError::Invalid
            }
        })?;
        let mut parts = ticket.split('.');
        let version = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let expires_at = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(DesktopReviewAuthError::Invalid)?;
        let nonce = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let signature = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        if parts.next().is_some()
            || version != TICKET_VERSION
            || nonce.len() < 16
            || nonce.len() > 96
            || !nonce
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'-')
        {
            return Err(DesktopReviewAuthError::Invalid);
        }
        if expires_at.saturating_add(CLOCK_SKEW_SECS) < now
            || expires_at > now.saturating_add(MAX_TICKET_LIFETIME_SECS)
        {
            return Err(DesktopReviewAuthError::Expired);
        }
        let message = ticket_message(owner_user_id, task_id, expires_at, nonce);
        let expected = hmac_sha256(credential, message.as_bytes());
        let supplied = hex::decode(signature).map_err(|_| DesktopReviewAuthError::Invalid)?;
        if !constant_time_eq(&expected, &supplied) {
            return Err(DesktopReviewAuthError::Invalid);
        }
        Ok(())
    }

    fn verify_v2_ticket(
        &self,
        ticket: &str,
        owner_user_id: &str,
        task_id: &str,
        now: u64,
    ) -> Result<(), DesktopReviewAuthError> {
        let mut parts = ticket.split('.');
        let version = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let key_id = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let expires_at = parts
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(DesktopReviewAuthError::Invalid)?;
        let nonce = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        let signature = parts.next().ok_or(DesktopReviewAuthError::Invalid)?;
        if parts.next().is_some()
            || version != "v2"
            || key_id.len() != 16
            || !key_id.bytes().all(|value| value.is_ascii_hexdigit())
            || nonce.len() < 16
            || nonce.len() > 96
            || !nonce
                .bytes()
                .all(|value| value.is_ascii_alphanumeric() || value == b'-')
        {
            return Err(DesktopReviewAuthError::Invalid);
        }
        if expires_at.saturating_add(CLOCK_SKEW_SECS) < now
            || expires_at > now.saturating_add(MAX_TICKET_LIFETIME_SECS)
        {
            return Err(DesktopReviewAuthError::Expired);
        }
        let (_, public_key) = self
            .public_keys
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(key_id))
            .ok_or_else(|| {
                if self.public_keys.is_empty() {
                    DesktopReviewAuthError::NotConfigured
                } else {
                    DesktopReviewAuthError::Invalid
                }
            })?;
        let supplied = BASE64
            .decode(signature)
            .ok()
            .and_then(|bytes| RsaSignature::try_from(bytes.as_slice()).ok())
            .ok_or(DesktopReviewAuthError::Invalid)?;
        let message = ticket_message_v2(owner_user_id, task_id, expires_at, nonce);
        VerifyingKey::<Sha256>::new(public_key.clone())
            .verify(message.as_bytes(), &supplied)
            .map_err(|_| DesktopReviewAuthError::Invalid)
    }

    #[cfg(test)]
    pub(crate) fn mint_for_test(
        &self,
        owner_user_id: &str,
        task_id: &str,
        expires_at: u64,
        nonce: &str,
    ) -> String {
        let credential = self.credential.as_deref().expect("test credential");
        let message = ticket_message(owner_user_id, task_id, expires_at, nonce);
        format!(
            "{TICKET_VERSION}.{expires_at}.{nonce}.{}",
            hex::encode(hmac_sha256(credential, message.as_bytes()))
        )
    }
}

fn ticket_message(owner_user_id: &str, task_id: &str, expires_at: u64, nonce: &str) -> String {
    format!("{TICKET_VERSION}\n{owner_user_id}\n{task_id}\n{expires_at}\n{nonce}")
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut normalized = [0_u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        normalized[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36_u8; BLOCK_SIZE];
    let mut outer_pad = [0x5c_u8; BLOCK_SIZE];
    for index in 0..BLOCK_SIZE {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn constant_time_eq(expected: &[u8], supplied: &[u8]) -> bool {
    if expected.len() != supplied.len() {
        return false;
    }
    expected
        .iter()
        .zip(supplied)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::{
        pkcs1v15::SigningKey,
        rand_core::OsRng,
        signature::{SignatureEncoding, Signer},
        RsaPrivateKey,
    };

    const SECRET: &str = "desktop-only-test-credential-32-bytes-long";

    #[test]
    fn desktop_ticket_is_bound_to_owner_task_and_expiry() {
        let auth = DesktopReviewAuth::for_test(SECRET);
        let ticket = auth.mint_for_test("owner-a", "local-a", 1_120, "nonce-1234567890");
        assert_eq!(
            auth.verify_ticket(&ticket, "owner-a", "local-a", 1_000),
            Ok(())
        );
        assert_eq!(
            auth.verify_ticket(&ticket, "owner-b", "local-a", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
        assert_eq!(
            auth.verify_ticket(&ticket, "owner-a", "local-b", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
        assert_eq!(
            auth.verify_ticket(&ticket, "owner-a", "local-a", 1_200),
            Err(DesktopReviewAuthError::Expired)
        );
    }

    #[test]
    fn missing_wrong_and_unconfigured_credentials_fail_closed() {
        let auth = DesktopReviewAuth::for_test(SECRET);
        assert_eq!(
            auth.verify_ticket("", "owner", "task", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
        let other = DesktopReviewAuth::for_test("another-desktop-credential-32-bytes-long");
        let ticket = other.mint_for_test("owner", "task", 1_100, "nonce-1234567890");
        assert_eq!(
            auth.verify_ticket(&ticket, "owner", "task", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
        let unavailable = DesktopReviewAuth {
            credential: None,
            public_keys: Arc::new(Vec::new()),
        };
        assert_eq!(
            unavailable.verify_ticket(&ticket, "owner", "task", 1_000),
            Err(DesktopReviewAuthError::NotConfigured)
        );
    }

    #[test]
    fn v2_public_key_rotation_and_wrong_signature_fail_closed() {
        let private = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        let other = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        let key_id = "0123456789abcdef";
        let encode = |key: &RsaPrivateKey| {
            let public = key.to_public_key();
            format!(
                "{key_id}:{}:{}",
                BASE64.encode(public.n().to_bytes_be()),
                BASE64.encode(public.e().to_bytes_be())
            )
        };
        let auth = DesktopReviewAuth {
            credential: None,
            public_keys: Arc::new(parse_public_keys(&encode(&private))),
        };
        let message = ticket_message_v2("owner", "task", 1_100, "nonce-1234567890");
        let signature = SigningKey::<Sha256>::new(private).sign(message.as_bytes());
        let ticket = format!(
            "v2.{key_id}.1100.nonce-1234567890.{}",
            BASE64.encode(signature.to_bytes())
        );
        assert_eq!(auth.verify_ticket(&ticket, "owner", "task", 1_000), Ok(()));

        let wrong = SigningKey::<Sha256>::new(other).sign(message.as_bytes());
        let wrong_ticket = format!(
            "v2.{key_id}.1100.nonce-1234567890.{}",
            BASE64.encode(wrong.to_bytes())
        );
        assert_eq!(
            auth.verify_ticket(&wrong_ticket, "owner", "task", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
        assert_eq!(
            auth.verify_ticket(&ticket, "owner", "other-task", 1_000),
            Err(DesktopReviewAuthError::Invalid)
        );
    }
}

fn ticket_message_v2(owner_user_id: &str, task_id: &str, expires_at: u64, nonce: &str) -> String {
    format!("v2\n{owner_user_id}\n{task_id}\n{expires_at}\n{nonce}")
}

fn parse_public_keys(value: &str) -> Vec<(String, RsaPublicKey)> {
    value
        .split(';')
        .filter_map(|entry| {
            let mut fields = entry.trim().split(':');
            let key_id = fields.next()?;
            let modulus = fields.next()?;
            let exponent = fields.next()?;
            if fields.next().is_some() {
                return None;
            }
            if key_id.len() != 16 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            let modulus = BASE64.decode(modulus.trim()).ok()?;
            let exponent = BASE64.decode(exponent.trim()).ok()?;
            let key = RsaPublicKey::new(
                BigUint::from_bytes_be(&modulus),
                BigUint::from_bytes_be(&exponent),
            )
            .ok()?;
            (key.size() >= 256).then(|| (key_id.to_ascii_lowercase(), key))
        })
        .collect()
}
