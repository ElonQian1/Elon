// server/src/agent.rs

use anyhow::Result;
use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    agent_api_loop::run_api_inner_with_workspace,
    agent_intent::{
        has_origin_remote, is_project_delivery_request, is_pure_project_delivery_message,
        is_short_build_command, is_short_resume_command,
    },
    agent_pc_workspace::{
        project_chat_should_use_pc_cli, project_cli_runtime_permission,
        project_cli_runtime_permission_fallback, project_requires_pc_workspace,
        should_attempt_pc_apk_sync,
    },
    agent_routing::{
        api_agent_name, choose_backend, has_api_agents, is_local_cli_option,
        requested_agent_for_runtime_route, resolve_cli_option_id,
    },
    ai_cli, context_compiler,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    pc_node_display::pc_node_progress_name,
    project_workspace_provision,
    route_a_session_lease::{self, RouteARuntimePrewarmResult},
    source_hygiene,
    store::{ProjectAccess, ProjectDevProfile, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AiBackend, AppState, UserAgentConfig, WsMessage},
};

mod pc_node_select;
mod public_dev;
use pc_node_select::{
    connected_pc_agent_for_route, connected_pc_agent_with_existing_workspace,
    connected_pc_agent_with_recorded_workspace_binding, connected_pc_project_agent_for_route,
};
#[cfg(test)]
use public_dev::{cli_lists_intersect, public_dev_runtime_ready_for_route};
use public_dev::{
    pc_agent_authorized_for_bound_node, pc_agent_authorized_for_route,
    pc_agent_belongs_to_user_quiet, pc_agent_public_dev_enabled_for_consumer,
    pc_agent_runtime_ready_for_route, route_allows_public_dev_node,
};

#[cfg(test)]
#[path = "agent_tests.rs"]
mod agent_tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectWorkflowRouting {
    Auto,
    ForceProjectWorkflow,
    ForceCasualChat,
}

/// 一龙自项目路径（默认 /root/Elon，可由 ELON_SELF_PATH 环境变量覆盖）
mod dispatch;
mod pc_binding;
mod pc_binding_utils;
mod routing;
mod runtime_binding;

pub(crate) use pc_binding::prewarm_route_a_runtime_for_project;
#[cfg(test)]
use pc_binding::{AUTO_BOUND_PC_NODE_RECONNECT_WAIT_SECS, BOUND_PC_NODE_RECONNECT_WAIT_SECS};

use dispatch::run_backend_with_workspace;
use pc_binding::{
    append_project_dev_profile_context, is_codex_fallback_error, node_cli_available,
    resolve_pc_project_binding_with_options, send_pc_workspace_unavailable_error,
};
#[cfg(test)]
use pc_binding::{
    pc_workspace_inspect_error_allows_bound_dispatch, pc_workspace_inspect_problem,
    pc_workspace_inspect_usable, pc_workspace_inspect_usable_for_route,
};
use routing::run_for_project_in_workspace_with_routing;
use runtime_binding::resolve_pc_project_runtime_binding;

pub fn elon_self_workspace() -> std::path::PathBuf {
    std::path::PathBuf::from(
        std::env::var("ELON_SELF_PATH").unwrap_or_else(|_| "/root/Elon".into()),
    )
}

pub async fn run_for_project(
    user_id: &str,
    project: &ProjectAccess,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    run_for_project_in_workspace_with_routing(
        user_id,
        project,
        &workspace,
        download_base,
        conversation_id,
        user_message,
        agent_name,
        pc_runtime_route,
        trace_id,
        state,
        tx,
        ProjectWorkflowRouting::Auto,
    )
    .await;
}

pub async fn run_project_workflow_for_project(
    user_id: &str,
    project: &ProjectAccess,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    run_for_project_in_workspace_with_routing(
        user_id,
        project,
        &workspace,
        download_base,
        conversation_id,
        user_message,
        agent_name,
        pc_runtime_route,
        trace_id,
        state,
        tx,
        ProjectWorkflowRouting::ForceProjectWorkflow,
    )
    .await;
}

pub async fn run_pc_cli_passthrough_for_project(
    user_id: &str,
    project: &ProjectAccess,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    project_preflight_note: Option<&str>,
    download_base: &str,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let pc_runtime_binding = match resolve_pc_project_runtime_binding(
        state,
        user_id,
        project,
        conversation_id,
        Some(&tx),
        agent_name,
        pc_runtime_route,
    )
    .await
    {
        Ok(Some(binding)) => binding,
        Ok(None) => {
            if project.source_type == "pc_managed" {
                send_pc_workspace_unavailable_error(project, &tx);
            } else {
                let _ = tx.send(WsMessage::error("当前项目还没有绑定可用 PC 节点。").to_json());
            }
            return;
        }
        Err(error) => {
            let _ = tx.send(WsMessage::error(error).to_json());
            return;
        }
    };
    let pc_binding = pc_runtime_binding.binding;
    let runtime_choice = pc_runtime_binding.runtime_choice;

    let agent_id = pc_binding.agent_id.as_str();
    let pc_workspace = pc_binding.workspace.as_str();

    let _ = tx.send(
        WsMessage::progress(format!(
            "正在直连 PC 节点 {} 使用 {} 处理本轮消息。",
            pc_node_progress_name(state.as_ref(), agent_id).await,
            runtime_choice.progress_label()
        ))
        .to_json(),
    );
    let preferred_runtime_permission = project_cli_runtime_permission(project);
    let session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
        project_id: project.id.clone(),
        user_id: user_id.to_string(),
        conversation_id: cid.to_string(),
        runtime_permission: preferred_runtime_permission.clone(),
    });
    let server_artifact_workspace = state.get_project_workspace(&project.workspace_key);
    let attempt_apk_sync = should_attempt_pc_apk_sync(project, user_message);

    let compiler_note = if project_preflight_note.is_some() {
        context_compiler::compile_preflight_note(
            state,
            server_artifact_workspace.as_path(),
            user_id,
            user_message,
            None,
        )
        .await
    } else {
        None
    };
    let combined_preflight_note =
        combine_project_preflight_notes(project_preflight_note, compiler_note.as_deref());
    let run_result = run_pc_project_cli_workspace(
        agent_id,
        user_id,
        pc_workspace,
        user_message,
        combined_preflight_note.as_deref(),
        session_scope.clone(),
        Some(download_base),
        Some(server_artifact_workspace.as_path()),
        attempt_apk_sync,
        Some(runtime_choice.cli_name.as_str()),
        runtime_choice.copilot_model.as_deref(),
        runtime_choice.codex_reasoning_effort.as_deref(),
        runtime_choice.model_label.as_deref(),
        state,
        &tx,
    )
    .await;
    let run_result = match run_result {
        Err(error) => {
            let error_str = error.to_string();
            if let Some(fallback_permission) =
                project_cli_runtime_permission_fallback(&preferred_runtime_permission, &error_str)
            {
                let _ = tx.send(
                    WsMessage::progress(
                        "本机节点尚未确认完全访问，已自动切换为项目目录写入模式重试。",
                    )
                    .to_json(),
                );
                let fallback_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
                    project_id: project.id.clone(),
                    user_id: user_id.to_string(),
                    conversation_id: cid.to_string(),
                    runtime_permission: fallback_permission.to_string(),
                });
                run_pc_project_cli_workspace(
                    agent_id,
                    user_id,
                    pc_workspace,
                    user_message,
                    combined_preflight_note.as_deref(),
                    fallback_scope,
                    Some(download_base),
                    Some(server_artifact_workspace.as_path()),
                    attempt_apk_sync,
                    Some(runtime_choice.cli_name.as_str()),
                    runtime_choice.copilot_model.as_deref(),
                    runtime_choice.codex_reasoning_effort.as_deref(),
                    runtime_choice.model_label.as_deref(),
                    state,
                    &tx,
                )
                .await
            } else {
                Err(error)
            }
        }
        other => other,
    };

    match run_result {
        Ok(ai_cli::PcAgentChatOutcome::Answered) => {}
        Ok(ai_cli::PcAgentChatOutcome::NoReadableReply { diagnostic }) => {
            let detail = diagnostic
                .unwrap_or_else(|| "这轮没有返回可读内容，请稍后直接重发一次。".to_string());
            let msg = format!("本机 AI：{detail}我不会自动切换到平台 AI。");
            warn!("{msg}");
            let _ = tx.send(WsMessage::error(msg).to_json());
        }
        Err(error) => {
            error!("PC 本地 Codex 直连运行出错: {}", error);
            let _ = tx.send(
                WsMessage::classified_error(crate::errors::classify_ai_error(&error.to_string()))
                    .to_json(),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_pc_project_cli_workspace(
    agent_id: &str,
    user_id: &str,
    workspace_path: &str,
    user_message: &str,
    preflight_note: Option<&str>,
    native_session_scope: Option<ai_cli::NativeSessionScope>,
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    attempt_apk_sync: bool,
    cli_name: Option<&str>,
    copilot_model: Option<&str>,
    codex_reasoning_effort: Option<&str>,
    model_label: Option<&str>,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<ai_cli::PcAgentChatOutcome> {
    ai_cli::run_with_pc_agent_passthrough_workspace(
        agent_id,
        user_id,
        workspace_path,
        user_message,
        preflight_note,
        native_session_scope,
        download_base,
        artifact_workspace,
        attempt_apk_sync,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    )
    .await
}

fn combine_project_preflight_notes(first: Option<&str>, second: Option<&str>) -> Option<String> {
    let notes = [first, second]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    (!notes.is_empty()).then(|| notes.join("\n\n---\n\n"))
}

pub async fn run_chat_only_for_project(
    user_id: &str,
    project: &ProjectAccess,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    run_for_project_in_workspace_with_routing(
        user_id,
        project,
        &workspace,
        download_base,
        conversation_id,
        user_message,
        agent_name,
        pc_runtime_route,
        trace_id,
        state,
        tx,
        ProjectWorkflowRouting::ForceCasualChat,
    )
    .await;
}

pub async fn run_project_workflow_for_project_in_workspace(
    user_id: &str,
    project: &ProjectAccess,
    workspace: &Path,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    run_for_project_in_workspace_with_routing(
        user_id,
        project,
        workspace,
        download_base,
        conversation_id,
        user_message,
        agent_name,
        pc_runtime_route,
        trace_id,
        state,
        tx,
        ProjectWorkflowRouting::ForceProjectWorkflow,
    )
    .await;
}

fn requires_project_workflow_for_message(user_message: &str, workspace: &Path) -> bool {
    if is_pure_project_delivery_message(user_message) {
        return false;
    }
    let decision = intent_router::classify(user_message);
    decision.route != CapabilityRoute::ChatAgent
        || is_short_resume_command(user_message, workspace)
        || is_short_build_command(user_message, workspace)
        || is_project_delivery_request(user_message, workspace)
}

fn latest_project_delivery_apk_url(
    state: &AppState,
    project: &ProjectAccess,
    workspace: &Path,
    download_base: &str,
) -> Option<String> {
    match state.store.latest_project_apk_url(&project.id) {
        Ok(Some(apk_url)) => return Some(apk_url),
        Ok(None) => {}
        Err(error) => warn!(
            project_id = %project.id,
            error = %error,
            "读取项目历史 APK 下载地址失败，回退到工作区扫描"
        ),
    }

    let managed_workspace = state.get_project_workspace(&project.workspace_key);
    let has_apk = tools::find_latest_apk(&managed_workspace).is_some()
        || tools::find_latest_apk(workspace).is_some();
    has_apk.then(|| tools::stable_apk_url(download_base))
}

fn pc_cli_chat_route_label(pc_runtime_route: Option<PcRuntimeRoutePreference>) -> &'static str {
    match pc_runtime_route {
        None => "自动选择",
        Some(PcRuntimeRoutePreference::RouteA) => "本机 AI",
        Some(PcRuntimeRoutePreference::RouteB) => "本机 API key",
        Some(PcRuntimeRoutePreference::RouteC) => "平台 AI",
        Some(PcRuntimeRoutePreference::RouteC2) => "远程 AI",
        Some(PcRuntimeRoutePreference::RouteC3) => "远程 Codex",
    }
}

pub async fn plan_for_project_in_workspace(
    user_id: &str,
    project: &ProjectAccess,
    workspace: &Path,
    download_base: &str,
    conversation_id: Option<&str>,
    user_message: &str,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let pc_runtime_binding = match resolve_pc_project_runtime_binding(
        state,
        user_id,
        project,
        conversation_id,
        Some(&tx),
        agent_name,
        pc_runtime_route,
    )
    .await
    {
        Ok(Some(binding)) => Some(binding),
        Ok(None) => None,
        Err(error) => {
            let _ = tx.send(WsMessage::error(error).to_json());
            return;
        }
    };
    if let Some(pc_runtime_binding) = pc_runtime_binding {
        let pc_binding = pc_runtime_binding.binding;
        let runtime_choice = pc_runtime_binding.runtime_choice;
        let agent_id = pc_binding.agent_id.as_str();
        let pc_workspace = pc_binding.workspace.as_str();
        let _ = tx.send(
            WsMessage::progress(format!(
                "正在直连 PC 节点 {} 使用 {} 规划本地项目。",
                pc_node_progress_name(state.as_ref(), agent_id).await,
                runtime_choice.progress_label()
            ))
            .to_json(),
        );
        let pc_user_message =
            append_project_dev_profile_context(state, user_id, project, user_message);
        let plan_session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: cid.to_string(),
            runtime_permission: project_cli_runtime_permission(project),
        });
        let plan_result = ai_cli::run_with_pc_agent_workspace(
            agent_id,
            user_id,
            pc_workspace,
            &pc_user_message,
            None,
            ai_cli::AiCliRequestMode::Plan,
            plan_session_scope.clone(),
            None,
            None,
            false,
            Some(runtime_choice.cli_name.as_str()),
            runtime_choice.copilot_model.as_deref(),
            runtime_choice.codex_reasoning_effort.as_deref(),
            runtime_choice.model_label.as_deref(),
            state,
            &tx,
        )
        .await;
        if let Err(e) = plan_result {
            let error_str = e.to_string();
            if runtime_choice.cli_name == "codex"
                && is_codex_fallback_error(&error_str)
                && node_cli_available(state, agent_id, "copilot").await
            {
                let _ = tx.send(
                    WsMessage::progress("Codex 额度已用尽，正在自动切换到 Copilot 继续规划…")
                        .to_json(),
                );
                if let Err(e2) = ai_cli::run_with_pc_agent_workspace(
                    agent_id,
                    user_id,
                    pc_workspace,
                    &pc_user_message,
                    None,
                    ai_cli::AiCliRequestMode::Plan,
                    plan_session_scope,
                    None,
                    None,
                    false,
                    Some("copilot"),
                    None,
                    None,
                    Some("Copilot（Codex 额度回退）"),
                    state,
                    &tx,
                )
                .await
                {
                    error!("Copilot 规划回退出错: {}", e2);
                    let _ = tx.send(WsMessage::error(e2.to_string()).to_json());
                }
            } else {
                error!("PC 本地项目规划运行出错: {}", e);
                let _ = tx.send(WsMessage::error(error_str).to_json());
            }
        }
        return;
    }

    if project_requires_pc_workspace(project) {
        send_pc_workspace_unavailable_error(project, &tx);
        return;
    }

    let user_config_workspace = state.get_user_workspace(user_id);
    let route_agent_name = requested_agent_for_runtime_route(agent_name, pc_runtime_route);
    if let Err(e) = run_backend_with_workspace(
        user_id,
        workspace,
        &user_config_workspace,
        download_base,
        Some(ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.unwrap_or("default").to_string(),
            runtime_permission: project_cli_runtime_permission(project),
        }),
        user_message,
        None,
        route_agent_name,
        trace_id,
        CapabilityRoute::CodeAgent,
        true,
        false,
        true,
        state,
        &tx,
    )
    .await
    {
        error!("项目级规划 AI 运行出错: {}", e);
        let _ = tx.send(WsMessage::error(e.to_string()).to_json());
    }
}
