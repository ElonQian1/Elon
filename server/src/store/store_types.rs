//! Store 层公共数据结构定义。
//!
//! 所有 `Store` 方法的输入/输出类型集中在此文件，避免 store.rs 因类型膨胀超限。
//! 外部代码仍通过 `crate::store::PublicUser` 等路径访问（store.rs 做 `pub use store_types::*`）。

use serde::Serialize;

use crate::project_ws_protocol::ProjectAttachmentRef;

// ── 用户 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAgentSessionState {
    pub native_session_id: String,
    pub chat_bootstrapped: bool,
    pub dev_bootstrapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub role: String,
    pub status: String,
    pub avatar_data_url: Option<String>,
}

// ── 好友 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FriendProfile {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub avatar_data_url: Option<String>,
    pub friend_since: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: i64,
    /// 当前是否在线（由 API 层在返回前注入，store 层默认 false）
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendSearchResult {
    pub user: FriendProfile,
    pub already_friend: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddFriendResult {
    pub friend: FriendProfile,
    pub already_friend: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendChatMessage {
    pub id: String,
    pub sender_user_id: String,
    pub receiver_user_id: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub attachments: Vec<ProjectAttachmentRef>,
    pub created_at: String,
    pub context_user_id: Option<String>,
    pub outgoing: bool,
}

// ── 群聊 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct FriendGroupMemberPreview {
    pub id: String,
    pub display_name: String,
    pub avatar_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendGroupProfile {
    pub id: String,
    pub name: String,
    pub member_count: i64,
    pub members: Vec<FriendGroupMemberPreview>,
    pub created_at: String,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendGroupMessage {
    pub id: String,
    pub group_id: String,
    pub sender_user_id: String,
    pub sender_name: String,
    pub content: String,
    pub attachments: Vec<ProjectAttachmentRef>,
    pub created_at: String,
    pub outgoing: bool,
}

// ── 管理后台 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AdminUserSummary {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub role: String,
    pub status: String,
    pub project_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 管理员后台「用户项目」总览，每行代表一个项目
#[derive(Debug, Clone, Serialize)]
pub struct AdminProjectDetail {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    /// 项目在服务器上的实际绝对路径（handler 层注入）
    pub workspace_dir: String,
    /// local_path 类型项目在 DB 里记录的自定义路径
    pub workspace_path: Option<String>,
    /// PC 本地项目绑定的节点 ID；为空表示服务器本机工作区
    pub node_id: Option<String>,
    pub source_type: String,
    pub template: String,
    pub status: String,
    pub created_by_id: String,
    pub created_by_account: String,
    pub last_task_status: Option<String>,
    pub last_apk_url: Option<String>,
    pub updated_at: String,
    /// 创建者最近登录设备名
    pub last_device_name: Option<String>,
    /// 创建者最近登录时的 APK 版本
    pub last_apk_version: Option<String>,
}

/// 管理员后台「活跃设备」总览，每行代表一个登录 session
#[derive(Debug, Clone, Serialize)]
pub struct AdminSessionEntry {
    pub id: String,
    pub user_id: String,
    pub user_account: String,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

/// 管理员后台「会话列表」，每行代表一个 conversation
#[derive(Debug, Clone, Serialize)]
pub struct AdminConversationEntry {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub user_account: String,
    pub title: Option<String>,
    pub status: String,
    pub message_count: i64,
    pub task_count: i64,
    pub last_task_status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

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
    pub description: Option<String>,
    pub template: String,
    pub owner_account: String,
    pub owner_id: String,
    pub member_count: i64,
    pub is_public: bool,
    pub join_mode: String, // "open" | "approval" | "invite" | "readonly"
    pub last_task_status: Option<String>,
    pub latest_apk_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 项目成员条目（商店/成员列表用）
#[derive(Debug, Clone, Serialize)]
pub struct ProjectMemberEntry {
    pub user_id: String,
    pub account: String,
    pub avatar_data_url: Option<String>,
    pub role: String, // "owner" | "admin" | "editor" | "member" | "observer"
    pub joined_at: String,
}

// ── 项目空间 / 频道 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProjectSpaceSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub role: String,
    pub member_count: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannel {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub kind: String,
    pub position: i64,
    pub last_message: Option<String>,
    pub last_message_at: Option<String>,
    pub unread_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectChannelMessage {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub sender_user_id: Option<String>,
    pub sender_name: Option<String>,
    pub kind: String,
    pub content: String,
    pub task_id: Option<String>,
    pub suggestion_status: Option<String>,
    pub suggestion_resolved_by: Option<String>,
    pub suggestion_resolved_by_name: Option<String>,
    pub suggestion_resolved_at: Option<String>,
    pub created_at: String,
    pub outgoing: bool,
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
    pub role: String,
    pub content: String,
    pub created_at: String,
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
    pub description: Option<String>,
    pub workspace_key: String,
    pub template: String,
    pub source_type: String,
    pub repo_url: Option<String>,
    pub branch: Option<String>,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub status: String,
    pub role: String,
    pub member_count: i64,
    pub is_public: bool,
    pub join_mode: String,
    pub last_task_status: Option<String>,
    pub last_apk_url: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectAccess {
    pub id: String,
    pub name: String,
    pub workspace_key: String,
    pub source_type: String,
    pub workspace_path: Option<String>,
    pub node_id: Option<String>,
    pub role: String,
    pub status: String,
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

#[derive(Debug, Clone)]
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
