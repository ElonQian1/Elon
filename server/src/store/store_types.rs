// server/src/store/store_types.rs
//! Store 层公共数据结构定义。
//!
//! 所有 `Store` 方法的输入/输出类型集中在此文件，避免 store.rs 因类型膨胀超限。
//! 外部代码仍通过 `crate::store::PublicUser` 等路径访问（store.rs 做 `pub use store_types::*`）。

use homecli_proto::ProjectWorkspaceInspectStatus;
use serde::Serialize;

use crate::project_ws_protocol::ProjectAttachmentRef;

pub const PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE: &str = "project_write";
pub const PROJECT_RUNTIME_PERMISSION_FULL_ACCESS: &str = "full_access";
pub const PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS: &str = "danger_full_access";

pub fn normalize_project_runtime_permission(value: &str) -> Option<&'static str> {
    match value.trim() {
        PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE => Some(PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE),
        PROJECT_RUNTIME_PERMISSION_FULL_ACCESS => Some(PROJECT_RUNTIME_PERMISSION_FULL_ACCESS),
        PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS => {
            Some(PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS)
        }
        _ => None,
    }
}

pub fn default_project_runtime_permission() -> String {
    PROJECT_RUNTIME_PERMISSION_PROJECT_WRITE.to_string()
}

pub fn project_runtime_permission_allows_full_access(value: &str) -> bool {
    matches!(
        normalize_project_runtime_permission(value),
        Some(
            PROJECT_RUNTIME_PERMISSION_FULL_ACCESS | PROJECT_RUNTIME_PERMISSION_DANGER_FULL_ACCESS
        )
    )
}

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
pub struct UserConversationEntry {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub status: String,
    pub message_count: i64,
    pub last_message: Option<String>,
    pub last_message_role: Option<String>,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserConversationMessage {
    pub id: String,
    pub project_id: String,
    pub conversation_id: Option<String>,
    pub task_id: Option<String>,
    pub user_id: Option<String>,
    pub sender_name: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub outgoing: bool,
    pub recalled_at: Option<String>,
    pub recalled_by: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendSearchResult {
    pub user: FriendProfile,
    pub already_friend: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FriendRecommendation {
    pub id: String,
    pub account: String,
    pub nickname: Option<String>,
    pub phone: Option<String>,
    pub avatar_data_url: Option<String>,
    pub mutual_friend_count: i64,
    pub already_friend: bool,
    /// 当前是否在线（由 API 层在返回前注入，store 层默认 false）
    pub is_online: bool,
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
    pub recalled_at: Option<String>,
    pub recalled_by: Option<String>,
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
    pub recalled_at: Option<String>,
    pub recalled_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupAiDocument {
    pub group_id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    pub updated_by: Option<String>,
    pub updated_by_name: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupSummarySourceMessage {
    pub id: String,
    pub group_id: String,
    pub sender_user_id: String,
    pub sender_name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct GroupChatRetrievalInput {
    pub query: Option<String>,
    pub sender: Option<String>,
    pub message_ids: Vec<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupChatRetrievalHit {
    pub message: GroupSummarySourceMessage,
    pub score: i64,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupChatRetrievalResult {
    pub strategy: Vec<String>,
    pub vector_status: String,
    pub hits: Vec<GroupChatRetrievalHit>,
}

#[derive(Debug, Clone)]
pub struct GroupSummaryCreateInput {
    pub title: Option<String>,
    pub topic: Option<String>,
    pub instructions: Option<String>,
    pub message_ids: Vec<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub limit: i64,
    pub pin: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupSummaryContextPack {
    pub id: String,
    pub group_id: String,
    pub purpose: String,
    pub query: Option<String>,
    pub payload_json: String,
    pub source_start_at: Option<String>,
    pub source_end_at: Option<String>,
    pub message_count: i64,
    pub created_by: String,
    pub created_by_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupSummaryPost {
    pub id: String,
    pub group_id: String,
    pub title: String,
    pub topic: Option<String>,
    pub summary: String,
    pub status: String,
    pub context_pack_id: String,
    pub source_start_at: Option<String>,
    pub source_end_at: Option<String>,
    pub source_message_count: i64,
    pub model_used: Option<String>,
    pub error: Option<String>,
    pub pinned_at: Option<String>,
    pub pinned_by: Option<String>,
    pub pinned_by_name: Option<String>,
    pub created_by: String,
    pub created_by_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupSummaryPostDetail {
    pub post: GroupSummaryPost,
    pub context_pack: GroupSummaryContextPack,
    pub sources: Vec<GroupSummarySourceMessage>,
}

// ── 外部应用 / 子项目接入 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppGroupSeed {
    pub app_id: String,
    pub external_group_id: String,
    pub group_id: String,
    pub name: String,
    pub position: i64,
    pub auto_join: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppGroupLink {
    pub app_id: String,
    pub external_group_id: String,
    pub group_id: String,
    pub name: String,
    pub position: i64,
    pub auto_join: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAccountOrigin {
    pub app_id: String,
    pub external_user_id: String,
    pub account: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub main_user_id: Option<String>,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ExternalAccountUpsert {
    pub external_user_id: String,
    pub account: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExternalAccountSessionInput {
    pub external_user_id: String,
    pub account: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAccountSession {
    pub token: String,
    pub expires_at: String,
    pub user: PublicUser,
    pub account: ExternalAccountOrigin,
    pub default_groups: Vec<ExternalAppGroupLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trial_credit: Option<ExternalAppTrialCredit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppTrialCredit {
    pub app_id: String,
    pub amount_fen: i64,
    pub balance_after_fen: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppAuthorizationCode {
    pub code: String,
    pub app_id: String,
    pub user_id: String,
    pub scopes: Vec<String>,
    pub redirect_uri: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalAppAuthorizationExchange {
    pub app_id: String,
    pub user: PublicUser,
    pub scopes: Vec<String>,
    pub redirect_uri: Option<String>,
    pub created_at: String,
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
