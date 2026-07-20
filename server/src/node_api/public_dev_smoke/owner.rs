use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{admin, types::AppState};

use super::{
    support::{
        clean, cli_list_contains, default_smoke_prompt, resolve_side,
        run_smoke_direction_with_preflight, SmokeDirection, SmokePreflight, SmokeSide,
    },
    DEFAULT_CLI, DEFAULT_LEFT_NODE, DEFAULT_LEFT_OWNER,
};

#[derive(Debug, Deserialize)]
pub struct OwnerCodexSmokeRequest {
    pub execute: Option<bool>,
    pub owner: Option<String>,
    pub node: Option<String>,
    pub cli_name: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
struct OwnerCodexSmokeResponse {
    ok: bool,
    execute: bool,
    cli_name: String,
    generated_at: String,
    side: SmokeSide,
    direction: SmokeDirection,
}

pub async fn admin_owner_codex_smoke_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<OwnerCodexSmokeRequest>,
) -> impl IntoResponse {
    if !admin::check_auth(&headers, &state.admin_token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"admin token required"})),
        )
            .into_response();
    }

    let execute = req.execute.unwrap_or(false);
    let cli_name = clean(req.cli_name.as_deref()).unwrap_or_else(|| DEFAULT_CLI.to_string());
    let prompt = req
        .prompt
        .unwrap_or_else(|| default_smoke_prompt(&cli_name));
    let side = match resolve_side(
        &state,
        req.owner.as_deref().unwrap_or(DEFAULT_LEFT_OWNER),
        req.node.as_deref().unwrap_or(DEFAULT_LEFT_NODE),
    )
    .await
    {
        Ok(side) => side,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": error.to_string()})),
            )
                .into_response()
        }
    };
    let owner_label = side
        .owner
        .nickname
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&side.owner.account);
    let label = format!("{owner_label}使用自身{}", side.node.display_name);
    let preflight = build_owner_preflight(&side, &cli_name);
    let direction = run_smoke_direction_with_preflight(
        &state,
        &label,
        &side.owner,
        &side,
        &cli_name,
        &prompt,
        execute,
        preflight,
    )
    .await;
    let expected_status = if execute { "passed" } else { "ready" };
    Json(OwnerCodexSmokeResponse {
        ok: direction.status == expected_status,
        execute,
        cli_name,
        generated_at: chrono::Utc::now().to_rfc3339(),
        side,
        direction,
    })
    .into_response()
}

fn build_owner_preflight(owner_side: &SmokeSide, cli_name: &str) -> SmokePreflight {
    let node = &owner_side.node;
    let cli_reported_by_node = cli_list_contains(&node.allowed_clis, cli_name);
    let ready = node.online && node.cli_connected && cli_reported_by_node;
    let mut notes = Vec::new();
    if !node.online || !node.cli_connected {
        notes.push("节点不在线或 CLI 通道未连接".to_string());
    }
    if !cli_reported_by_node {
        notes.push(format!("节点未上报 {cli_name}"));
    }
    SmokePreflight {
        authorized: true,
        ready,
        cli_allowed_by_share: true,
        cli_reported_by_node,
        route: "RouteOwn/owner-node-codex".to_string(),
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::build_owner_preflight;
    use crate::node_api::public_dev_smoke::support::{SmokeNode, SmokeSide, SmokeUser};

    fn owner_side(online: bool, cli_connected: bool, allowed_clis: Vec<String>) -> SmokeSide {
        SmokeSide {
            owner: SmokeUser {
                id: "owner-user".to_string(),
                account: "owner@example.com".to_string(),
                nickname: Some("owner".to_string()),
            },
            node: SmokeNode {
                node_id: "owner-node".to_string(),
                display_name: "owner node".to_string(),
                device_name: None,
                short_id: "owner".to_string(),
                public_dev_enabled: false,
                public_dev_handshake_ready: false,
                public_dev_handshake_status: "not_required".to_string(),
                online,
                cli_connected,
                allowed_clis,
                last_handshake_allowed_clis: Vec::new(),
                last_handshake_at: None,
                agent_version: Some("test".to_string()),
            },
        }
    }

    #[test]
    fn owner_smoke_requires_online_bound_node_cli_but_not_public_dev_sharing() {
        let ready =
            build_owner_preflight(&owner_side(true, true, vec!["codex".to_string()]), "codex");
        assert!(ready.authorized);
        assert!(ready.ready);
        assert_eq!(ready.route, "RouteOwn/owner-node-codex");

        let offline =
            build_owner_preflight(&owner_side(false, true, vec!["codex".to_string()]), "codex");
        assert!(!offline.ready);
        let missing_cli = build_owner_preflight(&owner_side(true, true, Vec::new()), "codex");
        assert!(!missing_cli.ready);
    }
}
