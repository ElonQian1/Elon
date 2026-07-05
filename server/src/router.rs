// server/src/router.rs
use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderValue},
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::types::AppState;
use crate::{
    admin, admin_quota, admin_token_stats, agent_balloon, api, app_update, auth_api, billing_admin,
    billing_api, billing_pay, chat_attachments, codex_vault_api, context_compiler,
    external_app_api, external_app_chat_bootstrap, external_app_mvp_chat, external_app_route_c_sdk,
    external_app_tool_report_api, friend_api, global_ws, group_ai, group_chat_retrieval_api,
    group_summary_api, lan_peer, lm_chat, peer_relay, project_api, project_attachments,
    project_channels, project_chat, project_conversation_identity, project_deletion, project_docs,
    project_git, project_join_requests, project_landing_api, project_membership, project_releases,
    project_runtime_permission_api, project_space, project_space_task_snapshot,
    project_storage_git, project_store, project_workspace_health, project_workspace_recovery,
    release_claim, server_agent_runtime, speech_translate, token_usage_api, user_api,
    user_archive_api, user_memory_api, user_progression, voice_asr_upload, voice_tts_api,
    voice_ws_realtime_chat, voice_ws_transcribe, voice_ws_virtual_mic, web,
};

mod node_routes;

/// 读取 `CORS_ALLOW_ORIGINS` 环境变量构造 CORS 策略。
///
/// - 未设置或值为 `*` → 允许所有来源（开发友好，生产环境建议显式配置）
/// - 逗号分隔的 Origin 列表 → 仅允许列出的来源，其余拒绝
fn build_cors() -> CorsLayer {
    let origins_env = std::env::var("CORS_ALLOW_ORIGINS").unwrap_or_default();
    let allow_origin = if origins_env.is_empty() || origins_env.trim() == "*" {
        AllowOrigin::any()
    } else {
        let list: Vec<axum::http::HeaderValue> = origins_env
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if list.is_empty() {
            AllowOrigin::any()
        } else {
            AllowOrigin::list(list)
        }
    };
    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(Any)
        .allow_headers(Any)
}

pub fn build_app(state: Arc<AppState>) -> Router {
    let cors = build_cors();

    // 应用自身 APK 更新文件目录（由发布脚本部署后填充）
    let app_dir = state.data_dir.join("app");
    let latest_apk = app_dir.join("ElonSpeed-latest.apk");

    // PC 新前端 dist 目录（由发布脚本构建并上传后填充）
    // /pc 是主路由；/pc-next 保留为向后兼容别名。
    // /pc-legacy 是发布脚本从新框架引入前的历史提交导出的只读对照快照。
    let pc_next_dist = state.data_dir.join("pc-next-dist");
    let pc_legacy_dist = state.data_dir.join("pc-legacy-dist");

    // HTML 入口：no-cache 确保浏览器每次都拉最新 HTML（JS/CSS 有 hash 可正常缓存）
    let no_cache = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    // /pc: assets 用 ServeDir，其余路径 fallback 到 index.html，支持强刷子路由。
    let pc_assets_svc = tower::ServiceBuilder::new()
        .layer(no_cache.clone())
        .service(ServeDir::new(pc_next_dist.join("assets")));
    let pc_router = axum::Router::new()
        .route("/pc-workbench-sw.js", get(web::pc_workbench_service_worker))
        .nest_service("/assets", pc_assets_svc)
        .fallback(web::pc_spa_index)
        .with_state(Arc::clone(&state));
    // /pc-next 保持向后兼容（与 /pc 相同）
    let pc_next_assets_svc = tower::ServiceBuilder::new()
        .layer(no_cache.clone())
        .service(ServeDir::new(pc_next_dist.join("assets")));
    let pc_next_router = axum::Router::new()
        .nest_service("/assets", pc_next_assets_svc)
        .fallback(web::pc_spa_index)
        .with_state(Arc::clone(&state));
    let pc_legacy_svc = tower::ServiceBuilder::new().layer(no_cache).service(
        ServeDir::new(&pc_legacy_dist)
            .not_found_service(ServeFile::new(pc_legacy_dist.join("index.html"))),
    );

    Router::new()
        .route("/", get(web::web_page))
        .route("/web", get(web::web_page))
        .route("/favicon.ico", get(web::favicon))
        .nest_service("/pc", pc_router)
        .nest_service("/pc-next", pc_next_router)
        .nest_service("/pc-legacy", pc_legacy_svc)
        .route("/manifest.json", get(web::pwa_manifest))
        .route("/sw.js", get(web::service_worker))
        .route("/assets/project_plaza.css", get(web::project_plaza_css))
        .route("/assets/project_plaza.js", get(web::project_plaza_js))
        .route(
            "/assets/ic_plaza_enter_space.png",
            get(web::project_plaza_enter_space_icon),
        )
        .route(
            "/assets/ic_plaza_share_project.png",
            get(web::project_plaza_share_project_icon),
        )
        .route(
            "/assets/ic_plaza_download_apk.png",
            get(web::project_plaza_download_apk_icon),
        )
        .route(
            "/assets/ic_plaza_member_stat.png",
            get(web::project_plaza_member_stat_icon),
        )
        .route(
            "/assets/ic_side_menu_folder_closed.png",
            get(web::side_menu_folder_closed_icon),
        )
        .route(
            "/assets/ic_project_members_toolbar.png",
            get(web::project_members_toolbar_icon),
        )
        .route(
            "/assets/ic_project_space_post_share.png",
            get(web::project_space_post_share_icon),
        )
        .route(
            "/assets/ic_project_space_post_comment.png",
            get(web::project_space_post_comment_icon),
        )
        .route(
            "/assets/ic_project_space_post_like.png",
            get(web::project_space_post_like_icon),
        )
        .route(
            "/assets/ic_project_post_compose.png",
            get(web::project_post_compose_icon),
        )
        .route(
            "/assets/ic_project_preview_placeholder.png",
            get(web::project_preview_placeholder_icon),
        )
        .route(
            "/assets/ic_add_friend_scan.png",
            get(web::add_friend_scan_icon),
        )
        .route("/assets/project_home.css", get(web::project_home_css))
        .route("/assets/project_home.js", get(web::project_home_js))
        .route("/assets/voice_tts_sdk.js", get(web::voice_tts_sdk_js))
        .route("/assets/elon_route_c_sdk.js", get(web::elon_route_c_sdk_js))
        .route("/health", get(api::health))
        .route("/healthz", get(api::health))
        .route("/readyz", get(api::readyz))
        .route("/api/runtime", get(api::readyz))
        .route("/api/server/version", get(api::server_version))
        .route("/api/release/claim", post(release_claim::claim_handler))
        .route(
            "/api/release/heartbeat",
            post(release_claim::heartbeat_handler),
        )
        .route("/api/release/finish", post(release_claim::finish_handler))
        .route("/api/release/status", get(release_claim::status_handler))
        .route("/api/debug/codex-health", get(api::codex_health))
        .route("/api/debug/traces/:trace_id", get(api::server_trace))
        .route("/api/image/generate", post(api::generate_image))
        .route(
            "/api/agent/runtime/chat",
            post(server_agent_runtime::chat_handler),
        )
        .route(
            "/api/agent/runtime/status",
            get(server_agent_runtime::status_handler),
        )
        .route("/api/auth/login", post(auth_api::login))
        .route("/api/auth/register", post(auth_api::register))
        .route("/api/me", get(auth_api::me))
        .merge(codex_vault_api::routes())
        .route("/api/me/progression", get(user_progression::get_my_progression))
        .route(
            "/api/external/apps/:app_id",
            get(external_app_api::get_external_app),
        )
        .route(
            "/api/external/apps/:app_id/context-contract",
            get(external_app_api::get_external_app_context_contract),
        )
        .route(
            "/api/external/apps/:app_id/chat-bootstrap",
            get(external_app_chat_bootstrap::get_chat_bootstrap),
        )
        .route(
            "/api/external/apps/:app_id/mvp-chat",
            post(external_app_mvp_chat::mvp_chat_handler),
        )
        .route(
            "/api/external/apps/:app_id/route-c/chat",
            post(external_app_route_c_sdk::route_c_chat_handler),
        )
        .route(
            "/api/external/apps/:app_id/tool-executions",
            get(external_app_tool_report_api::get_external_app_tool_execution_report),
        )
        .route(
            "/api/external/apps/:app_id/accounts/lookup",
            post(external_app_api::lookup_external_account),
        )
        .route(
            "/api/external/apps/:app_id/accounts/sync",
            post(external_app_api::sync_external_account),
        )
        .route(
            "/api/external/apps/:app_id/accounts/session",
            post(external_app_api::create_external_account_session),
        )
        .route(
            "/api/external/apps/:app_id/authorize",
            post(external_app_api::authorize_external_app),
        )
        .route(
            "/api/external/apps/:app_id/authorize/exchange",
            post(external_app_api::exchange_external_app_authorization),
        )
        .route(
            "/api/me/profile",
            axum::routing::patch(project_api::update_profile),
        )
        .route(
            "/api/me/presence",
            get(user_api::get_my_presence).patch(user_api::update_my_presence),
        )
        .route("/api/me/archive", get(user_archive_api::get_user_archive))
        .route("/api/me/workspaces", get(user_archive_api::get_user_archive))
        .route("/api/memories", get(user_memory_api::list_memories).post(user_memory_api::create_memory))
        .route("/api/memories/:id", delete(user_memory_api::delete_memory))
        .merge(node_routes::routes())
        .route(
            "/api/agent-balloon/ensure",
            post(agent_balloon::ensure_balloon_project),
        )
        .route("/api/llm/chat", post(lm_chat::lm_chat_handler))
        .route("/api/me/ai/conversations", get(lm_chat::list_ai_chat_conversations))
        .route(
            "/api/me/ai/conversations/:conversation_id/messages",
            get(lm_chat::list_ai_chat_conversation_messages),
        )
        // ───────────────────────────────────────────────────────────────────────
        .route(
            "/api/me/friends",
            get(friend_api::list_friends).post(friend_api::add_friend_by_phone),
        )
        .route(
            "/api/me/friends/search",
            get(friend_api::search_friend_by_phone),
        )
        .route(
            "/api/me/friends/recommendations",
            get(friend_api::list_friend_recommendations),
        )
        .route(
            "/api/me/project-share-messages/:project_id",
            delete(friend_api::delete_project_share_messages),
        )
        .route(
            "/api/me/friends/:friend_id/messages",
            get(friend_api::list_friend_messages).post(friend_api::send_friend_message),
        )
        .route(
            "/api/me/friends/:friend_id/messages/:message_id",
            delete(friend_api::delete_friend_message),
        )
        .route(
            "/api/me/friends/:friend_id/messages/:message_id/ai-reply",
            post(friend_api::request_friend_ai_reply),
        )
        .route(
            "/api/me/groups",
            get(friend_api::list_friend_groups).post(friend_api::create_friend_group),
        )
        .route(
            "/api/me/groups/:group_id/members",
            post(friend_api::add_group_members),
        )
        .route(
            "/api/me/groups/:group_id/messages",
            get(friend_api::list_friend_group_messages).post(friend_api::send_friend_group_message),
        )
        .route(
            "/api/me/groups/:group_id/messages/search",
            post(group_chat_retrieval_api::search_group_chat_messages),
        )
        .route(
            "/api/me/groups/:group_id/messages/:message_id",
            delete(friend_api::delete_friend_group_message),
        )
        .route(
            "/api/me/groups/:group_id/messages/:message_id/ai-reply",
            post(friend_api::request_group_ai_reply),
        )
        .route(
            "/api/me/groups/:group_id/ai-docs",
            get(group_summary_api::list_group_ai_documents)
                .post(group_summary_api::update_group_ai_document),
        )
        .route(
            "/api/me/groups/:group_id/summary-posts",
            get(group_summary_api::list_group_summary_posts)
                .post(group_summary_api::create_group_summary_post),
        )
        .route(
            "/api/me/groups/:group_id/summary-posts/auto-split",
            post(group_summary_api::auto_split_group_summary_posts),
        )
        .route(
            "/api/me/groups/:group_id/summary-posts/:post_id",
            get(group_summary_api::get_group_summary_post)
                .patch(group_summary_api::update_group_summary_post),
        )
        .route("/api/me/projects", get(project_api::list_my_projects))
        .route("/api/projects", post(project_api::create_project))
        .route(
            "/api/projects/external",
            post(project_api::register_external_project),
        )
        .route("/api/projects/:id", delete(project_deletion::delete_project))
        // ── 项目商店 ─────────────────────────────────────────────────────
        .route(
            "/api/store/projects",
            get(project_store::list_store_projects),
        )
        .route(
            "/api/store/projects/:id",
            get(project_store::get_store_project),
        )
        .route(
            "/api/store/joined",
            get(project_store::list_joined_projects),
        )
        // ── 成员管理 ─────────────────────────────────────────────────────
        .route(
            "/api/projects/:id/join",
            post(project_membership::join_project),
        )
        .route(
            "/api/projects/:id/leave",
            delete(project_membership::leave_project),
        )
        .route(
            "/api/projects/:id/members",
            get(project_membership::list_members).post(project_membership::add_member),
        )
        .route(
            "/api/projects/:id/invite-links",
            get(project_membership::list_project_invite_links)
                .post(project_membership::create_project_invite_link),
        )
        .route(
            "/api/projects/:id/invite-links/:code",
            delete(project_membership::revoke_project_invite_link),
        )
        .route(
            "/api/project-invites/:code",
            get(project_membership::get_project_invite_preview),
        )
        .route(
            "/api/project-invites/:code/join",
            post(project_membership::join_project_by_invite_link),
        )
        .route(
            "/api/projects/:id/member-audit",
            get(project_membership::list_member_audit),
        )
        .route(
            "/api/projects/:id/roles",
            get(project_membership::list_project_roles).post(project_membership::create_project_role),
        )
        .route(
            "/api/projects/:id/roles/:role_id",
            axum::routing::patch(project_membership::update_project_role)
                .delete(project_membership::delete_project_role),
        )
        .route(
            "/api/projects/:id/visibility",
            axum::routing::patch(project_membership::update_visibility),
        )
        .route(
            "/api/projects/:id/runtime-permission",
            get(project_runtime_permission_api::get_runtime_permission)
                .patch(project_runtime_permission_api::update_runtime_permission),
        )
        .route(
            "/api/projects/:id/icon",
            axum::routing::patch(project_membership::update_project_icon),
        )
        .route(
            "/api/projects/:id/brand",
            axum::routing::patch(project_membership::update_project_brand),
        )
        .route(
            "/api/projects/:id/members/:user_id",
            axum::routing::patch(project_membership::update_member_role)
                .delete(project_membership::remove_member),
        )
        .route(
            "/api/projects/:id/members/:user_id/profile",
            axum::routing::patch(project_membership::update_member_profile),
        )
        .route(
            "/api/projects/:id/members/:user_id/moderation",
            axum::routing::patch(project_membership::update_member_moderation),
        )
        // ── 加入申请审批 ──────────────────────────────────────────────────
        .route(
            "/api/projects/:id/request-join",
            post(project_join_requests::request_join),
        )
        .route(
            "/api/projects/:id/join-requests",
            get(project_join_requests::list_join_requests),
        )
        .route(
            "/api/projects/:id/join-requests/:req_id",
            axum::routing::patch(project_join_requests::review_join_request),
        )
        .route(
            "/api/me/join-requests",
            get(project_join_requests::my_join_requests),
        )
        .route(
            "/api/me/join-requests/:req_id",
            axum::routing::delete(project_join_requests::cancel_my_join_request),
        )
        .route(
            "/api/me/owned-projects/pending-counts",
            get(project_join_requests::owned_projects_pending_counts),
        )
        // ── 项目空间：频道、成员协作、集体 AI 开发 ───────────────────────────
        .route(
            "/api/projects/:project_id/space",
            get(project_space::get_project_space),
        )
        .route(
            "/api/projects/:project_id/space/description",
            axum::routing::patch(project_space::update_project_description),
        )
        .route(
            "/api/projects/:project_id/space/gallery-image",
            axum::routing::patch(project_space::update_project_gallery_image),
        )
        .route(
            "/api/projects/:project_id/ai/available-nodes",
            get(group_ai::api::available_nodes),
        )
        .route(
            "/api/projects/:project_id/ai/node-authorizations",
            post(group_ai::api::upsert_node_authorization),
        )
        .route(
            "/api/projects/:project_id/ai/bots",
            get(group_ai::api::list_bots),
        )
        .route(
            "/api/projects/:project_id/ai/matters/plan",
            post(group_ai::api::create_matter_plan),
        )
        .route(
            "/api/projects/:project_id/ai/matters",
            get(group_ai::api::list_matters),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/approve",
            post(group_ai::api::approve_matter),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/start",
            post(group_ai::api::start_matter),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/run-all",
            post(group_ai::automation_api::run_matter_assignments),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/review",
            post(group_ai::automation_api::run_review_assignment),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/events",
            get(group_ai::automation_api::list_matter_events),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/governance",
            get(group_ai::governance_api::get_matter_governance),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/budget-policy",
            post(group_ai::governance_api::update_matter_budget_policy_handler),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/reviews",
            post(group_ai::governance_api::record_matter_review),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/merge-requests",
            post(group_ai::governance_api::create_merge_request),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/merge-requests/:merge_request_id",
            post(group_ai::governance_api::update_merge_request),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/merge-requests/:merge_request_id/check",
            post(group_ai::governance_api::check_merge_request),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/merge-requests/:merge_request_id/apply",
            post(group_ai::governance_api::apply_merge_request),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/request-changes",
            post(group_ai::api::request_matter_changes),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/accept",
            post(group_ai::api::accept_matter),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/cancel",
            post(group_ai::api::cancel_matter),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/complete",
            post(group_ai::api::complete_assignment),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/fail",
            post(group_ai::api::fail_assignment),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/retry",
            post(group_ai::api::retry_assignment),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/run",
            post(group_ai::api::run_assignment),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/artifact",
            get(group_ai::automation_api::get_assignment_artifact),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/artifacts",
            post(group_ai::governance_api::record_assignment_artifact),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id/assignments/:assignment_id/settlement",
            post(group_ai::api::record_assignment_settlement_handler),
        )
        .route(
            "/api/projects/:project_id/ai/matters/:matter_id",
            get(group_ai::api::get_matter),
        )
        .route(
            "/api/projects/:project_id/landing/sync",
            post(project_landing_api::sync_project_landing),
        )
        .route(
            "/api/projects/:project_id/landing/token",
            post(project_landing_api::rotate_project_landing_token),
        )
        .route(
            "/api/projects/:project_id/docs",
            get(project_docs::get_project_document),
        )
        .route(
            "/api/projects/:project_id/members/:member_user_id/conversations",
            get(project_space::list_member_conversations),
        )
        .route(
            "/api/projects/:project_id/members/:member_user_id/conversations/:conversation_id/messages",
            get(project_space::list_member_conversation_messages)
                .post(project_space::send_member_conversation_message),
        )
        .route(
            "/api/projects/:project_id/conversations/:conversation_id/visibility",
            axum::routing::patch(project_space::update_member_conversation_visibility),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/messages",
            get(project_space::list_channel_messages).post(project_space::send_channel_message),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/permissions",
            get(project_space::get_channel_permissions)
                .patch(project_space::update_channel_permissions),
        )
        .route(
            "/api/projects/:project_id/channel-categories/:category_id/permissions",
            get(project_space::get_channel_category_permissions)
                .patch(project_space::update_channel_category_permissions),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/messages/:message_id/suggestion",
            axum::routing::patch(project_space::mark_suggestion_updated),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks",
            post(project_space::start_channel_ai_task),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/cancel",
            post(project_space::cancel_channel_ai_task),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/snapshot",
            get(project_space_task_snapshot::snapshot_channel_ai_task),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/events",
            get(project_space_task_snapshot::list_channel_ai_task_events),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/tool-approvals/:approval_id/decision",
            post(project_space::decide_channel_ai_tool_approval),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/summaries",
            post(project_space::summarize_channel_selection),
        )
        .route(
            "/api/projects/:project_id/chat",
            post(project_chat::chat_project),
        )
        .route(
            "/api/projects/:project_id/prewarm",
            post(project_chat::prewarm_project),
        )
        .route(
            "/api/projects/:project_id/workspace/health",
            get(project_workspace_health::get_project_workspace_health),
        )
        .route(
            "/api/projects/:project_id/workspace/recover",
            post(project_workspace_recovery::recover_project_workspace),
        )
        .route(
            "/api/projects/:project_id/conversations/:conversation_id/identity",
            get(project_conversation_identity::conversation_identity_project),
        )
        .route(
            "/api/projects/:project_id/attachments",
            post(project_attachments::upload_project_attachment).layer(DefaultBodyLimit::max(
                project_attachments::MAX_PROJECT_ATTACHMENT_BYTES,
            )),
        )
        .route(
            "/api/projects/:project_id/attachments/:conversation_id/:filename",
            get(project_attachments::download_project_attachment),
        )
        .route(
            "/ws/projects/:project_id",
            get(project_api::ws_project_handler),
        )
        .route(
            "/ws/user/:user_id/projects/:project_id",
            get(project_api::ws_user_project_handler),
        )
        // 轻量通知 WS（保留，兼容旧版 APK）
        .route("/ws/notify", get(app_update::ws_notify_handler))
        // 全局 WS 通道：统一实时推送（更新 + 未来好友消息等）
        .route("/ws/app", get(global_ws::global_ws_handler))
        // ── 实时语音通道（两条路并行测试）─────────────────────────────────
        // 方案 A：Android PCM → 服务器 PipeWire 虚拟麦克风（投喂 Codex CLI 本地音频采集）
        .route(
            "/ws/voice/virtual-mic",
            get(voice_ws_virtual_mic::ws_virtual_mic_handler),
        )
        // 方案 B：Android PCM → OpenAI Realtime Transcription → 转写文本 → CLI
        .route(
            "/ws/voice/transcribe",
            get(voice_ws_transcribe::ws_transcribe_handler),
        )
        // 方案 C：Android PCM ↔ OpenAI Realtime speech-to-speech，全双工一龙AI通话
        .route(
            "/ws/voice/realtime-chat",
            get(voice_ws_realtime_chat::ws_realtime_chat_handler),
        )
        .route(
            "/api/user/:user_id/projects/:project_id",
            delete(project_deletion::delete_user_project),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/space",
            get(project_space::get_user_project_space),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/space/description",
            axum::routing::patch(project_space::update_user_project_description),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/space/gallery-image",
            axum::routing::patch(project_space::update_user_project_gallery_image),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/docs",
            get(project_docs::get_user_project_document),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/messages",
            get(project_space::list_user_project_channel_messages)
                .post(project_space::send_user_project_channel_message),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/messages/:message_id/suggestion",
            axum::routing::patch(project_space::mark_user_project_suggestion_updated),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/ai-tasks",
            post(project_space::start_user_project_channel_ai_task),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/cancel",
            post(project_space::cancel_user_project_channel_ai_task),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/snapshot",
            get(project_space_task_snapshot::snapshot_user_channel_ai_task),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/events",
            get(project_space_task_snapshot::list_user_channel_ai_task_events),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/ai-tasks/:task_id/tool-approvals/:approval_id/decision",
            post(project_space::decide_user_project_channel_ai_tool_approval),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/channels/:channel_id/summaries",
            post(project_space::summarize_user_project_channel_selection),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/status",
            get(project_git::user_project_git_status),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/prewarm",
            post(project_chat::prewarm_user_project),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/conversations/:conversation_id/identity",
            get(project_conversation_identity::conversation_identity_user_project),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/attachments",
            post(project_attachments::upload_user_project_attachment).layer(DefaultBodyLimit::max(
                project_attachments::MAX_PROJECT_ATTACHMENT_BYTES,
            )),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/attachments/:conversation_id/:filename",
            get(project_attachments::download_user_project_attachment),
        )
        .route(
            "/api/user/:user_id/chat-attachments",
            post(chat_attachments::upload_user_chat_attachment).layer(DefaultBodyLimit::max(
                chat_attachments::MAX_CHAT_ATTACHMENT_BYTES,
            )),
        )
        .route(
            "/api/user/:user_id/chat-attachments/:conversation_id/:filename",
            get(chat_attachments::download_user_chat_attachment),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/deploy-key",
            post(project_git::user_project_deploy_key),
        )
        .route(
            "/api/user/:user_id/projects/:project_id/git/config",
            post(project_git::user_project_git_config),
        )
        .merge(project_channels::routes().merge(project_releases::routes()).merge(crate::project_git_worktree_audit_api::routes()))
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
        // ── 局域网 PC 种子节点（开发 PC 发布 APK 后直接在 WiFi 内提供下载）
        .route("/app/lan-peer/register", post(lan_peer::register_lan_peer))
        // ── homecli PC agent 反向 WSS 通道 ────────────────────────────
        .route("/agent/ws", get(crate::homecli_agent::agent_ws_handler))
        .route(
            "/api/_test_dispatch",
            post(crate::homecli_agent::test_dispatch),
        )
        .route(
            "/api/_test_cli_prompt",
            post(crate::homecli_agent::test_cli_prompt),
        )
        // ── PC 本地 server 反向代理中继（Symmetric NAT 穿透）────────
        // APK 通过 /api/pc-relay/{agent_id}/... 访问 PC 本机 HTTP 服务
        .route(
            "/api/pc-relay/:agent_id/*path",
            axum::routing::any(crate::pc_relay::pc_relay_handler),
        )
        .route(
            "/api/storage-git/:node_id/:token/*path",
            axum::routing::any(project_storage_git::storage_git_handler),
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
        .route("/api/admin/projects", get(admin::list_projects))
        .route(
            "/api/admin/projects/:id/conversations",
            get(admin::list_project_conversations),
        )
        .route("/api/admin/sessions", get(admin::list_sessions))
        // ── Token 用量统计 ────────────────────────────────────────────────────
        .route(
            "/api/admin/token-stats/summary",
            get(admin_token_stats::get_platform_summary),
        )
        .route(
            "/api/admin/token-stats/users",
            get(admin_token_stats::get_users_usage),
        )
        .route(
            "/api/admin/token-stats/users/:user_id",
            get(admin_token_stats::get_user_detail),
        )
        .route(
            "/api/admin/token-stats/trend",
            get(admin_token_stats::get_platform_trend),
        )
        .route(
            "/api/admin/token-stats/accounting-audit",
            get(admin_token_stats::get_accounting_audit),
        )
        .route(
            "/api/admin/token-stats/reconciliation-summary",
            get(admin_token_stats::get_reconciliation_summary),
        )
        .route(
            "/api/admin/token-stats/billing-alerts",
            get(admin_token_stats::get_billing_alerts),
        )
        .route(
            "/api/admin/token-stats/compute-meter-summary",
            get(admin_token_stats::get_compute_meter_summary),
        )
        .route(
            "/api/admin/token-stats/compute-meter-events",
            get(admin_token_stats::get_compute_meter_events),
        )
        .route(
            "/api/admin/external-apps/tool-executions",
            get(external_app_tool_report_api::get_tool_execution_report),
        )
        // ── 用户配额管理 ──────────────────────────────────────────────────────
        .route(
            "/api/admin/quotas",
            get(admin_quota::list_quotas),
        )
        .route(
            "/api/admin/quotas/:user_id",
            axum::routing::put(admin_quota::upsert_quota)
                .delete(admin_quota::delete_quota),
        )
        .route(
            "/api/admin/context/symbol-index/search",
            get(context_compiler::symbol_index_api::search_symbol_index),
        )
        .route(
            "/api/admin/context/symbol-index/chunks",
            get(context_compiler::symbol_index_api::search_symbol_chunks),
        )
        .route(
            "/api/admin/context/symbol-index/embedding-status",
            get(context_compiler::symbol_index_api::get_symbol_embedding_status),
        )
        .route(
            "/api/admin/context/symbol-index/vector-backfill",
            post(context_compiler::symbol_index_api::backfill_symbol_vectors),
        )
        .route(
            "/api/admin/context/symbol-index/vector-search",
            get(context_compiler::symbol_index_api::search_symbol_vectors),
        )
        .route(
            "/api/admin/context/symbol-index/eval",
            get(context_compiler::symbol_index_api::eval_symbol_retrieval),
        )
        .route(
            "/api/admin/context/symbol-index/eval-batch",
            post(context_compiler::symbol_index_api::eval_symbol_retrieval_batch),
        )
        .route(
            "/api/admin/context/symbol-index/eval-runs",
            get(context_compiler::symbol_index_api::list_symbol_retrieval_runs),
        )
        .route(
            "/api/admin/context/symbol-index/eval-run",
            get(context_compiler::symbol_index_api::get_symbol_retrieval_run),
        )
        .route(
            "/api/admin/context/symbol-index/eval-compare",
            get(context_compiler::symbol_index_eval_compare_api::compare_symbol_retrieval_runs),
        )
        .route(
            "/api/admin/context/symbol-index/retrieval-learning",
            get(context_compiler::symbol_index_api::get_symbol_retrieval_learning),
        )
        .route(
            "/api/admin/context/symbol-index/symbol",
            get(context_compiler::symbol_index_api::get_symbol_graph),
        )
        .route(
            "/api/admin/context/symbol-index/impact",
            get(context_compiler::symbol_index_api::get_symbol_impact),
        )
        .route(
            "/api/admin/context/symbol-index/impact-pack",
            get(context_compiler::symbol_index_api::get_symbol_impact_pack),
        )
        .route(
            "/api/admin/context/symbol-index/task-pack",
            get(context_compiler::symbol_index_api::get_symbol_task_pack),
        )
        .route(
            "/api/admin/context/symbol-index/patch-check",
            post(context_compiler::symbol_index_patch_api::check_symbol_patch),
        )
        .route(
            "/api/admin/context/symbol-index/patch-dry-run",
            post(context_compiler::symbol_index_patch_api::dry_run_symbol_patch),
        )
        .route(
            "/api/admin/context/symbol-index/patch-verify",
            post(context_compiler::symbol_index_patch_api::verify_symbol_patch),
        )
        .route(
            "/api/admin/context/symbol-index/patch-verify-run",
            post(context_compiler::symbol_index_patch_api::run_symbol_patch_verification),
        )
        .route(
            "/api/admin/context/symbol-index/patch-repair-attempt",
            post(context_compiler::symbol_index_patch_api::run_symbol_patch_repair_attempt),
        )
        .route(
            "/api/admin/context/symbol-index/patch-repair-generate",
            post(context_compiler::symbol_index_patch_api::generate_symbol_patch_repair),
        )
        .route(
            "/api/admin/context/symbol-index/patch-review",
            post(context_compiler::symbol_index_patch_api::review_symbol_patch),
        )
        .route(
            "/api/admin/context/symbol-index/patch-apply",
            post(context_compiler::symbol_index_patch_api::apply_symbol_patch),
        )
        .route(
            "/api/admin/context/symbol-index/patch-rollback",
            post(context_compiler::symbol_index_patch_api::rollback_symbol_patch_handler),
        )
        .route(
            "/api/user/:user_id/agent",
            get(user_api::get_user_agent).put(user_api::set_user_agent),
        )
        .route(
            "/api/user/:user_id/agent/test",
            post(user_api::test_user_agent),
        )
        .route(
            "/api/user/:user_id/usage/stats",
            get(token_usage_api::get_usage_stats),
        )
        .route(
            "/api/user/:user_id/usage/report",
            post(token_usage_api::report_client_usage),
        )
        .route(
            "/api/user/:user_id/speech/translate",
            post(speech_translate::translate_user_speech),
        )
        .route(
            "/api/voice/asr",
            post(voice_asr_upload::asr_upload_handler),
        )
        .route(
            "/api/voice/tts/catalog",
            get(voice_tts_api::catalog_handler),
        )
        .route("/api/voice/tts", post(voice_tts_api::synthesize_handler))
        // ── 用户头像（公开查看 / 登录后上传）─────────────────────────────────
        .route("/api/users/:user_id/avatar", get(user_api::get_user_avatar))
        .route(
            "/api/me/avatar",
            axum::routing::put(user_api::put_my_avatar)
                .layer(DefaultBodyLimit::max(800_000)),
        )
        // ── 用户计费（余额查询 / 扣费明细）──────────────────────────────────
        .route("/api/me/balance", get(billing_api::get_my_balance))
        .route("/api/me/billing", get(billing_api::list_my_billing))
        // ── 微信支付（创建订单 / 异步回调 / 订单查询）────────────────────────
        .route("/api/me/pay/create_order", post(billing_pay::create_order))
        .route("/api/me/pay/orders", get(billing_pay::list_my_orders))
        .route("/api/pay/notify", post(billing_pay::pay_notify))
        // ── 管理员计费（充值 / 余额列表 / 配置）──────────────────────────────
        .route("/api/admin/billing/recharge", post(billing_admin::recharge_user))
        .route("/api/admin/billing/users", get(billing_admin::list_users))
        .route("/api/admin/billing/users/:user_id", get(billing_admin::get_user))
        .route("/api/admin/billing/events", get(billing_admin::list_events))
        .route(
            "/api/admin/billing/reservations",
            get(billing_admin::list_reservations),
        )
        .route(
            "/api/admin/billing/price-rules",
            get(billing_admin::list_price_rules).put(billing_admin::upsert_price_rule),
        )
        .route(
            "/api/admin/billing/config",
            get(billing_admin::get_config).put(billing_admin::set_config),
        )
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state)
}
