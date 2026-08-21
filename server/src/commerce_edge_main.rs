mod commerce_edge;

use std::{path::PathBuf, sync::Arc};

use anyhow::{bail, Context, Result};
use commerce_edge::{
    config::read_config,
    proxy::{build_client, build_router},
    reload::{probe_routes, watch_config},
    routes::{RouteRegistry, RouteTable},
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let check_config = parse_args()?;
    let config_path = PathBuf::from(
        std::env::var("YILONG_COMMERCE_EDGE_CONFIG_PATH")
            .context("YILONG_COMMERCE_EDGE_CONFIG_PATH is required")?,
    );
    if !config_path.is_absolute() {
        bail!("COMMERCE_EDGE_CONFIG_PATH_NOT_ABSOLUTE");
    }
    let (config, initial_digest) = read_config(&config_path)?;
    commerce_edge::tls::validate_provider_material(&config)?;
    if check_config {
        println!(
            "COMMERCE_EDGE_CONFIG_CHECK=passed schema={}",
            config.schema()
        );
        return Ok(());
    }

    let client = build_client(&config)?;
    let table = RouteTable::from_config(&config)?;
    probe_routes(&config, &client).await?;
    let registry = Arc::new(RouteRegistry::new(table));
    let app = build_router(&config, Arc::clone(&registry), client.clone());
    let reload_task = tokio::spawn(watch_config(
        config_path,
        config.immutable_identity(),
        Arc::clone(&registry),
        client,
        config.reload_interval(),
        initial_digest,
    ));
    info!(
        active_routes = registry.snapshot().enabled_routes(),
        "commerce edge startup checks passed"
    );
    let result = commerce_edge::tls::serve(&config, app, shutdown_signal()).await;
    reload_task.abort();
    result
}

fn parse_args() -> Result<bool> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (None, None) => Ok(false),
        (Some("--check-config"), None) => Ok(true),
        _ => bail!("COMMERCE_EDGE_ARGUMENT_INVALID"),
    }
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
