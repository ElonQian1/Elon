// server/src/agent.rs

use anyhow::Result;
use homecli_proto::{AgentToServer, ProjectWorkspaceInspectStatus};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    agent_api_loop::run_api_inner_with_workspace,
    agent_intent::{
        has_origin_remote, is_project_delivery_request, is_pure_project_delivery_message,
        is_short_build_command, is_short_resume_command,
    },
    agent_routing::{
        api_agent_name, choose_backend, has_api_agents, is_local_cli_option, resolve_cli_option_id,
    },
    ai_cli, context_compiler,
    intent_router::{self, CapabilityRoute, RoutingDecision},
    pc_agent_runtime_choice::{choose_pc_agent_runtime, PcRuntimeRoutePreference},
    pc_node_display::pc_node_progress_name,
    project_workspace_provision, source_hygiene,
    store::{ProjectAccess, ProjectDevProfile, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AiBackend, AppState, UserAgentConfig, WsMessage},
};

const BOUND_PC_NODE_RECONNECT_WAIT_SECS: u64 = 120;
const BOUND_PC_NODE_RECONNECT_POLL_MS: u64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectWorkflowRouting {
    Auto,
    ForceProjectWorkflow,
    ForceCasualChat,
}

/// 一龙自项目路径（默认 /root/Elon，可由 ELON_SELF_PATH 环境变量覆盖）
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
    state: &Arc<AppState>,
    tx: UnboundedSender<String>,
) {
    let Some(pc_binding) = resolve_pc_project_binding(state, user_id, project, Some(&tx)).await
    else {
        if project.source_type == "pc_managed" {
            send_pc_workspace_unavailable_error(project, &tx);
        } else {
            let _ = tx.send(WsMessage::error("当前项目还没有绑定可用 PC 节点。").to_json());
        }
        return;
    };

    let agent_id = pc_binding.agent_id.as_str();
    let pc_workspace = pc_binding.workspace.as_str();
    let runtime_choice =
        choose_pc_agent_runtime(state, agent_id, agent_name, pc_runtime_route).await;
    if let Some(error) = runtime_choice.error {
        let _ = tx.send(WsMessage::error(error).to_json());
        return;
    }

    let _ = tx.send(
        WsMessage::progress(format!(
            "正在直连 PC 节点 {} 使用 {} 处理本轮消息。",
            pc_node_progress_name(state.as_ref(), agent_id).await,
            runtime_choice.progress_label()
        ))
        .to_json(),
    );
    let session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
        project_id: project.id.clone(),
        user_id: user_id.to_string(),
        conversation_id: cid.to_string(),
        runtime_permission: project.runtime_permission.clone(),
    });

    match ai_cli::run_with_pc_agent_passthrough_workspace(
        agent_id,
        user_id,
        pc_workspace,
        user_message,
        session_scope,
        Some(runtime_choice.cli_name.as_str()),
        runtime_choice.copilot_model.as_deref(),
        runtime_choice.codex_reasoning_effort.as_deref(),
        runtime_choice.model_label.as_deref(),
        state,
        &tx,
    )
    .await
    {
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

async fn run_for_project_in_workspace_with_routing(
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
    workflow_routing: ProjectWorkflowRouting,
) {
    let user_config_workspace = state.get_user_workspace(user_id);
    let require_existing_git = matches!(
        project.source_type.as_str(),
        "local_path" | "github" | "pc_managed"
    );
    let auto_requires_project_workflow =
        requires_project_workflow_for_message(user_message, workspace);
    let requires_project_workflow = match workflow_routing {
        ProjectWorkflowRouting::Auto => auto_requires_project_workflow,
        ProjectWorkflowRouting::ForceProjectWorkflow => true,
        ProjectWorkflowRouting::ForceCasualChat => false,
    };

    if is_pure_project_delivery_message(user_message) {
        let apk_url = latest_project_delivery_apk_url(state, project, workspace, download_base);
        let message = if apk_url.is_some() {
            "这个项目的安装包已经放到项目空间的「安装」按钮上了。你可以回到项目空间点「安装」自动下载并安装；我也把直接下载链接发给你。"
        } else {
            "我查了这个项目，目前还没有可安装 APK 记录。等开发任务真正构建出安装包后，项目空间的「安装」按钮会自动出现下载入口。"
        };
        let _ = tx.send(
            WsMessage::Done {
                message: message.to_string(),
                apk_url,
                image_url: None,
                model_used: None,
                node_id: None,
            }
            .to_json(),
        );
        return;
    }

    if !requires_project_workflow {
        let requested_agent_name = requested_agent_for_runtime_route(agent_name, pc_runtime_route);
        let agent_is_local_cli = agent_name
            .map(|name| is_local_cli_option(state, name))
            .unwrap_or(false);
        if project_chat_should_use_pc_cli(pc_runtime_route, agent_name, agent_is_local_cli) {
            let route_label = pc_cli_chat_route_label(pc_runtime_route);
            let Some(agent_id) = resolve_pc_chat_agent(state, user_id, project).await else {
                let msg = format!(
                    "项目会话默认交给{route_label}处理，但当前项目还没有绑定可用 PC 节点。请先连接节点，或手动切换到平台 AI。"
                );
                warn!("{msg}");
                let _ = tx.send(WsMessage::error(msg).to_json());
                return;
            };
            let agent_id = agent_id.as_str();

            let runtime_choice =
                choose_pc_agent_runtime(state, agent_id, agent_name, pc_runtime_route).await;
            if let Some(error) = runtime_choice.error {
                warn!("PC CLI 轻量聊天不可用，不回退 API: {}", error);
                let _ = tx.send(WsMessage::error(error).to_json());
                return;
            }

            let session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
                project_id: project.id.clone(),
                user_id: user_id.to_string(),
                conversation_id: cid.to_string(),
                runtime_permission: "read_only".to_string(),
            });
            match ai_cli::run_with_pc_agent_chat(
                agent_id,
                user_id,
                user_message,
                session_scope,
                Some(runtime_choice.cli_name.as_str()),
                runtime_choice.copilot_model.as_deref(),
                runtime_choice.codex_reasoning_effort.as_deref(),
                runtime_choice.model_label.as_deref(),
                state,
                &tx,
            )
            .await
            {
                Ok(ai_cli::PcAgentChatOutcome::Answered) => return,
                Ok(ai_cli::PcAgentChatOutcome::NoReadableReply { diagnostic }) => {
                    let detail = diagnostic.unwrap_or_else(|| {
                        "这轮没有返回可读内容，请稍后直接重发一次。".to_string()
                    });
                    let msg = format!("{route_label}：{detail}我不会自动切换到平台 AI。");
                    warn!("{msg}");
                    let _ = tx.send(WsMessage::error(msg).to_json());
                    return;
                }
                Err(error) => {
                    let msg = format!("{route_label}执行失败：{error}");
                    warn!("{msg}");
                    let _ = tx.send(WsMessage::error(msg).to_json());
                    return;
                }
            }
        }

        if let Err(error) = run_api_inner_with_workspace(
            user_id,
            workspace,
            &user_config_workspace,
            download_base,
            user_message,
            None,
            trace_id,
            api_agent_name(state, requested_agent_name),
            false,
            Some(MEMORY_SCOPE_PROJECT),
            Some(&project.id),
            state,
            &tx,
        )
        .await
        {
            error!("项目普通聊天 AI 运行出错: {}", error);
            let _ = tx.send(
                WsMessage::classified_error(crate::errors::classify_ai_error(&error.to_string()))
                    .to_json(),
            );
        }
        return;
    }

    if requires_project_workflow {
        if let Some(pc_binding) =
            resolve_pc_project_binding(state, user_id, project, Some(&tx)).await
        {
            let agent_id = pc_binding.agent_id.as_str();
            let pc_workspace = pc_binding.workspace.as_str();
            let runtime_choice =
                choose_pc_agent_runtime(state, agent_id, agent_name, pc_runtime_route).await;
            if let Some(error) = runtime_choice.error {
                let _ = tx.send(WsMessage::error(error).to_json());
                return;
            }
            let _ = tx.send(
                WsMessage::progress(format!(
                    "正在直连 PC 节点 {} 使用 {} 处理本地项目。",
                    pc_node_progress_name(state.as_ref(), agent_id).await,
                    runtime_choice.progress_label()
                ))
                .to_json(),
            );
            let session_scope = conversation_id.map(|cid| ai_cli::NativeSessionScope {
                project_id: project.id.clone(),
                user_id: user_id.to_string(),
                conversation_id: cid.to_string(),
                runtime_permission: project.runtime_permission.clone(),
            });
            let pc_user_message =
                append_project_dev_profile_context(state, user_id, project, user_message);
            let server_artifact_workspace = state.get_project_workspace(&project.workspace_key);
            let attempt_apk_sync = should_attempt_pc_apk_sync(project, &pc_user_message);
            let run_result = ai_cli::run_with_pc_agent_workspace(
                agent_id,
                user_id,
                pc_workspace,
                &pc_user_message,
                None,
                ai_cli::AiCliRequestMode::Execute,
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
            if let Err(e) = run_result {
                let error_str = e.to_string();
                // Codex 额度/认证失败时，自动切换 Copilot 重试
                if runtime_choice.cli_name == "codex"
                    && is_codex_fallback_error(&error_str)
                    && node_cli_available(state, agent_id, "copilot").await
                {
                    let _ = tx.send(
                        WsMessage::progress("Codex 额度已用尽，正在自动切换到 Copilot 继续执行…")
                            .to_json(),
                    );
                    if let Err(e2) = ai_cli::run_with_pc_agent_workspace(
                        agent_id,
                        user_id,
                        pc_workspace,
                        &pc_user_message,
                        None,
                        ai_cli::AiCliRequestMode::Execute,
                        session_scope,
                        Some(download_base),
                        Some(server_artifact_workspace.as_path()),
                        attempt_apk_sync,
                        Some("copilot"),
                        None,
                        None,
                        Some("Copilot（Codex 额度回退）"),
                        state,
                        &tx,
                    )
                    .await
                    {
                        error!("Copilot 回退执行出错: {}", e2);
                        let _ = tx.send(
                            WsMessage::classified_error(crate::errors::classify_ai_error(
                                &e2.to_string(),
                            ))
                            .to_json(),
                        );
                    }
                } else {
                    error!("PC 本地项目代理运行出错: {}", e);
                    let _ = tx.send(
                        WsMessage::classified_error(crate::errors::classify_ai_error(&error_str))
                            .to_json(),
                    );
                }
            }
            return;
        }
        if project.source_type == "pc_managed" {
            send_pc_workspace_unavailable_error(project, &tx);
            return;
        }
    }

    if requires_project_workflow && project_requires_pc_workspace(project) {
        send_pc_workspace_unavailable_error(project, &tx);
        return;
    }

    let require_git_for_this_request = require_existing_git && requires_project_workflow;
    let mut preflight_note: Option<String> = None;
    if require_git_for_this_request
        && (!workspace.join(".git").exists() || !has_origin_remote(&workspace))
    {
        let _ = tx.send(
            WsMessage::error(format!(
                "当前项目被标记为 Git/local_path 项目，但 {} 不是带 origin 远端的 Git 仓库。请先把它设置成真实 git clone，并配置可用远端后再继续。",
                workspace.display()
            ))
            .to_json(),
        );
        return;
    }
    if require_git_for_this_request {
        match tools::git_fetch_status(&workspace) {
            Ok(msg) => {
                if msg.starts_with("git fetch 未成功")
                    || msg.contains("本地落后")
                    || msg.contains("未推送提交")
                    || msg.contains("已分叉")
                {
                    warn!("项目同步预检未完成，交给 AI CLI 处理: {}", msg);
                    let _ = tx.send(
                        WsMessage::progress("同步状态检查已完成，远端状态已交给 AI 助手处理。")
                            .to_json(),
                    );
                    preflight_note = Some(msg);
                } else {
                    let _ = tx.send(WsMessage::progress(msg).to_json());
                }
            }
            Err(e) => {
                let msg = format!("git fetch origin main 执行出错: {}", e);
                warn!("项目同步预检执行出错，交给 AI CLI 处理: {}", msg);
                let _ = tx
                    .send(WsMessage::progress("同步检查执行出错，已交给 AI 助手处理。").to_json());
                preflight_note = Some(msg);
            }
        }
    }
    if let Err(e) = run_dispatch_with_workspace(
        user_id,
        &workspace,
        &user_config_workspace,
        download_base,
        Some(ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.to_string(),
            conversation_id: conversation_id.unwrap_or("default").to_string(),
            runtime_permission: project.runtime_permission.clone(),
        }),
        user_message,
        preflight_note.as_deref(),
        agent_name,
        pc_runtime_route,
        trace_id,
        require_git_for_this_request,
        workflow_routing == ProjectWorkflowRouting::ForceProjectWorkflow,
        state,
        &tx,
    )
    .await
    {
        error!("项目级 AI 代理运行出错: {}", e);
        let _ = tx.send(
            WsMessage::classified_error(crate::errors::classify_ai_error(&e.to_string())).to_json(),
        );
    }
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

fn should_attempt_pc_apk_sync(project: &ProjectAccess, user_message: &str) -> bool {
    project.template.eq_ignore_ascii_case("android")
        || ai_cli::looks_like_android_task(user_message)
}

fn pc_cli_chat_requested(pc_runtime_route: Option<PcRuntimeRoutePreference>) -> bool {
    matches!(
        pc_runtime_route,
        Some(PcRuntimeRoutePreference::RouteA | PcRuntimeRoutePreference::RouteC3)
    )
}

fn project_chat_should_use_pc_cli(
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    agent_name: Option<&str>,
    agent_is_local_cli: bool,
) -> bool {
    if pc_cli_chat_requested(pc_runtime_route) {
        return true;
    }
    if matches!(
        pc_runtime_route,
        Some(
            PcRuntimeRoutePreference::RouteB
                | PcRuntimeRoutePreference::RouteC
                | PcRuntimeRoutePreference::RouteC2
        )
    ) {
        return false;
    }
    if agent_name
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
    {
        return agent_is_local_cli;
    }
    true
}

fn pc_cli_chat_route_label(pc_runtime_route: Option<PcRuntimeRoutePreference>) -> &'static str {
    match pc_runtime_route {
        Some(PcRuntimeRoutePreference::RouteC3) => "远程 Codex",
        _ => "本机 AI",
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
    if let Some(pc_binding) = resolve_pc_project_binding(state, user_id, project, Some(&tx)).await {
        let agent_id = pc_binding.agent_id.as_str();
        let pc_workspace = pc_binding.workspace.as_str();
        let runtime_choice =
            choose_pc_agent_runtime(state, agent_id, agent_name, pc_runtime_route).await;
        if let Some(error) = runtime_choice.error {
            let _ = tx.send(WsMessage::error(error).to_json());
            return;
        }
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
            runtime_permission: project.runtime_permission.clone(),
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
            runtime_permission: project.runtime_permission.clone(),
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
        error!("项目级 AI 规划运行出错: {}", e);
        let _ = tx.send(WsMessage::error(e.to_string()).to_json());
    }
}

fn requested_agent_for_runtime_route<'a>(
    agent_name: Option<&'a str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
) -> Option<&'a str> {
    if agent_name
        .map(str::trim)
        .is_some_and(|name| !name.is_empty())
    {
        return agent_name;
    }
    match pc_runtime_route {
        Some(
            PcRuntimeRoutePreference::RouteB
            | PcRuntimeRoutePreference::RouteC
            | PcRuntimeRoutePreference::RouteC2,
        ) => Some("api"),
        _ => agent_name,
    }
}

#[derive(Debug, Clone)]
struct PcProjectBinding {
    agent_id: String,
    workspace: String,
}

fn project_requires_pc_workspace(project: &ProjectAccess) -> bool {
    project_fields_require_pc_workspace(
        &project.source_type,
        project.node_id.as_deref(),
        project.workspace_path.as_deref(),
    )
}

fn project_fields_require_pc_workspace(
    source_type: &str,
    node_id: Option<&str>,
    workspace_path: Option<&str>,
) -> bool {
    if source_type == "pc_managed" {
        return true;
    }
    if node_id
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return true;
    }
    workspace_path
        .map(str::trim)
        .is_some_and(path_looks_windows_workspace)
}

fn path_looks_windows_workspace(path: &str) -> bool {
    let value = path.trim();
    if value.starts_with("\\\\") || value.starts_with("//") {
        return true;
    }
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

async fn resolve_pc_chat_agent(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
) -> Option<String> {
    if let Some(agent_id) = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if pc_agent_belongs_to_user(state, user_id, agent_id)
            && pc_agent_is_connected(state, agent_id).await
        {
            return Some(agent_id.to_string());
        }
    }
    connected_pc_agent_for_user(state, user_id).await
}

async fn resolve_pc_project_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    tx: Option<&UnboundedSender<String>>,
) -> Option<PcProjectBinding> {
    let workspace = project
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut bound_agent_wrong_owner = false;
    let bound_agent_missing = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none();

    if let Some(agent_id) = project
        .node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let belongs_to_user = pc_agent_belongs_to_user(state, user_id, agent_id);
        if belongs_to_user {
            let connected = if pc_agent_is_connected(state, agent_id).await {
                true
            } else {
                wait_for_bound_pc_agent_reconnect(state, agent_id, tx).await
            };
            if connected {
                if let Some(binding) =
                    usable_project_binding_for_agent(state, user_id, project, agent_id, true, tx)
                        .await
                {
                    return Some(binding);
                }
            }
        } else {
            bound_agent_wrong_owner = true;
        }
        warn!(
            project_id = %project.id,
            user_id = %user_id,
            bound_agent_id = %agent_id,
            "PC project bound node is not usable for the current user; trying an online user node"
        );
    }

    if let Some(binding) = connected_pc_agent_with_recorded_workspace_binding(
        state,
        user_id,
        project,
        project.node_id.as_deref(),
    )
    .await
    {
        send_optional_progress(tx, "已找到当前节点记录的项目路径，正在切换执行。");
        return Some(binding);
    }

    if let Some(workspace) = workspace {
        if let Some(fallback_agent_id) = connected_pc_agent_with_existing_workspace(
            state,
            user_id,
            workspace,
            project.node_id.as_deref(),
        )
        .await
        {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                fallback_agent_id = %fallback_agent_id,
                workspace_path = %workspace,
                "PC project will run on another online node that has the same workspace path"
            );
            send_optional_progress(tx, "已找到同一路径可用的在线 PC 节点，正在切换执行。");
            return Some(PcProjectBinding {
                agent_id: fallback_agent_id,
                workspace: workspace.to_string(),
            });
        }
    }

    if project.source_type != "pc_managed" {
        warn!(
            project_id = %project.id,
            user_id = %user_id,
            workspace_path = ?workspace,
            "local path PC project has no online node with the recorded workspace"
        );
        return None;
    }

    let fallback_agent_id = connected_pc_project_agent_for_user(state, user_id).await?;
    warn!(
        project_id = %project.id,
        user_id = %user_id,
        fallback_agent_id = %fallback_agent_id,
        "PC project will run on the current user's online node"
    );
    if project.source_type == "pc_managed" {
        let clone_url = clone_url_for_project_access(project, &fallback_agent_id);
        let can_recreate_without_remote = workspace.is_none()
            || clone_url.is_some()
            || (project.role == "owner" && (bound_agent_wrong_owner || bound_agent_missing));
        if can_recreate_without_remote {
            return provision_pc_project_binding(
                state,
                user_id,
                project,
                &fallback_agent_id,
                clone_url,
                tx,
            )
            .await;
        }

        warn!(
            project_id = %project.id,
            user_id = %user_id,
            fallback_agent_id = %fallback_agent_id,
            workspace_path = ?workspace,
            "PC managed project cannot move to fallback node because no portable git/storage source is available"
        );
        return None;
    }
    let workspace = workspace?;
    Some(PcProjectBinding {
        agent_id: fallback_agent_id,
        workspace: workspace.to_string(),
    })
}

async fn connected_pc_agent_with_recorded_workspace_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    skip_agent_id: Option<&str>,
) -> Option<PcProjectBinding> {
    let skip_agent_id = skip_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    for agent in state.agent_manager.list().await {
        if skip_agent_id == Some(agent.agent_id.as_str()) {
            continue;
        }
        if !pc_agent_belongs_to_user(state, user_id, &agent.agent_id) {
            continue;
        }
        if let Some(binding) =
            usable_project_binding_for_agent(state, user_id, project, &agent.agent_id, false, None)
                .await
        {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                fallback_agent_id = %binding.agent_id,
                workspace_path = %binding.workspace,
                "PC project will run on another online node using that node's recorded workspace"
            );
            return Some(binding);
        }
    }
    None
}

async fn wait_for_bound_pc_agent_reconnect(
    state: &Arc<AppState>,
    agent_id: &str,
    tx: Option<&UnboundedSender<String>>,
) -> bool {
    send_optional_progress(
        tx,
        "绑定的 PC 节点正在重连，最长等待 2 分钟让原节点恢复，避免把同一项目错误切到其它电脑。",
    );
    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(BOUND_PC_NODE_RECONNECT_WAIT_SECS);
    loop {
        tokio::time::sleep(Duration::from_millis(BOUND_PC_NODE_RECONNECT_POLL_MS)).await;
        if pc_agent_is_connected(state, agent_id).await {
            send_optional_progress(tx, "绑定的 PC 节点已恢复连接，继续使用原本项目路径执行。");
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
    }
}

async fn usable_project_binding_for_agent(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    agent_id: &str,
    is_bound_agent: bool,
    tx: Option<&UnboundedSender<String>>,
) -> Option<PcProjectBinding> {
    let recorded =
        match state
            .store
            .get_project_pc_workspace_binding(user_id, &project.id, agent_id)
        {
            Ok(binding) => binding,
            Err(error) => {
                warn!(
                    project_id = %project.id,
                    user_id = %user_id,
                    agent_id = %agent_id,
                    error = %error,
                    "failed to read node-specific PC workspace binding"
                );
                None
            }
        };

    let workspace = recorded
        .as_ref()
        .map(|binding| binding.workspace_path.as_str())
        .or_else(|| {
            if project.node_id.as_deref() == Some(agent_id) {
                project.workspace_path.as_deref()
            } else {
                None
            }
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    let bound_agent_progress_name = if is_bound_agent {
        Some(pc_node_progress_name(state.as_ref(), agent_id).await)
    } else {
        None
    };
    if let Some(name) = bound_agent_progress_name.as_deref() {
        send_optional_progress(
            tx,
            &format!(
                "正在快速检查绑定 PC 节点 {name} 的项目目录；若巡检未及时返回，将继续直连该节点。"
            ),
        );
    }

    match inspect_pc_agent_workspace(state, agent_id, workspace).await {
        Ok(status) if pc_workspace_inspect_usable(&status) => Some(PcProjectBinding {
            agent_id: agent_id.to_string(),
            workspace: workspace.to_string(),
        }),
        Ok(status) => {
            let problem = pc_workspace_inspect_problem(&status);
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                workspace_path = %workspace,
                problem = %problem,
                "PC project workspace binding is not usable"
            );
            if is_bound_agent {
                let message = bound_agent_progress_name
                    .as_deref()
                    .map(|name| {
                        format!("绑定的 PC 节点 {name} 工作区不可用，正在查找其它在线 PC 节点。")
                    })
                    .unwrap_or_else(|| {
                        "绑定的 PC 节点工作区不可用，正在查找其它在线 PC 节点。".to_string()
                    });
                send_optional_progress(tx, &message);
            }
            None
        }
        Err(error) => {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                workspace_path = %workspace,
                error = %error,
                "could not inspect PC project workspace binding"
            );
            if is_bound_agent {
                if pc_workspace_inspect_error_allows_bound_dispatch(&error) {
                    let message = bound_agent_progress_name
                        .as_deref()
                        .map(|name| {
                            format!(
                                "绑定的 PC 节点 {name} 工作区检查未及时返回，已跳过巡检并继续直连，避免自动切换到其它电脑。"
                            )
                        })
                        .unwrap_or_else(|| {
                            "绑定的 PC 节点工作区检查未及时返回，已跳过巡检并继续直连，避免自动切换到其它电脑。".to_string()
                        });
                    send_optional_progress(tx, &message);
                    return Some(PcProjectBinding {
                        agent_id: agent_id.to_string(),
                        workspace: workspace.to_string(),
                    });
                }
                let message = bound_agent_progress_name
                    .as_deref()
                    .map(|name| {
                        format!(
                            "绑定的 PC 节点 {name} 暂时无法确认工作区状态，正在查找其它在线 PC 节点。"
                        )
                    })
                    .unwrap_or_else(|| {
                        "绑定的 PC 节点暂时无法确认工作区状态，正在查找其它在线 PC 节点。".to_string()
                    });
                send_optional_progress(tx, &message);
            }
            None
        }
    }
}

async fn provision_pc_project_binding(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    agent_id: &str,
    clone_url: Option<String>,
    tx: Option<&UnboundedSender<String>>,
) -> Option<PcProjectBinding> {
    send_optional_progress(
        tx,
        if clone_url.is_some() {
            "当前 PC 节点没有可用项目目录，正在从代码源重建本机工作区。"
        } else {
            "当前 PC 节点没有可用项目目录，正在重新创建本机托管工作区。"
        },
    );
    let template = if project.template.trim().is_empty() {
        "android"
    } else {
        project.template.as_str()
    };
    let provisioned = match project_workspace_provision::provision_project_workspace(
        state,
        agent_id,
        user_id,
        &project.id,
        &project.name,
        template,
        clone_url.as_deref(),
        project.branch.as_deref(),
    )
    .await
    {
        Ok(workspace) => workspace,
        Err(error) => {
            warn!(
                project_id = %project.id,
                user_id = %user_id,
                agent_id = %agent_id,
                error = %error,
                "failed to provision PC project workspace before dispatch"
            );
            return None;
        }
    };

    let local_storage_path = project.storage_repo_path.as_deref().filter(|path| {
        project.storage_repo_url.is_none()
            && project.storage_node_id.as_deref() == Some(agent_id)
            && clone_url.as_deref() == Some(*path)
    });
    let persisted_remote_origin = provisioned
        .git_remote_origin
        .as_deref()
        .filter(|origin| Some(*origin) != local_storage_path)
        .or(project.repo_url.as_deref());

    if let Err(error) = state.store.bind_project_to_pc_workspace(
        user_id,
        &project.id,
        &provisioned.workspace_path,
        agent_id,
        provisioned.git_head.as_deref(),
        persisted_remote_origin,
        provisioned
            .git_branch
            .as_deref()
            .or(project.branch.as_deref()),
    ) {
        warn!(
            project_id = %project.id,
            user_id = %user_id,
            agent_id = %agent_id,
            workspace_path = %provisioned.workspace_path,
            error = %error,
            "failed to persist PC project workspace binding"
        );
        return None;
    }

    Some(PcProjectBinding {
        agent_id: agent_id.to_string(),
        workspace: provisioned.workspace_path,
    })
}

fn clone_url_for_project_access(project: &ProjectAccess, target_agent_id: &str) -> Option<String> {
    if let Some(repo_url) = project
        .repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(repo_url.to_string());
    }
    if project.storage_node_id.as_deref() == Some(target_agent_id) {
        if let Some(path) = project
            .storage_repo_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(path.to_string());
        }
    }
    project
        .storage_repo_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn send_optional_progress(tx: Option<&UnboundedSender<String>>, message: &str) {
    if let Some(tx) = tx {
        let _ = tx.send(WsMessage::progress(message.to_string()).to_json());
    }
}

fn send_pc_workspace_unavailable_error(project: &ProjectAccess, tx: &UnboundedSender<String>) {
    let detail = project
        .workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("未记录目录");
    let msg = format!(
        "当前项目绑定的 PC 工作区不可用，且没有可自动迁移的 Git/硬盘代码源。原目录：{detail}。请先让原 PC 节点上线，或重新创建项目后再发送开发需求。"
    );
    warn!(project_id = %project.id, "{}", msg);
    let _ = tx.send(WsMessage::error(msg).to_json());
}

fn pc_agent_belongs_to_user(state: &Arc<AppState>, user_id: &str, agent_id: &str) -> bool {
    match state.store.get_node_credential_owner(agent_id) {
        Ok(Some(owner)) if owner == user_id => true,
        Ok(Some(owner)) => {
            warn!(
                agent_id = %agent_id,
                owner_user_id = %owner,
                request_user_id = %user_id,
                "refusing PC node owned by another user"
            );
            false
        }
        Ok(None) => {
            warn!(agent_id = %agent_id, "refusing PC node without credential owner");
            false
        }
        Err(error) => {
            warn!(
                agent_id = %agent_id,
                error = %error,
                "failed to query PC node owner; refusing node"
            );
            false
        }
    }
}

async fn pc_agent_is_connected(state: &Arc<AppState>, agent_id: &str) -> bool {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .any(|agent| agent.agent_id == agent_id)
}

async fn connected_pc_agent_for_user(state: &Arc<AppState>, user_id: &str) -> Option<String> {
    for agent in state.agent_manager.list().await {
        if pc_agent_belongs_to_user(state, user_id, &agent.agent_id) {
            return Some(agent.agent_id);
        }
    }
    None
}

async fn connected_pc_project_agent_for_user(
    state: &Arc<AppState>,
    user_id: &str,
) -> Option<String> {
    for agent in state.agent_manager.list().await {
        if !pc_agent_belongs_to_user(state, user_id, &agent.agent_id) {
            continue;
        }
        if project_workspace_provision::resolve_pc_project_node(
            state,
            user_id,
            Some(&agent.agent_id),
        )
        .await
        .is_ok()
        {
            return Some(agent.agent_id);
        }
    }
    None
}

async fn connected_pc_agent_with_existing_workspace(
    state: &Arc<AppState>,
    user_id: &str,
    workspace: &str,
    skip_agent_id: Option<&str>,
) -> Option<String> {
    let skip_agent_id = skip_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    for agent in state.agent_manager.list().await {
        if skip_agent_id == Some(agent.agent_id.as_str()) {
            continue;
        }
        if !pc_agent_belongs_to_user(state, user_id, &agent.agent_id) {
            continue;
        }
        match inspect_pc_agent_workspace(state, &agent.agent_id, workspace).await {
            Ok(status) if pc_workspace_inspect_usable(&status) => return Some(agent.agent_id),
            Ok(status) => {
                warn!(
                    agent_id = %agent.agent_id,
                    workspace_path = %workspace,
                    problem = %pc_workspace_inspect_problem(&status),
                    "online PC node does not have a usable matching workspace"
                );
            }
            Err(error) => {
                warn!(
                    agent_id = %agent.agent_id,
                    workspace_path = %workspace,
                    error = %error,
                    "failed to inspect matching workspace on online PC node"
                );
            }
        }
    }
    None
}

async fn inspect_pc_agent_workspace(
    state: &Arc<AppState>,
    agent_id: &str,
    workspace: &str,
) -> std::result::Result<ProjectWorkspaceInspectStatus, String> {
    match state
        .agent_manager
        .dispatch_project_workspace_inspect(agent_id, workspace.to_string())
        .await
    {
        Ok(AgentToServer::ProjectWorkspaceInspected { status, .. }) => Ok(status),
        Ok(AgentToServer::ProjectWorkspaceInspectError { message, .. }) => Err(message),
        Ok(other) => Err(format!("unexpected inspect response: {other:?}")),
        Err(error) => Err(error.to_string()),
    }
}

fn pc_workspace_inspect_usable(status: &ProjectWorkspaceInspectStatus) -> bool {
    status.path_exists && status.is_dir && (status.codex_available || status.copilot_available)
}

fn pc_workspace_inspect_problem(status: &ProjectWorkspaceInspectStatus) -> &'static str {
    if !status.path_exists {
        "workspace_path_missing"
    } else if !status.is_dir {
        "workspace_path_not_directory"
    } else if !status.codex_available && !status.copilot_available {
        "cli_unavailable"
    } else {
        "unknown"
    }
}

fn pc_workspace_inspect_error_allows_bound_dispatch(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("timeout") || lower.contains("timed out") || lower.contains("超时")
}

/// Codex 额度耗尽或认证失效时返回 true，此时可自动切换到 Copilot
fn is_codex_fallback_error(error: &str) -> bool {
    use crate::errors::{classify_ai_error, AiErrorCategory};
    let classified = classify_ai_error(error);
    matches!(
        classified.category,
        AiErrorCategory::Quota | AiErrorCategory::AuthConfig
    )
}

/// 检查指定 PC 节点上某个 CLI 是否可用
async fn node_cli_available(state: &Arc<AppState>, agent_id: &str, cli_name: &str) -> bool {
    state
        .agent_manager
        .list()
        .await
        .into_iter()
        .find(|a| a.agent_id == agent_id)
        .map(|a| {
            a.allowed_clis
                .iter()
                .any(|c| c.eq_ignore_ascii_case(cli_name))
        })
        .unwrap_or(false)
}

fn append_project_dev_profile_context(
    state: &Arc<AppState>,
    user_id: &str,
    project: &ProjectAccess,
    user_message: &str,
) -> String {
    let profile = match state
        .store
        .get_project_dev_profile_for_user(user_id, &project.id)
    {
        Ok(Some(profile)) if !profile.is_empty() => profile,
        Ok(_) => return user_message.to_string(),
        Err(error) => {
            warn!(
                project_id = %project.id,
                "读取项目开发命令 profile 失败，继续使用原始用户消息: {error}"
            );
            return user_message.to_string();
        }
    };
    format!(
        "{user_message}\n\n{}",
        project_dev_profile_prompt_block(&profile)
    )
}

fn project_dev_profile_prompt_block(profile: &ProjectDevProfile) -> String {
    let mut lines = vec![
        "系统自动识别的本地项目开发命令；执行 run/test/build 时优先参考，除非仓库文档给出更明确命令。".to_string(),
        "<project_dev_profile>".to_string(),
    ];
    push_profile_line(&mut lines, "project_type", profile.project_type.as_deref());
    push_profile_line(
        &mut lines,
        "package_manager",
        profile.package_manager.as_deref(),
    );
    push_profile_line(&mut lines, "run_command", profile.run_command.as_deref());
    push_profile_line(&mut lines, "test_command", profile.test_command.as_deref());
    push_profile_line(
        &mut lines,
        "build_command",
        profile.build_command.as_deref(),
    );
    if !profile.detected_files.is_empty() {
        lines.push(format!(
            "detected_files: {}",
            profile.detected_files.join(", ")
        ));
    }
    lines.push("</project_dev_profile>".to_string());
    lines.join("\n")
}

fn push_profile_line(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("{key}: {value}"));
    }
}

async fn run_dispatch_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    native_session_scope: Option<ai_cli::NativeSessionScope>,
    user_message: &str,
    preflight_note: Option<&str>,
    agent_name: Option<&str>,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    trace_id: Option<&str>,
    require_existing_git: bool,
    force_code_route: bool,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let resume_command = is_short_resume_command(user_message, workspace);
    let delivery_request = is_project_delivery_request(user_message, workspace);
    if delivery_request && !resume_command {
        if tools::find_latest_apk(workspace).is_some() {
            let apk_url = tools::stable_apk_url(download_base);
            let _ = tx.send(
                WsMessage::Done {
                    message: "我看了当前项目状态，APK 已经生成了。你现在最需要的是下载安装测试，所以我先把下载链接给你。".into(),
                    apk_url: Some(apk_url),
                    image_url: None,
                    model_used: None,
                    node_id: None,
                }
                .to_json(),
            );
            return Ok(());
        }
    }

    let mut decision = intent_router::classify(user_message);
    if resume_command || delivery_request || is_short_build_command(user_message, workspace) {
        decision = RoutingDecision {
            intent: intent_router::UserIntent::AppDevelopment,
            route: CapabilityRoute::CodeAgent,
            confidence: 88,
            needs_image_generation: false,
            needs_code_change: true,
            allow_user_agent_preference: true,
            reason: "project_resume_command",
        };
    }
    if force_code_route && decision.route == CapabilityRoute::ChatAgent {
        decision = RoutingDecision {
            intent: intent_router::UserIntent::AppDevelopment,
            route: CapabilityRoute::CodeAgent,
            confidence: 76,
            needs_image_generation: false,
            needs_code_change: true,
            allow_user_agent_preference: true,
            reason: "project_direct_codex_mode",
        };
    }
    info!("intent routing decision: {:?}", decision);

    let codex_cli_only = state.ai_cli.codex_cli_only;
    let requested_agent_name = requested_agent_for_runtime_route(agent_name, pc_runtime_route);
    let user_byok_api_override = codex_cli_only
        && crate::user_agent_secrets::user_byok_api_enabled()
        && UserAgentConfig::load(user_config_workspace)
            .as_ref()
            .map(|cfg| cfg.has_direct_custom_api())
            .unwrap_or(false);
    // ImageThenCode: 有文生图模型时走两步管道（先生图，再集成到代码）；否则降级为 CodeAgent
    let is_image_then_code =
        matches!(decision.route, CapabilityRoute::ImageThenCode) && state.image_model.is_some();
    let image_cli_only = !is_image_then_code
        && (matches!(
            decision.intent,
            intent_router::UserIntent::TextToImage | intent_router::UserIntent::ImageAssetForApp
        ) || matches!(
            decision.route,
            CapabilityRoute::TextToImage | CapabilityRoute::ImageThenCode
        ));
    let backend_route = if codex_cli_only && !user_byok_api_override {
        if !state.ai_cli.enabled {
            return Err(anyhow::anyhow!(
                "当前已锁定只使用 Codex CLI，但服务端没有可用的 Codex CLI 选项"
            ));
        }
        if requested_agent_name
            .map(|name| !is_local_cli_option(state, name))
            .unwrap_or(false)
        {
            let _ = tx.send(
                WsMessage::progress("当前已锁定使用 Codex CLI，不切换到其他 AI 代理。").to_json(),
            );
        }
        decision.route
    } else if image_cli_only {
        if !state.ai_cli.enabled {
            return Err(anyhow::anyhow!(
                "图片处理测试模式仅使用 Codex CLI，但本地 AI CLI 未启用"
            ));
        }
        let _ = tx.send(
            WsMessage::progress("图片处理已切换为 Codex CLI，不调用独立图片模型。").to_json(),
        );
        CapabilityRoute::CodeAgent
    } else {
        decision.route
    };
    let backend_agent_name = if codex_cli_only && !user_byok_api_override {
        requested_agent_name.filter(|name| is_local_cli_option(state, name))
    } else if state.ai_cli.enabled {
        match requested_agent_name {
            Some(name) if is_local_cli_option(state, name) => requested_agent_name,
            Some(_)
                if matches!(
                    pc_runtime_route,
                    Some(
                        PcRuntimeRoutePreference::RouteB
                            | PcRuntimeRoutePreference::RouteC
                            | PcRuntimeRoutePreference::RouteC2
                    )
                ) =>
            {
                requested_agent_name
            }
            _ => None,
        }
    } else if image_cli_only {
        match requested_agent_name {
            Some(name) if is_local_cli_option(state, name) => requested_agent_name,
            _ => Some("codex_cli"),
        }
    } else {
        requested_agent_name
    };

    // ImageThenCode 两步管道：先文生图，把 URL 注入消息，再走代码 Agent 集成
    if is_image_then_code {
        let image_model = state.image_model.as_ref().map(|cfg| cfg.model.clone());
        let mut image_billing_call = if let Some(model) = image_model.as_deref() {
            let key = crate::billing_lifecycle::new_compute_call_id("image_then_code");
            Some(
                crate::compute_usage::reserve_image_generation(
                    &state.store,
                    user_id,
                    &key,
                    "image_then_code",
                    model,
                    user_message,
                )
                .map_err(|msg| anyhow::anyhow!(msg))?,
            )
        } else {
            crate::billing::check_can_call(&state.store, user_id)
                .map_err(|msg| anyhow::anyhow!(msg))?;
            None
        };
        let _ = tx.send(WsMessage::progress("正在生成图片资源...").to_json());
        match crate::image_generation::generate_text_to_image(state, user_message).await {
            Ok(img) => {
                if let (Some(model), Some(billing_call)) =
                    (image_model.as_deref(), image_billing_call.as_mut())
                {
                    crate::compute_usage::record_image_generation_with_key(
                        &state.store,
                        user_id,
                        "image_then_code",
                        model,
                        user_message,
                        Some(billing_call.key()),
                    );
                    billing_call.mark_settled();
                }
                let injected_message = format!(
                    "{}\n\n[已生成图片: {}]\n请将上方图片 URL 下载后集成到项目中作为所需的图片资源。",
                    user_message, img.url
                );
                let _ = tx.send(WsMessage::progress("图片生成完成，正在集成到代码...").to_json());
                return run_backend_with_workspace(
                    user_id,
                    workspace,
                    user_config_workspace,
                    download_base,
                    native_session_scope,
                    &injected_message,
                    preflight_note,
                    backend_agent_name,
                    trace_id,
                    CapabilityRoute::CodeAgent,
                    true,
                    require_existing_git,
                    false, // planning_mode: 图片集成走执行路径，不进入规划
                    state,
                    tx,
                )
                .await;
            }
            Err(e) => {
                warn!("文生图失败，降级到纯代码路径: {}", e);
                let _ = tx.send(
                    WsMessage::progress(format!("图片生成失败（{}），将尝试用代码实现。", e))
                        .to_json(),
                );
            }
        }
    }

    run_backend_with_workspace(
        user_id,
        workspace,
        user_config_workspace,
        download_base,
        native_session_scope,
        user_message,
        preflight_note,
        backend_agent_name,
        trace_id,
        backend_route,
        !(codex_cli_only || image_cli_only),
        require_existing_git,
        false, // planning_mode: dispatch 走执行路径，规划走 plan_for_project_in_workspace
        state,
        tx,
    )
    .await
}

async fn run_backend_with_workspace(
    user_id: &str,
    workspace: &Path,
    user_config_workspace: &Path,
    download_base: &str,
    native_session_scope: Option<ai_cli::NativeSessionScope>,
    user_message: &str,
    preflight_note: Option<&str>,
    agent_name: Option<&str>,
    trace_id: Option<&str>,
    route: CapabilityRoute,
    allow_api_fallback: bool,
    require_existing_git: bool,
    planning_mode: bool,
    state: &Arc<AppState>,
    tx: &UnboundedSender<String>,
) -> Result<()> {
    let user_config = UserAgentConfig::load(&user_config_workspace);
    let backend = choose_backend(state, user_config.as_ref(), agent_name, route);
    let source_hygiene_note = if route == CapabilityRoute::ChatAgent {
        None
    } else {
        source_hygiene::source_size_preflight_note(workspace)
    };
    let base_preflight_note =
        combine_preflight_notes(preflight_note, source_hygiene_note.as_deref());
    let context_compiler_note = if route == CapabilityRoute::ChatAgent {
        None
    } else {
        context_compiler::compile_preflight_note(state, workspace, user_id, user_message, trace_id)
            .await
    };
    let combined_preflight_note = combine_preflight_notes(
        base_preflight_note.as_deref(),
        context_compiler_note.as_deref(),
    );
    let memory_scope_project_id = native_session_scope
        .as_ref()
        .map(|scope| scope.project_id.as_str());
    let memory_scope_type = memory_scope_project_id.map(|_| MEMORY_SCOPE_PROJECT);

    match backend {
        AiBackend::LocalCli => {
            let preferred_local_agent = agent_name.or_else(|| {
                user_config
                    .as_ref()
                    .and_then(|cfg| cfg.use_agent.as_deref())
                    .filter(|name| is_local_cli_option(state, name))
            });
            let primary_option_owned = resolve_cli_option_id(state, preferred_local_agent);
            let primary_option = primary_option_owned.as_deref();
            let cli_result = if planning_mode {
                ai_cli::run_plan_with_workspace(
                    user_id,
                    workspace,
                    download_base,
                    user_message,
                    combined_preflight_note.as_deref(),
                    primary_option,
                    native_session_scope.clone(),
                    trace_id,
                    state,
                    tx,
                )
                .await
            } else {
                ai_cli::run_with_workspace(
                    user_id,
                    workspace,
                    download_base,
                    user_message,
                    combined_preflight_note.as_deref(),
                    primary_option,
                    route,
                    require_existing_git,
                    native_session_scope.clone(),
                    trace_id,
                    state,
                    tx,
                )
                .await
            };

            match cli_result {
                Ok(()) => Ok(()),
                Err(e) if state.ai_cli.fallback_cli_option.is_some() => {
                    let fallback_id = state.ai_cli.fallback_cli_option.clone().unwrap();
                    // 主 CLI 与备用 CLI 不同时才回退，避免无效重试
                    let primary_resolved = state
                        .ai_cli
                        .find_option(primary_option)
                        .map(|opt| opt.id.as_str())
                        .unwrap_or("");
                    if primary_resolved.eq_ignore_ascii_case(&fallback_id) {
                        return Err(e);
                    }
                    warn!(
                        "主 CLI 执行失败（{}），正在切换备用 CLI {}: {}",
                        primary_resolved, fallback_id, e
                    );
                    let _ = tx.send(
                        WsMessage::progress(format!(
                            "主 AI CLI 暂不可用，正在切换备用 CLI ({fallback_id})…"
                        ))
                        .to_json(),
                    );
                    let fallback_result = if planning_mode {
                        ai_cli::run_plan_with_workspace(
                            user_id,
                            workspace,
                            download_base,
                            user_message,
                            combined_preflight_note.as_deref(),
                            Some(fallback_id.as_str()),
                            native_session_scope.clone(),
                            trace_id,
                            state,
                            tx,
                        )
                        .await
                    } else {
                        ai_cli::run_with_workspace(
                            user_id,
                            workspace,
                            download_base,
                            user_message,
                            combined_preflight_note.as_deref(),
                            Some(fallback_id.as_str()),
                            route,
                            require_existing_git,
                            native_session_scope.clone(),
                            trace_id,
                            state,
                            tx,
                        )
                        .await
                    };
                    match fallback_result {
                        Ok(()) => Ok(()),
                        Err(fallback_e)
                            if allow_api_fallback
                                && state.ai_cli.fallback_to_api
                                && has_api_agents(state).await =>
                        {
                            warn!("备用 CLI 也失败，回退到 API 代理: {}", fallback_e);
                            let _ = tx.send(
                                WsMessage::progress(format!(
                                    "本地 AI CLI 均不可用，正在切换 API 代理: {}",
                                    fallback_e
                                ))
                                .to_json(),
                            );
                            run_api_inner_with_workspace(
                                user_id,
                                workspace,
                                user_config_workspace,
                                download_base,
                                user_message,
                                combined_preflight_note.as_deref(),
                                trace_id,
                                api_agent_name(state, agent_name),
                                planning_mode,
                                memory_scope_type,
                                memory_scope_project_id,
                                state,
                                tx,
                            )
                            .await
                        }
                        Err(fallback_e) => Err(fallback_e),
                    }
                }
                Err(e)
                    if allow_api_fallback
                        && state.ai_cli.fallback_to_api
                        && has_api_agents(state).await =>
                {
                    warn!("本地 AI CLI 执行失败，回退到 API 代理: {}", e);
                    let _ = tx.send(
                        WsMessage::progress(format!(
                            "本地 AI CLI 暂不可用，正在切换原 API 代理: {}",
                            e
                        ))
                        .to_json(),
                    );
                    run_api_inner_with_workspace(
                        user_id,
                        workspace,
                        user_config_workspace,
                        download_base,
                        user_message,
                        combined_preflight_note.as_deref(),
                        trace_id,
                        api_agent_name(state, agent_name),
                        planning_mode,
                        memory_scope_type,
                        memory_scope_project_id,
                        state,
                        tx,
                    )
                    .await
                }
                Err(e) => Err(e),
            }
        }
        AiBackend::Api => {
            run_api_inner_with_workspace(
                user_id,
                workspace,
                user_config_workspace,
                download_base,
                user_message,
                combined_preflight_note.as_deref(),
                trace_id,
                api_agent_name(state, agent_name),
                planning_mode,
                memory_scope_type,
                memory_scope_project_id,
                state,
                tx,
            )
            .await
        }
    }
}

fn combine_preflight_notes(git_note: Option<&str>, source_note: Option<&str>) -> Option<String> {
    match (git_note, source_note) {
        (Some(git), Some(source)) => Some(format!("{git}\n\n{source}")),
        (Some(git), None) => Some(git.to_string()),
        (None, Some(source)) => Some(source.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        pc_cli_chat_requested, pc_cli_chat_route_label,
        pc_workspace_inspect_error_allows_bound_dispatch, pc_workspace_inspect_problem,
        pc_workspace_inspect_usable, project_chat_should_use_pc_cli,
        project_fields_require_pc_workspace, requires_project_workflow_for_message,
        should_attempt_pc_apk_sync, BOUND_PC_NODE_RECONNECT_WAIT_SECS,
    };
    use crate::pc_agent_runtime_choice::PcRuntimeRoutePreference;
    use crate::store::ProjectAccess;
    use homecli_proto::ProjectWorkspaceInspectStatus;
    use std::path::Path;

    #[test]
    fn casual_greeting_does_not_require_project_workflow() {
        assert!(!requires_project_workflow_for_message(
            "你好",
            Path::new("C:/tmp/project")
        ));
        assert!(!requires_project_workflow_for_message(
            "你好吗？",
            Path::new("C:/tmp/project")
        ));
    }

    #[test]
    fn app_change_requires_project_workflow() {
        assert!(requires_project_workflow_for_message(
            "帮我在首页加一个按钮",
            Path::new("C:/tmp/project")
        ));
    }

    #[test]
    fn open_idea_stays_in_chat_route() {
        assert!(!requires_project_workflow_for_message(
            "我有一个想法",
            Path::new("C:/tmp/project")
        ));
    }

    #[test]
    fn completed_project_install_question_stays_in_chat_route() {
        assert!(!requires_project_workflow_for_message(
            "完成的项目我在哪里下载安装呢",
            Path::new("C:/tmp/project__demo")
        ));
    }

    #[test]
    fn explicit_pc_cli_routes_are_cli_first_for_chat() {
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteA
        )));
        assert!(pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC3
        )));
        assert!(!pc_cli_chat_requested(Some(
            PcRuntimeRoutePreference::RouteC
        )));
        assert!(!pc_cli_chat_requested(None));

        assert!(project_chat_should_use_pc_cli(None, None, false));
        assert!(project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteA),
            None,
            false
        ));
        assert!(project_chat_should_use_pc_cli(None, Some("codex"), true));
        assert!(!project_chat_should_use_pc_cli(
            Some(PcRuntimeRoutePreference::RouteC),
            None,
            false
        ));
        assert!(!project_chat_should_use_pc_cli(None, Some("api"), false));
    }

    #[test]
    fn pc_cli_chat_labels_match_user_selected_route() {
        assert_eq!(
            pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteA)),
            "本机 AI"
        );
        assert_eq!(
            pc_cli_chat_route_label(Some(PcRuntimeRoutePreference::RouteC3)),
            "远程 Codex"
        );
        assert_eq!(pc_cli_chat_route_label(None), "本机 AI");
    }

    #[test]
    fn android_template_pc_project_attempts_apk_sync_for_ui_changes() {
        let project = ProjectAccess {
            id: "prj_android".into(),
            name: "大大泡泡".into(),
            workspace_key: "prj_android".into(),
            template: "android".into(),
            source_type: "pc_managed".into(),
            repo_url: None,
            branch: None,
            workspace_path: Some(r"C:\Users\Administrator\Elon\workspaces\prj\repo".into()),
            node_id: Some("node-local".into()),
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            role: "owner".into(),
            status: "active".into(),
            runtime_permission: "workspace_write".into(),
        };

        assert!(should_attempt_pc_apk_sync(&project, "把按钮改成绿色"));
    }

    #[test]
    fn pc_managed_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "pc_managed",
            None,
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn bound_node_projects_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            Some("node-local"),
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn windows_local_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"D:\rust\active-projects\elon cli")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("D:/rust/active-projects/elon cli")
        ));
    }

    #[test]
    fn unc_paths_require_pc_workspace_route() {
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some(r"\\workstation\repos\elon")
        ));
        assert!(project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("//workstation/repos/elon")
        ));
    }

    #[test]
    fn server_local_paths_can_still_use_server_git_route() {
        assert!(!project_fields_require_pc_workspace(
            "local_path",
            None,
            Some("/srv/elon/project")
        ));
    }

    #[test]
    fn pc_workspace_inspect_requires_existing_dir_and_cli() {
        let mut status = inspect_status();
        assert!(pc_workspace_inspect_usable(&status));

        status.path_exists = false;
        assert!(!pc_workspace_inspect_usable(&status));
        assert_eq!(
            pc_workspace_inspect_problem(&status),
            "workspace_path_missing"
        );

        status = inspect_status();
        status.codex_available = false;
        status.copilot_available = false;
        assert!(!pc_workspace_inspect_usable(&status));
        assert_eq!(pc_workspace_inspect_problem(&status), "cli_unavailable");
    }

    #[test]
    fn pc_workspace_inspect_timeout_keeps_bound_node() {
        assert!(pc_workspace_inspect_error_allows_bound_dispatch(
            "project workspace inspect timeout (3s)"
        ));
        assert!(pc_workspace_inspect_error_allows_bound_dispatch(
            "PC 节点创建项目工作区超时（30 秒）"
        ));
        assert!(!pc_workspace_inspect_error_allows_bound_dispatch(
            "workspace path does not exist"
        ));
    }

    #[test]
    fn bound_pc_node_reconnect_window_covers_server_restart() {
        assert!(BOUND_PC_NODE_RECONNECT_WAIT_SECS >= 90);
    }

    fn inspect_status() -> ProjectWorkspaceInspectStatus {
        ProjectWorkspaceInspectStatus {
            workspace_path: r"D:\rust\active-projects\elon cli".to_string(),
            path_exists: true,
            is_dir: true,
            is_git_worktree: true,
            git_branch: Some("main".to_string()),
            git_head: Some("2580208".to_string()),
            git_remote_origin: Some("git@github.com:ElonQian1/Elon.git".to_string()),
            has_uncommitted_changes: false,
            uncommitted_count: Some(0),
            disk_free_bytes: Some(10 * 1024 * 1024 * 1024),
            codex_available: true,
            copilot_available: false,
        }
    }
}
