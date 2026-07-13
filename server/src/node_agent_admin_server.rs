//! PC 节点本地管理员 HTTP 服务器（登录/注销/设置/诊断等）。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tracing::{info, warn};

use super::node_agent_config::cloud_login;
use super::node_agent_env::node_agent_env_file_path;
use super::node_agent_registration::provision_node;
use super::{
    node_agent_admin_status, node_agent_android_inspector, node_agent_android_live,
    node_agent_api_runtime_config, node_agent_cli_sidecar_admin, node_agent_client_diagnostics,
    node_agent_client_maintenance, node_agent_cloud_net, node_agent_codex_vault,
    node_agent_data_root,
    node_agent_download_router, node_agent_full_access, node_agent_install_env,
    node_agent_local_admin, node_agent_local_pc_frontend, node_agent_local_tasks,
    node_agent_project_agent_runs, node_agent_project_picker, node_agent_source_preview,
    node_agent_task_journal_api, pc_storage_git_http, pc_storage_repo, project_landing,
    project_workspace_inspect, windows_doctor, NodeRuntime,
};

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
            .merge(node_agent_android_inspector::routes())
            .merge(node_agent_android_live::protected_routes())
            .merge(node_agent_source_preview::routes())
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
            .merge(node_agent_local_tasks::routes())
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
            .route(
                "/api/node-data-root",
                axum::routing::get(node_agent_data_root::admin::get)
                    .post(node_agent_data_root::admin::set),
            )
            .route(
                "/api/node-data-root/cleanup",
                axum::routing::post(node_agent_data_root::admin::cleanup),
            )
            .route("/api/tts-status", axum::routing::get(admin_tts_status))
            .route(
                "/api/tts-relay-config",
                axum::routing::get(admin_tts_relay_get).post(admin_tts_relay_set),
            )
            .route_layer(local_admin_guard);
        let app = axum::Router::new()
            .merge(node_agent_local_pc_frontend::routes())
            .merge(node_agent_android_live::runtime_routes())
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

#[path = "node_agent_admin_server_handlers.rs"]
mod handlers;
pub(super) use self::handlers::*;
