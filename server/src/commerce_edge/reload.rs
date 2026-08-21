use std::{path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use futures::future::try_join_all;
use tracing::{info, warn};

use super::{
    config::{config_digest, read_config_bytes, EdgeConfig, ImmutableConfigIdentity},
    routes::{RouteRegistry, RouteTable},
};

pub(crate) async fn probe_routes(config: &EdgeConfig, client: &reqwest::Client) -> Result<()> {
    let probes = config
        .routes()
        .iter()
        .filter(|route| route.enabled())
        .map(|route| async move {
            let url = format!("http://{}/health", route.upstream_addr());
            let response = tokio::time::timeout(config.request_timeout(), client.get(url).send())
                .await
                .context("COMMERCE_EDGE_HEALTH_TIMEOUT")?
                .context("COMMERCE_EDGE_HEALTH_UNREACHABLE")?;
            if !response.status().is_success() {
                bail!("COMMERCE_EDGE_HEALTH_STATUS_REJECTED");
            }
            Ok::<(), anyhow::Error>(())
        });
    try_join_all(probes).await?;
    Ok(())
}

pub(crate) async fn watch_config(
    path: PathBuf,
    immutable_identity: ImmutableConfigIdentity,
    registry: Arc<RouteRegistry>,
    client: reqwest::Client,
    reload_interval: std::time::Duration,
    initial_digest: String,
) {
    let mut last_seen_digest = initial_digest;
    let mut interval = tokio::time::interval(reload_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    interval.tick().await;
    loop {
        interval.tick().await;
        let bytes = match read_config_bytes(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                warn!(error = %error, "commerce edge config reload read failed; keeping current routes");
                continue;
            }
        };
        let digest = config_digest(&bytes);
        if digest == last_seen_digest {
            continue;
        }
        last_seen_digest = digest;
        if let Err(error) = apply_candidate(&bytes, &immutable_identity, &registry, &client).await {
            warn!(error = %error, "commerce edge config candidate rejected; keeping current routes");
            continue;
        }
        info!(
            active_routes = registry.snapshot().enabled_routes(),
            "commerce edge routes replaced"
        );
    }
}

async fn apply_candidate(
    bytes: &[u8],
    immutable_identity: &ImmutableConfigIdentity,
    registry: &RouteRegistry,
    client: &reqwest::Client,
) -> Result<()> {
    let candidate = EdgeConfig::parse(bytes)?;
    if &candidate.immutable_identity() != immutable_identity {
        bail!("COMMERCE_EDGE_RELOAD_RESTART_REQUIRED");
    }
    let table = RouteTable::from_config(&candidate)?;
    probe_routes(&candidate, client).await?;
    registry.replace(table);
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{routing::get, Router};

    use super::*;

    #[tokio::test]
    async fn unhealthy_candidate_keeps_current_table_and_healthy_candidate_replaces_it() {
        let healthy = spawn_health(StatusCode::OK).await;
        let unhealthy = spawn_health(StatusCode::SERVICE_UNAVAILABLE).await;
        let initial = config_for("coffee-a", healthy);
        let registry = RouteRegistry::new(RouteTable::from_config(&initial).unwrap());
        let client = crate::commerce_edge::proxy::build_client(&initial).unwrap();

        let rejected = config_bytes("coffee-b", unhealthy, &initial);
        assert!(
            apply_candidate(&rejected, &initial.immutable_identity(), &registry, &client)
                .await
                .is_err()
        );
        assert!(registry
            .snapshot()
            .resolve(
                Some("commerce.example.com"),
                &axum::http::Method::GET,
                &"/merchants/coffee-a/health".parse().unwrap()
            )
            .is_ok());

        let accepted = config_bytes("coffee-b", healthy, &initial);
        apply_candidate(&accepted, &initial.immutable_identity(), &registry, &client)
            .await
            .unwrap();
        assert!(registry
            .snapshot()
            .resolve(
                Some("commerce.example.com"),
                &axum::http::Method::GET,
                &"/merchants/coffee-b/health".parse().unwrap()
            )
            .is_ok());
    }

    use axum::http::StatusCode;

    async fn spawn_health(status: StatusCode) -> std::net::SocketAddr {
        let app = Router::new().route("/health", get(move || async move { status }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    fn config_for(instance_id: &str, upstream: std::net::SocketAddr) -> EdgeConfig {
        EdgeConfig::parse(&config_bytes_raw(instance_id, upstream)).unwrap()
    }

    fn config_bytes(
        instance_id: &str,
        upstream: std::net::SocketAddr,
        _identity_source: &EdgeConfig,
    ) -> Vec<u8> {
        config_bytes_raw(instance_id, upstream)
    }

    fn config_bytes_raw(instance_id: &str, upstream: std::net::SocketAddr) -> Vec<u8> {
        let cert = std::env::temp_dir()
            .join("edge-cert.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        let key = std::env::temp_dir()
            .join("edge-key.pem")
            .to_string_lossy()
            .replace('\\', "\\\\");
        format!(
            r#"{{"schema":"yilong.commerce-edge.v1","listen_addr":"127.0.0.1:18443","certificate_chain_path":"{cert}","private_key_path":"{key}","public_hosts":["commerce.example.com"],"routes":[{{"instance_id":"{instance_id}","public_base_path":"/merchants/{instance_id}","upstream_addr":"{upstream}"}}]}}"#
        )
        .into_bytes()
    }
}
