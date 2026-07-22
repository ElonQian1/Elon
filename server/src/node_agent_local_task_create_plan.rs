//! Durable, exact dispatch context for retrying `POST /api/local-tasks`.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    node_agent_local_task_resume_context::CompiledResumeContext,
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::SupervisionContract,
    pc_workspace_provisioner::ConversationWorkspaceResult, NodeRuntime,
};

use super::idempotency;

const SCHEMA: &str = "elon.local_task_create_plan.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DurableCreatePlan {
    schema: String,
    pub(super) task_id: String,
    pub(super) supervision: Option<SupervisionContract>,
    pub(super) conversation_id: String,
    channel_id: Option<String>,
    resolved_workspace_path: String,
    pub(super) execution_workspace_path: String,
    pub(super) record_prompt: String,
    pub(super) executor_prompt: String,
    pub(super) inherited_workspace: Option<ConversationWorkspaceResult>,
    pub(super) inherited_authorization_task_id: Option<String>,
    pub(super) workspace_inheritance: Option<serde_json::Value>,
    pub(super) resume_context_journal: Option<serde_json::Value>,
    pub(super) resume_context_response: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) response_body: Option<serde_json::Value>,
}

pub(super) struct PreparedPlan<'a> {
    pub task_id: &'a str,
    pub supervision: Option<SupervisionContract>,
    pub conversation_id: &'a str,
    pub channel_id: Option<String>,
    pub resolved_workspace_path: &'a str,
    pub execution_workspace_path: &'a str,
    pub record_prompt: &'a str,
    pub executor_prompt: &'a str,
    pub inherited_workspace: Option<ConversationWorkspaceResult>,
    pub inherited_authorization_record: Option<&'a LocalTaskRecord>,
    pub workspace_inheritance: Option<serde_json::Value>,
    pub resume_context: Option<&'a CompiledResumeContext>,
}

impl DurableCreatePlan {
    pub(super) fn prepared(input: PreparedPlan<'_>) -> Self {
        Self {
            schema: SCHEMA.to_string(),
            task_id: input.task_id.to_string(),
            supervision: input.supervision,
            conversation_id: input.conversation_id.to_string(),
            channel_id: input.channel_id,
            resolved_workspace_path: input.resolved_workspace_path.to_string(),
            execution_workspace_path: input.execution_workspace_path.to_string(),
            record_prompt: input.record_prompt.to_string(),
            executor_prompt: input.executor_prompt.to_string(),
            inherited_workspace: input.inherited_workspace,
            inherited_authorization_task_id: input
                .inherited_authorization_record
                .map(|record| record.task_id.clone()),
            workspace_inheritance: input.workspace_inheritance,
            resume_context_journal: input
                .resume_context
                .map(|context| context.journal_payload.clone()),
            resume_context_response: input.resume_context.map(|context| {
                json!({
                    "schema": crate::node_agent_local_task_resume_context::RESUME_CONTEXT_SCHEMA,
                    "digest": context.digest,
                })
            }),
            response_body: None,
        }
    }

    pub(super) fn persist_or_recover(
        runtime: &NodeRuntime,
        owner_user_id: &str,
        binding: Option<&idempotency::Binding>,
        computed: Self,
    ) -> Result<Self, axum::response::Response> {
        let Some(binding) = binding else {
            return Ok(computed);
        };
        if let Some(value) = binding.request_state.clone() {
            let persisted: Self = serde_json::from_value(value).map_err(internal_error)?;
            if persisted.same_prepared_identity(&computed) {
                return Ok(persisted);
            }
            return Err(json_error(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_RECOVERY_CONTEXT_DRIFT: 持久 dispatch 上下文与重试结果不一致。",
            ));
        }
        let value = serde_json::to_value(&computed).map_err(internal_error)?;
        idempotency::save_state(runtime, owner_user_id, Some(binding), &value)?;
        Ok(computed)
    }

    pub(super) fn persist_response(
        &mut self,
        runtime: &NodeRuntime,
        owner_user_id: &str,
        binding: Option<&idempotency::Binding>,
        response: &serde_json::Value,
    ) -> Result<(), axum::response::Response> {
        self.response_body = Some(response.clone());
        let value = serde_json::to_value(self).map_err(internal_error)?;
        idempotency::save_state(runtime, owner_user_id, binding, &value)
    }

    fn same_prepared_identity(&self, other: &Self) -> bool {
        self.schema == other.schema
            && self.task_id == other.task_id
            && self.supervision == other.supervision
            && self.conversation_id == other.conversation_id
            && self.channel_id == other.channel_id
            && self.resolved_workspace_path == other.resolved_workspace_path
            && self.execution_workspace_path == other.execution_workspace_path
            && self.record_prompt == other.record_prompt
            && self.executor_prompt == other.executor_prompt
            && self.inherited_workspace == other.inherited_workspace
            && self.inherited_authorization_task_id == other.inherited_authorization_task_id
            && self.workspace_inheritance == other.workspace_inheritance
            && self.resume_context_journal == other.resume_context_journal
            && self.resume_context_response == other.resume_context_response
    }
}

fn json_error(status: StatusCode, message: impl Into<String>) -> axum::response::Response {
    (
        status,
        Json(json!({ "ok": false, "error": message.into() })),
    )
        .into_response()
}

fn internal_error(error: serde_json::Error) -> axum::response::Response {
    json_error(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_plan_keeps_exact_executor_authorization_and_resume_context() {
        let authorization = LocalTaskRecord {
            task_id: "parent-auth".into(),
            owner_user_id: "owner".into(),
            agent_id: "agent".into(),
            install_id: "install".into(),
            project_id: "project".into(),
            channel_id: None,
            conversation_id: "parent".into(),
            workspace_path: "C:/worktree".into(),
            prompt: "root".into(),
            cli: "codex".into(),
            runtime_permission: "full_access".into(),
            execution_origin: "local_offline".into(),
            billing_source: "own_codex".into(),
            status: "failed".into(),
            error: None,
            final_reply: None,
            model: None,
            codex_session_id: None,
            input_tokens: None,
            cached_input_tokens: None,
            output_tokens: None,
            reasoning_tokens: None,
            total_tokens: None,
            workspace_status: None,
            sync_state: "local_only".into(),
            completion_event_id: None,
            started_at_ms: 1,
            finished_at_ms: Some(2),
            server_ack_at_ms: None,
        };
        let contract = SupervisionContract {
            protocol: crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL.into(),
            supervisor: "codex_desktop".into(),
            task_role: "resume_original".into(),
            parent_task_id: Some("parent-auth".into()),
            root_task_id: Some("root".into()),
            acceptance_criteria: vec!["exact".into()],
            improvement_policy: "after_task_only".into(),
        };
        let resume = CompiledResumeContext {
            record_prompt: "resume-record".into(),
            executor_prompt: "EXACT_RESUME_EXECUTOR_PROMPT".into(),
            journal_payload: json!({"schema":"elon.resume_context.v1","digest":"digest-1"}),
            digest: "digest-1".into(),
        };
        let plan = DurableCreatePlan::prepared(PreparedPlan {
            task_id: "local-resume",
            supervision: Some(contract.clone()),
            conversation_id: "resume",
            channel_id: None,
            resolved_workspace_path: "C:/repo",
            execution_workspace_path: "C:/worktree",
            record_prompt: &resume.record_prompt,
            executor_prompt: &resume.executor_prompt,
            inherited_workspace: Some(ConversationWorkspaceResult {
                base_workspace_path: Some("C:/repo".into()),
                workspace_path: "C:/worktree".into(),
                isolated: true,
                branch: Some("ai/session/project/root".into()),
                supervision_root_task_id: Some("root".into()),
            }),
            inherited_authorization_record: Some(&authorization),
            workspace_inheritance: None,
            resume_context: Some(&resume),
        });
        let recovered: DurableCreatePlan =
            serde_json::from_value(serde_json::to_value(&plan).unwrap()).unwrap();
        assert_eq!(recovered.executor_prompt, "EXACT_RESUME_EXECUTOR_PROMPT");
        assert_eq!(recovered.supervision, Some(contract));
        assert_eq!(
            recovered.inherited_authorization_task_id.as_deref(),
            Some("parent-auth")
        );
        assert_eq!(
            recovered.resume_context_response.unwrap()["digest"],
            "digest-1"
        );
    }
}
