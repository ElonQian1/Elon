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
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon_conversation_router_{}.db",
            Uuid::new_v4().simple()
        ));
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn scope_aliases_route_to_expected_system_entry() {
        assert_eq!(
            ConversationEntryKind::from_scope(Some("chat")),
            ConversationEntryKind::ChatMemory
        );
        assert_eq!(
            ConversationEntryKind::from_scope(Some("normal_chat")),
            ConversationEntryKind::ChatMemory
        );
        assert_eq!(
            ConversationEntryKind::from_scope(Some("phone_control")),
            ConversationEntryKind::PhoneControl
        );
        assert_eq!(
            ConversationEntryKind::from_scope(None),
            ConversationEntryKind::PhoneControl
        );
    }

    #[test]
    fn system_routes_create_distinct_project_archives() {
        let store = temp_store();
        let user = store
            .create_user("conversation-router@example.com", "secret1", None, None)
            .expect("user should be created");

        let phone = resolve_system_conversation_route(
            &store,
            &user.id,
            ConversationEntryKind::PhoneControl,
        )
        .expect("phone route should resolve");
        let chat =
            resolve_system_conversation_route(&store, &user.id, ConversationEntryKind::ChatMemory)
                .expect("chat route should resolve");

        assert_ne!(phone.project_id, chat.project_id);
        assert_eq!(phone.entry_key, "phone_control");
        assert_eq!(chat.entry_key, "chat_memory");
        assert!(phone.project_created);
        assert!(chat.project_created);
        assert_eq!(
            phone.memory_scope_id.as_deref(),
            Some(phone.project_id.as_str())
        );
        assert_eq!(
            chat.memory_scope_id.as_deref(),
            Some(chat.project_id.as_str())
        );
    }

    #[test]
    fn project_route_uses_project_memory_scope() {
        let store = temp_store();
        let user = store
            .create_user("project-route@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "项目会话", None, None)
            .expect("project should create")
            .project;

        let route = resolve_project_conversation_route(&store, &user.id, &project.id)
            .expect("project route should resolve");

        assert_eq!(route.project_id, project.id);
        assert_eq!(route.entry_key, "project");
        assert_eq!(route.memory_scope_type, MEMORY_SCOPE_PROJECT);
        assert_eq!(route.memory_scope_id.as_deref(), Some(project.id.as_str()));
    }
}
