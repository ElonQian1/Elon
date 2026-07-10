use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    agent, agent_intent,
    agent_routing::is_local_cli_option,
    ai_cli, intent_router,
    pc_agent_runtime_choice::PcRuntimeRoutePreference,
    project_attachment_notes::{
        append_project_attachment_notes, append_project_cli_attachment_artifacts,
    },
    project_auth::can_edit,
    project_chat_executor::run_project_agent_in_execution_workspace,
    project_chat_pc_node::{
        acquire_pc_node_cli_permit, chat_billing_block, pc_node_cli_execution_progress_message,
        pc_node_fast_path_route, record_pc_node_cli_execution_granted, run_bill,
        should_auto_bind_local_node,
    },
    project_chat_reply::{append_nonempty_ws_text, chat_reply_after_intent_gate},
    project_conversation_workspace::{
        prepare_project_conversation_workspace, project_conversation_execution_key,
        project_shared_execution_key, ProjectConversationWorkspace,
    },
    project_execution_mode::ProjectExecutionMode,
    project_keys::{clean_trace_id, codex_prewarm_key},
    project_trace_events::record_server_message,
    project_workspace_recovery,
    project_ws_protocol::{ProjectAttachmentRef, ProjectChatRequest},
    store::{ProjectAccess, MEMORY_SCOPE_PROJECT},
    tools,
    types::{AppState, WsMessage},
};

use crate::project_chat_helpers::*;
// 向外部调用方透明重新导出
pub(crate) use crate::project_chat_helpers::looks_like_replaced_unicode_mojibake;

pub(crate) async fn run_project_agent_with_scheduler(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    download_base: String,
    conversation_id: String,
    message: String,
    project_icon_data_url: Option<String>,
    agent_name: Option<String>,
    attachments: Option<Vec<ProjectAttachmentRef>>,
    execution_mode: ProjectExecutionMode,
    pc_runtime_route: Option<PcRuntimeRoutePreference>,
    direct_pc_cli: bool,
    project_preflight_note: Option<String>,
    trace_id: Option<String>,
    tx: UnboundedSender<String>,
) {
    let agent = agent_name.as_deref();
    if let Some(msg) = run_bill(
        &state,
        &user_id,
        &project,
        agent,
        pc_runtime_route,
        direct_pc_cli,
    ) {
        let _ = tx.send(WsMessage::error(msg).to_json());
        return;
    }
    let project_icon_data_url = project_icon_data_url.or_else(|| {
        state
            .store
            .project_space_summary(&user_id, &project.id)
            .ok()
            .and_then(|project| project.icon_data_url)
    });

    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_workflow_start",
            serde_json::json!({
                "project_id": &project.id,
                "user_id": &user_id,
                "conversation_id": &conversation_id,
                "message_chars": message.chars().count(),
                "has_project_icon": project_icon_data_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
                "agent": agent_name.as_deref(),
                "execution_mode": execution_mode.as_str(),
                "pc_runtime_route": pc_runtime_route.map(|route| route.as_request_value()),
            }),
        );
    }
    let routing_decision = intent_router::classify(&message);
    let base_workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    // PC 节点项目（有 node_id）的路径在用户 PC 上，不在服务器本地。
    let is_pc_node_project = project
        .node_id
        .as_deref()
        .map(|n| !n.is_empty())
        .unwrap_or(false);
    let direct_pc_cli_enabled = direct_pc_cli && is_pc_node_project && !execution_mode.is_plan();
    let lightweight_chat_split_enabled = ai_cli::project_lightweight_chat_split_enabled();
    // force_cli: 悬浮球手机控制专用模式，绕过本地 intent_router 分流，
    // 直接进入 Codex CLI 意图门控，由 Codex 自己判断"闲聊还是生成脚本"。
    let needs_project_workflow = project_preflight_note.is_some()
        || execution_mode.is_plan()
        || execution_mode.is_force_cli()
        || routing_decision.route != intent_router::CapabilityRoute::ChatAgent;
    let use_pc_node_fast_path = should_use_pc_node_fast_path(
        is_pc_node_project,
        needs_project_workflow,
        direct_pc_cli_enabled,
        pc_runtime_route,
    );
    if needs_project_workflow && !can_edit(&project.role) {
        let apk_url = if agent_intent::is_project_delivery_request(&message, &base_workspace)
            && tools::find_latest_apk(&base_workspace).is_some()
        {
            Some(tools::stable_apk_url(&download_base))
        } else {
            None
        };
        let message = if apk_url.is_some() {
            "当前项目已有可下载 APK。你是只读成员，可以下载体验，但不能发起修改代码、编译或发布。"
                .to_string()
        } else {
            "你当前是只读成员，可以在项目频道里询问 AI、查看讨论和结果，但不能发起修改代码、编译或发布。请联系项目 owner 获取协作权限。".to_string()
        };
        let _ = tx.send(
            WsMessage::Done {
                message,
                apk_url,
                image_url: None,
                model_used: None,
                node_id: None,
            }
            .to_json(),
        );
        return;
    }
    // Phase 2 优化：本地分类置信度 >= 84 的明确代码任务跳过 codex 意图门控。
    // force_cli 模式（悬浮球手机控制）强制走意图门控，让 Codex 自己判断是闲聊还是生成脚本，
    // 不能跳过（否则 Codex 不读 AGENTS.md，无法生成手机控制 JSON）。
    let skip_intent_gate = needs_project_workflow
        && !execution_mode.is_force_cli()
        && (!lightweight_chat_split_enabled || routing_decision.confidence >= 84);
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_intent_classified",
            serde_json::json!({
                "needs_project_workflow": needs_project_workflow,
                "local_confidence": routing_decision.confidence,
                "local_reason": routing_decision.reason,
                "skip_intent_gate": skip_intent_gate,
                "direct_pc_cli": direct_pc_cli_enabled,
                "lightweight_chat_split_enabled": lightweight_chat_split_enabled,
                "execution_mode": execution_mode.as_str(),
            }),
        );
    }
    // 服务器上不应创建 worktree——直接透传给 agent 层，由 pc_project_binding 接管。
    // 同时 bypass 整个 scheduler（PC项目无需 worktree/合并锁），减少不必要的等待。
    if use_pc_node_fast_path {
        // PC 节点项目快速路径：服务器不创建 worktree，但仍按会话串行，避免同一
        // conversation 的多个 CLI 进程同时写同一个 PC 会话 worktree。
        if needs_project_workflow {
            let _ = tx.send(
                WsMessage::progress(
                    "PC 节点项目已启用本机会话隔离：代码会在你的 PC 节点上创建/复用会话 worktree 后执行。",
                )
                .to_json(),
            );
        }
        let queued_tx = tx.clone();
        let trace_state = state.clone();
        let queued_trace_id = trace_id.clone();
        let queued_project_id = project.id.clone();
        let queued_conversation_id = conversation_id.clone();
        let conversation_execution_key =
            project_conversation_execution_key(&project.id, &conversation_id);
        let conversation_permit = state
            .project_task_scheduler
            .acquire(&conversation_execution_key, move || {
                if let Some(trace_id) = queued_trace_id.as_deref() {
                    trace_state.server_traces.record(
                        trace_id,
                        "server_pc_conversation_queue_wait",
                        serde_json::json!({
                            "project_id": &queued_project_id,
                            "conversation_id": &queued_conversation_id,
                        }),
                    );
                }
                let _ = queued_tx.send(
                    WsMessage::progress("当前 PC 会话已有任务在运行，本次消息已进入该会话队列；其他会话仍可并行执行。")
                        .to_json(),
                );
            })
            .await;
        if needs_project_workflow {
            let message = if conversation_permit.was_queued() {
                "已轮到本 PC 会话任务，开始交给 PC 节点执行。"
            } else {
                "已获得本 PC 会话执行权，开始交给 PC 节点执行。"
            };
            let _ = tx.send(WsMessage::progress(message).to_json());
        }
        let message = if should_append_project_icon_context_for_pc_fast_path(needs_project_workflow)
        {
            append_project_icon_context(
                &state,
                &project,
                &base_workspace,
                message,
                project_icon_data_url.as_deref(),
            )
        } else {
            message
        };
        let pc_node_id = project.node_id.clone().unwrap_or_default();
        let node_cli_permit = acquire_pc_node_cli_permit(
            &state,
            &tx,
            trace_id.as_deref(),
            &project.id,
            &conversation_id,
            &pc_node_id,
        )
        .await;
        let node_was_queued = node_cli_permit.permit.was_queued();
        let node_parallel_limit = node_cli_permit.parallel_limit;
        let node_message =
            pc_node_cli_execution_progress_message(node_was_queued, node_parallel_limit);
        record_pc_node_cli_execution_granted(
            &state,
            trace_id.as_deref(),
            &project.id,
            &conversation_id,
            &pc_node_id,
            node_was_queued,
            node_parallel_limit,
        );
        let _ = tx.send(WsMessage::progress(node_message).to_json());
        let _keep_conversation_permit = conversation_permit;
        let _keep_node_cli_permit = node_cli_permit.permit;
        if execution_mode.is_plan() {
            agent::plan_for_project_in_workspace(
                &user_id,
                &project,
                &base_workspace,
                &download_base,
                Some(&conversation_id),
                &message,
                agent_name.as_deref(),
                pc_node_fast_path_route(pc_runtime_route, direct_pc_cli_enabled),
                trace_id.as_deref(),
                &state,
                tx,
            )
            .await;
            return;
        }
        agent::run_pc_cli_passthrough_for_project(
            &user_id,
            &project,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            pc_node_fast_path_route(pc_runtime_route, direct_pc_cli_enabled),
            project_preflight_note.as_deref(),
            &download_base,
            &state,
            tx,
        )
        .await;
        return;
    }
    let prepared_execution_workspace =
        if needs_project_workflow && !execution_mode.is_plan() && !is_pc_node_project {
            match prepare_project_conversation_workspace(&state, &project, &conversation_id) {
                Ok(workspace) => Some(workspace),
                Err(error) => {
                    let _ = tx.send(
                        WsMessage::error(format!("创建会话 worktree 失败: {}", error)).to_json(),
                    );
                    return;
                }
            }
        } else {
            None
        };
    let workspace = prepared_execution_workspace
        .as_ref()
        .map(|workspace| workspace.active_path())
        .unwrap_or(base_workspace.as_path());
    let workspace_key = workspace.display().to_string();
    let prewarm_agent = if state.ai_cli.codex_cli_only {
        agent_name
            .as_deref()
            .filter(|name| is_local_cli_option(&state, name))
    } else {
        agent_name.as_deref()
    };
    let prewarm_key = codex_prewarm_key(
        &project.id,
        &user_id,
        &conversation_id,
        prewarm_agent,
        &workspace_key,
    );
    state.codex_prewarm.cancel(&prewarm_key).await;
    if !needs_project_workflow {
        agent::run_for_project(
            &user_id,
            &project,
            &download_base,
            Some(&conversation_id),
            &message,
            agent_name.as_deref(),
            pc_runtime_route,
            trace_id.as_deref(),
            &state,
            tx,
        )
        .await;
        return;
    }

    let message = append_project_icon_context(
        &state,
        &project,
        workspace,
        message,
        project_icon_data_url.as_deref(),
    );

    if execution_mode.is_plan() {
        let _ =
            tx.send(WsMessage::progress("已开启先规划模式：本轮只生成计划，不改代码。").to_json());
    } else if skip_intent_gate {
        if let Some(trace_id) = trace_id.as_deref() {
            state.server_traces.record(
                trace_id,
                "server_intent_gate_skipped",
                serde_json::json!({
                    "confidence": routing_decision.confidence,
                    "reason": routing_decision.reason,
                }),
            );
        }
        tracing::info!(
            confidence = routing_decision.confidence,
            reason = routing_decision.reason,
            "Skipped codex intent gate (high local confidence)"
        );
        let _ = tx.send(WsMessage::progress("已识别为开发任务，直接进入项目工作流。").to_json());
    } else {
        let _ = tx.send(WsMessage::progress("正在确认这是否需要进入开发流程。").to_json());
        let native_session_scope = ai_cli::NativeSessionScope {
            project_id: project.id.clone(),
            user_id: user_id.clone(),
            conversation_id: conversation_id.clone(),
            runtime_permission: project.runtime_permission.clone(),
        };
        match ai_cli::confirm_project_intent(
            workspace,
            &message,
            agent_name.as_deref(),
            Some(native_session_scope),
            trace_id.as_deref(),
            &state,
        )
        .await
        {
            Ok(gate) if !gate.should_enter_development() => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_kept_chat",
                        serde_json::json!({
                            "confidence": gate.confidence,
                            "reason": gate.reason,
                        }),
                    );
                }
                tracing::info!(
                    confidence = gate.confidence,
                    reason = %gate.reason,
                    "Codex CLI kept request in lightweight chat"
                );
                let reply = chat_reply_after_intent_gate(&message, gate.chat_reply);
                let _ = tx.send(
                    WsMessage::Done {
                        message: reply,
                        apk_url: None,
                        image_url: None,
                        model_used: None,
                        node_id: None,
                    }
                    .to_json(),
                );
                return;
            }
            Ok(gate) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_enter_development",
                        serde_json::json!({
                            "confidence": gate.confidence,
                            "reason": gate.reason,
                        }),
                    );
                }
                tracing::info!(
                    confidence = gate.confidence,
                    reason = %gate.reason,
                    "Codex CLI confirmed development workflow"
                );
            }
            Err(error) => {
                if let Some(trace_id) = trace_id.as_deref() {
                    state.server_traces.record(
                        trace_id,
                        "server_intent_error",
                        serde_json::json!({
                            "error": error.to_string(),
                        }),
                    );
                }
                let _ = tx
                    .send(WsMessage::error(format!("Codex CLI 意图确认失败: {}", error)).to_json());
                return;
            }
        }
    }

    let _ = tx.send(
        WsMessage::progress("通用项目工作流已启用：服务器会为本会话准备独立 worktree/分支；同一会话串行，编码阶段可跨会话并行，最终合并、版本号和发布仍串行。"
                )
        .to_json(),
    );

    let queued_tx = tx.clone();
    let trace_state = state.clone();
    let queued_trace_id = trace_id.clone();
    let queued_project_id = project.id.clone();
    let queued_conversation_id = conversation_id.clone();
    let conversation_execution_key =
        project_conversation_execution_key(&project.id, &conversation_id);
    let conversation_permit = state
        .project_task_scheduler
        .acquire(&conversation_execution_key, move || {
            if let Some(trace_id) = queued_trace_id.as_deref() {
                trace_state.server_traces.record(
                    trace_id,
                    "server_conversation_queue_wait",
                    serde_json::json!({
                        "project_id": &queued_project_id,
                        "conversation_id": &queued_conversation_id,
                    }),
                );
            }
            let _ = queued_tx.send(
                WsMessage::progress("当前会话已有任务在运行，本次任务已进入该会话队列；其他会话仍可使用独立 worktree 并行开发。"
                        )
                .to_json(),
            );
        })
        .await;

    let execution_workspace = prepared_execution_workspace
        .unwrap_or_else(|| ProjectConversationWorkspace::shared(base_workspace.clone()));

    let shared_project_permit = if execution_mode.is_plan() || execution_workspace.is_isolated() {
        None
    } else {
        let queued_tx = tx.clone();
        let trace_state = state.clone();
        let queued_trace_id = trace_id.clone();
        let queued_project_id = project.id.clone();
        let shared_key = project_shared_execution_key(&project.id);
        Some(
            state
                .project_task_scheduler
                .acquire(&shared_key, move || {
                    if let Some(trace_id) = queued_trace_id.as_deref() {
                        trace_state.server_traces.record(
                            trace_id,
                            "server_project_queue_wait",
                            serde_json::json!({ "project_id": &queued_project_id }),
                        );
                    }
                    let _ = queued_tx.send(
                        WsMessage::progress(
                            "当前项目无法创建独立 worktree，已退回共享工作区串行执行。",
                        )
                        .to_json(),
                    );
                })
                .await,
        )
    };

    let message_text = if execution_mode.is_plan() && conversation_permit.was_queued() {
        "已轮到本会话规划任务，开始生成计划。"
    } else if execution_mode.is_plan() {
        "已获得本会话规划执行权，开始生成计划。"
    } else if conversation_permit.was_queued() {
        "已轮到本会话任务，开始在会话 worktree 中调用 AI 修改项目。"
    } else if execution_workspace.is_isolated() {
        "已获得本会话执行权，开始在独立 worktree 中调用 AI 修改项目。"
    } else {
        "已获得项目执行权，开始在共享工作区中调用 AI 修改项目。"
    };
    if let Some(trace_id) = trace_id.as_deref() {
        state.server_traces.record(
            trace_id,
            "server_conversation_execution_granted",
            serde_json::json!({
                "project_id": &project.id,
                "conversation_id": &conversation_id,
                "was_queued": conversation_permit.was_queued(),
                "workspace": execution_workspace.active_path().display().to_string(),
                "isolated": execution_workspace.is_isolated(),
            }),
        );
    }
    let _ = tx.send(WsMessage::progress(message_text).to_json());

    let _keep_conversation_permit = conversation_permit;
    let _keep_shared_project_permit = shared_project_permit;
    let message = append_project_cli_attachment_artifacts(
        state.as_ref(),
        &project,
        &conversation_id,
        message,
        attachments.as_deref(),
        execution_workspace.active_path(),
    )
    .await;
    run_project_agent_in_execution_workspace(
        state,
        user_id,
        project,
        download_base,
        conversation_id,
        message,
        agent_name,
        execution_mode,
        trace_id,
        execution_workspace,
        tx,
    )
    .await;
}
