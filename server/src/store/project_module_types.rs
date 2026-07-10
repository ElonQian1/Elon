use rusqlite::Row;
use serde::Serialize;

pub(crate) const UI_TUNER_MODULE_KEY: &str = "ui-tuner";
pub(super) const UI_TUNER_MAIN_CONVERSATION_PREFIX: &str = "ui-tuner-main";
pub(super) const UI_TUNER_MAIN_TITLE: &str = "微调画布 · 主会话";
pub(super) const UI_TUNER_DEFAULT_SUMMARY: &str = "微调画布的长期目标是：真机截图/XML 采集、清晰过滤、点击元素、项目 Codex CLI 读取并修改源码、沉淀可复用 UI 标准配置、重新真机验收。";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectModuleWorkspace {
    pub project_id: String,
    pub user_id: String,
    pub module_key: String,
    pub canonical_conversation_id: String,
    pub active_conversation_id: String,
    pub stable_summary: String,
    pub memory_revision: i64,
    pub last_checkpoint_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectModuleConversation {
    pub conversation_id: String,
    pub title: String,
    pub is_canonical: bool,
    pub parent_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub source_checkpoint_id: Option<String>,
    pub selected_element_name: Option<String>,
    pub status: String,
    pub last_task_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectModuleMemory {
    pub id: String,
    pub owner_user_id: Option<String>,
    pub scope_type: String,
    pub category: String,
    pub content: String,
    pub status: String,
    pub importance: i64,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
    pub source_task_id: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectModuleCheckpoint {
    pub id: String,
    pub conversation_id: String,
    pub source_message_id: String,
    pub task_id: String,
    pub context_artifact_id: Option<String>,
    pub memory_revision: i64,
    pub status: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectModuleContextArtifact {
    pub id: String,
    pub conversation_id: String,
    pub schema_version: String,
    #[serde(skip_serializing)]
    pub payload_json: String,
    pub payload_sha256: String,
    pub selected_element_name: Option<String>,
    pub resource_id: Option<String>,
    pub source_file: Option<String>,
    pub user_intent: String,
    pub task_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiTunerWorkspaceBundle {
    pub workspace: ProjectModuleWorkspace,
    pub sessions: Vec<ProjectModuleConversation>,
    pub memories: Vec<ProjectModuleMemory>,
    pub latest_checkpoint: Option<ProjectModuleCheckpoint>,
}

pub(crate) struct CreateUiTunerContextArtifact<'a> {
    pub conversation_id: &'a str,
    pub schema_version: &'a str,
    pub payload_json: &'a str,
    pub payload_sha256: &'a str,
    pub selected_element_name: Option<&'a str>,
    pub resource_id: Option<&'a str>,
    pub source_file: Option<&'a str>,
    pub user_intent: &'a str,
}

pub(super) fn map_workspace(row: &Row<'_>) -> rusqlite::Result<ProjectModuleWorkspace> {
    Ok(ProjectModuleWorkspace {
        project_id: row.get(0)?,
        user_id: row.get(1)?,
        module_key: row.get(2)?,
        canonical_conversation_id: row.get(3)?,
        active_conversation_id: row.get(4)?,
        stable_summary: row.get(5)?,
        memory_revision: row.get(6)?,
        last_checkpoint_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub(super) fn map_conversation(row: &Row<'_>) -> rusqlite::Result<ProjectModuleConversation> {
    Ok(ProjectModuleConversation {
        conversation_id: row.get(0)?,
        title: row.get(1)?,
        is_canonical: row.get::<_, i64>(2)? != 0,
        parent_conversation_id: row.get(3)?,
        source_message_id: row.get(4)?,
        source_checkpoint_id: row.get(5)?,
        selected_element_name: row.get(6)?,
        status: row.get(7)?,
        last_task_id: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub(super) fn map_memory(row: &Row<'_>) -> rusqlite::Result<ProjectModuleMemory> {
    Ok(ProjectModuleMemory {
        id: row.get(0)?,
        owner_user_id: row.get(1)?,
        scope_type: row.get(2)?,
        category: row.get(3)?,
        content: row.get(4)?,
        status: row.get(5)?,
        importance: row.get(6)?,
        source_conversation_id: row.get(7)?,
        source_message_id: row.get(8)?,
        source_task_id: row.get(9)?,
        reviewed_by: row.get(10)?,
        reviewed_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub(super) fn map_checkpoint(row: &Row<'_>) -> rusqlite::Result<ProjectModuleCheckpoint> {
    Ok(ProjectModuleCheckpoint {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        source_message_id: row.get(2)?,
        task_id: row.get(3)?,
        context_artifact_id: row.get(4)?,
        memory_revision: row.get(5)?,
        status: row.get(6)?,
        summary: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub(super) fn map_context_artifact(
    row: &Row<'_>,
) -> rusqlite::Result<ProjectModuleContextArtifact> {
    Ok(ProjectModuleContextArtifact {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        schema_version: row.get(2)?,
        payload_json: row.get(3)?,
        payload_sha256: row.get(4)?,
        selected_element_name: row.get(5)?,
        resource_id: row.get(6)?,
        source_file: row.get(7)?,
        user_intent: row.get(8)?,
        task_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
