use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio_tungstenite::{
    connect_async, tungstenite::client::IntoClientRequest, MaybeTlsStream, WebSocketStream,
};

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
