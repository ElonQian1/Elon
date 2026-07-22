use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{node_agent_cloud_net, NodeRuntime};

#[derive(Debug, Default, Deserialize)]
struct LocalCloudProjectsQuery {
    include_system: Option<bool>,
    #[serde(default)]
    include_icons: bool,
}

#[derive(Debug, Deserialize)]
struct ProjectBindingRequest {
    project_id: String,
    workspace_path: String,
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/cloud-projects", get(list_local_cloud_projects))
        .route(
            "/api/cloud-projects/inspect-binding",
            post(inspect_project_binding),
        )
        .route("/api/cloud-projects/rebind", post(rebind_project))
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
    let started = Instant::now();
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

    match local_projects_payload(
        &credentials.agent_id,
        cloud_payload,
        query.include_icons,
        started.elapsed().as_millis(),
    ) {
        Ok(payload) => Json(payload).into_response(),
        Err(error) => error_response(StatusCode::BAD_GATEWAY, error),
    }
}

fn local_projects_payload(
    node_id: &str,
    cloud_payload: Value,
    include_icons: bool,
    cloud_round_trip_ms: u128,
) -> Result<Value, String> {
    let mut projects = cloud_payload
        .get("projects")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "云端项目响应缺少 projects 数组".to_string())?;
    if !include_icons {
        for project in &mut projects {
            if let Some(project) = project.as_object_mut() {
                project.remove("icon");
                project.remove("icon_url");
            }
        }
    }
    Ok(json!({
        "ok": true,
        "node_id": node_id,
        "transport": "direct_reqwest_no_proxy",
        "cloud_round_trip_ms": cloud_round_trip_ms,
        "project_count": projects.len(),
        "icons_included": include_icons,
        "projects": projects,
    }))
}

async fn inspect_project_binding(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<ProjectBindingRequest>,
) -> Response {
    match validated_binding(&runtime, &request).await {
        Ok(binding) => Json(json!({"ok": true, "binding": binding})).into_response(),
        Err((status, error)) => error_response(status, error),
    }
}

async fn rebind_project(
    State(runtime): State<Arc<NodeRuntime>>,
    Json(request): Json<ProjectBindingRequest>,
) -> Response {
    let started = Instant::now();
    let binding = match validated_binding(&runtime, &request).await {
        Ok(binding) => binding,
        Err((status, error)) => return error_response(status, error),
    };
    let Some(credentials) = runtime.creds().await else {
        return error_response(StatusCode::UNAUTHORIZED, "当前 PC 节点尚未登录");
    };
    let Some(user_token) = credentials
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return error_response(
            StatusCode::UNAUTHORIZED,
            "当前节点凭证不含登录 token，请重新登录",
        );
    };
    let canonical_path = binding["workspace_path"].as_str().unwrap_or_default();
    let url = format!(
        "{}/api/projects/{}/workspace/recover",
        runtime.cfg.cloud_http_url.trim_end_matches('/'),
        request.project_id.trim()
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
    let cloud_started = Instant::now();
    let response = match client
        .post(url)
        .bearer_auth(user_token)
        .json(&json!({
            "action": "bind_pc_node",
            "node_id": credentials.agent_id,
            "workspace_path": canonical_path,
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("直连云端绑定项目失败: {error}"),
            )
        }
    };
    let cloud_round_trip_ms = cloud_started.elapsed().as_millis();
    let status = response.status();
    let payload = match response.json::<Value>().await {
        Ok(payload) => payload,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                format!("云端绑定响应不是有效 JSON: {error}"),
            )
        }
    };
    if !status.is_success() {
        let status = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        return error_response(status, cloud_error_message(&payload, status.as_u16()));
    }
    let response_node = payload.get("node_id").and_then(Value::as_str);
    let response_path = payload.get("workspace_path").and_then(Value::as_str);
    if response_node != Some(credentials.agent_id.as_str())
        || !response_path.is_some_and(|path| same_path(path, canonical_path))
    {
        return error_response(
            StatusCode::BAD_GATEWAY,
            "云端绑定回执与已验证的节点或工作区不一致",
        );
    }
    Json(json!({
        "ok": true,
        "project_id": request.project_id,
        "binding": binding,
        "cloud_receipt": payload,
        "timings": {
            "inspect_and_validate_ms": cloud_started.duration_since(started).as_millis(),
            "cloud_round_trip_ms": cloud_round_trip_ms,
            "total_ms": started.elapsed().as_millis(),
        }
    }))
    .into_response()
}

async fn validated_binding(
    runtime: &NodeRuntime,
    request: &ProjectBindingRequest,
) -> Result<Value, (StatusCode, String)> {
    let project_id = request.project_id.trim();
    let requested_path = request.workspace_path.trim();
    if project_id.is_empty() || requested_path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "project_id 和 workspace_path 不能为空".to_string(),
        ));
    }
    let credentials = runtime
        .creds()
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "当前 PC 节点尚未登录".to_string()))?;
    let user_token = credentials
        .user_token
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "当前节点凭证不含登录 token，请重新登录".to_string(),
        ))?;
    let canonical = std::fs::canonicalize(PathBuf::from(requested_path)).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            format!("工作区无法规范化: {error}"),
        )
    })?;
    let canonical_path = canonical.to_string_lossy().to_string();
    let inspect_started = Instant::now();
    let inspection = crate::project_workspace_inspect::inspect_project_workspace(&canonical_path)
        .map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            format!("检查本机工作区失败: {error}"),
        )
    })?;
    if !inspection.path_exists || !inspection.is_dir || !inspection.is_git_worktree {
        return Err((
            StatusCode::CONFLICT,
            "工作区必须是已存在的 Git 工作树".to_string(),
        ));
    }
    if !inspection.codex_available && !inspection.copilot_available {
        return Err((
            StatusCode::CONFLICT,
            "工作区缺少可用的 Codex 或 Copilot CLI".to_string(),
        ));
    }
    let client =
        node_agent_cloud_net::direct_cloud_client(Duration::from_secs(15)).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("创建云端直连客户端失败: {error}"),
            )
        })?;
    let cloud_started = Instant::now();
    let response = client
        .get(format!(
            "{}/api/me/projects",
            runtime.cfg.cloud_http_url.trim_end_matches('/')
        ))
        .bearer_auth(user_token)
        .query(&[
            ("include_system", "false"),
            ("node_id", credentials.agent_id.as_str()),
        ])
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("直连云端读取项目失败: {error}"),
            )
        })?;
    let status = response.status();
    let payload = response.json::<Value>().await.map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("云端项目响应不是有效 JSON: {error}"),
        )
    })?;
    if !status.is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            cloud_error_message(&payload, status.as_u16()),
        ));
    }
    let project = payload
        .get("projects")
        .and_then(Value::as_array)
        .and_then(|projects| {
            projects
                .iter()
                .find(|project| project.get("id").and_then(Value::as_str) == Some(project_id))
        })
        .ok_or((
            StatusCode::NOT_FOUND,
            "当前账号无权访问指定项目".to_string(),
        ))?;
    let expected_origin = project
        .get("repo_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or((
            StatusCode::CONFLICT,
            "项目缺少可验证的 Git repo_url".to_string(),
        ))?;
    let actual_origin = inspection
        .git_remote_origin
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or((StatusCode::CONFLICT, "工作区缺少 Git origin".to_string()))?;
    if normalize_git_remote(expected_origin) != normalize_git_remote(actual_origin) {
        return Err((StatusCode::CONFLICT, format!("工作区 origin 与项目仓库不一致: expected={expected_origin}, actual={actual_origin}")));
    }
    Ok(json!({
        "project_id": project_id,
        "project_name": project.get("name").and_then(Value::as_str),
        "node_id": credentials.agent_id,
        "workspace_path": canonical_path,
        "git_branch": inspection.git_branch,
        "git_head": inspection.git_head,
        "git_remote_origin": inspection.git_remote_origin,
        "has_uncommitted_changes": inspection.has_uncommitted_changes,
        "cli": {"codex": inspection.codex_available, "copilot": inspection.copilot_available},
        "timings": {"local_inspect_ms": inspect_started.elapsed().as_millis(), "cloud_round_trip_ms": cloud_started.elapsed().as_millis()},
        "identity_verified": true,
    }))
}

fn normalize_git_remote(remote: &str) -> String {
    remote
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .replace("git@github.com:", "github.com/")
        .replace("ssh://git@github.com/", "github.com/")
        .replace("https://github.com/", "github.com/")
        .to_ascii_lowercase()
}

fn same_path(left: &str, right: &str) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| PathBuf::from(left));
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| PathBuf::from(right));
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
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
            false,
            17,
        )
        .expect("payload");

        assert_eq!(payload["node_id"], "node-current");
        assert_eq!(payload["transport"], "direct_reqwest_no_proxy");
        assert_eq!(payload["cloud_round_trip_ms"], 17);
        assert_eq!(payload["projects"][0]["node_id"], "node-current");
        assert!(payload.get("token").is_none());
        assert!(payload.get("user_token").is_none());
    }

    #[test]
    fn project_payload_omits_large_icons_by_default() {
        let payload = local_projects_payload(
            "node",
            json!({"projects":[{"id":"p","icon":"data:image/png;base64,large"}]}),
            false,
            1,
        )
        .unwrap();
        assert!(payload["projects"][0].get("icon").is_none());
        assert_eq!(payload["icons_included"], false);
    }

    #[test]
    fn git_remote_normalization_accepts_ssh_and_https_equivalents() {
        assert_eq!(
            normalize_git_remote("git@github.com:ElonQian1/Elon.git"),
            normalize_git_remote("https://github.com/ElonQian1/Elon.git")
        );
    }

    #[test]
    fn invalid_cloud_payload_is_rejected() {
        let error = local_projects_payload("node-current", json!({ "ok": true }), false, 0)
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
