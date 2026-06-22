// server/src/node_agent_route_c_status.rs

use std::time::Duration;

use tracing::warn;

/// Route C 代表“使用云端服务器模型”。这里必须问云端真实 runtime 状态，
/// 不能只因为本机有登录 token 就告诉前端“可以开发”。
pub(crate) async fn server_runtime_ready_from_cloud(
    cloud_http_url: &str,
    user_token: Option<&str>,
) -> bool {
    let Some(token) = user_token.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            warn!("Route C 服务器模型预检无法创建 HTTP client: {e}");
            return false;
        }
    };
    let url = format!(
        "{}/api/agent/runtime/status",
        cloud_http_url.trim_end_matches('/')
    );
    let response = match client.get(url).bearer_auth(token).send().await {
        Ok(response) => response,
        Err(e) => {
            warn!("Route C 服务器模型预检失败: {e}");
            return false;
        }
    };
    let status = response.status();
    if !status.is_success() {
        warn!("Route C 服务器模型预检返回 {status}");
        return false;
    }
    match response.json::<serde_json::Value>().await {
        Ok(value) => server_runtime_ready_from_status_value(&value),
        Err(e) => {
            warn!("Route C 服务器模型预检响应不是 JSON: {e}");
            false
        }
    }
}

fn server_runtime_ready_from_status_value(value: &serde_json::Value) -> bool {
    value
        .get("ready")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::server_runtime_ready_from_status_value;
    use serde_json::json;

    #[test]
    fn parses_server_runtime_ready_status() {
        assert!(server_runtime_ready_from_status_value(
            &json!({"ready": true})
        ));
        assert!(!server_runtime_ready_from_status_value(
            &json!({"ready": false})
        ));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "ready": false,
            "status": "disabled",
            "policy": {"enabled": false}
        })));
        assert!(!server_runtime_ready_from_status_value(&json!({
            "status": "ready"
        })));
    }
}
