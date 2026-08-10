use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use crate::node_endpoint_transport::direct_tls::DirectTlsPeerAddress;

const WINDOW: Duration = Duration::from_secs(15 * 60);
const PEER_LIMIT: usize = 32;
const BEARER_LIMIT: usize = 8;
const MAX_PEER_KEYS: usize = 4096;
const MAX_BEARER_KEYS: usize = 4096;

struct Bucket {
    attempts: VecDeque<Instant>,
}

struct FailClosedLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    max_keys: usize,
}

impl FailClosedLimiter {
    fn check(&self, key: String, limit: usize) -> Result<(), u64> {
        let now = Instant::now();
        let Ok(mut buckets) = self.buckets.lock() else {
            return Err(WINDOW.as_secs());
        };
        buckets.retain(|_, bucket| {
            prune(&mut bucket.attempts, now);
            !bucket.attempts.is_empty()
        });
        if !buckets.contains_key(&key) && buckets.len() >= self.max_keys {
            return Err(WINDOW.as_secs());
        }
        let bucket = buckets.entry(key).or_insert_with(|| Bucket {
            attempts: VecDeque::new(),
        });
        prune(&mut bucket.attempts, now);
        if bucket.attempts.len() >= limit {
            return Err(bucket
                .attempts
                .front()
                .map(|started| WINDOW.saturating_sub(now.saturating_duration_since(*started)))
                .unwrap_or(WINDOW)
                .as_secs()
                .max(1));
        }
        bucket.attempts.push_back(now);
        Ok(())
    }
}

pub(super) fn check_peer(peer: DirectTlsPeerAddress) -> Result<(), u64> {
    peer_limiter().check(peer.rate_limit_key(), PEER_LIMIT)
}

pub(super) fn check_bearer(key: &str) -> Result<(), u64> {
    bearer_limiter().check(key.to_string(), BEARER_LIMIT)
}

fn peer_limiter() -> &'static FailClosedLimiter {
    static LIMITER: OnceLock<FailClosedLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| FailClosedLimiter {
        buckets: Mutex::new(HashMap::new()),
        max_keys: MAX_PEER_KEYS,
    })
}

fn bearer_limiter() -> &'static FailClosedLimiter {
    static LIMITER: OnceLock<FailClosedLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| FailClosedLimiter {
        buckets: Mutex::new(HashMap::new()),
        max_keys: MAX_BEARER_KEYS,
    })
}

fn prune(attempts: &mut VecDeque<Instant>, now: Instant) {
    while attempts
        .front()
        .is_some_and(|attempt| now.saturating_duration_since(*attempt) >= WINDOW)
    {
        attempts.pop_front();
    }
}
