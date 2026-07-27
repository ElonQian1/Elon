//! Durable PC CLI completion replay and acknowledgement.

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use homecli_proto::{CliCompletionEnvelope, ServerToAgent};
use sha2::{Digest, Sha256};

use super::{
    pc_billing::{
        record_pc_cli_trusted_usage_result, settle_pc_cli_node_usage, PcCliBillingContext,
    },
    pc_billing_policy::{
        bind_pc_cli_usage_to_frozen_model, hold_pc_cli_usage_for_verification,
        pc_cli_unknown_usage_requires_verification,
    },
    pc_cli_failure::{
        pc_cli_readable_output, pc_cli_terminal_error_message, pc_codex_error_output_can_complete,
    },
};
use crate::{
    cli_usage::{usage_from_optional_parts, CliTokenUsage},
    project_space_task_result::result_message,
    store::{
        LocalOfflineNodeComputeRunClaim, LocalOfflineNodeComputeRunClaimOutcome,
        NodeCliCompletionIngestOutcome, NodeCliCompletionReceipt, NodeCliCompletionReceiptInput,
        NodeComputeRun, PcCliTaskCompletionApply, ProjectExecutionSession,
        ProjectExecutionSessionFinish, ProjectExecutionSessionStart,
    },
    types::AppState,
};

#[path = "pc_local_task_sync.rs"]
mod local_task_sync;
#[path = "pc_completion_replay_support.rs"]
mod support;

pub(crate) use local_task_sync::handle as handle_pc_local_task_sync;

use support::{
    billing_context_from_run, classify_replay_accounting_error, completion_ack, completion_display,
    completion_usage, frozen_run_model_id, prepare_binding, require_verified_billable_usage,
    serialize_completion_payload, validate_authenticated_producer, validate_envelope,
};
#[cfg(test)]
use support::{classify_local_offline_scope_store_error, local_offline_shared_lease_retry};

const LOCAL_OFFLINE_ORIGIN: &str = "local_offline";
const CLOUD_DISPATCH_ORIGIN: &str = "cloud_dispatch";
const MAX_PROMPT_CHARS: usize = 80_000;
const MAX_OUTPUT_BYTES: usize = 900_000;
const MAX_COMPLETION_PAYLOAD_BYTES: usize = 1024 * 1024;
const MAX_REPORTED_TOKENS: u64 = 100_000_000;

#[derive(Debug)]
enum ReplayFailure {
    Retry(String),
    Reject(String),
}

impl ReplayFailure {
    fn retry(error: impl std::fmt::Display) -> Self {
        Self::Retry(error.to_string())
    }

    fn reject(error: impl std::fmt::Display) -> Self {
        Self::Reject(error.to_string())
    }
}

struct ReplayBinding {
    run: NodeComputeRun,
    session: Option<ProjectExecutionSession>,
    existing_accounting: Option<crate::store::TokenUsageAccountingResult>,
    local_channel_id: Option<String>,
}

pub(crate) fn handle_pc_cli_completion_replay(
    state: &AppState,
    authenticated_node_id: &str,
    authenticated_owner_user_id: Option<&str>,
    authenticated_install_id: Option<&str>,
    completion: CliCompletionEnvelope,
) -> ServerToAgent {
    let event_id = completion.event_id.clone();
    let req_id = completion.req_id.clone();
    completion_ack_from_result(
        event_id,
        req_id,
        validate_authenticated_producer(
            authenticated_node_id,
            authenticated_owner_user_id,
            authenticated_install_id,
            &completion,
        )
        .and_then(|()| handle_replay_inner(state, authenticated_node_id, &completion)),
    )
}

fn completion_ack_from_result(
    event_id: String,
    req_id: String,
    result: std::result::Result<bool, ReplayFailure>,
) -> ServerToAgent {
    match result {
        Ok(deduplicated) => completion_ack(event_id, req_id, true, deduplicated, false, None),
        Err(ReplayFailure::Retry(error)) => {
            completion_ack(event_id, req_id, false, false, true, Some(error))
        }
        Err(ReplayFailure::Reject(error)) => {
            completion_ack(event_id, req_id, false, false, false, Some(error))
        }
    }
}

fn handle_replay_inner(
    state: &AppState,
    authenticated_node_id: &str,
    completion: &CliCompletionEnvelope,
) -> std::result::Result<bool, ReplayFailure> {
    validate_envelope(completion)?;
    let payload_json = serialize_completion_payload(completion)?;
    let binding = prepare_binding(state, authenticated_node_id, completion)?;
    let compute_call_id = format!("pc_agent_cli:{}", completion.req_id);
    let payload_sha256 = hex::encode(Sha256::digest(payload_json.as_bytes()));
    let ingest = state
        .store
        .ingest_node_cli_completion_receipt(NodeCliCompletionReceiptInput {
            event_id: &completion.event_id,
            req_id: &completion.req_id,
            compute_call_id: &compute_call_id,
            node_id: authenticated_node_id,
            user_id: &binding.run.consumer_user_id,
            payload_json: &payload_json,
            payload_sha256: &payload_sha256,
        })
        .map_err(ReplayFailure::retry)?;

    let (receipt, ingest_deduplicated) = match ingest {
        NodeCliCompletionIngestOutcome::Conflict { reason, .. } => {
            return Err(ReplayFailure::reject(format!(
                "CLI completion 幂等冲突：{reason}"
            )))
        }
        NodeCliCompletionIngestOutcome::Inserted(receipt) => (receipt, false),
        NodeCliCompletionIngestOutcome::Duplicate(receipt) => (receipt, true),
    };
    match receipt.status.as_str() {
        "applied" => return Ok(true),
        "rejected" => {
            return Err(ReplayFailure::reject(
                receipt
                    .reason
                    .unwrap_or_else(|| "服务器已永久拒绝该 completion".to_string()),
            ))
        }
        "pending" | "retry" | "processing" => {}
        status => {
            return Err(ReplayFailure::retry(format!(
                "未知 completion receipt 状态：{status}"
            )))
        }
    }

    let claim_id = uuid::Uuid::new_v4().to_string();
    let Some(receipt) = state
        .store
        .claim_node_cli_completion_receipt(&completion.event_id, &claim_id)
        .map_err(ReplayFailure::retry)?
    else {
        let current = state
            .store
            .get_node_cli_completion_receipt(&completion.event_id)
            .map_err(ReplayFailure::retry)?
            .ok_or_else(|| ReplayFailure::retry("completion receipt 在 claim 前消失"))?;
        return match current.status.as_str() {
            "applied" => Ok(true),
            "rejected" => {
                Err(ReplayFailure::reject(current.reason.unwrap_or_else(|| {
                    "服务器已永久拒绝该 completion".to_string()
                })))
            }
            _ => Err(ReplayFailure::retry(
                "completion 正由另一个幂等处理器结算，请稍后重试",
            )),
        };
    };

    match apply_receipt(state, &receipt, completion, binding, &claim_id) {
        Ok(()) => Ok(ingest_deduplicated),
        Err(ReplayFailure::Retry(error)) => {
            let finalized = state
                .store
                .finish_node_cli_completion_claim_retry(&completion.event_id, &claim_id, &error)
                .map_err(ReplayFailure::retry)?;
            if !finalized {
                return Err(ReplayFailure::retry(
                    "completion retry 状态未能由当前 claim 提交",
                ));
            }
            Err(ReplayFailure::Retry(error))
        }
        Err(ReplayFailure::Reject(error)) => {
            let finalized = state
                .store
                .finish_node_cli_completion_claim_rejected(&completion.event_id, &claim_id, &error)
                .map_err(ReplayFailure::retry)?;
            if !finalized {
                return Err(ReplayFailure::retry(
                    "completion rejected 状态未能由当前 claim 提交",
                ));
            }
            Err(ReplayFailure::Reject(error))
        }
    }
}

fn apply_receipt(
    state: &AppState,
    receipt: &NodeCliCompletionReceipt,
    completion: &CliCompletionEnvelope,
    mut binding: ReplayBinding,
    claim_id: &str,
) -> std::result::Result<(), ReplayFailure> {
    let context = billing_context_from_run(&binding.run);
    let frozen_model_id = frozen_run_model_id(&binding.run)?;
    let usage = completion_usage(completion, frozen_model_id);
    if let Err(failure) = require_verified_billable_usage(&context, usage.as_ref()) {
        hold_pc_cli_usage_for_verification(
            &state.store,
            &binding.run.consumer_user_id,
            &binding.run.compute_call_id,
        )
        .map_err(ReplayFailure::retry)?;
        return Err(failure);
    }
    let accounting_result = if let Some(usage) = usage.as_ref() {
        match binding.existing_accounting.take() {
            Some(existing) => Some(existing),
            None => record_pc_cli_trusted_usage_result(
                &state.store,
                &binding.run.consumer_user_id,
                &binding.run.feature,
                frozen_model_id,
                usage,
                &binding.run.compute_call_id,
                &context,
            )
            .map_err(classify_replay_accounting_error)?
            .ok_or_else(|| ReplayFailure::retry("completion 含 token，但用量没有成功入账"))
            .map(Some)?,
        }
    } else {
        binding.existing_accounting.take()
    };
    let node_transaction = settle_pc_cli_node_usage(
        state,
        &binding.run.consumer_user_id,
        &binding.run.node_id,
        &binding.run.feature,
        frozen_model_id,
        usage.as_ref().unwrap_or(&CliTokenUsage::default()),
        accounting_result.as_ref(),
        &context,
    )
    .map_err(ReplayFailure::retry)?;

    let display = completion_display(completion);
    let task_outcome = if completion.origin == LOCAL_OFFLINE_ORIGIN {
        let project_context = completion
            .project_context
            .as_ref()
            .ok_or_else(|| ReplayFailure::reject("本机 completion 缺少项目上下文"))?;
        let session_bound = state
            .store
            .record_project_execution_started(ProjectExecutionSessionStart {
                project_id: &project_context.project_id,
                conversation_id: &project_context.conversation_id,
                user_id: &binding.run.consumer_user_id,
                node_id: &binding.run.node_id,
                request_id: &completion.req_id,
                requested_workspace_path: completion
                    .workspace_status
                    .as_ref()
                    .map(|status| status.active_workspace_path.as_str()),
                model: Some(frozen_model_id),
            })
            .map_err(ReplayFailure::retry)?;
        if !session_bound {
            return Err(ReplayFailure::reject(
                "本机 completion 的项目执行会话身份冲突",
            ));
        }
        let outcome = state
            .store
            .apply_pc_cli_task_completion(PcCliTaskCompletionApply {
                completion_event_id: &completion.event_id,
                task_id: None,
                local_request_id: Some(&completion.req_id),
                project_id: &project_context.project_id,
                channel_id: binding.local_channel_id.as_deref(),
                conversation_id: &project_context.conversation_id,
                user_id: &binding.run.consumer_user_id,
                prompt: completion.prompt.as_deref(),
                final_reply: &display.reply,
                channel_result: &display.channel_result,
                status: display.status,
                error: display.error.as_deref(),
                codex_session_id: completion.session_id.as_deref(),
            })
            .map_err(ReplayFailure::retry)?;
        let task_bound = state
            .store
            .bind_project_execution_task_id(&completion.req_id, &outcome.task_id)
            .map_err(ReplayFailure::retry)?;
        if !task_bound {
            return Err(ReplayFailure::reject(
                "本机 completion 不能改写既有项目执行 task_id 绑定",
            ));
        }
        Some(outcome)
    } else if let Some(session) = binding.session.as_ref() {
        let task_id = session
            .task_id
            .as_deref()
            .ok_or_else(|| ReplayFailure::retry("项目执行会话尚未绑定 task_id"))?;
        Some(
            state
                .store
                .apply_pc_cli_task_completion(PcCliTaskCompletionApply {
                    completion_event_id: &completion.event_id,
                    task_id: Some(task_id),
                    local_request_id: None,
                    project_id: &session.project_id,
                    channel_id: None,
                    conversation_id: &session.conversation_id,
                    user_id: &binding.run.consumer_user_id,
                    prompt: None,
                    final_reply: &display.reply,
                    channel_result: &display.channel_result,
                    status: display.status,
                    error: display.error.as_deref(),
                    codex_session_id: completion.session_id.as_deref(),
                })
                .map_err(ReplayFailure::retry)?,
        )
    } else {
        None
    };
    if task_outcome
        .as_ref()
        .is_some_and(|outcome| outcome.terminal_conflict)
    {
        return Err(ReplayFailure::reject(
            "云端任务已有相反方向的真实终态，拒绝覆盖",
        ));
    }
    let task_canceled = task_outcome
        .as_ref()
        .is_some_and(|outcome| outcome.canceled);

    if let Some(session) = state
        .store
        .get_project_execution_session_by_request_id(&completion.req_id)
        .map_err(ReplayFailure::retry)?
    {
        let context = completion
            .project_context
            .as_ref()
            .ok_or_else(|| ReplayFailure::reject("项目执行 completion 缺少项目上下文"))?;
        if session.node_id != binding.run.node_id
            || session.user_id != binding.run.consumer_user_id
            || session.project_id != context.project_id
            || session.conversation_id != context.conversation_id
        {
            return Err(ReplayFailure::reject(
                "项目执行会话在应用前发生身份绑定冲突",
            ));
        }
        let finished = state
            .store
            .record_project_execution_finished(ProjectExecutionSessionFinish {
                request_id: &completion.req_id,
                project_id: &context.project_id,
                conversation_id: &context.conversation_id,
                user_id: &binding.run.consumer_user_id,
                node_id: &binding.run.node_id,
                base_workspace_path: completion
                    .workspace_status
                    .as_ref()
                    .and_then(|status| status.base_workspace_path.as_deref()),
                active_workspace_path: completion
                    .workspace_status
                    .as_ref()
                    .map(|status| status.active_workspace_path.as_str()),
                branch: completion
                    .workspace_status
                    .as_ref()
                    .and_then(|status| status.branch.as_deref()),
                isolated: completion
                    .workspace_status
                    .as_ref()
                    .map(|status| status.isolated)
                    .unwrap_or(false),
                status: if task_canceled {
                    "canceled"
                } else {
                    display.status
                },
                merge_status: if task_canceled {
                    Some("canceled")
                } else {
                    completion
                        .workspace_status
                        .as_ref()
                        .and_then(|status| status.merge_status.as_deref())
                        .or(Some("completion_replayed"))
                },
                last_error: if task_canceled {
                    Some("用户已取消任务")
                } else {
                    display.error.as_deref()
                },
                model: Some(frozen_model_id),
                prompt_tokens: usage.as_ref().map(|usage| usage.input_tokens),
                cached_input_tokens: usage.as_ref().map(|usage| usage.cached_input_tokens),
                completion_tokens: usage.as_ref().map(|usage| usage.output_tokens),
                reasoning_tokens: usage.as_ref().map(|usage| usage.reasoning_tokens),
                total_tokens: usage.as_ref().map(|usage| usage.total_tokens),
                token_usage_event_id: accounting_result
                    .as_ref()
                    .map(|result| result.token_usage_event_id.as_str()),
                billing_event_id: accounting_result
                    .as_ref()
                    .and_then(|result| result.billing_event_id.as_deref()),
            })
            .map_err(ReplayFailure::retry)?;
        if !finished {
            return Err(ReplayFailure::reject("项目执行会话完成写入拒绝了身份绑定"));
        }
    }

    state
        .store
        .finish_node_compute_run(
            &binding.run.compute_call_id,
            crate::store::NodeComputeRunFinish {
                provider_user_id: node_transaction
                    .as_ref()
                    .map(|transaction| transaction.provider_user_id.as_str()),
                status: if display.status == "done" {
                    if accounting_result
                        .as_ref()
                        .is_some_and(|result| result.deduplicated)
                    {
                        "deduplicated"
                    } else if usage.is_some() {
                        "settled"
                    } else {
                        "released_no_usage"
                    }
                } else {
                    "failed"
                },
                prompt_tokens: usage.as_ref().map(|usage| usage.input_tokens).unwrap_or(0),
                completion_tokens: usage.as_ref().map(|usage| usage.output_tokens).unwrap_or(0),
                billed_cost_rmb_fen: node_transaction
                    .as_ref()
                    .map(|transaction| transaction.billed_cost_rmb_fen)
                    .or_else(|| accounting_result.as_ref().map(|result| result.cost_rmb_fen))
                    .unwrap_or(0),
                provider_earned_fen: node_transaction
                    .as_ref()
                    .map(|transaction| transaction.provider_earned_fen)
                    .unwrap_or(0),
                settlement_status: node_transaction
                    .as_ref()
                    .map(|transaction| transaction.settlement_status.as_str())
                    .or_else(|| {
                        accounting_result
                            .as_ref()
                            .map(|result| result.accounting_status.as_str())
                    }),
                error_message: display.error.as_deref(),
            },
        )
        .map_err(ReplayFailure::retry)?;

    let finalized = state
        .store
        .finish_node_cli_completion_claim_applied(
            &receipt.event_id,
            claim_id,
            accounting_result
                .as_ref()
                .map(|result| result.token_usage_event_id.as_str()),
            accounting_result
                .as_ref()
                .and_then(|result| result.billing_event_id.as_deref()),
            node_transaction
                .as_ref()
                .map(|transaction| transaction.id.as_str()),
        )
        .map_err(ReplayFailure::retry)?;
    if !finalized {
        return Err(ReplayFailure::retry(
            "completion applied 状态未能由当前 claim 提交",
        ));
    }

    if let Some(outcome) = task_outcome {
        if outcome.changed {
            if let Some(channel_id) = outcome.channel_id.as_deref() {
                crate::project_space::publish_channel_message_updated(
                    state,
                    &outcome.project_id,
                    channel_id,
                    Some(&outcome.conversation_id),
                    Some(&outcome.task_id),
                    "ai_result",
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn spawn_pending_pc_cli_completion_replay(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            let receipts = match state.store.list_pending_node_cli_completion_receipts(100) {
                Ok(receipts) => receipts,
                Err(error) => {
                    tracing::warn!(%error, "读取 pending PC CLI completion receipts 失败");
                    continue;
                }
            };
            for candidate in receipts {
                let claim_id = uuid::Uuid::new_v4().to_string();
                let receipt = match state
                    .store
                    .claim_node_cli_completion_receipt(&candidate.event_id, &claim_id)
                {
                    Ok(Some(receipt)) => receipt,
                    Ok(None) => continue,
                    Err(error) => {
                        tracing::warn!(
                            event_id = %candidate.event_id,
                            %error,
                            "claim pending PC CLI completion receipt 失败"
                        );
                        continue;
                    }
                };
                let completion =
                    match serde_json::from_str::<CliCompletionEnvelope>(&receipt.payload_json)
                        .with_context(|| format!("解析 completion receipt {}", receipt.event_id))
                    {
                        Ok(completion) => completion,
                        Err(error) => {
                            let _ = state.store.finish_node_cli_completion_claim_rejected(
                                &receipt.event_id,
                                &claim_id,
                                &error.to_string(),
                            );
                            continue;
                        }
                    };
                let binding = match prepare_binding(&state, &receipt.node_id, &completion) {
                    Ok(binding) => binding,
                    Err(ReplayFailure::Retry(error)) => {
                        let _ = state.store.finish_node_cli_completion_claim_retry(
                            &receipt.event_id,
                            &claim_id,
                            &error,
                        );
                        continue;
                    }
                    Err(ReplayFailure::Reject(error)) => {
                        let _ = state.store.finish_node_cli_completion_claim_rejected(
                            &receipt.event_id,
                            &claim_id,
                            &error,
                        );
                        continue;
                    }
                };
                if let Err(error) = apply_receipt(&state, &receipt, &completion, binding, &claim_id)
                {
                    match error {
                        ReplayFailure::Retry(error) => {
                            let _ = state.store.finish_node_cli_completion_claim_retry(
                                &receipt.event_id,
                                &claim_id,
                                &error,
                            );
                        }
                        ReplayFailure::Reject(error) => {
                            let _ = state.store.finish_node_cli_completion_claim_rejected(
                                &receipt.event_id,
                                &claim_id,
                                &error,
                            );
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
#[path = "pc_completion_replay_tests.rs"]
mod tests;
