use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeFile;

use crate::types::AppState;
use crate::{admin, api, app_update, peer_relay, project_api, user_api, web};

pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 应用自身 APK 更新文件目录（由发布脚本部署后填充）
    let app_dir = state.data_dir.join("app");
    let latest_apk = app_dir.join("ElonSpeed-latest.apk");

    Router::new()
        .route("/", get(web::web_page))
        .route("/web", get(web::web_page))
        .route("/health", get(api::health))
        .route("/healthz", get(api::health))
        .route("/readyz", get(api::readyz))
        .route("/api/runtime", get(api::readyz))
        .route("/api/server/version", get(api::server_version))
        .route("/api/debug/codex-health", get(api::codex_health))
        .route("/api/debug/traces/:trace_id", get(api::server_trace))
        .route("/api/image/generate", post(api::generate_image))
        .route("/api/auth/login", post(project_api::login))
        .route("/api/auth/register", post(project_api::register))
        .route("/api/me", get(project_api::me))
        .route("/api/me/projects", get(project_api::list_my_projects))
        .route("/api/projects", post(project_api::create_project))
        .route(
            "/api/projects/:project_id/chat",
            post(project_api::chat_project),
        )
        .route(
            "/api/projects/:project_id/prewarm",
            post(project_api::prewarm_project),
        )
        .route(
            "/api/projects/:project_id/attachments",
            post(project_api::upload_project_attachment),
        )
        .route(
            "/api/projects/:project_id/attachments/:conversation_id/:filename",
            get(project_api::download_project_attachment),
        )
        .route(
            "/ws/projects/:project_id",
            get(project_api::ws_project_handler),
        )
        .route(
            "/ws/user/:user_id/projects/:project_id",
            get(project_api::ws_user_project_handler),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/status",
            get(project_api::user_project_git_status),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/prewarm",
            post(project_api::prewarm_user_project),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/attachments",
            post(project_api::upload_user_project_attachment),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/attachments/:conversation_id/:filename",
            get(project_api::download_user_project_attachment),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/deploy-key",
            post(project_api::user_project_deploy_key),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/config",
            post(project_api::user_project_git_config),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/download/:filename",
            get(project_api::download_user_project_apk),
        )
        .route(
            "/api/projects/:project_id/download/:filename",
            get(project_api::download_project_apk),
        )
        // ── 应用自更新（Android 客户端检查版本 / 下载 APK）────────────────────
        .route("/app/download", get(web::download_page))
        .route("/download", get(web::download_page))
        .route(
            "/api/app/update/broadcast",
            post(app_update::broadcast_latest_update),
        )
        // version.json 动态生成（注入在线 seeder 的 mirrors 字段）
        .route("/app/version.json", get(peer_relay::version_json))
        .route_service("/app/ElonSpeed-latest.apk", ServeFile::new(latest_apk))
        // ── P2P 同WiFi 中继 ──────────────────────────────────────────────
        // Seeder 设备连接 WS 注册自己为种子
        .route("/app/peer/ws", get(peer_relay::peer_ws_handler))
        // 下载方通过服务器中继获取对应 seeder 的 APK
        .route("/app/relay/peer/:peer_id/apk", get(peer_relay::relay_apk))
        // ── homecli PC agent 反向 WSS 通道 ────────────────────────────
        .route("/agent/ws", get(crate::homecli_agent::agent_ws_handler))
        .route(
            "/api/_test_dispatch",
            post(crate::homecli_agent::test_dispatch),
        )
        .route("/admin", get(admin::admin_page))
        .route(
            "/api/admin/agents",
            get(admin::list_agents).post(admin::upsert_agent),
        )
        .route("/api/admin/agents/:name", delete(admin::delete_agent))
        .route("/api/admin/agents/:name/key", get(admin::get_agent_key))
        .route("/api/admin/default/:name", post(admin::set_default_agent))
        .route(
            "/api/admin/users",
            get(admin::list_users).post(admin::create_user),
        )
        .route(
            "/api/user/:user_id/agent",
            get(user_api::get_user_agent).put(user_api::set_user_agent),
        )
        .layer(cors)
        .with_state(state)
}
