//! Bounded, disposable public link metadata, shared by HTTP and the production-source harness.
#[path = "cover.rs"]
mod cover;
#[path = "fetch.rs"]
mod fetch;
#[path = "metadata.rs"]
mod metadata;
#[path = "policy.rs"]
mod policy;
#[path = "report.rs"]
mod report;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
pub(super) use report::Read;
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
    description: String,
    author: String,
    image: Option<String>,
    /// Inline thumbnail copied by the server; clients prefer it over hot-linking `image`.
    cover_data_url: Option<String>,
    embed: Option<policy::Embed>,
    status: &'static str,
    /// `server` = fetched here; `member` = re-validated read-back from a signed-in client.
    source: &'static str,
}
impl Preview {
    fn fallback(url: &reqwest::Url) -> Self {
        Self {
            schema: 1,
            url: url.to_string(),
            site: policy::label(url),
            title: String::new(),
            description: String::new(),
            author: String::new(),
            image: None,
            cover_data_url: None,
            embed: policy::embed(url),
            status: "unavailable",
            source: "server",
        }
    }
}
type Cached = Arc<OnceCell<(Instant, Preview)>>;
static CACHE: LazyLock<Mutex<HashMap<[u8; 32], Cached>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static FETCHES: Semaphore = Semaphore::const_new(8);
const CAPACITY: usize = 512;

pub(super) async fn preview(url: reqwest::Url) -> Preview {
    cached(url).await
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
            Ok(_permit) => tokio::time::timeout(Duration::from_secs(12), fetch::resolve(url))
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

/// Stores a member read-back unless the server already holds its own complete result.
pub(super) async fn report(
    user: &str,
    url: reqwest::Url,
    read: &Read,
) -> Result<Preview, &'static str> {
    if !report::allow(user) {
        return Err("回填过于频繁，请稍后再试");
    }
    let accepted = report::validate(&url, read)?;
    let key: [u8; 32] = Sha256::digest(url.as_str()).into();
    let existing = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
        .and_then(|cell| cell.get().cloned());
    if let Some((at, value)) = existing {
        if value.status == "ready" && value.source == "server" && at.elapsed() < ttl(&value) {
            return Ok(value);
        }
    }
    let mut preview = Preview::fallback(&url);
    preview.title = accepted.title;
    preview.author = accepted.author;
    preview.description = accepted.description;
    if let Some(image) = accepted.image.as_deref().and_then(policy::public_url) {
        preview.cover_data_url = cover::fetch(&image).await;
    }
    preview.image = accepted.image;
    preview.status = "ready";
    preview.source = "member";
    store(key, preview.clone());
    Ok(preview)
}

fn store(key: [u8; 32], value: Preview) {
    let cell = Arc::new(OnceCell::new());
    let _ = cell.set((Instant::now(), value));
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= CAPACITY && !cache.contains_key(&key) {
        let oldest = cache
            .iter()
            .filter_map(|(k, c)| c.get().map(|(at, _)| (*k, *at)))
            .min_by_key(|(_, at)| *at)
            .map(|(k, _)| k);
        if let Some(k) = oldest {
            cache.remove(&k);
        }
    }
    cache.insert(key, cell);
}
