use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeFile;

use crate::types::AppState;
use crate::{
    admin, api, app_update, auth_api, chat_attachments, friend_api, global_ws, lan_peer,
    peer_relay, project_api, project_attachments, project_chat, project_conversation_identity,
    project_deletion, project_downloads, project_git, project_membership, project_space,
    project_store, release_claim, speech_translate, token_usage_api, user_api, voice_ws_transcribe,
    voice_ws_virtual_mic, web,
};

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

    Router::new()
        .route("/", get(web::web_page))
        .route("/web", get(web::web_page))
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
        .route("/api/auth/login", post(auth_api::login))
        .route("/api/auth/register", post(auth_api::register))
        .route("/api/me", get(auth_api::me))
        .route(
            "/api/me/profile",
            axum::routing::patch(project_api::update_profile),
        )
        .route(
            "/api/me/friends",
            get(friend_api::list_friends).post(friend_api::add_friend_by_phone),
        )
        .route(
            "/api/me/friends/search",
            get(friend_api::search_friend_by_phone),
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
            "/api/me/groups/:group_id/messages/:message_id",
            delete(friend_api::delete_friend_group_message),
        )
        .route(
            "/api/me/groups/:group_id/messages/:message_id/ai-reply",
            post(friend_api::request_group_ai_reply),
        )
        .route("/api/me/projects", get(project_api::list_my_projects))
        .route("/api/projects", post(project_api::create_project))
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
            get(project_membership::list_members),
        )
        .route(
            "/api/projects/:id/visibility",
            axum::routing::patch(project_membership::update_visibility),
        )
        // ── 项目空间：频道、成员协作、集体 AI 开发 ───────────────────────────
        .route(
            "/api/projects/:project_id/space",
            get(project_space::get_project_space),
        )
        .route(
            "/api/projects/:project_id/members/:member_user_id/conversations",
            get(project_space::list_member_conversations),
        )
        .route(
            "/api/projects/:project_id/members/:member_user_id/conversations/:conversation_id/messages",
            get(project_space::list_member_conversation_messages),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/messages",
            get(project_space::list_channel_messages).post(project_space::send_channel_message),
        )
        .route(
            "/api/projects/:project_id/channels/:channel_id/ai-tasks",
            post(project_space::start_channel_ai_task),
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
        .route(
            "/api/user/:user_id/projects/:project_id",
            delete(project_deletion::delete_user_project),
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
        .route(
            "/api/user/:user_id/projects/:project_id/download/:filename",
            get(project_downloads::download_user_project_apk),
        )
        .route(
            "/api/projects/:project_id/download/:filename",
            get(project_downloads::download_project_apk),
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
        // ── 局域网 PC 种子节点（开发 PC 发布 APK 后直接在 WiFi 内提供下载）
        .route("/app/lan-peer/register", post(lan_peer::register_lan_peer))
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
        .route("/api/admin/projects", get(admin::list_projects))
        .route(
            "/api/admin/projects/:id/conversations",
            get(admin::list_project_conversations),
        )
        .route("/api/admin/sessions", get(admin::list_sessions))
        .route(
            "/api/user/:user_id/agent",
            get(user_api::get_user_agent).put(user_api::set_user_agent),
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
        // ── 用户头像（公开查看 / 登录后上传）─────────────────────────────────
        .route("/api/users/:user_id/avatar", get(user_api::get_user_avatar))
        .route(
            "/api/me/avatar",
            axum::routing::put(user_api::put_my_avatar),
        )
        .layer(cors)
        .with_state(state)
}
