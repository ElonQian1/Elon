use std::sync::Arc;

use tracing::warn;

use crate::{
    node_runtime::{node_runtime_by_id, NodeRuntime},
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    types::AppState,
};

pub(super) fn pc_agent_authorized_for_bound_node(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
) -> bool {
    pc_agent_belongs_to_user_quiet(state, user_id, agent_id)
        || pc_agent_public_dev_enabled_for_consumer(state, user_id, agent_id)
}

pub(super) fn pc_agent_authorized_for_route(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> bool {
    if pc_agent_belongs_to_user_quiet(state, user_id, agent_id) {
        return true;
    }
    if !route_allows_public_dev_node(pc_runtime_route) {
        return false;
    }
    pc_agent_public_dev_enabled_for_consumer(state, user_id, agent_id)
}

pub(super) fn route_allows_public_dev_node(route: Option<PcRuntimeRoutePreference>) -> bool {
    matches!(
        route,
        None | Some(PcRuntimeRoutePreference::RouteC2 | PcRuntimeRoutePreference::RouteC3)
    )
}

pub(super) fn pc_agent_public_dev_enabled_for_consumer(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
) -> bool {
    match state.store.get_node_public_dev_sharing(agent_id) {
        Ok(Some(sharing)) => {
            sharing.enabled
                && sharing.owner_user_id.trim() != user_id
                && !sharing.allowed_clis.is_empty()
        }
        Ok(None) => false,
        Err(error) => {
            warn!(
                agent_id = %agent_id,
                error = %error,
                "failed to query public dev sharing; refusing shared PC node"
            );
            false
        }
    }
}

pub(super) fn pc_agent_belongs_to_user_quiet(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
) -> bool {
    matches!(
        state.store.get_node_credential_owner(agent_id),
        Ok(Some(owner)) if owner == user_id
    )
}

pub(super) async fn pc_agent_runtime_ready_for_route(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> bool {
    if pc_agent_belongs_to_user_quiet(state, user_id, agent_id) {
        return true;
    }
    match node_runtime_by_id(state, agent_id).await {
        Ok(Some(runtime)) => {
            runtime.cli_connected
                && pc_node_runtime_ready_for_route(
                    state,
                    user_id,
                    agent_id,
                    pc_runtime_route,
                    &runtime,
                )
        }
        Ok(None) => false,
        Err(error) => {
            warn!(
                agent_id = %agent_id,
                error = %error,
                "failed to inspect public dev node runtime readiness"
            );
            false
        }
    }
}

pub(super) fn pc_node_runtime_ready_for_route(
    state: &Arc<AppState>,
    user_id: &str,
    agent_id: &str,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    runtime: &NodeRuntime,
) -> bool {
    if pc_agent_belongs_to_user_quiet(state, user_id, agent_id) {
        return true;
    }
    let sharing = match state.store.get_node_public_dev_sharing(agent_id) {
        Ok(Some(sharing)) if sharing.enabled => sharing,
        _ => return false,
    };
    public_dev_runtime_ready_for_route(pc_runtime_route, &sharing.allowed_clis, runtime)
}

pub(super) fn public_dev_runtime_ready_for_route(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    sharing_allowed_clis: &[String],
    runtime: &NodeRuntime,
) -> bool {
    match pc_runtime_route {
        Some(PcRuntimeRoutePreference::RouteC2) => runtime
            .dev_runtime
            .as_ref()
            .map(|dev| dev.api_runtime_ready)
            .unwrap_or(false),
        Some(PcRuntimeRoutePreference::RouteC3) | None => {
            let route_a_ready = runtime
                .dev_runtime
                .as_ref()
                .map(|dev| dev.route_a_ready)
                .unwrap_or_else(|| !runtime.allowed_clis.is_empty());
            route_a_ready && cli_lists_intersect(sharing_allowed_clis, &runtime.allowed_clis)
        }
        _ => false,
    }
}

pub(super) fn cli_lists_intersect(left: &[String], right: &[String]) -> bool {
    if left.is_empty() {
        return !right.is_empty();
    }
    left.iter()
        .any(|a| right.iter().any(|b| a.eq_ignore_ascii_case(b)))
}
