use anyhow::{anyhow, Result};
use std::time::Duration;

use crate::{machine_label, node_agent_cloud_net, Credentials, NodeConfig};

/// 用登录 token 调用云端 `POST /api/me/nodes/register`，自动换取节点 agent_id + secret。
/// 若 `existing` 不为空，则带上旧凭证让服务器续约（保留原 agent_id）。
pub(crate) async fn provision_node(
    cfg: &NodeConfig,
    token: &str,
    existing: Option<&Credentials>,
    install_id: &str,
) -> Result<Credentials> {
    let url = format!(
        "{}/api/me/nodes/register",
        cfg.cloud_http_url.trim_end_matches('/')
    );
    let client = node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15));
    let device_name = machine_label();
    let mut body = serde_json::json!({
        "label": device_name,
        "device_name": device_name,
        "install_id": install_id,
    });
    if let Some(creds) = existing {
        body["existing_agent_id"] = serde_json::Value::String(creds.agent_id.clone());
        body["existing_secret"] = serde_json::Value::String(creds.agent_secret.clone());
    }
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("注册节点失败 {}: {}", status, body));
    }
    let j: serde_json::Value = resp.json().await?;
    let agent_id = j
        .get("agent_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("响应缺少 agent_id"))?
        .to_string();
    let agent_secret = j
        .get("agent_secret")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("响应缺少 agent_secret"))?
        .to_string();
    let owner_user_id = j
        .get("owner_user_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    Ok(Credentials {
        agent_id,
        agent_secret,
        owner_user_id,
        user_token: Some(token.to_string()),
    })
}
