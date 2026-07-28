use std::sync::Arc;

use tokio::sync::mpsc::UnboundedSender;
use tracing::warn;

use crate::{
    pc_agent_runtime_choice::{
        choose_pc_agent_runtime, PcAgentRuntimeChoice, PcRuntimeRoutePreference,
    },
    store::ProjectAccess,
    types::{AppState, WsMessage},
};

use super::pc_binding::{resolve_pc_chat_agent, resolve_pc_project_binding, PcProjectBinding};

#[derive(Debug)]
pub(super) struct PcProjectRuntimeBinding {
    pub(super) binding: PcProjectBinding,
    pub(super) runtime_choice: PcAgentRuntimeChoice,
}

#[derive(Debug)]
pub(super) struct PcChatRuntimeBinding {
    pub(super) agent_id: String,
    pub(super) runtime_choice: PcAgentRuntimeChoice,
}

pub(super) fn auto_runtime_route_candidates() -> &'static [PcRuntimeRoutePreference] {
    &[
        PcRuntimeRoutePreference::RouteA,
        PcRuntimeRoutePreference::RouteB,
        PcRuntimeRoutePreference::RouteC,
    ]
}

pub(super) async fn resolve_pc_project_runtime_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    tx: Option<&UnboundedSender<String>>,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Result<Option<PcProjectRuntimeBinding>, String> {
    let explicit_route = pc_runtime_route.is_some();
    for candidate in runtime_route_candidates(pc_runtime_route) {
        let Some(binding) = resolve_pc_project_binding(
            state,
            user_id,
            project,
            conversation_id,
            tx,
            candidate,
            !explicit_route,
        )
        .await
        else {
            continue;
        };
        let runtime_choice =
            choose_pc_agent_runtime(state, &binding.agent_id, agent_name, candidate).await;
        if let Some(error) = runtime_choice.error.clone() {
            if explicit_route {
                return Err(error);
            }
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %binding.agent_id,
                route = ?candidate,
                error = %error,
                "auto PC runtime route candidate is not ready"
            );
            continue;
        }
        send_auto_route_progress(tx, pc_runtime_route, candidate);
        return Ok(Some(PcProjectRuntimeBinding {
            binding,
            runtime_choice,
        }));
    }
    Ok(None)
}

pub(super) async fn resolve_pc_chat_runtime_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    tx: Option<&UnboundedSender<String>>,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Result<Option<PcChatRuntimeBinding>, String> {
    let explicit_route = pc_runtime_route.is_some();
    for candidate in runtime_route_candidates(pc_runtime_route) {
        let Some(agent_id) = resolve_pc_chat_agent(state, user_id, project, candidate).await else {
            continue;
        };
        let runtime_choice = choose_pc_agent_runtime(state, &agent_id, agent_name, candidate).await;
        if let Some(error) = runtime_choice.error.clone() {
            if explicit_route {
                return Err(error);
            }
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                route = ?candidate,
                error = %error,
                "auto PC chat runtime route candidate is not ready"
            );
            continue;
        }
        send_auto_route_progress(tx, pc_runtime_route, candidate);
        return Ok(Some(PcChatRuntimeBinding {
            agent_id,
            runtime_choice,
        }));
    }
    Ok(None)
}

fn runtime_route_candidates(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Vec<Option<PcRuntimeRoutePreference>> {
    if pc_runtime_route.is_some() {
        return vec![pc_runtime_route];
    }
    auto_runtime_route_candidates()
        .iter()
        .copied()
        .map(Some)
        .collect()
}

fn send_auto_route_progress(
    tx: Option<&UnboundedSender<String>>,
    requested_route: Option<PcRuntimeRoutePreference>,
    selected_route: Option<PcRuntimeRoutePreference>,
) {
    if requested_route.is_some() {
        return;
    }
    let Some(message) = auto_route_progress_message(selected_route) else {
        return;
    };
    if let Some(tx) = tx {
        let _ = tx.send(WsMessage::progress(message).to_json());
    }
}

fn auto_route_progress_message(
    selected_route: Option<PcRuntimeRoutePreference>,
) -> Option<&'static str> {
    match selected_route {
        Some(PcRuntimeRoutePreference::RouteA) => None,
        Some(PcRuntimeRoutePreference::RouteB) => {
            Some("本机 AI 工具未就绪，自动改用本机 API key。")
        }
        Some(PcRuntimeRoutePreference::RouteC) => {
            Some("本机 AI 和我的 Key 未配置，自动使用平台 AI。")
        }
        Some(PcRuntimeRoutePreference::RouteC2 | PcRuntimeRoutePreference::RouteC3) => None,
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{auto_route_progress_message, auto_runtime_route_candidates};
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;

    #[test]
    fn auto_candidates_keep_platform_ai_as_last_fallback() {
        assert_eq!(
            auto_runtime_route_candidates(),
            &[
                PcRuntimeRoutePreference::RouteA,
                PcRuntimeRoutePreference::RouteB,
                PcRuntimeRoutePreference::RouteC,
            ]
        );
    }

    #[test]
    fn local_route_a_does_not_emit_fallback_progress() {
        assert!(auto_route_progress_message(Some(PcRuntimeRoutePreference::RouteA)).is_none());
    }
}
