use std::{sync::Arc, time::Duration};

use crate::{project_keys::route_a_session_lease_key, store::ProjectAccess, types::AppState};

pub(crate) const ROUTE_A_SESSION_LEASE_TTL_SECS: u64 = 60 * 60;

pub(crate) struct RouteARuntimePrewarmResult {
    pub agent_id: String,
    pub workspace: String,
    pub ttl_secs: u64,
    pub reused: bool,
}

pub(crate) async fn is_hot(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    agent_id: &str,
    workspace: &str,
) -> bool {
    let Some(connected_at) = pc_agent_connected_at(state, agent_id).await else {
        return false;
    };
    let lease_key = lease_key(project, user_id, conversation_id, agent_id, workspace);
    if state
        .route_a_session_leases
        .get_valid(&lease_key, connected_at)
        .await
        .is_some()
    {
        return true;
    }

    let fallback_key =
        route_a_session_lease_key(&project.id, user_id, "default", agent_id, workspace);
    fallback_key != lease_key
        && state
            .route_a_session_leases
            .get_valid(&fallback_key, connected_at)
            .await
            .is_some()
}

pub(crate) async fn record_verified(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    agent_id: &str,
    workspace: &str,
) -> bool {
    let Some(connected_at) = pc_agent_connected_at(state, agent_id).await else {
        return false;
    };
    let conversation_id = normalized_conversation_id(conversation_id);
    let lease_key =
        route_a_session_lease_key(&project.id, user_id, conversation_id, agent_id, workspace);
    state
        .route_a_session_leases
        .record_verified(
            lease_key,
            &project.id,
            user_id,
            conversation_id,
            agent_id,
            workspace,
            connected_at,
            Duration::from_secs(ROUTE_A_SESSION_LEASE_TTL_SECS),
        )
        .await;
    true
}

pub(crate) async fn prewarm_result(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: &str,
    agent_id: String,
    workspace: String,
) -> Option<RouteARuntimePrewarmResult> {
    let key =
        route_a_session_lease_key(&project.id, user_id, conversation_id, &agent_id, &workspace);
    let connected_at = pc_agent_connected_at(state, &agent_id).await?;
    let reused = state
        .route_a_session_leases
        .get_valid(&key, connected_at)
        .await
        .map(|snapshot| snapshot.age_ms > 0)
        .unwrap_or(false);
    Some(RouteARuntimePrewarmResult {
        agent_id,
        workspace,
        ttl_secs: ROUTE_A_SESSION_LEASE_TTL_SECS,
        reused,
    })
}

async fn pc_agent_connected_at(state: &Arc<AppState>, agent_id: &str) -> Option<u64> {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|agent| agent.agent_id == agent_id)
        .map(|agent| agent.connected_at)
}

fn lease_key(
    project: &ProjectAccess,
    user_id: &str,
    conversation_id: Option<&str>,
    agent_id: &str,
    workspace: &str,
) -> String {
    route_a_session_lease_key(
        &project.id,
        user_id,
        normalized_conversation_id(conversation_id),
        agent_id,
        workspace,
    )
}

fn normalized_conversation_id(conversation_id: Option<&str>) -> &str {
    conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
}
