//! Storage boundary for authentication abuse controls.
//!
//! The production default is deliberately process-local. A future shared
//! implementation must provide atomic increment + TTL semantics before it can
//! claim multi-replica safety.

use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const MAX_KEYS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RateLimitDecision {
    pub(crate) allowed: bool,
    pub(crate) retry_after_seconds: u64,
}

pub(crate) trait AuthRateLimitStore: Send + Sync {
    fn check_and_record(
        &self,
        action: &str,
        key: &str,
        limit: usize,
        window: Duration,
    ) -> RateLimitDecision;

    fn backend_id(&self) -> &'static str;
    fn distributed_safe(&self) -> bool;
}

#[derive(Default)]
pub(crate) struct ProcessLocalAuthRateLimitStore {
    entries: Mutex<HashMap<String, RateLimitBucket>>,
}

struct RateLimitBucket {
    timestamps: VecDeque<Instant>,
    window: Duration,
}

impl AuthRateLimitStore for ProcessLocalAuthRateLimitStore {
    fn check_and_record(
        &self,
        action: &str,
        key: &str,
        limit: usize,
        window: Duration,
    ) -> RateLimitDecision {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(|lock| lock.into_inner());
        entries.retain(|_, bucket| {
            prune(&mut bucket.timestamps, now, bucket.window);
            !bucket.timestamps.is_empty()
        });
        let entry_key = format!("{action}:{key}");
        if entries.len() >= MAX_KEYS && !entries.contains_key(&entry_key) {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, bucket)| bucket.timestamps.front().copied())
                .map(|(key, _)| key.clone())
            {
                entries.remove(&oldest);
            }
        }
        let bucket = entries.entry(entry_key).or_insert_with(|| RateLimitBucket {
            timestamps: VecDeque::new(),
            window,
        });
        bucket.window = window;
        prune(&mut bucket.timestamps, now, window);
        if bucket.timestamps.len() >= limit {
            let retry_after_seconds = bucket
                .timestamps
                .front()
                .map(|started| window.saturating_sub(now.saturating_duration_since(*started)))
                .unwrap_or(window)
                .as_secs()
                .max(1);
            return RateLimitDecision {
                allowed: false,
                retry_after_seconds,
            };
        }
        bucket.timestamps.push_back(now);
        RateLimitDecision {
            allowed: true,
            retry_after_seconds: 0,
        }
    }

    fn backend_id(&self) -> &'static str {
        "process_local_bounded_memory"
    }

    fn distributed_safe(&self) -> bool {
        false
    }
}

pub(crate) fn auth_rate_limit_store() -> &'static dyn AuthRateLimitStore {
    STORE
        .get_or_init(|| Box::new(ProcessLocalAuthRateLimitStore::default()))
        .as_ref()
}

#[allow(dead_code)]
pub(crate) fn install_auth_rate_limit_store(
    store: Box<dyn AuthRateLimitStore>,
) -> Result<(), &'static str> {
    STORE
        .set(store)
        .map_err(|_| "auth rate-limit store already initialized")
}

pub(crate) fn auth_safety_capabilities() -> Value {
    let store = auth_rate_limit_store();
    json!({
        "schema": "elon.auth_safety_capabilities.v1",
        "rate_limit": {
            "backend": store.backend_id(),
            "bounded_keys": MAX_KEYS,
            "distributed_safe": store.distributed_safe(),
            "required_shared_semantics": ["atomic_increment", "ttl", "bounded_keyspace"],
        },
        "federated_completion_replay": crate::federated_auth_idempotency::replay_capabilities(),
        "multi_replica_ready": false,
        "perimeter_rate_limit_required": true,
        "upgrade_boundary": "inject_shared_atomic_ttl_backend_before_multi_replica_enablement"
    })
}

fn prune(timestamps: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while timestamps
        .front()
        .is_some_and(|timestamp| now.saturating_duration_since(*timestamp) >= window)
    {
        timestamps.pop_front();
    }
}

static STORE: OnceLock<Box<dyn AuthRateLimitStore>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase2_contract_process_local_backend_enforces_budget_without_cluster_claim() {
        let store = ProcessLocalAuthRateLimitStore::default();
        assert!(
            store
                .check_and_record("login", "client", 2, Duration::from_secs(60))
                .allowed
        );
        assert!(
            store
                .check_and_record("login", "client", 2, Duration::from_secs(60))
                .allowed
        );
        let rejected = store.check_and_record("login", "client", 2, Duration::from_secs(60));
        assert!(!rejected.allowed);
        assert!(rejected.retry_after_seconds > 0);
        assert!(!store.distributed_safe());
    }
}
