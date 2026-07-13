//! Admission lifecycle for one PC CLI run.
//!
//! This module freezes prompt, billing, compute-run and project-execution
//! identity before the node receives a prompt, then waits for the node's
//! durable task handle to be accepted. Streaming and terminal settlement stay
//! in the parent orchestration module.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use super::ai_cli_pc_reply_helpers::pc_dispatch_started_event;
use super::{
    ai_cli_pc_config::{
        native_session_uuid, pc_display_model_label, pc_lightweight_chat_reasoning_effort,
        pc_project_reasoning_effort, pc_route_a_ui_args,
    },
    ai_cli_pc_execution::{
        finish_pc_node_compute_run, pc_route_a_prompt_bootstrapped, record_pc_execution_started,
        record_pc_execution_without_cli_done, start_pc_node_compute_run,
    },
    ai_cli_pc_guards::PcExecutionFinishOnDrop,
    ai_cli_pc_prompt::{
        contextual_passthrough_message, pc_lightweight_chat_prompt, pc_project_execution_prompt,
        pc_project_passthrough_prompt,
    },
    pc_agent_dispatch::{dispatch_pc_cli_prompt_until_accepted, PcCliPromptDispatchRequest},
    pc_billing::{bind_pc_cli_replay_policy, reserve_pc_cli_billing_call, PcCliBillingContext},
    pc_billing_policy::{
        defer_pc_cli_billing_after_acceptance, mark_pc_cli_dispatch_outcome_unknown,
        prepare_pc_cli_billing_for_dispatch,
    },
    pc_cli_model_id, AiCliRequestMode, NativeSessionScope,
};
use crate::{
    billing, billing_lifecycle::TrustedBillingCall, homecli_agent::CliPromptCancelHandle,
    pc_node_display::pc_node_progress_name, types::AppState,
};

pub(super) struct PcRunAdmissionRequest<'a> {
    pub(super) agent_id: &'a str,
    pub(super) user_id: &'a str,
    pub(super) cwd: Option<&'a str>,
    pub(super) user_message: &'a str,
    pub(super) preflight_note: Option<&'a str>,
    pub(super) request_mode: AiCliRequestMode,
    pub(super) native_session_scope: Option<&'a NativeSessionScope>,
    pub(super) cli_name: &'a str,
    pub(super) copilot_model: Option<&'a str>,
    pub(super) codex_reasoning_effort: Option<&'a str>,
    pub(super) model_label: Option<&'a str>,
    pub(super) state: &'a Arc<AppState>,
    pub(super) tx: &'a UnboundedSender<String>,
}

pub(super) struct PcRunAdmission<'a> {
    pub(super) raw_pc_passthrough: bool,
    pub(super) lightweight_pc_chat: bool,
    pub(super) apk_sync_probe_since: Option<u64>,
    pub(super) native_cli_session_uuid: Option<String>,
    pub(super) pc_development_prompt: bool,
    pub(super) pc_req_id: String,
    pub(super) pc_cli_feature: &'static str,
    pub(super) pc_accounting_key: String,
    pub(super) pc_billing_call: TrustedBillingCall<'a>,
    pub(super) pc_billing_context: PcCliBillingContext,
    pub(super) frozen_model_id: String,
    pub(super) display_model: String,
    pub(super) pc_execution_guard: PcExecutionFinishOnDrop,
    pub(super) rx: UnboundedReceiver<AgentToServer>,
    pub(super) cancel_handle: CliPromptCancelHandle,
    pub(super) first_cli_event: Option<AgentToServer>,
    pub(super) node_progress_name: String,
}

pub(super) async fn admit_pc_run<'a>(
    request: PcRunAdmissionRequest<'a>,
) -> Result<PcRunAdmission<'a>> {
    let PcRunAdmissionRequest {
        agent_id,
        user_id,
        cwd,
        user_message,
        preflight_note,
        request_mode,
        native_session_scope,
        cli_name,
        copilot_model,
        codex_reasoning_effort,
        model_label,
        state,
        tx,
    } = request;
    let raw_pc_passthrough = request_mode.is_passthrough();
    let lightweight_pc_chat = !request_mode.is_plan() && cwd.is_none();
    let apk_sync_probe_since = super::pc_apk_probe_since(request_mode, cwd);
    let effective_codex_reasoning_effort = if lightweight_pc_chat {
        pc_lightweight_chat_reasoning_effort(cli_name, codex_reasoning_effort)
    } else {
        pc_project_reasoning_effort(cli_name, codex_reasoning_effort, request_mode)
    };
    let native_cli_session_uuid =
        native_session_scope.map(|scope| native_session_uuid(cli_name, scope));
    let pc_development_prompt =
        !lightweight_pc_chat && !request_mode.is_plan() && !raw_pc_passthrough;
    let pc_prompt_bootstrapped = if pc_development_prompt {
        pc_route_a_prompt_bootstrapped(
            state,
            native_session_scope,
            cli_name,
            agent_id,
            cwd,
            native_cli_session_uuid.as_deref(),
            true,
        )
    } else {
        false
    };
    let prompt = if raw_pc_passthrough {
        pc_project_passthrough_prompt(&contextual_passthrough_message(
            user_message,
            preflight_note,
        ))
    } else if lightweight_pc_chat {
        pc_lightweight_chat_prompt(user_message, cli_name, model_label.or(copilot_model))
    } else if request_mode.is_plan() {
        match preflight_note {
            Some(note) => format!(
                "当前是 Plan 模式：只生成开发计划，不改文件、不运行命令、不提交、不打包。\n\n注意：{}\n\n{}",
                note, user_message
            ),
            None => format!(
                "当前是 Plan 模式：只生成开发计划，不改文件、不运行命令、不提交、不打包。\n\n{}",
                user_message
            ),
        }
    } else {
        pc_project_execution_prompt(
            user_message,
            preflight_note,
            cli_name,
            model_label.or(copilot_model),
            pc_prompt_bootstrapped,
        )
    };
    let extra_args = pc_route_a_ui_args(
        cli_name,
        native_cli_session_uuid.as_deref(),
        copilot_model,
        effective_codex_reasoning_effort.as_deref(),
        &prompt,
        &state.public_url,
    );

    let pc_req_id = Uuid::new_v4().to_string();
    let pc_cli_feature = if request_mode.is_plan() {
        "pc_agent_cli_plan"
    } else if raw_pc_passthrough {
        "pc_agent_cli_direct"
    } else if cwd.is_some() {
        "pc_agent_cli_dev"
    } else {
        "pc_agent_cli_chat"
    };
    let pc_accounting_key = format!("pc_agent_cli:{pc_req_id}");
    let pc_reserve_fen = billing::configured_reservation_fen(
        &state.store,
        if cwd.is_some() && !raw_pc_passthrough {
            "billing_cli_dev_reservation_fen"
        } else {
            "billing_cli_chat_reservation_fen"
        },
        if cwd.is_some() && !raw_pc_passthrough {
            100
        } else {
            10
        },
    );
    let display_model = pc_display_model_label(
        cli_name,
        model_label.or(copilot_model),
        effective_codex_reasoning_effort.as_deref(),
        lightweight_pc_chat,
        cli_name,
    );
    let reservation_model_id = pc_cli_model_id(Some(&display_model));
    let (mut pc_billing_call, pc_billing_context) = reserve_pc_cli_billing_call(
        state.as_ref(),
        user_id,
        agent_id,
        &pc_accounting_key,
        pc_cli_feature,
        Some(&reservation_model_id),
        pc_reserve_fen,
        cli_name,
    )
    .map_err(|message| anyhow!(message))?;
    let codex_credential_binding = pc_billing_context.codex_credential_binding(cli_name);
    let requires_cloud_control = pc_billing_context.requires_cloud_control();
    let compute_run = start_pc_node_compute_run(
        state,
        user_id,
        agent_id,
        &pc_accounting_key,
        pc_cli_feature,
        Some(&display_model),
    )
    .map_err(|error| anyhow!("建立 PC CLI 计算运行失败: {error:#}"))?;
    let frozen_model_id = compute_run
        .model_id
        .ok_or_else(|| anyhow!("PC CLI 计算运行缺少服务端冻结模型"))?;
    let dispatch_billing_ready =
        match prepare_pc_cli_billing_for_dispatch(&mut pc_billing_call, &pc_billing_context) {
            Ok(ready) => ready,
            Err(error) => {
                let message = format!("持久化 PC CLI 派发预授权失败: {error:#}");
                finish_pc_node_compute_run(
                    state,
                    &pc_accounting_key,
                    "failed",
                    None,
                    None,
                    None,
                    Some(&message),
                );
                pc_billing_call.release_dispatch_not_sent();
                return Err(anyhow!(message));
            }
        };
    let cloud_control_deadline = match bind_pc_cli_replay_policy(
        state.as_ref(),
        user_id,
        &pc_accounting_key,
        &pc_billing_context,
    ) {
        Ok(deadline) => deadline,
        Err(error) => {
            let message = format!("冻结 PC CLI 离线回放策略失败: {error:#}");
            finish_pc_node_compute_run(
                state,
                &pc_accounting_key,
                "settlement_failed",
                None,
                None,
                None,
                Some(&message),
            );
            pc_billing_call.release_dispatch_not_sent();
            return Err(anyhow!(message));
        }
    };
    if let Err(error) = record_pc_execution_started(
        state,
        native_session_scope,
        agent_id,
        &pc_req_id,
        cwd,
        model_label.or(copilot_model),
    ) {
        let message = format!("冻结 PC CLI 项目执行身份失败: {error:#}");
        finish_pc_node_compute_run(
            state,
            &pc_accounting_key,
            "failed",
            None,
            None,
            None,
            Some(&message),
        );
        pc_billing_call.release_dispatch_not_sent();
        return Err(anyhow!(message));
    }
    let mut pc_execution_guard = PcExecutionFinishOnDrop::armed(
        state.clone(),
        native_session_scope.cloned(),
        agent_id.to_string(),
        pc_req_id.clone(),
        Some(display_model.clone()),
    );

    let accepted_dispatch =
        match dispatch_pc_cli_prompt_until_accepted(PcCliPromptDispatchRequest {
            billing_ready: &dispatch_billing_ready,
            state,
            tx,
            agent_id,
            pc_req_id: &pc_req_id,
            cli_name,
            extra_args: &extra_args,
            cwd,
            prompt: &prompt,
            request_mode,
            native_session_scope,
            lightweight_pc_chat,
            codex_credential_binding,
            requires_cloud_control,
            cloud_control_deadline,
        })
        .await
        {
            Ok(dispatch) => dispatch,
            Err(dispatch_failure) => {
                const COMMUNICATION_FAILURE: &str = "PC节点通信自动恢复超时";
                let prompt_may_have_been_sent = dispatch_failure.prompt_may_have_been_sent();
                let error = dispatch_failure.into_error();
                if prompt_may_have_been_sent && pc_billing_context.charge_platform_balance {
                    if let Err(mark_error) = mark_pc_cli_dispatch_outcome_unknown(
                        &state.store,
                        user_id,
                        &pc_accounting_key,
                        COMMUNICATION_FAILURE,
                    ) {
                        tracing::error!(
                            compute_call_id = %pc_accounting_key,
                            %mark_error,
                            "failed to mark unknown PC CLI dispatch outcome"
                        );
                    }
                } else {
                    finish_pc_node_compute_run(
                        state,
                        &pc_accounting_key,
                        "failed",
                        None,
                        None,
                        None,
                        Some(COMMUNICATION_FAILURE),
                    );
                }
                record_pc_execution_without_cli_done(
                    state,
                    native_session_scope,
                    agent_id,
                    &pc_req_id,
                    false,
                    Some(COMMUNICATION_FAILURE),
                    Some(display_model.as_str()),
                );
                pc_execution_guard.disarm();
                if prompt_may_have_been_sent {
                    defer_pc_cli_billing_after_acceptance(
                        &mut pc_billing_call,
                        &pc_billing_context,
                    );
                } else {
                    pc_billing_call.release_dispatch_not_sent();
                }
                return Err(error);
            }
        };
    defer_pc_cli_billing_after_acceptance(&mut pc_billing_call, &pc_billing_context);
    let node_progress_name = pc_node_progress_name(state.as_ref(), agent_id).await;
    let _ = tx.send(pc_dispatch_started_event(
        &pc_req_id,
        agent_id,
        &node_progress_name,
        cli_name,
        cwd,
        native_session_scope,
        request_mode,
    ));

    Ok(PcRunAdmission {
        raw_pc_passthrough,
        lightweight_pc_chat,
        apk_sync_probe_since,
        native_cli_session_uuid,
        pc_development_prompt,
        pc_req_id,
        pc_cli_feature,
        pc_accounting_key,
        pc_billing_call,
        pc_billing_context,
        frozen_model_id,
        display_model,
        pc_execution_guard,
        rx: accepted_dispatch.rx,
        cancel_handle: accepted_dispatch.cancel_handle,
        first_cli_event: accepted_dispatch.first_cli_event,
        node_progress_name,
    })
}
