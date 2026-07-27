use super::*;

pub(super) fn prepare_binding(
    state: &AppState,
    authenticated_node_id: &str,
    completion: &CliCompletionEnvelope,
) -> std::result::Result<ReplayBinding, ReplayFailure> {
    // Live ingest already compared this frozen producer with the authenticated
    // WebSocket session. Pending receipt workers must keep using the immutable
    // payload instead of re-reading whichever account owns the node later.
    let authoritative_owner = completion
        .producer_identity
        .as_ref()
        .map(|identity| identity.owner_user_id.as_str())
        .ok_or_else(|| ReplayFailure::reject("completion 缺少冻结的生产者身份"))?;
    let compute_call_id = format!("pc_agent_cli:{}", completion.req_id);

    let mut local_claimed_run = None;
    let local_channel_id = if completion.origin == LOCAL_OFFLINE_ORIGIN {
        let channel_id = validate_local_offline_scope(
            state,
            authenticated_node_id,
            authoritative_owner,
            completion,
        )?;
        let model = completion
            .model
            .as_deref()
            .or(Some(completion.cli.as_str()));
        let context = completion
            .project_context
            .as_ref()
            .ok_or_else(|| ReplayFailure::reject("本机 completion 缺少项目上下文"))?;
        let claim = state
            .store
            .claim_local_offline_node_compute_run(LocalOfflineNodeComputeRunClaim {
                compute_call_id: &compute_call_id,
                request_id: &completion.req_id,
                owner_user_id: authoritative_owner,
                node_id: authenticated_node_id,
                project_id: &context.project_id,
                conversation_id: &context.conversation_id,
                model_id: model,
            })
            .map_err(ReplayFailure::retry)?;
        match claim {
            LocalOfflineNodeComputeRunClaimOutcome::Claimed { run, .. } => {
                local_claimed_run = Some(run)
            }
            LocalOfflineNodeComputeRunClaimOutcome::Conflict { reason } => {
                return Err(ReplayFailure::reject(format!(
                    "本机离线执行绑定冲突：{reason}"
                )))
            }
        }
        Some(channel_id)
    } else {
        None
    };

    let run = match local_claimed_run {
        Some(run) => run,
        None => state
            .store
            .get_node_compute_run_by_compute_call_id(&compute_call_id)
            .map_err(ReplayFailure::retry)?
            .ok_or_else(|| ReplayFailure::retry("completion 对应的计算运行尚未建立"))?,
    };
    if run.node_id != authenticated_node_id {
        return Err(ReplayFailure::reject("completion 不属于当前鉴权节点"));
    }
    if completion.origin == LOCAL_OFFLINE_ORIGIN && run.consumer_user_id != authoritative_owner {
        return Err(ReplayFailure::reject("本机离线 completion owner 不匹配"));
    }

    let session = state
        .store
        .get_project_execution_session_by_request_id(&completion.req_id)
        .map_err(ReplayFailure::retry)?;
    if completion.origin == CLOUD_DISPATCH_ORIGIN {
        validate_cloud_session(completion, &run, session.as_ref(), authenticated_node_id)?;
    } else if let Some(session) = session.as_ref() {
        let context = completion
            .project_context
            .as_ref()
            .ok_or_else(|| ReplayFailure::reject("本机 completion 缺少项目上下文"))?;
        if session.node_id != authenticated_node_id
            || session.user_id != authoritative_owner
            || session.project_id != context.project_id
            || session.conversation_id != context.conversation_id
        {
            return Err(ReplayFailure::reject(
                "本机 completion 与既有项目执行会话绑定不一致",
            ));
        }
    }

    let existing_accounting = state
        .store
        .get_token_usage_accounting_by_idempotency_key(&run.consumer_user_id, &compute_call_id)
        .map_err(ReplayFailure::retry)?;
    let usage = completion_usage(completion, frozen_run_model_id(&run)?);
    let unresolved_billable_usage = pc_cli_unknown_usage_requires_verification(
        Some(&billing_context_from_run(&run)),
        usage.as_ref(),
    );
    let policy_allows = state
        .store
        .can_replay_node_compute_run_offline(&compute_call_id)
        .map_err(ReplayFailure::retry)?;
    if existing_accounting.is_none() && !policy_allows && !unresolved_billable_usage {
        return Err(ReplayFailure::reject(
            "原任务没有可用的离线授权或有效预授权；共享/平台资源必须保持联网",
        ));
    }

    Ok(ReplayBinding {
        run,
        session,
        existing_accounting,
        local_channel_id,
    })
}

pub(super) fn validate_local_offline_scope(
    state: &AppState,
    node_id: &str,
    owner_user_id: &str,
    completion: &CliCompletionEnvelope,
) -> std::result::Result<String, ReplayFailure> {
    if !completion.cli.to_ascii_lowercase().contains("codex") {
        return Err(ReplayFailure::reject(
            "离线本机任务只允许使用 owner 自己的 Codex",
        ));
    }
    let context = completion
        .project_context
        .as_ref()
        .ok_or_else(|| ReplayFailure::reject("本机离线 completion 缺少项目上下文"))?;
    if !matches!(
        context.runtime_permission.as_deref(),
        Some("full_access" | "danger_full_access")
    ) {
        return Err(ReplayFailure::reject(
            "本机离线 completion 必须来自显式 full_access 授权",
        ));
    }
    if completion
        .prompt
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        return Err(ReplayFailure::reject(
            "本机离线 completion 缺少原始任务内容",
        ));
    }
    let project = state
        .store
        .get_project_access(owner_user_id, &context.project_id)
        .map_err(classify_local_offline_scope_store_error)?;
    if project.node_id.as_deref() != Some(node_id) {
        return Err(ReplayFailure::reject("项目没有绑定到当前本机节点"));
    }
    if !crate::store::project_runtime_permission_allows_full_access(&project.runtime_permission) {
        return Err(ReplayFailure::reject(
            "项目当前未授权 full_access，离线结果不能写回",
        ));
    }
    let requested_channel_id = completion
        .channel_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let channel_id = match requested_channel_id {
        Some(channel_id) => channel_id.to_string(),
        None => state
            .store
            .list_project_space_channels(owner_user_id, &context.project_id)
            .map_err(classify_local_offline_scope_store_error)?
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .map(|channel| channel.id)
            .ok_or_else(|| ReplayFailure::reject("项目没有可写回的 AI开发 频道"))?,
    };
    let channel_kind = state
        .store
        .get_project_channel_kind(&context.project_id, &channel_id)
        .map_err(classify_local_offline_scope_store_error)?;
    if channel_kind != "ai_development" {
        return Err(ReplayFailure::reject("离线结果只能写回 AI开发 频道"));
    }
    let permissions = state
        .store
        .project_member_channel_permissions(&context.project_id, &channel_id, owner_user_id)
        .map_err(classify_local_offline_scope_store_error)?;
    if !permissions.can_view || !permissions.can_start_ai {
        return Err(ReplayFailure::reject("当前成员权限不允许写回离线 AI 任务"));
    }
    if state
        .store
        .active_project_member_muted_until(&context.project_id, owner_user_id)
        .map_err(ReplayFailure::retry)?
        .is_some()
    {
        return Err(ReplayFailure::reject("当前成员已被禁言，不能写回离线任务"));
    }
    if let Some(lease) = state
        .store
        .get_active_codex_vault_emergency_lease_for_node(owner_user_id, node_id)
        .map_err(ReplayFailure::retry)?
    {
        if lease.provider_user_id != owner_user_id {
            return Err(local_offline_shared_lease_retry());
        }
    }
    Ok(channel_id)
}

/// Keep the shared-account boundary fail-closed while preserving the durable
/// owner-local result. The bounded lease will expire or be cleared, after
/// which the same outbox event can be validated and applied normally.
pub(super) fn local_offline_shared_lease_retry() -> ReplayFailure {
    ReplayFailure::retry("当前节点仍处于朋友共享 Codex 租约中，离线任务暂缓按自有账号上报")
}

pub(super) fn validate_cloud_session(
    completion: &CliCompletionEnvelope,
    run: &NodeComputeRun,
    session: Option<&ProjectExecutionSession>,
    node_id: &str,
) -> std::result::Result<(), ReplayFailure> {
    if completion.prompt.is_some() || completion.channel_id.is_some() {
        return Err(ReplayFailure::reject(
            "云端派发 completion 不允许携带本机 prompt/channel",
        ));
    }
    match (completion.project_context.as_ref(), session) {
        (Some(context), Some(session)) => {
            if session.node_id != node_id
                || session.user_id != run.consumer_user_id
                || session.project_id != context.project_id
                || session.conversation_id != context.conversation_id
            {
                return Err(ReplayFailure::reject("completion 与项目执行会话绑定不一致"));
            }
            if session.task_id.is_none() {
                return Err(ReplayFailure::retry("项目执行会话尚未绑定云端 task_id"));
            }
        }
        (Some(_), None) => {
            return Err(ReplayFailure::retry(
                "completion 对应的项目执行会话尚未建立",
            ))
        }
        (None, Some(_)) => {
            return Err(ReplayFailure::reject(
                "项目 completion 缺少节点派发时的项目上下文",
            ))
        }
        (None, None) => {}
    }
    Ok(())
}

pub(super) fn completion_usage(
    completion: &CliCompletionEnvelope,
    frozen_model_id: &str,
) -> Option<CliTokenUsage> {
    bind_pc_cli_usage_to_frozen_model(
        usage_from_optional_parts(
            completion.prompt_tokens,
            completion.cached_input_tokens,
            completion.completion_tokens,
            completion.reasoning_tokens,
            completion.total_tokens,
            completion.model.clone(),
        ),
        frozen_model_id,
    )
}

pub(super) fn frozen_run_model_id(
    run: &NodeComputeRun,
) -> std::result::Result<&str, ReplayFailure> {
    run.model_id
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .ok_or_else(|| ReplayFailure::retry("计算运行缺少服务端冻结模型，等待人工核验"))
}

pub(super) fn require_verified_billable_usage(
    context: &PcCliBillingContext,
    usage: Option<&CliTokenUsage>,
) -> std::result::Result<(), ReplayFailure> {
    if pc_cli_unknown_usage_requires_verification(Some(context), usage) {
        return Err(ReplayFailure::retry(
            "共享/平台 PC CLI completion 缺少可信 token 用量，已保留预授权等待核验",
        ));
    }
    Ok(())
}

pub(super) fn serialize_completion_payload(
    completion: &CliCompletionEnvelope,
) -> std::result::Result<String, ReplayFailure> {
    let payload_json = serde_json::to_string(completion).map_err(ReplayFailure::retry)?;
    if payload_json.len() > MAX_COMPLETION_PAYLOAD_BYTES {
        return Err(ReplayFailure::reject(format!(
            "completion payload 超过服务器 {} 字节限制",
            MAX_COMPLETION_PAYLOAD_BYTES
        )));
    }
    Ok(payload_json)
}

pub(super) fn classify_local_offline_scope_store_error(error: anyhow::Error) -> ReplayFailure {
    let message = error.to_string();
    if matches!(
        message.as_str(),
        "项目不存在"
            | "项目不存在，或当前用户无权访问"
            | "你已被该项目封禁，无法访问项目空间"
            | "频道不存在"
    ) {
        ReplayFailure::reject(message)
    } else {
        ReplayFailure::retry(message)
    }
}

pub(super) fn billing_context_from_run(run: &NodeComputeRun) -> PcCliBillingContext {
    PcCliBillingContext {
        billing_source: run.billing_source.clone(),
        resource_owner_user_id: run.resource_owner_user_id.clone(),
        lease_id: run.lease_id.clone(),
        replay_deadline: run.replay_deadline.clone(),
        charge_platform_balance: matches!(run.billing_source.as_str(), "platform" | "shared_codex"),
        max_cost_rmb_fen: run.max_cost_rmb_fen,
        allowance_id: run.allowance_id.clone(),
        frozen_reservation_required: matches!(
            run.billing_source.as_str(),
            "platform" | "shared_codex"
        ),
    }
}

pub(super) fn classify_replay_accounting_error(error: anyhow::Error) -> ReplayFailure {
    if error
        .downcast_ref::<crate::store::token_usage::BillingReservationConstraintViolation>()
        .is_some()
    {
        ReplayFailure::reject(error)
    } else {
        ReplayFailure::retry(error)
    }
}

pub(super) struct CompletionDisplay {
    pub(super) status: &'static str,
    pub(super) reply: String,
    pub(super) channel_result: String,
    pub(super) error: Option<String>,
}

pub(super) fn completion_display(completion: &CliCompletionEnvelope) -> CompletionDisplay {
    let is_codex = completion.cli.to_ascii_lowercase().contains("codex");
    let readable = pc_cli_readable_output(
        is_codex,
        false,
        !completion.final_output.trim().is_empty(),
        &completion.final_output,
    );
    let (effective_exit_ok, effective_error) = readable.completion_status(
        completion.exit_ok,
        false,
        is_codex,
        false,
        completion.error.as_deref(),
    );
    let allow_output_despite_error = pc_codex_error_output_can_complete(
        is_codex,
        readable.has_success_output,
        false,
        effective_error.as_deref(),
        &completion.final_output,
    );
    let succeeded = effective_exit_ok || allow_output_despite_error;
    if succeeded {
        let reply = if is_codex && !readable.codex_final_reply.trim().is_empty() {
            readable.codex_final_reply.trim().to_string()
        } else if !completion.final_output.trim().is_empty() {
            completion.final_output.trim().to_string()
        } else {
            "AI 开发任务已完成。".to_string()
        };
        CompletionDisplay {
            status: "done",
            channel_result: result_message(&reply, None, Some("已完成")),
            reply,
            error: None,
        }
    } else {
        let error = pc_cli_terminal_error_message(
            &completion.cli,
            false,
            effective_error.as_deref(),
            &completion.final_output,
        );
        CompletionDisplay {
            status: "failed",
            channel_result: result_message(&error, None, Some("失败")),
            reply: error.clone(),
            error: Some(error),
        }
    }
}

pub(super) fn validate_authenticated_producer(
    authenticated_node_id: &str,
    authenticated_owner_user_id: Option<&str>,
    authenticated_install_id: Option<&str>,
    completion: &CliCompletionEnvelope,
) -> std::result::Result<(), ReplayFailure> {
    let producer = completion
        .producer_identity
        .as_ref()
        .ok_or_else(|| ReplayFailure::reject("completion 缺少冻结的生产者身份"))?;
    let owner = authenticated_owner_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ReplayFailure::reject("当前节点会话没有可验证的 owner 身份"))?;
    let install_id = authenticated_install_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ReplayFailure::reject("当前节点会话没有可验证的 install_id"))?;
    if producer.agent_id != authenticated_node_id
        || producer.owner_user_id != owner
        || producer.install_id != install_id
    {
        return Err(ReplayFailure::reject(
            "completion 生产者身份与当前鉴权节点会话不一致",
        ));
    }
    Ok(())
}

pub(super) fn validate_envelope(
    completion: &CliCompletionEnvelope,
) -> std::result::Result<(), ReplayFailure> {
    for (field, value) in [
        ("event_id", completion.event_id.as_str()),
        ("req_id", completion.req_id.as_str()),
        ("cli", completion.cli.as_str()),
    ] {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
            return Err(ReplayFailure::reject(format!("completion {field} 无效")));
        }
    }
    let producer = completion
        .producer_identity
        .as_ref()
        .ok_or_else(|| ReplayFailure::reject("completion 缺少冻结的生产者身份"))?;
    for (field, value) in [
        ("producer.owner_user_id", producer.owner_user_id.as_str()),
        ("producer.agent_id", producer.agent_id.as_str()),
        ("producer.install_id", producer.install_id.as_str()),
    ] {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 200 || value.chars().any(char::is_control) {
            return Err(ReplayFailure::reject(format!("completion {field} 无效")));
        }
    }
    if !matches!(
        completion.origin.as_str(),
        CLOUD_DISPATCH_ORIGIN | LOCAL_OFFLINE_ORIGIN
    ) {
        return Err(ReplayFailure::reject("不支持的 completion origin"));
    }
    if completion.created_at_ms == 0 {
        return Err(ReplayFailure::reject("completion 缺少终态时间"));
    }
    if completion.final_output.len() > MAX_OUTPUT_BYTES {
        return Err(ReplayFailure::reject("completion 输出超过服务器限制"));
    }
    if completion
        .prompt
        .as_deref()
        .is_some_and(|prompt| prompt.chars().count() > MAX_PROMPT_CHARS)
    {
        return Err(ReplayFailure::reject("completion prompt 超过服务器限制"));
    }
    if [
        completion.prompt_tokens,
        completion.cached_input_tokens,
        completion.completion_tokens,
        completion.reasoning_tokens,
        completion.total_tokens,
    ]
    .into_iter()
    .flatten()
    .any(|tokens| tokens > MAX_REPORTED_TOKENS)
    {
        return Err(ReplayFailure::reject("completion token 数量异常"));
    }
    Ok(())
}

pub(super) fn completion_ack(
    event_id: String,
    req_id: String,
    accepted: bool,
    deduplicated: bool,
    retryable: bool,
    error: Option<String>,
) -> ServerToAgent {
    ServerToAgent::CliCompletionAck {
        event_id,
        req_id,
        accepted,
        deduplicated,
        retryable,
        error,
    }
}
