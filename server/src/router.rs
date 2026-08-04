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
use crate::ui_tuner_api;
use crate::ui_tuner_device_host_api;
use crate::ui_tuner_device_lease_api;
use crate::{
    admin, admin_quota, admin_token_stats, agent_balloon, api, app_update, auth_api, billing_admin,
    billing_api, billing_pay, chat_attachments, codex_vault_api, context_compiler,
    conversation_forks, external_app_api, external_app_chat_bootstrap, external_app_mvp_chat,
    external_app_route_c_sdk, external_app_tool_report_api, friend_api, global_ws, group_ai,
    group_chat_retrieval_api, group_summary_api, lan_peer, lm_chat, open_commerce_api, peer_relay,
    project_api, project_attachments, project_channels, project_chat,
    project_conversation_identity, project_deletion, project_docs,
    project_document_organization_api, project_git, project_join_requests, project_landing_api,
    project_membership, project_releases, project_runtime_permission_api, project_space,
    project_space_task_snapshot, project_storage_git, project_store, project_workspace_health,
    project_workspace_recovery, release_claim, server_agent_runtime, speech_translate,
    token_usage_api, user_api, user_archive_api, user_memory_api, user_progression,
    voice_asr_upload, voice_tts_api, voice_ws_realtime_chat, voice_ws_transcribe,
    voice_ws_virtual_mic, web,
};

mod admin_routes;
mod node_routes;
mod social_routes;

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
    let immutable_assets = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    // /pc: assets 用 ServeDir，其余路径 fallback 到 index.html，支持强刷子路由。
    let pc_assets_svc = tower::ServiceBuilder::new()
        .layer(immutable_assets.clone())
        .service(ServeDir::new(pc_next_dist.join("assets")));
    let pc_router = axum::Router::new()
        .route("/pc-workbench-sw.js", get(web::pc_workbench_service_worker))
        .route_service(
            "/task-progress-preview.html",
            tower::ServiceBuilder::new()
                .layer(no_cache.clone())
                .service(ServeFile::new(
                    pc_next_dist.join("task-progress-preview.html"),
                )),
        )
        .nest_service("/assets", pc_assets_svc)
        .fallback(web::pc_spa_index)
        .with_state(Arc::clone(&state));
    // /pc-next 保持向后兼容（与 /pc 相同）
    let pc_next_assets_svc = tower::ServiceBuilder::new()
        .layer(immutable_assets)
        .service(ServeDir::new(pc_next_dist.join("assets")));
    let pc_next_router = axum::Router::new()
        .route_service(
            "/task-progress-preview.html",
            tower::ServiceBuilder::new()
                .layer(no_cache.clone())
                .service(ServeFile::new(
                    pc_next_dist.join("task-progress-preview.html"),
                )),
        )
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
        .route(
            "/assets/project_view_sheet_background.png",
            get(web::project_view_sheet_background),
        )
        .route(
            "/assets/project_view_drag_handle.png",
            get(web::project_view_drag_handle),
        )
        .route(
            "/assets/project_view_avatar_placeholder.png",
            get(web::project_view_avatar_placeholder),
        )
        .route(
            "/assets/project_view_search_field.png",
            get(web::project_view_search_field),
        )
        .route(
            "/assets/project_view_search_icon.png",
            get(web::project_view_search_icon),
        )
        .route(
            "/assets/project_view_chevron.png",
            get(web::project_view_chevron),
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
        .route(
            "/api/auth/trust-current-device",
            post(auth_api::trust_current_device),
        )
        .route("/api/me", get(auth_api::me))
        .merge(codex_vault_api::routes())
        .merge(ui_tuner_api::routes())
        .merge(ui_tuner_device_host_api::routes())
        .merge(ui_tuner_device_lease_api::routes())
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
        .route(
            "/api/llm/chat/stream",
            post(lm_chat::lm_chat_stream_handler),
        )
        .route("/api/me/ai/conversations", get(lm_chat::list_ai_chat_conversations))
        .route("/api/me/ai/conversations/:conversation_id/messages", get(lm_chat::list_ai_chat_conversation_messages))
        .route("/api/me/ai/conversations/:conversation_id/fork", post(conversation_forks::fork_ai_chat_conversation))
        // ───────────────────────────────────────────────────────────────────────
        .merge(social_routes::routes())
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
            "/api/projects/:project_id/docs/catalog",
            get(project_docs::get_project_document_catalog),
        )
        .route(
            "/api/projects/:project_id/docs/federation",
            get(project_docs::get_project_document_federation),
        )
        .route(
            "/api/projects/:project_id/docs/file",
            get(project_docs::get_project_document_file)
                .put(project_docs::put_project_document_file),
        )
        .route(
            "/api/projects/:project_id/docs/organization/apply",
            post(project_document_organization_api::apply_organization_suggestions),
        )
        .route(
            "/api/projects/:project_id/members/:member_user_id/conversations",
            get(project_space::list_member_conversations),
        )
        .route("/api/projects/:project_id/members/:member_user_id/conversations/:conversation_id/messages", get(project_space::list_member_conversation_messages).post(project_space::send_member_conversation_message))
        .route("/api/projects/:project_id/members/:member_user_id/conversations/:conversation_id/fork", post(conversation_forks::fork_project_member_conversation))
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
        .route("/api/projects/:project_id/channels/:channel_id/messages/:message_id/suggestion", axum::routing::patch(project_space::mark_suggestion_updated))
        .route("/api/projects/:project_id/channels/:channel_id/messages/:message_id", axum::routing::delete(project_space::recall_channel_message))
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
        .route("/api/projects/:project_id/conversations/:conversation_id/identity", get(project_conversation_identity::conversation_identity_project))
        .route("/api/projects/:project_id/conversations/:conversation_id/messages/:message_id", axum::routing::delete(project_chat::recall_project_conversation_message))
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
        .route("/api/user/:user_id/projects/:project_id/channels/:channel_id/messages/:message_id/suggestion", axum::routing::patch(project_space::mark_user_project_suggestion_updated))
        .route("/api/user/:user_id/projects/:project_id/channels/:channel_id/messages/:message_id", axum::routing::delete(project_space::recall_user_project_channel_message))
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
        .merge(
            project_channels::routes()
                .merge(project_releases::routes())
                .merge(crate::project_git_worktree_audit_api::routes()),
        )
        .merge(open_commerce_api::routes())
        .merge(crate::open_commerce_capability_source_api::routes())
        .merge(crate::open_commerce_merchant_identity_api::routes())
        .merge(crate::open_commerce_action_confirmation_api::routes())
        .merge(crate::open_commerce_adapter_api::routes())
        .merge(crate::open_commerce_adapter_claim_api::routes())
        .merge(crate::open_commerce_app_block_api::routes())
        .merge(crate::open_commerce_business_handoff_api::routes())
        .merge(crate::open_commerce_rate_limit_api::routes())
        .merge(crate::open_commerce_client_api::routes())
        .merge(crate::open_commerce_client_lifecycle_api::routes())
        .merge(crate::open_commerce_developer_manifest_api::routes())
        .merge(crate::open_commerce_developer_event_api::routes())
        .merge(crate::open_commerce_webhook_api::routes())
        .merge(crate::open_commerce_webhook_dead_letter_api::routes())
        .merge(crate::open_commerce_merchant_evidence_api::routes())
        .merge(crate::open_commerce_relationship_api::routes())
        .merge(crate::open_commerce_data_erasure_evidence_api::routes())
        .merge(crate::open_commerce_data_request_api::routes())
        .merge(crate::open_commerce_consumer_vault_api::routes())
        .merge(crate::open_commerce_portability_api::routes())
        .merge(crate::open_commerce_portability_adoption_api::routes())
        .merge(crate::open_commerce_portability_merge_api::routes())
        .merge(crate::open_commerce_portability_reauthorization_api::routes())
        .merge(crate::open_commerce_portability_trust_api::routes())
        .merge(crate::open_commerce_consumer_preference_api::routes())
        .merge(crate::open_commerce_consumer_receipt_api::routes())
        .merge(crate::ai_resource_control::api::routes())
        .merge(crate::compute_federation_capacity_bucket_api::routes())
        .merge(crate::compute_federation_capacity_pool_api::routes())
        .merge(crate::compute_federation_capacity_supply_api::routes())
        .merge(crate::compute_federation_activation_api::routes())
        .merge(crate::compute_federation_attempt_api::routes())
        .merge(crate::compute_federation_attempt_finalization_api::routes())
        .merge(crate::compute_federation_attempt_settlement_api::routes())
        .merge(crate::compute_federation_attempt_settlement_challenge_api::routes())
        .merge(crate::compute_federation_attempt_settlement_challenge_resolution_api::routes())
        .merge(crate::compute_federation_attempt_settlement_correction_api::routes())
        .merge(crate::compute_federation_attempt_settlement_release_api::routes())
        .merge(crate::compute_federation_settlement_withdrawal_request_api::routes())
        .merge(crate::compute_federation_settlement_withdrawal_terminal_api::routes())
        .merge(crate::compute_federation_broker_api::routes())
        .merge(crate::compute_federation_offer_api::routes())
        .merge(crate::compute_federation_price_snapshot_api::routes())
        .merge(crate::compute_federation_provider_api::routes())
        .merge(crate::open_commerce_mcp::routes())
        .merge(crate::erp_blueprint_api::routes())
        .merge(crate::task_settlement::api::routes())
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
        .merge(admin_routes::routes())
        .layer(CompressionLayer::new())
        .layer(cors)
        .with_state(state)
}
