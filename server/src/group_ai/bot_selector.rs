use anyhow::Result;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    group_ai::types::{AvailableGroupAiNode, ProjectAiBot, ProjectAiNodeAuthorization},
    node_runtime::{node_runtime_by_id, short_node_id, user_node_runtimes, NodeRuntime},
    types::AppState,
};

pub(crate) async fn available_nodes_for_project(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> Result<Vec<AvailableGroupAiNode>> {
    let authorizations = state
        .store
        .list_project_ai_node_authorizations(project_id)?;
    let auth_by_node: HashMap<String, ProjectAiNodeAuthorization> = authorizations
        .iter()
        .cloned()
        .map(|authorization| (authorization.node_id.clone(), authorization))
        .collect();
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();

    for runtime in user_node_runtimes(state, user_id).await? {
        let authorization = auth_by_node.get(&runtime.node_id).cloned();
        seen.insert(runtime.node_id.clone());
        nodes.push(available_node_from_runtime(runtime, authorization));
    }

    for authorization in authorizations {
        if seen.contains(&authorization.node_id) {
            continue;
        }
        if let Some(runtime) = node_runtime_by_id(state, &authorization.node_id).await? {
            nodes.push(available_node_from_runtime(runtime, Some(authorization)));
        } else {
            nodes.push(offline_authorized_node(authorization));
        }
    }

    Ok(nodes)
}

pub(crate) async fn bots_for_project(
    state: &AppState,
    project_id: &str,
) -> Result<Vec<ProjectAiBot>> {
    let authorizations = state
        .store
        .list_project_ai_node_authorizations(project_id)?;
    let mut bots = Vec::new();
    for authorization in authorizations
        .into_iter()
        .filter(|authorization| authorization.enabled)
    {
        let runtime = node_runtime_by_id(state, &authorization.node_id).await?;
        for cli_name in candidate_clis(&authorization, runtime.as_ref()) {
            bots.push(ProjectAiBot {
                bot_id: bot_id(&authorization.node_id, &cli_name),
                project_id: project_id.to_string(),
                provider_user_id: authorization.provider_user_id.clone(),
                node_id: authorization.node_id.clone(),
                display_name: bot_display_name(&authorization, runtime.as_ref(), &cli_name),
                runtime_route: runtime_route_for_cli(&cli_name).to_string(),
                cli_name: cli_name.clone(),
                capabilities: capabilities_for_cli(&cli_name),
                risk_level: authorization.permission_level.clone(),
                online: runtime.as_ref().map(|node| node.online).unwrap_or(false),
                cli_connected: runtime
                    .as_ref()
                    .map(|node| node.cli_connected)
                    .unwrap_or(false),
            });
        }
    }
    bots.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then(left.display_name.cmp(&right.display_name))
            .then(left.cli_name.cmp(&right.cli_name))
    });
    Ok(bots)
}

fn available_node_from_runtime(
    runtime: NodeRuntime,
    authorization: Option<ProjectAiNodeAuthorization>,
) -> AvailableGroupAiNode {
    let allowed_clis = authorization
        .as_ref()
        .filter(|authorization| !authorization.allowed_clis.is_empty())
        .map(|authorization| authorization.allowed_clis.clone())
        .unwrap_or_else(|| runtime.allowed_clis.clone());
    AvailableGroupAiNode {
        node_id: runtime.node_id.clone(),
        provider_user_id: runtime.owner_user_id.clone(),
        display_name: runtime.display_name,
        short_id: runtime.short_id,
        online: runtime.online,
        cli_connected: runtime.cli_connected,
        allowed_clis,
        authorized: authorization.is_some(),
        authorization,
    }
}

fn offline_authorized_node(authorization: ProjectAiNodeAuthorization) -> AvailableGroupAiNode {
    AvailableGroupAiNode {
        node_id: authorization.node_id.clone(),
        provider_user_id: authorization.provider_user_id.clone(),
        display_name: short_node_id(&authorization.node_id),
        short_id: short_node_id(&authorization.node_id),
        online: false,
        cli_connected: false,
        allowed_clis: authorization.allowed_clis.clone(),
        authorized: true,
        authorization: Some(authorization),
    }
}

fn candidate_clis(
    authorization: &ProjectAiNodeAuthorization,
    runtime: Option<&NodeRuntime>,
) -> Vec<String> {
    let auth_allowlist = normalize_clis(&authorization.allowed_clis);
    let restrict_to_auth = !auth_allowlist.is_empty();
    let mut set = BTreeSet::new();

    if let Some(runtime) = runtime {
        for cli in normalize_clis(&runtime.allowed_clis) {
            if !restrict_to_auth || auth_allowlist.contains(&cli) {
                set.insert(cli);
            }
        }
        if runtime
            .dev_runtime
            .as_ref()
            .map(|profile| profile.api_runtime_ready)
            .unwrap_or(false)
            && (!restrict_to_auth || auth_allowlist.contains("api-runtime"))
        {
            set.insert("api-runtime".to_string());
        }
        if runtime
            .dev_runtime
            .as_ref()
            .map(|profile| profile.server_runtime_ready)
            .unwrap_or(false)
            && (!restrict_to_auth || auth_allowlist.contains("server-runtime"))
        {
            set.insert("server-runtime".to_string());
        }
    }

    if set.is_empty() && restrict_to_auth {
        set.extend(auth_allowlist);
    }
    set.into_iter().collect()
}

fn normalize_clis(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn bot_id(node_id: &str, cli_name: &str) -> String {
    format!("pai-bot:{node_id}:{cli_name}")
}

fn bot_display_name(
    authorization: &ProjectAiNodeAuthorization,
    runtime: Option<&NodeRuntime>,
    cli_name: &str,
) -> String {
    let node_name = runtime
        .map(|node| node.display_name.clone())
        .unwrap_or_else(|| short_node_id(&authorization.node_id));
    format!("{node_name} / {cli_name}")
}

fn runtime_route_for_cli(cli_name: &str) -> &'static str {
    match cli_name {
        "api-runtime" => "route_b_api_runtime",
        "server-runtime" => "route_c_server_runtime",
        _ => "route_a_cli",
    }
}

fn capabilities_for_cli(cli_name: &str) -> Vec<String> {
    let mut capabilities = vec![
        "implement".to_string(),
        "review".to_string(),
        "test".to_string(),
        "docs".to_string(),
    ];
    match cli_name {
        "api-runtime" => capabilities.push("api_model".to_string()),
        "server-runtime" => capabilities.push("server_model".to_string()),
        _ => capabilities.push("local_cli".to_string()),
    }
    capabilities
}
