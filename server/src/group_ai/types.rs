use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const COLLAB_MODE_SOLO: &str = "solo";
pub(crate) const COLLAB_MODE_CRITIC: &str = "critic";
pub(crate) const COLLAB_MODE_SPLIT: &str = "split";
pub(crate) const MATTER_STATUS_PLAN_READY: &str = "plan_ready";
pub(crate) const MATTER_STATUS_RUNNING: &str = "running";
pub(crate) const MATTER_STATUS_REVIEW_READY: &str = "review_ready";
pub(crate) const MATTER_STATUS_DONE: &str = "done";
pub(crate) const MATTER_STATUS_CANCELED: &str = "canceled";
pub(crate) const MATTER_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiNodeAuthorization {
    pub id: String,
    pub project_id: String,
    pub provider_user_id: String,
    pub node_id: String,
    pub allowed_clis: Vec<String>,
    pub permission_level: String,
    pub enabled: bool,
    pub created_by_user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpsertNodeAuthorizationRequest {
    #[serde(alias = "nodeId")]
    pub node_id: String,
    #[serde(default, alias = "allowedClis")]
    pub allowed_clis: Vec<String>,
    #[serde(default, alias = "permissionLevel")]
    pub permission_level: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AvailableGroupAiNode {
    pub node_id: String,
    pub provider_user_id: String,
    pub display_name: String,
    pub short_id: String,
    pub online: bool,
    pub cli_connected: bool,
    pub allowed_clis: Vec<String>,
    pub authorized: bool,
    pub authorization: Option<ProjectAiNodeAuthorization>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiBot {
    pub bot_id: String,
    pub project_id: String,
    pub provider_user_id: String,
    pub node_id: String,
    pub display_name: String,
    pub runtime_route: String,
    pub cli_name: String,
    pub capabilities: Vec<String>,
    pub risk_level: String,
    pub online: bool,
    pub cli_connected: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateMatterPlanRequest {
    #[serde(alias = "channelId")]
    pub channel_id: String,
    #[serde(default, alias = "sourceMessageId")]
    pub source_message_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub brief: String,
    #[serde(default, alias = "collaborationMode")]
    pub collaboration_mode: Option<String>,
    #[serde(default, alias = "acceptanceCriteria")]
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CreateMatterRecord {
    pub project_id: String,
    pub channel_id: String,
    pub requester_user_id: String,
    pub source_message_id: Option<String>,
    pub title: String,
    pub brief: String,
    pub collaboration_mode: String,
    pub participant_user_ids: Vec<String>,
    pub node_policy_json: Value,
    pub acceptance_criteria: Vec<String>,
    pub plan_json: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiMatter {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub requester_user_id: String,
    pub decision_user_id: Option<String>,
    pub source_message_id: Option<String>,
    pub title: String,
    pub brief: String,
    pub collaboration_mode: String,
    pub status: String,
    pub participant_user_ids: Vec<String>,
    pub node_policy: Value,
    pub acceptance_criteria: Vec<String>,
    pub plan: Value,
    pub final_summary: Option<String>,
    pub final_decision: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub(crate) struct CreateMatterAssignmentRecord {
    pub matter_id: String,
    pub bot_id: String,
    pub assignee_user_id: Option<String>,
    pub provider_user_id: String,
    pub node_id: String,
    pub role: String,
    pub runtime_route: String,
    pub cli_name: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiMatterAssignment {
    pub id: String,
    pub matter_id: String,
    pub bot_id: String,
    pub assignee_user_id: Option<String>,
    pub provider_user_id: String,
    pub node_id: String,
    pub role: String,
    pub runtime_route: String,
    pub cli_name: String,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub status: String,
    pub result_summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiEvent {
    pub id: String,
    pub matter_id: String,
    pub project_id: String,
    pub actor_user_id: Option<String>,
    pub event_type: String,
    pub payload: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RecordAssignmentArtifactRequest {
    #[serde(default, alias = "artifactKind")]
    pub artifact_kind: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, alias = "worktreePath")]
    pub worktree_path: Option<String>,
    #[serde(default, alias = "branchName")]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default, alias = "diffStat")]
    pub diff_stat: Vec<String>,
    #[serde(default, alias = "testResults")]
    pub test_results: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordAssignmentArtifactInput {
    pub project_id: String,
    pub matter_id: String,
    pub assignment_id: String,
    pub uploader_user_id: Option<String>,
    pub artifact_kind: String,
    pub summary: Option<String>,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub files: Vec<String>,
    pub diff_stat: Vec<String>,
    pub test_results: Vec<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiAssignmentArtifact {
    pub id: String,
    pub project_id: String,
    pub matter_id: String,
    pub assignment_id: String,
    pub uploader_user_id: Option<String>,
    pub artifact_kind: String,
    pub summary: Option<String>,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub files: Vec<String>,
    pub diff_stat: Vec<String>,
    pub test_results: Vec<String>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RecordReviewRequest {
    #[serde(default, alias = "reviewerBotId")]
    pub reviewer_bot_id: Option<String>,
    #[serde(default, alias = "targetAssignmentId")]
    pub target_assignment_id: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub finding: Option<Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordReviewInput {
    pub matter_id: String,
    pub reviewer_bot_id: Option<String>,
    pub reviewer_user_id: Option<String>,
    pub target_assignment_id: Option<String>,
    pub severity: String,
    pub finding: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiReview {
    pub id: String,
    pub matter_id: String,
    pub reviewer_bot_id: Option<String>,
    pub reviewer_user_id: Option<String>,
    pub target_assignment_id: Option<String>,
    pub severity: String,
    pub finding: Value,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CreateMergeRequestRequest {
    #[serde(alias = "assignmentId")]
    pub assignment_id: String,
    #[serde(default, alias = "worktreePath")]
    pub worktree_path: Option<String>,
    #[serde(default, alias = "branchName")]
    pub branch_name: Option<String>,
    #[serde(default, alias = "mergeStrategy")]
    pub merge_strategy: Option<String>,
    #[serde(default, alias = "reviewStatus")]
    pub review_status: Option<String>,
    #[serde(default, alias = "riskLevel")]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct UpdateMergeRequestRequest {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, alias = "reviewStatus")]
    pub review_status: Option<String>,
    #[serde(default, alias = "riskLevel")]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CreateMergeRequestInput {
    pub project_id: String,
    pub matter_id: String,
    pub assignment_id: String,
    pub requested_by_user_id: Option<String>,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub merge_strategy: String,
    pub review_status: String,
    pub risk_level: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProjectAiMergeRequest {
    pub id: String,
    pub project_id: String,
    pub matter_id: String,
    pub assignment_id: String,
    pub requested_by_user_id: Option<String>,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    pub status: String,
    pub merge_strategy: String,
    pub review_status: String,
    pub risk_level: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
