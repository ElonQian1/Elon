//! Bounded, disposable public link metadata, shared by HTTP and the production-source harness.
#[path = "fetch.rs"]
mod fetch;
#[path = "metadata.rs"]
mod metadata;
#[path = "policy.rs"]
mod policy;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
pub(super) fn public_url(value: &str) -> Option<reqwest::Url> {
    policy::public_url(value)
}
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{OnceCell, Semaphore};
#[derive(Clone, Debug, Serialize)]
pub(super) struct Preview {
    schema: u8,
    url: String,
    site: String,
    title: String,
    author: String,
    image: Option<String>,
    embed: Option<policy::Embed>,
    status: &'static str,
}
impl Preview {
    fn fallback(url: &reqwest::Url) -> Self {
        Self {
            schema: 1,
            url: url.to_string(),
            site: policy::site(url).into(),
            title: String::new(),
            author: String::new(),
            image: None,
            embed: policy::embed(url),
            status: "unavailable",
        }
    }
}
type Cached = Arc<OnceCell<(Instant, Preview)>>;
static CACHE: LazyLock<Mutex<HashMap<[u8; 32], Cached>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static FETCHES: Semaphore = Semaphore::const_new(8);
const CAPACITY: usize = 512;

pub(super) async fn preview(url: reqwest::Url) -> Preview {
    if policy::fetchable(&url) {
        cached(url).await
    } else {
        Preview::fallback(&url)
    }
}
async fn cached(url: reqwest::Url) -> Preview {
    let key: [u8; 32] = Sha256::digest(url.as_str()).into();
    let fallback = Preview::fallback(&url);
    let cell = {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|_, cell| {
            cell.get()
                .is_none_or(|(at, value)| at.elapsed() < ttl(value))
        });
        if let Some(cell) = cache.get(&key) {
            cell.clone()
        } else {
            if cache.len() >= CAPACITY {
                let oldest = cache
                    .iter()
                    .filter_map(|(k, c)| c.get().map(|(at, _)| (*k, *at)))
                    .min_by_key(|(_, at)| *at)
                    .map(|(k, _)| k);
                if let Some(key) = oldest {
                    cache.remove(&key);
                } else {
                    return fallback;
                }
            }
            let cell = Arc::new(OnceCell::new());
            cache.insert(key, cell.clone());
            cell
        }
    };
    cell.get_or_init(|| async {
        let value = match FETCHES.try_acquire() {
            Ok(_permit) => tokio::time::timeout(Duration::from_secs(10), fetch::resolve(url))
                .await
                .ok()
                .and_then(Result::ok)
                .unwrap_or(fallback),
            Err(_) => fallback,
        };
        (Instant::now(), value)
    })
    .await
    .1
    .clone()
}
fn ttl(value: &Preview) -> Duration {
    Duration::from_secs(if value.status == "ready" {
        24 * 3600
    } else {
        30
    })
}
