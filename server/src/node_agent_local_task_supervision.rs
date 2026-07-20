//! Codex Desktop -> Yilong PC supervised local-task protocol.
//!
//! The node remains the executor. This module only adds a durable contract,
//! deterministic evidence summary, and reviewer verdicts on top of the existing
//! local task journal.

#[path = "node_agent_local_task_supervision_evidence.rs"]
mod evidence;
#[path = "node_agent_local_task_supervision_validation.rs"]
mod validation;

use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use evidence::{
    codex_failure_summary, codex_item_failed, collect_changed_files, tool_result_failed,
};
use validation::{clean_enum, clean_id, clean_list, clean_optional_id};

use crate::{
    node_agent_task_journal::{TaskJournal, TaskJournalEventView},
    node_agent_task_journal_lock::with_task_journal_io_lock,
    NodeRuntime,
};

pub(crate) const SUPERVISION_PROTOCOL: &str = "elon.desktop_pc_supervision.v1";
const DEFAULT_SUPERVISOR: &str = "codex_desktop";
const MAX_CRITERIA: usize = 20;
const MAX_CRITERION_CHARS: usize = 2_000;
const MAX_REVIEW_CHARS: usize = 20_000;
const MAX_IMPROVEMENTS: usize = 20;
const MAX_IMPROVEMENT_CHARS: usize = 2_000;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SupervisionContractInput {
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub supervisor: Option<String>,
    #[serde(default)]
    pub task_role: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub root_task_id: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub improvement_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SupervisionContract {
    pub protocol: String,
    pub supervisor: String,
    pub task_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_task_id: Option<String>,
    pub acceptance_criteria: Vec<String>,
    pub improvement_policy: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SupervisionReviewRequest {
    verdict: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    improvements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupervisionReview {
    protocol: String,
    verdict: String,
    summary: String,
    improvements: Vec<String>,
    reviewed_by: String,
    #[serde(default)]
    review_source: String,
    reviewed_at_ms: u128,
}

#[derive(Debug, Default, Serialize)]
struct SupervisionEvidence {
    event_count: usize,
    tool_calls: usize,
    tool_results: usize,
    failed_tools: usize,
    file_change_events: usize,
    changed_files: Vec<String>,
    command_exit_codes: Vec<Value>,
    failure_summaries: Vec<String>,
    agent_messages: usize,
    terminal_event_seen: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct SupervisionState {
    pub protocol: &'static str,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract: Option<SupervisionContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review: Option<SupervisionReview>,
    evidence: SupervisionEvidence,
}

impl SupervisionState {
    pub(crate) fn contract(&self) -> Option<&SupervisionContract> {
        self.contract.as_ref()
    }
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/local-tasks/:task_id/supervision/review",
            post(review_task),
        )
        .route(
            "/api/local-tasks/:task_id/supervision/desktop-review",
            post(desktop_review_task),
        )
}

pub(crate) fn normalize_contract(
    input: SupervisionContractInput,
) -> Result<SupervisionContract, String> {
    let requested_protocol = input.protocol.as_deref().map(str::trim);
    if requested_protocol.is_some_and(|protocol| protocol != SUPERVISION_PROTOCOL) {
        return Err(format!(
            "supervision.protocol 必须是 {SUPERVISION_PROTOCOL}。"
        ));
    }
    let supervisor = clean_id(
        input.supervisor.as_deref().unwrap_or(DEFAULT_SUPERVISOR),
        "supervisor",
    )?;
    let task_role = clean_enum(
        input.task_role.as_deref().unwrap_or("requirement"),
        "task_role",
        &[
            "requirement",
            "capability_repair",
            "resume_original",
            "post_task_improvement",
        ],
    )?;
    if matches!(
        task_role.as_str(),
        "resume_original" | "post_task_improvement"
    ) && requested_protocol != Some(SUPERVISION_PROTOCOL)
    {
        return Err(format!(
            "{task_role} 必须显式携带 supervision.protocol={SUPERVISION_PROTOCOL}。"
        ));
    }
    let improvement_policy = clean_enum(
        input
            .improvement_policy
            .as_deref()
            .unwrap_or("after_task_or_unblock"),
        "improvement_policy",
        &["after_task_or_unblock", "after_task_only", "observe_only"],
    )?;
    let parent_task_id = clean_optional_id(input.parent_task_id.as_deref(), "parent_task_id")?;
    let root_task_id = clean_optional_id(input.root_task_id.as_deref(), "root_task_id")?;
    if task_role == "post_task_improvement" && (parent_task_id.is_none() || root_task_id.is_none())
    {
        return Err("post_task_improvement 必须携带 parent_task_id 和 root_task_id。".to_string());
    }
    let acceptance_criteria = clean_list(
        input.acceptance_criteria,
        MAX_CRITERIA,
        MAX_CRITERION_CHARS,
        "acceptance_criteria",
    )?;
    let acceptance_criteria = if acceptance_criteria.is_empty() {
        vec![
            "完成用户需求，并遵守项目自身的验证、提交、发布和收尾规则。".to_string(),
            "回传修改、测试、提交/发布结果，以及任何尚未解决的阻塞证据。".to_string(),
        ]
    } else {
        acceptance_criteria
    };
    Ok(SupervisionContract {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        supervisor,
        task_role,
        parent_task_id,
        root_task_id,
        acceptance_criteria,
        improvement_policy,
    })
}

pub(crate) fn load_supervision_contract(
    journal: &TaskJournal,
    task_id: &str,
) -> anyhow::Result<Option<SupervisionContract>> {
    Ok(load_supervision_state(journal, task_id)?.contract)
}

pub(crate) fn executor_prompt(user_prompt: &str, contract: Option<&SupervisionContract>) -> String {
    let Some(contract) = contract else {
        return user_prompt.to_string();
    };
    let contract_json = serde_json::to_string(contract).unwrap_or_else(|_| "{}".to_string());
    format!(
        r#"<elon-pc-executor version="1" protocol="{protocol}">
你是由一龙 PC 本机节点启动的执行者，不是桌面监督者。
1. 直接在当前项目完成任务；不得再次把写任务派发给 PC 节点，避免递归。
2. 读取并遵守项目 AGENTS.md、共享工作流、隔离 worktree、验证、提交、发布和统一收尾要求。
3. 桌面监督者会独立检查 journal、diff、测试和产物；不要只用文字声称完成。
4. 非阻塞的平台改进先记录，完成用户任务后再建议；只有能力缺口阻断原任务时，保存现场并明确标记 capability_gap。
5. 最终回复分别说明用户需求结果、验证证据、阻塞/风险和建议的平台改进。
supervision_contract={contract_json}
</elon-pc-executor>

<user-request>
{user_prompt}
</user-request>"#,
        protocol = SUPERVISION_PROTOCOL,
    )
}

pub(crate) fn contract_payload(contract: &SupervisionContract) -> Value {
    serde_json::to_value(contract).unwrap_or_else(|_| {
        json!({
            "protocol": SUPERVISION_PROTOCOL,
        })
    })
}

pub(crate) fn record_supervision_event(
    journal: &TaskJournal,
    req_id: &str,
    event_type: &str,
    payload: Value,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        matches!(event_type, "supervision_contract" | "supervision_review"),
        "unsupported supervision event type"
    );
    with_task_journal_io_lock(|| {
        journal.append_event(json!({
            "type": event_type,
            "req_id": req_id,
            "payload": payload,
            "at_ms": now_ms()
        }))
    })
}

pub(crate) fn supervision_state(events: &[TaskJournalEventView]) -> SupervisionState {
    let mut contract = None;
    let mut review = None;
    let mut evidence = SupervisionEvidence::default();
    let mut changed_files = BTreeSet::new();

    for view in events {
        observe_event(
            &view.event,
            &mut contract,
            &mut review,
            &mut evidence,
            &mut changed_files,
        );
    }
    finish_state(contract, review, evidence, changed_files)
}

pub(crate) fn load_supervision_state(
    journal: &TaskJournal,
    task_id: &str,
) -> anyhow::Result<SupervisionState> {
    with_task_journal_io_lock(|| {
        let path = journal.events_path();
        if !path.exists() {
            return Ok(finish_state(
                None,
                None,
                SupervisionEvidence::default(),
                BTreeSet::new(),
            ));
        }
        let file = File::open(&path).with_context(|| format!("打开 {:?}", path))?;
        let mut contract = None;
        let mut review = None;
        let mut evidence = SupervisionEvidence::default();
        let mut changed_files = BTreeSet::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line.with_context(|| format!("读取 {:?}", path))?;
            let event: Value = match serde_json::from_str(&line) {
                Ok(event) => event,
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        seq = index + 1,
                        %error,
                        "skipping corrupt supervision journal event line"
                    );
                    continue;
                }
            };
            if event.get("req_id").and_then(Value::as_str) != Some(task_id) {
                continue;
            }
            observe_event(
                &event,
                &mut contract,
                &mut review,
                &mut evidence,
                &mut changed_files,
            );
        }
        Ok(finish_state(contract, review, evidence, changed_files))
    })
}

async fn review_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(request): Json<SupervisionReviewRequest>,
) -> Response {
    review_task_as(runtime, task_id, request, "pc_operator", "local_pc_api").await
}

async fn desktop_review_task(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(task_id): Path<String>,
    Json(request): Json<SupervisionReviewRequest>,
) -> Response {
    review_task_as(
        runtime,
        task_id,
        request,
        DEFAULT_SUPERVISOR,
        "codex_desktop_helper",
    )
    .await
}

async fn review_task_as(
    runtime: Arc<NodeRuntime>,
    task_id: String,
    request: SupervisionReviewRequest,
    reviewed_by: &str,
    review_source: &str,
) -> Response {
    let Some(creds) = runtime.creds().await else {
        return json_error(
            StatusCode::UNAUTHORIZED,
            "本机节点尚未绑定账号，不能提交监督结论。",
        );
    };
    let task = match runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, task_id.trim())
    {
        Ok(Some(task)) => task,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "本机任务不存在。"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let contract = match load_supervision_state(&runtime.task_journal, task_id.trim()) {
        Ok(state) if state.enabled => state.contract,
        Ok(_) => return json_error(StatusCode::BAD_REQUEST, "该任务没有桌面监督契约。"),
        Err(error) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    };
    let review = match normalize_review(request, reviewed_by, review_source) {
        Ok(review) => review,
        Err(error) => return json_error(StatusCode::BAD_REQUEST, error),
    };
    let payload = serde_json::to_value(&review).unwrap_or_else(|_| json!({}));
    if let Err(error) = record_supervision_event(
        &runtime.task_journal,
        task_id.trim(),
        "supervision_review",
        payload,
    ) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    if let Err(error) = record_update_review_if_present(&runtime, task_id.trim(), &review) {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
    }
    if review.verdict == "accepted" {
        if let Err(error) =
            release_accepted_worktree_lease(&runtime, &task, contract.as_ref(), task_id.trim())
        {
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string());
        }
    }
    match load_supervision_state(&runtime.task_journal, task_id.trim()) {
        Ok(supervision) => Json(json!({
            "ok": true,
            "task_id": task_id,
            "supervision": supervision,
        }))
        .into_response(),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
    }
}

pub(crate) fn record_actor_review(
    runtime: &NodeRuntime,
    task_id: &str,
    verdict: &str,
    summary: &str,
    reviewed_by: &str,
    review_source: &str,
) -> anyhow::Result<()> {
    let task = runtime
        .local_tasks
        .get(task_id)?
        .context("reviewed local task was not found")?;
    let contract = load_supervision_state(&runtime.task_journal, task_id)?.contract;
    let review = SupervisionReview {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        verdict: verdict.to_string(),
        summary: summary.trim().to_string(),
        improvements: Vec::new(),
        reviewed_by: clean_id(reviewed_by, "reviewed_by").map_err(anyhow::Error::msg)?,
        review_source: clean_id(review_source, "review_source").map_err(anyhow::Error::msg)?,
        reviewed_at_ms: now_ms(),
    };
    let already_recorded = load_supervision_state(&runtime.task_journal, task_id)?
        .review
        .as_ref()
        .is_some_and(|current| {
            current.verdict == review.verdict
                && current.summary == review.summary
                && current.reviewed_by == review.reviewed_by
                && current.review_source == review.review_source
        });
    if !already_recorded {
        record_supervision_event(
            &runtime.task_journal,
            task_id,
            "supervision_review",
            serde_json::to_value(&review)?,
        )?;
        record_update_review_if_present(runtime, task_id, &review)?;
    }
    if verdict == "accepted" {
        release_accepted_worktree_lease(runtime, &task, contract.as_ref(), task_id)?;
    }
    Ok(())
}

fn record_update_review_if_present(
    runtime: &NodeRuntime,
    task_id: &str,
    review: &SupervisionReview,
) -> anyhow::Result<()> {
    if runtime.update_recovery.receipt_for_task(task_id)?.is_none() {
        return Ok(());
    }
    runtime.update_recovery.record_final_review(
        task_id,
        crate::node_agent_update_recovery::UpdateRecoveryReview {
            verdict: review.verdict.clone(),
            summary: review.summary.clone(),
            reviewed_by: review.reviewed_by.clone(),
            reviewed_at_ms: review.reviewed_at_ms,
        },
    )?;
    Ok(())
}

fn release_accepted_worktree_lease(
    runtime: &NodeRuntime,
    task: &crate::node_agent_local_task_store::LocalTaskRecord,
    contract: Option<&SupervisionContract>,
    task_id: &str,
) -> anyhow::Result<()> {
    let Some(contract) = contract else {
        return Ok(());
    };
    let root_task_id = contract
        .root_task_id
        .as_deref()
        .or(contract.parent_task_id.as_deref())
        .unwrap_or(task_id);
    let status = task.workspace_status.as_ref();
    let base = status
        .and_then(|value| value.get("base_workspace_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&task.workspace_path);
    let journal_cwd = runtime
        .task_journal
        .snapshot(task_id, 0, 1)?
        .record
        .and_then(|record| record.cwd);
    let receipt_workspace = runtime
        .update_recovery
        .receipt_for_task(task_id)?
        .map(|receipt| receipt.workspace.workspace_path);
    let active = status
        .and_then(|value| value.get("active_workspace_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(journal_cwd)
        .or(receipt_workspace);
    let Some(active) = active else {
        return Ok(());
    };
    let base = std::path::Path::new(base);
    let active = std::path::Path::new(&active);
    if !base.exists() || crate::node_agent_update_checkpoint::same_path(base, active) {
        return Ok(());
    }
    crate::node_agent_supervision_worktree_lease::release(base, active, root_task_id)
}

fn observe_event(
    event: &Value,
    contract: &mut Option<SupervisionContract>,
    review: &mut Option<SupervisionReview>,
    evidence: &mut SupervisionEvidence,
    changed_files: &mut BTreeSet<String>,
) {
    evidence.event_count += 1;
    match event.get("type").and_then(Value::as_str).unwrap_or("") {
        "supervision_contract" => {
            *contract = event
                .get("payload")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
        }
        "supervision_review" => {
            *review = event
                .get("payload")
                .cloned()
                .and_then(|value| serde_json::from_value(value).ok());
        }
        "finished" | "done" | "failed" | "canceled" | "interrupted" | "resume_required" => {
            evidence.terminal_event_seen = true;
        }
        _ => {}
    }

    let tool_event = if event.get("type").and_then(Value::as_str) == Some("tool_event") {
        event.get("event").unwrap_or(event)
    } else {
        event
    };
    match tool_event.get("type").and_then(Value::as_str).unwrap_or("") {
        "tool_call" => {
            evidence.tool_calls += 1;
            if tool_event.get("tool").and_then(Value::as_str) == Some("file_change") {
                evidence.file_change_events += 1;
            }
            collect_changed_files(tool_event, changed_files);
        }
        "tool_result" => {
            evidence.tool_results += 1;
            if tool_result_failed(tool_event) {
                evidence.failed_tools += 1;
            }
            collect_changed_files(tool_event, changed_files);
        }
        "codex_item" => {
            observe_codex_item(tool_event, evidence, changed_files);
        }
        _ => {}
    }
}

fn observe_codex_item(
    event: &Value,
    evidence: &mut SupervisionEvidence,
    changed_files: &mut BTreeSet<String>,
) {
    let lifecycle = event.get("lifecycle").and_then(Value::as_str).unwrap_or("");
    let item = event.get("item").unwrap_or(event);
    match item.get("type").and_then(Value::as_str).unwrap_or("") {
        "command_execution" => match lifecycle {
            "started" => evidence.tool_calls += 1,
            "completed" => {
                evidence.tool_results += 1;
                if let Some(exit_code) = item.get("exit_code").and_then(Value::as_i64) {
                    evidence.command_exit_codes.push(json!({
                        "command": item.get("command").and_then(Value::as_str).unwrap_or("command"),
                        "exit_code": exit_code,
                    }));
                }
                if codex_item_failed(item) {
                    evidence.failed_tools += 1;
                    if let Some(summary) = codex_failure_summary(item) {
                        evidence.failure_summaries.push(summary);
                    }
                }
            }
            _ => {}
        },
        "file_change" => {
            if lifecycle == "started" {
                evidence.tool_calls += 1;
            } else if lifecycle == "completed" {
                evidence.tool_results += 1;
                evidence.file_change_events += 1;
                if codex_item_failed(item) {
                    evidence.failed_tools += 1;
                }
            }
            collect_changed_files(item, changed_files);
        }
        "agent_message" if lifecycle == "completed" => {
            evidence.agent_messages += 1;
        }
        _ => {}
    }
}

fn finish_state(
    contract: Option<SupervisionContract>,
    review: Option<SupervisionReview>,
    mut evidence: SupervisionEvidence,
    changed_files: BTreeSet<String>,
) -> SupervisionState {
    evidence.changed_files = changed_files.into_iter().take(50).collect();
    evidence.command_exit_codes.truncate(100);
    evidence.failure_summaries.truncate(20);
    SupervisionState {
        protocol: SUPERVISION_PROTOCOL,
        enabled: contract.is_some(),
        contract,
        review,
        evidence,
    }
}

fn normalize_review(
    request: SupervisionReviewRequest,
    reviewed_by: &str,
    review_source: &str,
) -> Result<SupervisionReview, String> {
    let verdict = clean_enum(
        &request.verdict,
        "verdict",
        &[
            "observing",
            "accepted",
            "needs_follow_up",
            "blocked_capability",
            "rejected",
        ],
    )?;
    let summary = request.summary.trim().to_string();
    if summary.chars().count() > MAX_REVIEW_CHARS {
        return Err(format!("summary 不能超过 {MAX_REVIEW_CHARS} 个字符。"));
    }
    let improvements = clean_list(
        request.improvements,
        MAX_IMPROVEMENTS,
        MAX_IMPROVEMENT_CHARS,
        "improvements",
    )?;
    let reviewed_by = clean_id(reviewed_by, "reviewed_by")?;
    let review_source = clean_id(review_source, "review_source")?;
    Ok(SupervisionReview {
        protocol: SUPERVISION_PROTOCOL.to_string(),
        verdict,
        summary,
        improvements,
        reviewed_by,
        review_source,
        reviewed_at_ms: now_ms(),
    })
}

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({ "ok": false, "error": message.into() })),
    )
        .into_response()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
#[path = "node_agent_local_task_supervision_tests.rs"]
mod tests;
