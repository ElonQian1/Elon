//! Short-lived in-memory replay cache for federated completion responses.
//!
//! Login responses contain a bearer session, so they are never written to disk.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::OnceLock, time::Duration};
use tokio::{sync::Mutex, time::Instant};

const CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_ENTRIES: usize = 256;

#[derive(Clone)]
pub(crate) struct CachedFederatedCompletion {
    pub(crate) response: Value,
    pub(crate) mode: String,
    pub(crate) user_id: Option<String>,
    expires_at: Instant,
}

#[derive(Default)]
pub(crate) struct FederatedCompletionCache {
    entries: HashMap<String, CachedFederatedCompletion>,
}

impl FederatedCompletionCache {
    fn prune(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, value| value.expires_at > now);
    }
}

pub(crate) trait FederatedCompletionReplayStore: Send {
    fn get(&mut self, key: &str) -> Option<CachedFederatedCompletion>;
    fn insert(&mut self, key: String, mode: &str, user_id: Option<&str>, response: Value);
}

impl FederatedCompletionReplayStore for FederatedCompletionCache {
    fn get(&mut self, key: &str) -> Option<CachedFederatedCompletion> {
        self.prune();
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: String, mode: &str, user_id: Option<&str>, response: Value) {
        self.prune();
        if self.entries.len() >= MAX_ENTRIES {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, value)| value.expires_at)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            key,
            CachedFederatedCompletion {
                response,
                mode: mode.to_string(),
                user_id: user_id.map(ToOwned::to_owned),
                expires_at: Instant::now() + CACHE_TTL,
            },
        );
    }
}

pub(crate) fn completion_cache() -> &'static Mutex<Box<dyn FederatedCompletionReplayStore>> {
    CACHE.get_or_init(|| Mutex::new(Box::new(FederatedCompletionCache::default())))
}

#[allow(dead_code)]
pub(crate) fn install_completion_replay_store(
    store: Box<dyn FederatedCompletionReplayStore>,
) -> Result<(), &'static str> {
    CACHE
        .set(Mutex::new(store))
        .map_err(|_| "federated completion replay store already initialized")
}

pub(crate) fn completion_cache_key(
    challenge_id: &str,
    request_id: &str,
    client_key: &str,
) -> String {
    hex::encode(Sha256::digest(
        format!("{challenge_id}:{request_id}:{client_key}").as_bytes(),
    ))
}

static CACHE: OnceLock<Mutex<Box<dyn FederatedCompletionReplayStore>>> = OnceLock::new();

pub(crate) fn replay_capabilities() -> Value {
    serde_json::json!({
        "backend": "process_local_bearer_memory",
        "ttl_seconds": CACHE_TTL.as_secs(),
        "max_entries": MAX_ENTRIES,
        "distributed_safe": false,
        "bearer_persisted": false,
        "shared_backend_requirement": "encrypted_ephemeral_response_store_or_sticky_single_writer"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn phase2_contract_cache_replays_only_exact_challenge_request_and_client() {
        let key = completion_cache_key("challenge", "request-123", "client-a");
        let other = completion_cache_key("challenge", "request-123", "client-b");
        let mut cache = FederatedCompletionCache::default();
        cache.insert(
            key.clone(),
            "login",
            Some("user-1"),
            serde_json::json!({"session":{"token":"memory-only"}}),
        );
        assert!(cache.get(&key).is_some());
        assert!(cache.get(&other).is_none());
    }
}
