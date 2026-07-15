use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info, warn};

use crate::{
    agent_api_loop::run_api_inner_with_workspace,
    agent_intent::{
        has_origin_remote, is_project_delivery_request, is_pure_project_delivery_message,
        is_short_build_command, is_short_resume_command,
    },
    agent_pc_workspace::{
        project_chat_should_use_pc_cli, project_cli_runtime_permission_fallback,
        project_cli_runtime_permission_for_message, project_requires_pc_workspace,
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
    route_a_session_lease::{self, RouteARuntimePrewarmResult},
    source_hygiene,
    store::{ProjectAccess, ProjectDevProfile, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AiBackend, AppState, UserAgentConfig, WsMessage},
};

use super::dispatch::{run_backend_with_workspace, run_dispatch_with_workspace};
use super::pc_binding::{
    append_project_dev_profile_context, inspect_pc_agent_workspace, is_codex_fallback_error,
    node_cli_available, pc_workspace_inspect_problem, pc_workspace_inspect_usable,
    send_pc_workspace_unavailable_error, usable_project_binding_for_agent, PcProjectBinding,
};
use super::pc_node_select::{
    connected_pc_agent_for_route, connected_pc_agent_with_existing_workspace,
    connected_pc_agent_with_recorded_workspace_binding, connected_pc_project_agent_for_route,
};
use super::public_dev::{
    pc_agent_authorized_for_bound_node, pc_agent_authorized_for_route,
    pc_agent_belongs_to_user_quiet, pc_agent_public_dev_enabled_for_consumer,
    pc_agent_runtime_ready_for_route, route_allows_public_dev_node,
};
use super::runtime_binding::{resolve_pc_chat_runtime_binding, resolve_pc_project_runtime_binding};
use super::{
    latest_project_delivery_apk_url, pc_cli_chat_route_label,
    requires_project_workflow_for_message, ProjectWorkflowRouting,
};

pub(super) async fn run_for_project_in_workspace_with_routing(
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
            let chat_runtime = match resolve_pc_chat_runtime_binding(
                state,
                user_id,
                project,
                Some(&tx),
                agent_name,
                pc_runtime_route,
            )
            .await
            {
                Ok(Some(binding)) => binding,
                Ok(None) => {
                    let msg = format!(
                        "项目会话默认交给{route_label}处理，但当前项目还没有绑定可用 PC 节点。请先连接节点，或手动切换到平台 AI。"
                    );
                    warn!("{msg}");
                    let _ = tx.send(WsMessage::error(msg).to_json());
                    return;
                }
                Err(error) => {
                    warn!("PC CLI 轻量聊天不可用，不回退 API: {}", error);
                    let _ = tx.send(WsMessage::error(error).to_json());
                    return;
                }
            };
            let runtime_choice = chat_runtime.runtime_choice;
            let agent_id_owned = chat_runtime.agent_id;
            let agent_id = agent_id_owned.as_str();
            let route_label = runtime_choice.progress_label().to_string();

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
                runtime_permission: project_cli_runtime_permission_for_message(
                    project,
                    user_message,
                ),
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
            runtime_permission: project_cli_runtime_permission_for_message(project, user_message),
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
