//! Shared CLI task admission/launch path for cloud WebSocket dispatch and localhost workbench.

use std::sync::Arc;

use homecli_proto::{AgentToServer, CliCodexCredentialBinding, CliProjectContext};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

use crate::node_agent_cli_done::CliCompletionContext;
use crate::node_agent_codex_child_env::FrozenCodexHome;
use crate::{
    node_agent_active_task, node_agent_full_access, node_agent_task_journal,
    resolve_attachment_args, run_cli_prompt, ws_text, CliPromptRun, NodeRuntime,
};

#[path = "node_agent_cli_task_dispatch_failure.rs"]
mod failure;
use failure::send_preflight_failure;

#[derive(Debug)]
pub(crate) struct CliTaskDispatchRequest {
    pub req_id: String,
    pub cli: String,
    pub extra_args: Vec<String>,
    pub cwd: Option<String>,
    pub project_context: Option<CliProjectContext>,
    pub codex_credential_binding: Option<CliCodexCredentialBinding>,
    pub requires_cloud_control: bool,
    pub cloud_control_deadline: Option<String>,
    pub cloud_control_issued_at: Option<String>,
    pub cloud_control_ttl_ms: Option<u64>,
    pub prompt: String,
    pub completion_context: CliCompletionContext,
    /// Present only for a node-validated supervised resume. Authorization is
    /// checked against its base repo while execution reuses the inherited worktree.
    pub inherited_workspace: Option<crate::pc_workspace_provisioner::ConversationWorkspaceResult>,
    pub resume_admission:
        Option<crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard>,
    pub inherited_authorization_record: Option<crate::node_agent_local_task_store::LocalTaskRecord>,
    /// Offline-local work must never auto-switch into a borrowed/shared Codex slot.
    pub allow_codex_auth_switch: bool,
    /// Local work supplies this at creation time. Cloud work captures it once
    /// during admission. The runner must never consult process-global CODEX_HOME.
    pub frozen_codex_home: Option<FrozenCodexHome>,
}

pub(crate) fn spawn_cli_task(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    request: CliTaskDispatchRequest,
) {
    tokio::spawn(async move {
        run_cli_task(runtime, out_tx, request).await;
    });
}

fn send_cli_prompt_reattached(
    out_tx: &mpsc::UnboundedSender<Message>,
    req_id: String,
    cli: String,
    cwd: Option<String>,
    runtime_permission: Option<String>,
) {
    let _ = out_tx.send(ws_text(&crate::node_agent_cli_done::cli_prompt_accepted(
        req_id,
        Some(cli),
        cwd,
        runtime_permission,
    )));
}

async fn run_cli_task(
    runtime: Arc<NodeRuntime>,
    out_tx: mpsc::UnboundedSender<Message>,
    request: CliTaskDispatchRequest,
) {
    let CliTaskDispatchRequest {
        req_id,
        cli,
        extra_args,
        cwd,
        project_context,
        codex_credential_binding,
        requires_cloud_control,
        cloud_control_deadline,
        cloud_control_issued_at,
        cloud_control_ttl_ms,
        prompt,
        completion_context,
        inherited_workspace,
        resume_admission,
        inherited_authorization_record,
        allow_codex_auth_switch,
        frozen_codex_home,
    } = request;
    info!(%req_id, %cli, "starting admitted PC CLI task");
    let req_id_for_cleanup = req_id.clone();
    if let Err(error) = validate_completion_producer_identity(&runtime, &completion_context).await {
        send_preflight_failure(
            &runtime,
            &completion_context,
            &cli,
            &out_tx,
            req_id,
            error.to_string(),
        )
        .await;
        return;
    }
    if let Some(error) = prestart_cancel_admission_error(&runtime.task_journal, &req_id_for_cleanup)
    {
        send_preflight_failure(&runtime, &completion_context, &cli, &out_tx, req_id, error).await;
        return;
    }
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let requested_runtime_permission = project_context
        .as_ref()
        .and_then(|context| context.runtime_permission.clone());
    if runtime.cli_prompt_active(&req_id_for_cleanup).await {
        info!(%req_id_for_cleanup, "re-attaching duplicate dispatch to active PC CLI task");
        send_cli_prompt_reattached(&out_tx, req_id, cli, cwd, requested_runtime_permission);
        return;
    }
    match runtime
        .completion_outbox
        .latest_for_req_id_for_producer(&req_id_for_cleanup, &completion_context.producer_identity)
    {
        Ok(Some(_)) => {
            info!(%req_id_for_cleanup, "re-attaching duplicate dispatch to durable PC CLI completion");
            send_cli_prompt_reattached(&out_tx, req_id, cli, cwd, requested_runtime_permission);
            return;
        }
        Ok(None) => {}
        Err(error) => {
            send_preflight_failure(
                &runtime,
                &completion_context,
                &cli,
                &out_tx,
                req_id,
                format!("读取 durable completion 状态失败，已拒绝重复执行：{error}"),
            )
            .await;
            return;
        }
    }
    let resolved_cli = match runtime.resolve_cli(&cli).await {
        Ok(resolved) => resolved,
        Err(error) => {
            send_preflight_failure(
                &runtime,
                &completion_context,
                &cli,
                &out_tx,
                req_id,
                error.to_string(),
            )
            .await;
            return;
        }
    };
    let resolved_args = resolve_attachment_args(
        extra_args,
        resolved_cli.name(),
        runtime
            .creds
            .read()
            .await
            .as_ref()
            .and_then(|creds| creds.user_token.clone())
            .as_deref(),
    )
    .await;
    let runtime_permission = requested_runtime_permission.clone();
    let local_offline =
        completion_context.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN;
    let frozen_codex_home = if resolved_cli.name() == "codex" {
        let frozen = match frozen_codex_home {
            Some(frozen) => frozen,
            None if local_offline => {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    "本机离线任务没有在创建时冻结 CODEX_HOME，已拒绝启动。".to_string(),
                )
                .await;
                return;
            }
            None => match FrozenCodexHome::capture_for_task() {
                Ok(frozen) => frozen,
                Err(error) => {
                    send_preflight_failure(
                        &runtime,
                        &completion_context,
                        resolved_cli.name(),
                        &out_tx,
                        req_id,
                        error.to_string(),
                    )
                    .await;
                    return;
                }
            },
        };
        if let Err(error) =
            frozen.validate_for_task(local_offline, runtime.is_cloud_connected().await)
        {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                error.to_string(),
            )
            .await;
            return;
        }
        if local_offline {
            if codex_credential_binding.is_some() {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    "本机离线 Codex 任务不能携带云端凭据绑定。".to_string(),
                )
                .await;
                return;
            }
        } else {
            let Some(binding) = codex_credential_binding.as_ref() else {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    "云端 Codex 任务缺少凭据绑定，已拒绝启动。".to_string(),
                )
                .await;
                return;
            };
            if let Err(error) = frozen.validate_cloud_binding(binding) {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    error.to_string(),
                )
                .await;
                return;
            }
        }
        Some(frozen)
    } else {
        if codex_credential_binding.is_some() {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                "非 Codex 任务不能携带 Codex 凭据绑定。".to_string(),
            )
            .await;
            return;
        }
        if local_offline {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                "本机离线任务只允许使用已冻结本地凭据的 Codex CLI。".to_string(),
            )
            .await;
            return;
        }
        None
    };
    let credential_home_requires_cloud_control = frozen_codex_home
        .as_ref()
        .is_some_and(FrozenCodexHome::requires_cloud_control);
    if credential_home_requires_cloud_control && !requires_cloud_control {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            "托管 Codex 凭据必须由云端持续控制，已拒绝启动。".to_string(),
        )
        .await;
        return;
    }
    let effective_requires_cloud_control =
        requires_cloud_control || credential_home_requires_cloud_control;
    let cloud_control_deadline =
        match crate::node_agent_cloud_control::freeze_cloud_control_deadline(
            effective_requires_cloud_control,
            cloud_control_deadline.as_deref(),
            cloud_control_issued_at.as_deref(),
            cloud_control_ttl_ms,
            frozen_codex_home
                .as_ref()
                .and_then(FrozenCodexHome::managed_lease_expires_at),
        ) {
            Ok(value) => value,
            Err(error) => {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    error.to_string(),
                )
                .await;
                return;
            }
        };
    let full_access_identity = match node_agent_full_access::FullAccessGrantIdentity::new(
        &completion_context.producer_identity.owner_user_id,
        &completion_context.producer_identity.agent_id,
        &runtime.install_id,
    ) {
        Ok(identity) => identity,
        Err(error) => {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                error.to_string(),
            )
            .await;
            return;
        }
    };
    let current_task_record = runtime.local_tasks.get(&req_id).ok().flatten();
    if let Err(error) =
        node_agent_full_access::require_route_a_full_access_grant_with_inherited_evidence(
            &runtime.full_access_grants,
            &full_access_identity,
            resolved_cli.name(),
            runtime_permission.as_deref(),
            project_context.as_ref(),
            cwd.as_deref(),
            !local_offline,
            current_task_record.as_ref(),
            inherited_authorization_record.as_ref(),
        )
        .await
    {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            error.to_string(),
        )
        .await;
        return;
    }

    if let Some(cwd) = cwd.as_deref() {
        runtime
            .cache_advisor
            .observe_workspace(std::path::Path::new(cwd));
    }
    let write_task_can_prepare_data_root = project_context
        .as_ref()
        .is_some_and(|context| !crate::cli_prompt_read_only(context.runtime_permission.as_deref()));
    if write_task_can_prepare_data_root {
        if let Err(error) = runtime
            .ensure_node_data_root_for_workspace(cwd.as_deref().map(std::path::Path::new))
            .await
        {
            warn!(
                %error,
                "AI 临时工作区自动回填失败；保留原项目与原缓存并继续任务"
            );
        }
    }

    // Workspace preparation and build admission share the data-root transition
    // lock. A root switch can happen before this transaction or after the
    // lease is registered, never between selecting a workspace and its cache.
    let inherited_workspace_resume = inherited_workspace.is_some();
    let supervision_root_task_id =
        match crate::node_agent_cli_supervision_lease::root_task_id_for_task(
            &runtime.task_journal,
            &req_id_for_cleanup,
            completion_context.supervision_protocol.as_deref(),
        ) {
            Ok(value) => value,
            Err(error) => {
                send_preflight_failure(
                    &runtime,
                    &completion_context,
                    resolved_cli.name(),
                    &out_tx,
                    req_id,
                    format!("failed to resolve supervision root lease identity: {error}"),
                )
                .await;
                return;
            }
        };
    let transition = runtime.node_data_root_transition.clone().lock_owned().await;
    let data_paths = runtime.node_data_root.read().await.paths.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let result = match inherited_workspace {
            Some(workspace) => crate::node_agent_cli_runner::prepare_inherited_cli_prompt_cwd_in(
                data_paths.as_ref(),
                workspace,
                project_context,
            ),
            None => crate::node_agent_cli_runner::prepare_cli_prompt_cwd_in_with_supervision(
                data_paths.as_ref(),
                cwd,
                project_context,
                supervision_root_task_id.as_deref(),
            ),
        };
        (transition, data_paths, result)
    })
    .await;
    let (transition, data_paths, prepared_cwd) = match prepared {
        Ok((transition, data_paths, Ok(cwd))) => (transition, data_paths, cwd),
        Ok((_transition, _data_paths, Err(error))) => {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                error.to_string(),
            )
            .await;
            return;
        }
        Err(error) => {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                format!("PC 项目工作区准备任务异常结束: {error}"),
            )
            .await;
            return;
        }
    };
    if let Err(error) = crate::node_agent_cli_supervision_lease::acquire_for_task(
        &runtime.task_journal,
        &req_id_for_cleanup,
        completion_context.supervision_protocol.as_deref(),
        prepared_cwd.conversation_workspace.as_ref(),
    ) {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            format!("failed to persist supervision worktree lease: {error}"),
        )
        .await;
        return;
    }
    let build_run_guard = if let Some(project_context) = prepared_cwd
        .project_context
        .as_ref()
        .filter(|context| !crate::cli_prompt_read_only(context.runtime_permission.as_deref()))
        .filter(|_| prepared_cwd.data_policy.uses_managed_workspace())
    {
        if let Some(data_paths) = data_paths.as_ref() {
            match crate::node_agent_build_runtime::register_cli_run(
                data_paths,
                crate::node_agent_build_runtime::BuildRunRequest {
                    task_id: &req_id_for_cleanup,
                    project_id: &project_context.project_id,
                    cwd: prepared_cwd.cwd.as_deref().map(std::path::Path::new),
                },
            ) {
                Ok(run) => Some(run),
                Err(error) => {
                    send_preflight_failure(
                        &runtime,
                        &completion_context,
                        resolved_cli.name(),
                        &out_tx,
                        req_id,
                        format!("一龙推荐构建环境准备失败: {error:#}"),
                    )
                    .await;
                    return;
                }
            }
        } else {
            warn!("推荐数据根暂不可用，继续继承原项目构建环境");
            None
        }
    } else {
        None
    };
    drop(transition);
    let original_prompt = prompt.clone();
    let ui_design_routed =
        crate::node_agent_ui_design_workspace::is_ui_design_task_prompt(&original_prompt);
    let ui_design_route_status =
        crate::node_agent_ui_design_workspace::ui_design_route_status(&original_prompt)
            .unwrap_or("READY");
    let (prompt, ui_design_workspace_ready) =
        match crate::node_agent_ui_design_workspace::prepare_ui_design_workspace(
            prompt,
            prepared_cwd.cwd.as_deref(),
            &resolved_args,
        ) {
            Ok(prompt) => (prompt, true),
            Err(error) => {
                warn!(%error, "UI design local artifacts unavailable; continuing degraded");
                (
                    format!(
                        "{original_prompt}\n\nUI design workspace preparation failed: {error:#}\n请先诊断附件与项目工作区，再继续任务。"
                    ),
                    false,
                )
            }
        };
    if ui_design_routed {
        let status = if ui_design_workspace_ready {
            ui_design_route_status
        } else {
            "DEGRADED"
        };
        let event = serde_json::json!({
            "type": "elon.ui_design.route",
            "status": status,
        });
        let _ = out_tx.send(ws_text(&AgentToServer::CliChunk {
            req_id: req_id_for_cleanup.clone(),
            text: format!("{event}\n"),
        }));
    }

    let deadline_cancel_tx = cancel_tx.clone();
    let handle = node_agent_active_task::ActiveCliPromptHandle::new(
        req_id_for_cleanup.clone(),
        resolved_cli.name().to_string(),
        node_agent_active_task::route_for_cli(resolved_cli.name()),
        prepared_cwd.cwd.clone(),
        runtime_permission.clone(),
        cancel_tx,
    )
    .with_requires_cloud_control(effective_requires_cloud_control)
    .with_exclusive_workspace(inherited_workspace_resume);
    match runtime.try_register_cli_prompt(handle).await {
        crate::node_agent_active_task_registry::CliPromptRegistration::Inserted => {}
        crate::node_agent_active_task_registry::CliPromptRegistration::DuplicateReq => {
            warn!(%req_id_for_cleanup, "PC CLI duplicate task registration race lost");
            send_cli_prompt_reattached(
                &out_tx,
                req_id,
                cli,
                prepared_cwd.cwd,
                requested_runtime_permission,
            );
            return;
        }
        crate::node_agent_active_task_registry::CliPromptRegistration::WorkspaceBusy => {
            warn!(%req_id_for_cleanup, "supervised resume workspace is already active");
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                "父任务隔离 worktree 已被其他活跃任务占用，已拒绝续跑。".to_string(),
            )
            .await;
            return;
        }
    }
    drop(resume_admission); // Active registry now owns CLI-lifetime exclusion.
    if let Some(error) = prestart_cancel_admission_error(&runtime.task_journal, &req_id_for_cleanup)
    {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            error,
        )
        .await;
        runtime.finish_cli_prompt(&req_id_for_cleanup).await;
        return;
    }
    if let Err(error) = validate_completion_producer_identity(&runtime, &completion_context).await {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            error.to_string(),
        )
        .await;
        runtime.finish_cli_prompt(&req_id_for_cleanup).await;
        return;
    }
    // Close the disconnect window between preflight and handle registration.
    // The post-insert check applies to every controlled task, including an
    // unmanaged Codex home billed through a cloud reservation.
    let cloud_connected = runtime.is_cloud_connected().await;
    if let Err(error) = crate::node_agent_cloud_control::validate_registered_cloud_control(
        effective_requires_cloud_control,
        cloud_connected,
        cloud_control_deadline.as_ref(),
    ) {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            error.to_string(),
        )
        .await;
        runtime.finish_cli_prompt(&req_id_for_cleanup).await;
        return;
    }
    if let Some(home) = frozen_codex_home.as_ref() {
        if let Err(error) = home.validate_for_task(local_offline, cloud_connected) {
            send_preflight_failure(
                &runtime,
                &completion_context,
                resolved_cli.name(),
                &out_tx,
                req_id,
                error.to_string(),
            )
            .await;
            runtime.finish_cli_prompt(&req_id_for_cleanup).await;
            return;
        }
    }
    crate::node_agent_cloud_control::spawn_absolute_deadline_cancel(
        cloud_control_deadline,
        deadline_cancel_tx,
        req_id_for_cleanup.clone(),
    );
    if out_tx
        .send(ws_text(&crate::node_agent_cli_done::cli_prompt_accepted(
            req_id_for_cleanup.clone(),
            Some(cli.clone()),
            prepared_cwd.cwd.clone(),
            requested_runtime_permission.clone(),
        )))
        .is_err()
    {
        send_preflight_failure(
            &runtime,
            &completion_context,
            resolved_cli.name(),
            &out_tx,
            req_id,
            "任务控制通道已断开，已拒绝在确认前启动 CLI。".to_string(),
        )
        .await;
        runtime.finish_cli_prompt(&req_id_for_cleanup).await;
        return;
    }
    if let Err(error) =
        runtime
            .task_journal
            .record_started(node_agent_task_journal::TaskJournalStart {
                req_id: &req_id_for_cleanup,
                cli_name: resolved_cli.name(),
                route: Some(node_agent_active_task::route_for_cli(resolved_cli.name())),
                run_handle_id: Some(&req_id_for_cleanup),
                cwd: prepared_cwd.cwd.as_deref(),
                runtime_permission: runtime_permission.as_deref(),
            })
    {
        warn!(%error, "failed to persist PC task start journal event");
    }

    run_cli_prompt(CliPromptRun {
        req_id,
        bin: resolved_cli.bin().to_string(),
        cli_name: resolved_cli.name().to_string(),
        extra_args: resolved_args,
        runtime_permission,
        cwd: prepared_cwd.cwd,
        conversation_workspace: prepared_cwd.conversation_workspace,
        prompt,
        server_runtime_config: Some(crate::node_agent_server_runtime::ServerRuntimeConfig {
            server_url: runtime.cloud_http_url(),
            user_token: runtime.user_token().await,
        }),
        approval_state: runtime.tool_approvals.clone(),
        task_journal: runtime.task_journal.clone(),
        runtime: runtime.clone(),
        cancel_rx,
        out_tx,
        codex_auth_attempts: crate::node_agent_codex_auth_switch::CodexAuthAttemptState::new(
            allow_codex_auth_switch,
        ),
        completion_context,
        frozen_codex_home,
    })
    .await;
    drop(build_run_guard);
    if let Err(error) = runtime.task_journal.record_finished(&req_id_for_cleanup) {
        warn!(%error, "failed to persist PC task final journal event");
    }
    runtime.finish_cli_prompt(&req_id_for_cleanup).await;
}

fn prestart_cancel_admission_error(
    journal: &node_agent_task_journal::TaskJournal,
    req_id: &str,
) -> Option<String> {
    match journal.prestart_cancel_tombstone_active(req_id) {
        Ok(false) => None,
        Ok(true) => Some("服务器已在任务启动前撤销该请求，已拒绝启动。".to_string()),
        Err(error) => Some(format!("读取持久取消状态失败，已拒绝启动：{error}")),
    }
}

async fn validate_completion_producer_identity(
    runtime: &NodeRuntime,
    context: &CliCompletionContext,
) -> anyhow::Result<()> {
    let current = runtime
        .creds()
        .await
        .ok_or_else(|| anyhow::anyhow!("本机节点当前未绑定账号，拒绝启动任务。"))?;
    let producer = &context.producer_identity;
    if producer.owner_user_id != current.owner_user_id
        || producer.agent_id != current.agent_id
        || producer.install_id != runtime.install_id
    {
        anyhow::bail!("任务冻结的 owner/节点/安装身份与当前绑定不一致，拒绝启动。")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_dispatch_returns_idempotent_acceptance_not_terminal() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        send_cli_prompt_reattached(
            &tx,
            "req-duplicate".to_string(),
            "codex".to_string(),
            Some("D:/workspace".to_string()),
            Some("full_access".to_string()),
        );

        let Message::Text(payload) = rx.try_recv().expect("duplicate response") else {
            panic!("duplicate response must be websocket text");
        };
        let event: AgentToServer =
            serde_json::from_str(payload.as_ref()).expect("valid duplicate response");
        assert!(matches!(
            event,
            AgentToServer::CliPromptAccepted {
                req_id,
                cli: Some(cli),
                cwd: Some(cwd),
                runtime_permission: Some(runtime_permission),
            } if req_id == "req-duplicate"
                && cli == "codex"
                && cwd == "D:/workspace"
                && runtime_permission == "full_access"
        ));
    }
}
