/// project_space.rs — 项目空间频道 API
///
/// 这是商城“加入项目”之后的协作空间入口。普通频道消息写入共享频道；
/// AI 开发频道可以把一次成员发起的开发任务写回同一频道，供项目成员共同跟进。
mod channel_ai_recovery;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};

use crate::{
    project_auth::{auth_from_headers, can_edit, json_error, project_access},
    project_events,
    project_landing,
    project_mobile::ensure_mobile_project,
    store::{
        ProjectAccess, PublicUser, CHANNEL_PERMISSION_MANAGE, CHANNEL_PERMISSION_SEND,
        CHANNEL_PERMISSION_START_AI, CHANNEL_PERMISSION_VIEW,
    },
    types::AppState,
};

pub(crate) use crate::project_space_task_control::{
    active_channel_ai_task_ids, is_channel_ai_task_active,
};

const DOCS_CHANNEL_KIND: &str = "docs";
const CHANNEL_AI_CANCEL_MESSAGE: &str = "用户已停止 AI 开发任务。";

mod channel_ai;
mod channel_ai_spawn;
mod channel_messages;
mod member_conversations;
mod permissions;

pub use channel_ai::{
    cancel_channel_ai_task, cancel_user_project_channel_ai_task,
    decide_channel_ai_tool_approval, decide_user_project_channel_ai_tool_approval,
    start_channel_ai_task, start_user_project_channel_ai_task,
    summarize_channel_selection, summarize_user_project_channel_selection,
};
pub use channel_messages::{
    list_channel_messages, list_user_project_channel_messages,
    recall_channel_message, recall_user_project_channel_message,
    send_channel_message, send_user_project_channel_message,
};
pub use member_conversations::{
    list_member_conversation_messages, list_member_conversations,
    send_member_conversation_message, update_member_conversation_visibility,
};
pub use permissions::{
    get_channel_category_permissions, get_channel_permissions,
    update_channel_category_permissions, update_channel_permissions,
};

#[derive(Deserialize)]
pub struct ChannelMessagesQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct MemberConversationQuery {
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendChannelMessageRequest {
    pub content: String,
    pub reply_to_message_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMemberConversationVisibilityRequest {
    pub is_public: bool,
}

#[derive(Deserialize)]
pub struct UpdateProjectDescriptionRequest {
    pub description: String,
}

#[derive(Deserialize)]
pub struct UpdateProjectGalleryImageRequest {
    #[serde(alias = "index")]
    pub slot: usize,
    #[serde(default, alias = "imageUrl", alias = "url")]
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateChannelRolePermissionRequest {
    pub member_id: Option<String>,
    #[serde(default)]
    pub role_id: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

pub async fn get_user_project_space(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    project_space_response(state, user, project).await
}

pub async fn get_project_space(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let access = match project_space_access(&state, &user.id, &project_id) {
        Ok(access) => access,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    project_space_response(state, user, access).await
}

pub async fn update_user_project_description(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<UpdateProjectDescriptionRequest>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    update_project_description_response(state, user.id, project, req)
}

pub async fn update_project_description(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectDescriptionRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    update_project_description_response(state, user.id, project, req)
}

pub async fn update_user_project_gallery_image(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    Json(req): Json<UpdateProjectGalleryImageRequest>,
) -> Response {
    let (_user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    update_project_gallery_image_response(state, project, req)
}

pub async fn update_project_gallery_image(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateProjectGalleryImageRequest>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    update_project_gallery_image_response(state, project, req)
}

fn update_project_description_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    req: UpdateProjectDescriptionRequest,
) -> Response {
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前成员角色不能编辑项目简介");
    }
    match state
        .store
        .update_project_description(&user_id, &project.id, &req.description)
    {
        Ok(project) => Json(serde_json::json!({ "project": project })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn update_project_gallery_image_response(
    state: Arc<AppState>,
    project: ProjectAccess,
    req: UpdateProjectGalleryImageRequest,
) -> Response {
    if !can_edit(&project.role) {
        return json_error(StatusCode::FORBIDDEN, "当前成员角色不能更换项目应用图片");
    }
    match state
        .store
        .update_project_gallery_image(&project.id, req.slot, req.image_url.as_deref())
    {
        Ok(images) => Json(serde_json::json!({ "gallery_images": images })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn project_space_response(
    state: Arc<AppState>,
    user: PublicUser,
    access: ProjectAccess,
) -> Response {
    let project = match state.store.project_space_summary(&user.id, &access.id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    let channels = match state
        .store
        .list_project_space_channels(&user.id, &access.id)
    {
        Ok(channels) => channels,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let categories = match state.store.list_project_channel_categories(&access.id) {
        Ok(categories) => categories,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let mut members = match state.store.list_project_members(&access.id) {
        Ok(members) => members,
        Err(e) => return json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    {
        let online = state.online_users.read().await;
        for member in &mut members {
            apply_project_member_presence(member, online.contains_key(&member.user_id));
        }
    }
    let visible_channel_ids: Vec<String> =
        channels.iter().map(|channel| channel.id.clone()).collect();
    for member in &mut members {
        let mut channel_permissions = HashMap::new();
        for channel_id in &visible_channel_ids {
            match state.store.project_member_channel_permissions(
                &access.id,
                channel_id,
                &member.user_id,
            ) {
                Ok(permissions) => {
                    channel_permissions.insert(channel_id.clone(), permissions);
                }
                Err(e) => {
                    tracing::warn!(
                        project_id = %access.id,
                        channel_id = %channel_id,
                        member_user_id = %member.user_id,
                        error = %e,
                        "failed to load project member channel permissions"
                    );
                }
            }
        }
        if !channel_permissions.is_empty() {
            member.channel_permissions = Some(channel_permissions);
        }
    }
    let latest_apk = latest_project_apk_delivery(&state, &access);
    Json(serde_json::json!({
        "project": project,
        "categories": categories,
        "channels": channels,
        "members": members,
        "landing": project_landing_manifest(&state, &user.id, &access),
        "latest_apk_url": latest_apk.as_ref().map(|apk| apk.url.as_str()),
        "latest_apk_identity": latest_apk.as_ref().map(|apk| apk.identity.as_str()),
        "latest_apk_updated_at": latest_apk.as_ref().and_then(|apk| apk.updated_at.as_deref()),
    }))
    .into_response()
}

fn apply_project_member_presence(member: &mut crate::store::ProjectMemberEntry, connected: bool) {
    let configured = member.presence_status.trim().to_ascii_lowercase();
    if !connected || configured == "invisible" {
        member.is_online = false;
        member.presence_status = "offline".to_string();
        return;
    }
    member.is_online = true;
    member.presence_status = match configured.as_str() {
        "idle" | "dnd" | "online" => configured,
        _ => "online".to_string(),
    };
}

pub async fn mark_user_project_suggestion_updated(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((user_id, project_id, channel_id, message_id)): Path<(String, String, String, String)>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let (user, project) = match ensure_user_project_for_space(
        &state,
        &headers,
        &user_id,
        &project_id,
        query.get("title").map(String::as_str),
    ) {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    mark_suggestion_updated_response(state, user.id, project, channel_id, message_id)
}

pub async fn mark_suggestion_updated(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((project_id, channel_id, message_id)): Path<(String, String, String)>,
) -> Response {
    let user = match auth_from_headers(&state, &headers) {
        Ok(user) => user,
        Err(e) => return json_error(StatusCode::UNAUTHORIZED, e.to_string()),
    };
    let project = match project_access(&state, &user.id, &project_id) {
        Ok(project) => project,
        Err(e) => return json_error(StatusCode::FORBIDDEN, e.to_string()),
    };
    mark_suggestion_updated_response(state, user.id, project, channel_id, message_id)
}

fn mark_suggestion_updated_response(
    state: Arc<AppState>,
    user_id: String,
    project: ProjectAccess,
    channel_id: String,
    message_id: String,
) -> Response {
    if !project_member_can_use_channel(
        &state,
        &project.id,
        &channel_id,
        &user_id,
        CHANNEL_PERMISSION_MANAGE,
    ) {
        return json_error(StatusCode::FORBIDDEN, "当前角色无权管理该频道建议");
    }
    match state.store.mark_project_suggestion_updated(
        &user_id,
        &project.id,
        &channel_id,
        &message_id,
    ) {
        Ok(message) => Json(serde_json::json!({ "message": message })).into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

fn publish_channel_message_updated(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    conversation_id: Option<&str>,
    task_id: Option<&str>,
    kind: &str,
) {
    project_events::publish_message_updated(
        state,
        project_id,
        Some(channel_id),
        conversation_id,
        task_id,
        kind,
    );
}


// 鈹€鈹€鈹€ 瀛愭ā鍧楀叡浜伐鍏凤紙pub(super) 瀵规墍鏈?project_space 瀛愭ā鍧楀彲瑙侊級鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

fn ensure_project_member_can_speak(
    state: &AppState,
    project_id: &str,
    user_id: &str,
) -> Result<(), Response> {
    match state
        .store
        .active_project_member_muted_until(project_id, user_id)
    {
        Ok(Some(until)) => Err(json_error(
            StatusCode::FORBIDDEN,
            format!("你已被该项目禁言，禁言截止时间：{until}"),
        )),
        Ok(None) => Ok(()),
        Err(e) => Err(json_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

fn project_member_can_use_channel(
    state: &AppState,
    project_id: &str,
    channel_id: &str,
    user_id: &str,
    permission: &str,
) -> bool {
    let Ok(permissions) = state
        .store
        .project_member_channel_permissions(project_id, channel_id, user_id)
    else {
        return false;
    };
    match permission {
        CHANNEL_PERMISSION_VIEW => permissions.can_view,
        CHANNEL_PERMISSION_SEND => permissions.can_send,
        CHANNEL_PERMISSION_START_AI => permissions.can_start_ai,
        CHANNEL_PERMISSION_MANAGE => permissions.can_manage,
        _ => false,
    }
}

fn project_member_can_use_channel_category(
    state: &AppState,
    project_id: &str,
    category_id: &str,
    user_id: &str,
    permission: &str,
) -> bool {
    let Ok(permissions) =
        state
            .store
            .project_member_channel_category_permissions(project_id, category_id, user_id)
    else {
        return false;
    };
    let category_allowed = match permission {
        CHANNEL_PERMISSION_VIEW => permissions.can_view,
        CHANNEL_PERMISSION_SEND => permissions.can_send,
        CHANNEL_PERMISSION_START_AI => permissions.can_start_ai,
        CHANNEL_PERMISSION_MANAGE => permissions.can_manage,
        _ => false,
    };
    let channel_manager_allowed = if permission == CHANNEL_PERMISSION_MANAGE {
        state
            .store
            .list_project_space_channels(user_id, project_id)
            .map(|channels| {
                channels.iter().any(|channel| {
                    channel.category_id.as_deref() == Some(category_id)
                        && channel.permissions.can_manage
                })
            })
            .unwrap_or(false)
    } else {
        false
    };
    category_allowed || channel_manager_allowed
}

struct LatestProjectApkDelivery {
    url: String,
    identity: String,
    updated_at: Option<String>,
}

fn latest_project_apk_delivery(
    state: &AppState,
    project: &crate::store::ProjectAccess,
) -> Option<LatestProjectApkDelivery> {
    match state.store.latest_project_apk_delivery(&project.id) {
        Ok(Some((task_id, apk_url, updated_at))) => {
            return Some(LatestProjectApkDelivery {
                url: apk_url.clone(),
                identity: format!("task:{}:{}:{}", task_id, updated_at, apk_url),
                updated_at: Some(updated_at),
            });
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(
            project_id = %project.id,
            error = %error,
            "读取项目历史 APK 交付记录失败"
        ),
    }
    None
}

fn project_landing_manifest(
    state: &AppState,
    user_id: &str,
    project: &crate::store::ProjectAccess,
) -> Option<serde_json::Value> {
    let workspace =
        state.resolve_project_workspace(&project.workspace_key, project.workspace_path.as_deref());
    let workspace_landing = project_landing::load_workspace_landing(&workspace);
    if workspace_landing
        .as_ref()
        .is_some_and(project_landing::has_display_content)
    {
        return workspace_landing;
    }

    let snapshot = match state.store.project_landing_snapshot(user_id, &project.id) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tracing::warn!(project_id = %project.id, "读取项目首页 landing 快照失败: {error}");
            None
        }
    };
    if snapshot
        .as_ref()
        .is_some_and(project_landing::has_display_content)
    {
        return snapshot;
    }
    workspace_landing.or(snapshot)
}

fn query_limit(query: &HashMap<String, String>, fallback: i64) -> i64 {
    query
        .get("limit")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback)
}

fn project_space_access(
    state: &AppState,
    user_id: &str,
    project_id: &str,
) -> anyhow::Result<ProjectAccess> {
    state.store.get_project_space_access(user_id, project_id)
}

fn ensure_user_project_for_space(
    state: &AppState,
    headers: &HeaderMap,
    user_id: &str,
    project_id: &str,
    project_title: Option<&str>,
) -> Result<(PublicUser, ProjectAccess), Response> {
    let effective_user_id = if state.require_login {
        match auth_from_headers(state, headers) {
            Ok(user) => user.id,
            Err(e) => {
                return Err(json_error(
                    StatusCode::UNAUTHORIZED,
                    format!("请先登录后再使用（{}）", e),
                ));
            }
        }
    } else {
        user_id.to_string()
    };
    ensure_mobile_project(state, &effective_user_id, project_id, project_title)
        .map_err(|e| json_error(StatusCode::BAD_REQUEST, e.to_string()))
}

