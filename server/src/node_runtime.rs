use anyhow::Result;
use homecli_proto::{
    ModelCapability, NodeDevRuntimeProfile, NodeHardwareProfile, NodeLifecycleReport,
    NodeStorageProfile,
};
use std::collections::{HashMap, HashSet};

use crate::{
    homecli_agent::AgentSummary, node_registry::NodeSummary, store::NodeCredential, types::AppState,
};

#[derive(Debug, Clone)]
pub struct NodeRuntime {
    pub node_id: String,
    pub owner_user_id: String,
    pub label: String,
    pub device_name: Option<String>,
    pub install_id: Option<String>,
    pub public_dev_enabled: bool,
    pub public_dev_allowed_clis: Vec<String>,
    pub public_dev_permission_level: String,
    pub last_handshake_at: Option<String>,
    pub last_handshake_agent_version: Option<String>,
    pub last_handshake_allowed_clis: Vec<String>,
    pub last_handshake_route_a_ready: bool,
    pub last_handshake_api_runtime_ready: bool,
    pub last_handshake_server_runtime_ready: bool,
    pub last_handshake_ai_cli_ready: bool,
    pub hardware: Option<NodeHardwareProfile>,
    pub storage: Option<NodeStorageProfile>,
    pub dev_runtime: Option<NodeDevRuntimeProfile>,
    pub lifecycle: Option<NodeLifecycleReport>,
    pub display_name: String,
    pub short_id: String,
    pub models: Vec<ModelCapability>,
    pub allowed_clis: Vec<String>,
    pub allowed_cwds: Vec<String>,
    pub agent_version: Option<String>,
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

    pub fn workspace_provision_ready(&self) -> bool {
        self.dev_runtime
            .as_ref()
            .map(|runtime| runtime.workspace_provision_ready)
            .unwrap_or_else(|| self.cli_project_ready())
    }

    pub fn storage_ready(&self) -> bool {
        self.storage
            .as_ref()
            .map(|storage| storage.enabled)
            .unwrap_or(false)
            && self.cli_connected
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

    let mut nodes = dedupe_node_runtimes(nodes);
    nodes.sort_by(preferred_node_order);
    Ok(nodes)
}

pub async fn node_runtime_by_id(state: &AppState, node_id: &str) -> Result<Option<NodeRuntime>> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Ok(None);
    }
    let credential = state.store.get_node_credential(node_id)?;
    let registry = state
        .node_registry
        .list_online()
        .await
        .into_iter()
        .find(|node| node.node_id == node_id);
    let cli_agents = state.agent_manager.list().await;
    let cli = cli_agents.iter().find(|agent| agent.agent_id == node_id);
    if registry.is_none() && cli.is_none() && credential.is_none() {
        return Ok(None);
    }

    let owner_user_id = registry
        .as_ref()
        .map(|node| node.owner_user_id.clone())
        .or_else(|| {
            credential
                .as_ref()
                .map(|credential| credential.owner_user_id.clone())
        })
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
        credential
            .as_ref()
            .map(|credential| credential.label.trim().to_string())
            .unwrap_or_default(),
        credential
            .as_ref()
            .and_then(|credential| credential.device_name.as_deref()),
        credential
            .as_ref()
            .and_then(|credential| credential.install_id.as_deref()),
        credential.as_ref(),
        credential
            .as_ref()
            .map(|credential| credential.created_at.clone())
            .unwrap_or_default(),
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
        credential.and_then(|credential| credential.install_id.as_deref()),
        credential,
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
    credential_install_id: Option<&str>,
    credential: Option<&NodeCredential>,
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
    let install_id = clean_string(credential_install_id);
    let display_label = if label == node_id { "" } else { &label };
    let display_name = display_node_name(display_label, device_name.as_deref(), &short_id);
    let models = registry.map(|node| node.models.clone()).unwrap_or_default();
    let hardware = registry
        .and_then(|node| node.hardware.clone())
        .or_else(|| cli.and_then(|agent| agent.hardware.clone()));
    let storage = registry
        .and_then(|node| node.storage.clone())
        .or_else(|| cli.and_then(|agent| agent.storage.clone()));
    let dev_runtime = registry
        .and_then(|node| node.dev_runtime.clone())
        .or_else(|| cli.and_then(|agent| agent.dev_runtime.clone()));
    let lifecycle = registry
        .and_then(|node| node.lifecycle.clone())
        .or_else(|| cli.and_then(|agent| agent.lifecycle.clone()));
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
        install_id,
        public_dev_enabled: credential
            .map(|credential| credential.public_dev_enabled)
            .unwrap_or(false),
        public_dev_allowed_clis: credential
            .map(|credential| credential.public_dev_allowed_clis.clone())
            .unwrap_or_default(),
        public_dev_permission_level: credential
            .map(|credential| credential.public_dev_permission_level.clone())
            .unwrap_or_else(|| "project_write".to_string()),
        last_handshake_at: credential.and_then(|credential| credential.last_handshake_at.clone()),
        last_handshake_agent_version: credential
            .and_then(|credential| credential.last_handshake_agent_version.clone()),
        last_handshake_allowed_clis: credential
            .map(|credential| credential.last_handshake_allowed_clis.clone())
            .unwrap_or_default(),
        last_handshake_route_a_ready: credential
            .map(|credential| credential.last_handshake_route_a_ready)
            .unwrap_or(false),
        last_handshake_api_runtime_ready: credential
            .map(|credential| credential.last_handshake_api_runtime_ready)
            .unwrap_or(false),
        last_handshake_server_runtime_ready: credential
            .map(|credential| credential.last_handshake_server_runtime_ready)
            .unwrap_or(false),
        last_handshake_ai_cli_ready: credential
            .map(|credential| credential.last_handshake_ai_cli_ready)
            .unwrap_or(false),
        hardware,
        storage,
        dev_runtime,
        lifecycle,
        display_name,
        short_id,
        models,
        allowed_clis: cli
            .map(|agent| agent.allowed_clis.clone())
            .unwrap_or_default(),
        allowed_cwds: cli
            .map(|agent| agent.allowed_cwds.clone())
            .unwrap_or_default(),
        agent_version: cli.map(|agent| agent.version.clone()),
        connected_at,
        created_at,
        online: registry_online || cli_connected,
        registry_online,
        cli_connected,
        project_count,
    }
}

fn preferred_node_order(left: &NodeRuntime, right: &NodeRuntime) -> std::cmp::Ordering {
    right
        .online
        .cmp(&left.online)
        .then(right.project_count.cmp(&left.project_count))
        .then(right.connected_at.cmp(&left.connected_at))
        .then(right.created_at.cmp(&left.created_at))
        .then_with(|| left.display_name.cmp(&right.display_name))
}

fn dedupe_node_runtimes(mut nodes: Vec<NodeRuntime>) -> Vec<NodeRuntime> {
    nodes.sort_by(preferred_node_order);
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(nodes.len());
    for node in nodes {
        if let Some(key) = node_dedupe_key(&node) {
            let first_for_device = seen.insert(key);
            if !first_for_device && node.project_count == 0 {
                continue;
            }
        }
        result.push(node);
    }
    result
}

fn node_dedupe_key(node: &NodeRuntime) -> Option<String> {
    if let Some(install_id) = clean_string(node.install_id.as_deref()) {
        return Some(format!("install:{}", normalize_dedupe_key(&install_id)));
    }
    if let Some(device_name) = clean_string(node.device_name.as_deref()) {
        return Some(format!(
            "legacy-device:{}",
            normalize_dedupe_key(&device_name)
        ));
    }
    if !node.online {
        if let Some(display_name) = clean_string(Some(&node.display_name)) {
            return Some(format!(
                "legacy-label:{}",
                normalize_dedupe_key(&display_name)
            ));
        }
    }
    None
}

fn normalize_dedupe_key(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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
#[path = "node_runtime_tests.rs"]
mod tests;
