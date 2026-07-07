use homecli_proto::ProjectWorkspaceInspectStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 项目加入申请记录
#[derive(Debug, Clone, Serialize)]
pub struct JoinRequestRecord {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub user_id: String,
    pub user_account: String,
    pub user_avatar: Option<String>,
    pub message: Option<String>,
    pub status: String, // "pending" | "approved" | "rejected"
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
}

// ── 项目商店 / 成员 ───────────────────────────────────────────────────────────

/// 项目商店卡片：公开展示的项目摘要（不含敏感路径信息）
#[derive(Debug, Clone, Serialize)]
pub struct PublicProjectItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub template: String,
    pub owner_account: String,
    pub owner_id: String,
    pub member_count: i64,
    pub is_public: bool,
    pub join_mode: String, // "open" | "approval" | "invite" | "readonly"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewer_role: Option<String>, // 当前访问者在该项目中的角色；未登录/未加入时为空
    pub last_task_status: Option<String>,
    pub latest_apk_url: Option<String>,
    pub icon_data_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 项目成员条目（商店/成员列表用）
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberEntry {
    pub user_id: String,
    pub account: String,
    pub global_account: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_note: Option<String>,
    pub avatar_data_url: Option<String>,
    pub role: String, // "owner" | "admin" | "editor" | "member" | "observer"
    pub roles: Vec<ProjectMemberRoleRef>,
    pub joined_at: String,
    pub is_online: bool,
    pub presence_status: String,
    pub custom_status: Option<String>,
    pub activity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banned_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banned_until: Option<String>,
    pub is_muted: bool,
    pub is_banned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_permissions: Option<std::collections::HashMap<String, ProjectChannelPermissions>>,
}

/// 项目成员持有的角色引用。`role` 仍表示最高/有效角色，`roles` 表示全部叠加角色。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberRoleRef {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub position: i64,
    pub builtin: bool,
}

/// 项目成员管理审计条目（邀请、审批、改角色、移除等）。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberAuditEntry {
    pub id: String,
    pub project_id: String,
    pub actor_user_id: Option<String>,
    pub actor_account: Option<String>,
    pub target_user_id: Option<String>,
    pub target_account: Option<String>,
    pub action: String,
    pub old_role: Option<String>,
    pub new_role: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

/// 项目成员限制状态（禁言 / 封禁）。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberModerationEntry {
    pub project_id: String,
    pub user_id: String,
    pub account: Option<String>,
    pub muted_until: Option<String>,
    pub banned_at: Option<String>,
    pub banned_until: Option<String>,
    pub note: Option<String>,
    pub updated_by: Option<String>,
    pub updated_by_account: Option<String>,
    pub updated_at: String,
    pub is_muted: bool,
    pub is_banned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPresenceSettings {
    pub user_id: String,
    pub status: String,
    pub custom_status: Option<String>,
    pub activity: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInviteLink {
    pub id: String,
    pub project_id: String,
    pub code: String,
    pub role: String,
    pub max_uses: Option<i64>,
    pub use_count: i64,
    pub expires_at: Option<String>,
    pub temporary: bool,
    pub revoked_at: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectInvitePreview {
    pub project_id: String,
    pub project_name: String,
    pub display_name: Option<String>,
    pub role: String,
    pub max_uses: Option<i64>,
    pub use_count: i64,
    pub expires_at: Option<String>,
    pub temporary: bool,
}

/// 项目角色定义。内置角色由代码生成，自定义角色来自 project_roles 表。
#[derive(Debug, Clone, Serialize)]
pub struct ProjectRoleEntry {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub color: Option<String>,
    pub position: i64,
    pub permissions: Vec<String>,
    pub builtin: bool,
    pub member_count: i64,
}

// ── 项目空间 / 频道 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSpaceSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub role: String,
    pub member_count: i64,
    pub icon_data_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gallery_images: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannel {
    pub id: String,
    pub project_id: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub category_kind: Option<String>,
    pub category_position: i64,
    pub permission_sync: bool,
    pub name: String,
    pub kind: String,
    pub position: i64,
    pub permissions: ProjectChannelPermissions,
    pub role_overrides: Vec<ProjectChannelRolePermissionOverride>,
    pub member_overrides: Vec<ProjectChannelMemberPermissionOverride>,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelCategory {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub kind: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelPermissions {
    pub can_view: bool,
    pub can_send: bool,
    pub can_start_ai: bool,
    pub can_manage: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelRolePermissionOverride {
    pub role_id: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelMemberPermissionOverride {
    pub user_id: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelMessage {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub sender_user_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_avatar_data_url: Option<String>,
    pub reply_to_message_id: Option<String>,
    pub kind: String,
    pub content: String,
    pub task_id: Option<String>,
    pub task_status: Option<String>,
    pub task_error: Option<String>,
    pub task_apk_url: Option<String>,
    pub task_codex_thread_id: Option<String>,
    pub suggestion_status: Option<String>,
    pub suggestion_resolved_by: Option<String>,
    pub suggestion_resolved_by_name: Option<String>,
    pub suggestion_resolved_at: Option<String>,
    pub created_at: String,
    pub outgoing: bool,
    pub recalled_at: Option<String>,
    pub recalled_by: Option<String>,
}

// ── 项目对话 / 任务 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberConversationEntry {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub user_account: String,
    pub title: Option<String>,
    pub status: String,
    pub is_public: bool,
    pub message_count: i64,
    pub task_count: i64,
    pub last_message: Option<String>,
    pub last_message_role: Option<String>,
    pub last_message_at: Option<String>,
    pub last_task_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberConversationMessage {
    pub id: String,
    pub project_id: String,
    pub conversation_id: Option<String>,
    pub task_id: Option<String>,
    pub user_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_avatar_data_url: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub outgoing: bool,
    pub recalled_at: Option<String>,
    pub recalled_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectResult {
    pub project: ProjectSummary,
    pub reused_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub workspace_key: String,
    pub template: String,
    pub source_type: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub storage_node_id: Option<String>,
    pub storage_repo_path: Option<String>,
    pub storage_repo_url: Option<String>,
    pub storage_worktree_path: Option<String>,
    pub storage_status: String,
    pub status: String,
    pub role: String,
    pub member_count: i64,
    pub is_public: bool,
    pub join_mode: String,
    pub runtime_permission: String,
    pub last_task_status: Option<String>,
    pub last_apk_url: Option<String>,
    pub icon_data_url: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserArchiveProject {
    #[serde(flatten)]
    pub project: ProjectSummary,
    pub owner_account: String,
    pub owner_id: String,
    /// `self` | `admin` | `system` | `member`
    pub project_origin_type: String,
    pub project_origin_label: String,
    pub conversation_count: i64,
    /// `system_archive` | `pc_node_workspace` | `external_workspace` | `server_workspace`
    pub workspace_kind: String,
    /// `phone_control` | `chat_memory`
    pub system_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_route: Option<UserArchiveConversationRoute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_status: Option<UserArchiveWorkspaceStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserArchiveConversationRoute {
    pub entry_key: String,
    pub project_id: String,
    pub project_name: String,
    pub conversation_title: String,
    pub memory_scope_type: String,
    pub memory_scope_id: Option<String>,
    pub project_created: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserArchiveWorkspaceStatus {
    pub project_id: String,
    pub workspace_kind: String,
    pub execution_target: String,
    pub health_label: String,
    pub health_tone: String,
    pub recommended_action: String,
    pub node_id: Option<String>,
    pub node_online: bool,
    pub node_cli_connected: bool,
    pub node_cli_project_ready: bool,
    pub node_display_name: Option<String>,
    pub can_run_on_pc: bool,
    pub cached_verified_can_run_on_pc: Option<bool>,
    pub latest_health_checked_at: Option<String>,
    pub latest_health_disk_free_bytes: Option<u64>,
    pub latest_execution_status: Option<String>,
    pub latest_execution_merge_status: Option<String>,
    pub latest_execution_active_workspace_path: Option<String>,
    pub warning_count: i64,
    pub warnings: Vec<String>,
    pub recovery_actions: Vec<crate::project_workspace_lifecycle::ProjectWorkspaceRecoveryAction>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserArchiveNode {
    pub node_id: String,
    pub label: String,
    pub device_name: Option<String>,
    pub display_name: String,
    pub short_id: String,
    pub online: bool,
    pub cli_connected: bool,
    pub cli_project_ready: bool,
    pub allowed_clis: Vec<String>,
    pub project_count: i64,
    pub project_limit: i64,
    pub project_slots_remaining: i64,
    pub disk_free_bytes: Option<u64>,
    pub capacity_label: String,
    pub capacity_tone: String,
    pub capacity_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserArchiveSummary {
    pub total_projects: i64,
    pub system_project_count: i64,
    pub owned_project_count: i64,
    pub shared_project_count: i64,
    pub node_count: i64,
    pub online_node_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectWorkspaceHealthSnapshot {
    pub project_id: String,
    pub node_id: Option<String>,
    pub workspace_path: Option<String>,
    pub can_run_on_pc: bool,
    pub verified_can_run_on_pc: Option<bool>,
    pub health_label: String,
    pub health_tone: String,
    pub recommended_action: String,
    pub warning_count: i64,
    pub warnings: Vec<String>,
    pub live_inspect: Option<ProjectWorkspaceInspectStatus>,
    pub inspect_error: Option<String>,
    pub disk_free_bytes: Option<u64>,
    pub path_exists: Option<bool>,
    pub is_dir: Option<bool>,
    pub is_git_worktree: Option<bool>,
    pub cli_available: Option<bool>,
    pub captured_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectWorkspaceHealthTarget {
    pub project_id: String,
    pub source_type: String,
    pub node_id: String,
    pub workspace_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectPcWorkspaceBinding {
    pub project_id: String,
    pub owner_user_id: String,
    pub node_id: String,
    pub workspace_path: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub git_head: Option<String>,
    pub source: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectAccess {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    pub template: String,
    pub source_type: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub storage_node_id: Option<String>,
    pub storage_repo_path: Option<String>,
    pub storage_repo_url: Option<String>,
    pub storage_worktree_path: Option<String>,
    pub storage_status: String,
    pub role: String,
    pub status: String,
    pub runtime_permission: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectRuntimePermission {
    pub project_id: String,
    pub mode: String,
    pub updated_by: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDeletionTarget {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    pub source_type: String,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: Option<String>,
    pub message: String,
    pub status: String,
    pub apk_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskEventRecord {
    pub seq: i64,
    pub event_json: String,
    pub created_at: String,
}
