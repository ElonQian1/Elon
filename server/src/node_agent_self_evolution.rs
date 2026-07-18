//! Durable, low-priority post-task evolution queue for the local PC node.

#[path = "node_agent_self_evolution_scheduler.rs"]
mod scheduler;
pub(crate) use scheduler::spawn_scheduler;
#[path = "node_agent_self_evolution_support.rs"]
mod support;
use support::{
    admission, default_max_retries, error_response, internal_admission, now_ms, retry_at,
    retryable_failure, same_gate_observation, schema_version,
};

use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use homecli_proto::{CancelRequestAudit, InterruptionSource};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    Credentials, NodeRuntime,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SelfEvolutionItem {
    pub logical_id: String,
    pub root_task_id: String,
    pub parent_task_id: String,
    pub owner_user_id: String,
    pub agent_id: String,
    pub install_id: String,
    pub project_id: String,
    pub channel_id: Option<String>,
    pub conversation_id: String,
    pub workspace_path: String,
    pub execution_worktree: Option<String>,
    pub execution_branch: Option<String>,
    #[serde(default)]
    pub execution_isolated: bool,
    pub prompt: String,
    pub runtime_permission: String,
    pub status: String,
    pub active_task_id: Option<String>,
    pub generation: u32,
    pub pause_reason: Option<String>,
    pub yield_reason: Option<String>,
    pub interruption_source: Option<InterruptionSource>,
    pub review_verdict: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_by: Option<String>,
    pub review_source: Option<String>,
    pub reviewed_at_ms: Option<u128>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    pub next_retry_at_ms: Option<u128>,
    pub last_error: Option<String>,
    pub pending_action: Option<PendingSelfEvolutionAction>,
    pub created_at_ms: u128,
    pub updated_at_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PendingSelfEvolutionAction {
    pub action_id: String,
    pub action: String,
    pub note: Option<String>,
    pub actor: String,
    pub source: String,
    pub requested_at_ms: u128,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct SelfEvolutionGates {
    #[serde(default)]
    pub foreground_task_ids: Vec<String>,
    #[serde(default)]
    pub publish_active: bool,
    #[serde(default)]
    pub publish_status: String,
    #[serde(default)]
    pub publish_owner: Option<String>,
    #[serde(default)]
    pub publish_waiter_count: usize,
    #[serde(default)]
    pub update_active: bool,
    #[serde(default)]
    pub resource_pressure: bool,
    #[serde(default)]
    pub checked_at_ms: u128,
}

impl SelfEvolutionGates {
    fn blocker(&self) -> Option<&'static str> {
        if !self.foreground_task_ids.is_empty() {
            Some("foreground_task")
        } else if self.publish_active {
            Some("global_publish")
        } else if self.update_active {
            Some("node_update")
        } else if self.resource_pressure {
            Some("resource_pressure")
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SelfEvolutionState {
    #[serde(default = "schema_version")]
    schema_version: u32,
    #[serde(default)]
    items: Vec<SelfEvolutionItem>,
    #[serde(default)]
    gates: SelfEvolutionGates,
}

impl Default for SelfEvolutionState {
    fn default() -> Self {
        Self {
            schema_version: schema_version(),
            items: Vec::new(),
            gates: SelfEvolutionGates::default(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct SelfEvolutionCoordinator {
    path: PathBuf,
    state: Arc<Mutex<SelfEvolutionState>>,
    load_error: Arc<Option<String>>,
}

impl SelfEvolutionCoordinator {
    pub(crate) fn default() -> Self {
        Self::new(super::state_path().with_file_name("self-evolution-queue.json"))
    }

    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let loaded = if path.exists() {
            std::fs::read(&path)
                .with_context(|| format!("read self evolution queue {}", path.display()))
                .and_then(|bytes| {
                    serde_json::from_slice(&bytes).context("parse self evolution queue")
                })
        } else {
            Ok(SelfEvolutionState::default())
        };
        let (state, load_error) = match loaded {
            Ok(state) => (state, None),
            Err(error) => (SelfEvolutionState::default(), Some(error.to_string())),
        };
        Self {
            path,
            state: Arc::new(Mutex::new(state)),
            load_error: Arc::new(load_error),
        }
    }

    fn mutate<T>(&self, apply: impl FnOnce(&mut SelfEvolutionState) -> Result<T>) -> Result<T> {
        if let Some(error) = self.load_error.as_ref() {
            anyhow::bail!("self evolution queue failed closed: {error}");
        }
        let mut state = self.state.lock().expect("self evolution queue lock");
        let before = serde_json::to_vec(&*state)?;
        let output = apply(&mut state)?;
        let after = serde_json::to_vec(&*state)?;
        if before != after {
            let bytes = serde_json::to_vec_pretty(&*state)?;
            crate::node_agent_atomic_file::write(&self.path, &bytes)?;
        }
        Ok(output)
    }

    fn register(&self, item: SelfEvolutionItem) -> Result<SelfEvolutionItem> {
        self.mutate(|state| {
            if state.items.iter().any(|current| {
                current.root_task_id == item.root_task_id
                    && !matches!(current.status.as_str(), "completed" | "failed")
            }) {
                anyhow::bail!("an active self evolution item already owns this root task");
            }
            state.items.push(item.clone());
            Ok(item)
        })
    }

    fn list_for_owner(&self, owner: &str) -> Result<(Vec<SelfEvolutionItem>, SelfEvolutionGates)> {
        if let Some(error) = self.load_error.as_ref() {
            anyhow::bail!("self evolution queue failed closed: {error}");
        }
        let state = self.state.lock().expect("self evolution queue lock");
        Ok((
            state
                .items
                .iter()
                .filter(|item| item.owner_user_id == owner)
                .cloned()
                .collect(),
            state.gates.clone(),
        ))
    }

    fn active_task_ids(&self) -> HashSet<String> {
        self.state
            .lock()
            .expect("self evolution queue lock")
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.status.as_str(),
                    "running" | "starting" | "pause_requested"
                )
            })
            .filter_map(|item| item.active_task_id.clone())
            .collect()
    }

    fn update_gates(&self, gates: SelfEvolutionGates) -> Result<()> {
        self.mutate(|state| {
            if !same_gate_observation(&state.gates, &gates) {
                state.gates = gates;
            }
            Ok(())
        })
    }

    fn reconcile_records(
        &self,
        local_tasks: &crate::node_agent_local_task_store::LocalTaskStore,
    ) -> Result<()> {
        self.mutate(|state| {
            for item in &mut state.items {
                let Some(task_id) = item.active_task_id.clone() else {
                    continue;
                };
                let before = (
                    item.status.clone(),
                    item.active_task_id.clone(),
                    item.pause_reason.clone(),
                    item.next_retry_at_ms,
                );
                let record = local_tasks.get(&task_id)?;
                let Some(record) = record else {
                    if item.status == "starting" {
                        item.status = "queued".to_string();
                        item.active_task_id = None;
                    }
                    continue;
                };
                match record.status.as_str() {
                    "running" | "recovering" | "reattaching" => {
                        if item.status != "pause_requested" {
                            item.status = "running".to_string();
                        }
                    }
                    "done" => {
                        item.status = "review_required".to_string();
                        item.pause_reason = None;
                    }
                    "canceled" | "failed" if item.status == "pause_requested" => {
                        item.status = "paused".to_string();
                        item.active_task_id = None;
                    }
                    "resume_required" | "interrupted" => {
                        item.status = "paused".to_string();
                        item.pause_reason = Some("node_restart".to_string());
                        item.yield_reason = Some("node_restart".to_string());
                        item.interruption_source = Some(InterruptionSource::NodeRestart);
                        item.active_task_id = None;
                    }
                    "canceled" | "failed" => {
                        let error = record
                            .error
                            .clone()
                            .unwrap_or_else(|| record.status.clone());
                        if retryable_failure(&error) && item.retry_count < item.max_retries {
                            item.retry_count += 1;
                            item.status = "retry_wait".to_string();
                            item.next_retry_at_ms = Some(retry_at(item.retry_count));
                            item.last_error = Some(error);
                        } else {
                            item.status = "failed".to_string();
                            item.last_error = Some(error);
                        }
                        item.active_task_id = None;
                    }
                    _ => {}
                }
                if before
                    != (
                        item.status.clone(),
                        item.active_task_id.clone(),
                        item.pause_reason.clone(),
                        item.next_retry_at_ms,
                    )
                {
                    item.updated_at_ms = now_ms();
                }
            }
            Ok(())
        })
    }

    fn request_gate_pauses(&self) -> Result<Vec<(String, String)>> {
        self.mutate(|state| {
            let Some(reason) = state.gates.blocker().map(str::to_string) else {
                return Ok(Vec::new());
            };
            let mut requests = Vec::new();
            for item in &mut state.items {
                if item.status == "running" {
                    if let Some(task_id) = item.active_task_id.clone() {
                        item.status = "pause_requested".to_string();
                        item.pause_reason = Some(reason.clone());
                        item.yield_reason = Some(reason.clone());
                        item.interruption_source = Some(if reason == "node_update" {
                            InterruptionSource::UpdaterApply
                        } else {
                            InterruptionSource::SupervisorIntervention
                        });
                        item.updated_at_ms = now_ms();
                        requests.push((task_id, reason.clone()));
                    }
                }
            }
            Ok(requests)
        })
    }

    fn reserve_next(&self) -> Result<Option<SelfEvolutionItem>> {
        self.mutate(|state| {
            if state.gates.blocker().is_some()
                || state.items.iter().any(|item| {
                    matches!(
                        item.status.as_str(),
                        "running" | "starting" | "pause_requested"
                    )
                })
            {
                return Ok(None);
            }
            let Some(item) = state.items.iter_mut().find(|item| {
                matches!(item.status.as_str(), "queued" | "paused")
                    || (item.status == "retry_wait"
                        && item.next_retry_at_ms.is_some_and(|at| at <= now_ms()))
            }) else {
                return Ok(None);
            };
            item.generation += 1;
            item.active_task_id = Some(format!("local-{}", Uuid::new_v4()));
            item.status = "starting".to_string();
            item.next_retry_at_ms = None;
            item.updated_at_ms = now_ms();
            Ok(Some(item.clone()))
        })
    }

    fn mark_dispatch_failed(&self, logical_id: &str, reason: &str) -> Result<()> {
        self.mutate(|state| {
            if let Some(item) = state
                .items
                .iter_mut()
                .find(|item| item.logical_id == logical_id)
            {
                item.last_error = Some(reason.to_string());
                if retryable_failure(reason) && item.retry_count < item.max_retries {
                    item.retry_count += 1;
                    item.status = "retry_wait".to_string();
                    item.next_retry_at_ms = Some(retry_at(item.retry_count));
                } else {
                    item.status = "failed".to_string();
                }
                item.pause_reason = Some("dispatch_failed".to_string());
                item.active_task_id = None;
                item.updated_at_ms = now_ms();
            }
            Ok(())
        })
    }

    fn record_execution_worktree(
        &self,
        logical_id: &str,
        workspace: &crate::pc_workspace_provisioner::ConversationWorkspaceResult,
    ) -> Result<()> {
        if !workspace.isolated || workspace.base_workspace_path.is_none() {
            anyhow::bail!("self evolution requires an isolated execution worktree");
        }
        self.mutate(|state| {
            let item = state
                .items
                .iter_mut()
                .find(|item| item.logical_id == logical_id)
                .context("self evolution item not found")?;
            item.execution_worktree = Some(workspace.workspace_path.clone());
            item.execution_branch = workspace.branch.clone();
            item.execution_isolated = true;
            item.updated_at_ms = now_ms();
            Ok(())
        })
    }

    fn begin_action(
        &self,
        owner: &str,
        logical_id: &str,
        action: &str,
        note: Option<String>,
        actor: &str,
        source: &str,
    ) -> Result<(SelfEvolutionItem, Option<String>)> {
        self.mutate(|state| {
            let item = state
                .items
                .iter_mut()
                .find(|item| item.owner_user_id == owner && item.logical_id == logical_id)
                .context("self evolution item not found")?;
            let valid = match action {
                "pause" => matches!(item.status.as_str(), "running" | "starting"),
                "resume" => matches!(item.status.as_str(), "paused" | "failed" | "retry_wait"),
                "approve" | "reject" => item.status == "review_required",
                _ => false,
            };
            if !valid {
                if item
                    .pending_action
                    .as_ref()
                    .is_some_and(|pending| pending.action == action)
                {
                    return Ok((item.clone(), item.active_task_id.clone()));
                }
                anyhow::bail!("self evolution action is not valid for current state");
            }
            item.pending_action = Some(PendingSelfEvolutionAction {
                action_id: format!("evolution-action-{}", Uuid::new_v4()),
                action: action.to_string(),
                note,
                actor: actor.to_string(),
                source: source.to_string(),
                requested_at_ms: now_ms(),
            });
            item.updated_at_ms = now_ms();
            Ok((item.clone(), item.active_task_id.clone()))
        })
    }

    fn commit_action(
        &self,
        owner: &str,
        logical_id: &str,
        action: &str,
    ) -> Result<SelfEvolutionItem> {
        self.mutate(|state| {
            let item = state
                .items
                .iter_mut()
                .find(|item| item.owner_user_id == owner && item.logical_id == logical_id)
                .context("self evolution item not found")?;
            let pending = item
                .pending_action
                .take()
                .context("self evolution action has no durable intent")?;
            if pending.action != action {
                anyhow::bail!("self evolution pending action does not match");
            }
            match action {
                "pause" => {
                    item.status = "pause_requested".to_string();
                    item.pause_reason = Some("manual_pause".to_string());
                    item.yield_reason = Some("manual_pause".to_string());
                    item.interruption_source = Some(InterruptionSource::SupervisorIntervention);
                }
                "resume" => {
                    item.status = "queued".to_string();
                    item.pause_reason = None;
                    item.review_verdict = None;
                    item.next_retry_at_ms = None;
                }
                "approve" => {
                    item.active_task_id = None;
                    item.status = "completed".to_string();
                    item.review_verdict = Some("approved".to_string());
                    item.review_note = pending.note;
                }
                "reject" => {
                    item.active_task_id = None;
                    item.status = "paused".to_string();
                    item.review_verdict = Some("changes_requested".to_string());
                    item.review_note = pending.note;
                    item.pause_reason = Some("review_changes_requested".to_string());
                }
                _ => anyhow::bail!("unsupported self evolution action"),
            }
            if matches!(action, "approve" | "reject") {
                item.reviewed_by = Some(pending.actor);
                item.review_source = Some(pending.source);
                item.reviewed_at_ms = Some(now_ms());
            }
            item.updated_at_ms = now_ms();
            Ok(item.clone())
        })
    }
}

pub(crate) struct SelfEvolutionEnqueue {
    pub project_id: String,
    pub channel_id: Option<String>,
    pub workspace_path: String,
    pub prompt: String,
    pub runtime_permission: String,
    pub contract: SupervisionContract,
}

pub(crate) struct AdmissionError {
    pub status: StatusCode,
    pub message: String,
}

pub(crate) async fn enqueue(
    runtime: &Arc<NodeRuntime>,
    creds: &Credentials,
    request: SelfEvolutionEnqueue,
) -> std::result::Result<SelfEvolutionItem, AdmissionError> {
    let parent_id = request.contract.parent_task_id.as_deref().ok_or_else(|| {
        admission(
            StatusCode::BAD_REQUEST,
            "post_task_improvement requires parent_task_id",
        )
    })?;
    let root_id = request.contract.root_task_id.as_deref().ok_or_else(|| {
        admission(
            StatusCode::BAD_REQUEST,
            "post_task_improvement requires root_task_id",
        )
    })?;
    if request.contract.protocol != SUPERVISION_PROTOCOL {
        return Err(admission(
            StatusCode::BAD_REQUEST,
            "post_task_improvement requires the supervision v1 protocol",
        ));
    }
    let parent = runtime
        .local_tasks
        .get_for_owner(&creds.owner_user_id, parent_id)
        .map_err(internal_admission)?
        .ok_or_else(|| admission(StatusCode::NOT_FOUND, "parent local task was not found"))?;
    if !matches!(parent.status.as_str(), "done" | "failed" | "canceled") {
        return Err(admission(
            StatusCode::CONFLICT,
            "the user task must reach a terminal state before self evolution is queued",
        ));
    }
    if parent.agent_id != creds.agent_id
        || parent.install_id != runtime.install_id
        || parent.project_id != request.project_id
    {
        return Err(admission(
            StatusCode::CONFLICT,
            "parent task identity, node installation, or project does not match",
        ));
    }
    let parent_contract = crate::node_agent_local_task_supervision::load_supervision_contract(
        &runtime.task_journal,
        &parent.task_id,
    )
    .map_err(internal_admission)?
    .ok_or_else(|| {
        admission(
            StatusCode::CONFLICT,
            "parent task has no durable supervision contract",
        )
    })?;
    let expected_root = parent_contract
        .root_task_id
        .as_deref()
        .unwrap_or(&parent.task_id);
    if root_id != expected_root {
        return Err(admission(
            StatusCode::CONFLICT,
            "self evolution root task does not match its parent root",
        ));
    }
    let requested_workspace = crate::node_agent_workspace_match::canonical_or_original(
        std::path::Path::new(&request.workspace_path),
    );
    let parent_workspace = crate::node_agent_workspace_match::canonical_or_original(
        std::path::Path::new(&parent.workspace_path),
    );
    if requested_workspace != parent_workspace {
        return Err(admission(
            StatusCode::CONFLICT,
            "self evolution must inherit the exact parent project workspace authorization",
        ));
    }
    let now = now_ms();
    let item = SelfEvolutionItem {
        logical_id: format!("evolution-{}", Uuid::new_v4()),
        root_task_id: root_id.to_string(),
        parent_task_id: parent.task_id,
        owner_user_id: creds.owner_user_id.clone(),
        agent_id: creds.agent_id.clone(),
        install_id: runtime.install_id.clone(),
        project_id: request.project_id,
        channel_id: request.channel_id,
        conversation_id: format!("self-evolution-{}", Uuid::new_v4()),
        workspace_path: parent.workspace_path,
        execution_worktree: None,
        execution_branch: None,
        execution_isolated: false,
        prompt: request.prompt,
        runtime_permission: request.runtime_permission,
        status: "queued".to_string(),
        active_task_id: None,
        generation: 0,
        pause_reason: None,
        yield_reason: None,
        interruption_source: None,
        review_verdict: None,
        review_note: None,
        reviewed_by: None,
        review_source: None,
        reviewed_at_ms: None,
        retry_count: 0,
        max_retries: default_max_retries(),
        next_retry_at_ms: None,
        last_error: None,
        pending_action: None,
        created_at_ms: now,
        updated_at_ms: now,
    };
    runtime
        .self_evolution
        .register(item)
        .map_err(internal_admission)
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route("/api/self-evolution", get(list_items))
        .route("/api/self-evolution/:logical_id/pause", post(pause_item))
        .route("/api/self-evolution/:logical_id/resume", post(resume_item))
        .route("/api/self-evolution/:logical_id/review", post(review_item))
}

#[derive(Deserialize)]
struct ReviewRequest {
    verdict: String,
    note: Option<String>,
}

async fn list_items(State(runtime): State<Arc<NodeRuntime>>) -> Response {
    let Some(creds) = runtime.creds().await else {
        return error_response(StatusCode::UNAUTHORIZED, "node has no bound owner");
    };
    match runtime.self_evolution.list_for_owner(&creds.owner_user_id) {
        Ok((items, gates)) => {
            Json(json!({"ok": true, "items": items, "gates": gates})).into_response()
        }
        Err(error) => error_response(StatusCode::CONFLICT, error.to_string()),
    }
}

async fn pause_item(State(runtime): State<Arc<NodeRuntime>>, Path(id): Path<String>) -> Response {
    action_response(runtime, id, "pause", None).await
}

async fn resume_item(State(runtime): State<Arc<NodeRuntime>>, Path(id): Path<String>) -> Response {
    action_response(runtime, id, "resume", None).await
}

async fn review_item(
    State(runtime): State<Arc<NodeRuntime>>,
    Path(id): Path<String>,
    Json(request): Json<ReviewRequest>,
) -> Response {
    let action = match request.verdict.trim() {
        "approve" => "approve",
        "reject" => "reject",
        _ => return error_response(StatusCode::BAD_REQUEST, "verdict must be approve or reject"),
    };
    action_response(runtime, id, action, request.note).await
}

async fn action_response(
    runtime: Arc<NodeRuntime>,
    id: String,
    action: &str,
    note: Option<String>,
) -> Response {
    let Some(creds) = runtime.creds().await else {
        return error_response(StatusCode::UNAUTHORIZED, "node has no bound owner");
    };
    let actor = format!("pc_operator:{}", creds.owner_user_id);
    match runtime.self_evolution.begin_action(
        &creds.owner_user_id,
        id.trim(),
        action,
        note,
        &actor,
        "local_pc_ui",
    ) {
        Ok((item, cancel_task_id)) => {
            if let Some(task_id) = cancel_task_id {
                if action == "pause" {
                    let audit =
                        CancelRequestAudit::now("node_agent", "local_pc_ui", "manual_pause")
                            .with_interruption_source(InterruptionSource::SupervisorIntervention);
                    if !runtime.cancel_cli_prompt_with_audit(&task_id, &audit).await {
                        let terminal = runtime
                            .local_tasks
                            .get(&task_id)
                            .ok()
                            .flatten()
                            .is_some_and(|record| {
                                matches!(
                                    record.status.as_str(),
                                    "cancel_requested" | "canceled" | "failed" | "done"
                                )
                            });
                        if !terminal {
                            return error_response(
                                StatusCode::CONFLICT,
                                "durable pause audit/cancel failed; retry is safe",
                            );
                        }
                    }
                } else if matches!(action, "approve" | "reject") {
                    let verdict = if action == "approve" {
                        "accepted"
                    } else {
                        "rejected"
                    };
                    if let Err(error) =
                        crate::node_agent_local_task_supervision::record_actor_review(
                            &runtime,
                            &task_id,
                            verdict,
                            item.pending_action
                                .as_ref()
                                .and_then(|pending| pending.note.as_deref())
                                .or(item.review_note.as_deref())
                                .as_deref()
                                .unwrap_or("self evolution queue review"),
                            &actor,
                            "local_pc_ui",
                        )
                    {
                        return error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            error.to_string(),
                        );
                    }
                }
            }
            match runtime
                .self_evolution
                .commit_action(&creds.owner_user_id, id.trim(), action)
            {
                Ok(item) => Json(json!({"ok": true, "item": item})).into_response(),
                Err(error) => error_response(StatusCode::CONFLICT, error.to_string()),
            }
        }
        Err(error) => error_response(StatusCode::CONFLICT, error.to_string()),
    }
}

#[cfg(test)]
#[path = "node_agent_self_evolution_tests.rs"]
mod tests;
