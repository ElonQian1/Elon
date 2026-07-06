//! 会话入口到项目档案的统一路由。
//!
//! 手机控制、普通聊天和显式项目会话都应先解析成稳定的 `project_id`，
//! 后续会话记录、短期历史和长期记忆再围绕该项目归档。

use anyhow::{anyhow, Result};

use crate::store::{
    ProjectAccess, Store, CHAT_MEMORY_PROJECT_NAME, MEMORY_SCOPE_CHAT, MEMORY_SCOPE_PHONE_CONTROL,
    MEMORY_SCOPE_PROJECT, PHONE_CONTROL_PROJECT_NAME,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationEntryKind {
    PhoneControl,
    ChatMemory,
}

impl ConversationEntryKind {
    pub fn from_scope(value: Option<&str>) -> Self {
        match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
            "chat" | "chat_memory" | "normal_chat" | "ordinary_chat" => Self::ChatMemory,
            _ => Self::PhoneControl,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::PhoneControl => "phone_control",
            Self::ChatMemory => "chat_memory",
        }
    }

    pub fn project_name(self) -> &'static str {
        match self {
            Self::PhoneControl => PHONE_CONTROL_PROJECT_NAME,
            Self::ChatMemory => CHAT_MEMORY_PROJECT_NAME,
        }
    }

    pub fn memory_scope_type(self) -> &'static str {
        match self {
            Self::PhoneControl => MEMORY_SCOPE_PHONE_CONTROL,
            Self::ChatMemory => MEMORY_SCOPE_CHAT,
        }
    }

    pub fn conversation_title(self) -> &'static str {
        match self {
            Self::PhoneControl => "悬浮球语音会话",
            Self::ChatMemory => "普通聊天会话",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationRoute {
    pub project_id: String,
    pub project_name: String,
    pub entry_key: String,
    pub memory_scope_type: String,
    pub memory_scope_id: Option<String>,
    pub conversation_title: String,
    pub project_created: bool,
}

impl ConversationRoute {
    fn system(kind: ConversationEntryKind, project_id: String, project_created: bool) -> Self {
        Self {
            project_id: project_id.clone(),
            project_name: kind.project_name().to_string(),
            entry_key: kind.key().to_string(),
            memory_scope_type: kind.memory_scope_type().to_string(),
            memory_scope_id: Some(project_id),
            conversation_title: kind.conversation_title().to_string(),
            project_created,
        }
    }

    #[allow(dead_code)]
    pub fn project(project: ProjectAccess) -> Self {
        let project_id = project.id;
        Self {
            memory_scope_id: Some(project_id.clone()),
            project_id,
            project_name: project.name,
            entry_key: "project".to_string(),
            memory_scope_type: MEMORY_SCOPE_PROJECT.to_string(),
            conversation_title: "项目开发会话".to_string(),
            project_created: false,
        }
    }
}

pub fn resolve_system_conversation_route(
    store: &Store,
    user_id: &str,
    kind: ConversationEntryKind,
) -> Result<ConversationRoute> {
    let (project_id, created) = match kind {
        ConversationEntryKind::PhoneControl => store.ensure_balloon_project_for_user(user_id),
        ConversationEntryKind::ChatMemory => store.ensure_chat_memory_project_for_user(user_id),
    }?;
    Ok(ConversationRoute::system(kind, project_id, created))
}

pub fn ensure_user_system_conversation_routes(
    store: &Store,
    user_id: &str,
) -> Result<Vec<ConversationRoute>> {
    Ok(vec![
        resolve_system_conversation_route(store, user_id, ConversationEntryKind::PhoneControl)?,
        resolve_system_conversation_route(store, user_id, ConversationEntryKind::ChatMemory)?,
    ])
}

#[allow(dead_code)]
pub fn resolve_project_conversation_route(
    store: &Store,
    user_id: &str,
    project_id: &str,
) -> Result<ConversationRoute> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        return Err(anyhow!("project_id 不能为空"));
    }
    let project = store.get_project_access(user_id, project_id)?;
    Ok(ConversationRoute::project(project))
}


#[cfg(test)]
#[path = "conversation_router_tests.rs"]
mod tests;
