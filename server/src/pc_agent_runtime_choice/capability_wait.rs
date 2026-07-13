use std::{sync::Arc, time::Duration};

use tokio::time::{sleep, Instant};

use super::PcRuntimeRoutePreference;
use crate::{homecli_agent::AgentSummary, types::AppState};

const ROUTE_A_CAPABILITY_WAIT: Duration = Duration::from_secs(8);
const ROUTE_A_CAPABILITY_POLL: Duration = Duration::from_millis(250);

pub(super) async fn agent_summary_after_capability_scan(
    state: &Arc<AppState>,
    agent_id: &str,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> Option<AgentSummary> {
    let mut summary = find_agent_summary(state, agent_id).await;
    if !should_wait_for_route_a_capabilities(summary.as_ref(), route_preference) {
        return summary;
    }

    let deadline = Instant::now() + ROUTE_A_CAPABILITY_WAIT;
    while Instant::now() < deadline {
        sleep(ROUTE_A_CAPABILITY_POLL).await;
        summary = find_agent_summary(state, agent_id).await;
        if !should_wait_for_route_a_capabilities(summary.as_ref(), route_preference) {
            break;
        }
    }
    summary
}

async fn find_agent_summary(state: &Arc<AppState>, agent_id: &str) -> Option<AgentSummary> {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|agent| agent.agent_id == agent_id)
}

fn should_wait_for_route_a_capabilities(
    summary: Option<&AgentSummary>,
    route_preference: Option<PcRuntimeRoutePreference>,
) -> bool {
    if !matches!(
        route_preference,
        Some(PcRuntimeRoutePreference::RouteA | PcRuntimeRoutePreference::RouteC3)
    ) {
        return false;
    }
    let Some(summary) = summary else {
        return false;
    };
    if !summary.allowed_clis.is_empty() {
        return false;
    }
    summary
        .dev_runtime
        .as_ref()
        .map(|runtime| runtime.toolchains.is_empty())
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use homecli_proto::{DevToolchainStatus, NodeDevRuntimeProfile};

    use super::*;

    #[test]
    fn forced_route_a_waits_only_until_capability_probe_finishes() {
        let initial_register = agent_summary(Vec::new(), None);
        assert!(should_wait_for_route_a_capabilities(
            Some(&initial_register),
            Some(PcRuntimeRoutePreference::RouteA),
        ));

        let scanned_without_cli = agent_summary(
            Vec::new(),
            Some(NodeDevRuntimeProfile {
                route_a_ready: false,
                toolchains: vec![DevToolchainStatus {
                    name: "codex".to_string(),
                    available: false,
                    version: None,
                    path: None,
                }],
                ..Default::default()
            }),
        );
        assert!(!should_wait_for_route_a_capabilities(
            Some(&scanned_without_cli),
            Some(PcRuntimeRoutePreference::RouteA),
        ));

        let scanned_with_cli = agent_summary(vec!["codex".to_string()], None);
        assert!(!should_wait_for_route_a_capabilities(
            Some(&scanned_with_cli),
            Some(PcRuntimeRoutePreference::RouteA),
        ));
        assert!(!should_wait_for_route_a_capabilities(
            Some(&initial_register),
            Some(PcRuntimeRoutePreference::RouteB),
        ));
    }

    fn agent_summary(
        allowed_clis: Vec<String>,
        dev_runtime: Option<NodeDevRuntimeProfile>,
    ) -> AgentSummary {
        AgentSummary {
            agent_id: "node-a".to_string(),
            version: "test".to_string(),
            proto_version: homecli_proto::PROTO_VERSION,
            capabilities: vec![homecli_proto::CAP_PROJECT_BUILD_CACHE_V1.to_string()],
            device_name: None,
            hardware: None,
            storage: None,
            dev_runtime,
            lifecycle: None,
            allowed_clis,
            allowed_cwds: Vec::new(),
            connected_at: 1,
        }
    }
}
