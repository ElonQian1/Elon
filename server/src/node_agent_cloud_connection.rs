use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use futures::SinkExt;
use tokio_tungstenite::{
    connect_async, connect_async_tls_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig},
    Connector, MaybeTlsStream, WebSocketStream,
};

const ENDPOINT_MAX_MESSAGE_BYTES: usize = 64 * 1024;

pub(crate) async fn connect(
    cfg: &crate::node_agent_config::NodeConfig,
    creds: &crate::Credentials,
    runtime: &Arc<crate::NodeRuntime>,
) -> Result<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>> {
    let mut request = cfg.cloud_url.as_str().into_client_request()?;
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", creds.agent_secret).parse()?,
    );
    runtime
        .set_connection_stage("cloud_websocket_handshake")
        .await;
    let (stream, _) = tokio::time::timeout(Duration::from_secs(12), connect_async(request))
        .await
        .context("云端 WebSocket 握手超时")??;
    Ok(stream)
}

pub(crate) async fn connect_endpoint(
    runtime: &Arc<crate::NodeRuntime>,
) -> Result<
    Option<(
        WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
        crate::node_agent_endpoint_credentials::EndpointSessionLease,
    )>,
> {
    let (request, lease) = runtime
        .endpoint_credentials
        .prepare_wss_request(runtime.cfg.endpoint_https_origin.as_deref())
        .await?;
    let mut endpoint_epoch = runtime
        .endpoint_credentials
        .subscribe_endpoint_session_epoch();
    if runtime
        .endpoint_credentials
        .require_current_endpoint_session(&lease)
        .await
        .is_err()
    {
        return Ok(None);
    }
    runtime
        .set_connection_stage("endpoint_websocket_handshake")
        .await;
    let websocket_config = WebSocketConfig {
        max_message_size: Some(ENDPOINT_MAX_MESSAGE_BYTES),
        max_frame_size: Some(ENDPOINT_MAX_MESSAGE_BYTES),
        ..WebSocketConfig::default()
    };
    let connector = endpoint_tls_connector()?;
    let handshake = tokio::time::timeout(
        Duration::from_secs(12),
        connect_async_tls_with_config(request, Some(websocket_config), false, Some(connector)),
    );
    let (mut stream, _) = tokio::select! {
        changed = endpoint_epoch.changed() => {
            changed.context("NODE_ENDPOINT_SESSION_EPOCH_CLOSED")?;
            return Ok(None);
        }
        result = handshake => result
            .context("安全 endpoint WebSocket 握手超时")??,
    };
    if let Err(error) = runtime
        .endpoint_credentials
        .require_current_endpoint_session(&lease)
        .await
    {
        let _ = stream.close(None).await;
        tracing::debug!(%error, "endpoint credential changed during WebSocket handshake");
        return Ok(None);
    }
    Ok(Some((stream, lease)))
}

fn endpoint_tls_connector() -> Result<Connector> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Connector::Rustls(Arc::new(config)))
}
