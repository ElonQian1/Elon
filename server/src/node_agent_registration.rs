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
    let url = registration_endpoint(&cfg.cloud_http_url)?;
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
        .await
        .map_err(|error| {
            anyhow!(
                "无法连接一龙云端注册节点（{}）。首次绑定必须联网；已有节点凭证不会因此失效，仍可使用本机离线任务。底层错误: {}",
                cfg.cloud_http_url,
                error
            )
        })?;
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

fn registration_endpoint(cloud_http_url: &str) -> Result<String> {
    let base = cloud_http_url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(base)
        .map_err(|error| anyhow!("NODE_CLOUD_HTTP_URL 不是有效地址: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(anyhow!(
            "NODE_CLOUD_HTTP_URL 必须是有效的 http/https 云端地址"
        ));
    }
    if parsed.port_or_known_default() == Some(9)
        && parsed
            .host_str()
            .is_some_and(|host| matches!(host, "127.0.0.1" | "localhost" | "::1"))
    {
        return Err(anyhow!(
            "节点云端地址被配置为 {base}；127.0.0.1:9 是不可用的测试/代理地址。请检查安装目录 _internal/node-agent.env 或系统环境变量 NODE_CLOUD_URL、NODE_CLOUD_HTTP_URL"
        ));
    }
    Ok(format!("{base}/api/me/nodes/register"))
}

#[cfg(test)]
mod tests {
    use super::registration_endpoint;

    #[test]
    fn registration_endpoint_rejects_loopback_discard_port_with_actionable_error() {
        let error = registration_endpoint("http://127.0.0.1:9")
            .expect_err("discard port must not be used as cloud endpoint")
            .to_string();
        assert!(error.contains("127.0.0.1:9"));
        assert!(error.contains("NODE_CLOUD_URL"));
    }

    #[test]
    fn registration_endpoint_accepts_production_and_local_development_servers() {
        assert_eq!(
            registration_endpoint("http://43.139.149.158:8080/").unwrap(),
            "http://43.139.149.158:8080/api/me/nodes/register"
        );
        assert_eq!(
            registration_endpoint("http://127.0.0.1:8080").unwrap(),
            "http://127.0.0.1:8080/api/me/nodes/register"
        );
    }
}
