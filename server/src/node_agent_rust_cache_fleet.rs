use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Url};
use tracing::{debug, warn};

use crate::{node_agent_config::NodeConfig, NodeRuntime};

mod model;
mod storage;

const DEFAULT_INTERVAL_SECONDS: u64 = 300;
const INITIAL_DELAY_SECONDS: u64 = 20;
const MAX_ACK_BYTES: u64 = 64 * 1024;
const MAX_BATCH_SIZE: usize = 4;

pub(crate) fn spawn(runtime: Arc<NodeRuntime>) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(INITIAL_DELAY_SECONDS)).await;
        let interval = upload_interval();
        loop {
            if let Err(error) = upload_once(&runtime).await {
                warn!(error = %error, "Rust cache fleet outbox pass failed closed");
            }
            tokio::time::sleep(interval).await;
        }
    });
}

async fn upload_once(runtime: &Arc<NodeRuntime>) -> Result<()> {
    let (credentials, credential_epoch) = runtime.credential_session().await;
    let Some(credentials) = credentials else {
        return Ok(());
    };
    let node_data_root = runtime
        .node_data_root
        .read()
        .await
        .paths
        .as_ref()
        .map(|paths| paths.root().to_path_buf());
    let Some(cache_root) = storage::find_cache_root(node_data_root.as_deref()) else {
        return Ok(());
    };
    let envelopes = storage::pending_envelopes(&cache_root, MAX_BATCH_SIZE)?;
    if envelopes.is_empty() {
        return Ok(());
    }
    let base_url = secure_upload_origin(&runtime.cfg)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .context("build Rust cache fleet HTTP client")?;

    for path in envelopes {
        runtime.require_credential_epoch(credential_epoch)?;
        match upload_envelope(
            &client,
            &base_url,
            &credentials.agent_id,
            &credentials.agent_secret,
            &path,
        )
        .await
        {
            Ok(receipt) => {
                storage::archive_accepted(&cache_root, &path, &receipt)?;
                debug!(
                    envelope_id = %receipt.envelope_id,
                    deduplicated = receipt.deduplicated,
                    "Rust cache fleet envelope acknowledged"
                );
            }
            Err(failure) => {
                storage::record_attempt(&cache_root, &path, &failure)?;
                warn!(
                    category = failure.category,
                    http_status = failure.http_status,
                    "Rust cache fleet envelope upload deferred"
                );
            }
        }
    }
    Ok(())
}

async fn upload_envelope(
    client: &Client,
    base_url: &Url,
    node_id: &str,
    agent_secret: &str,
    path: &Path,
) -> Result<model::UploadReceipt, model::UploadFailure> {
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| model::UploadFailure::local("read-envelope"))?;
    let envelope = model::validate_envelope(&bytes, node_id)
        .map_err(|_| model::UploadFailure::local("invalid-envelope"))?;
    let endpoint = upload_endpoint(base_url, node_id)
        .map_err(|_| model::UploadFailure::local("invalid-server-origin"))?;
    let response = client
        .post(endpoint)
        .bearer_auth(agent_secret)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(bytes)
        .send()
        .await
        .map_err(|_| model::UploadFailure::network("request-failed"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(model::UploadFailure::http(status.as_u16()));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_ACK_BYTES)
    {
        return Err(model::UploadFailure::network("ack-too-large"));
    }
    let ack_bytes = response
        .bytes()
        .await
        .map_err(|_| model::UploadFailure::network("ack-read-failed"))?;
    if ack_bytes.len() as u64 > MAX_ACK_BYTES {
        return Err(model::UploadFailure::network("ack-too-large"));
    }
    model::validate_ack(&ack_bytes, &envelope)
        .map_err(|_| model::UploadFailure::network("ack-contract-mismatch"))
}

fn secure_upload_origin(config: &NodeConfig) -> Result<Url> {
    config
        .endpoint_https_origin
        .as_deref()
        .into_iter()
        .chain(std::iter::once(config.cloud_http_url.as_str()))
        .filter_map(|candidate| Url::parse(candidate.trim()).ok())
        .find(|url| url.scheme() == "https" || is_loopback_http(url))
        .ok_or_else(|| anyhow!("Rust cache fleet upload requires HTTPS or a loopback HTTP origin"))
}

fn is_loopback_http(url: &Url) -> bool {
    url.scheme() == "http"
        && matches!(
            url.host_str().map(str::to_ascii_lowercase).as_deref(),
            Some("127.0.0.1" | "localhost" | "::1")
        )
}

fn upload_endpoint(base_url: &Url, node_id: &str) -> Result<Url> {
    let mut endpoint = base_url.clone();
    endpoint.set_query(None);
    endpoint.set_fragment(None);
    endpoint.set_path("/");
    endpoint
        .path_segments_mut()
        .map_err(|_| anyhow!("server URL cannot be a base"))?
        .extend(["api", "node", "cache-reports", node_id]);
    Ok(endpoint)
}

fn upload_interval() -> Duration {
    let seconds = std::env::var("ELON_RUST_CACHE_FLEET_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECONDS)
        .clamp(60, 3600);
    Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_origin_requires_https_except_for_loopback() {
        let secure = NodeConfig {
            cloud_url: "ws://example.test".into(),
            cloud_http_url: "http://example.test".into(),
            endpoint_https_origin: Some("https://secure.example.test".into()),
            ollama_url: String::new(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        };
        assert_eq!(secure_upload_origin(&secure).unwrap().scheme(), "https");
        let local = NodeConfig {
            endpoint_https_origin: None,
            cloud_http_url: "http://127.0.0.1:8080".into(),
            ..secure
        };
        assert!(secure_upload_origin(&local).is_ok());

        let insecure_remote = NodeConfig {
            endpoint_https_origin: None,
            cloud_http_url: "http://example.test".into(),
            ..local
        };
        assert!(secure_upload_origin(&insecure_remote).is_err());
    }
}
