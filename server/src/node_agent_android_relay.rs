//! Restricted cloud-to-node relay for a shared Android device host.
//!
//! The cloud never receives the node's local-admin token. The node accepts
//! only the Android inspector/live REST surface, injects its in-memory token,
//! and always targets its own loopback admin server.

use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use homecli_proto::{AgentToServer, AndroidDeviceHostRequest};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::NodeRuntime;

const RELAY_TIMEOUT: Duration = Duration::from_secs(20 * 60 + 30);

pub(crate) fn spawn(
    runtime: Arc<NodeRuntime>,
    tx: mpsc::UnboundedSender<Message>,
    request: AndroidDeviceHostRequest,
) {
    tokio::spawn(async move {
        let response = relay(
            &runtime,
            &request.req_id,
            &request.method,
            &request.path,
            request.headers,
            request.body_b64,
        )
        .await;
        if let Ok(payload) = serde_json::to_string(&response) {
            let _ = tx.send(Message::Text(payload));
        }
    });
}

async fn relay(
    runtime: &NodeRuntime,
    req_id: &str,
    method: &str,
    path: &str,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> AgentToServer {
    match relay_inner(runtime, req_id, method, path, headers, body_b64).await {
        Ok(response) => response,
        Err(error) => AgentToServer::HttpError {
            req_id: req_id.to_string(),
            message: error.to_string(),
        },
    }
}

async fn relay_inner(
    runtime: &NodeRuntime,
    req_id: &str,
    method: &str,
    path: &str,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> Result<AgentToServer> {
    validate_relay_target(method, path)?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(RELAY_TIMEOUT)
        .build()?;
    let url = format!(
        "http://127.0.0.1:{}{}",
        crate::node_agent_admin_open::admin_port_from_env(),
        path
    );
    let mut request = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "DELETE" => client.delete(url),
        _ => return Err(anyhow!("共享真机代理不支持 {method} 请求")),
    };
    request = request.header(
        crate::node_agent_local_admin::LOCAL_ADMIN_TOKEN_HEADER,
        runtime.local_admin_token(),
    );
    if let Some((_, content_type)) = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    {
        request = request.header("content-type", content_type);
    }
    if let Some(body) = body_b64 {
        request = request.body(
            B64.decode(body)
                .map_err(|error| anyhow!("body base64: {error}"))?,
        );
    }
    let response = request.send().await?;
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter(|(name, _)| {
            matches!(
                name.as_str(),
                "content-type" | "content-length" | "cache-control"
            )
        })
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect();
    let body = response.bytes().await?;
    Ok(AgentToServer::HttpResponse {
        req_id: req_id.to_string(),
        status,
        headers,
        body_b64: (!body.is_empty()).then(|| B64.encode(body)),
    })
}

pub(crate) fn validate_relay_target(method: &str, path: &str) -> Result<()> {
    if !matches!(method, "GET" | "POST" | "DELETE") {
        return Err(anyhow!("共享真机代理不支持 {method} 请求"));
    }
    let path_only = path.split('?').next().unwrap_or(path);
    let inspector_allowed = matches!(
        path_only,
        "/api/android-inspector/status"
            | "/api/android-inspector/devices"
            | "/api/android-inspector/wireless/reconnect"
            | "/api/android-inspector/capture"
            | "/api/android-inspector/selection-artifact"
    );
    let live_allowed = path_only.starts_with("/api/android-live/")
        && !path_only.starts_with("/api/android-live/runtime")
        && !path_only.contains("/mcp/")
        && !path_only.starts_with("/api/android-live/project-mcp/");
    if !inspector_allowed && !live_allowed {
        return Err(anyhow!("该本机接口不允许通过共享真机代理访问"));
    }
    if path.contains("//") || path.contains("..") || path.contains('#') {
        return Err(anyhow!("共享真机代理路径不合法"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_relay_target;

    #[test]
    fn only_android_rest_surface_is_relayed() {
        assert!(validate_relay_target("GET", "/api/android-inspector/devices").is_ok());
        assert!(validate_relay_target("POST", "/api/android-live/sessions/one/patch").is_ok());
        assert!(validate_relay_target("DELETE", "/api/android-live/sessions/one").is_ok());
        assert!(validate_relay_target("POST", "/api/android-inspector/wireless/forget").is_err());
        assert!(validate_relay_target("GET", "/api/android-live/runtime").is_err());
        assert!(validate_relay_target("POST", "/api/android-live/mcp/one").is_err());
        assert!(validate_relay_target("GET", "/api/status").is_err());
        assert!(validate_relay_target("PATCH", "/api/android-live/sessions/one").is_err());
    }
}
