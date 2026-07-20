use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{
    node_agent_downloads::rg_win, node_api, node_compute_admin, node_exec_api, node_payout_admin,
    route_c_admin, types::AppState,
};

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/nodes", get(node_api::list_nodes))
        .route("/api/nodes/models", get(node_api::list_available_models))
        .route("/api/nodes/chat", post(node_api::chat_with_node))
        .route("/api/node-agent/version", get(node_api::node_agent_version))
        .route(
            "/api/node-agent/download/windows",
            get(node_api::download_node_agent_windows),
        )
        .route(
            "/api/node-agent/download/windows-client",
            get(node_api::download_node_agent_windows_client),
        )
        .route("/api/node-agent/download/ripgrep-windows", get(rg_win))
        .route(
            "/api/node-agent/download/linux",
            get(node_api::download_node_agent_linux),
        )
        .route("/api/me/nodes", get(node_api::my_nodes))
        .route("/api/me/nodes/register", post(node_api::register_node))
        .route(
            "/api/me/nodes/:node_id/sharing",
            axum::routing::patch(node_api::update_my_node_sharing),
        )
        .route("/api/me/node/exec", post(node_exec_api::node_exec_handler))
        .route(
            "/api/admin/nodes/public-dev-handshake",
            get(node_api::admin_public_dev_handshake),
        )
        .route(
            "/api/admin/nodes/public-dev-mutual-smoke",
            get(node_api::admin_public_dev_mutual_smoke_get)
                .post(node_api::admin_public_dev_mutual_smoke_post),
        )
        .route(
            "/api/admin/nodes/owner-codex-smoke",
            post(node_api::admin_owner_codex_smoke_post),
        )
        .route(
            "/api/admin/nodes/push-update",
            post(node_api::push_node_update),
        )
        .route("/api/me/node-balance", get(node_api::my_node_balance))
        .route(
            "/api/me/node-transactions",
            get(node_api::my_node_transactions),
        )
        .route("/api/me/node-usage", get(node_api::my_node_usage))
        .route(
            "/api/me/node-payouts",
            get(node_api::my_node_payouts).post(node_api::create_node_payout),
        )
        .route(
            "/api/me/node-payouts/:payout_id/cancel",
            post(node_api::cancel_node_payout),
        )
        .route(
            "/api/admin/node-payouts",
            get(node_payout_admin::list_payouts),
        )
        .route(
            "/api/admin/node-compute-runs",
            get(node_compute_admin::list_runs),
        )
        .route(
            "/api/admin/route-c/budget",
            get(route_c_admin::budget_report),
        )
        .route(
            "/api/admin/node-payouts/:payout_id/paid",
            post(node_payout_admin::mark_paid),
        )
        .route(
            "/api/admin/node-payouts/:payout_id/reject",
            post(node_payout_admin::reject),
        )
}
