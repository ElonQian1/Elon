//! PC 节点本地管理员 HTTP 服务器（登录/注销/设置/诊断等）。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use super::{
    node_agent_admin_status,
    node_agent_api_runtime_config,
    node_agent_cli_sidecar_admin,
    node_agent_client_diagnostics,
    node_agent_client_maintenance,
    node_agent_cloud_net,
    node_agent_codex_vault,
    node_agent_download_router,
    node_agent_full_access,
    node_agent_install_env,
    node_agent_local_admin,
    node_agent_local_pc_frontend,
    node_agent_project_agent_runs,
    node_agent_project_picker,
    node_agent_task_journal_api,
    pc_storage_git_http,
    pc_storage_repo,
    project_landing,
    project_workspace_inspect,
    windows_doctor,
    NodeRuntime,
};
use super::node_agent_config::cloud_login;
use super::node_agent_env::node_agent_env_file_path;
use super::node_agent_registration::provision_node;

pub(super) fn spawn_admin_server(runtime: Arc<NodeRuntime>, port: u16) {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    tokio::spawn(async move {
        let cors = node_agent_local_admin::cors_layer(&runtime.cfg.cloud_http_url);
        let local_admin_guard = axum::middleware::from_fn_with_state(
            runtime.clone(),
            node_agent_local_admin::require_local_admin,
        );
        let protected_routes = axum::Router::new()
            .route("/api/env-check", axum::routing::get(admin_env_check))
            .route(
                "/api/install-env",
                axum::routing::post(node_agent_install_env::admin_install_env),
            )
            .route(
                "/api/codex-cli/refresh",
                axum::routing::post(admin_codex_cli_refresh),
            )
            .route(
                "/api/doctor/snapshot",
                axum::routing::get(windows_doctor::snapshot_handler),
            )
            .route(
                "/api/doctor/analyze",
                axum::routing::post(windows_doctor::analyze_handler),
            )
            .route(
                "/api/doctor/sessions",
                axum::routing::get(windows_doctor::sessions_list_handler)
                    .post(windows_doctor::session_create_handler),
            )
            .route(
                "/api/doctor/sessions/:session_id",
                axum::routing::get(windows_doctor::session_get_handler)
                    .delete(windows_doctor::session_delete_handler),
            )
            .route(
                "/api/doctor/memory",
                axum::routing::get(windows_doctor::memory_list_handler)
                    .post(windows_doctor::memory_save_handler),
            )
            .route(
                "/api/doctor/repair",
                axum::routing::post(windows_doctor::repair_handler),
            )
            .merge(node_agent_download_router::routes())
            .route(
                "/api/save-openai-key",
                axum::routing::post(admin_save_openai_key),
            )
            .route("/api/login", axum::routing::post(admin_login))
            .route("/api/logout", axum::routing::post(admin_logout))
            .route(
                "/api/register-project",
                axum::routing::post(admin_register_project),
            )
            .merge(node_agent_cli_sidecar_admin::routes())
            .merge(node_agent_codex_vault::routes())
            .merge(node_agent_task_journal_api::routes())
            .route(
                "/api/project-folder/pick",
                axum::routing::post(node_agent_project_picker::pick_local_project_folder),
            )
            .route(
                "/api/project-folder/default",
                axum::routing::post(node_agent_project_picker::prepare_default_project_folder),
            )
            .route(
                "/api/project-folder/inspect",
                axum::routing::post(node_agent_project_picker::inspect_local_project_folder),
            )
            .route(
                "/api/project-agent-runs",
                axum::routing::post(node_agent_project_agent_runs::list_handler),
            )
            .route(
                "/api/full-access/grants",
                axum::routing::get(node_agent_full_access::list_handler)
                    .post(node_agent_full_access::grant_handler),
            )
            .route(
                "/api/client-maintenance",
                axum::routing::get(node_agent_client_maintenance::status_handler),
            )
            .route(
                "/api/client-maintenance/autostart",
                axum::routing::get(node_agent_client_maintenance::autostart_status_handler)
                    .post(node_agent_client_maintenance::autostart_set_handler),
            )
            .route(
                "/api/client-maintenance/open",
                axum::routing::post(node_agent_client_maintenance::open_target_handler),
            )
            .route(
                "/api/client-maintenance/diagnostics/export",
                axum::routing::post(node_agent_client_diagnostics::export_handler),
            )
            .route(
                "/api/client-maintenance/update",
                axum::routing::post(node_agent_client_maintenance::update_handler),
            )
            .route(
                "/api/client-maintenance/repair",
                axum::routing::post(node_agent_client_maintenance::repair_handler),
            )
            .route(
                "/api/client-maintenance/uninstall",
                axum::routing::post(node_agent_client_maintenance::uninstall_handler),
            )
            .route(
                "/api/storage-config",
                axum::routing::get(admin_storage_config_get).post(admin_storage_config_set),
            )
            .route("/api/tts-status", axum::routing::get(admin_tts_status))
            .route(
                "/api/tts-relay-config",
                axum::routing::get(admin_tts_relay_get).post(admin_tts_relay_set),
            )
            .route_layer(local_admin_guard);
        let app = axum::Router::new()
            .merge(node_agent_local_pc_frontend::routes())
            .route(
                "/api/status",
                axum::routing::get(node_agent_admin_status::admin_status),
            )
            .route(
                "/storage/git/:token/*path",
                axum::routing::any(admin_storage_git_http),
            )
            .merge(protected_routes)
            .with_state(runtime)
            .layer(cors)
            .layer(node_agent_local_admin::private_network_header_layer());
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                info!("🖥️  本地 PC 工作台: http://127.0.0.1:{}/pc", port);
                if let Err(e) = axum::serve(listener, app).await {
                    warn!("admin server 退出: {e}");
                }
            }
            Err(e) => warn!("admin server 无法监听 {addr}: {e}"),
        }
    });
}

async fn admin_tts_relay_get(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let url = rt.tts_worker_url.read().await.clone();
    axum::Json(serde_json::json!({ "ttsWorkerUrl": url }))
}

#[derive(serde::Deserialize)]
struct TtsRelaySetReq {
    tts_worker_url: Option<String>,
}

async fn admin_tts_relay_set(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<TtsRelaySetReq>,
) -> axum::Json<serde_json::Value> {
    let url = req
        .tts_worker_url
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty());
    *rt.tts_worker_url.write().await = url.clone();
    rt.wake.notify_one();
    axum::Json(serde_json::json!({ "ok": true, "ttsWorkerUrl": url }))
}

async fn admin_storage_config_get(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let settings = rt.storage_settings.read().await.clone();
    let profile = pc_storage_repo::storage_profile(&settings);
    axum::Json(serde_json::json!({
        "enabled": settings.enabled,
        "root_path": settings.root_path,
        "git_base_url": settings.git_base_url,
        "profile": profile,
    }))
}

#[derive(serde::Deserialize)]
struct StorageConfigSetReq {
    enabled: Option<bool>,
    root_path: Option<String>,
    git_base_url: Option<String>,
}

async fn admin_storage_config_set(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<StorageConfigSetReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let enabled = req.enabled.unwrap_or(false);
    let root_path = clean_optional_admin_field(req.root_path.as_deref()).or_else(|| {
        enabled.then(|| {
            pc_storage_repo::default_storage_root()
                .to_string_lossy()
                .to_string()
        })
    });
    if enabled {
        if let Some(root) = root_path.as_deref() {
            if let Err(e) = std::fs::create_dir_all(root) {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "ok": false,
                        "error": format!("创建硬盘服务目录失败: {e}"),
                    })),
                );
            }
        }
    }
    let settings = pc_storage_repo::StorageSettings {
        enabled,
        root_path,
        git_base_url: clean_optional_admin_field(req.git_base_url.as_deref()),
    };
    rt.set_storage_settings(settings.clone()).await;
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "enabled": settings.enabled,
            "root_path": settings.root_path,
            "git_base_url": settings.git_base_url,
            "profile": pc_storage_repo::storage_profile(&settings),
        })),
    )
}

async fn admin_tts_status() -> axum::Json<serde_json::Value> {
    let port = std::env::var("ELON_TTS_WORKER_PORT")
        .or_else(|_| std::env::var("TTS_WORKER_PORT"))
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(5011);
    let enabled = std::env::var("TTS_WORKER_ENABLED")
        .map(|v| {
            matches!(
                v.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    let url = format!("http://127.0.0.1:{}/health", port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                return axum::Json(serde_json::json!({
                    "running": true,
                    "enabled_in_env": enabled,
                    "port": port,
                    "health": body,
                }));
            }
            axum::Json(
                serde_json::json!({ "running": true, "enabled_in_env": enabled, "port": port }),
            )
        }
        _ => axum::Json(serde_json::json!({
            "running": false,
            "enabled_in_env": enabled,
            "port": port,
        })),
    }
}

async fn admin_codex_cli_refresh(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let cli_probe = rt.refresh_cli_probe_now().await;
    axum::Json(serde_json::json!({
        "ok": true,
        "cli_probe": {
            "refreshing": rt.cli_probe_refreshing.load(Ordering::Acquire),
            "refreshed_at_ms": cli_probe.refreshed_at_ms,
            "stale": cli_probe.is_stale(),
        },
        "codex_cli": cli_probe.codex_status(),
        "allowed_clis": cli_probe.available_names(),
        "cli_tools": cli_probe.tools,
    }))
}

#[derive(Deserialize)]
struct AdminLoginReq {
    /// 账号（手机号/邮箱），搭配 password 登录
    account: Option<String>,
    password: Option<String>,
    /// 或直接粘贴已有的 elon 登录 token
    token: Option<String>,
}

/// 本地管理页 → 登录并自动注册节点。
/// 流程：账号+密码换 token（或直接用粘贴的 token）→ 调用云端注册节点拿 agent_id+secret → 持久化 → 唤醒重连。
async fn admin_login(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<AdminLoginReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    // 1) 取得 token：优先直接粘贴的 token，否则账号+密码登录
    let token = if let Some(t) = req
        .token
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        t
    } else {
        let account = req.account.unwrap_or_default();
        let account = account.trim();
        let password = req.password.unwrap_or_default();
        if account.is_empty() || password.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(
                    serde_json::json!({ "ok": false, "error": "请填写账号和密码，或直接粘贴 token" }),
                ),
            );
        }
        match cloud_login(&rt.cfg, account, &password).await {
            Ok(t) => t,
            Err(e) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(
                        serde_json::json!({ "ok": false, "error": format!("登录失败: {e}") }),
                    ),
                );
            }
        }
    };

    // 2) 用 token 注册/换取节点凭证；若已有凭证则尝试续约（保留 agent_id）
    let existing = rt.creds.read().await.clone();
    match provision_node(&rt.cfg, &token, existing.as_ref(), &rt.install_id).await {
        Ok(c) => {
            let agent_id = c.agent_id.clone();
            rt.set_creds(Some(c)).await;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({ "ok": true, "agent_id": agent_id })),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({ "ok": false, "error": format!("注册节点失败: {e}") })),
        ),
    }
}

/// 本地管理页 → 登出：清除本地凭证并断开。
async fn admin_logout(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    rt.set_creds(None).await;
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
}

#[derive(Deserialize)]
struct AdminRegisterReq {
    project_id: Option<String>,
    name: String,
    workspace_path: String,
    description: Option<String>,
    repo_url: Option<String>,
    branch: Option<String>,
    dev_profile: Option<serde_json::Value>,
}

/// 本地管理页 → 注册外部本地项目到云端。
/// 流程：
///   1. 在 PC 本地校验路径存在且为目录（这是关键 —— 服务器看不到 PC 路径）
///   2. 用 NODE_USER_TOKEN 调用云端 POST /api/projects/external，附带 node_id 让服务器跳过路径校验
async fn admin_register_project(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<AdminRegisterReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    let name = req.name.trim();
    let path = req.workspace_path.trim();
    if name.is_empty() || path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(
                serde_json::json!({ "ok": false, "error": "name 和 workspace_path 不能为空" }),
            ),
        );
    }

    // 1) PC 本地校验
    let pb = std::path::Path::new(path);
    if !pb.exists() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": format!("PC 本地路径不存在: {}", path),
            })),
        );
    }
    if !pb.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "ok": false, "error": "workspace_path 必须是目录" })),
        );
    }
    let inspect = project_workspace_inspect::inspect_project_workspace(path).ok();
    let repo_url = clean_optional_admin_field(req.repo_url.as_deref())
        .or_else(|| {
            inspect
                .as_ref()
                .and_then(|status| status.git_remote_origin.clone())
        })
        .or_else(|| git_value_at(pb, &["remote", "get-url", "origin"]));
    let branch = clean_optional_admin_field(req.branch.as_deref())
        .or_else(|| {
            inspect
                .as_ref()
                .and_then(|status| status.git_branch.clone())
        })
        .or_else(|| {
            git_value_at(pb, &["rev-parse", "--abbrev-ref", "HEAD"]).filter(|value| value != "HEAD")
        });

    // 2) 必须已登录（有凭证 + token）才能调用云端
    let creds = match rt.creds().await {
        Some(c) => c,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": "尚未登录，请先在页面顶部用账号密码登录。",
                })),
            );
        }
    };
    let token = match creds.user_token.as_ref() {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": "当前节点凭证不含登录 token，请在页面顶部重新登录。",
                })),
            );
        }
    };

    // 3) 转发到云端
    let url = format!(
        "{}/api/projects/external",
        rt.cfg.cloud_http_url.trim_end_matches('/')
    );
    let body = serde_json::json!({
        "project_id": req.project_id.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "name": name,
        "workspace_path": path,
        "description": req.description,
        "node_id": creds.agent_id,
        "repo_url": repo_url,
        "branch": branch,
        "landing": project_landing::load_workspace_landing(pb),
        "dev_profile": req.dev_profile,
    });
    let client = node_agent_cloud_net::direct_cloud_client_or_default(Duration::from_secs(15));
    match client
        .post(&url)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
            if status.is_success() {
                (
                    StatusCode::OK,
                    axum::Json(serde_json::json!({
                        "ok": true,
                        "cloud": json,
                    })),
                )
            } else {
                (
                    StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                    axum::Json(serde_json::json!({
                        "ok": false,
                        "error": format!("云端返回 {}: {}", status, json),
                    })),
                )
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "ok": false,
                "error": format!("调用云端失败: {}", e),
            })),
        ),
    }
}

async fn admin_storage_git_http(
    axum::extract::State(rt): axum::extract::State<Arc<NodeRuntime>>,
    req: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    let settings = rt.storage_settings.read().await.clone();
    pc_storage_git_http::handle_git_http(settings, req).await
}

fn clean_optional_admin_field(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn git_value_at(path: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = elon_pc_dev_runtime::command_output("git", args, Some(path)).ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

// ── AI 编码工具 & Android 环境检查 / 安装 ────────────────────────────────────

/// 检查单个命令行工具是否可用（PATH + 常见安装目录双路扫描）。
fn tool_available(bin: &str) -> bool {
    if elon_pc_dev_runtime::command_path(bin).is_some() {
        return true;
    }
    false
}

/// 检查 Android SDK 是否配置好（platforms/android-34 + build-tools/34.0.0）。
fn android_sdk_ready() -> bool {
    let candidates: Vec<String> = [
        std::env::var("ANDROID_HOME").ok(),
        std::env::var("ANDROID_SDK_ROOT").ok(),
        // Windows 默认路径
        #[cfg(windows)]
        std::env::var("LOCALAPPDATA")
            .ok()
            .map(|p| format!("{}\\Android\\Sdk", p)),
        #[cfg(not(windows))]
        Some(format!(
            "{}/android-sdk",
            std::env::var("HOME").unwrap_or_default()
        )),
    ]
    .into_iter()
    .flatten()
    .collect();

    candidates.iter().any(|base| {
        std::path::Path::new(base)
            .join("platforms")
            .join("android-34")
            .exists()
    })
}

/// 检查 Gradle 阿里云镜像是否已配置。
fn gradle_mirror_ok() -> bool {
    let home =
        std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let init = std::path::PathBuf::from(&home)
        .join(".gradle")
        .join("init.gradle");
    if !init.exists() {
        return false;
    }
    std::fs::read_to_string(&init)
        .map(|s| s.contains("maven.aliyun.com"))
        .unwrap_or(false)
}

/// GET /api/env-check — 返回各工具安装状态。
async fn admin_env_check(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
) -> axum::Json<serde_json::Value> {
    let result = tokio::task::spawn_blocking(|| {
        let api_runtime = node_agent_api_runtime_config::status_from_env();
        let api_runtime_contract = node_agent_api_runtime_config::tool_contract();
        serde_json::json!({
            "git":          tool_available("git"),
            "java":         tool_available("java"),
            "node":         tool_available("node"),
            "npm":          tool_available("npm"),
            "codex":        tool_available("codex"),
            "copilot":      tool_available("copilot"),
            "claude":       tool_available("claude"),
            "gemini":       tool_available("gemini"),
            "android_sdk":  android_sdk_ready(),
            "gradle_mirror": gradle_mirror_ok(),
            "ollama":       tool_available("ollama"),
            "openai_key":   api_runtime.key_configured,
            "api_runtime_key": api_runtime.key_configured,
            "api_runtime_model": api_runtime.model,
            "api_runtime_model_configured": api_runtime.model_configured,
            "api_runtime_base": api_runtime.api_base,
            "api_runtime_ready": api_runtime.ready,
            "api_runtime_contract": api_runtime_contract,
        })
    })
    .await
    .unwrap_or_else(|_| serde_json::json!({}));
    axum::Json(result)
}

#[derive(Deserialize)]
struct SaveOpenAiKeyReq {
    api_key: String,
    model: Option<String>,
    api_base: Option<String>,
    base_url: Option<String>,
}

/// POST /api/save-openai-key — 保存本机 API key / Codex 共用的 OpenAI-compatible 配置。
async fn admin_save_openai_key(
    axum::extract::State(_rt): axum::extract::State<Arc<NodeRuntime>>,
    axum::Json(req): axum::Json<SaveOpenAiKeyReq>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    use axum::http::StatusCode;

    let base = req.api_base.as_deref().or(req.base_url.as_deref());
    let save = match node_agent_api_runtime_config::validate_save(
        &req.api_key,
        req.model.as_deref(),
        base,
    ) {
        Ok(save) => save,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": error.to_string()
                })),
            );
        }
    };

    // 当前进程立即生效：本机 API key 运行方式和 Codex 子进程都会继承。
    node_agent_api_runtime_config::apply_to_process(&save);

    // 持久化到启动器实际读取的 _internal/node-agent.env，避免重启后本机 API key 配置丢失。
    if let Some(env_file) = node_agent_env_file_path() {
        if let Err(error) = node_agent_api_runtime_config::persist_to_env_file(&env_file, &save) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "ok": false,
                    "error": error.to_string()
                })),
            );
        }
    }

    let status = node_agent_api_runtime_config::status_from_env();
    let contract = node_agent_api_runtime_config::tool_contract();
    let msg = if status.ready {
        "我的 API key 已就绪，Codex 也会继承该 API key"
    } else {
        "API key 已保存；还需要配置模型后才会就绪"
    };
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "msg": msg,
            "api_runtime_ready": status.ready,
            "api_runtime_model": status.model,
            "api_runtime_base": status.api_base,
            "api_runtime_contract": contract,
        })),
    )
}
