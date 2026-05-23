use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::types::AppState;
use crate::{admin, api, client_gateway, project_api, user_api, web};

pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(web::web_page))
        .route("/web", get(web::web_page))
        .route("/health", get(api::health))
        .route("/healthz", get(api::health))
        .route("/readyz", get(api::readyz))
        .route("/api/runtime", get(api::readyz))
        .route("/ws", get(client_gateway::ws_handler))
        .route("/api/chat", post(api::chat))
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
            "/ws/projects/:project_id",
            get(project_api::ws_project_handler),
        )
        .route(
            "/api/projects/:project_id/download/:filename",
            get(project_api::download_project_apk),
        )
        .route(
            "/download/:user_id/:filename",
            get(client_gateway::download_apk),
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
