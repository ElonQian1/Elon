use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{node_agent_cloud_net, NodeRuntime};

#[derive(Debug, Default, Deserialize)]
struct LocalCloudProjectsQuery {
    include_system: Option<bool>,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new().route("/api/cloud-projects", get(list_local_cloud_projects))
}

/// 供 PC 工作台、Codex Desktop 和本机脚本读取当前账号的项目列表。
/// 云端访问由 NodeAgent 使用 no_proxy 客户端完成，登录 token 不进入响应。
async fn list_local_cloud_projects(
    State(runtime): State<Arc<NodeRuntime>>,
    Query(query): Query<LocalCloudProjectsQuery>,
) -> Response {
    let Some(credentials) = runtime.creds().await else {
        return error_response(StatusCode::UNAUTHORIZED, "当前 PC 节点尚未登录");
    };
    let Some(user_token) = credentials
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
    else {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "当前节点凭证不含登录 token，请重新登录",
        );
    };

    let url = format!(
        "{}/api/me/projects",
        runtime.cfg.cloud_http_url.trim_end_matches('/')
    );
    let client = match node_agent_cloud_net::direct_cloud_client(Duration::from_secs(15)) {
        Ok(client) => client,
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("创建云端直连客户端失败: {error}"),
            )
        }
    };
    let include_system = query.include_system.unwrap_or(false).to_string();
    let response = match client
        .get(url)
        .bearer_auth(user_token)
        .query(&[
            ("include_system", include_system.as_str()),
            ("node_id", credentials.agent_id.as_str()),
        ])
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("直连云端读取项目失败: {error}"),
            )
        }
    };

    let cloud_status = response.status();
    let cloud_payload = match response.json::<Value>().await {
        Ok(payload) => payload,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("云端项目响应不是有效 JSON: {error}"),
            )
        }
    };
    if !cloud_status.is_success() {
        let status = StatusCode::from_u16(cloud_status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        return error_response(
            status,
            cloud_error_message(&cloud_payload, cloud_status.as_u16()),
        );
    }

    match local_projects_payload(&credentials.agent_id, cloud_payload) {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error),
    }
}

fn local_projects_payload(node_id: &str, cloud_payload: Value) -> Result<Value, String> {
    let projects = cloud_payload
        .get("projects")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "云端项目响应缺少 projects 数组".to_string())?;
    Ok(json!({
        "ok": true,
        "node_id": node_id,
        "transport": "direct_reqwest_no_proxy",
        "projects": projects,
    }))
}

fn cloud_error_message(payload: &Value, status: u16) -> String {
    payload
        .get("error")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("云端项目接口返回 HTTP {status}"))
}

fn error_response(status: StatusCode, error: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": error.into() }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_payload_exposes_node_binding_without_credentials() {
        let payload = local_projects_payload(
            "node-current",
            json!({
                "projects": [{
                    "id": "project-bb64a",
                    "node_id": "node-current",
                    "workspace_path": "D:\\rust\\active-projects\\bb64a"
                }]
            }),
        )
        .expect("payload");

        assert_eq!(payload["node_id"], "node-current");
        assert_eq!(payload["transport"], "direct_reqwest_no_proxy");
        assert_eq!(payload["projects"][0]["node_id"], "node-current");
        assert!(payload.get("token").is_none());
        assert!(payload.get("user_token").is_none());
    }

    #[test]
    fn invalid_cloud_payload_is_rejected() {
        let error = local_projects_payload("node-current", json!({ "ok": true }))
            .expect_err("missing projects must fail");
        assert!(error.contains("projects"));
    }

    #[test]
    fn cloud_error_prefers_safe_error_field() {
        assert_eq!(
            cloud_error_message(&json!({ "error": "未登录" }), 401),
            "未登录"
        );
        assert_eq!(
            cloud_error_message(&json!({}), 502),
            "云端项目接口返回 HTTP 502"
        );
    }
}
