use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

pub(crate) const DESKTOP_REVIEW_CREDENTIAL_ENV: &str = "ELON_DESKTOP_REVIEW_CREDENTIAL";
pub(crate) const DESKTOP_REVIEW_TICKET_HEADER: &str = "x-elon-desktop-review-ticket";
const TICKET_VERSION: &str = "v1";
const MAX_TICKET_LIFETIME_SECS: u64 = 180;
const CLOCK_SKEW_SECS: u64 = 15;

#[derive(Clone, Debug)]
pub(crate) struct DesktopReviewAuth {
    credential: Option<Arc<[u8]>>,
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
        Self { credential }
    }

    #[cfg(test)]
    pub(crate) fn for_test(credential: &str) -> Self {
        Self {
            credential: Some(Arc::from(credential.as_bytes())),
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
        let credential = self
            .credential
            .as_deref()
            .ok_or(DesktopReviewAuthError::NotConfigured)?;
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
        let unavailable = DesktopReviewAuth { credential: None };
        assert_eq!(
            unavailable.verify_ticket(&ticket, "owner", "task", 1_000),
            Err(DesktopReviewAuthError::NotConfigured)
        );
    }
}
