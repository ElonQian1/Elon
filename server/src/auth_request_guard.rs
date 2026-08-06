//! Process-local abuse guard for first-party auth endpoints.
//!
//! This complements, but never replaces, perimeter/IP rate limiting.

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use thiserror::Error;

const MAX_KEYS: usize = 4096;

#[derive(Debug, Error)]
#[error("请求过于频繁，请稍后再试")]
pub(crate) struct AuthRateLimited {
    pub(crate) retry_after_seconds: u64,
}

pub(crate) fn validate_request_id(value: &str) -> bool {
    let value = value.trim();
    (8..=128).contains(&value.len())
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character))
}

pub(crate) fn client_key(
    headers: &HeaderMap,
    supplied_instance_id: Option<&str>,
    namespace: &str,
) -> String {
    let supplied = supplied_instance_id
        .map(str::trim)
        .filter(|value| validate_request_id(value));
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown-agent");
    let language = headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown-language");
    let source = supplied
        .map(|value| format!("instance:{value}"))
        .unwrap_or_else(|| format!("fallback:{user_agent}:{language}"));
    hex::encode(Sha256::digest(format!("{namespace}:{source}").as_bytes()))
}

pub(crate) fn check_rate_limit(
    action: &str,
    key: &str,
    limit: usize,
    window: Duration,
) -> Result<(), AuthRateLimited> {
    let now = Instant::now();
    let limiter = LIMITER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut entries = limiter.lock().unwrap_or_else(|lock| lock.into_inner());
    entries.retain(|_, timestamps| {
        prune(timestamps, now, window);
        !timestamps.is_empty()
    });
    if entries.len() >= MAX_KEYS && !entries.contains_key(&format!("{action}:{key}")) {
        if let Some(oldest) = entries
            .iter()
            .min_by_key(|(_, timestamps)| timestamps.front().copied())
            .map(|(entry_key, _)| entry_key.clone())
        {
            entries.remove(&oldest);
        }
    }
    let timestamps = entries.entry(format!("{action}:{key}")).or_default();
    prune(timestamps, now, window);
    if timestamps.len() >= limit {
        let retry_after_seconds = timestamps
            .front()
            .map(|started| window.saturating_sub(now.saturating_duration_since(*started)))
            .unwrap_or(window)
            .as_secs()
            .max(1);
        return Err(AuthRateLimited {
            retry_after_seconds,
        });
    }
    timestamps.push_back(now);
    Ok(())
}

fn prune(timestamps: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while timestamps
        .front()
        .is_some_and(|timestamp| now.saturating_duration_since(*timestamp) >= window)
    {
        timestamps.pop_front();
    }
}

static LIMITER: OnceLock<Mutex<HashMap<String, VecDeque<Instant>>>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_ids_are_bounded_and_machine_safe() {
        assert!(validate_request_id("pc:12345678-1234"));
        assert!(!validate_request_id("short"));
        assert!(!validate_request_id("contains space 123"));
    }

    #[test]
    fn limiter_rejects_only_after_the_configured_budget() {
        let key = UuidLike::new();
        assert!(check_rate_limit("test", &key.0, 2, Duration::from_secs(60)).is_ok());
        assert!(check_rate_limit("test", &key.0, 2, Duration::from_secs(60)).is_ok());
        assert!(check_rate_limit("test", &key.0, 2, Duration::from_secs(60)).is_err());
    }

    struct UuidLike(String);
    impl UuidLike {
        fn new() -> Self {
            Self(uuid::Uuid::new_v4().to_string())
        }
    }
}
