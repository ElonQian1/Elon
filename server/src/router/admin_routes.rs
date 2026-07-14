use crate::types::AppState;
use crate::{
    admin, admin_quota, admin_token_stats, billing_admin, billing_api, billing_pay,
    context_compiler, external_app_tool_report_api, speech_translate, token_usage_api, user_api,
    voice_asr_upload, voice_tts_api,
};
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
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
        .route("/api/admin/quotas", get(admin_quota::list_quotas))
        .route(
            "/api/admin/quotas/:user_id",
            axum::routing::put(admin_quota::upsert_quota).delete(admin_quota::delete_quota),
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
        .route("/api/voice/asr", post(voice_asr_upload::asr_upload_handler))
        .route(
            "/api/voice/tts/catalog",
            get(voice_tts_api::catalog_handler),
        )
        .route("/api/voice/tts", post(voice_tts_api::synthesize_handler))
        // ── 用户头像（公开查看 / 登录后上传）─────────────────────────────────
        .route("/api/users/:user_id/avatar", get(user_api::get_user_avatar))
        .route(
            "/api/me/avatar",
            axum::routing::put(user_api::put_my_avatar).layer(DefaultBodyLimit::max(800_000)),
        )
        // ── 用户计费（余额查询 / 扣费明细）──────────────────────────────────
        .route("/api/me/balance", get(billing_api::get_my_balance))
        .route("/api/me/billing", get(billing_api::list_my_billing))
        // ── 微信支付（创建订单 / 异步回调 / 订单查询）────────────────────────
        .route("/api/me/pay/create_order", post(billing_pay::create_order))
        .route("/api/me/pay/orders", get(billing_pay::list_my_orders))
        .route("/api/pay/notify", post(billing_pay::pay_notify))
        // ── 管理员计费（充值 / 余额列表 / 配置）──────────────────────────────
        .route(
            "/api/admin/billing/recharge",
            post(billing_admin::recharge_user),
        )
        .route("/api/admin/billing/users", get(billing_admin::list_users))
        .route(
            "/api/admin/billing/users/:user_id",
            get(billing_admin::get_user),
        )
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
}
