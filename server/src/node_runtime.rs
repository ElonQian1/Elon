use anyhow::Result;
use homecli_proto::{ModelCapability, NodeHardwareProfile};
use std::collections::HashMap;

use crate::{
    homecli_agent::AgentSummary, node_registry::NodeSummary, store::NodeCredential, types::AppState,
};

#[derive(Debug, Clone)]
pub struct NodeRuntime {
    pub node_id: String,
    pub owner_user_id: String,
    pub label: String,
    pub device_name: Option<String>,
    pub hardware: Option<NodeHardwareProfile>,
    pub display_name: String,
    pub short_id: String,
    pub models: Vec<ModelCapability>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub connected_at: u64,
    pub created_at: String,
    pub online: bool,
    pub registry_online: bool,
    pub cli_connected: bool,
    pub project_count: i64,
}

impl NodeRuntime {
    pub fn cli_project_ready(&self) -> bool {
        supports_project_cli(&self.allowed_clis)
    }
}

pub async fn user_node_runtimes(state: &AppState, user_id: &str) -> Result<Vec<NodeRuntime>> {
    let credentials = state.store.list_node_credentials(user_id)?;
    let project_counts = user_project_counts_by_node(state, user_id);
    let mut registry_by_id: HashMap<_, _> = state
        .node_registry
        .list_by_owner(user_id)
        .await
        .into_iter()
        .map(|node| (node.node_id.clone(), node))
        .collect();
    let cli_by_id: HashMap<_, _> = state
        .agent_manager
        .list()
        .await
        .into_iter()
        .map(|agent| (agent.agent_id.clone(), agent))
        .collect();

    let mut nodes = Vec::new();
    for credential in credentials {
        let node_id = credential.agent_id.clone();
        let registry = registry_by_id.remove(&node_id);
        let cli = cli_by_id.get(&node_id);
        nodes.push(build_runtime(
            &node_id,
            Some(&credential),
            registry.as_ref(),
            cli,
            project_counts.get(&node_id).copied().unwrap_or(0),
        ));
    }

    for registry in registry_by_id.into_values() {
        let node_id = registry.node_id.clone();
        nodes.push(build_runtime(
            &node_id,
            None,
            Some(&registry),
            cli_by_id.get(&node_id),
            project_counts.get(&node_id).copied().unwrap_or(0),
        ));
    }

    nodes.sort_by(|left, right| {
        right
            .online
            .cmp(&left.online)
            .then(right.project_count.cmp(&left.project_count))
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    Ok(nodes)
}

pub async fn node_runtime_by_id(state: &AppState, node_id: &str) -> Result<Option<NodeRuntime>> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Ok(None);
    }
    let registry = state
        .node_registry
        .list_online()
        .await
        .into_iter()
        .find(|node| node.node_id == node_id);
    let cli_agents = state.agent_manager.list().await;
    let cli = cli_agents.iter().find(|agent| agent.agent_id == node_id);
    if registry.is_none() && cli.is_none() {
        return Ok(None);
    }

    let owner_user_id = registry
        .as_ref()
        .map(|node| node.owner_user_id.clone())
        .or_else(|| {
            state
                .store
                .get_node_credential_owner(node_id)
                .ok()
                .flatten()
        })
        .unwrap_or_default();
    let project_count = state
        .store
        .count_active_pc_projects_for_node(node_id)
        .unwrap_or_else(|e| {
            tracing::warn!(node_id = %node_id, error = %e, "failed to count active PC projects for node");
            0
        });

    Ok(Some(build_runtime_for_parts(
        node_id,
        owner_user_id,
        String::new(),
        None,
        String::new(),
        registry.as_ref(),
        cli,
        project_count,
    )))
}

pub fn supports_project_cli(allowed_clis: &[String]) -> bool {
    allowed_clis
        .iter()
        .any(|cli| cli.eq_ignore_ascii_case("copilot") || cli.eq_ignore_ascii_case("codex"))
}

pub fn clean_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn display_node_name(label: &str, device_name: Option<&str>, short_id: &str) -> String {
    clean_string(Some(label))
        .or_else(|| clean_string(device_name))
        .unwrap_or_else(|| short_id.to_string())
}

pub fn short_node_id(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() > 16 {
        let tail: String = chars[chars.len() - 14..].iter().collect();
        format!("...{tail}")
    } else {
        id.to_string()
    }
}

fn build_runtime(
    node_id: &str,
    credential: Option<&NodeCredential>,
    registry: Option<&NodeSummary>,
    cli: Option<&AgentSummary>,
    project_count: i64,
) -> NodeRuntime {
    let owner_user_id = credential
        .map(|credential| credential.owner_user_id.clone())
        .or_else(|| registry.map(|node| node.owner_user_id.clone()))
        .unwrap_or_default();
    let label = credential
        .map(|credential| credential.label.trim().to_string())
        .unwrap_or_default();
    let created_at = credential
        .map(|credential| credential.created_at.clone())
        .unwrap_or_default();
    build_runtime_for_parts(
        node_id,
        owner_user_id,
        label,
        credential.and_then(|credential| credential.device_name.as_deref()),
        created_at,
        registry,
        cli,
        project_count,
    )
}

fn build_runtime_for_parts(
    node_id: &str,
    owner_user_id: String,
    label: String,
    credential_device_name: Option<&str>,
    created_at: String,
    registry: Option<&NodeSummary>,
    cli: Option<&AgentSummary>,
    project_count: i64,
) -> NodeRuntime {
    let short_id = short_node_id(node_id);
    let device_name = registry
        .and_then(|node| clean_string(node.device_name.as_deref()))
        .or_else(|| cli.and_then(|agent| clean_string(agent.device_name.as_deref())))
        .or_else(|| clean_string(credential_device_name));
    let display_label = if label == node_id { "" } else { &label };
    let display_name = display_node_name(display_label, device_name.as_deref(), &short_id);
    let models = registry.map(|node| node.models.clone()).unwrap_or_default();
    let hardware = registry
        .and_then(|node| node.hardware.clone())
        .or_else(|| cli.and_then(|agent| agent.hardware.clone()));
    let registry_online = registry.map(|node| node.online).unwrap_or(false);
    let cli_connected = cli.is_some();
    let connected_at = registry
        .map(|node| node.connected_at)
        .unwrap_or_else(|| cli.map(|agent| agent.connected_at).unwrap_or(0));

    NodeRuntime {
        node_id: node_id.to_string(),
        owner_user_id,
        label,
        device_name,
        hardware,
        display_name,
        short_id,
        models,
        allowed_clis: cli
            .map(|agent| agent.allowed_clis.clone())
            .unwrap_or_default(),
        allowed_cwds: cli
            .map(|agent| agent.allowed_cwds.clone())
            .unwrap_or_default(),
        connected_at,
        created_at,
        online: registry_online || cli_connected,
        registry_online,
        cli_connected,
        project_count,
    }
}

fn user_project_counts_by_node(state: &AppState, user_id: &str) -> HashMap<String, i64> {
    match state.store.list_projects_for_user(user_id) {
        Ok(projects) => projects
            .into_iter()
            .filter_map(|project| project.node_id)
            .fold(HashMap::<String, i64>::new(), |mut counts, node_id| {
                *counts.entry(node_id).or_insert(0) += 1;
                counts
            }),
        Err(e) => {
            tracing::warn!(user_id = %user_id, error = %e, "failed to count user projects per node");
            HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_cli_support_accepts_codex_or_copilot_case_insensitive() {
        assert!(supports_project_cli(&["Codex".to_string()]));
        assert!(supports_project_cli(&["copilot".to_string()]));
        assert!(!supports_project_cli(&["node".to_string()]));
    }

    #[test]
    fn short_node_id_keeps_tail_for_long_ids() {
        assert_eq!(short_node_id("node-short"), "node-short");
        assert_eq!(
            short_node_id("node-user-1234567890abcdef"),
            "...34567890abcdef"
        );
    }
}
